//! `[Template]` defines a type; its instances are checked against it.

mod common;
use common::*;
use libtypedmd::Sigil;
use libtypedtmpl::{ErrorKind, FieldDef, FieldType, NoSources, Templates, Value};

const CARD: &str = concat!(
    "# [Template] StoryCard\n",
    "- name: Link\n",
    "- [abc] text\n",
    "- title?: String\n",
    "- cover?: Image\n",
    "- [Content]\n",
    "  ```html\n",
    "  <a href=\"{{ [-] this.name }}\" title=\"{{ [abc] this.title }}\">{{ [abc] this.text }}</a>\n",
    "  ```\n",
    "\n",
    "# [Template] Image\n",
    "- src: Link\n",
    "- alt: String\n",
    "- [Content]\n",
    "  ```html\n",
    "  <img src=\"{{ [-] this.src }}\" alt=\"{{ [abc] this.alt }}\">\n",
    "  ```\n",
    "\n",
);

fn section(body: &str) -> String {
    format!("{CARD}# [Section] s\n{body}")
}

#[test]
fn definitions_read_as_objects_with_types_as_values() {
    let templates = Templates::from_stream(&parse(CARD)).unwrap();
    let card = templates
        .get("story_card")
        .expect("matched without case or `_`");
    assert_eq!(card.name, "StoryCard");
    let field = |name: &str, kind, optional| FieldDef {
        name: name.into(),
        kind,
        optional,
    };
    assert_eq!(
        card.fields,
        [
            field("name", FieldType::Core(Sigil::Link), false),
            field("text", FieldType::Core(Sigil::String), false),
            field("title", FieldType::Core(Sigil::String), true),
            field("cover", FieldType::Named("Image".into()), true),
        ]
    );
}

#[test]
fn an_instance_reads_its_head_and_fields() {
    let tree = expand(
        &section("- [StoryCard] /stories/a/ text=Hello title=Greeting\n"),
        "Section",
        "s",
    );
    assert_eq!(
        text(&tree),
        "<a href=\"/stories/a/\" title=\"Greeting\">Hello</a>"
    );
    let card = tree.children().next().unwrap();
    assert_eq!(card.tag.as_deref(), Some("StoryCard"));
    let Value::Record(this) = &card.bindings[0].1 else {
        panic!()
    };
    assert_eq!(this.get("name"), Some(&Value::Link("/stories/a/".into())));
}

#[test]
fn an_absent_optional_field_writes_nothing() {
    let tree = expand(
        &section("- [StoryCard] /a/\n  - text: Hi\n"),
        "Section",
        "s",
    );
    assert_eq!(text(&tree), "<a href=\"/a/\" title=\"\">Hi</a>");
}

#[test]
fn fields_from_slots_in_the_callers_scope() {
    let source = format!(
        "# [Data] d\n- [Story] a\n  - [abc] title: First\n  - [-] url: /a/\n\n{}",
        section("- [List] l source={{ [1.] d }} item_alias=story\n  - [Item]\n    - [StoryCard] {{ [-] story.url }} text={{ [abc] story.title }}\n")
    );
    let tree = expand_with(&source, "Section", "s", &[("Data", "d")], &NoSources).unwrap();
    assert_eq!(text(&tree), "<a href=\"/a/\" title=\"\">First</a>");
}

#[test]
fn a_nested_property_set_for_a_defined_type() {
    let source = format!(
        "{CARD}# [Template] Feature\n- [abc] text\n- cover: Image\n- [Content]\n  ```html\n  {{{{ [abc] this.text }}}}:{{{{ [-] this.cover.src }}}}\n  ```\n\n# [Section] s\n- [Feature] text=Hi\n  - cover:\n    - src: /images/a.png\n    - alt: A photo\n"
    );
    assert_eq!(text(&expand(&source, "Section", "s")), "Hi:/images/a.png");
    let missing = source.replace("    - alt: A photo\n", "");
    let e = expand_err(&missing, "Section", "s");
    assert_eq!(e.kind, ErrorKind::MissingField);
    assert!(
        e.message
            .contains("the field `cover` (a `Image`) needs `alt`"),
        "{e}"
    );
}

#[test]
fn children_go_where_the_body_places_them() {
    let source = concat!(
        "# [Template] Sidebar\n",
        "- [Content]\n",
        "  ```html\n",
        "  <aside>{{ children }}</aside>\n",
        "  ```\n",
        "- [Children]\n",
        "\n",
        "# [Template] NavLink\n",
        "- name: Link\n",
        "- [abc] text\n",
        "- [Content]\n",
        "  ```html\n",
        "  <a href=\"{{ [-] this.name }}\">{{ [abc] this.text }}</a>\n",
        "  ```\n",
        "\n",
        "# [Section] s\n",
        "- [Sidebar]\n",
        "  - [NavLink] /about/ text=About\n",
        "  - [NavLink] /stories/ text=Stories\n",
    );
    let tree = expand(source, "Section", "s");
    assert_eq!(
        text(&tree),
        "<aside><a href=\"/about/\">About</a><a href=\"/stories/\">Stories</a></aside>"
    );
    assert_eq!(tree.count("NavLink"), 2);
    assert_eq!(tree.count("Children"), 1);
}

