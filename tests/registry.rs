//! Constructs registered by tag.

mod common;
use common::*;
use libtypedmd::Node;
use libtypedtmpl::{
    Construct, Error, ErrorKind, Expanded, Expander, NoSources, Part, Registry, Scope, Templates,
};

/// A test construct: its children reduced, with literals upper-cased.
struct Upper;
impl Construct for Upper {
    fn tag(&self) -> &str {
        "Upper"
    }
    fn reduce(
        &self,
        node: &Node,
        scope: &Scope<'_>,
        cx: &mut Expander<'_>,
    ) -> Result<Vec<Expanded>, Error> {
        let mut out = cx.expand_children(node, scope)?;
        fn shout(e: &mut Expanded) {
            for part in &mut e.parts {
                match part {
                    Part::Literal(s) => *s = s.to_uppercase(),
                    Part::Child(c) => shout(c),
                    _ => {}
                }
            }
        }
        out.iter_mut().for_each(shout);
        Ok(out)
    }
}

fn run(registry: &Registry, source: &str) -> Result<Vec<Expanded>, Error> {
    let stream = parse(source);
    let templates = Templates::from_stream(&stream)?;
    let node = section(&stream, "Section", "s");
    let mut cx = Expander::new(registry, &templates, &NoSources);
    cx.expand_children(node, &Scope::new())
}

#[test]
fn a_consumer_construct() {
    let mut registry = Registry::new();
    assert!(registry.register(Box::new(Upper)).is_none());
    let out = run(
        &registry,
        "# [Section] s\n- [Upper]\n  - [Content]\n    ```html\n    <b>quiet</b>\n    ```\n",
    )
    .unwrap();
    assert_eq!(out[0].parts, [Part::Literal("<B>QUIET</B>".into())]);
}

#[test]
fn an_empty_registry_knows_no_tags() {
    let e = run(&Registry::empty(), "# [Section] s\n- [Content]\n  x\n").unwrap_err();
    assert_eq!(e.kind, ErrorKind::UnknownType);
}

#[test]
fn replacing_a_construct_returns_the_old_one() {
    struct Nothing;
    impl Construct for Nothing {
        fn tag(&self) -> &str {
            "List"
        }
        fn reduce(
            &self,
            _: &Node,
            _: &Scope<'_>,
            _: &mut Expander<'_>,
        ) -> Result<Vec<Expanded>, Error> {
            Ok(Vec::new())
        }
    }
    let mut registry = Registry::new();
    assert!(registry.register(Box::new(Nothing)).is_some());
    let out = run(&registry, "# [Section] s\n- [List] l source={{ [1.] anything }} item_alias=x\n  - [Item]\n    - [Content]\n      x\n").unwrap();
    assert!(out.is_empty());
}

#[test]
fn arms_and_definitions_refuse_where_they_stand() {
    for tag in ["Item", "Empty"] {
        let e = run(
            &Registry::new(),
            &format!("# [Section] s\n- [{tag}]\n  - [Content]\n    x\n"),
        )
        .unwrap_err();
        assert_eq!(e.kind, ErrorKind::Misplaced, "{tag}");
    }
}
