---
title: "Typed records that draw themselves, and absent fields, released as 0.4.0"
date: 2026-10-31
status: Implemented
covers:
  - "A record may carry a type name (`ImageRef`) and a list of absent
    fields"
  - "Placing a typed record in content draws it through a site template
    for its type, or else a built-in renderer registered for the type"
  - "The built-in `ImageRef` renderer, registered by `html::register`"
  - "Absent fields: placing one writes nothing, passing one to an optional
    template field leaves the field out, a `[List]` over one draws
    `[Empty]`"
  - "Release 0.4.0 under Rule 5"
  - "Not covered: typing records read by `Record::from_object`, renderers
    for other types, and attributes holding typed records"
sources:
  - "The owner (2026-10-31), in tmplbuild's declared metadata plan: a
    cover is an ImageRef record that writes itself when placed; a site may
    replace the rendering with its own template, or a template may write
    its own tag; a story without a cover writes nothing, with no nested
    list and no conditional"
related:
  - "[Optional values in `[List]`](20261031_02_OPTIONAL_LISTS_PLAN.md)
    (In implementation): `Expander::items`, extended here"
  - "`~/src/tmplbuild` plan `20261031_04_META_RECORDS_PLAN.md`: the first
    user"
---

## 1. Goal

`{{ story.meta.cover }}` in a `[Content]` writes `<img src="…" alt="">`
when the story has a cover, and nothing when its declared optional cover
is absent. A site can replace that rendering for every ImageRef with
`# [Template] ImageRef`.

## 2. Typed records and absent fields

`Record` gains, through new methods (its fields stay private):

- `with_type(self, name) -> Record` and `type_name() -> Option<&str>`: the
  record's type, such as `ImageRef`;
- `mark_absent(&mut self, field)` and `is_absent(&self, field) -> bool`: a
  field the record's producer knows is optional and unset.

`get` still answers `None` for an absent field.

## 3. Resolving absent fields

- **A slot** whose path reaches an absent field, as its last field or on
  the way (`story.meta.cover.src` with `cover` absent), resolves to
  nothing. Placed in content it writes nothing. A field that is missing
  and not marked absent is still an error.
- **An optional template field** given a slot that resolves to nothing is
  left out, as if the instance had not set it (`this.cover` absent in the
  body). A required field given one is the existing "is left out" error.
- **A `[List]` source** that reaches an absent field draws no items
  (`[Empty]`), anywhere on the path.
- **A `[List]` source that is a typed record** draws that record once,
  bound by `item_alias`. An untyped record still draws its `items`.

## 4. Placing a typed record

In a `[Content]`, a slot with no sigil whose value is a typed record is
drawn by, in order:

1. a `[Template]` whose type is the record's type (compared as type names
   are), with `this` bound to the record and its absent fields absent in
   the body. Its required fields must be set, as for an instance;
2. a renderer registered for the type: a new trait `Render` (`type_name`,
   `render(&self, record, cx) -> Result<Vec<Part>, Error>`), registered
   with `Registry::register_render`;
3. otherwise the error "`x` is a `<Type>` record; define `[Template]
   <Type>` to draw it".

A template field declared with a type no template defines (`- cover?:
ImageRef`) accepts a record of that type. The type is checked against
the record's type name, and needs neither a template nor a value type.

## 5. The built-in `ImageRef`

`html::register` registers a renderer for `ImageRef`. It writes the
element `img` with `src` (the record's `src`, a Link, written with `[-]`'s
escaping and refused schemes) and `alt` (its `alt` as text, or empty). A
record without `src` is an error.

## 6. Version

Placing a record and absent fields writing nothing were errors in 0.3.0;
a typed record in a `[List]` drew its `items`. Those are behaviour
changes, so the release is **0.4.0** (Rule 5.4). The new methods, trait
and renderer are additive.

## 7. Steps

### Step 1: Typed records, absent fields, placing, ImageRef

`src/record.rs`, `src/path.rs`, `src/expand.rs`, `src/content.rs`,
`src/list.rs`, `src/html`: sections 2 to 5. Tests: an absent field placed
writes nothing (last and middle); a missing unmarked field still errors;
an optional template field left out by an absent slot; a required one
erroring; a `[List]` over an absent field and over a typed record; a
typed record placed by the built-in renderer, by a site template, and
with neither (the error); `- cover?: ImageRef` accepting an ImageRef
record; `src` escaping and a refused scheme. The README describes typed
records.

**Done when:** `cargo test` passes with and without the `markdown`
feature, with those tests.

### Step 2: Release 0.4.0

`version = "0.4.0"` with its `VERSIONS.md` entry, tagged `v0.4.0`; the
README's usage example naming it; `main` and the tag pushed to GitHub.

**Done when:** the tag is on GitHub and a crate depending on it builds.

## Amendment — 2026-11-01 00:02 EDT: implementation started

The owner agreed this plan (2026-10-31). Implementation starts with
step 1. From here this document is immutable; it changes only on the
owner's explicit instruction, and departures go to `CHANGES.md`.

## Amendment — 2026-11-01 00:03 EDT: implemented

Both steps are done and their **Done when:** holds.

- Step 1: `20be3b5`.
- Step 2: `ee834fb` (tagged `v0.4.0`) and `2a2bb31` (the README's
  usage example), pushed to GitHub; a crate depending on the tag builds.
