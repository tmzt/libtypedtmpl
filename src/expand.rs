//! Expansion: a typedmd graph reduced, in document order, into a tree that
//! keeps its structure. Constructs registered by tag give tags their
//! meaning; defined types (`[Template]`) are instantiated; `[Content]` is
//! the only node that writes text.

use std::collections::HashMap;
use std::fmt;

use libtypedmd::{AliasTable, LeafForm, Node, NodeKind, Sigil, Slot, Text, TextPart};

use crate::error::{Error, ErrorKind};
use crate::path::{NodePath, Segment, SlotPath};
use crate::record::{from_object_at, read_value, Record};
use crate::scope::Scope;
use crate::template::{tag, FieldDef, FieldType, Templates, TypeDef};
use crate::value::Value;

/// A node of the expanded tree: where it came from, what it bound, and
/// what it holds, in output order. Nothing is flattened here.
#[derive(Debug, Clone, PartialEq)]
pub struct Expanded {
    pub path: NodePath,
    pub tag: Option<String>,
    pub name: Option<String>,
    /// Aliases this node bound for its contents: an `[Item]`'s item alias,
    /// an instance's `this`.
    pub bindings: Vec<(String, Value)>,
    pub parts: Vec<Part>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Part {
    Literal(String),
    Value {
        sigil: Option<Sigil>,
        value: Value,
        slot: Slot,
    },
    Child(Expanded),
    /// An element a construct produced, with typed attributes.
    Element(Element),
}

/// An element: a tag, typed attributes in the order written, and children.
#[derive(Debug, Clone, PartialEq)]
pub struct Element {
    pub name: String,
    pub attrs: Vec<Attr>,
    pub children: Vec<Part>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Attr {
    pub name: String,
    pub value: AttrValue,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AttrValue {
    /// Escaped and checked by the collapse, like a slot's value.
    Value { sigil: Option<Sigil>, value: Value },
    /// A boolean attribute, written as its name.
    Flag,
}

impl Element {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            attrs: Vec::new(),
            children: Vec::new(),
        }
    }

    pub fn attr(mut self, name: impl Into<String>, sigil: Option<Sigil>, value: Value) -> Self {
        self.attrs.push(Attr {
            name: name.into(),
            value: AttrValue::Value { sigil, value },
        });
        self
    }

    pub fn flag(mut self, name: impl Into<String>) -> Self {
        self.attrs.push(Attr {
            name: name.into(),
            value: AttrValue::Flag,
        });
        self
    }

    fn write_outline(&self, f: &mut fmt::Formatter<'_>, pad: &str) -> fmt::Result {
        let mut head = format!("<{}", self.name);
        for attr in &self.attrs {
            match &attr.value {
                AttrValue::Flag => head.push_str(&format!(" {}", attr.name)),
                AttrValue::Value { value, .. } => {
                    head.push_str(&format!(" {}={}", attr.name, outline_value(value)))
                }
            }
        }
        writeln!(f, "{pad}{head}>")?;
        let inner = format!("{pad}  ");
        for part in &self.children {
            write_part(part, f, &inner)?;
        }
        Ok(())
    }
}

fn write_part(part: &Part, f: &mut fmt::Formatter<'_>, pad: &str) -> fmt::Result {
    match part {
        Part::Literal(text) => writeln!(f, "{pad}\"{}\"", quote(text)),
        Part::Value { sigil, value, .. } => {
            let label = sigil
                .map(|s| format!("[{}]", s.token()))
                .unwrap_or_else(|| "html".into());
            writeln!(f, "{pad}{label} {}", outline_value(value))
        }
        Part::Child(child) => child.write_outline(f, pad.len() / 2),
        Part::Element(element) => element.write_outline(f, pad),
    }
}

impl Expanded {
    pub(crate) fn new(path: &NodePath, tag: Option<&str>, name: Option<String>) -> Self {
        Self {
            path: path.clone(),
            tag: tag.map(str::to_string),
            name: name.filter(|n| !n.is_empty()),
            bindings: Vec::new(),
            parts: Vec::new(),
        }
    }

    /// Every node below this one, depth first; this one excluded.
    pub fn descendants(&self) -> impl Iterator<Item = &Expanded> {
        let mut stack: Vec<&Expanded> = self.children().rev().collect();
        std::iter::from_fn(move || {
            let next = stack.pop()?;
            stack.extend(next.children().rev());
            Some(next)
        })
    }

