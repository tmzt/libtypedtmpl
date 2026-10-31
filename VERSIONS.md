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
