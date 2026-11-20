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
