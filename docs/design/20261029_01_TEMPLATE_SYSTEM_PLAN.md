---
title: "The template system: typedmd graphs expanded by registered constructs into an inspectable tree, then collapsed into HTML"
date: 2026-10-29
status: Draft
covers:
  - "The crate `libtypedtmpl` at the repository root: values and records
    read from typedmd objects; fragments as fenced `html` payloads with
    typedmd slots, read by `libtypedmd::slots::parse_text` (strict)"
  - "Three stages: a typedmd graph is expanded into an `Expanded` tree by
    constructs registered by tag (`trait Construct`, a `Registry`), with
    scope bound, templates inlined and every slot resolved to a `Value`;
    the tree keeps its structure; it is then collapsed into output by a
    `trait Collapse`, the only place anything is flattened"
  - "The built-in constructs: `[List] <name> source={{ [1.] … }}
    item_alias=<name>` with exactly one `[Item]` and at most one `[Empty]`
    arm, and `[Template] <name>` definitions with their references
    (`template=<name>` and `{{ name }}`); named children placed by
    `{{ name }}`, an unplaced child being an error"
  - "`HtmlCollapse`: HTML escaping by sigil, the refused link schemes
    (`javascript:`, `data:`, `vbscript:`), refusal of values in unquoted
    attributes, tags, comments, `<script>` and `<style>`, and the date
    formatter"
  - "The consumer interface (`Sources`, `Scope`, `Templates`, `Registry`,
    `Expander`, `Error` with a node path), docs and an example, and
    release 0.1.0 under Rule 5"
  - "Not covered: conditionals (`[Case]`, `[Else]`; a later plan, which
    this design lets add as one more registered construct, section 5.6);
    any site, page, post or feed vocabulary (the tmzt.dev builder's own
    plan); highbay's own constructs and collapse; collapses other than
    HTML; slots inside `<script>`, `<style>` or unquoted attributes
    (refused, not supported); crates.io publication"
sources:
  - "Owner decisions, 2026-10-29: its own repository; a library with a
    public API, released as 0.1.0 and tagged; depends on libtypedmd 0.2.1
    by git `rev` and nothing else; no template engine; lists, items,
    empties and templates as in the tmzt.dev plan; expansion and collapse
    as three stages with `trait Construct`, a tag registry, an `Expanded`
    tree and `trait Collapse`, `HtmlCollapse` carrying all HTML escaping;
    `[Case]`/`[Else]` left to a later plan; no site or blog vocabulary"
  - "`~/src/tmzt.dev/docs/design/20261029_03_RUST_SITE_PLAN.md` section 4
    (values and records, fragments and slots, lists, items, templates,
    the interface): its content moves here"
  - "`~/src/libtypedmd` SPEC.md §1 (model), §2 (sigils), §3.2 (slots), §5
    (payloads), §7 (strict mode); `libtypedmd/src/model.rs` (`Node`,
    `Object`, `Member`, `Payload`, `Text`, `Slot`)"
  - "`~/src/highbay/RENDER_PIPELINE_DIRECTION.md`: a construct's meaning
    lived in a `match` on tag names in `reduce_constructs`; a pipeline
    should be a walk plus a lookup, the meaning living in the thing looked
    up"
  - "`~/src/highbay/LOFI_HIFI_FINDINGS.md` §2: an authored `<Item>` was
    compiled faithfully, then flattened into positional strings at a model
    boundary and its structure lost"
  - "`~/src/highbay/crates/libhbui/tests/fidelity_is_authored.rs`: tests
    count `Item` and `Empty` in the expanded tree and require the counts
    non-zero"
  - "A sketch of section 5.4's gallery parsed with libtypedmd's `dump`
    example in strict mode, 2026-10-29: `[List]`'s inline properties are
    Members, `[Item]` and `[Empty]` are Objects with an empty head, and a
    fence inside a list item attaches to that item"
related:
  - "`~/src/tmzt.dev/docs/design/20261029_03_RUST_SITE_PLAN.md` (Draft):
    the site builder, whose crate `site_build` (a general site build step)
    is the first consumer; its section 4 moves here, and it is amended to
    say libtypedtmpl is its own repository"
  - "`~/src/libtypedmd/docs/design/20261029_02_SLOT_TEXT_PLAN.md` (Draft):
    `libtypedmd::slots::parse_text`, released as libtypedmd 0.2.1; step 1
    here waits for its `v0.2.1` tag"
---

# The template system

## 1. Goal

`libtypedtmpl` fills fragments of HTML from typed Markdown, with no
template engine. A typedmd section is a record; its fenced `html` block is
a fragment whose typedmd slots (`{{ [abc] post.title }}`) are filled from
records in scope; a `[List]` draws its `[Item]` arm once per record and its
`[Empty]` arm when there are none; a `[Template]` is a named fragment drawn
by reference.

It runs in three stages:

```
typedmd graph  --expand-->  Expanded tree  --collapse-->  output
 (libtypedmd)    constructs     structure kept     Collapse     (String for HTML)
                 by tag         values resolved    impl
