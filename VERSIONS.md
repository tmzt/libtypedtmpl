# VERSIONS

Each change of the crate's version and the public API changes it carries
(AGENTS.md, Rule 5). Internal changes are not listed, and there is no entry
without a version change. Entries are appended, oldest first, and never
edited.

## 0.1.0 — 2026-10-29 (`7b00606`, tag `v0.1.0`)

The first release: the template system of
`docs/design/20261029_01_TEMPLATE_SYSTEM_PLAN.md`, steps 1 to 5, on
libtypedmd 0.2.0 (the commit tagged `v0.2.0`). The commit named is the last
one before the version commit, which cannot name itself; the tag is on the
version commit.

- **Values and records:** `Value` (exhaustive), `Html`, `Date`,
  `Record` with `Record::from_object`.
- **Paths and errors:** `NodePath`, `Segment`, `SlotPath`, `Error`,
  `ErrorKind` (not exhaustive).
- **Expansion:** `Scope`, `Expanded`, `Part`, `Element`, `Attr`,
  `AttrValue`, the `Construct` and `ValueType` traits, `Registry`
  (with named scopes and value types), `Sources`, `NoSources`,
  `Expander`; the built-in constructs `[List]` (with `[Item]` and
  `[Empty]`), `[Content]` and `[Children]`; `Templates`, `TypeDef`,
  `FieldDef`, `FieldType` and `normalise` for defined types.
- **Collapse:** the `Collapse` trait and `collapse`; `HtmlWriter`,
  `IoHtmlWriter`, `escape`, `is_void`; `HtmlCollapse` and
  `render_html`.
- **`html`:** `register`, the head constructs in the scope `head`,
  `AssetUrls`, `AssetKind`, `AsWritten`, `UrlResolver`, `AsGiven`,
  `Url` and `head_key`.
- **`markdown`** (feature, on by default): `markdown::render`.

## 0.1.1 — 2026-10-29 (`e0e9c62`, tag `v0.1.1`)

A fix, with no change to the public API.

- **Fixed:** wrapping now nests. A `[Content]` holding `{{ children }}`
  among the objects another one wraps wraps the objects after it, and an
  instance's children are wrapped like any other siblings. In 0.1.0 the
  wrapped objects were reduced one by one, so a document shell around a
  `<main>{{ children }}</main>` produced an empty `<main>`. The plan
  already says both; the output now matches it.

## 0.2.0 — 2026-10-31 (`1b2ea1c`, tag `v0.2.0`)

`Markdown` and `Html` field types for `[Template]`
(`docs/design/20261031_01_MARKDOWN_HTML_FIELDS_PLAN.md`). The commit named
is the last one before the version commit; the tag is on the version
commit.

- **Breaking:** `FieldType` (an exhaustive enum) gains the variants
  `Markdown` and `Html`; a `match` on it needs arms for them.
- **Added:** a definition may declare `- name: Markdown` and
  `- name: Html`, optional as `name?:`. A `Markdown` field takes a
  Markdown value, plain text (read as Markdown) or a literal; an `Html`
  field takes trusted HTML from a slot. Any other value is an error at the
  instance.

## 0.3.0 — 2026-10-31 (`674ade1`, tag `v0.3.0`)

Optional values in `[List]`, and Markdown into `Html` fields
(`docs/design/20261031_02_OPTIONAL_LISTS_PLAN.md`). The commit named is
the last one before the version commit; the tag is on the version commit.

- **Breaking:** `Expander::records` (returning `Vec<Record>`) is replaced
  by `Expander::items`, returning `Vec<Value>`.
- **Breaking (behaviour):** a `[List]` source that is a path to a single
  value (not a list or a record) draws it once, bound by `item_alias` as
  it is, where 0.2.0 gave `NotAList`; a path whose last field is missing
  draws no items (`[Empty]`), where 0.2.0 gave `UnknownField`. A missing
  alias or middle field is still an error.
- **Added:** an `Html` template field takes a Markdown value, which stays
  Markdown and is rendered where it is placed.

## 0.4.0 — 2026-11-01 (`20be3b5`, tag `v0.4.0`)

Typed records that draw themselves, and absent fields
(`docs/design/20261031_03_TYPED_RECORDS_PLAN.md`). The commit named is the
last one before the version commit; the tag is on the version commit.

- **Added:** `Record::with_type`, `Record::type_name`,
  `Record::mark_absent`, `Record::is_absent`; `SlotPath::lookup_present`;
  the trait `Render` with `Registry::register_render` and
  `Registry::render`; `html::ImageRef`, registered by `html::register`,
  writing `<img src alt>`.
- **Breaking (behaviour):** a typed record placed in content with no
  sigil is drawn by a `[Template]` for its type or a registered renderer,
  where 0.3.0 gave `SigilMismatch`; a slot reaching a field marked absent
  resolves to nothing (writes nothing; an optional template field is left
  out; a `[List]` draws `[Empty]`), where 0.3.0 gave `UnknownField`; a
  typed record as a `[List]` source draws once, where 0.3.0 drew its
  `items`; a template field whose type names a record type accepts a
  record of that type without a template or value type of that name.
