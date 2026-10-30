//! Step 1: values, records read from typedmd objects, slot paths, node
//! paths and errors.

use libtypedmd::{parse_with, Node, ParseOptions};
use libtypedtmpl::{Date, Error, ErrorKind, Html, NodePath, Record, Segment, SlotPath, Value};

/// The first top-level node of a strictly parsed document.
fn node(source: &str) -> Node {
    let stream = parse_with(source, &ParseOptions::strict()).expect("the test document parses");
    stream.documents[0].nodes[0].clone()
}

fn record(source: &str) -> Record {
    Record::from_object(&node(source)).unwrap()
}

fn error(source: &str) -> Error {
    match Record::from_object(&node(source)) {
        Ok(r) => panic!("{source:?} was read as {r:?}"),
        Err(e) => e,
    }
}

fn text(s: &str) -> Value {
    Value::Text(s.into())
}

#[test]
fn every_sigil_reads_into_its_variant() {
    let r = record(concat!(
        "# [Shot] inbox\n",
        "- plain: Inbox view\n",
        "- [abc] title: Inbox\n",
        "- [#] width: 1080\n",
        "- [#] ratio: -0.5\n",
        "- [31] taken: 2026-10-29\n",
        "- [-] src: /images/inbox.png\n",
        "- [_] dark: true\n",
        "- [$] price: 4.99 USD\n",
    ));
    assert_eq!(r.name(), Some("inbox"));
    assert_eq!(r.get("plain"), Some(&text("Inbox view")));
    assert_eq!(r.get("title"), Some(&text("Inbox")));
    assert_eq!(r.get("width"), Some(&Value::Number("1080".into())));
    assert_eq!(r.get("ratio"), Some(&Value::Number("-0.5".into())));
    assert_eq!(
        r.get("taken"),
        Some(&Value::Date(Date {
            year: 2026,
            month: 10,
            day: 29
        }))
    );
    assert_eq!(r.get("src"), Some(&Value::Link("/images/inbox.png".into())));
    assert_eq!(r.get("dark"), Some(&Value::Bool(true)));
    assert_eq!(r.get("price"), Some(&Value::Currency("4.99 USD".into())));
}

#[test]
fn inline_properties_are_fields_too() {
    let r = record("# [Shot] inbox mode=dark width=1080\n");
    assert_eq!(r.get("mode"), Some(&text("dark")));
    assert_eq!(r.get("width"), Some(&text("1080")), "no sigil: text");
}

#[test]
fn values_that_do_not_read_as_their_type_are_errors() {
    for (row, expect) in [
        ("- [#] width: 1x", "`width`: `1x` is not a number"),
        ("- [#] width: 1.", "`1.` is not a number"),
        ("- [#] width: .5", "`.5` is not a number"),
        ("- [31] taken: soon", "`taken`: `soon` is not a date"),
        ("- [31] taken: 2026-02-30", "`2026-02-30` is not a date"),
        ("- [_] dark: yes", "`yes` is not `true` or `false`"),
        ("- [1.] shots: one", "`[1.]` takes child rows, not a value"),
    ] {
        let e = error(&format!("# [Shot] inbox\n{row}\n"));
        assert_eq!(e.kind, ErrorKind::BadValue, "{row}");
        assert!(e.message.contains(expect), "{row}: {}", e.message);
        assert_eq!(
            e.to_string(),
            format!("[Shot] inbox > {}: {}", e.path.segments()[1], e.message)
        );
    }
}

#[test]
fn a_member_with_no_value_is_an_error() {
    let e = error("# [Record] user\n- [abc] id\n");
    assert_eq!(e.kind, ErrorKind::NoValue);
    assert_eq!(e.to_string(), "[Record] user > id: `id` has no value");
}

#[test]
fn slots_in_data_are_errors() {
    let e = error("# [Shot] inbox\n- title: {{ [abc] page.title }}\n");
    assert_eq!(e.kind, ErrorKind::SlotInData);
    let e = error("# [Shot] {{ [abc] x.name }}\n- title: T\n");
    assert_eq!(e.kind, ErrorKind::SlotInData);
}

#[test]
fn nested_member_records() {
    let r = record("# [Shot] inbox\n- device:\n  - model: Galaxy S23\n  - [#] width: 1080\n");
    let Some(Value::Record(device)) = r.get("device") else {
        panic!("{r:?}")
    };
    assert_eq!(device.name(), None);
    assert_eq!(device.get("model"), Some(&text("Galaxy S23")));
    assert_eq!(device.get("width"), Some(&Value::Number("1080".into())));
}

