//! `HtmlWriter`: where HTML output goes. Text and attribute values are
//! escaped here; raw markup is written as it is.

use std::io;

use crate::error::{Error, ErrorKind};

/// Elements that have no end tag and take no children.
pub const VOID: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source", "track",
    "wbr",
];

/// Whether `name` is a void element (case-insensitive).
pub fn is_void(name: &str) -> bool {
    VOID.iter().any(|v| v.eq_ignore_ascii_case(name))
}

/// Writes HTML.
pub trait HtmlWriter {
    /// `<name attr="value" flag>`; an attribute with `None` is a flag.
    fn start(&mut self, name: &str, attrs: &[(&str, Option<&str>)]) -> Result<(), Error>;
    /// `</name>`; nothing for a void element.
    fn end(&mut self, name: &str) -> Result<(), Error>;
    /// Text, escaped.
    fn text(&mut self, text: &str) -> Result<(), Error>;
    /// Markup, as it is.
    fn raw(&mut self, html: &str) -> Result<(), Error>;
}

/// `&`, `<`, `>`, `"` and `'` as entities.
pub fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c => out.push(c),
        }
    }
    out
}

fn start_tag(name: &str, attrs: &[(&str, Option<&str>)]) -> String {
    let mut out = format!("<{name}");
    for (attr, value) in attrs {
        match value {
            Some(v) => out.push_str(&format!(" {attr}=\"{}\"", escape(v))),
            None => out.push_str(&format!(" {attr}")),
        }
    }
    out.push('>');
    out
}

impl HtmlWriter for String {
    fn start(&mut self, name: &str, attrs: &[(&str, Option<&str>)]) -> Result<(), Error> {
        self.push_str(&start_tag(name, attrs));
        Ok(())
    }

    fn end(&mut self, name: &str) -> Result<(), Error> {
        if !is_void(name) {
            self.push_str(&format!("</{name}>"));
        }
        Ok(())
    }

    fn text(&mut self, text: &str) -> Result<(), Error> {
        self.push_str(&escape(text));
        Ok(())
    }

    fn raw(&mut self, html: &str) -> Result<(), Error> {
        self.push_str(html);
        Ok(())
    }
}

/// An `HtmlWriter` over any `io::Write`: a file, a socket, a buffer.
pub struct IoHtmlWriter<W: io::Write>(pub W);

impl<W: io::Write> IoHtmlWriter<W> {
    fn write(&mut self, s: &str) -> Result<(), Error> {
        self.0
            .write_all(s.as_bytes())
            .map_err(|e| Error::new(ErrorKind::Write, e.to_string()))
    }
}

impl<W: io::Write> HtmlWriter for IoHtmlWriter<W> {
    fn start(&mut self, name: &str, attrs: &[(&str, Option<&str>)]) -> Result<(), Error> {
        self.write(&start_tag(name, attrs))
    }

    fn end(&mut self, name: &str) -> Result<(), Error> {
        if is_void(name) {
            return Ok(());
        }
        self.write(&format!("</{name}>"))
    }

    fn text(&mut self, text: &str) -> Result<(), Error> {
        self.write(&escape(text))
    }

    fn raw(&mut self, html: &str) -> Result<(), Error> {
        self.write(html)
    }
}
