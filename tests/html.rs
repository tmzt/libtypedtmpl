//! `HtmlCollapse`: values by sigil, escaping, refused schemes, positions,
//! dates and elements, through a `String` writer.

use libtypedmd::{Sigil, Slot};
use libtypedtmpl::{
    collapse, Date, Element, ErrorKind, Expanded, Html, HtmlCollapse, NodePath, Part, Segment,
    Value,
};

fn node(parts: Vec<Part>) -> Expanded {
    Expanded {
        path: NodePath::new().child(Segment {
            tag: Some("Content".into()),
            name: None,
            index: None,
        }),
        tag: Some("Content".into()),
        name: None,
        bindings: Vec::new(),
        parts,
    }
}

fn lit(s: &str) -> Part {
    Part::Literal(s.into())
}

fn val(sigil: Option<Sigil>, value: Value) -> Part {
    let slot = match sigil {
        Some(s) => Slot::typed(s, "x.y"),
        None => Slot::new("x.y"),
    };
    Part::Value { sigil, value, slot }
}

fn html(parts: Vec<Part>) -> Result<String, libtypedtmpl::Error> {
    html_with(HtmlCollapse::new(), parts)
}

fn html_with(mut c: HtmlCollapse, parts: Vec<Part>) -> Result<String, libtypedtmpl::Error> {
    let mut out = String::new();
    collapse(&[node(parts)], &mut c, &mut out)?;
    Ok(out)
}

fn text(s: &str) -> Value {
    Value::Text(s.into())
}

#[test]
fn each_sigil_and_value() {
    let date = Value::Date(Date {
        year: 2026,
        month: 10,
        day: 29,
    });
    for (sigil, value, expect) in [
        (Some(Sigil::String), text("Hi"), "Hi"),
        (Some(Sigil::String), Value::Number("12".into()), "12"),
        (
            Some(Sigil::String),
            Value::Currency("4 USD".into()),
            "4 USD",
        ),
        (Some(Sigil::String), Value::Link("/a".into()), "/a"),
        (Some(Sigil::String), date.clone(), "2026-10-29"),
        (Some(Sigil::String), Value::Bool(true), "true"),
        (Some(Sigil::String), Value::Markdown("*md*".into()), "*md*"),
        (Some(Sigil::Number), Value::Number("1.5".into()), "1.5"),
        (Some(Sigil::Currency), Value::Currency("$3".into()), "$3"),
        (Some(Sigil::Date), date.clone(), "2026-10-29"),
        (Some(Sigil::Boolean), Value::Bool(false), "false"),
        (
            Some(Sigil::Link),
            Value::Link("https://tmzt.dev/".into()),
            "https://tmzt.dev/",
        ),
        (None, Value::Html(Html::trusted("<b>&</b>")), "<b>&</b>"),
    ] {
        assert_eq!(
            html(vec![val(sigil, value.clone())]).unwrap(),
            expect,
            "{sigil:?} {value:?}"
        );
    }
}

