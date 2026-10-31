---
title: "Optional values in `[List]`, and `Markdown` into `Html` fields, released as 0.3.0"
date: 2026-10-31
status: In implementation
covers:
  - "A `[List]` source naming a missing last field draws no items (its
    `[Empty]`), and one naming a single value draws it once, bound by
    `item_alias` as it is"
  - "An `Html` template field taking a Markdown value, kept as Markdown
    and rendered where it is placed"
  - "Release 0.3.0 under Rule 5"
  - "Not covered: other conditionals, slots writing nothing for a missing
    field outside `[List]`, and a Markdown tree value (the reserved
    `content` of tmplbuild's declared metadata plan)"
sources:
  - "The owner (2026-10-31), in tmplbuild's declared metadata plan: an
    optional key is drawn by a `[List]` over it, `[Item]` when present and
    `[Empty]` when not; `meta` holds only what the site wrote"
  - "The owner (2026-10-31): `Html` and `Markdown` are newtypes that keep
    their original contents and set the rules for escaping, as in Razor;
    both keep HTML inside them"
  - "The gap found in 0.2.0: a slot naming a field the record lacks is an
    error, even when it feeds an optional template field"
related:
  - "[Markdown and Html field types](20261031_01_MARKDOWN_HTML_FIELDS_PLAN.md)
    (Implemented): the field types this extends"
  - "`~/src/tmplbuild` plan `20261031_04_META_RECORDS_PLAN.md`: the first
    user, drawing `story.meta.cover`"
---

## 1. Goal

A site draws something only when an item has it, using the construct it
already knows:

```markdown
- [List] cover source={{ [1.] story.meta.cover }} item_alias=c
  - [Item]
    - [Content]
      ```html
      <img class="cover" src="{{ [-] c }}" alt="">
      ```
  - [Empty]
    - [Content]
      ```html
      <span class="no-cover"></span>
      ```
```

## 2. `[List]` sources

A `[1.]` source that is a path in scope (`alias.field…`) resolves as
today, with two changes:

| The path resolves to | 0.2.0 | 0.3.0 |
| --- | --- | --- |
| a list | its records | unchanged |
| a record | its `items` | unchanged |
| any other value (text, a link, Markdown, …) | error "is …, not a list" | one item: the value |
| nothing: the **last** field is missing on a record | error "no field" | no items (`[Empty]` draws) |

- Only the last field may be missing. A missing alias, or a missing field
  before the last (`story.mta.cover`), is still an error, so most typos
  are still caught. A misspelt last field draws `[Empty]`; that is the
  price of drawing absent keys this way.
- A single value is bound by `item_alias` as it is, so `{{ [-] c }}` reads
  it. For a list or a record's items, `item_alias` binds each record as
  today.
- Consumer sources (`Sources::list`) are unchanged.
- `Expander::records`, public, returns `Vec<Record>` and cannot hold a
  single value. It is replaced by `Expander::items`, which returns
  `Vec<Value>`. **Breaking.**

## 3. `Markdown` into `Html` fields

`Html` and `Markdown` are both trusted content that keep their original
text. They differ only in how they are written: an `Html` value as it is,
a `Markdown` value rendered. An `Html` field therefore also takes a
`Markdown` value from a slot. The value stays Markdown (`this.body` is
Markdown) and is rendered where it is placed. Plain text is still refused
by an `Html` field.

## 4. Version

Replacing `Expander::records`, and drawing items where 0.2.0 gave errors
(behaviour a caller can observe), are not strictly additive, so the
release is **0.3.0** (Rule 5.4).

## 5. Steps

### Step 1: Optional and single-value sources, Markdown into Html

`src/expand.rs`: `Expander::items` with section 2's table, the `[List]`
construct binding each item value, and `field_value` taking Markdown into
an `Html` field. Tests: each row of the table, `[Empty]` for a missing
last field, the errors for a missing alias and a missing middle field, a
link drawn once and read as `{{ [-] c }}`, a record source unchanged, and
a Markdown value through an `Html` field rendered. The README describes
optional sources.

**Done when:** `cargo test` passes with and without the `markdown`
feature, with those tests.

### Step 2: Release 0.3.0

`version = "0.3.0"` with its `VERSIONS.md` entry, tagged `v0.3.0`; the
README's usage example naming it; `main` and the tag pushed to GitHub.

**Done when:** the tag is on GitHub and a crate depending on it builds.

## Amendment — 2026-10-31 13:44 EDT: implementation started

The owner agreed this plan (2026-10-31). Implementation starts with
step 1. From here this document is immutable; it changes only on the
owner's explicit instruction, and departures go to `CHANGES.md`.
