# Research: Interface translation (i18n)

Summary of the study behind [spec.md](spec.md). The full report, in Spanish, is
`ESTUDIO-I18N.md` on branch `study/i18n` (commit `2394e141`), with its measurement scripts in
`study-i18n/`. The proof of concept is commit `26e562bf` on the same branch, based on master
`be859203`. Read them with `git show study/i18n:ESTUDIO-I18N.md` and
`git diff be859203 26e562bf`.

## Text inventory

Counted at `be859203` with `crates/dioxus-lint/src/bin/ui-strings.rs` (proof of concept), which
parses `rsx!` with `dioxus-rsx`, the parser the macro uses, and the rest of the code with `syn`.
Test code is not counted.

| Where | Occurrences | Distinct | Translate? |
| --- | ---: | ---: | --- |
| Interface (`crates/horae/src`: pages, components, `app.rs`, `route.rs`; no gallery) | 1,989 | 1,479 | Yes, first increment. 1,163 text nodes, 104 attributes, 149 props, 573 other literals. |
| Component gallery (`pages/gallery.rs`) | 131 | 117 | No: development page with sample data. |
| Server functions | 46 | 34 | Yes, with the server messages. |
| CLI and `main.rs` | 64 | 58 | No. |
| Operations (`crates/server/src/ops`) | 392 | 325 | Yes, in their own increment, by reason. |
| Importers | 71 | 63 | The ones the import screen and error download show. |
| Budget email | 5 | 5 | Yes, in the recipient's language. |
| Invoice PDF template | 20 | 20 | Yes, in the invoice's language. |
| CSV/XLSX exports | 49 | 30 | No: headers always in English (spec C-4). |
| Sign-in page | 18 | 16 | Yes, by `Accept-Language`. |
| `/api/v1` and Harvest API errors | 85 | 77 | No. |
| `horae-core` | 69 | 69 | Errors by variant; the English text stays. |

The ten interface files with most text are `pages/projects.rs` (141), `pages/timesheet.rs` (84),
`pages/new_project/billing.rs` (66), `pages/importers/harvest/run_panel.rs` (64),
`pages/permission_audit.rs` (60), `pages/admin/permission_editor.rs` (59),
`pages/new_project.rs` (57), `pages/reports.rs` (55), `pages/new_project/basics.rs` (55) and
`pages/importers/harvest/client_settings.rs` (54). Of the 1,989, 1,416 sit in `rsx!` (the lint's
starting point); the other 573 are literals in Rust expressions, which the counter recognises
heuristically (one clear false positive in a random sample of 60).

## Options compared

Each library was measured in a minimal crate (one message with a variable and a plural),
`wasm32-unknown-unknown` release, `wasm-opt -Oz`, gzip -9, against a 10.2 KB baseline.

| Option | WASM added (gzip) | Plurals | Missing key compiles? | Notes |
| --- | ---: | --- | --- | --- |
| `fluent-bundle` + generated methods (chosen) | +82.7 KB alone; +41.9 KB brotli in Horae | CLDR, selectors, terms | No | No Dioxus dependency; Mozilla's reference engine; `.ftl` works with Pontoon, Weblate, Crowdin. |
| `dioxus-i18n` 0.6 | Fluent plus its own (not measured) | Fluent's | Yes | Tied to Dioxus, so emails and PDFs would need another path; SSR not documented. |
| `fluent-templates` 0.15 | Fluent plus `fluent-langneg` (not measured) | Fluent's | Yes | |
| `i18n-embed` + `i18n-embed-fl` | +90.1 KB | Fluent's | No | Keeps the current language inside the loader: one loader per language on the server. |
| `rust-i18n` 4.2 | +16.9 KB | None | Yes | Process-global locale, unsuitable for a server answering several languages. |
| `gettext-rs` / `gettext` | n/a | `Plural-Forms` | Yes | Links C `libintl` (no WASM); the pure-Rust crate is unmaintained since 2019. |
| Keys as Rust enums or constants | 0 | By hand per language | No | Translators would edit Rust; every language rewrites plural logic. |

