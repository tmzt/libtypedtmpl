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
