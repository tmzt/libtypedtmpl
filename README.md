# libtypedtmpl

A small template system over [libtypedmd](https://github.com/tmzt/libtypedmd)
graphs (typed Markdown). Typed sections are records. `[Content]` is the only
node that writes text, as Markdown or one fenced `html` block, with typedmd
slots in it (`{{ [abc] story.title }}`). Lists are drawn by a `[List]` with
an `[Item]` child, drawn once per record, and an optional `[Empty]` child.
A `[Template]` defines a type, an object whose values are types, and its
instances are checked against it. There is no template language beyond
that.

A graph is first expanded into an inspectable tree, with every construct
reduced and every slot bound to a value but nothing flattened, and only then
collapsed into output by a collapse the consumer chooses. HTML, with its
escaping and position checks, is the collapse built in.

## Usage

From git, by tag:

```toml
[dependencies]
libtypedtmpl = { git = "https://github.com/tmzt/libtypedtmpl", tag = "v0.1.1" }
libtypedmd = { git = "https://github.com/tmzt/libtypedmd", rev = "a61c091e6420a1dfc40e89b8600cd26e628e5f7a" }
```

The `markdown` feature (on by default) renders Markdown content with
comrak; without it, only `html` blocks render.

The three stages:

1. **Records.** Read data sections with `Record::from_object` and bind them
   in a `Scope`.
2. **Expansion.** `Templates::from_stream` collects the types;
   `Expander::expand_children` reduces a section's objects into an
   `Expanded` tree, with constructs from a `Registry` and list sources
   from your `Sources`.
3. **Collapse.** `collapse` with an `HtmlCollapse` writes HTML into any
   `HtmlWriter`: a `String`, or an `IoHtmlWriter` over a file.

`html::register` adds the head constructs (`[Meta]`, `[Title]`,
`[Stylesheet]`, `[Script]`, `[Icon]`, `[Link]`, in the scope `head`) and
the value type `Url`, with your `AssetUrls` and `UrlResolver`.

```sh
cargo run --example gallery      # a screenshot gallery: its tree, then its HTML
cargo test                       # with the markdown feature
cargo test --no-default-features
```

## Licence

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or
  <http://opensource.org/licenses/MIT>)

at your option.

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms or
conditions.

## Architectural decisions

| Document | Date | Status | Decision |
|---|---|---|---|
| [The template system: typedmd graphs expanded by registered constructs into an inspectable tree, then collapsed into HTML](docs/design/20261029_01_TEMPLATE_SYSTEM_PLAN.md) | 2026-10-29 | Implemented | Objects reduce in document order; `[Content]` holds Markdown or HTML with typedmd slots, read by the crate itself; `[Template]` defines a type as an object whose values are types, and its instances are checked against it; `[List]` draws `[Item]`/`[Empty]`; head constructs and `Url`; a `Collapse` writing through an `HtmlWriter` is the only place the tree is flattened; implemented as 0.1.0 on libtypedmd 0.2.0. |
| [`Markdown` and `Html` field types for `[Template]`, released as 0.2.0](docs/design/20261031_01_MARKDOWN_HTML_FIELDS_PLAN.md) | 2026-10-31 | Draft | A template may declare `Markdown` and `Html` fields; a `Markdown` field reads plain text as Markdown, since libtypedmd cannot tell a one-line `|` block from a plain value; a minor release because `FieldType` gains variants; Draft, for the owner to review. |
