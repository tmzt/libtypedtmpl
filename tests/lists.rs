//! `[List]` with `[Item]` and `[Empty]`, its sources, and item scope.

mod common;
use common::*;
use libtypedtmpl::{ErrorKind, NoSources, Record, Sources, Value};

const GALLERY: &str = "# [Gallery] companion\n- [Shot] inbox\n  - [abc] title: Inbox\n- [Shot] thread\n  - [abc] title: A thread\n- [Shot] approve\n  - [abc] title: Approve\n\n";

fn list(arms: &str, source: &str) -> String {
    format!("{GALLERY}# [Section] s\n- [List] shots source={{{{ [1.] {source} }}}} item_alias=shot\n{arms}")
}

const ITEM: &str = "  - [Item]\n    - [Content]\n      ```html\n      <li>{{ [abc] shot.title }}</li>\n      ```\n";
const EMPTY: &str =
    "  - [Empty]\n    - [Content]\n      ```html\n      <p>No shots.</p>\n      ```\n";

#[test]
fn one_item_per_record_indexed_and_bound() {
    let tree = expand_with(
        &list(&format!("{ITEM}{EMPTY}"), "companion"),
        "Section",
        "s",
        &[("Gallery", "companion")],
        &NoSources,
    )
    .unwrap();
    assert_eq!(tree.count("Item"), 3);
    assert_eq!(tree.count("Empty"), 0);
    let list = tree.children().next().unwrap();
    for (i, item) in list.children().enumerate() {
        assert_eq!(item.path.segments().last().unwrap().index, Some(i + 1));
        assert!(matches!(&item.bindings[0], (alias, Value::Record(_)) if alias == "shot"));
    }
    assert_eq!(
        text(&tree),
        "<li>Inbox</li><li>A thread</li><li>Approve</li>"
    );
}

#[test]
fn no_records_draw_the_empty_arm() {
    let source = format!("# [Gallery] none\n- title: None\n\n# [Section] s\n- [List] shots source={{{{ [1.] none }}}} item_alias=shot\n{ITEM}{EMPTY}");
    let tree = expand_with(&source, "Section", "s", &[("Gallery", "none")], &NoSources).unwrap();
    assert_eq!((tree.count("Item"), tree.count("Empty")), (0, 1));
    assert_eq!(text(&tree), "<p>No shots.</p>");

    let source = format!("# [Gallery] none\n- title: None\n\n# [Section] s\n- [List] shots source={{{{ [1.] none }}}} item_alias=shot\n{ITEM}");
    let tree = expand_with(&source, "Section", "s", &[("Gallery", "none")], &NoSources).unwrap();
    assert_eq!(tree.count("List"), 1);
    assert!(
        tree.children().next().unwrap().parts.is_empty(),
        "no records and no [Empty]: an empty list"
    );
}

#[test]
fn a_path_to_a_list_field() {
    let source = format!("# [Gallery] g\n- shots:\n  - [Shot] a\n    - title: A\n\n# [Section] s\n- [List] shots source={{{{ [1.] g.shots }}}} item_alias=shot\n{ITEM}");
    let tree = expand_with(&source, "Section", "s", &[("Gallery", "g")], &NoSources).unwrap();
    assert_eq!(text(&tree), "<li>A</li>");
}

struct Recent;
impl Sources for Recent {
    fn list(&self, expr: &str) -> Result<Option<Vec<Record>>, String> {
        let make = |n: &str| {
            let mut r = Record::new(Some(n.into()));
            r.push("title", Value::Text(n.into()));
            r
        };
        match expr.split_once(' ') {
            Some(("recent", n)) => {
                let n: usize = n
                    .parse()
                    .map_err(|_| format!("`recent` takes a count, not `{n}`"))?;
                Ok(Some(
                    ["c", "b", "a"].iter().take(n).map(|s| make(s)).collect(),
                ))
            }
            _ => Ok(None),
        }
    }
}

#[test]
fn consumer_sources() {
    let tree = expand_with(&list(ITEM, "recent 2"), "Section", "s", &[], &Recent).unwrap();
    assert_eq!(text(&tree), "<li>c</li><li>b</li>");
    let e = expand_with(&list(ITEM, "recent two"), "Section", "s", &[], &Recent).unwrap_err();
    assert_eq!(
        (e.kind, e.message.as_str()),
        (
            ErrorKind::UnknownSource,
            "`recent` takes a count, not `two`"
        )
    );
    let e = expand_with(&list(ITEM, "oldest 2"), "Section", "s", &[], &Recent).unwrap_err();
    assert_eq!(e.kind, ErrorKind::UnknownSource);
    assert_eq!(e.slot.as_deref(), Some("{{ [1.] oldest 2 }}"));
}

const VALUE: &str =
    "  - [Item]\n    - [Content]\n      ```html\n      <b>{{ [abc] shot }}</b>\n      ```\n";

const COVERS: &str = "# [Story] a\n- [-] cover: /a/cover.png\n\n# [Story] b\n- [abc] title: B\n\n";

fn cover(story: &str) -> String {
    format!(
        "{COVERS}# [Section] s\n- [List] c source={{{{ [1.] {story}.cover }}}} item_alias=c\n{}{}",
        "  - [Item]\n    - [Content]\n      ```html\n      <img src=\"{{ [-] c }}\">\n      ```\n",
        EMPTY
    )
}

fn stories(source: &str) -> Result<libtypedtmpl::Expanded, libtypedtmpl::Error> {
    expand_with(
        source,
        "Section",
        "s",
        &[("Story", "a"), ("Story", "b")],
        &NoSources,
    )
}

