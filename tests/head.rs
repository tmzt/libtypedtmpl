//! Head constructs in the `head` scope, `AssetUrls` and `head_key`.

mod common;
use common::*;
use std::rc::Rc;

use libtypedtmpl::html::{self, head_key, AsGiven, AsWritten, AssetKind, AssetUrls};
use libtypedtmpl::{
    collapse, Error, ErrorKind, Expander, HtmlCollapse, NoSources, Record, Registry, Scope,
    Templates, Value,
};

/// A site's convention: bare names to folders and extensions.
struct Conventions;
impl AssetUrls for Conventions {
    fn url(&self, kind: AssetKind, name: &str) -> Result<String, String> {
        if name.starts_with('/') {
            return Ok(name.to_string());
        }
        match kind {
            AssetKind::Stylesheet if name == "missing" => Err("no styles/missing.css".into()),
            AssetKind::Stylesheet => Ok(format!("/styles/{name}.css")),
            AssetKind::Script => Ok(format!("/scripts/{name}.js")),
            AssetKind::Icon => Ok(format!("/images/{name}")),
            AssetKind::Link => Ok(name.to_string()),
        }
    }
}

fn head(entries: &str, description: &str) -> Result<String, Error> {
    let source = format!("# [Head] base\n{entries}");
    let stream = parse(&source);
    let templates = Templates::from_stream(&stream).unwrap();
    let mut registry = Registry::new();
    html::register(&mut registry, Rc::new(Conventions), Rc::new(AsGiven));
    let mut scope = Scope::new();
    let mut page = Record::new(None);
    page.push("title", Value::Text("Ports & phones".into()));
    page.push("description", Value::Text(description.into()));
    scope.bind("page", Value::Record(page));
    let node = section(&stream, "Head", "base");
    let mut cx = Expander::new(&registry, &templates, &NoSources);
    let trees = cx.with_scope("head", |cx| cx.expand_children(node, &scope))?;
    let mut out = String::new();
    collapse(&trees, &mut HtmlCollapse::new(), &mut out)?;
    Ok(out)
}

#[test]
fn each_construct() {
    let out = head(
        concat!(
            "- [Meta] charset=utf-8\n",
            "- [Meta] viewport content=\"width=device-width, initial-scale=1\"\n",
            "- [Title] {{ [abc] page.title }}\n",
            "- [Meta] description content={{ [abc] page.description }}\n",
            "- [Stylesheet] site\n",
            "- [Script] app defer=true async=false type=module\n",
            "- [Icon] icon.png sizes=512x512\n",
            "- [Link] /humans.txt rel=author\n",
        ),
        "About & more",
    )
    .unwrap();
    assert_eq!(
        out,
        concat!(
            "<meta charset=\"utf-8\">",
            "<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">",
            "<title>Ports &amp; phones</title>",
            "<meta name=\"description\" content=\"About &amp; more\">",
            "<link rel=\"stylesheet\" href=\"/styles/site.css\">",
            "<script src=\"/scripts/app.js\" defer type=\"module\"></script>",
            "<link rel=\"icon\" href=\"/images/icon.png\" sizes=\"512x512\">",
            "<link href=\"/humans.txt\" rel=\"author\">",
        )
    );
}

#[test]
fn an_empty_content_drops_its_meta() {
    let out = head(
        "- [Meta] description content={{ [abc] page.description }}\n",
        "",
    )
    .unwrap();
    assert_eq!(out, "");
}

#[test]
fn a_url_name_is_taken_as_written_and_errors_come_from_the_convention() {
    assert_eq!(
        head("- [Stylesheet] /theme.css\n", "").unwrap(),
        "<link rel=\"stylesheet\" href=\"/theme.css\">"
    );
    let e = head("- [Stylesheet] missing\n", "").unwrap_err();
    assert_eq!(
        (e.kind, e.message.as_str()),
        (ErrorKind::Consumer, "no styles/missing.css")
    );
}

#[test]
fn head_constructs_live_only_in_the_head_scope() {
    let stream = parse("# [Section] s\n- [Meta] charset=utf-8\n");
    let templates = Templates::from_stream(&stream).unwrap();
    let mut registry = Registry::new();
    html::register(&mut registry, Rc::new(AsWritten), Rc::new(AsGiven));
    let node = section(&stream, "Section", "s");
    let mut cx = Expander::new(&registry, &templates, &NoSources);
    let e = cx.expand_children(node, &Scope::new()).unwrap_err();
    assert_eq!(
        e.kind,
        ErrorKind::UnknownType,
        "outside `head`, [Meta] is not a construct"
    );
}

#[test]
fn a_site_may_define_its_own_link_type_outside_the_head() {
    let source = "# [Template] Link\n- name: String\n- [Content]\n  ```html\n  <a>{{ [abc] this.name }}</a>\n  ```\n\n# [Section] s\n- [Link] About\n";
    let stream = parse(source);
    let templates = Templates::from_stream(&stream).unwrap();
    let mut registry = Registry::new();
    html::register(&mut registry, Rc::new(AsWritten), Rc::new(AsGiven));
    let mut cx = Expander::new(&registry, &templates, &NoSources);
    let trees = cx
        .expand_children(section(&stream, "Section", "s"), &Scope::new())
        .unwrap();
    assert_eq!(text(&trees[0]), "<a>About</a>");
}

#[test]
fn head_keys() {
    let stream = parse(concat!(
        "# [Head] h\n",
        "- [Meta] charset=utf-8\n",
        "- [Meta] viewport content=x\n",
        "- [Title] T\n",
        "- [Stylesheet] site\n",
        "- [Script] app\n",
        "- [Icon] icon.png\n",
        "- [Link] /humans.txt rel=author\n",
        "- [Other] x\n",
    ));
    let keys: Vec<Option<String>> = section(&stream, "Head", "h")
        .nodes
        .iter()
        .map(|n| head_key(n, &Conventions))
        .collect();
    assert_eq!(
        keys,
        [
            Some("meta:charset".into()),
            Some("meta:viewport".into()),
            Some("title".into()),
            Some("stylesheet:/styles/site.css".into()),
            Some("script:/scripts/app.js".into()),
            Some("icon:/images/icon.png".into()),
            Some("link:/humans.txt".into()),
            None,
        ]
    );
}