    /// The direct child nodes, in order.
    pub fn children(&self) -> impl DoubleEndedIterator<Item = &Expanded> {
        self.parts.iter().filter_map(|p| match p {
            Part::Child(c) => Some(c),
            _ => None,
        })
    }

    /// How many descendants have this tag.
    pub fn count(&self, tag: &str) -> usize {
        self.descendants()
            .filter(|e| e.tag.as_deref() == Some(tag))
            .count()
    }

    fn write_outline(&self, f: &mut fmt::Formatter<'_>, depth: usize) -> fmt::Result {
        let pad = "  ".repeat(depth);
        let segment = self.path.segments().last();
        let mut head = match (&self.tag, &self.name) {
            (Some(t), Some(n)) => format!("[{t}] {n}"),
            (Some(t), None) => format!("[{t}]"),
            (None, Some(n)) => n.clone(),
            (None, None) => "(node)".into(),
        };
        if let Some(i) = segment.and_then(|s| s.index) {
            head.push_str(&format!(" #{i}"));
        }
        for (alias, value) in &self.bindings {
            let shown = match value {
                Value::Record(r) => r.name().map(str::to_string).unwrap_or_else(|| "{…}".into()),
                other => outline_value(other),
            };
            head.push_str(&format!(" {alias}={shown}"));
        }
        writeln!(f, "{pad}{head}")?;
        let inner = format!("{pad}  ");
        for part in &self.parts {
            write_part(part, f, &inner)?;
        }
        Ok(())
    }
}

fn quote(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

fn outline_value(value: &Value) -> String {
    match value {
        Value::Text(s) | Value::Number(s) | Value::Link(s) | Value::Currency(s) => s.clone(),
        Value::Date(d) => d.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Html(h) => quote(h.as_str()),
        Value::Markdown(m) => quote(m),
        Value::List(items) => format!("({} records)", items.len()),
        Value::Record(r) => r.name().map(str::to_string).unwrap_or_else(|| "{…}".into()),
    }
}

/// One line per node, two spaces of indent per level, for tests and
/// debugging.
impl fmt::Display for Expanded {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.write_outline(f, 0)
    }
}

/// A tag with meaning in expansion.
pub trait Construct {
    /// The tag this construct gives meaning to, such as `List`.
    fn tag(&self) -> &str;
    /// Reduce `node` (whose tag is [`tag`](Self::tag)) in `scope` into the
    /// nodes that stand where it was: zero, one or several.
    fn reduce(
        &self,
        node: &Node,
        scope: &Scope<'_>,
        cx: &mut Expander<'_>,
    ) -> Result<Vec<Expanded>, Error>;
}

/// Constructs by tag.
pub struct Registry {
    constructs: HashMap<String, Box<dyn Construct>>,
    /// Constructs that apply only inside a named scope, such as `head`.
    scoped: HashMap<String, HashMap<String, Box<dyn Construct>>>,
    value_types: HashMap<String, Box<dyn ValueType>>,
}

/// A type a field may have that is read from text, such as `Url`. It
/// checks and normalises the text into one of the core values.
pub trait ValueType {
    /// The type's name, as a definition writes it: `Url`.
    fn name(&self) -> &str;
    /// Read a field's value: its text, or a slot's value written as text.
    fn read(&self, text: &str, at: &NodePath) -> Result<Value, String>;
}

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}

impl Registry {
    /// The built-ins: `List`, `Content`, `Children`, and `Item`, `Empty`
    /// and `Template`, which refuse where they stand.
    pub fn new() -> Self {
        let mut registry = Self::empty();
        registry.register(Box::new(crate::list::ListConstruct));
        registry.register(Box::new(crate::content::ContentConstruct));
        registry.register(Box::new(ChildrenConstruct));
        for (tag, message) in [
            ("Item", "an `[Item]` belongs in a `[List]`"),
            ("Empty", "an `[Empty]` belongs in a `[List]`"),
            (
                "Template",
                "a `[Template]` is a definition, not drawn where it stands",
            ),
        ] {
            registry.register(Box::new(Refuse { tag, message }));
        }
        registry
    }

    /// No constructs.
    pub fn empty() -> Self {
        Self {
            constructs: HashMap::new(),
            scoped: HashMap::new(),
            value_types: HashMap::new(),
        }
    }