#[test]
fn a_single_value_is_drawn_once_bound_as_it_is() {
    let tree = expand_with(&list(VALUE, "site.title"), "Section", "s", &[], &NoSources).unwrap();
    assert_eq!(tree.count("Item"), 1);
    assert_eq!(text(&tree), "<b>Example</b>");
    let tree = stories(&cover("a")).unwrap();
    assert_eq!(text(&tree), r#"<img src="/a/cover.png">"#);
    let list = tree.children().next().unwrap();
    let item = list.children().next().unwrap();
    assert!(matches!(&item.bindings[0], (alias, Value::Link(_)) if alias == "c"));
}

#[test]
fn a_missing_last_field_draws_empty() {
    let tree = stories(&cover("b")).unwrap();
    assert_eq!(tree.count("Item"), 0);
    assert_eq!(text(&tree), "<p>No shots.</p>");
    let without_empty = cover("b").replace(EMPTY, "");
    assert_eq!(text(&stories(&without_empty).unwrap()), "");
}

#[test]
fn a_missing_alias_or_middle_field_is_still_an_error() {
    let e = stories(&cover("nobody")).unwrap_err();
    assert_eq!(e.kind, ErrorKind::UnknownSource, "{e}");
    let e = stories(&cover("a.meta")).unwrap_err();
    assert_eq!(e.kind, ErrorKind::UnknownField, "{e}");
    assert!(e.message.contains("`a` has no field `meta`"), "{e}");
    let e = stories(&cover("a.cover")).unwrap_err();
    assert_eq!(e.kind, ErrorKind::NotARecord, "{e}");
    assert!(
        e.message
            .contains("`a.cover` is a link, which has no field `cover`"),
        "{e}"
    );
}

#[test]
fn nested_lists_see_both_aliases_and_inner_shadows_outer() {
    let source = concat!(
        "# [Data] d\n",
        "- [Group] phones\n",
        "  - [Shot] a\n",
        "    - title: A\n",
        "  - [Shot] b\n",
        "    - title: B\n",
        "\n",
        "# [Section] s\n",
        "- [List] groups source={{ [1.] d }} item_alias=g\n",
        "  - [Item]\n",
        "    - [List] shots source={{ [1.] g }} item_alias=shot\n",
        "      - [Item]\n",
        "        - [Content]\n",
        "          ```html\n",
        "          {{ [abc] g.name }}/{{ [abc] shot.title }};\n",
        "          ```\n",
    );
    let tree = expand_with(source, "Section", "s", &[("Data", "d")], &NoSources).unwrap();
    assert_eq!(text(&tree), "phones/A;phones/B;");
    let shadow = source.replace("item_alias=shot", "item_alias=g").replace(
        "{{ [abc] g.name }}/{{ [abc] shot.title }}",
        "{{ [abc] g.title }}",
    );
    assert_eq!(
        text(&expand_with(&shadow, "Section", "s", &[("Data", "d")], &NoSources).unwrap()),
        "A;B;"
    );
}

#[test]
fn the_item_alias_is_gone_after_its_list_and_in_empty() {
    let after = format!(
        "{}- [Content]\n  ```html\n  {{{{ [abc] shot.title }}}}\n  ```\n",
        list(ITEM, "companion")
    );
    let e = expand_with(
        &after,
        "Section",
        "s",
        &[("Gallery", "companion")],
        &NoSources,
    )
    .unwrap_err();
    assert_eq!(e.kind, ErrorKind::UnknownAlias);
    let empty = "# [Gallery] none\n- title: N\n\n# [Section] s\n- [List] l source={{ [1.] none }} item_alias=shot\n  - [Item]\n    - [Content]\n      x\n  - [Empty]\n    - [Content]\n      ```html\n      {{ [abc] shot.title }}\n      ```\n";
    let e = expand_with(empty, "Section", "s", &[("Gallery", "none")], &NoSources).unwrap_err();
    assert_eq!(e.kind, ErrorKind::UnknownAlias);
}

#[test]
fn list_shape_errors() {
    for (list_row, arms, kind) in [
        (
            "- [List] source={{ [1.] companion }} item_alias=shot",
            ITEM,
            ErrorKind::ListShape,
        ),
        ("- [List] l item_alias=shot", ITEM, ErrorKind::ListShape),
        (
            "- [List] l source={{ [1.] companion }}",
            ITEM,
            ErrorKind::ListShape,
        ),
        (
            "- [List] l source={{ [abc] companion }} item_alias=shot",
            ITEM,
            ErrorKind::ListShape,
        ),
        (
            "- [List] l source={{ [1.] companion }} item_alias=a.b",
            ITEM,
            ErrorKind::ListShape,
        ),
        (
            "- [List] l source={{ [1.] companion }} item_alias=shot",
            EMPTY,
            ErrorKind::ListShape,
        ),
        (
            "- [List] l source={{ [1.] companion }} item_alias=shot colour=red",
            ITEM,
            ErrorKind::ListShape,
        ),
        (
            "- [List] l source={{ [1.] companion }} item_alias=shot",
            "  - [Item] template=phone\n",
            ErrorKind::ArmShape,
        ),
        (
            "- [List] l source={{ [1.] companion }} item_alias=shot",
            "  - [Item]\n\n    Only a note.\n",
            ErrorKind::NothingToDraw,
        ),
        (
            "- [List] l source={{ [1.] companion }} item_alias=shot",
            "  - [Item]\n    - [Content]\n      x\n  - [Card] c\n",
            ErrorKind::ListShape,
        ),
    ] {
        let source = format!("{GALLERY}# [Section] s\n{list_row}\n{arms}");
        let e = expand_with(
            &source,
            "Section",
            "s",
            &[("Gallery", "companion")],
            &NoSources,
        )
        .unwrap_err();
        assert_eq!(e.kind, kind, "{list_row} / {arms:?}: {e}");
    }
}
