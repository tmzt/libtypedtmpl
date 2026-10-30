//! Errors: a kind, a message, where in the tree, and the slot that caused
//! it.

use std::fmt;

use libtypedmd::Slot;

use crate::path::NodePath;

/// An error from reading records, expanding or collapsing.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Error {
    /// What went wrong.
    pub kind: ErrorKind,
    /// A sentence saying what went wrong, naming what caused it.
    pub message: String,
    /// From the expansion root down, such as `[Section] intro > [List] items > [Item] #3`.
    pub path: NodePath,
    /// The slot as written back by `Slot::render`, when a slot caused it.
    pub slot: Option<String>,
}

impl Error {
    /// An error with an empty path and no slot.
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            path: NodePath::default(),
            slot: None,
        }
    }

    /// This error at `path`.
    pub fn at(mut self, path: NodePath) -> Self {
        self.path = path;
        self
    }

    /// This error caused by `slot`, written back as it reads.
    pub fn in_slot(mut self, slot: &Slot) -> Self {
        self.slot = Some(slot.render());
        self
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.path.is_empty() {
            f.write_str(&self.message)?;
        } else {
            write!(f, "{}: {}", self.path, self.message)?;
        }
        if let Some(slot) = &self.slot {
            write!(f, " in `{slot}`")?;
        }
        Ok(())
    }
}

impl std::error::Error for Error {}

/// What went wrong. Not exhaustive: later stages add kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ErrorKind {
    /// A value that does not read as its sigil's type.
    BadValue,
    /// A member with neither a value nor children.
    NoValue,
    /// A slot where a record needs plain data.
    SlotInData,
    /// A node a record cannot be read from, or children it cannot hold.
    RecordShape,
    /// Slot text that is not `alias(.field)*`.
    NotAPath,
    /// A field a record does not have.
    UnknownField,
    /// A field asked of a value that is not a record.
    NotARecord,
    /// A slot in content that cannot be read: unclosed, or empty.
    SlotSyntax,
    /// A value whose type does not agree with the slot's sigil, or with a
    /// field's type.
    SigilMismatch,
    /// A `[1.]` slot in content: lists are drawn by `[List]`.
    ListSlot,
    /// A slot whose alias is not in scope.
    UnknownAlias,
    /// A node with nothing to output.
    NothingToDraw,
    /// A node where it does not belong: a plain row, an unreadable span.
    StrayNode,
    /// A `[Content]` that is not Markdown or one `html` block.
    ContentShape,
    /// More than one `html` block in a `[Content]`.
    TwoFragments,
    /// A placeholder Markdown rendering did not keep exactly once.
    SlotLost,
    /// Markdown content without the `markdown` feature.
    MarkdownDisabled,
    /// A `[List]` without its `source`, `item_alias` or `[Item]`.
    ListShape,
    /// An `[Item]` or `[Empty]` that is not a body.
    ArmShape,
    /// A construct outside the place it belongs (`[Item]` outside a list).
    Misplaced,
    /// A list source no one defines.
    UnknownSource,
    /// A list source that is not a list.
    NotAList,
    /// A tag that is neither a construct nor a defined type.
    UnknownType,
    /// A `[Template]` that is not a valid definition.
    BadDefinition,
    /// Two templates for one type.
    DuplicateTemplate,
    /// A type whose body instantiates itself.
    TemplateCycle,
    /// An instance missing a required field.
    MissingField,
    /// An instance given children its type does not place.
    ChildrenNotPlaced,
    /// A link whose scheme is refused (`javascript:`, `data:`, `vbscript:`).
    RefusedScheme,
    /// A value where escaping cannot make it safe: an unquoted attribute,
    /// inside a tag, a comment, a script or a style.
    UnsafePosition,
    /// The writer failed.
    Write,
    /// A void element (`link`, `meta`, `img`, …) given children.
    VoidChildren,
    /// A consumer's construct, source or collapse.
    Consumer,
}