    /// Register by `construct.tag()`, returning any construct it replaces.
    pub fn register(&mut self, construct: Box<dyn Construct>) -> Option<Box<dyn Construct>> {
        self.constructs
            .insert(construct.tag().to_string(), construct)
    }

    pub fn get(&self, tag: &str) -> Option<&dyn Construct> {
        self.constructs.get(tag).map(|c| c.as_ref())
    }

    /// Register a construct that applies only inside the scope `scope`.
    pub fn register_in(
        &mut self,
        scope: &str,
        construct: Box<dyn Construct>,
    ) -> Option<Box<dyn Construct>> {
        self.scoped
            .entry(scope.to_string())
            .or_default()
            .insert(construct.tag().to_string(), construct)
    }

    /// The construct for `tag` inside `scope` (falling back to the default
    /// scope), or in the default scope when `scope` is `None`.
    pub fn get_in(&self, scope: Option<&str>, tag: &str) -> Option<&dyn Construct> {
        scope
            .and_then(|s| self.scoped.get(s))
            .and_then(|c| c.get(tag))
            .map(|c| c.as_ref())
            .or_else(|| self.get(tag))
    }

    /// Register a value type by its name, returning any it replaces.
    pub fn register_value_type(
        &mut self,
        value_type: Box<dyn ValueType>,
    ) -> Option<Box<dyn ValueType>> {
        self.value_types
            .insert(value_type.name().to_string(), value_type)
    }

    pub fn value_type(&self, name: &str) -> Option<&dyn ValueType> {
        self.value_types.get(name).map(|v| v.as_ref())
    }
}

/// List sources a consumer defines, beyond paths in scope.
pub trait Sources {
    /// `Ok(None)`: not an expression this consumer defines.
    fn list(&self, expr: &str) -> Result<Option<Vec<Record>>, String>;
}

/// Defines no sources.
pub struct NoSources;

impl Sources for NoSources {
    fn list(&self, _expr: &str) -> Result<Option<Vec<Record>>, String> {
        Ok(None)
    }
}

/// Walks a graph into [`Expanded`] nodes.
pub struct Expander<'a> {
    registry: &'a Registry,
    templates: &'a Templates,
    sources: &'a dyn Sources,
    aliases: AliasTable,
    path: NodePath,
    /// The children of the instances being reduced, innermost last, already
    /// reduced in their callers' scopes.
    children: Vec<Vec<Expanded>>,
    /// The types being instantiated, innermost last.
    types: Vec<String>,
    /// The named construct scopes entered, innermost last.
    scopes: Vec<String>,
}

impl<'a> Expander<'a> {
    pub fn new(registry: &'a Registry, templates: &'a Templates, sources: &'a dyn Sources) -> Self {
        Self {
            registry,
            templates,
            sources,
            aliases: AliasTable::new(),
            path: NodePath::new(),
            children: Vec::new(),
            types: Vec::new(),
            scopes: Vec::new(),
        }
    }

    /// The alias table the consumer parses with, so sigils in content
    /// resolve as they do in rows.
    pub fn with_aliases(mut self, aliases: AliasTable) -> Self {
        self.aliases = aliases;
        self
    }

    pub(crate) fn aliases(&self) -> &AliasTable {
        &self.aliases
    }

    pub fn path(&self) -> &NodePath {
        &self.path
    }

    /// An error at the current path.
    pub fn error(&self, kind: ErrorKind, message: impl Into<String>) -> Error {
        Error::new(kind, message).at(self.path.clone())
    }

    /// Run `f` with `node` (and an index) pushed on the path; errors from
    /// it that have no path get this one.
    pub fn within<T>(
        &mut self,
        node: &Node,
        index: Option<usize>,
        f: impl FnOnce(&mut Self) -> Result<T, Error>,
    ) -> Result<T, Error> {
        let mut segment = Segment::of(node);
        segment.index = index;
        self.path.push(segment);
        let result = f(self).map_err(|e| {
            if e.path.is_empty() {
                e.at(self.path.clone())
            } else {
                e
            }
        });
        self.path.pop();
        result
    }

    /// Run `f` with the named construct scope `name` entered: inside it,
    /// constructs registered in that scope apply (the `html` module's head
    /// constructs live in `head`).
    pub fn with_scope<T>(
        &mut self,
        name: &str,
        f: impl FnOnce(&mut Self) -> Result<T, Error>,
    ) -> Result<T, Error> {
        self.scopes.push(name.to_string());
        let result = f(self);
        self.scopes.pop();
        result
    }