```

- **Expand** walks the graph and, per node, looks the node's tag up in a
  `Registry` of constructs. The walk knows no construct by name; the
  meaning of `[List]` is in the `List` construct, as highbay's
  `RENDER_PIPELINE_DIRECTION.md` asks (its `reduce_constructs` was a
  `match` on tag names). Expansion binds scope, inlines templates and
  resolves every slot to a typed `Value`. The result has no constructs
  left in it.
- **The expanded tree** keeps every node it drew, in order, with its
  source path: each drawn `[Item]` is its own node, its fragment's
  literal text, values and child nodes in the order written. Nothing is
  flattened, so tests count `[Item]` and `[Empty]` nodes in it as
  highbay's fidelity tests do, and the failure in highbay's
  `LOFI_HIFI_FINDINGS.md` §2 (an `<Item>` compiled, then flattened into
  positional strings) has no place to happen.
- **Collapse** walks the expanded tree and hands each node, literal and
  value to a `Collapse` implementation. This is the only place output is
  flattened, and every rule about the output format (HTML escaping, link
  schemes, date formatting) lives in the collapse. `HtmlCollapse` is the
  built-in one; a consumer such as highbay supplies its own, and its own
  constructs, without changing this crate.

The crate is generic. Nothing in `src/` names sites, pages, posts, blogs,
feeds or galleries; a test holds this (step 2). Examples below use posts
and a screenshot gallery as illustrations only.

## 2. Crate and dependencies

```
Cargo.toml          [package] name = "libtypedtmpl", edition = "2021"
src/lib.rs          crate docs, re-exports
src/value.rs        Value, Record, Date, Html
src/record.rs       Record::from_object
src/scope.rs        Scope
src/path.rs         NodePath, Segment; slot path parsing and lookup
src/expand.rs       Expanded, Part, Expander, Registry, trait Construct, the fragment rule
src/list.rs         the List construct and its arms
src/template.rs     Templates, the Template construct
src/collapse.rs     trait Collapse, collapse()
src/html.rs         HtmlCollapse
src/error.rs        Error, ErrorKind
tests/              integration tests, one file per area (steps 1 to 3)
examples/gallery.rs the worked example (step 4)
```

One crate at the repository root; no workspace. The one dependency:

```toml
[dependencies]
libtypedmd = { git = "https://github.com/tmzt/libtypedmd", rev = "<the full hash of the commit tagged v0.2.1>" }
```

as libtypedmd's README shows for 0.2.0. Nothing else, including for
tests: dates, escaping and the example use the standard library. The
toolchain is the owner's, rustc 1.94.1. `Cargo.toml` has
`publish = false` (it is used from git) and `version = "0.0.0"` until
step 4; 0.0.0 is the unreleased placeholder and is neither tagged nor
given a `VERSIONS.md` entry.

## 3. Values, records and fragments

### 3.1 Values

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Text(String),        // [abc], and members with no sigil
    Number(String),      // [#], kept as written
    Date(Date),          // [31], YYYY-MM-DD
    Link(String),        // [-]
    Bool(bool),          // [_]
    Currency(String),    // [$], kept as written
    Html(Html),          // trusted markup: a collapsed fragment, a rendered Markdown body
    List(Vec<Record>),   // a source for [List]
    Record(Record),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Html(String);
impl Html {
    pub fn trusted(markup: impl Into<String>) -> Self;
    pub fn as_str(&self) -> &str;
    pub fn into_string(self) -> String;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Date { pub year: i32, pub month: u8, pub day: u8 }
impl Date {
    pub fn parse(text: &str) -> Option<Date>; // YYYY-MM-DD, a real calendar day
}
impl std::fmt::Display for Date { /* YYYY-MM-DD */ }

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Record { name: Option<String>, fields: Vec<(String, Value)> }
impl Record {
    pub fn new(name: Option<String>) -> Self;
    pub fn from_object(node: &libtypedmd::Node) -> Result<Record, Error>;
    pub fn name(&self) -> Option<&str>;
    pub fn push(&mut self, field: impl Into<String>, value: Value); // appended; a later push of the same name wins in get()
    pub fn get(&self, field: &str) -> Option<&Value>;
    pub fn fields(&self) -> &[(String, Value)];
}
```

`Value` is an exhaustive enum on purpose: a `Collapse` matches on it, and
a new variant should break those matches (Rule 5.4 then makes it a minor
release). `Html` is constructed only by `Html::trusted`; the crate never
turns a non-`Html` value into one.

`Date::parse` accepts exactly four digits, `-`, two, `-`, two, with month
1 to 12 and a day that exists in that month (leap years by the Gregorian
rule).

### 3.2 Records from objects

`Record::from_object(&Node)` reads a typedmd Object; any other node kind
is an error. The record's fields are in source order.

- The head is the record's `name`: it must be plain text (no slots); an
  empty head is `None`.
- Each Member child (a row or an inline property, which are the same node)
  is a field:
  - with an RHS and no children: the RHS must be one literal part (slots
    in data are an error). Its sigil picks the variant: none is `Text`,
    `[abc]` is `Text`, `[#]` is `Number` (it must read as `-?[0-9]+(\.[0-9]+)?`),
    `[31]` is `Date` (by `Date::parse`), `[-]` is `Link` (non-empty),
    `[_]` is `Bool` (`true` or `false` exactly), `[$]` is `Currency`
    (non-empty, kept as written). `[1.]` with an RHS is an error. A value
    that does not read as its sigil's type (`- [31] when: soon`) is an
    error naming the member.
  - with children and no RHS: if every child is an Object, a `List` of
    their records (the member may carry `[1.]` or no sigil); if every
    child is a Member, a `Record` with no name holding them (no sigil).
    Mixed children, Leaf children, or any other sigil are errors.
  - with neither an RHS nor children (`- [abc] id`): an error, "no
    value".
  - with both: an error.
- Child Objects of the object itself are its items: a field `items`, a
  `List`, always present (empty when there are none), placed after the
  member fields. A member named `items` is an error.
- Leaf and Fallback children are errors (nothing is dropped silently).
- Payloads and sinks are not read into records; they are the consumer's.

So `[Gallery] companion` with `[Shot]` children is a record whose `items`
are the shots, and a `[List]` with `source={{ [1.] companion }}` draws
them (section 4.3).

`Record::get` finds the last field of that name. `get("name")` with no
field called `name` answers the head, as `Value::Text`, so `shot.name`
reads `[Shot] inbox`'s head.

### 3.3 Fragments

A node's fragment is its payload whose info string, trimmed, is exactly
`html`. A node with two such payloads is an error; payloads with other
info strings are ignored by this crate. The body loses one trailing
newline (the one every fence body ends with), so a fragment placed in the
middle of a line adds no line break of its own.

The body is read with `libtypedmd::slots::parse_text(body,
&ParseOptions::strict())`, so slots follow SPEC.md §3.2 exactly: one
line, an optional leading sigil, `\{{` for a literal `{{`, and an
unclosed `{{` is an error. A parse error becomes an `Error` naming the
line and column within the block.

