## 2026-10-29 21:36 EDT — the template system plan

Plan: `docs/design/20261029_01_TEMPLATE_SYSTEM_PLAN.md` (Draft). The
first plan of this repository: the crate `libtypedtmpl` at the root, on
libtypedmd 0.2.1, carrying section 4 of tmzt.dev's Rust site plan.
Typedmd graphs are expanded by constructs registered by tag into an
inspectable tree, then collapsed into HTML. Four steps, ending in 0.1.0.
No code until the owner has reviewed it.

## 2026-10-29 21:46 EDT — the template system plan: elements, writers and head constructs

Plan: [The template system](docs/design/20261029_01_TEMPLATE_SYSTEM_PLAN.md)
(Draft, amended).

- Constructs can produce element nodes with typed attributes; `Collapse`
  takes its writer on every call and no longer returns an output.
- `HtmlWriter`, with `String` and `IoHtmlWriter` implementations, does
  the writing and escaping; `HtmlCollapse` keeps the decisions.
- An `html` module of head constructs (`[Meta]`, `[Title]`,
  `[Stylesheet]`, `[Script]`, `[Icon]`, `[Link]`), with `AssetUrls`
  for a consumer's naming conventions and `head_key` for merging.

## 2026-10-29 22:00 EDT — the template system plan: Markdown content by default

Plan: [The template system](docs/design/20261029_01_TEMPLATE_SYSTEM_PLAN.md)
(Draft, amended).

- A rendered node's content is Markdown (its paragraphs and `- |` blocks)
  by default, rendered with comrak behind a default `markdown` feature;
  a fenced `html` block stays the raw form.
- Slots are applied by the object that renders; `slots=false` turns them
  off, and text in data is never replaced.
- Slots become placeholders before Markdown rendering and values after
  it, so a value is never read as Markdown.
- `Value::Markdown` for `- name: |` fields, rendered when placed with no
  sigil; named Markdown fragments on rendered nodes.

## 2026-10-29 22:16 EDT — the template system plan: reduction in document order, `[Content]`, defined types and value types

Plan: [The template system](docs/design/20261029_01_TEMPLATE_SYSTEM_PLAN.md)
(Draft, amended).

- Objects reduce in document order; `[Content]` is the only object that
  writes text, and wraps the objects after it with `{{ children }}`.
- `[Template]` defines a type (`type: nav_link` defines `[NavLink]`)
  with typed fields, optional fields and `head:`; a body places an
  instance's children with `[Children]`.
- Value types, with `[Url]` in the `html` module and a consumer's
  `UrlResolver`; head constructs live in a `head` scope.
- Placing named children, `template=` and named Markdown fragments are
  removed.

## 2026-10-29 22:37 EDT — the template system plan: definitions as objects with types as values

Plan: [The template system](docs/design/20261029_01_TEMPLATE_SYSTEM_PLAN.md)
(Draft, amended).

- A `[Template]`'s head is its type; every member is a field whose
  right-hand side (or sigil) is its type; `name?: Type` is optional;
  `name` is the field an instance's head fills; every object is the
  body. Instances are the same format with values.
- `[Content]` takes `- |` blocks; `- [Content] |` is refused with a
  hint, leaving literal blocks on tagged rows to a later libtypedmd change.
- Steps 1 to 3 build on libtypedmd 0.2.0; 0.2.1 is needed only for slots
  in `[Content]` text.

## 2026-10-29 22:42 EDT — the template system plan, step 1: departures

Plan: [The template system](docs/design/20261029_01_TEMPLATE_SYSTEM_PLAN.md)
(In implementation from step 1). Small departures (Rule 1.7):

- **Markdown fields are told by their line breaks.** libtypedmd 0.2 gives a
  `- caption: |` member the block's text as its right-hand side and keeps
  no mark that it was a block, so `Record::from_object` reads a member
  with no sigil whose value spans more than one line as `Value::Markdown`.
  A one-line `|` block reads as `Text`. An explicit `[abc]` keeps a
  multi-line value as `Text`.
- **`Record` keeps its name as a value** (boxed, since `Value` holds
  records), so `get("name")` can return `Option<&Value>` as section 3.1
  gives it.
- **`ErrorKind` grows with the steps.** Step 1 has the kinds it raises
  (`BadValue`, `NoValue`, `SlotInData`, `RecordShape`, `NotAPath`,
  `UnknownField`, `NotARecord`) and `Consumer`; the enum is
  `#[non_exhaustive]`, and the later steps add theirs.
- **Prose in a record's object is a note**, following the amendment that
  makes prose outside `[Content]` a note, rather than section 3.2's error
  for Leaf children; a plain list row stays an error.