#[test]
fn member_lists_of_objects() {
    for list in ["- shots:", "- [1.] shots:"] {
        let r = record(&format!(
            "# [Gallery] companion\n{list}\n  - [Shot] inbox\n  - [Shot] thread\n"
        ));
        let Some(Value::List(shots)) = r.get("shots") else {
            panic!("{r:?}")
        };
        let names: Vec<_> = shots.iter().map(|s| s.name()).collect();
        assert_eq!(names, [Some("inbox"), Some("thread")]);
    }
}

#[test]
fn mixed_and_wrongly_typed_children_are_errors() {
    let e = error("# [Gallery] g\n- shots:\n  - [Shot] a\n  - title: T\n");
    assert_eq!(e.kind, ErrorKind::RecordShape);
    assert!(e.message.contains("a mix of rows"), "{}", e.message);
    let e = error("# [Gallery] g\n- [abc] device:\n  - model: M\n");
    assert_eq!(e.kind, ErrorKind::RecordShape);
    assert!(
        e.message.contains("take no sigil, not `[abc]`"),
        "{}",
        e.message
    );
}

#[test]
fn child_objects_are_the_items_in_order() {
    let r = record("# [Gallery] companion\n- title: Companion\n- [Shot] inbox\n- [Shot] thread\n- [Shot] approve\n");
    let Some(Value::List(items)) = r.get("items") else {
        panic!("{r:?}")
    };
    let names: Vec<_> = items.iter().map(|s| s.name()).collect();
    assert_eq!(names, [Some("inbox"), Some("thread"), Some("approve")]);
    let fields: Vec<&str> = r.fields().iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(fields, ["title", "items"], "items come after the members");
}

#[test]
fn items_are_present_and_empty_without_child_objects() {
    let r = record("# [Shot] inbox\n- title: Inbox\n");
    assert_eq!(r.get("items"), Some(&Value::List(Vec::new())));
}

#[test]
fn a_member_named_items_is_an_error() {
    let e = error("# [Gallery] g\n- items: three\n");
    assert_eq!(e.kind, ErrorKind::RecordShape);
    assert!(e.message.contains("may not be named `items`"));
}

#[test]
fn prose_is_a_note_and_plain_rows_are_errors() {
    let r = record("# [Shot] inbox\n- title: Inbox\n\nA note about this shot.\n");
    assert_eq!(r.fields().len(), 2, "title and items; the note is not read");
    let e = error("# [Shot] inbox\n- title Inbox\n");
    assert_eq!(e.kind, ErrorKind::RecordShape);
    assert!(e
        .message
        .contains("the row `title Inbox` is neither a member nor an object"));
}

#[test]
fn a_literal_block_without_a_sigil_is_markdown() {
    let r = record("# [Shot] inbox\n- caption: |\n    The *inbox*,\n    two lines.\n- [abc] alt: |\n    Plain\n    text.\n");
    assert_eq!(
        r.get("caption"),
        Some(&Value::Markdown("The *inbox*,\ntwo lines.".into()))
    );
    assert_eq!(
        r.get("alt"),
        Some(&text("Plain\ntext.")),
        "an explicit [abc] stays text"
    );
}

#[test]
fn get_name_answers_the_head() {
    let r = record("# [Shot] inbox\n- title: Inbox\n");
    assert_eq!(r.get("name"), Some(&text("inbox")));
    let r = record("# [Shot] inbox\n- name: Overridden\n");
    assert_eq!(
        r.get("name"),
        Some(&text("Overridden")),
        "a field named `name` wins"
    );
    let r = record("# [Shot]\n- title: Untitled\n");
    assert_eq!(r.name(), None);
    assert_eq!(r.get("name"), None);
}

#[test]
fn the_last_field_wins() {
    let mut r = Record::new(None);
    r.push("title", text("first"));
    r.push("title", text("second"));
    assert_eq!(r.get("title"), Some(&text("second")));
    assert_eq!(r.fields().len(), 2, "both are kept, in order");
}

#[test]
fn only_objects_are_records() {
    let stream = parse_with("# [Shot] a\n- title: T\n", &ParseOptions::strict()).unwrap();
    let member = &stream.documents[0].nodes[0].nodes[0];
    let e = Record::from_object(member).unwrap_err();
    assert_eq!(e.kind, ErrorKind::RecordShape);
}

