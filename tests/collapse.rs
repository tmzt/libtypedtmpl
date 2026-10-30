//! The collapse walk, the writers, and `render_html` end to end.

mod common;
use common::*;
use libtypedmd::Sigil;
use libtypedtmpl::{
    collapse, render_html, Collapse, Element, Error, ErrorKind, Expanded, Expander, HtmlCollapse,
    HtmlWriter, IoHtmlWriter, NoSources, Record, Registry, Scope, Templates, Value,
};

/// Records the calls a walk makes.
struct Recorder;
impl Collapse for Recorder {
    type Writer = Vec<String>;
    fn enter(&mut self, node: &Expanded, out: &mut Vec<String>) -> Result<(), Error> {
        out.push(format!("enter {}", node.tag.as_deref().unwrap_or("-")));
        Ok(())
    }
    fn literal(&mut self, text: &str, out: &mut Vec<String>) -> Result<(), Error> {
        out.push(format!("literal {text}"));
        Ok(())
    }
    fn value(
        &mut self,
        _: Option<Sigil>,
        value: &Value,
        out: &mut Vec<String>,
    ) -> Result<(), Error> {
        if let Value::Text(t) = value {
            if t == "boom" {
                return Err(Error::new(ErrorKind::Consumer, "boom"));
            }
        }
        out.push(format!("value {value:?}"));
        Ok(())
    }
    fn start_element(&mut self, e: &Element, out: &mut Vec<String>) -> Result<(), Error> {
        out.push(format!("start {}", e.name));
        Ok(())
    }
    fn end_element(&mut self, e: &Element, out: &mut Vec<String>) -> Result<(), Error> {
        out.push(format!("end {}", e.name));
        Ok(())
    }
    fn leave(&mut self, node: &Expanded, out: &mut Vec<String>) -> Result<(), Error> {
        out.push(format!("leave {}", node.tag.as_deref().unwrap_or("-")));
        Ok(())
    }
}

const GALLERY: &str = concat!(
    "# [Gallery] companion\n",
    "- [Shot] inbox\n",
    "  - [abc] title: Inbox\n",
    "  - [-] src: /img/inbox.png\n",
    "- [Shot] thread\n",
    "  - [abc] title: A <thread>\n",
    "  - [-] src: /img/thread.png\n",
    "\n",
    "# [Template] Phone\n",
    "- src: Link\n",
    "- title: String\n",
    "- [Content]\n",
    "  ```html\n",
    "  <figure><img src=\"{{ [-] this.src }}\" alt=\"{{ [abc] this.title }}\"></figure>\n",
    "  ```\n",
    "\n",
    "# [Section] screens\n",
    "- [Content]\n",
    "  ```html\n",
    "  <section>{{ children }}</section>\n",
    "  ```\n",
    "- [List] gallery source={{ [1.] companion }} item_alias=shot\n",
    "  - [Item]\n",
    "    - [Phone] src={{ [-] shot.src }} title={{ [abc] shot.title }}\n",
    "  - [Empty]\n",
    "    - [Content]\n",
    "      ```html\n",
    "      <p>No screenshots.</p>\n",
    "      ```\n",
);

#[test]
fn the_walk_order() {
    let tree = expand(
        "# [Section] s\n- [Content]\n  ```html\n  <p>{{ [abc] site.title }}</p>\n  ```\n",
        "Section",
        "s",
    );
    let mut calls = Vec::new();
    collapse(&[tree], &mut Recorder, &mut calls).unwrap();
    assert_eq!(
        calls,
        [
            "enter Section",
            "enter Content",
            "literal <p>",
            "value Text(\"Example\")",
            "literal </p>",
            "leave Content",
            "leave Section"
        ]
    );
}

#[test]
fn an_error_from_a_value_gets_the_path_and_slot() {
    let source = "# [Data] d\n- t: boom\n\n# [Section] s\n- [Content]\n  ```html\n  {{ [abc] d.t }}\n  ```\n";
    let tree = expand_with(source, "Section", "s", &[("Data", "d")], &NoSources).unwrap();
    let content = tree.children().next().unwrap().clone();
    let e = collapse(&[content], &mut Recorder, &mut Vec::new()).unwrap_err();
    assert_eq!(
        e.to_string(),
        "[Section] s > [Content]: boom in `{{ [abc] d.t }}`"
    );
}

#[test]
fn io_writer_matches_the_string_writer_and_reports_errors() {
    let tree = expand_with(
        GALLERY,
        "Section",
        "screens",
        &[("Gallery", "companion")],
        &NoSources,
    )
    .unwrap();
    let mut s = String::new();
    collapse(
        std::slice::from_ref(&tree),
        &mut HtmlCollapse::new(),
        &mut s,
    )
    .unwrap();
    let mut io = IoHtmlWriter(Vec::new());
    collapse(
        std::slice::from_ref(&tree),
        &mut HtmlCollapse::new(),
        &mut io,
    )
    .unwrap();
    assert_eq!(String::from_utf8(io.0).unwrap(), s);

    struct Full;
    impl std::io::Write for Full {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("disk full"))
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let e = IoHtmlWriter(Full).raw("x").unwrap_err();
    assert_eq!(
        (e.kind, e.message.as_str()),
        (ErrorKind::Write, "disk full")
    );
}

#[test]
fn render_html_end_to_end() {
    let stream = parse(GALLERY);
    let templates = Templates::from_stream(&stream).unwrap();
    let registry = Registry::new();
    let mut scope = Scope::new();
    scope.bind(
        "companion",
        Value::Record(Record::from_object(section(&stream, "Gallery", "companion")).unwrap()),
    );
    let screens = section(&stream, "Section", "screens");
    let mut cx = Expander::new(&registry, &templates, &NoSources);
    let mut out = String::new();
    // render_html reduces the node itself; a section's body is its children.
    let content = &screens.nodes[0];
    let list = &screens.nodes[1];
    let mut wrapped = String::new();
    render_html(
        list,
        &scope,
        &mut cx,
        &mut HtmlCollapse::new(),
        &mut wrapped,
    )
    .unwrap();
    assert_eq!(
        wrapped,
        "<figure><img src=\"/img/inbox.png\" alt=\"Inbox\"></figure><figure><img src=\"/img/thread.png\" alt=\"A &lt;thread&gt;\"></figure>"
    );
    let trees = cx.expand_children(screens, &scope).unwrap();
    collapse(&trees, &mut HtmlCollapse::new(), &mut out).unwrap();
    assert_eq!(out, format!("<section>{wrapped}</section>"));
    assert!(content.as_object().is_some());
}
