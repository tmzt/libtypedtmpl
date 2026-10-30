//! `HtmlCollapse`: the HTML collapse. It decides; the writer writes.
//!
//! It tracks where the output stands (text, inside a tag, a quoted
//! attribute value, a comment, or a script's or style's raw text) over the
//! literal text it writes, so a value is written only where escaping makes
//! it safe: in text or a quoted attribute value, and `Html` only in text. It
//! checks each value against its sigil, refuses unsafe link schemes, and
//! formats dates.

mod head;

pub use head::{head_key, register, AsGiven, AsWritten, AssetKind, AssetUrls, Url, UrlResolver};

use libtypedmd::{Node, Sigil};

use crate::collapse::{collapse, Collapse};
use crate::error::{Error, ErrorKind};
use crate::expand::{AttrValue, Element, Expanded, Expander};
use crate::scope::Scope;
use crate::value::{Date, Value};
use crate::writer::{is_void, HtmlWriter};

#[derive(Debug, Clone, PartialEq, Eq)]
enum Position {
    Text,
    /// Inside a tag; the name, lowercase, and whether it is an end tag.
    Tag {
        name: String,
        end: bool,
    },
    Quoted {
        quote: char,
        name: String,
    },
    Comment,
    /// Inside `<script>` or `<style>`, until its end tag.
    Raw(String),
}

pub struct HtmlCollapse {
    position: Position,
    /// How `[31]` values are written.
    dates: Box<dyn Fn(&Date) -> String>,
}

impl Default for HtmlCollapse {
    fn default() -> Self {
        Self::new()
    }
}

impl HtmlCollapse {
    /// Dates written `YYYY-MM-DD`.
    pub fn new() -> Self {
        Self {
            position: Position::Text,
            dates: Box::new(|d| d.to_string()),
        }
    }

    /// How `[31]` values are written.
    pub fn with_dates(mut self, format: impl Fn(&Date) -> String + 'static) -> Self {
        self.dates = Box::new(format);
        self
    }

    /// Advance the position over literal markup.
    fn advance(&mut self, text: &str) {
        let mut i = 0;
        let bytes = text.as_bytes();
        while i < bytes.len() {
            let rest = &text[i..];
            let c = rest.chars().next().expect("in bounds");
            let mut step = c.len_utf8();
            self.position = match std::mem::replace(&mut self.position, Position::Text) {
                Position::Text if rest.starts_with("<!--") => {
                    step = 4;
                    Position::Comment
                }
                Position::Text if c == '<' => {
                    let after = &rest[1..];
                    let end = after.starts_with('/');
                    let name: String = after
                        .trim_start_matches('/')
                        .chars()
                        .take_while(|c| c.is_ascii_alphanumeric())
                        .collect();
                    if name.starts_with(|c: char| c.is_ascii_alphabetic()) {
                        Position::Tag {
                            name: name.to_ascii_lowercase(),
                            end,
                        }
                    } else {
                        Position::Text
                    }
                }
                Position::Text => Position::Text,
                Position::Tag { name, end } => match c {
                    '"' | '\'' => Position::Quoted { quote: c, name },
                    '>' if !end && (name == "script" || name == "style") => Position::Raw(name),
                    '>' => Position::Text,
                    _ => Position::Tag { name, end },
                },
                Position::Quoted { quote, name } if c == quote => {
                    Position::Tag { name, end: false }
                }
                quoted @ Position::Quoted { .. } => quoted,
                Position::Comment if rest.starts_with("-->") => {
                    step = 3;
                    Position::Text
                }
                Position::Comment => Position::Comment,
                Position::Raw(name) => {
                    let close = format!("</{name}");
                    if rest.len() >= close.len() && rest[..close.len()].eq_ignore_ascii_case(&close)
                    {
                        step = close.len();
                        Position::Tag { name, end: true }
                    } else {
                        Position::Raw(name)
                    }
                }
            };
            i += step;
        }
    }

    fn unsafe_position(&self, what: &str) -> Error {
        let place = match &self.position {
            Position::Text => "text",
            Position::Tag { .. } => "a tag or an unquoted attribute value",
            Position::Quoted { .. } => "a quoted attribute value",
            Position::Comment => "a comment",
            Position::Raw(name) => {
                if name == "script" {
                    "a script"
                } else {
                    "a style"
                }
            }
        };
        Error::new(
            ErrorKind::UnsafePosition,
            format!("{what} cannot be written in {place}"),
        )
    }