| Slot | Expansion | Part in the tree |
| --- | --- | --- |
| `{{ [sigil] path }}` for `[abc]`, `[#]`, `[31]`, `[-]`, `[_]`, `[$]` | the value at `path`, checked against the sigil | `Part::Value` |
| `{{ name }}`, a single name | a named child of this node, else a `[Template]` of that name, else an `Html` value bound to `name` | `Part::Child`, or `Part::Value` for `Html` |
| `{{ path.with.fields }}`, no sigil | an `Html` value at the path | `Part::Value` |
| `{{ [1.] … }}` | an error: lists are drawn by `[List]` | none |

A **path** is `alias(.field)*`, each part `[A-Za-z_][A-Za-z0-9_-]*` with
no spaces; the first is an alias in scope (section 4.2), each further one
a field of the record before it. A missing alias, a missing field, a
field of a value that is not a `Record`, or slot text that is not a path
is an error.

**Sigil agreement** is checked at expansion, so the tree is well typed
whatever collapses it:

| Sigil | Accepts |
| --- | --- |
| `[abc]` | `Text`, and also `Number`, `Date`, `Link`, `Bool`, `Currency` (written as text) |
| `[#]` | `Number` |
| `[31]` | `Date` |
| `[-]` | `Link` |
| `[_]` | `Bool` |
| `[$]` | `Currency` |
| none | `Html` |

Any other pairing (`{{ [abc] post.date }}` is fine; `{{ [31] post.title }}`
on `Text`, `{{ [abc] shot }}` on a `Record`, `{{ post.title }}` on `Text`)
is an error naming the slot.

## 4. Expansion

### 4.1 The expanded tree

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct Expanded {
    pub path: NodePath,                 // from the expansion root down to this node
    pub tag: Option<String>,            // the source node's tag: "List", "Item", "Template", "Page", …
    pub name: Option<String>,           // its head, when plain text and not empty
    pub bindings: Vec<(String, Value)>, // aliases this node bound for its contents ([Item]: item_alias → the record)
    pub parts: Vec<Part>,               // in output order
}

