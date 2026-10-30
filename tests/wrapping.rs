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

#[test]
fn wrapping_nests() {
    let tree = expand(
        concat!(
            "# [Section] s\n",
            "- [Content]\n  ```html\n  <body><nav>{{ children }}</nav>\n  ```\n",
            "- [Content]\n  ```html\n  <a>home</a>\n  ```\n",
            "- [Content]\n  ```html\n  <main>{{ children }}</main></body>\n  ```\n",
            "- [Content]\n  ```html\n  <p>page</p>\n  ```\n",
        ),
        "Section",
        "s",
    );
    assert_eq!(
        text(&tree),
        "<body><nav><a>home</a><main><p>page</p></main></body></nav>"
    );
}

#[test]
fn an_instances_children_wrap_too() {
    let source = concat!(
        "# [Template] Box\n",
        "- [Content]\n  ```html\n  <div>{{ children }}</div>\n  ```\n",
        "- [Children]\n",
        "\n",
        "# [Section] s\n",
        "- [Box]\n",
        "  - [Content]\n    ```html\n    <ul>{{ children }}</ul>\n    ```\n",
        "  - [Content]\n    ```html\n    <li>a</li>\n    ```\n",
    );
    assert_eq!(
        text(&expand(source, "Section", "s")),
        "<div><ul><li>a</li></ul></div>"
    );
}
