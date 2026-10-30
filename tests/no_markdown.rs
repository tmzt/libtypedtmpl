//! Without the `markdown` feature, Markdown is refused and `html` blocks
//! still render.
#![cfg(not(feature = "markdown"))]

mod common;
use common::*;
use libtypedtmpl::{ErrorKind, NoSources};

#[test]
fn markdown_content_is_refused() {
    let e = expand_err(
        "# [Section] s\n- [Content]\n  Some *Markdown*.\n",
        "Section",
        "s",
    );
    assert_eq!(e.kind, ErrorKind::MarkdownDisabled);
}

#[test]
fn a_markdown_value_without_a_sigil_is_refused() {
    let source = "# [Data] d\n- caption: |\n    two\n    lines\n\n# [Section] s\n- [Content]\n  ```html\n  {{ d.caption }}\n  ```\n";
    let e = expand_with(source, "Section", "s", &[("Data", "d")], &NoSources).unwrap_err();
    assert_eq!(e.kind, ErrorKind::MarkdownDisabled);
}

#[test]
fn html_blocks_still_render() {
    let tree = expand(
        "# [Section] s\n- [Content]\n  ```html\n  <p>ok</p>\n  ```\n",
        "Section",
        "s",
    );
    assert_eq!(text(&tree), "<p>ok</p>");
}
