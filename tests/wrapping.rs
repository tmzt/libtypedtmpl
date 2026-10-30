//! `{{ children }}` in a `[Content]` wraps the objects after it.
#![cfg(feature = "markdown")]

mod common;
use common::*;

#[test]
fn a_shell_wraps_the_objects_after_it() {
    let tree = expand(
        "# [Section] s\n- [Content]\n  ```html\n  <main>{{ children }}</main>\n  ```\n- [Content]\n  One.\n- [Content]\n  Two.\n",
        "Section",
        "s",
    );
    assert_eq!(
        tree.children().count(),
        1,
        "the wrapped nodes are inside the shell, not after it"
    );
    assert_eq!(text(&tree), "<main><p>One.</p>\n<p>Two.</p>\n</main>");
    let shell = tree.children().next().unwrap();
    assert_eq!(shell.children().count(), 2);
}

#[test]
fn without_children_the_objects_follow() {
    let tree = expand(
        "# [Section] s\n- [Content]\n  First.\n- [Content]\n  Second.\n",
        "Section",
        "s",
    );
    assert_eq!(tree.children().count(), 2);
    assert_eq!(text(&tree), "<p>First.</p>\n<p>Second.</p>\n");
}

#[test]
fn markdown_wrapping_unwraps_its_paragraph() {
    let tree = expand("# [Section] s\n- [Content]\n  Before.\n\n  {{ children }}\n\n  After.\n- [Content]\n  ```html\n  <hr>\n  ```\n", "Section", "s");
    assert_eq!(text(&tree), "<p>Before.</p>\n<hr>\n<p>After.</p>\n");
}
