//! Typed records drawn by a template for their type or a registered
//! renderer, and absent fields.

mod common;
use common::*;
use std::rc::Rc;

use libtypedtmpl::html::{register, AsGiven, AsWritten};
use libtypedtmpl::{
    collapse, ErrorKind, Expander, HtmlCollapse, NoSources, Record, Registry, Scope, Templates,
    Value,
};

/// `story` with a cover, `bare` without one (its `cover` absent).
fn stories() -> (Record, Record) {
    let cover = Record::new(None).with_type("ImageRef");
    let mut cover = cover;
    cover.push("src", Value::Link("/stories/a/cover.jpg".into()));
    let mut meta = Record::new(None);
    meta.push("title", Value::Text("A".into()));
    meta.push("cover", Value::Record(cover));
    let mut story = Record::new(Some("a".into()));
    story.push("meta", Value::Record(meta));

    let mut meta = Record::new(None);
    meta.push("title", Value::Text("B".into()));
    meta.mark_absent("cover");
    let mut bare = Record::new(Some("b".into()));
    bare.push("meta", Value::Record(meta));
    (story, bare)
}

/// Render `[Page] p` of `source` to HTML, with `story` and `bare` bound.
fn html(source: &str) -> Result<String, libtypedtmpl::Error> {
    let stream = parse(source);
    let templates = Templates::from_stream(&stream)?;
    let mut registry = Registry::new();
    register(&mut registry, Rc::new(AsWritten), Rc::new(AsGiven));
    let (story, bare) = stories();
    let mut scope = Scope::new();
    scope.bind("story", Value::Record(story));
    scope.bind("bare", Value::Record(bare));
    let node = section(&stream, "Page", "p");
    let mut cx = Expander::new(&registry, &templates, &NoSources);
    let trees = cx.expand_children(node, &scope)?;
    let mut out = String::new();
    collapse(&trees, &mut HtmlCollapse::new(), &mut out)?;
    Ok(out)
}

fn content(body: &str) -> String {
    format!("- [Content]\n  ```html\n  {body}\n  ```\n")
}

fn page(templates: &str, body: &str) -> String {
    format!("{templates}# [Page] p\n{body}")
}

#[test]
fn a_typed_record_draws_itself_and_an_absent_one_writes_nothing() {
    let out = html(&page(
        "",
        &content("<li>{{ story.meta.cover }}A</li><li>{{ bare.meta.cover }}B</li>"),
    ))
    .unwrap();
    assert_eq!(
        out,
        r#"<li><img src="/stories/a/cover.jpg" alt="">A</li><li>B</li>"#
    );
}

#[test]
fn an_absent_field_on_the_way_writes_nothing_and_a_missing_one_still_errors() {
    let out = html(&page("", &content("[{{ [-] bare.meta.cover.src }}]"))).unwrap();
    assert_eq!(out, "[]");
    let e = html(&page("", &content("{{ [abc] bare.meta.covr }}"))).unwrap_err();
    assert_eq!(e.kind, ErrorKind::UnknownField, "{e}");
}

const OWN: &str = concat!(
    "# [Template] ImageRef\n",
    "- [-] src\n",
    "- [abc] alt?\n",
    "- [Content]\n",
    "  ```html\n",
    "  <img class=\"cover\" loading=\"lazy\" src=\"{{ [-] this.src }}\" alt=\"{{ [abc] this.alt }}\">\n",
    "  ```\n",
    "\n",
);

#[test]
fn a_site_template_for_the_type_replaces_the_built_in() {
    let out = html(&page(
        OWN,
        &content("{{ story.meta.cover }}|{{ bare.meta.cover }}"),
    ))
    .unwrap();
    assert_eq!(
        out,
        r#"<img class="cover" loading="lazy" src="/stories/a/cover.jpg" alt="">|"#
    );
}

const CARD: &str = concat!(
    "# [Template] Card\n",
    "- [abc] title\n",
    "- cover?: ImageRef\n",
    "- [Content]\n",
    "  ```html\n",
    "  <h2>{{ [abc] this.title }}</h2>{{ this.cover }}\n",
    "  ```\n",
    "\n",
);

