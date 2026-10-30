//! `libtypedtmpl`: a small template system over libtypedmd graphs.
//!
//! A typedmd graph is expanded into an inspectable tree by constructs
//! registered by tag, then collapsed into output; HTML is the collapse built
//! in. This release holds the first stage's foundations: values, records read
//! from typedmd objects, slot paths, node paths and errors.

pub mod error;
pub mod path;
pub mod record;
pub mod value;

pub use error::{Error, ErrorKind};
pub use path::{NodePath, Segment, SlotPath};
pub use record::Record;
pub use value::{Date, Html, Value};
