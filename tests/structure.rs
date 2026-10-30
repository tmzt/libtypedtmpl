//! The expanded tree keeps its structure (the finding in highbay's
//! LOFI_HIFI_FINDINGS.md §2: an authored item flattened into strings).

mod common;
use common::*;
use libtypedtmpl::{NoSources, Part};

const SOURCE: &str = concat!(
    "# [Gallery] companion\n",
    "- [Shot] inbox\n",
    "  - [abc] title: Inbox\n",
    "  - [-] src: /img/inbox.png\n",
    "  - [#] width: 1080\n",
    "  - [_] dark: true\n",
    "  - tags:\n",
    "    - [Tag] new\n",
    "    - [Tag] pinned\n",
    "\n",
    "# [Section] screens\n",
    "- [List] gallery source={{ [1.] companion }} item_alias=shot\n",
    "  - [Item]\n",
    "    - [Content]\n",
    "      ```html\n",
    "      <figure>{{ [abc] shot.title }}|{{ [-] shot.src }}|{{ [#] shot.width }}|{{ [_] shot.dark }}{{ children }}</figure>\n",
    "      ```\n",
    "    - [List] tags source={{ [1.] shot.tags }} item_alias=tag\n",
    "      - [Item]\n",
    "        - [Content]\n",
    "          ```html\n",
    "          <i>{{ [abc] tag.name }}</i>\n",
    "          ```\n",
    "- [List] again source={{ [1.] companion }} item_alias=shot\n",
    "  - [Item]\n",
    "    - [Content]\n",
    "      ```html\n",
    "      <p>{{ [abc] shot.title }}</p>\n",
    "      ```\n",
);

#[test]
fn an_item_keeps_every_value_and_its_nested_list_in_order() {
    let tree = expand_with(
        SOURCE,
        "Section",
        "screens",
        &[("Gallery", "companion")],
        &NoSources,
    )
    .unwrap();
    let item = tree.children().next().unwrap().children().next().unwrap();
    let content = item.children().next().unwrap();
    let kinds: Vec<&str> = content
        .parts
        .iter()
        .map(|p| match p {
            Part::Literal(_) => "literal",
            Part::Value { .. } => "value",
            Part::Child(_) => "child",
            Part::Element(_) => "element",
        })
        .collect();
    assert_eq!(
        kinds,
        [
            "literal", "value", "literal", "value", "literal", "value", "literal", "value",
            "child", "literal"
        ],
        "four values, the wrapped list, and the text between them, in order"
    );
    assert_eq!(
        text(&tree),
        "<figure>Inbox|/img/inbox.png|1080|true<i>new</i><i>pinned</i></figure><p>Inbox</p>"
    );
}

#[test]
fn two_lists_side_by_side_stay_two() {
    let tree = expand_with(
        SOURCE,
        "Section",
        "screens",
        &[("Gallery", "companion")],
        &NoSources,
    )
    .unwrap();
    assert_eq!(tree.children().count(), 2);
    assert_eq!(tree.count("List"), 3, "two at the top, one nested");
}

#[test]
fn the_outline() {
    let tree = expand_with(
        SOURCE,
        "Section",
        "screens",
        &[("Gallery", "companion")],
        &NoSources,
    )
    .unwrap();
    let outline = tree.children().last().unwrap().to_string();
    assert_eq!(
        outline,
        "[List] again\n  [Item] #1 shot=inbox\n    [Content]\n      \"<p>\"\n      [abc] Inbox\n      \"</p>\"\n"
    );
}
