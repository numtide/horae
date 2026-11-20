# Implementation Plan: Interface translation (i18n)

**Branch**: `plan/i18n` (plan and tasks); each PR below lands as its own branch and pull request |
**Date**: 2026-11-20 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/023-i18n/spec.md` (clarified 2026-11-20, PR #439),
the study and proof of concept on `study/i18n` (`26e562bf`) summarised in
[research.md](research.md), and the maintainer's constraints C-1 to C-9.

## Summary

Move every text of the web app into one English Fluent catalog, read through a small crate,
`horae-i18n`, that generates one method per message, so a missing message or argument does not
compile. Nobody sees a difference: the catalog reproduces today's words, and each migrated screen's
server-rendered HTML is compared byte for byte before and after. A new rule of the Dioxus lint
counts loose text in `rsx!` as a ratchet, so the count only goes down and new screens start at
zero. The work ships as thirteen small pull requests, I-1 to I-13: the infrastructure, then the
proof of concept's screens, the shared components, one PR per area, and a last one that closes the
ratchet. After it, PR-10 (the glossary) edits only `horae.ftl` and tests.

This plan covers the first increment (User Stories 1 and 2) in full. It also records the design
the later increments start from (choosing the language, storing the preferences, the invoice
language), so the first one leaves room for them; those increments get their own plan and tasks.

## Technical Context

**Language/Version**: Rust 2024 (workspace toolchain from the Nix dev shell), Dioxus 0.7.9
fullstack (SSR plus WASM)

**Primary Dependencies**: `fluent-bundle` 0.16 and `unic-langid` 0.9 (new, runtime);
`fluent-syntax` 0.12 (new, build script and the unused-message test); `dioxus-rsx` and `syn`,
already used by `crates/dioxus-lint`

**Storage**: none in the first increment. The later increments add nullable `language` columns on
`users` and `organizations`, `invoice_language` on `organizations` and `language` on `invoices`
([data-model.md](data-model.md))

**Testing**: `cargo test -p horae-i18n` (generator, fallback, plurals, `split_tag`, unused
messages; no database); `cargo test -p horae-dioxus-lint` (fixtures of the new rule, rule-to-skill
check); the existing UI tests of each migrated screen, unchanged, plus an HTML dump test per screen
file run before and after the migration (research R-7); `cargo test -p horae --features server`
for the screens tested inside the crate

**Target Platform**: Linux server and the browser (WASM), from the same catalog

**Project Type**: web application (Cargo workspace)

**Performance Goals**: the catalog is parsed once per process (a `LazyLock` per language); a
message costs one lookup and one `String`. No render-time fetch (FR-004)

**Constraints**: no visible change (FR-011, SC-002); at most +60 KB brotli on the served WASM over
master (SC-005); `horae-core` and `horae-types` do not depend on the catalog (FR-003); the
infrastructure lands after PR-4 and the `Button` API (C-9); no PR rewrites a file an open design
PR is rewriting (FR-013)

**Scale/Scope**: on master `af6521cb`, about 1,420 loose texts in `rsx!` markup and about 590 prose
literals outside it, in 90 files of `crates/horae/src` (gallery excluded); thirteen PRs of 40 to 300
texts each

## Constitution Check

*GATE: checked before Phase 0 and after Phase 1. Checked against constitution 7.0.0.*

**Status: PASS.** No deviation to track.

- **I. Exactness**: Pass. Numbers keep coming from `horae-core`'s formatters; the catalog receives
  formatted money, durations and dates as strings, and plural counts as integers (research R-4). No
  floating point is introduced.
- **II. Domain Purity**: Pass. `horae-core` gains no dependency (FR-003). Labels of core enums shown
  on screen move to the app, which owns the words; the rules stay in `core` (research R-3).
- **III. Single Datastore**: Pass. No schema change in the first increment. The later increments'
  columns live on tables that already carry `org_id`.
- **IV. Mutations Through Shared Operations**: Pass. The first increment mutates nothing. The later
  language settings are written by `ops::profile::update` and an `ops::organization` operation,
  called from server functions, like the time zone today.
- **V. Reproducible Builds & Formatting Gate**: Pass. `horae-i18n` joins the tests and clippy
  checks of `nix flake check`; the lint rule runs in the existing `dioxus-lint` check. Whether
  crane's dummy-source build accepts the crate's `build.rs` is verified in I-1 (research R-9).
- **Data constraints**: Pass. Nothing touches authorization.

**Re-check after design**: unchanged. The design adds a library crate with no I/O, one lint rule,
and tests.

## Project Structure

### Documentation (this feature)

```text
specs/023-i18n/
├── spec.md
├── plan.md                 # this file
├── research.md             # the study, then planning decisions R-1 to R-12
├── data-model.md           # catalog, message, term; the later increments' columns
├── quickstart.md           # how to check each PR and the increment
├── contracts/
│   ├── catalog.md          # horae-i18n's API and the catalog conventions
│   ├── lint-rule.md        # dioxus/literal-text: what it counts, exceptions, output
│   └── language-settings.md  # later increments: server functions and fields (draft)
├── checklists/requirements.md
└── tasks.md
```

### Source Code

```text
crates/i18n/                                  # horae-i18n (new, I-1)
├── Cargo.toml
├── build.rs                                  # generator (from 26e562bf, R-1)
├── build/checks.rs                           # parsing and checks, shared with tests/build_checks.rs
├── locales/en/horae.ftl                      # the English catalog, one section per area (R-2)
├── src/lib.rs                                # Lang, Tr, split_tag, bundles with English fallback
├── tests/build_checks.rs                     # each build-time check fails as contracts/catalog.md says
└── tests/unused_messages.rs                  # every English message is used somewhere (R-6)
crates/horae/src/i18n.rs                      # use_tr_provider, use_tr (I-1)
crates/horae/src/app.rs                       # calls use_tr_provider (I-1)
crates/horae/src/main.rs                      # mod i18n (I-1)
crates/horae/Cargo.toml                       # horae-i18n dependency (I-1)
crates/horae/Dioxus.toml                      # watch_path gains ../i18n (I-1)
crates/horae/src/{pages,components}/**        # migrated area by area (I-2 to I-12)
crates/horae/tests/*_ui.rs and others         # mod i18n include (I-1, R-8); HTML dump tests (R-7)
crates/dioxus-lint/src/main.rs                # rule dioxus/literal-text, EXEMPT_FILES, NOT_LANGUAGE (I-1)
crates/dioxus-lint/baseline.toml              # the rule's per-file baseline, lowered by every PR
crates/dioxus-lint/fixtures/literal-text/     # bad.rs and good.rs (I-1)
.agents/skills/dioxus-best-practices/SKILL.md # new section "Interface text" (I-1)
.agents/skills/dioxus-best-practices/references/source-evidence.md   # rsx/src/text_node.rs (I-1)
Cargo.toml                                    # workspace member (I-1)
nix/checks/{tests,clippy}.nix                 # -p horae-i18n (I-1)
AGENTS.md                                     # the crate in Architecture, its commands (I-1)
design/IMPLEMENTATION-GAPS.md                 # PR-10 note, then the increment's status (I-1, I-13)
```

**Structure Decision**: one new library crate, `crates/i18n`, used by `horae` now and by
`horae-server` and `horae-api` in later increments. The app's glue is one module,
`crates/horae/src/i18n.rs`. Module layout follows `foo.rs` + `foo/`.

## Design

### The catalog crate

[contracts/catalog.md](contracts/catalog.md) is the full contract. In short: `build.rs` reads
`locales/<lang>/horae.ftl`, checks syntax, duplicate ids, unknown references and, per translation,
extra messages or other variables (errors) and missing messages (warnings), and generates `Lang`
(one variant per directory, English first and default) and one method per English message on `Tr`,
with the message's variables as `impl Into<FluentValue>` parameters and the English text as its
doc comment. `Tr::new(lang)` formats from that language's bundle and falls back to English per
message. Isolation marks are off, so English output equals today's text. `split_tag` cuts a
sentence around its one `<tag>…</tag>` so the screen can wrap the middle in a link or a `span`
(FR-008).

The proof of concept's generator is kept, with the three additions of research R-1.

### Catalog conventions (FR-007, FR-008)

Written as the header of `horae.ftl` and repeated in the skill's "Interface text" section:

- **Ids**: `<area>-<what>`, lower case, words joined by `-`. The area is the screen or the
  component (`settings`, `api-tokens`, `tasks`, `clients`, `invoices`, `harvest`, `people`,
  `permissions`, `audit`, `reports`, `new-project`, `approvals`, `projects`, `timesheet`, `nav`,
  `date-picker`, `timer`, …). The suffix says what the text is: `-title` (a heading), `-hint`,
  `-label` (an accessible name), `-placeholder`, `-empty`, `-loading`, `-failed`, `-confirm`.
- **Shared ids**: `action-*` for a button or menu item, `common-*` for a word or status, only when
  two or more areas use the same English text with the same meaning. The same English with two
  meanings ("Open" the verb and "Open" the status) gets two ids, because other languages may need
  two words.
- **Terms**: the brand (`-brand`) and each glossary word of `DESIGN.md` become terms the first time
  a migrated screen uses them (`-people`, `-restore`, `-my-settings`, …), with a `case` variant
  where English needs both "People" and "people". Messages refer to the term instead of repeating
  the word, so PR-10 and each language rename it once (User Story 2).
- **Counts**: one message per text with a count, with the count as a variable and the plural forms
  in the message. No screen chooses between two messages with `n == 1` or builds "item(s)".
- **Markup**: a sentence with a link or emphasis inside is one message with `<tag>…</tag>` around
  the part; the screen splits it with `split_tag`. No sentence is assembled from fragments.
- **Whitespace, braces and numbers**: as research R-4.

### How screens use it

- `App` calls `crate::i18n::use_tr_provider()`, which provides `Tr::new(Lang::En)`.
- A component calls `let tr = use_tr();` once, then writes `{tr.tasks_title()}` for a text node,
  `aria_label: tr.tasks_pages()` for an attribute, and `label: tr.action_edit()` for a prop.
- A plain function that returns text takes `tr: Tr` (`fn error_message(tr: Tr, error: &…) -> String`). Constant tables of text (`const TASK_GROUPS`) become functions of `tr`.
- An enum whose label was a `&'static str` comparison becomes a match on the enum (the proof of
  concept did this for the token status).
- A value of `horae-core` or `horae-types` shown with words gets a function in the app
  (`project_type_label(tr, t)`); research R-3.
- Text that comes from the server (`error.message`, names, notes) is shown as it arrives.

`Tr` is `Copy` and compares by language, so it is safe in props, closures and `use_callback`
captures (`dioxus-best-practices`, *Props*).

### The lint rule (FR-020 to FR-023)

[contracts/lint-rule.md](contracts/lint-rule.md) is the contract. `dioxus/literal-text` counts, per
file of `crates/horae/src`, text nodes with a letter, literal readable attributes and literal prose
props, minus the gallery (`EXEMPT_FILES`) and listed non-language literals (`NOT_LANGUAGE`, each
with its reason). It lands with the counts of that moment as its baseline, so CI fails when a
file's count rises and asks for the baseline to be lowered when it falls. Every migration PR lowers
its files' lines; I-13 removes the rule's table from `baseline.toml`, after which any loose text
fails. The lint also prints the count of prose literals outside markup as information.

### Proving nothing changed (SC-002)

Each migration PR follows [quickstart.md](quickstart.md) *Per-PR check*: its first commit adds an
HTML dump test to each migrated screen's test file (research R-7); the dump is taken at that commit
and at the PR's head, normalised and diffed; the diff must be empty, and the screen's existing
tests pass with no change to their assertions. A screen with no SSR test yet gets a dump test for
the states it can render with its mocks, and the PR says which states it covers.

### WASM budget (SC-005)

Measured as research R-9 says: `dx build --release --locked` on base and head, size of
`horae_bg-*.wasm.br`, at I-1, I-2 and I-13. The expected total is about 42 KB for the engine plus
about 15 KB for the English text. Past 50 KB before I-13, the work stops for a decision.

### Later increments: where the language will come from

Recorded for their plans (research R-12); nothing in the first increment depends on it.

- **Resolution** (C-2, FR-032): person's `language`, then the organization's `language`, then the
  first tag of `Accept-Language` that `Lang::from_tag` knows, then English. One pure function in
  `horae-i18n`, `resolve(person, org, accept_language) -> Lang`, so the server functions, the
  sign-in page and the email preparation share it. A stored tag `Lang::from_tag` no longer knows
  counts as unset (spec *Edge Cases*).
- **SSR and hydration** (FR-033): `App` reads the resolved tag with `use_loader` over a server
  function. On the first load `use_loader` suspends SSR and serializes the value into the page, so
  the WASM client hydrates with the same `Lang` and no second request; the provider then holds that
  `Tr`. A plain `use_resource` would render English on the server and switch after hydration,
  which SC-006 forbids. The server also writes `<html lang>`; how is that increment's first
  research item, because dx's `index.html` has a fixed `<html>`.
- **Preferences** ([data-model.md](data-model.md),
  [contracts/language-settings.md](contracts/language-settings.md)): stored like the time zone, the
  personal one through `update_profile`, the organization's through the workspace settings.
  Changing either reloads the page.
- **Documents**: the invoice renders in `invoices.language`, or the organization's
  `invoice_language` while it is a draft; issuing (draft to sent) records it. Emails resolve the
  recipient's language without the browser step. Exports never change (C-4).
- **Server messages** (FR-041): `OpError` gets a stable reason and arguments per visible message,
  passed through `ServerFnError` details, and the screen formats the catalog message with the
  English text as fallback. `/api/v1` keeps today's bodies (FR-042).

## PR split

Each PR is small enough to rebase in minutes, lowers the lint baseline of the files it migrates,
and passes `nix flake check` on its own.

| PR | Content | Texts (rsx + other) | Depends on |
| --- | --- | ---: | --- |
| **I-1** | `horae-i18n` with the R-1 additions; English catalog with only the section headers and `-brand`; `i18n.rs` and the provider in `App`; the `mod i18n` include in the 15 UI test files (R-8); lint rule with baseline, fixtures, skill section and evidence; unused-message test; Nix checks; `Dioxus.toml`; AGENTS.md; WASM measured | 0 | PR-4 and the `Button` API merged (C-9) |
| **I-2** | Settings (`settings.rs`, `settings/*`), API tokens (also `admin/api_tokens.rs`) and Tasks (`tasks.rs`, `tasks/editor.rs`), from `26e562bf` plus the files the proof of concept left; HTML dumps; WASM measured | ~170 | I-1 |
| **I-3** | Shared components (`crates/horae/src/components/*`: sidebar, `AdminShell`, layout, toast, modal, `DatePicker` day names, `permission_description`, timer, version, the PR-4 states and `Button`) and `app.rs`, `route.rs` | ~145 | I-1 |
| **I-4** | Clients (`pages/clients.rs`, `clients/*`) | ~110 | I-1 |
| **I-5** | Invoices (`pages/invoices.rs`, `invoices/*`) | ~195 | I-1 |
| **I-6** | Importers, connection: `importers.rs`, `harvest.rs`, `overview`, `connection_card`, `connect_banner`, `connect_outcome`, `change_account`, `credentials_hint`, `oauth`, `redirect_url_field`, `registered_redirect`, `client_settings` | ~185 | I-1 |
| **I-7** | Importers, runs: `run_panel`, `run_view`, `run`, `runs`, `run_list`, `runs_cursor`, `history_table`, `entity_table`, `error_groups`, `missing_people`, `missing_person_row`, `presentation`, `csv` | ~205 | I-1 |
| **I-8** | Admin and permissions: `admin.rs`, `admin/*` (people, add person, rates, workspace, permission editor and its modules) and `permission_audit.rs` | ~250 | I-1 |
| **I-9** | Reports (`pages/reports.rs`, `reports/*`) | ~145 | I-1 |
| **I-10** | New project (`pages/new_project.rs`, `new_project/*`) | ~295 | I-1 |
| **I-11** | Approvals (`pages/approvals.rs`) | ~40 | I-1 |
| **I-12** | Projects (`pages/projects.rs`) and Timesheet (`pages/timesheet.rs`, `timesheet/*`) | ~245 | I-1 |
| **I-13** | Close the ratchet: remove the rule's baseline table, last `NOT_LANGUAGE` review, WASM measured, SC-001 recorded, `IMPLEMENTATION-GAPS.md` and the spec's status updated | 0 | I-2 to I-12 |

The spec splits the importers into one area; at about 390 texts it is two PRs here (I-6, I-7) to
stay within the 200 to 300 the spec asks for. Projects and Timesheet share a PR because both wait
for the most design work (PR-6, PR-7, PR-8); if either is still blocked when the other is free,
they split.

```text
PR-4 ─┐
      ├─► I-1 ─┬─► I-2 ─┐
Button┘        ├─► I-3  │
               ├─► I-4  │
               ├─► …    ├─► I-13 ─► PR-10
               └─► I-12 ┘
```

Parallel lanes: after I-1, every PR from I-2 to I-12 touches its own files and its own section of
`horae.ftl`, so any of them can run at the same time. Their order follows the spec (I-2 first, as
it is ready; then components and areas; Projects and Timesheet last) and is adjusted per area
against open design branches (research R-10). Two PRs adding the same shared id is the only
expected clash: the second to merge drops its copy.

## Open questions for the maintainer

1. **How many migration PRs open at once.** Technically all of I-2 to I-12 can run in parallel.
   Recommendation: two at a time (one area plus I-3 or another small one), so review keeps pace and
   each rebases on the last merged catalog.
1. **Importers as two PRs (I-6, I-7)** instead of the spec's one area, and **Projects with
   Timesheet** as one. Recommendation: as planned; both keep each PR within the spec's 200 to 300
   texts.
1. **The HTML dump tests stay after merge** (research R-7), skipped without `HORAE_HTML_DUMP`.
   Recommendation: keep them, for PR-10 and later refactors; the alternative is to delete them in
   the PR that adds them, once the comparison is recorded.

## Complexity Tracking

No constitution deviations.
