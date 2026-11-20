# Data model: Interface translation (i18n)

The first increment stores nothing: its entities are files and generated code. The stored
entities belong to later increments and are recorded here so their plans start from one shape.

## First increment: the catalog

### Catalog

One Fluent resource per language, `crates/i18n/locales/<tag>/horae.ftl`. The directory name is
the language's BCP 47 tag (`en`, later `es`, `pt-BR`).

- **English** (`en`) is the source. It is required, and every other catalog is checked against it.
- **A translation** may lack messages (each shows in English; the build prints a warning) but may
  not have a message English lacks or use other variables than English for the same message (the
  build fails). FR-005.
- **Sections**: `##` comments split the file by area, in the order of [plan.md](plan.md) *PR
  split*, so PRs that run in parallel add to different places (research R-2).

### Message

| Field | Rule |
| --- | --- |
| id | `<area>-<what>` or a shared `action-*` / `common-*` id; unique in its catalog; no two ids may map to the same method name (`a-b` and `a_b`) |
| value | Required. Today's English text, character for character (FR-011), with the whitespace, brace and number rules of research R-4 |
| variables | `$name` placeables. The method takes one parameter per variable, including variables of messages it references. A variable may not be a Rust keyword |
| plural forms | A select on a count variable with CLDR categories; English uses `[one]` and `*[other]` |
| markup | At most one `<tag>…</tag>` per message, split by `split_tag` |

**Used**: a message is used when its method appears in Rust code under `crates/` outside
`crates/i18n`, or another used message references it. An unused message fails the catalog's test
(FR-006, research R-6).

### Term

A Fluent term (`-name`) for a word that several messages share: the brand and the glossary words
of `DESIGN.md`. Terms have no method; messages use them. A term may take a `case` variant when
English needs both a capitalised and a lower-case form. Terms are added when the first migrated
message needs them, so none is unused.

### Generated code

| Item | Shape |
| --- | --- |
| `Lang` | One variant per catalog directory, English first and `Default`; `ALL`, `tag()`; `from_tag()` arrives with language selection |
| `Tr` | `Copy` value holding a `Lang`; one method per English message returning `String`, documented with its English text |

### Lint baseline

`["dioxus/literal-text"]` in `crates/dioxus-lint/baseline.toml`: a `reason` and one
`"<file>" = <count>` line per file with loose text, taken when I-1 lands. Each migration PR lowers
or removes its files' lines; I-13 removes the table. `EXEMPT_FILES` and `NOT_LANGUAGE` live in the
lint's source with a reason per entry ([contracts/lint-rule.md](contracts/lint-rule.md)).

## Later increments: stored settings (not in tasks.md)

All columns hold a catalog tag or are null. They are validated against `Lang::ALL` when written,
and read through `Lang::from_tag`, so a tag that is no longer in Horae counts as unset (spec *Edge
Cases*). They are migrations `0005_*` onwards (the next free number when each merges).

### `users.language` (language preference)

`text NULL`. Null means "follow the organization". Set and cleared by the person in My settings,
through `ops::profile::update`, beside `users.timezone` (FR-030). Read with the organization's value
in the same query as the time zone (`server_fns::auth::current_user`).

### `organizations.language` (organization default language)

`text NULL`. Null means "no default": the browser's language decides, then English. Set by a person
with `Permission::CompanyWrite`, the permission of the other workspace settings, through an
`ops::organization` operation beside `set_timezone` (FR-031). New and existing organizations start null (spec
*Assumptions*).

### `organizations.invoice_language` (invoice language)

`text NOT NULL DEFAULT 'en'`. Set in the invoicing settings with `Permission::InvoiceWriteAll`,
the permission that governs `ops::organization::update_branding` today (FR-050). Every
organization starts in English.

### `invoices.language` (issued invoice's language)

`text NULL`. Written once, in the transition from `draft` to `sent` in `ops::invoices`, with the
organization's `invoice_language` at that moment, and never changed afterwards. Null while the
invoice is a draft, and on a draft that was voided without being issued. The migration sets `en`
on every invoice that is not a draft (spec *Assumptions*). Rendering uses `invoices.language` when
set and the organization's `invoice_language` otherwise; who downloads the invoice plays no part
(FR-051). An issued invoice whose tag is no longer in Horae renders in English.

### State: resolving the interface language

```text
person.language ──known──► use it
      │ null or unknown
      ▼
org.language ────known──► use it
      │ null or unknown
      ▼
Accept-Language: first tag known (exact, then primary subtag) ──► use it
      │ none
      ▼
English
```

Before sign-in only the last two steps apply. Emails skip the browser step (FR-053).

### Error reason (server messages increment)

Each user-visible `OpError` message gets a stable `reason` (`&'static str`, already a field of
`ErrorDetail`) and named arguments, carried to the interface in the server function error's
details. The English `message` stays as the fallback and as what `/api/v1` returns (FR-041,
FR-042). Its shape is planned with that increment.