#[test]
fn text_and_quoted_attributes_are_escaped() {
    let nasty = text(r#"<a href="x">&'"#);
    assert_eq!(
        html(vec![
            lit("<p>"),
            val(Some(Sigil::String), nasty.clone()),
            lit("</p>")
        ])
        .unwrap(),
        "<p>&lt;a href=&quot;x&quot;&gt;&amp;&#39;</p>"
    );
    assert_eq!(
        html(vec![
            lit("<p title=\""),
            val(Some(Sigil::String), nasty.clone()),
            lit("\">")
        ])
        .unwrap(),
        "<p title=\"&lt;a href=&quot;x&quot;&gt;&amp;&#39;\">"
    );
    assert_eq!(
        html(vec![
            lit("<p title='"),
            val(Some(Sigil::String), nasty),
            lit("'>")
        ])
        .unwrap(),
        "<p title='&lt;a href=&quot;x&quot;&gt;&amp;&#39;'>"
    );
}

#[test]
fn refused_schemes() {
    for link in [
        "javascript:alert(1)",
        "JAVASCRIPT:x",
        "  data:text/html,<b>",
        "vbscript:x",
        "java\tscript:x",
        "java\nscript:x",
    ] {
        let e = html(vec![
            lit("<a href=\""),
            val(Some(Sigil::Link), Value::Link(link.into())),
            lit("\">"),
        ])
        .unwrap_err();
        assert_eq!(e.kind, ErrorKind::RefusedScheme, "{link:?}");
        assert_eq!(e.path.to_string(), "[Content]");
        assert_eq!(e.slot.as_deref(), Some("{{ [-] x.y }}"));
    }
    for link in ["/a", "https://x", "mailto:a@b", "#top", "files/data:x"] {
        assert!(
            html(vec![val(Some(Sigil::Link), Value::Link(link.into()))]).is_ok(),
            "{link}"
        );
    }
}

#[test]
fn unsafe_positions() {
    let v = || val(Some(Sigil::String), text("x"));
    for (before, after) in [
        ("<a href=", ">"),
        ("<a ", ">"),
        ("<!-- ", " -->"),
        ("<script>", "</script>"),
        ("<STYLE>", "</style>"),
        ("<script type=\"x\">var a = '", "';</script>"),
    ] {
        let e = html(vec![lit(before), v(), lit(after)]).unwrap_err();
        assert_eq!(e.kind, ErrorKind::UnsafePosition, "{before:?}");
    }
    assert!(
        html(vec![lit("<script>1</script><p>"), v()]).is_ok(),
        "after the script ends"
    );
    assert!(
        html(vec![lit("<!-- c --><p>"), v()]).is_ok(),
        "after the comment ends"
    );
    let e = html(vec![
        lit("<p title=\""),
        val(None, Value::Html(Html::trusted("<b>"))),
        lit("\">"),
    ])
    .unwrap_err();
    assert_eq!(e.kind, ErrorKind::UnsafePosition, "HTML only in text");
}

#[test]
fn mismatched_values() {
    for (sigil, value) in [
        (Some(Sigil::Number), text("x")),
        (Some(Sigil::Date), text("x")),
        (None, text("x")),
        (Some(Sigil::String), Value::List(Vec::new())),
    ] {
        assert_eq!(
            html(vec![val(sigil, value)]).unwrap_err().kind,
            ErrorKind::SigilMismatch
        );
    }
}

#[test]
fn dates_by_the_formatter_for_date_slots_only() {
    let date = Value::Date(Date {
        year: 2026,
        month: 7,
        day: 2,
    });
    let c = || HtmlCollapse::new().with_dates(|d| format!("{}/{}", d.day, d.month));
    assert_eq!(
        html_with(c(), vec![val(Some(Sigil::Date), date.clone())]).unwrap(),
        "2/7"
    );
    assert_eq!(
        html_with(c(), vec![val(Some(Sigil::String), date)]).unwrap(),
        "2026-07-02"
    );
}

#[test]
fn elements() {
    let link = Element::new("link")
        .attr("rel", None, text("stylesheet"))
        .attr(
            "href",
            Some(Sigil::Link),
            Value::Link("/styles/site.css".into()),
        );
    let script = Element::new("script")
        .attr(
            "src",
            Some(Sigil::Link),
            Value::Link("/app.js?a=1&b=\"2\"".into()),
        )
        .flag("defer");
    let mut title = Element::new("title");
    title.children.push(val(Some(Sigil::String), text("A & B")));
    assert_eq!(
        html(vec![Part::Element(link), Part::Element(script), Part::Element(title)]).unwrap(),
        "<link rel=\"stylesheet\" href=\"/styles/site.css\"><script src=\"/app.js?a=1&amp;b=&quot;2&quot;\" defer></script><title>A &amp; B</title>"
    );
}

#[test]
fn element_errors() {
    let mut meta = Element::new("meta");
    meta.children.push(lit("x"));
    assert_eq!(
        html(vec![Part::Element(meta)]).unwrap_err().kind,
        ErrorKind::VoidChildren
    );
    let e = html(vec![lit("<a "), Part::Element(Element::new("b"))]).unwrap_err();
    assert_eq!(e.kind, ErrorKind::UnsafePosition);
    let bad = Element::new("a").attr(
        "href",
        Some(Sigil::Link),
        Value::Link("javascript:x".into()),
    );
    assert_eq!(
        html(vec![Part::Element(bad)]).unwrap_err().kind,
        ErrorKind::RefusedScheme
    );
    let html_attr = Element::new("a").attr("title", None, Value::Html(Html::trusted("<b>")));
    assert_eq!(
        html(vec![Part::Element(html_attr)]).unwrap_err().kind,
        ErrorKind::UnsafePosition
    );
    let mut script = Element::new("script");
    script.children.push(val(Some(Sigil::String), text("x")));
    assert_eq!(
        html(vec![Part::Element(script)]).unwrap_err().kind,
        ErrorKind::UnsafePosition,
        "a value inside a script element"
    );
}
