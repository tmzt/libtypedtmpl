//! `[Content]`: the only node that writes text. It holds Markdown (its
//! paragraphs and `- |` blocks) or one `html` block; slots in it are read by
//! [`crate::slots`] unless it has `slots=false`; `{{ children }}` places the
//! objects that follow it.

use libtypedmd::{LeafForm, Node, NodeKind, Sigil, Slot};

use crate::error::{Error, ErrorKind};
use crate::expand::{Construct, Expanded, Expander, Part};
use crate::scope::Scope;
use crate::slots::{read, Piece};
use crate::value::{Html, Value};

pub(crate) struct ContentConstruct;

impl Construct for ContentConstruct {
    fn tag(&self) -> &str {
        "Content"
    }

    fn reduce(
        &self,
        node: &Node,
        scope: &Scope<'_>,
        cx: &mut Expander<'_>,
    ) -> Result<Vec<Expanded>, Error> {
        reduce(node, scope, cx, None).map(|e| vec![e])
    }
}

/// What a `[Content]` holds, checked.
enum Body<'n> {
    Markdown(Vec<&'n str>),
    Html(&'n str),
}

fn body<'n>(node: &'n Node) -> Result<(Body<'n>, bool), Error> {
    let shape = |message: String| Error::new(ErrorKind::ContentShape, message);
    let head = node
        .as_object()
        .map(|o| o.head.render())
        .unwrap_or_default();
    let head = head.trim();
    if head == "|" || head.ends_with(" |") {
        return Err(shape(
            "`|` on a `[Content]` row is not a literal block; put `- |` on its own row under it"
                .into(),
        ));
    }
    let mut slots = true;
    let mut texts = Vec::new();
    for n in &node.nodes {
        match &n.kind {
            NodeKind::Member(m) if m.name == "slots" => {
                slots = match m.rhs.as_ref().and_then(|t| t.as_literal()) {
                    Some("true") => true,
                    Some("false") => false,
                    _ => return Err(shape("`slots` is `true` or `false`".into())),
                }
            }
            NodeKind::Member(m) => {
                return Err(shape(format!("takes only `slots`, not `{}`", m.name)))
            }
            NodeKind::Leaf(leaf) if leaf.form == LeafForm::Row => {
                return Err(shape(format!(
                    "the row `{}` belongs in a `- |` block",
                    leaf.text
                )))
            }
            NodeKind::Leaf(leaf) => texts.push(leaf.text.as_str()),
            NodeKind::Object(_) => return Err(shape("holds no objects".into())),
            NodeKind::Fallback(f) => return Err(shape(format!("could not read `{}`", f.text))),
        }
    }
    let html: Vec<&str> = node
        .payloads
        .iter()
        .filter(|p| p.info.trim() == "html")
        .map(|p| p.body.strip_suffix('\n').unwrap_or(&p.body))
        .collect();
    match (texts.is_empty(), html.as_slice()) {
        (false, []) => Ok((Body::Markdown(texts), slots)),
        (true, [one]) => Ok((Body::Html(one), slots)),
        (false, [_, ..]) => Err(shape(
            "both Markdown and an `html` block; it takes one".into(),
        )),
        (true, []) => Err(shape(
            "is empty: write Markdown, a `- |` block, or an `html` block".into(),
        )),
        (true, _) => Err(Error::new(
            ErrorKind::TwoFragments,
            "more than one `html` block",
        )),
    }
}