Crates that generate Rust from `.ftl` (`fluent-static`, `fluent-typed`, `es-fluent`,
`fluent-zero`) do what the proof of concept's `build.rs` does, with under about 40 k downloads
each; the generator is about 200 lines.

If the bundle cost ever matters, the generator can emit plain Rust bodies (`format!`, English
plural) while English is the only language, and link `fluent-bundle` in WASM only when a second
language arrives. That duplicates Fluent's semantics in the generator, so it is not the first
choice; the maintainer accepted the cost (spec C-1).

## Proof of concept

- `crates/i18n` (`horae-i18n`): `build.rs` generates a `Lang` per directory of `locales/` and one
  method per message on `Tr`. It checks syntax, duplicate ids, references to missing messages,
  and, per translation, extra messages and different variables; missing messages are a
  `cargo:warning` and show in English. `src/lib.rs` builds one concurrent `FluentBundle` per
  language on first use with English fallback, plus `split_tag` for sentences with a link or
  span inside. Four unit tests.
- `locales/en/horae.ftl`: 81 messages, ids `<screen>-<what>`, shared `action-*` and `common-*`,
  and the term `-brand`.
- `crates/horae/src/i18n.rs`: `use_tr_provider` in `App` (with `Lang::En`) and `use_tr`, which
  falls back to English without a provider, for component tests that mount one section.
- Settings, API tokens and Tasks migrated: 92 texts, 6 left, each justified (token prefix, a date
  pattern, the theme `eval` and an internal message never shown). The token status moved from
  comparing `&'static str` to an enum.

Results:

- Server-rendered HTML of both screens across their states (12 KB each) was byte for byte
  identical before and after, once random UUIDs and the mock's clock time were removed. The 15
  existing tests of the two screens passed unchanged.
- `dx build --release --locked` built server and WASM.
- WASM (`horae_bg-*.wasm`, dx release): 6,079,624 B to 6,210,274 B (+2.1 %); brotli 1,413,442 B
  to 1,455,375 B (+41,933 B, +3.0 %); gzip -9 +58,349 B (+2.7 %). The increase is almost all the
  Fluent engine. The full English catalog is about 49 KB uncompressed (all 1,561 distinct
  interface texts, gallery included), estimated at 15 KB brotli.