    /// Reduce one object: its tag's construct, or an instance of the type
    /// its tag names.
    pub fn expand(&mut self, node: &Node, scope: &Scope<'_>) -> Result<Vec<Expanded>, Error> {
        if node.as_object().is_none() {
            return Ok(Vec::new());
        }
        let registry = self.registry;
        let templates = self.templates;
        self.within(node, None, |cx| {
            let t = tag(node);
            let named = cx.scopes.last().map(String::as_str);
            if let Some(construct) = registry.get_in(named, t) {
                construct.reduce(node, scope, cx)
            } else if let Some(def) = templates.get(t) {
                cx.instance(def, node, scope).map(|e| vec![e])
            } else if registry.value_type(t).is_some() {
                let head = node
                    .as_object()
                    .map(|o| o.head.render())
                    .unwrap_or_default();
                Err(cx.error(
                    ErrorKind::UnknownType,
                    format!(
                        "`[{t}]` is a value type, a field's type: declare the field as `- {}: {t}`",
                        head.trim()
                    ),
                ))
            } else {
                Err(cx.error(
                    ErrorKind::UnknownType,
                    format!("`[{t}]` is neither a construct nor a defined type"),
                ))
            }
        })
    }

    /// Reduce a node's child objects in document order. Members are not
    /// output; prose and `- |` blocks are notes; a plain row is an error. A
    /// `[Content]` holding `{{ children }}` wraps the objects after it.
    pub fn expand_children(
        &mut self,
        node: &Node,
        scope: &Scope<'_>,
    ) -> Result<Vec<Expanded>, Error> {
        let mut objects = Vec::new();
        for child in &node.nodes {
            match &child.kind {
                NodeKind::Object(_) => objects.push(child),
                NodeKind::Member(_) => {}
                NodeKind::Leaf(leaf) if leaf.form == LeafForm::Row => {
                    return Err(self.error(
                        ErrorKind::StrayNode,
                        format!("the row `{}` is neither a member nor an object", leaf.text),
                    ))
                }
                NodeKind::Leaf(_) => {} // a note
                NodeKind::Fallback(f) => {
                    return Err(
                        self.error(ErrorKind::StrayNode, format!("could not read `{}`", f.text))
                    )
                }
            }
        }
        let mut out = Vec::new();
        for (i, child) in objects.iter().enumerate() {
            if tag(child) == "Content" && crate::content::wraps(child, &self.aliases)? {
                let mut wrapped = Vec::new();
                for following in &objects[i + 1..] {
                    wrapped.extend(self.expand(following, scope)?);
                }
                let content = self.within(child, None, |cx| {
                    crate::content::reduce(child, scope, cx, Some(wrapped))
                })?;
                out.push(content);
                return Ok(out);
            }
            out.extend(self.expand(child, scope)?);
        }
        Ok(out)
    }

    /// Resolve a slot's path in scope and check it against its sigil.
    /// `Ok(None)`: an optional field of `this` that the instance left out.
    pub fn resolve(&self, slot: &Slot, scope: &Scope<'_>) -> Result<Option<Value>, Error> {
        let in_slot = |e: Error| e.in_slot(slot);
        let path = SlotPath::parse(&slot.expr).map_err(in_slot)?;
        let root = scope.get(&path.alias).ok_or_else(|| {
            Error::new(
                ErrorKind::UnknownAlias,
                format!("`{}` is not in scope", path.alias),
            )
            .in_slot(slot)
        })?;
        if path.alias == "this" && path.fields.len() == 1 && scope.is_absent(&path.fields[0]) {
            return Ok(None);
        }
        let value = path.lookup(root).map_err(in_slot)?.clone();
        if let Some(sigil) = slot.sigil {
            agree(sigil, &value)
                .map_err(|m| Error::new(ErrorKind::SigilMismatch, m).in_slot(slot))?;
        }
        Ok(Some(value))
    }

