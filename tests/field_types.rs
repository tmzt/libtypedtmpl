//! `Markdown` and `Html` fields: what an instance may pass, and how the
//! body places them.

mod common;
use common::*;
use libtypedtmpl::{
    ErrorKind, Expander, FieldType, Html, NoSources, Part, Record, Registry, Scope, Templates,
    Value,
};

const SHOT: &str = concat!(
    "# [Template] Shot\n",
    "- [abc] title\n",
    "- details?: Markdown\n",
    "- [Content]\n",
    "  ```html\n",
    "  <h2>{{ [abc] this.title }}</h2>{{ this.details }}|{{ [abc] this.details }}\n",
    "  ```\n",
    "\n",
    "# [Template] Story\n",
    "- body: Html\n",
    "- [Content]\n",
    "  ```html\n",
    "  <article>{{ this.body }}</article>\n",
    "  ```\n",
    "\n",
    "# [Gallery] shots\n",
    "- [Shot] one\n",
    "  - title: One\n",
    "  - details: A *single* line.\n",
    "- [Shot] many\n",
    "  - title: Many\n",
    "  - details: |\n",
    "      First *line*.\n",
    "\n",
    "      Second.\n",
    "- [Shot] dated\n",
    "  - title: Dated\n",
    "  - [31] details: 2026-10-31\n",
    "\n",
);

fn page(body: &str) -> String {
    format!("{SHOT}# [Page] p\n{body}")
}

#[test]
fn the_types_are_declared_by_name() {
    let templates = Templates::from_stream(&parse(SHOT)).unwrap();
    let shot = templates.get("Shot").unwrap();
    assert_eq!(shot.field("details").unwrap().kind, FieldType::Markdown);
    assert!(shot.field("details").unwrap().optional);
    let story = templates.get("Story").unwrap();
    assert_eq!(story.field("body").unwrap().kind, FieldType::Html);
}

/// Expand `[Page] p` with `shot` bound to the gallery's item `name` and
/// `story` to a record holding a rendered body.
fn with_shot(name: &str, body: &str) -> Result<libtypedtmpl::Expanded, libtypedtmpl::Error> {
    let source = page(body);
    let stream = parse(&source);
    let templates = Templates::from_stream(&stream)?;
    let registry = Registry::new();
    let gallery = Record::from_object(section(&stream, "Gallery", "shots"))?;
    let Some(Value::List(items)) = gallery.get("items") else {
        panic!("items")
    };
    let shot = items
        .iter()
        .find(|r| r.name() == Some(name))
        .expect("the item")
        .clone();
    let mut story = Record::new(Some("a".into()));
    story.push("body", Value::Html(Html::trusted("<p>Rendered.</p>")));
    let mut scope = Scope::new();
    scope.bind("shot", Value::Record(shot));
    scope.bind("story", Value::Record(story));
    let node = section(&stream, "Page", "p");
    let mut cx = Expander::new(&registry, &templates, &NoSources);
    let parts = cx.within(node, None, |cx| cx.expand_children(node, &scope))?;
    Ok(libtypedtmpl::Expanded {
        path: Default::default(),
        tag: Some("Page".into()),
        name: Some("p".into()),
        bindings: Vec::new(),
        parts: parts.into_iter().map(Part::Child).collect(),
    })
}

const PASS: &str = "- [Shot] x title={{ [abc] shot.title }} details={{ shot.details }}\n";

#[test]
#[cfg(feature = "markdown")]
fn plain_text_through_a_markdown_field_is_rendered() {
    let out = text(&with_shot("one", PASS).unwrap());
    assert_eq!(
        out,
        "<h2>One</h2><p>A <em>single</em> line.</p>\n|A *single* line."
    );
}

#[test]
#[cfg(feature = "markdown")]
fn a_markdown_block_through_a_markdown_field_is_rendered() {
    let out = text(&with_shot("many", PASS).unwrap());
    assert_eq!(
        out,
        "<h2>Many</h2><p>First <em>line</em>.</p>\n<p>Second.</p>\n|First *line*.\n\nSecond."
    );
}

#[test]
#[cfg(feature = "markdown")]
fn a_literal_is_markdown_and_an_optional_field_left_out_writes_nothing() {
    let out = text(&with_shot("one", "- [Shot] x title=T details=**bold**\n").unwrap());
    assert_eq!(out, "<h2>T</h2><p><strong>bold</strong></p>\n|**bold**");
    let out = text(&with_shot("one", "- [Shot] x title=T\n").unwrap());
    assert_eq!(out, "<h2>T</h2>|");
}

#[test]
fn other_values_are_refused_at_the_instance() {
    let e = with_shot("dated", PASS).unwrap_err();
    assert_eq!(e.kind, ErrorKind::SigilMismatch);
    assert!(
        e.message
            .contains("the field `details` is Markdown, but `shot.details` is a date"),
        "{e}"
    );
    let e = with_shot("one", "- [Shot] x title=T details={{ story.body }}\n").unwrap_err();
    assert!(
        e.message.contains("is Markdown, but `story.body` is HTML"),
        "{e}"
    );
    let e = with_shot("one", "- [Shot] x title=T\n  - details:\n    - a: b\n").unwrap_err();
    assert!(e.message.contains("takes a value, not child rows"), "{e}");
}

#[test]
fn an_html_field_takes_trusted_html_only() {
    let out = text(&with_shot("one", "- [Story] s body={{ story.body }}\n").unwrap());
    assert_eq!(out, "<article><p>Rendered.</p></article>");
    let e = with_shot("one", "- [Story] s body={{ shot.title }}\n").unwrap_err();
    assert!(
        e.message
            .contains("the field `body` is HTML, but `shot.title` is text"),
        "{e}"
    );
    let e = with_shot("one", "- [Story] s body=<p>x</p>\n").unwrap_err();
    assert!(e.message.contains("a literal is not trusted"), "{e}");
}