Not verified by the study: the full suite, `nix flake check` and `nix build` (in particular that
crane's `buildDepsOnly` accepts a `build.rs` that reads `locales/`), the whole Settings screen in
a browser, and ICU4X's size in WASM (estimated, not measured).

## Formats

- **Durations**: `horae-core::duration`; PR-6 makes one formatter with the organization and
  person preference. `13:00` is language-neutral; decimal hours need the decimal separator.
- **Money**: `horae-core::money::format_cents` (`USD 10,000.00`); other languages change
  separators and code position.
- **Dates**: `chrono` patterns (`%d %b %Y`, `%A`, `%B`) in about 15 places, and `DatePicker`'s own
  day abbreviations. Other languages change names and order.
- ICU4X does all of this for any language, but its per-language CLDR data costs hundreds of KB in
  WASM unless trimmed; `chrono`'s `unstable-locales` embeds every language at once. For Horae's
  few patterns, month and day names can come from the catalog. None of it is needed while English
  is the only language (spec C-8, FR-061).

## How other tools choose the language

| Tool | Interface language | Invoices |
| --- | --- | --- |
| Harvest | English only ([help](https://support.getharvest.com/hc/en-us/articles/12917201128973-Can-I-use-Harvest-in-a-language-other-than-English)). | One set of translated labels per account, under Invoices > Configure > Translations; a change applies to every invoice, past ones included ([help](https://support.getharvest.com/hc/en-us/articles/12917201128973-Can-I-use-Harvest-in-a-language-other-than-English)). |
| Kimai | Per person, in the user preferences, with English as fallback for missing translations; administrators set the default for new users ([user preferences](https://www.kimai.org/documentation/user-preferences.html), [initial setup](https://www.kimai.org/documentation/initial-setup.html)). | Per invoice template: the template's language drives its translations and formats ([invoice templates](https://www.kimai.org/documentation/invoice-templates.html)). |
| Clockify | Per person only, in Preferences > General ([profile settings](https://clockify.me/help/getting-started/manage-your-profile-settings)). | n/a |
| GitLab | An instance default, set by administrators, plus a per-person preference stored on the server ([localization](https://docs.gitlab.com/administration/settings/localization/)). | n/a |
| Grafana | Server default (`default_language`), organization, team and person; the person's choice wins ([user preferences](https://grafana.com/docs/grafana/next/administration/manage-user-preferences)). | n/a |

Horae's order (spec C-2) is Grafana's without teams and with the browser as the last step before
English. Its invoice language (C-3) is one setting per organization, as in Harvest, but an issued
invoice keeps its language, which Harvest does not do. A per-client language, close to Kimai's
per-template one, is left as a possible extension.

## Language selection

- Order (spec C-2): person, organization, `Accept-Language`, English.
- SSR and hydration: the catalog is compiled into both sides, so both only need the language
  before the first paint. The study proposes reading it in `App` with `use_loader` over a server
  function: per the `dioxus-best-practices` skill, `use_loader` suspends SSR on first load and
  serialises the value into the HTML, so hydration does not fetch it again. `use_resource` would
  render the default language on the server. The server also sets `<html lang>` (today
  `crates/api/src/auth/page.rs` writes `lang="en"`).
- Emails are prepared in `notifications::preparation` with the recipient's language; the invoice
  template receives its labels through `sys.inputs`, as it receives its data today.

## Server messages

The operations' text reaches three places: the interface through the server functions
(`server_fns::op_error` copies it into `ServerFnError::message`, and about 96 branches of the
pages show it as is), `/api/v1` as `detail`, and the Harvest-compatible API. `OpError` already has
`ErrorDetail::reason` (a stable `&'static str`) and `field`. Giving each visible message a reason
and its arguments, passed through `details`, lets the interface translate it with the English
`message` as fallback while the APIs stay as they are (spec C-5, FR-041).

## Notes for planning

- Each lint rule names a `## ` heading of the skill's `SKILL.md` and an evidence file listed in
  `references/source-evidence.md`
  (`every_rule_names_a_skill_heading_and_evidence` in `crates/dioxus-lint/src/main.rs`). The new
  rule needs both; the evidence is the `dioxus-rsx` parser it shares with the macro.
- `ui-strings.rs` already holds the rule's logic (text with a letter, readable attributes, prose
  props) and the exceptions it must allow.

## Planning decisions

Added by `/speckit-plan` ([plan.md](plan.md)). Each entry gives the decision, why, and what else
was weighed. Counts below were taken on master `af6521cb` with the proof of concept's
`ui-strings` counter; they differ slightly from the study's because master moved on (PR-1, PR-2,
PR-7).

### R-1 The catalog crate is the proof of concept's, with four additions

- **Decision**: take `crates/i18n` from `26e562bf` as it is (`build.rs` generator, `Tr`, `Lang`,
  `split_tag`, English fallback, isolation marks off) and add three things: (1) the generator rejects two ids
  that map to the same method (`a-b` and `a_b`) and a variable whose name is a Rust keyword;
  (2) each generated method carries the English text as its doc comment, so an editor shows the
  words when hovering the call; (3) a test that fails when an English message is not used
  anywhere (R-6). `Lang::from_tag` and each language's own name wait for the increment that
  chooses the language, their first caller.
- **Rationale**: the proof of concept already compiles on both targets and kept the HTML
  identical. The additions close gaps a full migration would hit: method collisions and keyword
  variables become likely with 1,400 ids, and the doc comment is the only way a reader of
  `tr.tasks_empty()` sees the text without opening the catalog.
- **Alternatives**: `fluent-static`, `fluent-typed`, `es-fluent` (research above); a proc macro
  instead of `build.rs` (slower to compile, and needs its own crate).

### R-2 One `horae.ftl` per language, with a section per area

- **Decision**: keep one file, `crates/i18n/locales/en/horae.ftl`. The infrastructure PR writes
  a `##` section header for every area of [Delivery](spec.md#delivery) (shared actions, common
  words, glossary terms, components, then one per area), and each migration PR adds its messages
  under its own header.
- **Rationale**: C-6 and SC-003 name `horae.ftl`, so PR-10 edits one file. Pre-made sections keep
  parallel PRs' insertions apart, so Git merges them without conflicts in the usual case. The one
  real clash is two PRs adding the same shared id (`action-save`); `build.rs` rejects the
  duplicate in the merge queue and the fix is to delete one line.
- **Alternatives**: one file per area (`locales/en/clients.ftl`, …). It removes the merge risk and
  `build.rs` could read every `*.ftl` of a directory, but it contradicts the letter of C-6 and
  makes a glossary change touch several files. Not chosen; it stays possible later without
  touching any screen.

### R-3 How the screens reach the catalog

- **Decision**: `crates/horae/src/i18n.rs` from the proof of concept: `use_tr_provider()` in
  `App` puts `Tr::new(Lang::En)` in a context, and `use_tr()` reads it, falling back to English
  when no provider is above (component tests that mount one section). A component calls
  `use_tr()` once at the top; plain functions take `tr: Tr` as a parameter. Labels of
  `horae-core` and `horae-types` enums shown on screen (`ProjectType::label`, the import
  connection status) become functions in `horae` that take `Tr` and the value; the enum keeps no
  English label unless the server or an export also uses it.
- **Rationale**: `Tr` is `Copy` and `PartialEq` and holds only the language, so passing it as a
  prop or capturing it in a closure costs nothing and never makes props compare unequal
  (`dioxus-best-practices`, *Props*). Strings returned by `tr.x()` are new on every render but
  compare equal, so a child with a `String` prop does not re-render for them. FR-003 forbids
  `horae-core` and `horae-types` from depending on the catalog, so the mapping from a value to its
  words lives in the app.
- **Alternatives**: a global `static` language (cannot serve two languages in one server process
  once preferences exist); passing `Tr` down as props everywhere (noise, no gain over a context).

### R-4 Fluent's whitespace, braces and numbers

- **Decision**: three rules in the catalog conventions, checked in review and by the HTML
  comparison (R-7):
  - A message never relies on leading or trailing spaces, because Fluent trims them. Spaces
    between a message and the markup around it stay in the `rsx!`, as `{" "}` or inside the
    element's own text.
  - Literal `{` and `}` in text are written `{"{"}` and `{"}"}`.
  - Counts that select a plural pass as integers; values that are already formatted (money from
    `format_cents`, durations, dates) pass as strings. Fluent writes an integer as its decimal
    digits with no grouping, which is what `format!("{n}")` writes today.
- **Rationale**: these are the three ways a moved text can render differently from today's
  while the build stays green, and FR-011 asks for the same output byte for byte.
- **Alternatives**: `NUMBER()` with options in messages (not needed while English has no
  separators; C-8 keeps number formats in `horae-core`).

### R-5 The lint rule `dioxus/literal-text`

- **Decision**: a sixth rule in `crates/dioxus-lint/src/main.rs`, with the counting logic of
  `ui-strings.rs`: a text node with a letter in a literal segment, a literal value of a readable
  attribute (`placeholder`, `title`, `alt`, `label`, `aria-*` names that are read out,
  `data-tooltip`), and a literal prop of a component that is not in the machine-prop list
  (`class`, `id`, `key`, `href`, `to`, `variant`…). It is a ratchet like the others, with its
  baseline in `baseline.toml`. Two lists in the lint source take text out of the count, each
  entry with a reason: `EXEMPT_FILES` (the gallery) and `NOT_LANGUAGE`, exact literals that are
  not language (`horae_pat_`, format patterns, product names). The rule names a new `SKILL.md`
  section, "Interface text", whose evidence is `rsx/src/text_node.rs` in `dioxus-rsx` 0.7.9,
  added to `references/source-evidence.md`.
- The lint also prints, without failing, the count of prose literals outside `rsx!` markup
  (toasts, messages built in Rust, `if busy { "Saving…" }`) with `ui-strings`' heuristic, so
  each PR of the first increment can quote it (FR-023). `ui-strings.rs` itself is not merged.
- **Rationale**: the lint already parses every `rsx!` with the macro's parser and has the
  ratchet, the per-file baseline and the skill check; a second tool would duplicate them.
  Listing exceptions by literal keeps them visible in one place, and the reason travels with
  each one as FR-021 asks.
- **Alternatives**: an inline marker comment per exception (`// i18n: not language`), which the
  `dioxus-rsx` parser does not see inside `rsx!`; failing on literals outside markup, which the
  heuristic's false positives rule out (spec FR-023).

### R-6 The unused-message check is a test of `horae-i18n`

- **Decision**: `crates/i18n/tests/unused_messages.rs` parses the English catalog with
  `fluent-syntax`, turns every message id into its method name, and fails listing each one that
  appears as `.method_name(` in no `.rs` file under `crates/` outside `crates/i18n`. A message
  referenced only from another message counts as used. It runs in `nix flake check` through the
  tests check, with no database.
- **Rationale**: FR-006. `dead_code` does not warn about unused public methods. The check fails
  rather than warns, because a warning nobody reads lets dead text accumulate, and the fix is one
  deleted line. A consequence for planning: the infrastructure PR adds only messages that some
  code uses, and shared ids (`action-*`, `common-*`) arrive with the first screen that uses them.
- **Alternatives**: a rule of the Dioxus lint (it walks only `crates/horae/src`, and the server's
  emails and the sign-in page will use the catalog later); a `cargo:warning` from `build.rs`
  (cannot see the callers).

### R-7 Proving that nothing changed

- **Decision**: each migration PR's first commit adds, to the screen's existing test file, a test
  that writes the server-rendered HTML of every state the file already covers to
  `$HORAE_HTML_DUMP/<test>.html` and returns early when the variable is unset (the proof of
  concept's `i18n_study_html_snapshot`, renamed). The PR runs it at that commit and at its last
  one, strips the random UUIDs and clock times with one `sed` expression per test, and diffs the
  two. The PR description carries the result. Screens whose tests live in the crate
  (`pages/projects.rs`, `pages/timesheet.rs`, `pages/reports.rs`, `pages/new_project/billing.rs`)
  get the dump test in their own `tests` module. The dump tests stay after merge.
- **Rationale**: SC-002 asks for byte-for-byte identical HTML in each tested state, and the
  existing UI tests already build those states with mocks. Keeping the dump test lets PR-10 and
  later refactors repeat the comparison at no cost.
- **Alternatives**: committed HTML snapshots asserted on every run (they would make every later
  wording or markup change rewrite large fixtures, and PR-10 would touch them all); comparing in a
  browser (not byte for byte, and slower).

### R-8 Tests that include page sources with `#[path]`

- **Decision**: the 15 test files under `crates/horae/tests/` that include a page or component
  with `#[path = "../src/…"]` also include `#[path = "../src/i18n.rs"] mod i18n;`, added by the
  PR that migrates the included file. `horae-i18n` is a normal dependency of `horae`, so the test
  crates can use it without a new dev-dependency.
- **Rationale**: a migrated file names `crate::i18n::use_tr`, and those tests compile it as part
  of their own crate. The proof of concept did this for `api_tokens_ui.rs`.

### R-9 Build and bundle

- **Decision**: `crates/i18n` joins the workspace, `nix/checks/tests.nix` (`cargo nextest run -p horae-i18n`, no database), `nix/checks/clippy.nix` (host and `wasm32-unknown-unknown`, like
  `horae-types`), and `watch_path` in `crates/horae/Dioxus.toml` so `dx serve` rebuilds when a
  catalog changes. The infrastructure PR verifies with `nix build` and
  `nix build .#checks.x86_64-linux.tests` that crane's dummy-source dependency build accepts the
  crate's `build.rs`; the study did not verify it. The package source filter already keeps
  `.ftl` files (it drops only `.nix` and `.md`).
- **Bundle**: SC-005 allows 60 KB brotli over master. The served file is
  `target/dx/horae/release/web/public/assets/horae_bg-*.wasm.br` from `dx build --release --locked`, the same command as `nix/package.nix`. The size is measured on the PR's base and
  head at three points: the infrastructure PR (engine only, expected about +42 KB), the proof of
  concept's screens, and the PR that closes the ratchet (whole catalog). If a measurement passes
  50 KB before the last area, the work stops and the maintainer decides (the fallback in this
  file's *Options compared* is available).
- **Rationale**: Principle V; the crate builds for both targets, so both are linted. Three
  measurements bracket the two costs (engine and text) without a release build in every PR.

### R-10 Order against the design PRs

Which pending design PR touches which area, from `design/IMPLEMENTATION-GAPS.md`:

| Area | PR-3 dialogs | PR-5 filters and selects | PR-6 formats and dates | PR-8 buttons and tables | PR-9 toasts |
| --- | --- | --- | --- | --- | --- |
| Settings, API tokens, Tasks | token revoke | Settings theme, `tasks.rs`, `tasks/editor.rs` | | Tasks table | yes |
| Shared components | `Modal`, `ConfirmDialog` | `form::Select`, `SelectField`, `project_task_picker.rs` | `DatePicker` | `Button`, `Badge`, `Menu` | `toast.rs`, `layout.rs` |
| Clients | archive with projects | filters | | Clients table | yes |
| Invoices | over-invoicing dialogs | | preparation dates | Invoice settings submits | yes |
| Importers | disconnect Harvest, change account | | | | yes |
| Admin and permissions | | `admin/people.rs`, `add_person.rs`, `permission_editor.rs` | | People table | yes |
| Reports | | three selects, filter card | dates, durations | | |
| New project | | | milestone dates | | yes |
| Approvals | | | `PeriodNav` | inline Approve and Reopen, "Approve all" | yes |
| Projects | archive | filters | page formatter removed | Projects table | yes |
| Timesheet | `.ts-modal-*` | | `PeriodNav` | "Submit week" split button | yes |

- **Decision**: no area is free of every pending design PR, so the order cannot avoid them all.
  The rule is the spec's: before an area starts, its PR lists the files that open design branches
  change (`git diff --name-only master...<branch>`) and agrees the order with their authors, so
  one file is not rewritten by both at once. When a design PR is open on an area, the migration
  waits for it to merge, or migrates the rest of the area and leaves that file for a follow-up
  commit. The migration never changes markup or behaviour, so rebasing a design PR over it means
  re-applying `tr.x()` calls, not redoing design work.
- **Rationale**: FR-013, C-9. Migration PRs are mechanical and quick to redo; design PRs are not,
  so the migration yields.

### R-11 Server messages, the sign-in page and formats stay out of the first increment

- **Decision**: the first increment does not touch `server_fns::op_error`, the operations, the
  sign-in page (`crates/api/src/auth/page.rs`), the budget email, the invoice template, the
  exports or the CLI (FR-012). Pages that print a server message as it arrives
  (`error.message`) keep doing so. Date patterns (`format("%d %b %Y")`, about 9 files) stay where
  they are until PR-6 moves them into `horae-core` (C-8); they are `NOT_LANGUAGE` entries in the
  meantime. `DatePicker`'s day abbreviations (`["Mo", …]`) move to the catalog with the shared
  components, with the same text.

### R-12 Choosing the language (later increment, recorded for its plan)

Not part of `tasks.md`. Recorded so the first increment leaves room for it.

- **Storage**: the same pattern as the time zone (`users.timezone` nullable, read with the
  organization's value in `server_fns::auth::current_user`): nullable `users.language` and
  `organizations.language`, written through `ops::profile::update` and an `ops::organization`
  operation under the permission that governs the workspace settings (`CompanyWrite`)
  ([data-model.md](data-model.md)).
- **SSR and hydration**: the study's approach. `App` reads the resolved language with
  `use_loader` over a server function, which suspends SSR on the first load and serializes the
  value into the HTML, so the client hydrates with the same `Lang` and no fetch
  (`dioxus-best-practices`, *Resources and loading*). The provider then holds that `Tr` instead
  of `Tr::new(Lang::En)`, so no screen changes.
- **`<html lang>`**: dx writes a fixed `<html>` without `lang` (checked in a release build's
  `index.html`). How the server sets it per request (an Axum layer over the Dioxus routes that
  rewrites the opening tag, or a custom `index.html`) is that increment's first research item;
  nothing here relies on either.