    /// A `[List]` source's records: a path to a list (or to a record's
    /// items) in scope, or an expression the consumer defines.
    pub fn records(&self, slot: &Slot, scope: &Scope<'_>) -> Result<Vec<Record>, Error> {
        if let Ok(path) = SlotPath::parse(&slot.expr) {
            if let Some(root) = scope.get(&path.alias) {
                let value = path.lookup(root).map_err(|e| e.in_slot(slot))?;
                return match value {
                    Value::List(records) => Ok(records.clone()),
                    Value::Record(record) => match record.get("items") {
                        Some(Value::List(records)) => Ok(records.clone()),
                        _ => Err(Error::new(
                            ErrorKind::NotAList,
                            format!("`{}` has no items", slot.expr),
                        )
                        .in_slot(slot)),
                    },
                    other => Err(Error::new(
                        ErrorKind::NotAList,
                        format!("`{}` is {}, not a list", slot.expr, other.kind()),
                    )
                    .in_slot(slot)),
                };
            }
        }
        match self.sources.list(&slot.expr) {
            Ok(Some(records)) => Ok(records),
            Ok(None) => Err(Error::new(
                ErrorKind::UnknownSource,
                format!("unknown source `{}`", slot.expr),
            )
            .in_slot(slot)),
            Err(message) => Err(Error::new(ErrorKind::UnknownSource, message).in_slot(slot)),
        }
    }

    /// Reduce an instance of a defined type: read its fields against the
    /// definition, reduce its children in the caller's scope, and reduce
    /// the body in a scope of the roots and `this`.
    fn instance(
        &mut self,
        def: &TypeDef,
        node: &Node,
        scope: &Scope<'_>,
    ) -> Result<Expanded, Error> {
        if let Some(at) = self.types.iter().position(|t| t == &def.key) {
            let cycle: Vec<&str> = self.types[at..]
                .iter()
                .map(String::as_str)
                .chain([def.key.as_str()])
                .collect();
            return Err(self.error(
                ErrorKind::TemplateCycle,
                format!("the type cycle {}", cycle.join(" > ")),
            ));
        }
        let object = node.as_object().expect("instances are objects");
        let head = object.head.render().trim().to_string();
        let mut this = Record::new(Some(head.clone()).filter(|h| !h.is_empty()));
        let mut given: Vec<&str> = Vec::new();

        if let Some(field) = def.field("name") {
            if !object.head.is_empty() {
                let value = self.field_value(field, &object.head, scope)?;
                this.push("name", value);
                given.push("name");
            }
        }
        let mut children = Vec::new();
        for child in &node.nodes {
            match &child.kind {
                NodeKind::Member(m) => {
                    let field = def.field(&m.name).ok_or_else(|| {
                        self.error(
                            ErrorKind::UnknownField,
                            format!("`[{}]` has no field `{}`", def.name, m.name),
                        )
                    })?;
                    let value = match &m.rhs {
                        Some(text) if child.nodes.is_empty() => {
                            self.field_value(field, text, scope)?
                        }
                        None if !child.nodes.is_empty() => {
                            self.nested_value(field, child, scope)?
                        }
                        None => {
                            return Err(self.error(
                                ErrorKind::NoValue,
                                format!("the field `{}` has no value", m.name),
                            ))
                        }
                        Some(_) => {
                            return Err(self.error(
                                ErrorKind::RecordShape,
                                format!("the field `{}` has both a value and child rows", m.name),
                            ))
                        }
                    };
                    if !given.contains(&field.name.as_str()) {
                        given.push(&field.name);
                    }
                    this.push(field.name.clone(), value);
                }
                NodeKind::Object(_) => children.push(child),
                NodeKind::Leaf(leaf) if leaf.form == LeafForm::Row => {
                    return Err(self.error(
                        ErrorKind::StrayNode,
                        format!("the row `{}` is neither a field nor an object", leaf.text),
                    ))
                }
                _ => {}
            }
        }
        let mut absent = Vec::new();
        for field in &def.fields {
            if given.contains(&field.name.as_str()) {
                continue;
            }
            if field.optional || field.name == "name" {
                absent.push(field.name.clone());
            } else {
                return Err(self.error(
                    ErrorKind::MissingField,
                    format!("`[{}]` needs the field `{}`", def.name, field.name),
                ));
            }
        }
        if !children.is_empty() && !def.places_children() {
            return Err(self.error(
                ErrorKind::ChildrenNotPlaced,
                format!(
                    "`[{}]` has no `[Children]`, so it takes no children",
                    def.name
                ),
            ));
        }
        let mut reduced = Vec::new();
        for child in children {
            reduced.extend(self.expand(child, scope)?);
        }

        let mut body_scope = scope.root().child();
        body_scope.bind("this", Value::Record(this.clone()));
        for field in absent {
            body_scope.mark_absent(field);
        }
        self.types.push(def.key.clone());
        self.children.push(reduced);
        let body = self.expand_children(&def.node, &body_scope);
        self.children.pop();
        self.types.pop();

        let mut out = Expanded::new(&self.path, object.tag.as_deref(), Some(head));
        out.bindings.push(("this".into(), Value::Record(this)));
        out.parts = body?.into_iter().map(Part::Child).collect();
        Ok(out)
    }

