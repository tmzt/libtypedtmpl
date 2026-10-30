//! Head constructs and the `Url` value type.
//!
//! `[Meta]`, `[Title]`, `[Stylesheet]`, `[Script]`, `[Icon]` and `[Link]`
//! reduce to elements. They are registered in the scope `head`, so in the
//! default scope a consumer may define its own types with these names.

use std::rc::Rc;

use libtypedmd::{Node, Sigil, Text, TextPart};

use crate::error::{Error, ErrorKind};
use crate::expand::{AttrValue, Construct, Element, Expanded, Expander, Part, Registry};
use crate::path::NodePath;
use crate::scope::Scope;
use crate::template::tag;
use crate::value::Value;

/// Turns a name written in a head entry into a URL. A consumer applies its
/// conventions here (a bare name to a folder and extension).
pub trait AssetUrls {
    fn url(&self, kind: AssetKind, name: &str) -> Result<String, String>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetKind {
    Stylesheet,
    Script,
    Icon,
    Link,
}

impl AssetKind {
    fn key(self) -> &'static str {
        match self {
            AssetKind::Stylesheet => "stylesheet",
            AssetKind::Script => "script",
            AssetKind::Icon => "icon",
            AssetKind::Link => "link",
        }
    }
}

/// `AssetUrls` that returns the name as written.
pub struct AsWritten;

impl AssetUrls for AsWritten {
    fn url(&self, _kind: AssetKind, name: &str) -> Result<String, String> {
        Ok(name.to_string())
    }
}

/// Resolves a URL given as a value. A consumer resolves its own relative
/// URLs, and records references, here.
pub trait UrlResolver {
    fn resolve(&self, url: &str, at: &NodePath) -> Result<String, String>;
}

/// `UrlResolver` that returns the URL unchanged.
pub struct AsGiven;

impl UrlResolver for AsGiven {
    fn resolve(&self, url: &str, _at: &NodePath) -> Result<String, String> {
        Ok(url.to_string())
    }
}

/// The value type `Url`: trimmed, refused if its scheme is `javascript:`,
/// `data:` or `vbscript:`, then resolved; read as a `Link`.
pub struct Url {
    resolver: Rc<dyn UrlResolver>,
}

impl Url {
    pub fn new(resolver: Rc<dyn UrlResolver>) -> Self {
        Self { resolver }
    }
}

impl crate::expand::ValueType for Url {
    fn name(&self) -> &str {
        "Url"
    }

    fn read(&self, text: &str, at: &NodePath) -> Result<Value, String> {
        let url = text.trim();
        if url.is_empty() {
            return Err("an empty URL".into());
        }
        super::checked_link(url).map_err(|e| e.message)?;
        self.resolver.resolve(url, at).map(Value::Link)
    }
}

/// Register the head constructs in the scope `head`, and the value type
/// `Url`.
pub fn register(registry: &mut Registry, assets: Rc<dyn AssetUrls>, urls: Rc<dyn UrlResolver>) {
    registry.register_value_type(Box::new(Url::new(urls)));
    registry.register_in("head", Box::new(Meta));
    registry.register_in("head", Box::new(Title));
    for (tag, kind) in [
        ("Stylesheet", AssetKind::Stylesheet),
        ("Script", AssetKind::Script),
        ("Icon", AssetKind::Icon),
        ("Link", AssetKind::Link),
    ] {
        registry.register_in(
            "head",
            Box::new(Asset {
                tag,
                kind,
                assets: assets.clone(),
            }),
        );
    }
}

/// The identity of a head entry, for merging lists of them: `meta:charset`,
/// `meta:<name>`, `title`, or `<kind>:<url>`; `None` for any other node.
pub fn head_key(node: &Node, assets: &dyn AssetUrls) -> Option<String> {
    let object = node.as_object()?;
    let head = object.head.render();
    let head = head.trim();
    match tag(node) {
        "Meta" if head.is_empty() => {
            let first = node.nodes.iter().find_map(|n| n.as_member())?;
            Some(format!("meta:{}", first.name))
        }
        "Meta" => Some(format!("meta:{head}")),
        "Title" => Some("title".into()),
        t => {
            let kind = match t {
                "Stylesheet" => AssetKind::Stylesheet,
                "Script" => AssetKind::Script,
                "Icon" => AssetKind::Icon,
                "Link" => AssetKind::Link,
                _ => return None,
            };
            let url = assets.url(kind, head).unwrap_or_else(|_| head.to_string());
            Some(format!("{}:{url}", kind.key()))
        }
    }
}

/// An entry's properties as attributes: a literal is text, `true` a flag,
/// `false` no attribute, and a slot its value. `None` for a slot reading an
/// absent optional field.
fn properties(
    node: &Node,
    scope: &Scope<'_>,
    cx: &Expander<'_>,
) -> Result<Vec<(String, Option<AttrValue>)>, Error> {
    let mut out = Vec::new();
    for n in &node.nodes {
        let Some(m) = n.as_member() else {
            return Err(cx.error(
                ErrorKind::StrayNode,
                "a head entry holds only its properties",
            ));
        };
        let text = m
            .rhs
            .as_ref()
            .ok_or_else(|| cx.error(ErrorKind::NoValue, format!("`{}` has no value", m.name)))?;
        let value = match text.parts() {
            [TextPart::Slot(slot)] => cx.resolve(slot, scope)?.map(|value| AttrValue::Value {
                sigil: slot.sigil,
                value,
            }),
            _ => match text.as_literal().map(str::trim) {
                Some("true") => Some(AttrValue::Flag),
                Some("false") => continue,
                Some(literal) => Some(AttrValue::Value {
                    sigil: None,
                    value: Value::Text(literal.to_string()),
                }),
                None => {
                    return Err(cx.error(
                        ErrorKind::BadValue,
                        format!("`{}` takes one literal or one slot", m.name),
                    ))
                }
            },
        };
        out.push((m.name.clone(), value));
    }
    Ok(out)
}