#[derive(Debug, Clone, PartialEq)]
pub enum Part {
    Literal(String),
    Value { sigil: Option<Sigil>, value: Value, slot: Slot },
    Child(Expanded),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NodePath(Vec<Segment>);
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segment { pub tag: Option<String>, pub name: Option<String>, pub index: Option<usize> }

impl Expanded {
    pub fn descendants(&self) -> impl Iterator<Item = &Expanded>; // depth first, self excluded
    pub fn count(&self, tag: &str) -> usize;                      // descendants with that tag
}
impl std::fmt::Display for Expanded { /* the outline below */ }
impl std::fmt::Display for NodePath { /* "[Page] home > [List] posts > [Item] #2" */ }
```

`Sigil` and `Slot` are libtypedmd's. `Segment` displays as `[Tag] name`,
`[Tag]` with no name, `name` with no tag, followed by ` #n` (1-based) when
it has an index: the drawn `[Item]`s of a list are indexed, so an error in
the third record names `[Item] #3`.

What each node holds:

- A node drawn by its fragment: its literal text, values and placed
  children, in the order the block writes them.
- A `[List]`: one `Child` per drawn arm, in record order (each an `[Item]`
  with its index and its binding), or one `[Empty]` child, or no parts.
  It has no literal text of its own.
- An `[Item]` or `[Empty]` drawn by `template=`: one `Child`, the
  `[Template] <name>` node.
- A template drawn by `{{ name }}`: a `Child` whose segment is
  `[Template] <name>`.

`Display` writes one line per node, two spaces of indent per level, for
tests and debugging:

```
[Section] screens
  "<section>"
  [List] gallery
    [Item] #1 shot=inbox
      [Template] phone
        "<figure><img src=\""
        [-] /img/inbox.png
        "\" alt=\""
        [abc] Inbox
        "\"></figure>"
  "</section>"
```

Literals are quoted with `\"`, `\\` and `\n` escaped; values are the
sigil (or `html`) and the value written plainly; bindings show the alias
and the record's name (its head), or `{…}` for a record with no name.

### 4.2 Scope

```rust
#[derive(Debug, Default)]
pub struct Scope<'p> { parent: Option<&'p Scope<'p>>, bindings: Vec<(String, Value)> }
impl<'p> Scope<'p> {
    pub fn new() -> Scope<'static>;
    pub fn child(&self) -> Scope<'_>;                        // a frame whose lookups fall through to self
    pub fn bind(&mut self, alias: impl Into<String>, value: Value);
    pub fn get(&self, alias: &str) -> Option<&Value>;      // innermost frame first, last binding in a frame first
}
```

The consumer binds its roots (`site`, `page`, a data section by name) in
`Scope::new()`. A `[List]` binds `item_alias` in a child frame per record,
for its `[Item]`'s fragment and everything inside it. An inner alias of
the same name shadows an outer one inside the item, and is gone after it.

### 4.3 Constructs and the registry

```rust
pub trait Construct {
    /// The tag this construct gives meaning to, e.g. "List".
    fn tag(&self) -> &str;
    /// Expand `node` (whose tag is `tag()`) in `scope`, into the nodes that
    /// stand where it was placed: zero, one or several.
    fn reduce(&self, node: &Node, scope: &Scope<'_>, cx: &mut Expander<'_>) -> Result<Vec<Expanded>, Error>;
}

pub struct Registry { /* Box<dyn Construct> by tag */ }
impl Registry {
    pub fn new() -> Self;   // the built-ins: List, Item, Empty, Template
    pub fn empty() -> Self; // none
    /// Register by `construct.tag()`, replacing and returning any construct with that tag.
    pub fn register(&mut self, construct: Box<dyn Construct>) -> Option<Box<dyn Construct>>;
    pub fn get(&self, tag: &str) -> Option<&dyn Construct>;
}

pub trait Sources {
    /// Resolve a [List] source expression that is not a path in scope.
    /// Ok(None): not an expression this consumer defines.
    fn list(&self, expr: &str) -> Result<Option<Vec<Record>>, String>;
}
pub struct NoSources; // answers Ok(None)

pub struct Expander<'a> { /* &Registry, &Templates, &dyn Sources, the path stack, the template stack */ }
impl<'a> Expander<'a> {
    pub fn new(registry: &'a Registry, templates: &'a Templates, sources: &'a dyn Sources) -> Self;

    /// Expand one node: its tag's construct if one is registered, otherwise
    /// the fragment rule (section 4.4).
    pub fn expand(&mut self, node: &Node, scope: &Scope<'_>) -> Result<Vec<Expanded>, Error>;

    // For constructs:
    /// Draw `node` by its own html block (the fragment rule), as one node.
    pub fn fragment(&mut self, node: &Node, scope: &Scope<'_>) -> Result<Expanded, Error>;
    /// Draw the [Template] `name` in `scope`, as one node.
    pub fn template(&mut self, name: &str, scope: &Scope<'_>) -> Result<Expanded, Error>;
    /// Run `f` with `node` (and an index) pushed on the path, so nodes it
    /// builds and errors it raises carry it.
    pub fn within<T>(&mut self, node: &Node, index: Option<usize>,
                     f: impl FnOnce(&mut Self) -> Result<T, Error>) -> Result<T, Error>;
    pub fn path(&self) -> &NodePath;
    /// The value at a slot path (section 3.3).
    pub fn resolve(&self, path: &str, scope: &Scope<'_>) -> Result<Value, Error>;
    /// A [List] source expression's records (section 4.5).
    pub fn records(&self, expr: &str, scope: &Scope<'_>) -> Result<Vec<Record>, Error>;
    /// An error at the current path.
    pub fn error(&self, kind: ErrorKind, message: impl Into<String>) -> Error;
}
```

`Expander::expand` pushes the node on the path, then either calls the
construct registered for the node's tag or applies the fragment rule.
That lookup is the walk's only knowledge of tags; the built-ins are
registered like any consumer's construct and can be replaced.

Built-in constructs:

| Tag | Construct |
| --- | --- |
| `List` | section 4.5: reads its own `[Item]` and `[Empty]` arms. |
| `Item`, `Empty` | refuse: an arm outside a `[List]` is an error. The `List` construct reads its arms itself and never dispatches them. |
| `Template` | refuses where it stands: a `[Template]` met during expansion is not top level. Definitions are collected by `Templates` (section 4.6) and drawn through `Expander::template`, which `template=` and `{{ name }}` both use. |

### 4.4 The fragment rule: nodes with a block, and placement

A node with no registered construct is drawn by its `html` block; with no
block it is an error ("nothing to draw"). Its children:

- **Member** children are not drawn and not checked; they are the
  consumer's (`- title: …`).
- **Object** children are its named children. Each needs a plain-text,
  non-empty head, its name; two with the same name are an error. Each is
  drawable (a registered construct, or a node with a block) or it is an
  error. Each must be placed exactly once by `{{ name }}` in the block: a
  child never placed, or placed twice, is an error, so nothing is dropped
  silently and the tree stays a tree.
- **Leaf** and **Fallback** children are errors.

A child is expanded in the parent's scope, at the point its slot is read,
through `Expander::expand`, so a placed `[List]` goes to the `List`
construct and a placed node with a block to this rule again. Its nodes
become `Part::Child` parts at that place.

An unsigiled single name resolves to a named child first, then a
template, then an `Html` value in scope. A child therefore shadows a
template of the same name; that is allowed.

### 4.5 `[List]`, `[Item]`, `[Empty]`

```markdown
- [List] posts source={{ [1.] posts.recent 10 }} item_alias=post
  - [Item]
    ```html
    <article><a href="{{ [-] post.url }}">{{ [abc] post.title }}</a></article>
    ```
  - [Empty]
    ```html
    <p>No posts yet.</p>
    ```
- [List] gallery source={{ [1.] companion }} item_alias=shot
  - [Item] template=phone
```

- `[List] <name>` has exactly two members, both required: `source`, whose
  RHS is exactly one slot with the sigil `[1.]`, and `item_alias`, a bare
  name (`[A-Za-z_][A-Za-z0-9_-]*`). Any other member, a missing one, a
  head with slots, or an `html` block on the `[List]` is an error.
- Its Object children are exactly one `[Item]` and at most one `[Empty]`,
  in either order. Anything else (a second `[Item]`, another tag, a Leaf)
  is an error.
- **Source.** The slot's expression is resolved by `Expander::records`:
  if it is a path whose alias is in scope, its value must be a `List`
  (its records) or a `Record` (its `items` field, which must be a
  `List`); otherwise it goes to `Sources::list`. `Ok(None)` there is an
  error, "unknown source", naming the expression; `Err(message)` is an
  error with that message. So `companion` (a data record bound by the
  consumer) and `posts.recent 10` (a name the consumer defines) both
  work.
- **Arms.** `[Item]` is drawn once per record, in order, in a child scope
  binding `item_alias` to `Value::Record(record)`; the drawn node has the
  index (1-based) on its segment and the binding in `bindings`. `[Empty]`
  is drawn, in the list's scope, when there are no records; with no
  `[Empty]`, an empty list draws a `[List]` node with no parts.
- Each arm has either a `template=<name>` member (and nothing else: no
  other member, no children, no block) or its own `html` block, drawn by
  the fragment rule with its own named children, a nested `[List]`
  among them. Both, or neither, is an error.
- The `List` construct returns one node, the `[List]`, holding the drawn
  arms. An `[Item]` is drawn whole: its block's literals, values and
  children all stay in it, in order.

### 4.6 Templates

```rust
pub struct Templates { /* [Template] nodes by name */ }
impl Templates {
    pub fn new() -> Self;
    /// Every top-level [Template] in every document of `stream`.
    pub fn from_stream(stream: &libtypedmd::Stream) -> Result<Self, Error>;
    /// Add one [Template] node.
    pub fn add(&mut self, node: &libtypedmd::Node) -> Result<(), Error>;
    pub fn get(&self, name: &str) -> Option<&libtypedmd::Node>;
}
```

A `[Template] <name>` needs a plain-text head and an `html` block; it may
have named children placed in its block, like any node with a block. Two
templates of one name are an error. A template is drawn in the scope of
the place that refers to it. A template that refers to itself, directly
or through others, is an error naming the cycle
(`phone > frame > phone`).

### 4.7 Errors

```rust
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Error {
    pub kind: ErrorKind,
    pub message: String,
    pub path: NodePath,        // e.g. [Page] home > [List] posts > [Item] #3
    pub slot: Option<String>,  // the slot as written back by Slot::render, when a slot caused it
}
impl Error {
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self; // empty path, no slot
    pub fn at(self, path: NodePath) -> Self;
    pub fn in_slot(self, slot: &Slot) -> Self;
}
impl std::fmt::Display for Error { /* "{path}: {message}", then " in `{slot}`" when there is one */ }
impl std::error::Error for Error {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ErrorKind {
    // records (3.2)
    BadValue, NoValue, SlotInData, RecordShape,
    // fragments and slots (3.3)
    SlotSyntax, TwoFragments, NotAPath, UnknownAlias, UnknownField, NotARecord,
    SigilMismatch, ListSlot, UnknownName,
    // placement (4.4)
    NothingToDraw, Unnamed, DuplicateName, NotDrawable, NotPlaced, PlacedTwice, StrayNode,
    // constructs (4.3, 4.5, 4.6)
    ListShape, ArmShape, Misplaced, UnknownSource, NotAList,
    UnknownTemplate, DuplicateTemplate, TemplateCycle,
    // collapse (5)
    RefusedScheme, UnsafePosition,
    // a consumer's construct, source or collapse
    Consumer,
}
```

Payloads carry no line numbers in libtypedmd 0.2, so the node path is how
an error is found; a slot syntax error adds the line and column within the
block to the message. Every error kind has a test (steps 1 to 3).

## 5. Collapse

### 5.1 The trait and the walk

```rust
pub trait Collapse {
    type Output;
    fn enter(&mut self, node: &Expanded) -> Result<(), Error>;
    fn literal(&mut self, text: &str) -> Result<(), Error>;
    fn value(&mut self, sigil: Option<Sigil>, value: &Value) -> Result<(), Error>;
    fn leave(&mut self, node: &Expanded) -> Result<(), Error>;
    fn finish(self) -> Self::Output;
}

/// Walk `trees` in order: enter, each part (literal, value, or a child
/// walked the same way), leave; then finish.
pub fn collapse<C: Collapse>(trees: &[Expanded], collapse: C) -> Result<C::Output, Error>;
```

The trait is as the owner gave it. A `Collapse` raises errors with
`Error::new`; `collapse()` fills in the path of the node being walked and,
for an error from `value`, the slot, when the error has none. That keeps
`value`'s signature small and every error located.

### 5.2 `HtmlCollapse`

```rust
pub struct HtmlCollapse { /* output, the HTML position state, the date formatter */ }
impl HtmlCollapse {
    pub fn new() -> Self;   // dates written YYYY-MM-DD
    pub fn with_dates(self, format: impl Fn(&Date) -> String + 'static) -> Self;
}
impl Collapse for HtmlCollapse { type Output = String; /* … */ }

/// Expand `node` and collapse it to HTML: the two stages in one call.
pub fn render_html(node: &Node, scope: &Scope<'_>, cx: &mut Expander<'_>, html: HtmlCollapse) -> Result<String, Error>;
```

`enter` and `leave` write nothing. `literal` writes its text unchanged.
`value` writes:

| Sigil | Value | Written as |
| --- | --- | --- |
| `[abc]` | `Text`, `Number`, `Currency`, `Link` | the text, escaped |
| `[abc]` | `Date` | `YYYY-MM-DD`, escaped |
| `[abc]` | `Bool` | `true` or `false` |
| `[#]`, `[$]` | `Number`, `Currency` | as written, escaped |
| `[31]` | `Date` | the date formatter's text, escaped |
| `[_]` | `Bool` | `true` or `false` |
| `[-]` | `Link` | escaped; refused if its scheme is `javascript:`, `data:` or `vbscript:` |
| none | `Html` | as is |

Escaping replaces `&`, `<`, `>`, `"` and `'` with `&amp;`, `&lt;`, `&gt;`,
`&quot;` and `&#39;`. A link's scheme is checked on the link with leading
and trailing whitespace trimmed and ASCII tab, newline and carriage return
removed (browsers ignore them inside a scheme), compared case-insensitively;
`JavaScript:`, ` data:` and `java\tscript:` are all refused
(`ErrorKind::RefusedScheme`). Any other pairing of sigil and value
reaching `HtmlCollapse` (only possible from a hand-built tree) is a
`SigilMismatch` error.

**Positions.** Escaping is safe in text and in quoted attribute values,
and only there. `HtmlCollapse` tracks where the output stands with a small
state machine over the literal text it has written: text, inside a tag,
inside a double- or single-quoted attribute value, inside a comment
(`<!--` to `-->`), or inside the raw text of a `<script>` or `<style>`
element (from the end of its start tag to `</script` or `</style>`,
case-insensitive). A value is written only in text or a quoted attribute
value, and an `Html` value only in text; anywhere else it is an
`UnsafePosition` error. This refuses `href={{ [-] … }}` (unquoted),
`<a {{ [abc] … }}>`, and any slot in a script, a style or a comment.
`Html` values are trusted and do not move the state.

**The date formatter belongs here.** In the tmzt.dev plan, `Sources`
carried `fn date(&Date) -> String`. Formatting a date is output, so it
moves to the collapse: `HtmlCollapse::with_dates`. `Sources` keeps only
list expressions, which are expansion.

### 5.3 Other consumers

A consumer that is not HTML implements `Collapse` with its own `Output`
(a draw list, a widget tree) and reads the expanded tree's tags, names,
bindings and parts. It registers its own constructs with
`Registry::register`. Neither needs a change here.

### 5.4 Example

````markdown
# [Gallery] companion
- [Shot] inbox
  - title: Inbox
  - [-] src: /img/inbox.png
- [Shot] thread
  - title: A thread
  - [-] src: /img/thread.png

# [Template] phone
```html
<figure><img src="{{ [-] shot.src }}" alt="{{ [abc] shot.title }}"></figure>
```

# [Section] screens
- [List] gallery source={{ [1.] companion }} item_alias=shot
  - [Item] template=phone
  - [Empty]
    ```html
    <p>No screenshots.</p>
    ```
```html
<section>{{ gallery }}</section>
```
````

The consumer binds `companion` to `Record::from_object` of the
`[Gallery]`, builds `Templates::from_stream`, and expands `[Section]
screens`: a tree with one `[List]` holding two `[Item]`s, each holding a
`[Template] phone`. `HtmlCollapse` writes
`<section><figure><img src="/img/inbox.png" alt="Inbox"></figure><figure>…</figure></section>`.

### 5.5 Changes from the owner's signatures

- `Construct::reduce` takes `&Scope<'_>` and `&mut Expander<'_>`: `Scope`
  is a chain of frames borrowed from its parent, so binding an alias per
  record costs one small frame instead of a copy of the whole scope.
- `Collapse` is unchanged. Errors are located by the `collapse()` walk
  (section 5.1) instead of by more arguments to `value`.
- `HtmlCollapse::Output` is `String`, as given; a consumer wraps it with
  `Html::trusted` to bind it as a value (for example as a layout's
  `main`).
- From the tmzt.dev plan's section 4.4: `render(node, scope, templates,
  sources)` becomes `Expander::expand` plus `collapse`, with
  `render_html` as the one-call form; `Sources::date` moves to
  `HtmlCollapse::with_dates`; `Sources::list` takes `Result<Option<…>,
  String>` so a consumer can reject a malformed expression it owns
  (`posts.recent ten`) instead of having it reported as unknown;
  `Error::path` is a `NodePath` (displayed as the string the old plan
  showed) and `Error` gains a `kind`; `Scope::bind` stays.

### 5.6 How `[Case]` and `[Else]` fit later

A conditional is one more `Construct`. Like `[List]`, it owns its arms as
children (for instance `[Case] <name> value={{ … }}` with arm children,
`[Else]` last), reads its condition with `Expander::resolve`, and returns
the chosen arm's nodes through `Expander::fragment` or `template`, or
none (`reduce` returns a `Vec` for that reason). It is registered with
`Registry::register`; nothing in the walk or the collapse changes.
Highbay's spelling, sibling `<Case>`/`<Else>` under a `<Switch>`, is the
same shape with the parent as the construct. A form where a construct
must see its following siblings would need `reduce` to receive them; that
is the later plan's question (section 7).

## 6. Steps

Every code step is one commit of the crate and passes
`cargo fmt -p libtypedtmpl -- --check`, `cargo clippy -p libtypedtmpl
--all-targets` with no warnings, and `cargo test -p libtypedtmpl`
(Rule 3.4). Tests build their
graphs by parsing small typedmd documents with
`libtypedmd::parse_with(…, &ParseOptions::strict())`.

### Step 1: The crate, values and records

Waits for libtypedmd's `v0.2.1` tag. `Cargo.toml` (section 2, with the
`rev` of the commit tagged `v0.2.1`), `src/lib.rs`, `value.rs`,
`record.rs`, `path.rs` (path parsing and `NodePath`) and `error.rs`.
This step's first commit sets this plan to `In implementation` with its
amendment (Rule 1.5, Rule 3.2).

Tests (`tests/records.rs`): every sigil read into its variant, and each
rejected value (`[#] 1x`, `[31] 2026-02-30`, `[_] yes`, an empty `[-]`,
`[1.]` with an RHS); a member with no value; slots in data; nested member
records; member lists of objects; mixed children; `items` (present and
empty, in order, and the `items` member conflict); leaves refused;
`get("name")` answering the head; the last field winning; `Date::parse`
on leap days; path parsing (good paths, spaces, empty parts).