    /// A field's value from a literal or one slot.
    fn field_value(
        &self,
        field: &FieldDef,
        text: &Text,
        scope: &Scope<'_>,
    ) -> Result<Value, Error> {
        let mismatch = |message: String| self.error(ErrorKind::SigilMismatch, message);
        match text.parts() {
            [TextPart::Slot(slot)] => {
                let value = self.resolve(slot, scope)?.ok_or_else(|| {
                    self.error(
                        ErrorKind::MissingField,
                        format!("`{}` is left out", slot.expr),
                    )
                    .in_slot(slot)
                })?;
                match &field.kind {
                    FieldType::Core(sigil) => agree(*sigil, &value).map_err(|m| {
                        mismatch(format!("the field `{}`: {m}", field.name)).in_slot(slot)
                    })?,
                    FieldType::Named(name) if self.registry.value_type(name).is_some() => {
                        let text = value_text(&value).ok_or_else(|| {
                            mismatch(format!(
                                "the field `{}` is a `{name}`, but `{}` is {}",
                                field.name,
                                slot.expr,
                                value.kind()
                            ))
                            .in_slot(slot)
                        })?;
                        return self
                            .read_value_type(field, name, &text)
                            .map_err(|e| e.in_slot(slot));
                    }
                    FieldType::Named(name) => {
                        if self.templates.get(name).is_none() {
                            return Err(self.unknown_type(field, name));
                        }
                        if !matches!(value, Value::Record(_)) {
                            return Err(mismatch(format!(
                                "the field `{}` is a `{name}`, but `{}` is {}",
                                field.name,
                                slot.expr,
                                value.kind()
                            ))
                            .in_slot(slot));
                        }
                    }
                }
                Ok(value)
            }
            _ => {
                let literal = text.as_literal().ok_or_else(|| {
                    self.error(
                        ErrorKind::BadValue,
                        format!("the field `{}` takes one literal or one slot", field.name),
                    )
                })?;
                match &field.kind {
                    FieldType::Core(sigil) => read_value(Some(*sigil), literal).map_err(|m| {
                        self.error(
                            ErrorKind::BadValue,
                            format!("the field `{}`: {m}", field.name),
                        )
                    }),
                    FieldType::Named(name) if self.templates.get(name).is_some() => Err(self
                        .error(
                            ErrorKind::BadValue,
                            format!(
                                "the field `{}` is a `{name}`, set with child rows",
                                field.name
                            ),
                        )),
                    FieldType::Named(name) if self.registry.value_type(name).is_some() => {
                        self.read_value_type(field, name, literal)
                    }
                    FieldType::Named(name) => Err(self.unknown_type(field, name)),
                }
            }
        }
    }

    /// Read a field of a value type from text.
    fn read_value_type(&self, field: &FieldDef, name: &str, text: &str) -> Result<Value, Error> {
        let value_type = self
            .registry
            .value_type(name)
            .expect("checked by the caller");
        value_type.read(text, &self.path).map_err(|m| {
            self.error(
                ErrorKind::BadValue,
                format!("the field `{}`: {m}", field.name),
            )
        })
    }

