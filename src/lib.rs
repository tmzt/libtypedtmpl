//! `libtypedtmpl`: a small template system over libtypedmd graphs.
//!
//! A typed-Markdown document is read with libtypedmd, then turned into
//! output in three stages:
//!
//! 1. **Records.** Data sections are read as [`Record`]s with typed fields
//!    ([`Record::from_object`]) and bound in a [`Scope`].
//! 2. **Expansion.** An [`Expander`] reduces objects in document order into
//!    an [`Expanded`] tree that keeps its structure. Constructs registered
//!    by tag in a [`Registry`] give tags their meaning: `[List]` draws its
//!    `[Item]` once per record or its `[Empty]`, and `[Content]` is the only
//!    node that writes text, as Markdown (feature `markdown`) or one `html`
//!    block, with typedmd slots in it. A `[Template]` defines a type, an
//!    object whose values are types; its instances are checked against it
//!    and reduced to its body, with `[Children]` placing their children.
//! 3. **Collapse.** A [`Collapse`] is the only place the tree is flattened.
//!    [`HtmlCollapse`] writes HTML through an [`HtmlWriter`], escaping
//!    values by sigil and refusing values where escaping cannot make them
//!    safe.
//!
//! ```
//! use libtypedmd::{parse_with, ParseOptions};
//! use libtypedtmpl::{collapse, Expander, HtmlCollapse, NoSources, Record, Registry, Scope, Templates, Value};
//!
//! let doc = parse_with(concat!(
//!     "# [Shots] companion\n",
//!     "- [Shot] inbox\n",
//!     "  - title: Inbox & more\n",
//!     "\n",
//!     "# [Template] Card\n",
//!     "- title: String\n",
//!     "- [Content]\n",
//!     "  ```html\n",
//!     "  <li>{{ [abc] this.title }}</li>\n",
//!     "  ```\n",
//!     "\n",
//!     "# [Section] list\n",
//!     "- [List] shots source={{ [1.] companion }} item_alias=shot\n",
//!     "  - [Item]\n",
//!     "    - [Card] title={{ [abc] shot.title }}\n",
//! ), &ParseOptions::strict()).unwrap();
//! let nodes = &doc.documents[0].nodes;
//!
//! // 1. Records
//! let mut scope = Scope::new();
//! scope.bind("companion", Value::Record(Record::from_object(&nodes[0]).unwrap()));
//!
//! // 2. Expansion
//! let templates = Templates::from_stream(&doc).unwrap();
//! let registry = Registry::new();
//! let mut cx = Expander::new(&registry, &templates, &NoSources);
//! let tree = cx.expand_children(&nodes[2], &scope).unwrap();
//! assert_eq!(tree[0].count("Item"), 1);
//!
//! // 3. Collapse
//! let mut html = String::new();
//! collapse(&tree, &mut HtmlCollapse::new(), &mut html).unwrap();
//! assert_eq!(html, "<li>Inbox &amp; more</li>");
//! ```

#![warn(missing_docs)]

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
    ValueType,
};
pub use html::{render_html, HtmlCollapse};
pub use path::{NodePath, Segment, SlotPath};
pub use record::Record;
pub use scope::Scope;
pub use template::{normalise, FieldDef, FieldType, Templates, TypeDef};
pub use value::{Date, Html, Value};
pub use writer::{escape, is_void, HtmlWriter, IoHtmlWriter};
