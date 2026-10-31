---
title: "`Markdown` and `Html` field types for `[Template]`, released as 0.2.0"
date: 2026-10-31
status: Draft
covers:
  - "Two field types a `[Template]` may declare: `Markdown` and `Html`,
    what an instance may pass to each, and how the body places them"
  - "A `Markdown` field reads plain text as Markdown, so a one-line value
    is Markdown when the field says so"
  - "Release 0.2.0 under Rule 5"
  - "Not covered: reading a one-line `- name: |` block in data as Markdown
    (libtypedmd's tree does not record that a value came from a block;
    see section 2), Markdown with slots applied, and converting `Html` to
    or from `Markdown`"
sources:
  - "A request from the tmzt.dev design session (2026-10-31), forwarded on
    the owner's behalf: a `[MobileScreenshot]` template taking each
    screenshot's `- details: |` Markdown as `- details?: Markdown` and
    placing it rendered; an `Html` field type for trusted fragments such
    as a story's body; any `|` block read as Markdown"
  - "Its findings at libtypedtmpl 0.1.1: `- details?: Markdown` fails as an
    unknown type (`CORE_TYPES` in `src/template.rs` has no entry, though
    `Value::Markdown` and `Value::Html` exist), and a one-line `|` block
    is read as text (`read_value` in `src/record.rs` makes Markdown only
    when the text holds a newline)"
  - "The owner (2026-10-31): the field decides; a `Markdown` field takes
    plain text as Markdown, and libtypedmd keeps its pin"
related:
  - "[The template system](20261029_01_TEMPLATE_SYSTEM_PLAN.md)
    (Implemented): `[Template]` definitions, `FieldType`, placement in
    `[Content]`"
  - "`~/src/tmplbuild` plan `20261031_01_MARKDOWN_HTML_FIELDS_PLAN.md`:
    the builder taking 0.2.0"
---

## 1. Goal

A template can take Markdown and trusted HTML as fields. An instance
passes them from a slot (`details={{ shot.details }}`, no sigil), and the
body places them with `{{ this.details }}`. Today neither type can be
declared, so a site has to place such values outside the template.

## 2. Why the field decides

The request asked that every `|` literal block be Markdown, whatever its
line count. libtypedmd 0.2.0 cannot tell the two apart: the rows
`- details: |` followed by one indented line, and `- details: one line`,
give the same `Member` (one literal part, `"one line"`), and libtypedmd is
pinned at 0.2.0. So the declared type decides instead. A `Markdown` field
reads plain text as Markdown. Data members read by `Record::from_object`
are unchanged: no sigil and more than one line is Markdown, otherwise
text.

## 3. The field types

`FieldType` gains two variants, `Markdown` and `Html`. A definition
declares them by name: `- details: Markdown`, `- details?: Markdown`,
`- body: Html`. They have no sigil form.

What an instance may give each field:

| Field | From a slot (`x={{ a.b }}`) | From a literal (`x=text`) | From child rows |
| --- | --- | --- | --- |
| `Markdown` | `Value::Markdown` as is; `Value::Text` as `Value::Markdown` of the same text | `Value::Markdown` of the literal | error |
| `Html` | `Value::Html` only | error: a literal is not trusted HTML | error |

Any other value is an error at the instance, naming the field, the slot
and the value's kind, for example "the field `details` is Markdown, but
`shot.date` is a date". An optional field left out is absent from `this`,
as for every type, so `{{ this.details }}` writes nothing.

Placing is unchanged, because placement already handles both values:

- `{{ this.details }}` in a `[Content]` renders a Markdown value through
  comrak (feature `markdown`) and writes an Html value as it is;
- `{{ [abc] this.details }}` writes a Markdown value's source as escaped
  text;
- an Html value with a sigil is refused, as today.

## 4. Version

Adding variants to `FieldType`, an exhaustive public enum, is not strictly
additive (Rule 5.4), so the release is **0.2.0**. Its `VERSIONS.md` entry
lists `FieldType::Markdown` and `FieldType::Html` as **Breaking** (new
variants of an exhaustive enum) and the two field types as added.

## 5. Steps

### Step 1: The field types

`src/template.rs`: `FieldType::Markdown` and `FieldType::Html`, read from
the type names `Markdown` and `Html`. `src/expand.rs`: an instance's field
values as section 3's table says, in `field_value` (slots and literals)
and `nested_value` (child rows). Tests: each cell of the table, the error
messages, a one-line text placed through a `Markdown` field rendered as
Markdown (`*a*` gives `<em>a</em>`), `[abc]` placing its source, an
optional field left out writing nothing, and an `Html` field taking a
rendered fragment. The README's template section lists the two types.

**Done when:** `cargo test` passes with those tests, and a `[Template]`
with `- details?: Markdown` renders one-line and many-line details the
same way.

### Step 2: Release 0.2.0

`version = "0.2.0"` in `Cargo.toml` with its `VERSIONS.md` entry in the
same commit, tagged `v0.2.0`, and the README's usage example naming
`v0.2.0` in its own commit. Then `main` and the tag are pushed to
`github.com/tmzt/libtypedtmpl`, so tmplbuild can depend on the tag.

**Done when:** the tag `v0.2.0` is on GitHub, and a crate depending on
`libtypedtmpl = { git = "https://github.com/tmzt/libtypedtmpl", tag =
"v0.2.0" }` builds.