**Done when:** `cargo test -p libtypedtmpl` passes with these cases and
the gates are clean.

### Step 2: Expansion

`scope.rs`, `expand.rs`, `list.rs`, `template.rs`: the expanded tree,
`Display`, scope, registry, `Construct`, `Expander`, the fragment rule,
`[List]`/`[Item]`/`[Empty]`, `Templates`. All tests here look at the
**expanded tree**, never at output.

Tests:

- `tests/lists.rs`: three records give a `[List]` with exactly three
  `[Item]` children, indexed 1 to 3, each binding its record, and no
  `[Empty]`; no records give one `[Empty]` and no `[Item]`; no records and
  no `[Empty]` give a `[List]` with no parts; counts asserted non-zero
  where something is expected (`Expanded::count`). Sources: a path to a
  `List`, a path to a `Record` (its `items`), `Sources::list` answering,
  `Ok(None)` and `Err`.
- `tests/structure.rs`, the highbay finding: an `[Item]` whose block has
  four values and a nested named child keeps all four values and the
  child, in the order written, with the literal text between them; two
  lists side by side stay two `[List]` nodes; the `Display` outline of the
  section 5.4 example, compared whole.
- `tests/scope.rs`: nested lists (shots grouped under galleries) where the
  inner item reads both aliases; an inner alias shadowing an outer one;
  an inner alias used after its list (an `UnknownAlias` error); the
  `[Empty]` arm not seeing the item alias.