/// A head's text: literals and slot values, as text.
fn head_text(text: &Text, scope: &Scope<'_>, cx: &Expander<'_>) -> Result<String, Error> {
    let mut out = String::new();
    for part in text.parts() {
        match part {
            TextPart::Literal(s) => out.push_str(s),
            TextPart::Slot(slot) => match cx.resolve(slot, scope)? {
                Some(Value::Text(s) | Value::Link(s) | Value::Number(s) | Value::Currency(s)) => {
                    out.push_str(&s)
                }
                Some(Value::Date(d)) => out.push_str(&d.to_string()),
                Some(other) => {
                    return Err(cx
                        .error(
                            ErrorKind::SigilMismatch,
                            format!("{} cannot be a head entry's text", other.kind()),
                        )
                        .in_slot(slot))
                }
                None => {}
            },
        }
    }
    Ok(out.trim().to_string())
}

fn one(cx: &Expander<'_>, tag: &str, element: Element) -> Vec<Expanded> {
    let mut node = Expanded::new(cx.path(), Some(tag), None);
    node.parts.push(Part::Element(element));
    vec![node]
}

fn with_attrs(mut element: Element, attrs: Vec<(String, Option<AttrValue>)>) -> Element {
    for (name, value) in attrs {
        if let Some(value) = value {
            element.attrs.push(crate::expand::Attr { name, value });
        }
    }
    element
}

struct Meta;

impl Construct for Meta {
    fn tag(&self) -> &str {
        "Meta"
    }

    fn reduce(
        &self,
        node: &Node,
        scope: &Scope<'_>,
        cx: &mut Expander<'_>,
    ) -> Result<Vec<Expanded>, Error> {
        let object = node.as_object().expect("constructs get objects");
        let name = head_text(&object.head, scope, cx)?;
        let attrs = properties(node, scope, cx)?;
        let content = attrs.iter().find(|(n, _)| n == "content");
        let empty = match content {
            Some((_, None)) => true,
            Some((
                _,
                Some(AttrValue::Value {
                    value: Value::Text(t),
                    ..
                }),
            )) => t.trim().is_empty(),
            _ => false,
        };
        if empty {
            return Ok(Vec::new());
        }
        let mut element = Element::new("meta");
        if !name.is_empty() {
            element = element.attr("name", None, Value::Text(name));
        }
        Ok(one(cx, "Meta", with_attrs(element, attrs)))
    }
}

struct Title;

impl Construct for Title {
    fn tag(&self) -> &str {
        "Title"
    }

    fn reduce(
        &self,
        node: &Node,
        scope: &Scope<'_>,
        cx: &mut Expander<'_>,
    ) -> Result<Vec<Expanded>, Error> {
        let object = node.as_object().expect("constructs get objects");
        let mut element = Element::new("title");
        element.children.push(Part::Value {
            sigil: Some(Sigil::String),
            value: Value::Text(head_text(&object.head, scope, cx)?),
            slot: libtypedmd::Slot::typed(Sigil::String, "title"),
        });
        Ok(one(cx, "Title", element))
    }
}

struct Asset {
    tag: &'static str,
    kind: AssetKind,
    assets: Rc<dyn AssetUrls>,
}

impl Construct for Asset {
    fn tag(&self) -> &str {
        self.tag
    }

    fn reduce(
        &self,
        node: &Node,
        scope: &Scope<'_>,
        cx: &mut Expander<'_>,
    ) -> Result<Vec<Expanded>, Error> {
        let object = node.as_object().expect("constructs get objects");
        let name = head_text(&object.head, scope, cx)?;
        if name.is_empty() {
            return Err(cx.error(
                ErrorKind::NoValue,
                format!("`[{}]` needs a name or URL", self.tag),
            ));
        }
        let url = self
            .assets
            .url(self.kind, &name)
            .map_err(|m| cx.error(ErrorKind::Consumer, m))?;
        let url = Value::Link(url);
        let element = match self.kind {
            AssetKind::Stylesheet => Element::new("link")
                .attr("rel", None, Value::Text("stylesheet".into()))
                .attr("href", Some(Sigil::Link), url),
            AssetKind::Script => Element::new("script").attr("src", Some(Sigil::Link), url),
            AssetKind::Icon => Element::new("link")
                .attr("rel", None, Value::Text("icon".into()))
                .attr("href", Some(Sigil::Link), url),
            AssetKind::Link => Element::new("link").attr("href", Some(Sigil::Link), url),
        };
        Ok(one(
            cx,
            self.tag,
            with_attrs(element, properties(node, scope, cx)?),
        ))
    }
}
