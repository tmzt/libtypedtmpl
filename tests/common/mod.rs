//! Shared test setup: parse a document strictly, collect its templates,
//! bind its data sections, and expand one section.
#![allow(dead_code)]

use libtypedmd::{parse_with, Node, ParseOptions, Stream};
use libtypedtmpl::{
    Error, Expanded, Expander, NoSources, Part, Record, Registry, Scope, Sources, Templates, Value,
};

pub fn parse(source: &str) -> Stream {
    parse_with(source, &ParseOptions::strict()).expect("the test document parses")
}

/// The top-level section with this tag and head.
pub fn section<'s>(stream: &'s Stream, tag: &str, head: &str) -> &'s Node {
    stream.documents[0]
        .nodes
        .iter()
        .find(|n| {
            n.as_object()
                .is_some_and(|o| o.tag.as_deref() == Some(tag) && o.head.render().trim() == head)
        })
        .unwrap_or_else(|| panic!("no [{tag}] {head}"))
}

/// Expand the children of `[tag] head`, with every `[Data]`-like section
/// listed in `data` bound by its head, and `sources` for list expressions.
pub fn expand_with(
    source: &str,
    tag: &str,
    head: &str,
    data: &[(&str, &str)],
    sources: &dyn Sources,
) -> Result<Expanded, Error> {
    let stream = parse(source);
    let templates = Templates::from_stream(&stream)?;
    let registry = Registry::new();
    let mut scope = Scope::new();
    for (data_tag, data_head) in data {
        let record = Record::from_object(section(&stream, data_tag, data_head))?;
        scope.bind(*data_head, Value::Record(record));
    }
    scope.bind(
        "site",
        Value::Record({
            let mut r = Record::new(Some("example".into()));
            r.push("title", Value::Text("Example".into()));
            r
        }),
    );
    let node = section(&stream, tag, head);
    let mut cx = Expander::new(&registry, &templates, sources);
    let parts = cx.within(node, None, |cx| cx.expand_children(node, &scope))?;
    let mut root = Expanded {
        path: Default::default(),
        tag: Some(tag.into()),
        name: Some(head.into()),
        bindings: Vec::new(),
        parts: parts.into_iter().map(Part::Child).collect(),
    };
    root.path = Default::default();
    Ok(root)
}

pub fn expand(source: &str, tag: &str, head: &str) -> Expanded {
    expand_with(source, tag, head, &[], &NoSources).unwrap()
}

pub fn expand_err(source: &str, tag: &str, head: &str) -> Error {
    match expand_with(source, tag, head, &[], &NoSources) {
        Ok(e) => panic!("expected an error, got:\n{e}"),
        Err(e) => e,
    }
}

/// All literal text in the tree, in order, concatenated, with values
/// written plainly: enough to check what a tree holds without a collapse.
pub fn text(e: &Expanded) -> String {
    let mut out = String::new();
    for part in &e.parts {
        match part {
            Part::Literal(s) => out.push_str(s),
            Part::Value { value, .. } => match value {
                Value::Text(s) | Value::Number(s) | Value::Link(s) | Value::Currency(s) => {
                    out.push_str(s)
                }
                Value::Html(h) => out.push_str(h.as_str()),
                Value::Markdown(m) => out.push_str(m),
                Value::Date(d) => out.push_str(&d.to_string()),
                Value::Bool(b) => out.push_str(&b.to_string()),
                other => out.push_str(&format!("{other:?}")),
            },
            Part::Child(c) => out.push_str(&text(c)),
        }
    }
    out
}
