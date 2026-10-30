//! `[Content]`: Markdown or one `html` block, with slots read by the
//! crate's reader.

mod common;
use common::*;
use libtypedtmpl::ErrorKind;
#[cfg(feature = "markdown")]
use libtypedtmpl::{Part, Value};

const DATA: &str = "# [Data] story\n- [abc] title: Ports *and* <tags>\n- [-] url: /stories/a/\n- caption: |\n    A *caption*\n    on two lines.\n\n";

fn content(body: &str) -> String {
    let source = format!("{DATA}# [Section] s\n{body}");
    let tree = expand_with(
        &source,
        "Section",
        "s",
        &[("Data", "story")],
        &libtypedtmpl::NoSources,
    )
    .unwrap();
    text(&tree)
}

fn content_err(body: &str) -> libtypedtmpl::Error {
    let source = format!("{DATA}# [Section] s\n{body}");
    expand_with(
        &source,
        "Section",
        "s",
        &[("Data", "story")],
        &libtypedtmpl::NoSources,
    )
    .unwrap_err()
}

#[test]
fn html_with_slots() {
    let out = content("- [Content]\n  ```html\n  <a href=\"{{ [-] story.url }}\">{{ [abc] story.title }}</a>\n  ```\n");
    assert_eq!(
        out, "<a href=\"/stories/a/\">Ports *and* <tags></a>",
        "values are parts; escaping is the collapse's"
    );
}

#[test]
#[cfg(feature = "markdown")]
fn markdown_paragraphs_and_literal_blocks_in_order() {
    let out = content("- [Content]\n  Intro **bold**.\n\n  - |\n    ## Projects\n\n    - [Card] not a card\n    - item: with a colon\n");
    assert!(
        out.starts_with("<p>Intro <strong>bold</strong>.</p>\n<h2 id=\"projects\">"),
        "{out}"
    );
    assert!(
        out.contains("<li>[Card] not a card</li>"),
        "typed notation inside is text: {out}"
    );
    assert!(out.contains("<li>item: with a colon</li>"), "{out}");
}

#[test]
#[cfg(feature = "markdown")]
fn a_value_is_never_read_as_markdown() {
    let source =
        format!("{DATA}# [Section] s\n- [Content]\n  Title: {{{{ [abc] story.title }}}}\n");
    let tree = expand_with(
        &source,
        "Section",
        "s",
        &[("Data", "story")],
        &libtypedtmpl::NoSources,
    )
    .unwrap();
    let content = tree.children().next().unwrap();
    assert_eq!(content.parts.len(), 3);
    assert_eq!(content.parts[0], Part::Literal("<p>Title: ".into()));
    assert!(
        matches!(&content.parts[1], Part::Value { value: Value::Text(t), .. } if t == "Ports *and* <tags>")
    );
    assert_eq!(content.parts[2], Part::Literal("</p>\n".into()));
}

#[test]
#[cfg(feature = "markdown")]
fn a_slot_in_a_link_destination() {
    let out = content("- [Content]\n  [Read it]({{ [-] story.url }})\n");
    assert_eq!(out, "<p><a href=\"/stories/a/\">Read it</a></p>\n");
}

#[test]
#[cfg(feature = "markdown")]
fn tables_and_footnotes() {
    let out = content("- [Content]\n  - |\n    | a | b |\n    | --- | --- |\n    | 1 | 2 |\n\n    A note.[^1]\n\n    [^1]: The note.\n");
    assert!(
        out.contains("<table>") && out.contains("<td>1</td>"),
        "{out}"
    );
    assert!(out.contains("footnote"), "{out}");
}

#[test]
fn slots_false_writes_text_as_it_stands() {
    let out =
        content("- [Content] slots=false\n  ```html\n  <p>{{ literal }} \\{{ kept }}</p>\n  ```\n");
    assert_eq!(out, "<p>{{ literal }} \\{{ kept }}</p>");
    if cfg!(feature = "markdown") {
        let out = content("- [Content] slots=false\n  Braces {{ here }}.\n");
        assert_eq!(out, "<p>Braces {{ here }}.</p>\n");
    }
}

#[test]
fn an_escaped_opening_is_literal() {
    assert_eq!(
        content("- [Content]\n  ```html\n  \\{{ name }}\n  ```\n"),
        "{{ name }}"
    );
}

