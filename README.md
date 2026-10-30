# libtypedtmpl

A small template system over [libtypedmd](https://github.com/tmzt/libtypedmd)
graphs (typed Markdown). Typed sections are records; a fenced `html` block
is a fragment of output with typedmd slots in it (`{{ [abc] post.title }}`);
lists are drawn by a `[List]` section with an `[Item]` child, drawn once
per record, and an optional `[Empty]` child, drawn when there are none.
There is no template language beyond that.

A graph is first expanded into an inspectable tree, with every construct
resolved and every slot bound to a value but nothing flattened, and only
then collapsed into output by a collapse the consumer chooses. HTML, with
its escaping, is the collapse built in. Nothing is implemented yet: the
first plan is a Draft.

## Status

Not yet released. The library will be used from git by revision, as
libtypedmd is; the Cargo snippet arrives with the first release.

## Architectural decisions

| Document | Date | Status | Decision |
|---|---|---|---|
| [The template system: typedmd graphs expanded by registered constructs into an inspectable tree, then collapsed into HTML](docs/design/20261029_01_TEMPLATE_SYSTEM_PLAN.md) | 2026-10-29 | Draft | Fragments are `html` blocks with typedmd slots; `[List]`/`[Item]`/`[Empty]` and `[Template]` are constructs registered by tag; expansion yields a tree that keeps its structure, and a `Collapse` (HTML built in, with all escaping) is the only place it is flattened; 0.1.0 on libtypedmd 0.2.1; Draft, for the owner to review. |
