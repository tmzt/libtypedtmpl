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

#[test]
fn a_source_that_is_not_a_list() {
    let e = expand_with(&list(ITEM, "site.title"), "Section", "s", &[], &NoSources).unwrap_err();
    assert_eq!(e.kind, ErrorKind::NotAList);
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
