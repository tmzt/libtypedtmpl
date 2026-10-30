//! `Collapse`: the only place the expanded tree is flattened, into the
//! output its writer holds.

use libtypedmd::Sigil;

use crate::error::Error;
use crate::expand::{Element, Expanded, Part};
use crate::value::Value;

/// Folds an expanded tree into output, through a writer passed on every
/// call.
pub trait Collapse {
    type Writer: ?Sized;
    fn enter(&mut self, node: &Expanded, out: &mut Self::Writer) -> Result<(), Error>;
    fn literal(&mut self, text: &str, out: &mut Self::Writer) -> Result<(), Error>;
    fn value(
        &mut self,
        sigil: Option<Sigil>,
        value: &Value,
        out: &mut Self::Writer,
    ) -> Result<(), Error>;
    fn start_element(&mut self, element: &Element, out: &mut Self::Writer) -> Result<(), Error>;
    fn end_element(&mut self, element: &Element, out: &mut Self::Writer) -> Result<(), Error>;
    fn leave(&mut self, node: &Expanded, out: &mut Self::Writer) -> Result<(), Error>;
}

/// Walk `trees` in order: enter, each part (literal, value, element with
/// its children, or a child walked the same way), leave. An error with no
/// path gets the node's; one from a value gets its slot.
pub fn collapse<C: Collapse>(
    trees: &[Expanded],
    collapse: &mut C,
    out: &mut C::Writer,
) -> Result<(), Error> {
    for tree in trees {
        walk(tree, collapse, out)?;
    }
    Ok(())
}

fn walk<C: Collapse>(node: &Expanded, c: &mut C, out: &mut C::Writer) -> Result<(), Error> {
    let locate = |e: Error| {
        if e.path.is_empty() {
            e.at(node.path.clone())
        } else {
            e
        }
    };
    c.enter(node, out).map_err(locate)?;
    parts(&node.parts, node, c, out)?;
    c.leave(node, out).map_err(locate)
}

fn parts<C: Collapse>(
    parts: &[Part],
    node: &Expanded,
    c: &mut C,
    out: &mut C::Writer,
) -> Result<(), Error> {
    let locate = |e: Error| {
        if e.path.is_empty() {
            e.at(node.path.clone())
        } else {
            e
        }
    };
    for part in parts {
        match part {
            Part::Literal(text) => c.literal(text, out).map_err(locate)?,
            Part::Value { sigil, value, slot } => c.value(*sigil, value, out).map_err(|e| {
                let e = locate(e);
                if e.slot.is_none() {
                    e.in_slot(slot)
                } else {
                    e
                }
            })?,
            Part::Child(child) => walk(child, c, out)?,
            Part::Element(element) => {
                c.start_element(element, out).map_err(locate)?;
                self::parts(&element.children, node, c, out)?;
                c.end_element(element, out).map_err(locate)?;
            }
        }
    }
    Ok(())
}
