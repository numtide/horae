# Contract: `horae-i18n` and the catalog

The interface between the catalog and the code that shows its text. It starts from the proof of
concept (`study/i18n`, `26e562bf`, `crates/i18n`) and adds what research R-1 lists. Anything not
written here is an implementation detail of the crate.

## Crate

- Name `horae-i18n`, path `crates/i18n`, edition 2024, workspace member.
- Runtime dependencies: `fluent-bundle` 0.16, `unic-langid` 0.9. Build dependency:
  `fluent-syntax` 0.12, also a dev-dependency for the unused-message test.
- No dependency on Dioxus, Axum, sqlx, Tokio or `horae-*` crates. Builds for the host and for
  `wasm32-unknown-unknown` (FR-003).
- Used by `horae` from I-1. `horae-server` and `horae-api` may use it in later increments.
  `horae-core` and `horae-types` must not.

## Public API

```rust
/// One variant per directory of `locales/`, English first.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Lang { #[default] En /* , … */ }

impl Lang {
    pub const ALL: &[Lang];
    /// BCP 47 tag, as in `<html lang>` and the stored settings: "en".
    pub fn tag(self) -> &'static str;
}

/// The catalog of one language. `Copy`; compares by language.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Tr { /* lang */ }

impl Tr {
    pub const fn new(lang: Lang) -> Self;
    pub fn lang(self) -> Lang;
    // Generated, one per English message:
    /// Token “{ $name }” created
    pub fn api_tokens_created<'a>(&self, name: impl Into<FluentValue<'a>>) -> String;
}

pub use fluent_bundle::FluentValue;

/// Splits `text` around its one `<tag>…</tag>`: (before, inside, after).
/// Without the tag, the whole text comes back as `before`.
pub fn split_tag<'a>(text: &'a str, tag: &str) -> (&'a str, &'a str, &'a str);
```

The increment that chooses the language adds `Lang::from_tag` (exact tag, then the primary
subtag, so `es-MX` finds `es`) and each language's own name for the picker (FR-035). The first
increment has no caller for either.

## Behaviour

| Case | Result |
| --- | --- |
| Message present in the language | Formatted from that language's bundle |
| Message missing in the language | Formatted from English (FR-005 edge case) |
| Formatting error at runtime (an argument of the wrong kind for a selector) | `debug_assert!` in debug builds; in release, Fluent's partial output |
| Bidi isolation | Off, so English output equals today's text |
| First use of a language | Its bundle is parsed once (`LazyLock`) and shared by all threads |

## Build-time checks (`build.rs`)

Each fails the build with the file and the id, except the warning.

| Check | Outcome |
| --- | --- |
| `locales/en/horae.ftl` missing | Error |
| Syntax error in any catalog | Error |
| Message defined twice in one catalog | Error |
| Message without a value | Error |
| Reference to a message that does not exist | Error |
| Two ids that map to one method name | Error |
| A variable named like a Rust keyword | Error |
| Translation has a message English lacks | Error |
| Translation uses other variables than English for a message | Error |
| Translation lacks messages English has | `cargo:warning` with the count; those show in English |

## Tests of the crate

- Plain messages, variables, plurals (`1 day`, `30 days`), terms, `split_tag` with and without
  the tag (the proof of concept's four, kept; they need real messages, so they return with the
  first screens in I-2).
- `tests/build_checks.rs`: each check above, through `build/checks.rs`, which `build.rs` and the
  test share.
- `tests/unused_messages.rs`: every English message id is used as
  `.method(` in a `.rs` file under `crates/` outside `crates/i18n`, or referenced by a used message
  (research R-6). The failure lists the unused ids.

## Catalog file conventions

The header of `locales/en/horae.ftl` states them; [plan.md](../plan.md) *Catalog conventions*
holds the full list. The file is ordered by these sections, which I-1 writes empty apart from
`-brand`:

```text
## Glossary          (terms: -brand, then the DESIGN.md glossary words as they are used)
## Shared actions    (action-*)
## Common words      (common-*)
## Navigation and shell, ## Components
## Settings, ## API tokens, ## Tasks
## Clients, ## Invoices
## Importers: connection, ## Importers: runs
## People and permissions, ## Audit log
## Reports, ## New project, ## Approvals, ## Projects, ## Timesheet
```

## The app's glue (`crates/horae/src/i18n.rs`)

```rust
pub(crate) use horae_i18n::{Lang, Tr, split_tag};

/// Once, in `App`. English until the language setting exists.
pub(crate) fn use_tr_provider();

/// The page's catalog; English when no provider is above (component tests).
pub(crate) fn use_tr() -> Tr;
```

In a later increment `use_tr_provider` takes the resolved language from the server; its callers do
not change.
