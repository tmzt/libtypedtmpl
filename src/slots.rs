//! The content slot reader: slots in text that typedmd keeps verbatim
//! (paragraphs, `- |` blocks, `html` bodies), read by SPEC.md §3.2's rules
//! for slots in rows, into libtypedmd's own `Slot` values.

use libtypedmd::{AliasTable, Slot};

use crate::error::{Error, ErrorKind};

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Piece {
    Literal(String),
    /// `line` and `column` are 1-based, within the piece of content read.
    Slot {
        slot: Slot,
        line: usize,
        column: usize,
    },
}

/// Split `text` into literal text and slots.
///
/// `{{` opens a slot, closed by the first `}}` on its line; a slot never
/// spans lines or nests. A leading `[token]` that resolves to a sigil types
/// it; any other token stays in the expression. `\{{` is a literal `{{`. An
/// unclosed or empty slot is a `SlotSyntax` error naming its line and
/// column.
pub(crate) fn read(text: &str, aliases: &AliasTable) -> Result<Vec<Piece>, Error> {
    let mut pieces = Vec::new();
    let mut literal = String::new();
    for (index, line) in text.split_inclusive('\n').enumerate() {
        let mut rest = line;
        let mut column = 1;
        while let Some(at) = rest.find("{{") {
            if rest[..at].ends_with('\\') {
                literal.push_str(&rest[..at - 1]);
                literal.push_str("{{");
                column += at + 2;
                rest = &rest[at + 2..];
                continue;
            }
            literal.push_str(&rest[..at]);
            let open = column + at;
            let after = &rest[at + 2..];
            let body_end = after.trim_end_matches(['\n', '\r']).len();
            let Some(close) = after[..body_end].find("}}") else {
                return Err(Error::new(
                    ErrorKind::SlotSyntax,
                    format!(
                        "line {}, column {open}: `{{{{` with no closing `}}}}` on its line",
                        index + 1
                    ),
                ));
            };
            let inner = after[..close].trim();
            if inner.is_empty() {
                return Err(Error::new(
                    ErrorKind::SlotSyntax,
                    format!("line {}, column {open}: an empty slot", index + 1),
                ));
            }
            if !literal.is_empty() {
                pieces.push(Piece::Literal(std::mem::take(&mut literal)));
            }
            pieces.push(Piece::Slot {
                slot: slot(inner, aliases),
                line: index + 1,
                column: open,
            });
            column = open + 2 + close + 2;
            rest = &after[close + 2..];
        }
        literal.push_str(rest);
    }
    if !literal.is_empty() {
        pieces.push(Piece::Literal(literal));
    }
    Ok(pieces)
}

/// A slot's inner text: an optional leading sigil, then the expression.
fn slot(inner: &str, aliases: &AliasTable) -> Slot {
    if let Some(rest) = inner.strip_prefix('[') {
        if let Some(end) = rest.find(']') {
            let after = &rest[end + 1..];
            if after.is_empty() || after.starts_with(char::is_whitespace) {
                if let Some(sigil) = aliases.resolve_sigil(&rest[..end]) {
                    return Slot::typed(sigil, after.trim());
                }
            }
        }
    }
    Slot::new(inner)
}

#[cfg(test)]
mod tests {
    use super::*;
    use libtypedmd::Sigil;

    fn pieces(text: &str) -> Vec<Piece> {
        read(text, &AliasTable::new()).unwrap()
    }

    fn lit(s: &str) -> Piece {
        Piece::Literal(s.into())
    }

    fn at(slot: Slot, line: usize, column: usize) -> Piece {
        Piece::Slot { slot, line, column }
    }

    #[test]
    fn literal_text_alone() {
        assert_eq!(pieces("No slots here.\n"), [lit("No slots here.\n")]);
        assert!(pieces("").is_empty());
    }

    #[test]
    fn typed_and_untyped_slots_with_positions() {
        assert_eq!(
            pieces("Hi {{ [abc] story.title }} and {{ children }}.\nThen {{x}}"),
            [
                lit("Hi "),
                at(Slot::typed(Sigil::String, "story.title"), 1, 4),
                lit(" and "),
                at(Slot::new("children"), 1, 32),
                lit(".\nThen "),
                at(Slot::new("x"), 2, 6),
            ]
        );
    }

    #[test]
    fn every_sigil() {
        for sigil in libtypedmd::SIGILS {
            let text = format!("{{{{ [{}] a.b }}}}", sigil.token());
            assert_eq!(
                pieces(&text),
                [at(Slot::typed(sigil, "a.b"), 1, 1)],
                "{text}"
            );
        }
    }

    #[test]
    fn an_aliased_sigil_and_a_tag_kept_in_the_expression() {
        let mut aliases = AliasTable::new();
        aliases.sigil("str", Sigil::String);
        assert_eq!(
            read("{{ [str] a }}", &aliases).unwrap(),
            [at(Slot::typed(Sigil::String, "a"), 1, 1)]
        );
        assert_eq!(
            pieces("{{ [Panel] c.d }}"),
            [at(Slot::new("[Panel] c.d"), 1, 1)]
        );
        assert_eq!(
            pieces("{{ [abc]x }}"),
            [at(Slot::new("[abc]x"), 1, 1)],
            "no space after the token"
        );
    }

    #[test]
    fn an_escaped_opening_is_literal() {
        assert_eq!(
            pieces("Write \\{{ name }} literally."),
            [lit("Write {{ name }} literally.")]
        );
    }

    #[test]
    fn unclosed_and_empty_slots_are_errors() {
        let e = read("ok\nand {{ open\n}} later", &AliasTable::new()).unwrap_err();
        assert_eq!(e.kind, ErrorKind::SlotSyntax);
        assert!(e.message.starts_with("line 2, column 5:"), "{}", e.message);
        let e = read("x {{   }} y", &AliasTable::new()).unwrap_err();
        assert!(
            e.message.contains("line 1, column 3: an empty slot"),
            "{}",
            e.message
        );
    }
}