#[test]
fn dates() {
    assert_eq!(
        Date::parse("2024-02-29"),
        Some(Date {
            year: 2024,
            month: 2,
            day: 29
        })
    );
    assert_eq!(
        Date::parse("2000-02-29").map(|d| d.to_string()),
        Some("2000-02-29".into())
    );
    for bad in [
        "2023-02-29",
        "1900-02-29",
        "2026-13-01",
        "2026-00-10",
        "2026-04-31",
        "2026-4-01",
        "26-04-01",
        "2026/04/01",
        "2026-04-01 ",
        "2026-0a-01",
    ] {
        assert_eq!(Date::parse(bad), None, "{bad}");
    }
}

#[test]
fn html_is_only_made_trusted() {
    let h = Html::trusted("<b>x</b>");
    assert_eq!(h.as_str(), "<b>x</b>");
    assert_eq!(h.into_string(), "<b>x</b>");
}

#[test]
fn slot_paths_parse() {
    let p = SlotPath::parse("shot.device.model").unwrap();
    assert_eq!(
        (p.alias.as_str(), p.fields.as_slice()),
        ("shot", &["device".to_string(), "model".to_string()][..])
    );
    assert_eq!(p.to_string(), "shot.device.model");
    assert_eq!(SlotPath::parse("_x").unwrap().fields.len(), 0);
    assert!(SlotPath::parse("story-card.sub_title").is_ok());
    for bad in [
        "",
        "shot.",
        ".shot",
        "shot..x",
        "shot title",
        "9shot",
        "shot.9",
        "shot.title!",
        "shot.(x)",
    ] {
        let e = SlotPath::parse(bad).unwrap_err();
        assert_eq!(e.kind, ErrorKind::NotAPath, "{bad:?}");
    }
}

#[test]
fn slot_paths_look_up_fields() {
    let r = record("# [Shot] inbox\n- device:\n  - model: Galaxy S23\n- title: Inbox\n");
    let shot = Value::Record(r);
    assert_eq!(
        SlotPath::parse("shot.device.model").unwrap().lookup(&shot),
        Ok(&text("Galaxy S23"))
    );
    assert_eq!(
        SlotPath::parse("shot.name").unwrap().lookup(&shot),
        Ok(&text("inbox"))
    );
    assert_eq!(SlotPath::parse("shot").unwrap().lookup(&shot), Ok(&shot));

    let e = SlotPath::parse("shot.colour")
        .unwrap()
        .lookup(&shot)
        .unwrap_err();
    assert_eq!(e.kind, ErrorKind::UnknownField);
    assert_eq!(e.message, "`shot` has no field `colour`");
    let e = SlotPath::parse("shot.title.length")
        .unwrap()
        .lookup(&shot)
        .unwrap_err();
    assert_eq!(e.kind, ErrorKind::NotARecord);
    assert_eq!(
        e.message,
        "`shot.title` is text, which has no field `length`"
    );
}

#[test]
fn node_paths_display() {
    let mut path = NodePath::new();
    assert!(path.is_empty());
    assert_eq!(path.to_string(), "");
    path.push(Segment {
        tag: Some("Page".into()),
        name: Some("home".into()),
        index: None,
    });
    path.push(Segment {
        tag: Some("List".into()),
        name: Some("posts".into()),
        index: None,
    });
    path.push(
        Segment {
            tag: Some("Item".into()),
            name: None,
            index: None,
        }
        .with_index(3),
    );
    assert_eq!(path.to_string(), "[Page] home > [List] posts > [Item] #3");
    assert_eq!(
        path.child(Segment {
            tag: None,
            name: Some("title".into()),
            index: None
        })
        .segments()
        .len(),
        4
    );
    assert_eq!(path.pop().map(|s| s.to_string()), Some("[Item] #3".into()));
}

#[test]
fn errors_display_their_path_and_slot() {
    let slot = libtypedmd::Slot::typed(libtypedmd::Sigil::String, "story.title");
    let path = NodePath::new().child(Segment {
        tag: Some("Item".into()),
        name: None,
        index: Some(2),
    });
    let e = Error::new(ErrorKind::Consumer, "something failed")
        .at(path)
        .in_slot(&slot);
    assert_eq!(
        e.to_string(),
        "[Item] #2: something failed in `{{ [abc] story.title }}`"
    );
    assert_eq!(
        Error::new(ErrorKind::Consumer, "plain").to_string(),
        "plain"
    );
}
