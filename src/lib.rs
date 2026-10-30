//! `libtypedtmpl`: a small template system over libtypedmd graphs.
//!
//! A typedmd graph is expanded into an inspectable tree, then collapsed into
//! output. Expansion reduces objects in document order: constructs
//! registered by tag (`[List]`, `[Content]`, `[Children]`) give tags their
//! meaning, a `[Template]` defines a type whose instances are checked
//! against it, and `[Content]` is the only node that writes text, as
//! Markdown or HTML with typedmd slots in it.

mod content;
pub mod error;
pub mod expand;
mod list;
pub mod markdown;
pub mod path;
pub mod record;
pub mod scope;
mod slots;
pub mod template;
pub mod value;

pub use error::{Error, ErrorKind};
pub use expand::{Construct, Expanded, Expander, NoSources, Part, Registry, Sources};
pub use path::{NodePath, Segment, SlotPath};
pub use record::Record;
pub use scope::Scope;
pub use template::{normalise, FieldDef, FieldType, Templates, TypeDef};
pub use value::{Date, Html, Value};