- `tests/templates.rs`: `template=` and an inline block drawing the same
  output tree shape apart from the `[Template]` node; `{{ name }}`
  drawing a template; a template with a nested list; a child shadowing a
  template; `Templates::from_stream` with two documents.
- `tests/registry.rs`: a consumer construct (a test-only `[Upper]` that
  returns its fragment's node with literals upper-cased) registered and
  placed; `Registry::empty()` making `[List]` an ordinary node (then
  `NothingToDraw`); replacing `List` returns the old one.
- `tests/errors.rs`: one test per `ErrorKind` of sections 3 and 4, each
  checking the kind, the displayed path (including an `[Item] #n`
  index) and the slot text where a slot caused it.
- `tests/agnostic.rs`: no word of `site`, `page`, `post`, `posts`,
  `blog`, `feed`, `gallery`, `highbay` (whole words, any case) in
  `src/`.

**Done when:** `cargo test -p libtypedtmpl` passes with every case above,
and no test of this step reads collapsed output.

### Step 3: Collapse and HTML

`collapse.rs` and `html.rs`. Tests (`tests/html.rs`, `tests/collapse.rs`):

- each row of section 5.2's table, with each of `& < > " '` escaped in
  text and in a quoted attribute, and `Html` inserted unescaped;
- each refused scheme in several spellings (`javascript:`,
  `JAVASCRIPT:`, `  data:text/html,…`, `vbscript:`, `java\tscript:`), and
  allowed links (`/a`, `https://…`, `mailto:…`, `#top`, a relative path
  containing `data:` after a `/`);
