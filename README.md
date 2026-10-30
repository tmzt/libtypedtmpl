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
