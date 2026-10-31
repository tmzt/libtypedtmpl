//! `[List]`: draws its `[Item]` once per record, or its `[Empty]` when there
//! are none.

use libtypedmd::{LeafForm, Node, NodeKind, Sigil, TextPart};

use crate::error::{Error, ErrorKind};
use crate::expand::{Construct, Expanded, Expander, Part};
use crate::path::is_name;
use crate::scope::Scope;
use crate::template::tag;

pub(crate) struct ListConstruct;

impl Construct for ListConstruct {
    fn tag(&self) -> &str {
        "List"
    }

    fn reduce(
        &self,
        node: &Node,
        scope: &Scope<'_>,
        cx: &mut Expander<'_>,
    ) -> Result<Vec<Expanded>, Error> {
        let shape = |cx: &Expander<'_>, message: String| cx.error(ErrorKind::ListShape, message);
        let object = node.as_object().expect("constructs get objects");
        let name = object.head.as_literal().map(str::trim).unwrap_or("");
        if name.is_empty() {
            return Err(shape(cx, "a `[List]` needs a plain name".into()));
        }
        if !node.payloads.is_empty() {
            return Err(shape(cx, "a `[List]` takes no fenced block".into()));
        }
        let mut source = None;
        let mut alias = None;
        let (mut item, mut empty) = (None, None);
        for n in &node.nodes {
            match &n.kind {
                NodeKind::Member(m) if m.name == "source" => {
                    source = match m.rhs.as_ref().map(|t| t.parts()) {
                        Some([TextPart::Slot(slot)]) if slot.sigil == Some(Sigil::List) => {
                            Some(slot.clone())
                        }
                        _ => return Err(shape(cx, "`source` is one `{{ [1.] … }}` slot".into())),
                    }
                }
                NodeKind::Member(m) if m.name == "item_alias" => {
                    alias = match m.rhs.as_ref().and_then(|t| t.as_literal()).map(str::trim) {
                        Some(a) if is_name(a) => Some(a.to_string()),
                        _ => return Err(shape(cx, "`item_alias` is a bare name".into())),
                    }
                }
                NodeKind::Member(m) => {
                    return Err(shape(cx, format!("a `[List]` has no member `{}`", m.name)))
                }
                NodeKind::Object(_) if tag(n) == "Item" && item.is_none() => item = Some(n),
                NodeKind::Object(_) if tag(n) == "Empty" && empty.is_none() => empty = Some(n),
                NodeKind::Object(_) => {
                    return Err(shape(
                        cx,
                        format!(
                            "a `[List]` holds one `[Item]` and at most one `[Empty]`, not `[{}]`",
                            tag(n)
                        ),
                    ))
                }
                NodeKind::Leaf(leaf) if leaf.form == LeafForm::Row => {
                    return Err(shape(
                        cx,
                        format!("the row `{}` is not part of a list", leaf.text),
                    ))
                }
                NodeKind::Leaf(_) => {}
                NodeKind::Fallback(f) => {
                    return Err(shape(cx, format!("could not read `{}`", f.text)))
                }
            }
        }
        let source = source.ok_or_else(|| shape(cx, "a `[List]` needs `source`".into()))?;
        let alias = alias.ok_or_else(|| shape(cx, "a `[List]` needs `item_alias`".into()))?;
        let item = item.ok_or_else(|| shape(cx, "a `[List]` needs an `[Item]`".into()))?;
        let items = cx.items(&source, scope)?;

        let mut out = Expanded::new(cx.path(), Some("List"), Some(name.to_string()));
        if items.is_empty() {
            if let Some(empty) = empty {
                let arm = cx.within(empty, None, |cx| arm(empty, scope, cx))?;
                out.parts.push(Part::Child(arm));
            }
        } else {
            for (i, value) in items.into_iter().enumerate() {
                let mut item_scope = scope.child();
                item_scope.bind(alias.clone(), value.clone());
                let mut drawn = cx.within(item, Some(i + 1), |cx| arm(item, &item_scope, cx))?;
                drawn.bindings.push((alias.clone(), value));
                out.parts.push(Part::Child(drawn));
            }
        }
        Ok(vec![out])
    }
}

/// An `[Item]` or `[Empty]`: a body, its objects reduced in order.
fn arm(node: &Node, scope: &Scope<'_>, cx: &mut Expander<'_>) -> Result<Expanded, Error> {
    if let Some(m) = node.nodes.iter().find_map(|n| n.as_member()) {
        return Err(cx.error(
            ErrorKind::ArmShape,
            format!(
                "an arm takes no members (`{}`); write an instance or a `[Content]`",
                m.name
            ),
        ));
    }
    if !node.payloads.is_empty() {
        return Err(cx.error(
            ErrorKind::ArmShape,
            "an arm's fenced block belongs inside a `[Content]`",
        ));
    }
    if !node.nodes.iter().any(|n| n.as_object().is_some()) {
        return Err(cx.error(ErrorKind::NothingToDraw, "an arm with nothing to draw"));
    }
    let mut out = Expanded::new(cx.path(), Some(tag(node)), None);
    out.parts = cx
        .expand_children(node, scope)?
        .into_iter()
        .map(Part::Child)
        .collect();
    Ok(out)
}
