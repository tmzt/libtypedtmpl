//! The built-in rendering of `ImageRef` records: `<img src alt>`.

use libtypedmd::Sigil;

use crate::error::{Error, ErrorKind};
use crate::expand::{Element, Expander, Part, Render};
use crate::record::Record;
use crate::value::Value;

/// Draws an `ImageRef` record as `<img src="…" alt="…">`: `src` is the
/// record's `src`, written as a link; `alt` its `alt` as text, or empty.
pub struct ImageRef;

impl Render for ImageRef {
    fn type_name(&self) -> &str {
        "ImageRef"
    }

    fn render(&self, record: &Record, cx: &Expander<'_>) -> Result<Vec<Part>, Error> {
        let src = match record.get("src") {
            Some(value @ Value::Link(_)) => value.clone(),
            Some(Value::Text(text)) => Value::Link(text.clone()),
            _ => {
                return Err(cx.error(
                    ErrorKind::MissingField,
                    "an `ImageRef` record needs a `src` link",
                ))
            }
        };
        let alt = match record.get("alt") {
            Some(Value::Text(text)) => text.clone(),
            _ => String::new(),
        };
        Ok(vec![Part::Element(
            Element::new("img")
                .attr("src", Some(Sigil::Link), src)
                .attr("alt", Some(Sigil::String), Value::Text(alt)),
        )])
    }
}