    /// A field set by child rows: a nested instance of a defined type
    /// (members only, checked against its definition), or a `List` field
    /// of objects.
    fn nested_value(
        &self,
        field: &FieldDef,
        member: &Node,
        scope: &Scope<'_>,
    ) -> Result<Value, Error> {
        match &field.kind {
            FieldType::Core(Sigil::List)
                if member.nodes.iter().all(|n| n.as_object().is_some()) =>
            {
                Ok(Value::List(
                    member
                        .nodes
                        .iter()
                        .map(|n| from_object_at(n, &self.path))
                        .collect::<Result<_, _>>()?,
                ))
            }
            FieldType::Named(name) => {
                let def = self
                    .templates
                    .get(name)
                    .ok_or_else(|| self.unknown_type(field, name))?;
                let mut record = Record::new(None);
                let mut given = Vec::new();
                for n in &member.nodes {
                    let m = n.as_member().ok_or_else(|| {
                        self.error(
                            ErrorKind::RecordShape,
                            format!("the field `{}` holds only members", field.name),
                        )
                    })?;
                    let inner = def.field(&m.name).ok_or_else(|| {
                        self.error(
                            ErrorKind::UnknownField,
                            format!("`{name}` has no field `{}`", m.name),
                        )
                    })?;
                    let value = match &m.rhs {
                        Some(text) if n.nodes.is_empty() => self.field_value(inner, text, scope)?,
                        None if !n.nodes.is_empty() => self.nested_value(inner, n, scope)?,
                        _ => {
                            return Err(self
                                .error(ErrorKind::NoValue, format!("`{}` has no value", m.name)))
                        }
                    };
                    given.push(inner.name.clone());
                    record.push(inner.name.clone(), value);
                }
                if let Some(missing) = def
                    .fields
                    .iter()
                    .find(|f| !f.optional && f.name != "name" && !given.contains(&f.name))
                {
                    return Err(self.error(
                        ErrorKind::MissingField,
                        format!(
                            "the field `{}` (a `{name}`) needs `{}`",
                            field.name, missing.name
                        ),
                    ));
                }
                Ok(Value::Record(record))
            }
            FieldType::Core(sigil) => Err(self.error(
                ErrorKind::BadValue,
                format!(
                    "the field `{}` is `[{}]` and takes a value, not child rows",
                    field.name,
                    sigil.token()
                ),
            )),
        }
    }

    fn unknown_type(&self, field: &FieldDef, name: &str) -> Error {
        self.error(
            ErrorKind::UnknownType,
            format!("the field `{}` has the unknown type `{name}`", field.name),
        )
    }

    /// The children of the innermost instance, for `[Children]`.
    fn instance_children(&self) -> Option<&Vec<Expanded>> {
        self.children.last()
    }
}

/// A plain value as text, for a value type to read.
fn value_text(value: &Value) -> Option<String> {
    match value {
        Value::Text(s) | Value::Number(s) | Value::Link(s) | Value::Currency(s) => Some(s.clone()),
        Value::Date(d) => Some(d.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

/// Whether a value may be written with a sigil.
pub(crate) fn agree(sigil: Sigil, value: &Value) -> Result<(), String> {
    let ok = match sigil {
        Sigil::String => matches!(
            value,
            Value::Text(_)
                | Value::Number(_)
                | Value::Date(_)
                | Value::Link(_)
                | Value::Bool(_)
                | Value::Currency(_)
                | Value::Markdown(_)
        ),
        Sigil::Number => matches!(value, Value::Number(_)),
        Sigil::Date => matches!(value, Value::Date(_)),
        Sigil::Link => matches!(value, Value::Link(_)),
        Sigil::Boolean => matches!(value, Value::Bool(_)),
        Sigil::Currency => matches!(value, Value::Currency(_)),
        Sigil::List => matches!(value, Value::List(_)),
    };
    if ok {
        Ok(())
    } else {
        Err(format!("{} is not `[{}]`", value.kind(), sigil.token()))
    }
}

/// `[Children]`: the children of the instance whose body it is in.
struct ChildrenConstruct;

impl Construct for ChildrenConstruct {
    fn tag(&self) -> &str {
        "Children"
    }

    fn reduce(
        &self,
        node: &Node,
        _scope: &Scope<'_>,
        cx: &mut Expander<'_>,
    ) -> Result<Vec<Expanded>, Error> {
        let children = cx.instance_children().cloned().ok_or_else(|| {
            cx.error(
                ErrorKind::Misplaced,
                "`[Children]` belongs in a template's body",
            )
        })?;
        let mut out = Expanded::new(cx.path(), Some("Children"), None);
        if node
            .nodes
            .iter()
            .any(|n| n.as_object().is_some() || n.as_member().is_some())
        {
            return Err(cx.error(ErrorKind::Misplaced, "`[Children]` holds nothing"));
        }
        out.parts = children.into_iter().map(Part::Child).collect();
        Ok(vec![out])
    }
}

/// A tag that refuses where it stands.
struct Refuse {
    tag: &'static str,
    message: &'static str,
}

impl Construct for Refuse {
    fn tag(&self) -> &str {
        self.tag
    }

    fn reduce(
        &self,
        _node: &Node,
        _scope: &Scope<'_>,
        cx: &mut Expander<'_>,
    ) -> Result<Vec<Expanded>, Error> {
        Err(cx.error(ErrorKind::Misplaced, self.message))
    }
}