#[test]
fn the_body_sees_roots_and_this_but_not_the_callers_aliases() {
    let body = "# [Template] Peek\n- [Content]\n  ```html\n  {{ [abc] site.title }}/{{ [abc] shot.title }}\n  ```\n\n";
    let source = format!("{body}# [Gallery] g\n- [Shot] a\n  - title: A\n\n# [Section] s\n- [List] l source={{{{ [1.] g }}}} item_alias=shot\n  - [Item]\n    - [Peek]\n");
    let e = expand_with(&source, "Section", "s", &[("Gallery", "g")], &NoSources).unwrap_err();
    assert_eq!(e.kind, ErrorKind::UnknownAlias, "{e}");
    assert!(e.message.contains("`shot` is not in scope"));

    let wrapper =
        "# [Template] Wrap\n- [Content]\n  ```html\n  [{{ children }}]\n  ```\n- [Children]\n\n";
    let source = format!("{wrapper}# [Gallery] g\n- [Shot] a\n  - title: A\n\n# [Section] s\n- [List] l source={{{{ [1.] g }}}} item_alias=shot\n  - [Item]\n    - [Wrap]\n      - [Content]\n        ```html\n        {{{{ [abc] shot.title }}}}\n        ```\n");
    let tree = expand_with(&source, "Section", "s", &[("Gallery", "g")], &NoSources).unwrap();
    assert_eq!(
        text(&tree),
        "[A]",
        "children are reduced in the caller's scope"
    );
}

#[test]
fn instance_errors() {
    for (body, kind, expect) in [
        (
            "- [StoryCard] /a/\n",
            ErrorKind::MissingField,
            "needs the field `text`",
        ),
        (
            "- [StoryCard] /a/ text=Hi colour=red\n",
            ErrorKind::UnknownField,
            "has no field `colour`",
        ),
        (
            "- [StoryCard] /a/ text=Hi\n  - cover: /x.png\n",
            ErrorKind::BadValue,
            "set with child rows",
        ),
        (
            "- [StoryCard] text=Hi\n  - name: {{ [abc] site.title }}\n",
            ErrorKind::SigilMismatch,
            "text is not `[-]`",
        ),
        (
            "- [StoryCard] /a/ text=Hi\n  - [Content]\n    x\n",
            ErrorKind::ChildrenNotPlaced,
            "takes no children",
        ),
        (
            "- [Nobody]\n",
            ErrorKind::UnknownType,
            "neither a construct nor a defined type",
        ),
        (
            "- [StoryCard] /a/ text=Hi\n  - title Oops\n",
            ErrorKind::StrayNode,
            "neither a field nor an object",
        ),
    ] {
        let e = expand_err(&section(body), "Section", "s");
        assert_eq!(e.kind, kind, "{body:?}: {e}");
        assert!(e.message.contains(expect), "{body:?}: {e}");
    }
}

#[test]
fn a_type_that_instantiates_itself() {
    let source = "# [Template] A\n- [B]\n\n# [Template] B\n- [A]\n\n# [Section] s\n- [A]\n";
    let e = expand_err(source, "Section", "s");
    assert_eq!(e.kind, ErrorKind::TemplateCycle);
    assert!(e.message.contains("a > b > a"), "{e}");
}

#[test]
fn children_outside_a_body_are_misplaced() {
    let e = expand_err("# [Section] s\n- [Children]\n", "Section", "s");
    assert_eq!(e.kind, ErrorKind::Misplaced);
}

#[test]
fn definition_errors() {
    for (def, kind, expect) in [
        (
            "# [Template] Card\n- [Content]\n  A\n\n# [Template] card\n- [Content]\n  B\n",
            ErrorKind::DuplicateTemplate,
            "defined twice",
        ),
        (
            "# [Template] Card\n- type: card\n- [Content]\n  X\n",
            ErrorKind::BadDefinition,
            "the type is the template's head",
        ),
        (
            "# [Template] Card\n- cover: Image?\n- [Content]\n  X\n",
            ErrorKind::BadDefinition,
            "as `cover?: Image`",
        ),
        (
            "# [Template] Card\n- [abc?] title\n- [Content]\n  X\n",
            ErrorKind::BadDefinition,
            "as `- [abc] name?`",
        ),
        (
            "# [Template] Card\n- [abc] text: Hi\n- [Content]\n  X\n",
            ErrorKind::BadDefinition,
            "declares types, not values",
        ),
        (
            "# [Template] Card\n- [abc] text\n",
            ErrorKind::BadDefinition,
            "has no body",
        ),
        (
            "# [Template] Content\n- [Content]\n  X\n",
            ErrorKind::BadDefinition,
            "would redefine a built-in tag",
        ),
        (
            "# [Template] 9card\n- [Content]\n  X\n",
            ErrorKind::BadDefinition,
            "must be a name",
        ),
    ] {
        let e = Templates::from_stream(&parse(def)).unwrap_err();
        assert_eq!(e.kind, kind, "{def:?}: {e}");
        assert!(e.message.contains(expect), "{def:?}: {e}");
    }
}
