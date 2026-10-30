//! `libtypedtmpl`: a small template system over libtypedmd graphs.
//!
//! A typedmd graph is expanded into an inspectable tree, then collapsed into
//! output. Expansion reduces objects in document order: constructs
//! registered by tag (`[List]`, `[Content]`, `[Children]`) give tags their
//! meaning, a `[Template]` defines a type whose instances are checked
//! against it, and `[Content]` is the only node that writes text, as
//! Markdown or HTML with typedmd slots in it. A [`Collapse`] is the only
//! place the tree is flattened; [`HtmlCollapse`] writes HTML through an
//! [`HtmlWriter`].

pub mod collapse;
mod content;
pub mod error;
pub mod expand;
pub mod html;
mod list;
pub mod markdown;
pub mod path;
pub mod record;
pub mod scope;
mod slots;
pub mod template;
pub mod value;
pub mod writer;

pub use collapse::{collapse, Collapse};
pub use error::{Error, ErrorKind};
pub use expand::{
    Attr, AttrValue, Construct, Element, Expanded, Expander, NoSources, Part, Registry, Sources,
};
pub use html::{render_html, HtmlCollapse};
pub use path::{NodePath, Segment, SlotPath};
pub use record::Record;
pub use scope::Scope;
pub use template::{normalise, FieldDef, FieldType, Templates, TypeDef};
pub use value::{Date, Html, Value};
pub use writer::{escape, is_void, HtmlWriter, IoHtmlWriter};
