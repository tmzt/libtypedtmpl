//! Errors: a kind, a message, where in the tree, and the slot that caused
//! it.

use std::fmt;

use libtypedmd::Slot;

use crate::path::NodePath;

#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Error {
    pub kind: ErrorKind,
    pub message: String,
    /// From the expansion root down, such as `[Page] home > [List] posts > [Item] #3`.
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

    pub fn at(mut self, path: NodePath) -> Self {
        self.path = path;
        self
    }

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
    /// A consumer's construct, source or collapse.
    Consumer,
}