#[test]
#[cfg(feature = "markdown")]
fn markdown_fields() {
    let as_text = content("- [Content]\n  ```html\n  {{ [abc] story.caption }}\n  ```\n");
    assert_eq!(
        as_text, "A *caption*\non two lines.",
        "with a sigil, the source as text"
    );
    let rendered = content("- [Content]\n  ```html\n  {{ story.caption }}\n  ```\n");
    assert_eq!(rendered, "<p>A <em>caption</em>\non two lines.</p>\n");
}

#[test]
#[cfg(feature = "markdown")]
fn an_html_value_alone_in_a_paragraph_takes_its_place() {
    let source = format!(
        "{DATA}# [Section] s\n- [Content]\n  Before.\n\n  {{{{ story.caption }}}}\n\n  After.\n"
    );
    let tree = expand_with(
        &source,
        "Section",
        "s",
        &[("Data", "story")],
        &libtypedtmpl::NoSources,
    )
    .unwrap();
    let out = text(&tree);
    assert!(
        out.contains("</p>\n<p>A <em>caption</em>"),
        "no <p> around the placed block: {out}"
    );
    assert!(!out.contains("<p><p>"), "{out}");
}

#[test]
fn shape_errors() {
    for (body, kind, expect) in [
        (
            "- [Content]\n  Markdown.\n\n  ```html\n  <p>x</p>\n  ```\n",
            ErrorKind::ContentShape,
            "both Markdown and an `html` block",
        ),
        ("- [Content]\n", ErrorKind::ContentShape, "is empty"),
        (
            "- [Content] |\n  ## Projects\n",
            ErrorKind::ContentShape,
            "put `- |` on its own row under it",
        ),
        (
            "- [Content] colour=red\n  X\n",
            ErrorKind::ContentShape,
            "takes only `slots`",
        ),
        (
            "- [Content] slots=maybe\n  X\n",
            ErrorKind::ContentShape,
            "`slots` is `true` or `false`",
        ),
        (
            "- [Content]\n  ```html\n  a\n  ```\n  ```html\n  b\n  ```\n",
            ErrorKind::TwoFragments,
            "more than one `html` block",
        ),
    ] {
        let e = content_err(body);
        assert_eq!(e.kind, kind, "{body:?}: {e}");
        assert!(e.message.contains(expect), "{body:?}: {e}");
        assert!(
            e.path.to_string().starts_with("[Section] s > [Content]"),
            "{body:?}: {}",
            e.path
        );
    }
}

#[test]
fn slot_errors() {
    for (body, kind, expect) in [
        (
            "- [Content]\n  ```html\n  {{ [1.] story.items }}\n  ```\n",
            ErrorKind::ListSlot,
            "lists are drawn by `[List]`",
        ),
        (
            "- [Content]\n  ```html\n  {{ story.title }}\n  ```\n",
            ErrorKind::SigilMismatch,
            "write it with a sigil",
        ),
        (
            "- [Content]\n  ```html\n  {{ [31] story.title }}\n  ```\n",
            ErrorKind::SigilMismatch,
            "text is not `[31]`",
        ),
        (
            "- [Content]\n  ```html\n  {{ [abc] nobody.title }}\n  ```\n",
            ErrorKind::UnknownAlias,
            "`nobody` is not in scope",
        ),
        (
            "- [Content]\n  ```html\n  {{ [abc] story.colour }}\n  ```\n",
            ErrorKind::UnknownField,
            "has no field `colour`",
        ),
        (
            "- [Content]\n  ```html\n  {{ [abc] story title }}\n  ```\n",
            ErrorKind::NotAPath,
            "is not a path",
        ),
        (
            "- [Content]\n  ```html\n  <p>\n  {{ open\n  ```\n",
            ErrorKind::SlotSyntax,
            "line 2, column 1",
        ),
    ] {
        let e = content_err(body);
        assert_eq!(e.kind, kind, "{body:?}: {e}");
        assert!(e.message.contains(expect), "{body:?}: {e}");
    }
    let e = content_err("- [Content]\n  ```html\n  {{ [31] story.title }}\n  ```\n");
    assert_eq!(e.slot.as_deref(), Some("{{ [31] story.title }}"));
}

#[test]
#[cfg(feature = "markdown")]
fn a_slot_markdown_drops_is_lost() {
    let e = content_err("- [Content]\n  Ref.\n\n  [unused]: {{ [-] story.url }}\n");
    assert_eq!(e.kind, ErrorKind::SlotLost, "{e}");
    assert!(e.message.contains("did not keep this slot"));
}

#[test]
#[cfg(feature = "markdown")]
fn prose_outside_content_is_a_note() {
    assert_eq!(
        content("A note, not output.\n\n- [Content]\n  Shown.\n"),
        "<p>Shown.</p>\n"
    );
}
