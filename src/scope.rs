//! Scope: the values slots can read, as a chain of frames.

use crate::value::Value;

/// A frame of bindings whose lookups fall through to its parent. The
/// consumer binds its roots in [`Scope::new`]; a list binds its item alias
/// in a child frame per record; a type's body sees the root frame and
/// `this`, not the frames around its instance.
#[derive(Debug, Default)]
pub struct Scope<'p> {
    parent: Option<&'p Scope<'p>>,
    bindings: Vec<(String, Value)>,
    /// Optional fields of `this` that the instance left out.
    absent: Vec<String>,
}

impl Scope<'static> {
    pub fn new() -> Scope<'static> {
        Scope::default()
    }
}

impl<'p> Scope<'p> {
    /// A frame whose lookups fall through to this one.
    pub fn child(&self) -> Scope<'_> {
        Scope {
            parent: Some(self),
            bindings: Vec::new(),
            absent: Vec::new(),
        }
    }

    pub fn bind(&mut self, alias: impl Into<String>, value: Value) {
        self.bindings.push((alias.into(), value));
    }

    /// Innermost frame first; within a frame, the last binding first.
    pub fn get(&self, alias: &str) -> Option<&Value> {
        self.bindings
            .iter()
            .rev()
            .find(|(name, _)| name == alias)
            .map(|(_, value)| value)
            .or_else(|| self.parent.and_then(|p| p.get(alias)))
    }

    /// The outermost frame: the consumer's roots.
    pub fn root(&self) -> &Scope<'p> {
        let mut scope = self;
        while let Some(parent) = scope.parent {
            scope = parent;
        }
        scope
    }

    pub(crate) fn mark_absent(&mut self, field: impl Into<String>) {
        self.absent.push(field.into());
    }

    /// Whether `this.<field>` is an optional field the instance left out.
    pub(crate) fn is_absent(&self, field: &str) -> bool {
        self.absent.iter().any(|f| f == field) || self.parent.is_some_and(|p| p.is_absent(field))
    }
}