- `UnsafePosition` for an unquoted attribute, a slot inside a tag, inside
  a comment, inside `<script>` and `<STYLE>`; and a value after
  `</script>` allowed; an `Html` value inside a quoted attribute refused;
- `with_dates` used for `[31]` and not for `[abc]` on a date;
- a test `Collapse` that records its calls, showing the walk order
  (enter, parts, leave, nested) and that an error from `value` comes back
  with the node path and the slot filled in;
- the section 5.4 example end to end with `render_html`.

**Done when:** `cargo test -p libtypedtmpl` passes with every case above.

### Step 4: Docs, example and 0.1.0

1. Crate commit: rustdoc on every public item, the crate docs (the three
   stages, a short example that runs as a doctest), and
   `examples/gallery.rs` printing section 5.4's tree outline and its
   HTML. `cargo doc --no-deps` gives no warnings.
2. README commit: a Usage section (the Cargo dependency by `rev`, the
   three stages, the example) between the introduction and the decision
   table.
3. Version commit: `version = "0.1.0"` in `Cargo.toml`, with the
   `VERSIONS.md` entry `## 0.1.0 — <date> (<short commit>, tag v0.1.0)`
   listing the initial public API (Rule 5.3); tag that commit `v0.1.0`
   at once (Rule 5.1). As in libtypedmd's entries, the short commit named
   is the last one before the version commit, which cannot name itself.
4. An amendment setting this plan to `Implemented`, listing each step's
   commits, with its README row, in the same commit.

**Done when:** `cargo run --example gallery` prints the outline and the
HTML of section 5.4, `v0.1.0` is tagged on the version commit, and
`VERSIONS.md` has its entry.

## 7. Open questions

1. **Licence.** libtypedmd is MIT OR Apache-2.0 with `LICENSE-MIT` and
   `LICENSE-APACHE`. Use the same here (a repository-level commit before
   step 4, and `license` in `Cargo.toml`)?
2. **Fragment info string.** Only `html` is a fragment. Should a consumer
   whose output is not HTML (highbay) be able to name its own info
   string, or is `html` fixed for 0.1?
3. **Binding a tree, not a string.** A layout places the page's body as
   an `Html` value, so it is collapsed before it is placed and its
   structure is not in the layout's tree. Should 0.1 let the consumer
   pass extra named children to `Expander::fragment` as `Expanded` nodes
   (keeping the structure), or is `Html` enough until a consumer needs
   it?
4. **Strictness of the fragment rule.** Child Objects that are neither a
   construct nor a node with a block are errors under a node with a
   block, and so are prose Leaves. Is that wanted, or should data
   objects under a drawn node be allowed and ignored?
5. **Placed once.** A child placed twice is an error here. Should drawing
   the same named child twice be allowed?
6. **Lists of plain values.** A member list of leaves (`- tags:` with
   `- rust`, `- go`) is an error in records; drawing it needs a value
   type for "list of text". Leave it for a later plan?
7. **Sibling-aware constructs.** Section 5.6: if the later conditional
   plan wants highbay's sibling `[Case]`/`[Else]` without a parent, is a
   change to `reduce` (and so a minor version) acceptable then?

## Amendment — 2026-10-29 21:46 EDT: elements, a writer passed to `Collapse`, `HtmlWriter`, and HTML head constructs

Agreed with the owner on 2026-10-29. Where this amendment and the text
above disagree, this amendment wins.

### A. Element nodes in the expanded tree (adds to section 4.1)

Constructs may produce elements, not only fragment text, so that a
consumer's collapse receives structure (a tag and typed attributes) rather
than a string of HTML. `Part` gains a variant:

```rust
pub enum Part {
    Literal(String),
    Value { sigil: Option<Sigil>, value: Value, slot: Slot },
    Child(Expanded),
    Element(Element),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Element {
    pub name: String,              // "link", "meta", "script", …
    pub attrs: Vec<Attr>,          // in the order written
    pub children: Vec<Part>,       // e.g. a <title>'s text
}

#[derive(Debug, Clone, PartialEq)]
pub struct Attr { pub name: String, pub value: AttrValue }

#[derive(Debug, Clone, PartialEq)]
pub enum AttrValue {
    Value { sigil: Option<Sigil>, value: Value }, // escaped and checked by the collapse
    Flag,                                         // a boolean attribute: written as its name
}
```

`Display` writes an element as `<name attr=value …>` on its own line,
its children indented below it.

### B. `Collapse` takes its writer on every call (replaces section 5.1)

```rust
pub trait Collapse {
    type Writer: ?Sized;
    fn enter(&mut self, node: &Expanded, out: &mut Self::Writer) -> Result<(), Error>;
    fn literal(&mut self, text: &str, out: &mut Self::Writer) -> Result<(), Error>;
    fn value(&mut self, sigil: Option<Sigil>, value: &Value, out: &mut Self::Writer) -> Result<(), Error>;
    fn start_element(&mut self, element: &Element, out: &mut Self::Writer) -> Result<(), Error>;
    fn end_element(&mut self, element: &Element, out: &mut Self::Writer) -> Result<(), Error>;
    fn leave(&mut self, node: &Expanded, out: &mut Self::Writer) -> Result<(), Error>;
}

/// Walk `trees` in order: enter, each part (literal, value, element with
/// its children, or a child walked the same way), leave.
pub fn collapse<C: Collapse>(trees: &[Expanded], collapse: &mut C, out: &mut C::Writer) -> Result<(), Error>;
```