    /// A value as text, checked against its sigil. `Html` is `None`.
    fn text_of(&self, sigil: Option<Sigil>, value: &Value) -> Result<Option<String>, Error> {
        let mismatch = || {
            Error::new(
                ErrorKind::SigilMismatch,
                format!(
                    "{} cannot be written as `{}`",
                    value.kind(),
                    sigil
                        .map(|s| format!("[{}]", s.token()))
                        .unwrap_or_else(|| "HTML".into())
                ),
            )
        };
        Ok(Some(match (sigil, value) {
            (None, Value::Html(_)) => return Ok(None),
            (
                Some(Sigil::String),
                Value::Text(s) | Value::Number(s) | Value::Currency(s) | Value::Markdown(s),
            ) => s.clone(),
            (Some(Sigil::String), Value::Link(s)) => checked_link(s)?,
            (Some(Sigil::String), Value::Date(d)) => d.to_string(),
            (Some(Sigil::String), Value::Bool(b)) => b.to_string(),
            (Some(Sigil::Number), Value::Number(s))
            | (Some(Sigil::Currency), Value::Currency(s)) => s.clone(),
            (Some(Sigil::Date), Value::Date(d)) => (self.dates)(d),
            (Some(Sigil::Boolean), Value::Bool(b)) => b.to_string(),
            (Some(Sigil::Link), Value::Link(s)) => checked_link(s)?,
            _ => return Err(mismatch()),
        }))
    }
}

/// In an attribute, a value with no sigil is text; only `Html` stays
/// unsigiled, to be refused.
fn attr_sigil(sigil: Option<Sigil>, value: &Value) -> Option<Sigil> {
    match (sigil, value) {
        (None, Value::Html(_)) => None,
        (None, _) => Some(Sigil::String),
        (sigil, _) => sigil,
    }
}

/// A link, refused if its scheme is `javascript:`, `data:` or `vbscript:`
/// (compared without case, after trimming, and with tab, newline and
/// carriage return removed, as browsers do).
pub(crate) fn checked_link(link: &str) -> Result<String, Error> {
    let squeezed: String = link
        .trim()
        .chars()
        .filter(|c| !matches!(c, '\t' | '\n' | '\r'))
        .collect::<String>()
        .to_ascii_lowercase();
    for scheme in ["javascript:", "data:", "vbscript:"] {
        if squeezed.starts_with(scheme) {
            return Err(Error::new(
                ErrorKind::RefusedScheme,
                format!("the link `{link}` uses the refused scheme `{scheme}`"),
            ));
        }
    }
    Ok(link.to_string())
}

impl Collapse for HtmlCollapse {
    type Writer = dyn HtmlWriter;

    fn enter(&mut self, _node: &Expanded, _out: &mut Self::Writer) -> Result<(), Error> {
        Ok(())
    }

    fn literal(&mut self, text: &str, out: &mut Self::Writer) -> Result<(), Error> {
        self.advance(text);
        out.raw(text)
    }

    fn value(
        &mut self,
        sigil: Option<Sigil>,
        value: &Value,
        out: &mut Self::Writer,
    ) -> Result<(), Error> {
        let text = self.text_of(sigil, value)?;
        match (&self.position, text) {
            (Position::Text, Some(text)) | (Position::Quoted { .. }, Some(text)) => out.text(&text),
            (Position::Text, None) => match value {
                Value::Html(html) => out.raw(html.as_str()),
                _ => unreachable!("only Html has no text"),
            },
            (_, None) => Err(self.unsafe_position("HTML")),
            (_, Some(_)) => Err(self.unsafe_position("a value")),
        }
    }

    fn start_element(&mut self, element: &Element, out: &mut Self::Writer) -> Result<(), Error> {
        if self.position != Position::Text {
            return Err(self.unsafe_position(&format!("the element `<{}>`", element.name)));
        }
        if is_void(&element.name) && !element.children.is_empty() {
            return Err(Error::new(
                ErrorKind::VoidChildren,
                format!(
                    "`<{}>` is a void element and takes no children",
                    element.name
                ),
            ));
        }
        let mut values = Vec::new();
        for attr in &element.attrs {
            values.push(match &attr.value {
                AttrValue::Flag => None,
                AttrValue::Value { sigil, value } => Some(
                    self.text_of(attr_sigil(*sigil, value), value)?
                        .ok_or_else(|| {
                            Error::new(
                                ErrorKind::UnsafePosition,
                                format!("HTML cannot be written in the attribute `{}`", attr.name),
                            )
                        })?,
                ),
            });
        }
        let attrs: Vec<(&str, Option<&str>)> = element
            .attrs
            .iter()
            .zip(&values)
            .map(|(a, v)| (a.name.as_str(), v.as_deref()))
            .collect();
        out.start(&element.name, &attrs)?;
        let name = element.name.to_ascii_lowercase();
        if name == "script" || name == "style" {
            self.position = Position::Raw(name);
        }
        Ok(())
    }

    fn end_element(&mut self, element: &Element, out: &mut Self::Writer) -> Result<(), Error> {
        self.position = Position::Text;
        out.end(&element.name)
    }

    fn leave(&mut self, _node: &Expanded, _out: &mut Self::Writer) -> Result<(), Error> {
        Ok(())
    }
}

/// Expand `node` and collapse it to HTML: the two stages in one call.
pub fn render_html(
    node: &Node,
    scope: &Scope<'_>,
    cx: &mut Expander<'_>,
    html: &mut HtmlCollapse,
    out: &mut (dyn HtmlWriter + 'static),
) -> Result<(), Error> {
    let trees = cx.expand(node, scope)?;
    collapse(&trees, html, out)
}
