//! Defined types: a `[Template]` is a definition, an object whose members'
//! right-hand sides are types; an object with its tag is an instance.

use libtypedmd::{LeafForm, Node, NodeKind, Sigil, Stream};

use crate::error::{Error, ErrorKind};
use crate::path::{is_name, NodePath, Segment};

/// The tags the walk gives meaning to; no type may take them.
const RESERVED: &[&str] = &[
    "content", "children", "list", "item", "empty", "head", "template",
];

/// Core types by the names a definition's right-hand side may use.
const CORE_TYPES: &[(&str, Sigil)] = &[
    ("String", Sigil::String),
    ("Number", Sigil::Number),
    ("List", Sigil::List),
    ("Link", Sigil::Link),
    ("Date", Sigil::Date),
    ("Boolean", Sigil::Boolean),
    ("Currency", Sigil::Currency),
];

/// A type name or tag, compared case-insensitively without `_` and `-`.
pub fn normalise(name: &str) -> String {
    name.chars()
        .filter(|c| *c != '_' && *c != '-')
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

/// A defined type.
#[derive(Debug, Clone, PartialEq)]
pub struct TypeDef {
    /// The template's head, such as `StoryCard`.
    pub name: String,
    /// The name normalised.
    pub key: String,
    /// In declaration order.
    pub fields: Vec<FieldDef>,
    pub(crate) node: Node,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldDef {
    pub name: String,
    pub kind: FieldType,
    pub optional: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldType {
    /// A core type, by sigil or by name.
    Core(Sigil),
    /// Any other named type: a value type or another defined type, resolved
    /// when an instance is read.
    Named(String),
}

impl TypeDef {
    pub fn field(&self, name: &str) -> Option<&FieldDef> {
        self.fields.iter().find(|f| f.name == name)
    }

    /// The body: the definition's objects.
    pub(crate) fn body(&self) -> impl Iterator<Item = &Node> {
        self.node.nodes.iter().filter(|n| n.as_object().is_some())
    }

    /// Whether the body places an instance's children.
    pub(crate) fn places_children(&self) -> bool {
        self.body().any(|n| tag(n) == "Children")
    }
}

/// The defined types, by normalised name.
#[derive(Debug, Clone, Default)]
pub struct Templates {
    types: Vec<TypeDef>,
}

impl Templates {
    pub fn new() -> Self {
        Self::default()
    }

    /// Every top-level `[Template]` in every document of `stream`.
    pub fn from_stream(stream: &Stream) -> Result<Self, Error> {
        let mut templates = Self::new();
        for document in &stream.documents {
            for node in document.nodes.iter().filter(|n| tag(n) == "Template") {
                templates.add(node)?;
            }
        }
        Ok(templates)
    }

    /// Add one `[Template]` definition.
    pub fn add(&mut self, node: &Node) -> Result<(), Error> {
        let def = definition(node)?;
        if self.types.iter().any(|t| t.key == def.key) {
            return Err(Error::new(
                ErrorKind::DuplicateTemplate,
                format!("the type `{}` is defined twice", def.name),
            )
            .at(NodePath::new().child(Segment::of(node))));
        }
        self.types.push(def);
        Ok(())
    }

    /// The type a tag names, compared without case, `_` or `-`.
    pub fn get(&self, tag: &str) -> Option<&TypeDef> {
        let key = normalise(tag);
        self.types.iter().find(|t| t.key == key)
    }

    pub fn types(&self) -> &[TypeDef] {
        &self.types
    }
}

pub(crate) fn tag(node: &Node) -> &str {
    node.as_object()
        .and_then(|o| o.tag.as_deref())
        .unwrap_or("")
}

fn definition(node: &Node) -> Result<TypeDef, Error> {
    let path = NodePath::new().child(Segment::of(node));
    let bad = |message: String| Error::new(ErrorKind::BadDefinition, message).at(path.clone());
    let Some(object) = node.as_object().filter(|_| tag(node) == "Template") else {
        return Err(bad("a definition is a `[Template]` object".into()));
    };
    let name = object.head.as_literal().map(str::trim).unwrap_or("");
    if !(name.starts_with(|c: char| c.is_ascii_alphabetic()) && is_name(name)) {
        return Err(bad(format!(
            "the head is the type it defines and must be a name, not `{}`",
            object.head.render()
        )));
    }
    let key = normalise(name);
    if RESERVED.contains(&key.as_str()) {
        return Err(bad(format!(
            "the type `{name}` would redefine a built-in tag"
        )));
    }
    let mut fields: Vec<FieldDef> = Vec::new();
    for n in &node.nodes {
        match &n.kind {
            NodeKind::Member(m) => {
                let field = field(m).map_err(&bad)?;
                if fields.iter().any(|f| f.name == field.name) {
                    return Err(bad(format!("the field `{}` is declared twice", field.name)));
                }
                fields.push(field);
            }
            NodeKind::Object(_) if tag(n).ends_with('?') => {
                return Err(bad(format!(
                    "`[{}]`: optional goes on the field's name, as `- [{}] name?`",
                    tag(n),
                    tag(n).trim_end_matches('?')
                )))
            }
            NodeKind::Leaf(leaf) if leaf.form == LeafForm::Row => {
                return Err(bad(format!(
                    "the row `{}` is neither a field nor an object",
                    leaf.text
                )))
            }
            _ => {}
        }
    }
    let def = TypeDef {
        name: name.to_string(),
        key,
        fields,
        node: node.clone(),
    };
    if def.body().next().is_none() {
        return Err(bad("has no body: a `[Content]` or other objects".into()));
    }
    if def.body().filter(|n| tag(n) == "Children").count() > 1 {
        return Err(bad("more than one `[Children]`".into()));
    }
    Ok(def)
}

/// `- [abc] text`, `- title?: String`, `- cover: Image`.
fn field(m: &libtypedmd::Member) -> Result<FieldDef, String> {
    let (name, optional) = match m.name.strip_suffix('?') {
        Some(name) => (name.trim().to_string(), true),
        None => (m.name.trim().to_string(), false),
    };
    if matches!(name.as_str(), "type" | "head") {
        return Err(format!(
            "`{name}` would be a field: the type is the template's head, and an instance's head fills the field `name`"
        ));
    }
    if !is_name(&name) {
        return Err(format!("the field `{}` needs a plain name", m.name));
    }
    let kind = match (m.sigil, m.rhs.as_ref()) {
        (Some(sigil), None) => FieldType::Core(sigil),
        (Some(_), Some(value)) => {
            return Err(format!(
                "the field `{name}` has the value `{}`; a definition declares types, not values",
                value.render()
            ))
        }
        (None, None) => return Err(format!("the field `{name}` needs a type")),
        (None, Some(text)) => {
            let type_name = text
                .as_literal()
                .map(str::trim)
                .ok_or_else(|| format!("the field `{name}`'s type may not be a slot"))?;
            if let Some(inner) = type_name.strip_suffix('?') {
                return Err(format!(
                    "`{name}: {type_name}`: optional goes on the field's name, as `{name}?: {inner}`"
                ));
            }
            match CORE_TYPES.iter().find(|(n, _)| *n == type_name) {
                Some((_, sigil)) => FieldType::Core(*sigil),
                None if is_name(type_name) => FieldType::Named(type_name.to_string()),
                None => {
                    return Err(format!(
                        "the field `{name}`'s type `{type_name}` is not a type name"
                    ))
                }
            }
        }
    };
    Ok(FieldDef {
        name,
        kind,
        optional,
    })
}