/// Whether this `[Content]` places `{{ children }}`, so wraps the objects
/// after it.
pub(crate) fn wraps(node: &Node, aliases: &libtypedmd::AliasTable) -> Result<bool, Error> {
    let Ok((body, slots)) = body(node) else {
        return Ok(false); // reported when it is reduced
    };
    if !slots {
        return Ok(false);
    }
    let texts = match body {
        Body::Markdown(texts) => texts,
        Body::Html(html) => vec![html],
    };
    for text in texts {
        if let Ok(pieces) = read(text, aliases) {
            if pieces
                .iter()
                .any(|p| matches!(p, Piece::Slot { slot, .. } if is_children(slot)))
            {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn is_children(slot: &Slot) -> bool {
    slot.sigil.is_none() && slot.expr == "children"
}

/// Reduce a `[Content]`. `wrapped` holds the reduced objects that follow
/// it, when it places `{{ children }}`.
pub(crate) fn reduce(
    node: &Node,
    scope: &Scope<'_>,
    cx: &mut Expander<'_>,
    wrapped: Option<Vec<Expanded>>,
) -> Result<Expanded, Error> {
    let (body, slots) = body(node)?;
    let name = node
        .as_object()
        .and_then(|o| o.head.as_literal())
        .map(|h| h.trim().to_string());
    let mut out = Expanded::new(cx.path(), Some("Content"), name);
    let mut wrapped = wrapped.unwrap_or_default();
    out.parts = match body {
        Body::Html(html) if !slots => vec![Part::Literal(html.to_string())],
        Body::Html(html) => {
            let mut parts = Vec::new();
            for piece in read(html, cx.aliases())? {
                match piece {
                    Piece::Literal(text) => parts.push(Part::Literal(text)),
                    Piece::Slot { slot, .. } => {
                        parts.extend(place(&slot, scope, cx, &mut wrapped)?)
                    }
                }
            }
            parts
        }
        Body::Markdown(texts) => markdown(&texts, slots, scope, cx, &mut wrapped)?,
    };
    Ok(out)
}

/// The parts a slot in content becomes.
fn place(
    slot: &Slot,
    scope: &Scope<'_>,
    cx: &Expander<'_>,
    wrapped: &mut Vec<Expanded>,
) -> Result<Vec<Part>, Error> {
    if is_children(slot) {
        return Ok(std::mem::take(wrapped)
            .into_iter()
            .map(Part::Child)
            .collect());
    }
    if slot.sigil == Some(Sigil::List) {
        return Err(Error::new(
            ErrorKind::ListSlot,
            "a `[1.]` slot in content: lists are drawn by `[List]`",
        )
        .in_slot(slot));
    }
    let Some(value) = cx.resolve(slot, scope)? else {
        return Ok(Vec::new()); // an optional field left out writes nothing
    };
    let value = match (slot.sigil, value) {
        (Some(_), value) => value,
        (None, value @ Value::Html(_)) => value,
        (None, Value::Markdown(source)) => Value::Html(Html::trusted(
            render_markdown(&source).map_err(|e| e.in_slot(slot))?,
        )),
        (None, other) => {
            return Err(Error::new(
                ErrorKind::SigilMismatch,
                format!(
                    "`{}` is {}; write it with a sigil such as `[abc]`, or place HTML",
                    slot.expr,
                    other.kind()
                ),
            )
            .in_slot(slot))
        }
    };
    Ok(vec![Part::Value {
        sigil: slot.sigil,
        value,
        slot: slot.clone(),
    }])
}

#[cfg(feature = "markdown")]
fn render_markdown(text: &str) -> Result<String, Error> {
    Ok(crate::markdown::render(text))
}

#[cfg(not(feature = "markdown"))]
fn render_markdown(_text: &str) -> Result<String, Error> {
    Err(Error::new(
        ErrorKind::MarkdownDisabled,
        "Markdown needs the `markdown` feature",
    ))
}

/// Markdown content: slots become placeholders, comrak renders, and the
/// HTML is split at the placeholders, so no value is read as Markdown.
fn markdown(
    texts: &[&str],
    slots: bool,
    scope: &Scope<'_>,
    cx: &Expander<'_>,
    wrapped: &mut Vec<Expanded>,
) -> Result<Vec<Part>, Error> {
    let joined = texts.join("\n\n");
    if !slots {
        return Ok(vec![Part::Literal(render_markdown(&joined)?)]);
    }
    let nonce = nonce(&joined);
    let mut source = String::new();
    let mut found: Vec<(String, Slot)> = Vec::new();
    for (i, text) in texts.iter().enumerate() {
        if i > 0 {
            source.push_str("\n\n");
        }
        for piece in read(text, cx.aliases())? {
            match piece {
                Piece::Literal(literal) => source.push_str(&literal),
                Piece::Slot { slot, .. } => {
                    let token = format!("ltt{nonce}s{}z", found.len());
                    source.push_str(&token);
                    found.push((token, slot));
                }
            }
        }
    }
    let mut html = render_markdown(&source)?;
    for (token, slot) in &found {
        if html.matches(token.as_str()).count() != 1 {
            return Err(Error::new(
                ErrorKind::SlotLost,
                "Markdown rendering did not keep this slot in one place",
            )
            .in_slot(slot));
        }
        // A block placed alone in a paragraph takes the paragraph's place.
        if slot.sigil.is_none() {
            let alone = format!("<p>{token}</p>");
            html = html.replacen(&alone, token, 1);
        }
    }
    let mut parts = Vec::new();
    let mut rest = html.as_str();
    loop {
        let next = found
            .iter()
            .filter_map(|(token, slot)| rest.find(token.as_str()).map(|at| (at, token, slot)))
            .min_by_key(|(at, ..)| *at);
        let Some((at, token, slot)) = next else {
            break;
        };
        if at > 0 {
            parts.push(Part::Literal(rest[..at].to_string()));
        }
        parts.extend(place(slot, scope, cx, wrapped)?);
        rest = &rest[at + token.len()..];
    }
    if !rest.is_empty() {
        parts.push(Part::Literal(rest.to_string()));
    }
    Ok(parts)
}

/// 8 hex digits such that the text holds no `ltt<nonce>`.
fn nonce(text: &str) -> String {
    let mut seed: u32 = text.len() as u32 ^ 0x5eed_1e55;
    loop {
        let candidate = format!("{seed:08x}");
        if !text.contains(&format!("ltt{candidate}")) {
            return candidate;
        }
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    }
}