#[test]
fn an_optional_field_takes_a_typed_record_or_is_left_out_when_absent() {
    let body = concat!(
        "- [Card] a title={{ [abc] story.meta.title }} cover={{ story.meta.cover }}\n",
        "- [Card] b title={{ [abc] bare.meta.title }} cover={{ bare.meta.cover }}\n",
    );
    let out = html(&page(CARD, body)).unwrap();
    assert_eq!(
        out,
        r#"<h2>A</h2><img src="/stories/a/cover.jpg" alt=""><h2>B</h2>"#
    );
}

#[test]
fn a_required_field_given_an_absent_value_is_an_error() {
    let card = CARD.replace("- cover?: ImageRef", "- cover: ImageRef");
    let e = html(&page(
        &card,
        "- [Card] b title={{ [abc] bare.meta.title }} cover={{ bare.meta.cover }}\n",
    ))
    .unwrap_err();
    assert_eq!(e.kind, ErrorKind::MissingField, "{e}");
}

#[test]
fn a_list_draws_a_typed_record_once_and_an_absent_one_as_empty() {
    let list = |who: &str| {
        format!(
            "- [List] c source={{{{ [1.] {who}.meta.cover }}}} item_alias=c\n  - [Item]\n{}  - [Empty]\n{}",
            "    - [Content]\n      ```html\n      <i>{{ [-] c.src }}</i>\n      ```\n",
            "    - [Content]\n      ```html\n      <i>none</i>\n      ```\n",
        )
    };
    assert_eq!(
        html(&page("", &list("story"))).unwrap(),
        "<i>/stories/a/cover.jpg</i>"
    );
    assert_eq!(html(&page("", &list("bare"))).unwrap(), "<i>none</i>");
}

#[test]
fn a_typed_record_without_a_template_or_renderer_is_an_error() {
    let stream_source = page("", &content("{{ story.meta.cover }}"));
    let stream = parse(&stream_source);
    let templates = Templates::from_stream(&stream).unwrap();
    let registry = Registry::new(); // no html::register: no ImageRef renderer
    let (story, _) = stories();
    let mut scope = Scope::new();
    scope.bind("story", Value::Record(story));
    let node = section(&stream, "Page", "p");
    let mut cx = Expander::new(&registry, &templates, &NoSources);
    let e = cx.expand_children(node, &scope).unwrap_err();
    assert!(
        e.message
            .contains("`story.meta.cover` is a `ImageRef` record; define `[Template] ImageRef`"),
        "{e}"
    );
}

#[test]
fn image_src_is_escaped_and_refused_schemes_are_refused() {
    let mut cover = Record::new(None).with_type("ImageRef");
    cover.push("src", Value::Link("javascript:alert(1)".into()));
    let source = page("", &content("{{ c }}"));
    let stream = parse(&source);
    let templates = Templates::from_stream(&stream).unwrap();
    let mut registry = Registry::new();
    register(&mut registry, Rc::new(AsWritten), Rc::new(AsGiven));
    let mut scope = Scope::new();
    scope.bind("c", Value::Record(cover));
    let node = section(&stream, "Page", "p");
    let mut cx = Expander::new(&registry, &templates, &NoSources);
    let trees = cx.expand_children(node, &scope).unwrap();
    let mut out = String::new();
    assert!(collapse(&trees, &mut HtmlCollapse::new(), &mut out).is_err());

    let mut cover = Record::new(None).with_type("ImageRef");
    cover.push("src", Value::Link("/a\"b.png".into()));
    cover.push("alt", Value::Text("<A>".into()));
    let mut scope = Scope::new();
    scope.bind("c", Value::Record(cover));
    let trees = cx.expand_children(node, &scope).unwrap();
    let mut out = String::new();
    collapse(&trees, &mut HtmlCollapse::new(), &mut out).unwrap();
    assert_eq!(out, r#"<img src="/a&quot;b.png" alt="&lt;A&gt;">"#);
}
