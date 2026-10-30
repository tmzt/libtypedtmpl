//! Value types: `Url` and a consumer's `UrlResolver`.

mod common;
use common::*;
use std::cell::RefCell;
use std::rc::Rc;

use libtypedtmpl::html::{self, AsGiven, AsWritten, UrlResolver};
use libtypedtmpl::{
    Error, ErrorKind, Expander, NoSources, NodePath, Registry, Scope, Templates, Value, ValueType,
};

const NAV: &str = "# [Template] NavLink\n- name: Url\n- [abc] text\n- [Content]\n  ```html\n  <a href=\"{{ [-] this.name }}\">{{ [abc] this.text }}</a>\n  ```\n\n";

fn run(body: &str, urls: Rc<dyn UrlResolver>) -> Result<String, Error> {
    let source = format!("{NAV}# [Section] s\n{body}");
    let stream = parse(&source);
    let templates = Templates::from_stream(&stream)?;
    let mut registry = Registry::new();
    html::register(&mut registry, Rc::new(AsWritten), urls);
    let mut cx = Expander::new(&registry, &templates, &NoSources);
    let trees = cx.expand_children(section(&stream, "Section", "s"), &Scope::new())?;
    Ok(trees.iter().map(text).collect())
}

/// Resolves `~/x` to `/x`, refuses `/missing`, and records every URL.
struct Recording(RefCell<Vec<String>>);
impl UrlResolver for Recording {
    fn resolve(&self, url: &str, at: &NodePath) -> Result<String, String> {
        self.0.borrow_mut().push(format!("{url} at {at}"));
        match url {
            "/missing/" => Err("no page at /missing/".into()),
            u if u.starts_with("~/") => Ok(u[1..].to_string()),
            u => Ok(u.to_string()),
        }
    }
}

#[test]
fn a_url_field_from_the_head() {
    assert_eq!(
        run("- [NavLink] /about/ text=About\n", Rc::new(AsGiven)).unwrap(),
        "<a href=\"/about/\">About</a>"
    );
}

#[test]
fn the_resolver_rewrites_records_and_refuses() {
    let recorder = Rc::new(Recording(RefCell::new(Vec::new())));
    let out = run("- [NavLink] ~/stories/ text=Stories\n", recorder.clone()).unwrap();
    assert_eq!(out, "<a href=\"/stories/\">Stories</a>");
    assert_eq!(
        recorder.0.borrow().as_slice(),
        ["~/stories/ at [NavLink] ~/stories/"]
    );
    let e = run("- [NavLink] /missing/ text=Gone\n", recorder).unwrap_err();
    assert_eq!(e.kind, ErrorKind::BadValue);
    assert!(e.message.contains("no page at /missing/"), "{e}");
}

#[test]
fn refused_schemes_and_empty_urls() {
    for url in ["javascript:alert(1)", "DATA:text/html,x", "vbscript:x"] {
        let e = run(&format!("- [NavLink] {url} text=X\n"), Rc::new(AsGiven)).unwrap_err();
        assert_eq!(e.kind, ErrorKind::BadValue, "{url}");
        assert!(e.message.contains("refused scheme"), "{url}: {e}");
    }
    let url = html::Url::new(Rc::new(AsGiven));
    assert_eq!(url.read("  ", &NodePath::new()), Err("an empty URL".into()));
    assert_eq!(
        url.read(" /a ", &NodePath::new()),
        Ok(Value::Link("/a".into()))
    );
}

#[test]
fn a_url_from_a_slot() {
    let source = "- [NavLink] text=Home\n  - name: {{ [abc] site.home }}\n";
    let stream = parse(&format!("{NAV}# [Section] s\n{source}"));
    let templates = Templates::from_stream(&stream).unwrap();
    let mut registry = Registry::new();
    html::register(&mut registry, Rc::new(AsWritten), Rc::new(AsGiven));
    let mut scope = Scope::new();
    let mut site = libtypedtmpl::Record::new(None);
    site.push("home", Value::Text("/".into()));
    scope.bind("site", Value::Record(site));
    let mut cx = Expander::new(&registry, &templates, &NoSources);
    let trees = cx
        .expand_children(section(&stream, "Section", "s"), &scope)
        .unwrap();
    assert_eq!(text(&trees[0]), "<a href=\"/\">Home</a>");
}

#[test]
fn a_value_type_written_as_an_object_gets_a_hint() {
    let e = run("- [Url] home\n", Rc::new(AsGiven)).unwrap_err();
    assert_eq!(e.kind, ErrorKind::UnknownType);
    assert!(
        e.message.contains("declare the field as `- home: Url`"),
        "{e}"
    );
}

#[test]
fn without_the_value_type_a_url_field_is_unknown() {
    let source = format!("{NAV}# [Section] s\n- [NavLink] /a/ text=A\n");
    let stream = parse(&source);
    let templates = Templates::from_stream(&stream).unwrap();
    let registry = Registry::new();
    let mut cx = Expander::new(&registry, &templates, &NoSources);
    let e = cx
        .expand_children(section(&stream, "Section", "s"), &Scope::new())
        .unwrap_err();
    assert_eq!(e.kind, ErrorKind::UnknownType);
}
