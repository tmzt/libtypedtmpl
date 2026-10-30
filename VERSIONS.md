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