`Output` and `finish` are removed: the output is whatever the writer
holds. The walk still fills in the node path and slot of an error, as
section 5.1 says.

### C. `HtmlWriter` (adds to section 5.2)

```rust
/// Writes HTML. Text and attribute values are escaped here; raw is not.
pub trait HtmlWriter {
    fn start(&mut self, name: &str, attrs: &[(&str, Option<&str>)]) -> Result<(), Error>; // None: a flag
    fn end(&mut self, name: &str) -> Result<(), Error>;   // nothing for a void element
    fn text(&mut self, text: &str) -> Result<(), Error>;  // escaped
    fn raw(&mut self, html: &str) -> Result<(), Error>;   // as is
}
impl HtmlWriter for String { … }
pub struct IoHtmlWriter<W: std::io::Write>(pub W);
impl<W: std::io::Write> HtmlWriter for IoHtmlWriter<W> { … }  // io errors become ErrorKind::Write
```

- Escaping in `text` and attribute values is section 5.2's: `& < > " '`.
- Void elements (`area base br col embed hr img input link meta source
  track wbr`) get no end tag; `end` writes nothing for them, and a void
  element with children is an `ErrorKind::VoidChildren` error.
- `ErrorKind` gains `Write` and `VoidChildren` (it is not exhaustive).

`HtmlCollapse` (section 5.2) keeps everything it decides, the position
state machine, checking a value against its sigil, refused link schemes
and the date formatter, and writes through the writer:
`impl Collapse for HtmlCollapse { type Writer = dyn HtmlWriter; }`.

- `literal` advances the position state and calls `out.raw`.
- `value` checks the position, then writes text through `out.text` (the
  writer escapes) or an `Html` value through `out.raw`.
- `start_element` requires the position to be text (otherwise
  `UnsafePosition`), checks each attribute value as `value` would (a
  `[-]` value's scheme, the sigil), formats dates, and calls
  `out.start`; `end_element` calls `out.end`. An element's children
  are walked between them; the position state treats `<script>` and
  `<style>` elements as it treats them in literal text.

`render_html` becomes
`render_html(node, scope, cx, html: &mut HtmlCollapse, out: &mut dyn HtmlWriter) -> Result<(), Error>`.

### D. The `html` module: head constructs (new)

A module `html` holds constructs for elements of an HTML `<head>`. They
are general HTML vocabulary; a consumer registers them with
`html::register(&mut Registry, urls: Rc<dyn AssetUrls>)`.

```rust
/// Turns a name written in a construct into a URL. A consumer applies its
/// conventions here (a bare name to a folder and extension); the default
/// returns the name unchanged.
pub trait AssetUrls {
    fn url(&self, kind: AssetKind, name: &str) -> Result<String, String>;
}
pub enum AssetKind { Stylesheet, Script, Icon, Link }
pub struct AsWritten; // impl AssetUrls: the name as written
```

Each construct reads its head and inline properties; a property's value
is a bare token, a quoted string or a slot (resolved in scope), `true` is
a flag and `false` omits the attribute.

| Construct | Example | Reduces to |
| --- | --- | --- |
| `[Meta]` | `[Meta] charset=utf-8`; `[Meta] viewport content="width=device-width"`; `[Meta] description content={{ [abc] x.description }}` | `<meta charset>`, or `<meta name="<head>" content>`; any other properties as attributes. A `[Meta]` whose `content` is empty is dropped (no element). |
| `[Title]` | `[Title] {{ [abc] x.title }}` | `<title>` with the head as its text |
| `[Stylesheet]` | `[Stylesheet] main` | `<link rel="stylesheet" href="<url>">` |
| `[Script]` | `[Script] app defer=true type=module` | `<script src="<url>" …></script>` |
| `[Icon]` | `[Icon] icon.png sizes=512x512` | `<link rel="icon" href="<url>" …>` |
| `[Link]` | `[Link] /humans.txt rel=author` | `<link href="<url>" …>` with its properties |

`<url>` is `AssetUrls::url` of the head; its error is an
`ErrorKind::Asset` error at the construct's path. A construct's head may
be a slot. Also exported, for consumers that merge lists of head entries:

```rust
/// The identity of a head entry for merging: "meta:charset",
/// "meta:<name>", "title", or "<kind>:<url>" for the others; None for any
/// other node.
pub fn head_key(node: &Node, urls: &dyn AssetUrls) -> Option<String>;
```

The agnosticism test (step 2) applies to this module too; its docs use
neutral words.

### E. Steps (replaces section 6's step 3 and step 4)

- **Step 3: Collapse, the writer and HTML.** `collapse.rs`, `html.rs`
  (`HtmlCollapse`) and `writer.rs` (`HtmlWriter`, the `String` and
  `IoHtmlWriter` implementations), with element parts added to
  `expand.rs`. Tests: section 5.2's cases through a `String` writer;
  elements with text, flag and value attributes; attribute escaping;
  void elements and `VoidChildren`; an element in an unsafe position;
  `IoHtmlWriter` into a `Vec<u8>` matching the `String` writer; an
  io error becoming `Write`; the recording `Collapse`, now with
  element calls. **Done when:** `cargo test -p libtypedtmpl` passes with
  every case above.
- **Step 4: Head constructs.** `html/head.rs`: section D. Tests: each
  construct's element and attributes; flags and `false`; a slot head and
  a slot property; an empty `content` dropping its `[Meta]`; a test
  `AssetUrls` applying a convention and one returning an error;
  `head_key` for each construct. **Done when:** `cargo test -p
  libtypedtmpl` passes with every case above.
- **Step 5: Docs, example and 0.1.0**: the old step 4, unchanged, with
  the crate docs and the example also showing a head built from the
  constructs.
