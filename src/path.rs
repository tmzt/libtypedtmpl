//! Node paths, for locating errors, and slot paths, for reading values.

use std::fmt;

use libtypedmd::Node;

use crate::error::{Error, ErrorKind};
use crate::value::Value;

/// Where a node is: its ancestors' segments, from the expansion root down.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NodePath(Vec<Segment>);

/// One step of a [`NodePath`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segment {
    /// The node's tag.
    pub tag: Option<String>,
    /// The node's head, or a member's name.
    pub name: Option<String>,
    /// 1-based, for a drawn list item.
    pub index: Option<usize>,
}

impl Segment {
    /// A node's tag and plain head (a head with slots is written back as
    /// typed).
    pub fn of(node: &Node) -> Self {
        let (tag, name) = match &node.kind {
            libtypedmd::NodeKind::Object(o) => (
                o.tag.clone().filter(|t| !t.is_empty()),
                Some(o.head.render().trim().to_string()).filter(|h| !h.is_empty()),
            ),
            libtypedmd::NodeKind::Member(m) => (None, Some(m.name.clone())),
            _ => (None, None),
        };
        Self {
            tag,
            name,
            index: None,
        }
    }

    /// This segment, as the `index`th drawn item (1-based).
    pub fn with_index(mut self, index: usize) -> Self {
        self.index = Some(index);
        self
    }
}

impl fmt::Display for Segment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (&self.tag, &self.name) {
            (Some(tag), Some(name)) => write!(f, "[{tag}] {name}")?,
            (Some(tag), None) => write!(f, "[{tag}]")?,
            (None, Some(name)) => f.write_str(name)?,
            (None, None) => f.write_str("(node)")?,
        }
        if let Some(index) = self.index {
            write!(f, " #{index}")?;
        }
        Ok(())
    }
}

impl NodePath {
    /// An empty path: the expansion root.
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether the path has no segments.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The segments, outermost first.
    pub fn segments(&self) -> &[Segment] {
        &self.0
    }

    /// Add an innermost segment.
    pub fn push(&mut self, segment: Segment) {
        self.0.push(segment);
    }

    /// Remove the innermost segment.
    pub fn pop(&mut self) -> Option<Segment> {
        self.0.pop()
    }

    /// This path with one more segment.
    pub fn child(&self, segment: Segment) -> Self {
        let mut path = self.clone();
        path.push(segment);
        path
    }
}

impl fmt::Display for NodePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, segment) in self.0.iter().enumerate() {
            if i > 0 {
                f.write_str(" > ")?;
            }
            write!(f, "{segment}")?;
        }
        Ok(())
    }
}

/// A slot's expression read as a path: `alias(.field)*`, each part
/// `[A-Za-z_][A-Za-z0-9_-]*`, no spaces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlotPath {
    /// The first name: an alias in scope.
    pub alias: String,
    /// The fields after it, in order.
    pub fields: Vec<String>,
}

impl SlotPath {
    /// Read slot text as a path; a `NotAPath` error if it is not one.
    pub fn parse(text: &str) -> Result<Self, Error> {
        let not_a_path = || {
            Error::new(
                ErrorKind::NotAPath,
                format!("`{text}` is not a path (alias.field)"),
            )
        };
        let mut parts = text.split('.');
        let alias = parts.next().filter(|p| is_name(p)).ok_or_else(not_a_path)?;
        let fields = parts
            .map(|p| is_name(p).then(|| p.to_string()).ok_or_else(not_a_path))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            alias: alias.to_string(),
            fields,
        })
    }

    /// Follow the fields from `root`, the alias's value.
    pub fn lookup<'v>(&self, root: &'v Value) -> Result<&'v Value, Error> {
        let mut value = root;
        let mut walked = self.alias.clone();
        for field in &self.fields {
            let Value::Record(record) = value else {
                return Err(Error::new(
                    ErrorKind::NotARecord,
                    format!(
                        "`{walked}` is {}, which has no field `{field}`",
                        value.kind()
                    ),
                ));
            };
            value = record.get(field).ok_or_else(|| {
                Error::new(
                    ErrorKind::UnknownField,
                    format!("`{walked}` has no field `{field}`"),
                )
            })?;
            walked.push('.');
            walked.push_str(field);
        }
        Ok(value)
    }
}

impl fmt::Display for SlotPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.alias)?;
        for field in &self.fields {
            write!(f, ".{field}")?;
        }
        Ok(())
    }
}

/// `[A-Za-z_][A-Za-z0-9_-]*`.
pub(crate) fn is_name(s: &str) -> bool {
    let mut chars = s.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}
