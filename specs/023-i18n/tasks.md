# Tasks: Interface translation (i18n), first increment

**Input**: Design documents from `specs/023-i18n/`

**Prerequisites**: [plan.md](plan.md), [spec.md](spec.md), [research.md](research.md),
[data-model.md](data-model.md), [contracts/](contracts/), [quickstart.md](quickstart.md)

**Tests**: included. The spec measures the increment with tests: the catalog's build-time checks
and unit tests (FR-002, FR-005, FR-006), the lint's fixtures (FR-020), and the before-and-after
HTML comparison of every migrated screen (SC-002).

**Scope**: the first increment only (User Stories 1 and 2, spec *Delivery*). User Stories 3 to 5
are later increments with their own plan and tasks; [plan.md](plan.md) *Later increments* records
where they start.

**Organization**: one phase per pull request (plan.md *PR split*), in merge order. Each task
carries the user story it serves. Every PR ends with a checkpoint: `nix flake check` passes on its
own and its description carries what [quickstart.md](quickstart.md) *Per-PR check* asks for.

## Format: `[ID] [P?] [Story?] Description with file path`

- **[P]**: can run in parallel with the other [P] tasks of its phase (different files, no
  dependency on an unfinished task)
- **[USn]**: the user story the task serves

## Dependencies between PRs

| PR | Depends on | Can run alongside |
| --- | --- | --- |
| I-1 | PR-4 (screen states) and the `Button` API merged (C-9) | — |
| I-2 | I-1 | I-3 to I-12 |
| I-3 | I-1 | I-2, I-4 to I-12 |
| I-4 to I-12 | I-1; each also waits for any open design PR on its files (research R-10) | each other, I-2, I-3 |
| I-13 | I-2 to I-12 | — |
| PR-10 (glossary, not part of this list) | I-13 | — |

Every migration PR (I-2 to I-12) follows the same steps, so its tasks are written out per PR:

1. Check open design branches against the area's files and agree any overlap (quickstart step 1).
1. Add the HTML dump test and take the "before" dump (research R-7).
1. Move every visible text of the area into its section of `horae.ftl`, following the catalog
   conventions (plan.md *Catalog conventions*): text nodes, readable attributes, prose props, and
   the literals built in Rust (toasts, inline messages, button states, error fallbacks).
1. Lower the area's lines in `baseline.toml`, take the "after" dump, compare, and write the PR
   description.

______________________________________________________________________

## Phase 1: I-1, the catalog infrastructure

**Goal**: the catalog crate, the provider, the lint rule and the unused-message check exist and
run in CI, with no screen changed (spec *Delivery* item 1). From here on, FR-014 applies.

**Independent test**: quickstart *After I-1*: the crate's tests pass, the lint passes at its new
baseline, `nix build` and the tests check pass, and the four negative checks fail as described.

### Catalog crate

- [X] T001 [US1] Confirm PR-4 and the `Button` API (`design/button-api`) have merged into master, and branch `i18n/infrastructure` from it (C-9)
- [X] T002 [US1] Create `crates/i18n/Cargo.toml` (`horae-i18n`, edition 2024, `fluent-bundle` 0.16 and `unic-langid` 0.9; build- and dev-dependency `fluent-syntax` 0.12) from `study/i18n` `26e562bf`, and add `crates/i18n` to `members` in `Cargo.toml`
- [X] T003 [US1] Copy `crates/i18n/build.rs` from `26e562bf` and move its parsing and checks into `crates/i18n/build/checks.rs`, as functions that return `Result<Catalog, String>` instead of panicking; `build.rs` includes it with `#[path = "build/checks.rs"] mod checks;` and panics on an `Err`. Add the R-1 checks there (two ids that map to one method name, a variable that is a Rust keyword) and emit each message's English text as the generated method's `///` doc comment
- [X] T004 [US1] Copy `crates/i18n/src/lib.rs` from `26e562bf` (`Lang`, `Tr`, `split_tag`, `BUNDLES` with English fallback and isolation off). Keep the unit tests that need no message (`split_tag` on literal strings, `Lang::ALL` and `tag()`); the four that call messages return in I-2 (T027), because I-1's catalog has none
- [X] T005 [US1] Write `crates/i18n/locales/en/horae.ftl` with the conventions header of plan.md *Catalog conventions*, the `-brand` term and the empty `##` sections listed in contracts/catalog.md *Catalog file conventions*; no message yet (each arrives with its first caller, research R-6). Confirm `build.rs` generates an empty `impl Tr` without warnings
- [X] T006 [P] [US1] Add `crates/i18n/tests/unused_messages.rs`: parse `locales/en/horae.ftl` with `fluent-syntax`, map ids to method names, and fail listing every id not found as `.method(` in a `.rs` file under `crates/` outside `crates/i18n` and not referenced by a used message (research R-6)
- [X] T007 [P] [US1] Add `crates/i18n/tests/build_checks.rs`, which includes `build/checks.rs` with `#[path]` and asserts an `Err` naming the id for each check of contracts/catalog.md *Build-time checks*: syntax error, duplicate id, no value, unknown reference, method-name collision, keyword variable, a translation's extra message and other variables; and the missing-message count for a translation that lacks some

### App glue

- [X] T008 [US1] Copy `crates/horae/src/i18n.rs` from `26e562bf` (`use_tr_provider`, `use_tr`, re-exports), add `mod i18n;` in `crates/horae/src/main.rs`, the `horae-i18n` dependency in `crates/horae/Cargo.toml`, and the `use_tr_provider()` call at the top of `App` in `crates/horae/src/app.rs`
- [X] T009 [P] [US1] Add `#[allow(dead_code)] #[path = "../src/i18n.rs"] mod i18n;` to each of the 15 test files that include page or component sources with `#[path]`: `admin_shell.rs`, `api_tokens_ui.rs`, `approval_labels_ui.rs`, `detail_navigation.rs`, `import_jobs_ui.rs`, `new_project_screen.rs`, `own_permissions_ui.rs`, `permission_audit_ui.rs`, `permission_editor_ui.rs`, `profile_settings_ui.rs`, `scoped_reports_ui.rs`, `task_catalog_ui.rs`, `timer_widget.rs`, `timesheet_week_ui.rs`, `trigger_utilities.rs` in `crates/horae/tests/` (research R-8)
- [X] T010 [P] [US1] Add `"../i18n"` to `watch_path` in `crates/horae/Dioxus.toml`

### Lint rule

- [X] T011 [US1] Add the `dioxus/literal-text` rule to `RULES` and its counting (text nodes with a letter, readable attributes, prose props, with `MACHINE_PROPS`, `EXEMPT_FILES` and `NOT_LANGUAGE`) to `crates/dioxus-lint/src/main.rs`, porting the logic of `crates/dioxus-lint/src/bin/ui-strings.rs` from `26e562bf` (contracts/lint-rule.md)
- [X] T012 [US1] Print the informational count of prose literals outside `rsx!` markup after the per-rule totals, without affecting the exit code, in `crates/dioxus-lint/src/main.rs` (FR-023)
- [X] T013 [P] [US1] Add `crates/dioxus-lint/fixtures/literal-text/bad.rs` (one `// next line flagged` site per kind) and `good.rs` (the same markup through `tr.*()`, a formatted value with no letters, machine props, a `NOT_LANGUAGE` literal)
- [X] T014 [P] [US1] Add the `## Interface text` section to `.agents/skills/dioxus-best-practices/SKILL.md` (the catalog rules of the spec for developers: no loose text, `use_tr()` once per component, plain functions take `Tr`, plurals in the message, one message per sentence with `split_tag`, R-4's whitespace and number rules, exceptions and the gallery, and that the ratchet counts per file, so review rejects a change that swaps one loose text for another) and the `rsx/src/text_node.rs` excerpt to `.agents/skills/dioxus-best-practices/references/source-evidence.md`
- [X] T015 [US1] Run `cargo run -p horae-dioxus-lint`, and write `["dioxus/literal-text"]` with its reason and the per-file counts it reports into `crates/dioxus-lint/baseline.toml` (FR-021)

### Build, docs, measurement

- [X] T016 [P] [US1] Add `cargo nextest run -p horae-i18n` beside the other no-database crates in `nix/checks/tests.nix`, and `cargo clippy -p horae-i18n --all-targets` plus the `wasm32-unknown-unknown` clippy line in `nix/checks/clippy.nix`
- [X] T017 [P] [US1] Describe `horae-i18n` in *Architecture*, add `cargo test -p horae-i18n` to *Test & lint*, and add the catalog rule to *Conventions* (new interface text goes in `horae.ftl`) in `AGENTS.md`
- [X] T018 [P] [US2] Note under PR-10 that it edits only `crates/i18n/locales/en/horae.ftl` and tests once the first increment closes, and under PR-8 that it writes its text into the catalog, in `design/IMPLEMENTATION-GAPS.md` (C-6, C-9)
- [X] T019 [US1] Verify crane's dummy-source build with `nix build` and `nix build .#checks.x86_64-linux.tests` (research R-9); if it fails, fix it in `nix/package.nix` and record the fix in the PR
- [X] T020 [US1] Measure the WASM on base and head as quickstart *WASM budget* says and put both sizes in the PR description (expected about +42 KB brotli)
- [X] T021 [US1] Run the four negative checks of quickstart *After I-1* and record the results in the PR description
- [ ] T022 Checkpoint: `nix flake check` passes; PR I-1 opened

______________________________________________________________________

## Phase 2: I-2, Settings, API tokens and Tasks

**Goal**: the proof of concept's screens and the rest of My settings read from the catalog (spec
*Delivery* item 2).

**Independent test**: the dumps of `api_tokens_ui.rs`, `task_catalog_ui.rs`,
`profile_settings_ui.rs` and `own_permissions_ui.rs` are identical before and after; their
existing tests pass unchanged; the lint's counts for the area's files are 0 or a listed exception.

- [ ] T023 [US1] Check open design branches against `pages/settings.rs`, `pages/settings/*`, `pages/admin/api_tokens.rs`, `pages/tasks.rs`, `pages/tasks/editor.rs` under `crates/horae/src/` (quickstart step 1)
- [ ] T024 [US1] Add HTML dump tests (`html_dump`, skipped without `HORAE_HTML_DUMP`) to `crates/horae/tests/api_tokens_ui.rs` and `crates/horae/tests/task_catalog_ui.rs` from the proof of concept's `i18n_study_html_snapshot`, and new ones covering the states those files already build in `crates/horae/tests/profile_settings_ui.rs` and `crates/horae/tests/own_permissions_ui.rs`; take the "before" dump
- [ ] T025 [US1] Apply the proof of concept's migration of `crates/horae/src/pages/settings.rs`, `crates/horae/src/pages/settings/api_tokens.rs`, `crates/horae/src/pages/tasks.rs` and `crates/horae/src/pages/tasks/editor.rs` (`git diff be859203 26e562bf -- <file>`), resolving it against today's master, with its messages in the Settings, API tokens and Tasks sections of `crates/i18n/locales/en/horae.ftl`
- [ ] T026 [US1] Migrate the texts the proof of concept left: the rest of `crates/horae/src/pages/tasks/editor.rs`, `crates/horae/src/pages/settings/profile.rs`, `crates/horae/src/pages/settings/own_permissions.rs` and `crates/horae/src/pages/admin/api_tokens.rs`
- [ ] T027 [US1] Restore the proof of concept's four crate tests that need real messages (a plain message, a variable, plurals with `api-tokens-expiry-days`, `split_tag` on `api-tokens-intro`) in `crates/i18n/src/lib.rs`
- [ ] T028 [US1] Add `NOT_LANGUAGE` entries for the area (`horae_pat_` and any other) with reasons in `crates/dioxus-lint/src/main.rs`; lower the area's lines in `crates/dioxus-lint/baseline.toml`
- [ ] T029 [US1] Take the "after" dump, compare, measure the WASM (quickstart *WASM budget*), and write the PR description (quickstart step 5)
- [ ] T030 Checkpoint: `nix flake check` passes; PR I-2 opened

______________________________________________________________________

## Phase 3: I-3, shared components and the shell

**Goal**: the components every screen uses read from the catalog (spec *Delivery* item 3).

**Independent test**: the dumps of the component tests are identical before and after;
`DatePicker` shows the same day abbreviations; the lint's counts for `components/*`, `app.rs` and
`route.rs` are 0.

- [ ] T031 [US1] Check open design branches against `crates/horae/src/components/*`, `crates/horae/src/app.rs` and `crates/horae/src/route.rs`; PR-3, PR-5, PR-8 and PR-9 change components, so agree which files wait (research R-10)
- [ ] T032 [US1] Add HTML dump tests to `crates/horae/tests/admin_shell.rs`, `crates/horae/tests/timer_widget.rs`, `crates/horae/tests/trigger_utilities.rs` and `crates/horae/tests/detail_navigation.rs` (which mounts the toast, modal, menu, table and badge), and one for the sidebar and layout if no existing test renders them; take the "before" dump
- [ ] T033 [P] [US1] Migrate the shell: `crates/horae/src/components/sidebar.rs`, `layout.rs`, `admin_shell.rs`, `nav.rs`, `page_header.rs`, `back_link.rs`, `session.rs`, `version_info.rs`, `logo.rs`, `theme.rs`, plus `crates/horae/src/app.rs` and `crates/horae/src/route.rs` (the brand through `-brand`)
- [ ] T034 [P] [US1] Migrate the controls: `crates/horae/src/components/button.rs`, `menu.rs`, `combobox.rs`, `select_field.rs`, `form.rs`, `controls.rs`, `modal.rs`, `toast.rs`, `table.rs`, `badge.rs`, `card.rs`, `avatar.rs`, the PR-4 state components, `no_tracking.rs`, `today.rs`, `project_task_picker.rs`, `timer_widget.rs`
- [ ] T035 [P] [US1] Move `WEEKDAYS` and the other visible text of `crates/horae/src/components/date_picker.rs` into the Components section (same abbreviations, FR-011); its `chrono` date patterns stay for PR-6 as `NOT_LANGUAGE` entries (research R-11)
- [ ] T036 [P] [US1] Migrate `crates/horae/src/components/permission_description.rs` (about 50 texts), keeping the descriptions as one message per sentence
- [ ] T037 [US1] Lower the area's lines in `crates/dioxus-lint/baseline.toml`, take the "after" dump, compare, and write the PR description
- [ ] T038 Checkpoint: `nix flake check` passes; PR I-3 opened

______________________________________________________________________

## Phase 4: I-4, Clients

**Goal and independent test** (the same for I-4 to I-12): every text of the area comes from the
catalog; the area's dumps are identical before and after; its existing tests pass unchanged; the
lint's counts for its files are 0 or listed exceptions.

- [ ] T039 [US1] Check open design branches against `crates/horae/src/pages/clients.rs` and `crates/horae/src/pages/clients/*` (PR-3 archive dialog, PR-5 filters, PR-8 table, PR-9 toasts)
- [ ] T040 [US1] Add an HTML dump test for the client list and detail states to `crates/horae/tests/detail_navigation.rs`; take the "before" dump
- [ ] T041 [US1] Migrate `crates/horae/src/pages/clients.rs`, `clients/detail.rs`, `clients/filters.rs`, `clients/form.rs`, `clients/status.rs` into the Clients section of `crates/i18n/locales/en/horae.ftl`, including the scope labels built with `format!`, as messages with a count where they carry one
- [ ] T042 [US1] Lower the area's lines in `crates/dioxus-lint/baseline.toml`, take the "after" dump, compare, and write the PR description
- [ ] T043 Checkpoint: `nix flake check` passes; PR I-4 opened

______________________________________________________________________

## Phase 5: I-5, Invoices

- [ ] T044 [US1] Check open design branches against `crates/horae/src/pages/invoices.rs` and `crates/horae/src/pages/invoices/*` (PR-3 over-invoicing dialogs, PR-6 dates, PR-8 settings submits, PR-9)
- [ ] T045 [US1] Add an HTML dump test for the invoice list and detail states to `crates/horae/tests/detail_navigation.rs`, and one for invoice settings and preparation if a test builds them; take the "before" dump
- [ ] T046 [P] [US1] Migrate `crates/horae/src/pages/invoices.rs`, `invoices/editing.rs`, `invoices/recovery.rs` into the Invoices section
- [ ] T047 [P] [US1] Migrate `crates/horae/src/pages/invoices/settings.rs`, `invoices/defaults_form.rs`, `invoices/preparation.rs` and `invoices/preparation/*` into the Invoices section
- [ ] T048 [US1] Lower the area's lines in `crates/dioxus-lint/baseline.toml`, take the "after" dump, compare, and write the PR description
- [ ] T049 Checkpoint: `nix flake check` passes; PR I-5 opened

______________________________________________________________________

## Phase 6: I-6, Importers: connection

- [ ] T050 [US1] Check open design branches against the files of T052 (PR-3 disconnect and change-account dialogs, PR-9)
- [ ] T051 [US1] Add an HTML dump test for the importer index, the Harvest overview, the connection and OAuth states to `crates/horae/tests/import_jobs_ui.rs`; take the "before" dump
- [ ] T052 [US1] Migrate `crates/horae/src/pages/importers.rs`, `importers/harvest.rs` and, under `crates/horae/src/pages/importers/harvest/`, `overview.rs`, `connection_card.rs`, `connect_banner.rs`, `connect_outcome.rs`, `change_account.rs`, `credentials_hint.rs`, `oauth.rs`, `redirect_url_field.rs`, `registered_redirect.rs`, `client_settings.rs` into the "Importers: connection" section; the connection status label moves from the enum to a function taking `Tr` (research R-3). "Harvest" stays a product name (`NOT_LANGUAGE` only where it stands alone)
- [ ] T053 [US1] Lower the area's lines in `crates/dioxus-lint/baseline.toml`, take the "after" dump, compare, and write the PR description
- [ ] T054 Checkpoint: `nix flake check` passes; PR I-6 opened

______________________________________________________________________

## Phase 7: I-7, Importers: runs

- [ ] T055 [US1] Check open design branches against the files of T057
- [ ] T056 [US1] Add an HTML dump test for the run list, a run in each state, the error groups, missing people and the CSV import to `crates/horae/tests/import_jobs_ui.rs`; take the "before" dump
- [ ] T057 [US1] Migrate, under `crates/horae/src/pages/importers/harvest/`, `run_panel.rs`, `run_view.rs`, `run.rs`, `runs.rs`, `run_list.rs`, `runs_cursor.rs`, `history_table.rs`, `entity_table.rs`, `error_groups.rs`, `missing_people.rs`, `missing_person_row.rs`, `presentation.rs` and `csv.rs` into the "Importers: runs" section, with counts as plural messages. Messages that arrive from the importer jobs (server text) stay as they are (FR-012)
- [ ] T058 [US1] Lower the area's lines in `crates/dioxus-lint/baseline.toml`, take the "after" dump, compare, and write the PR description
- [ ] T059 Checkpoint: `nix flake check` passes; PR I-7 opened

______________________________________________________________________

## Phase 8: I-8, Admin and permissions

- [ ] T060 [US1] Check open design branches against `crates/horae/src/pages/admin.rs`, `crates/horae/src/pages/admin/*` (not `api_tokens.rs`, done in I-2) and `crates/horae/src/pages/permission_audit.rs` (PR-5 selects, PR-8 People table, PR-9)
- [ ] T061 [US1] Add HTML dump tests to `crates/horae/tests/permission_editor_ui.rs`, `crates/horae/tests/permission_audit_ui.rs`, and for the workspace settings to `crates/horae/tests/profile_settings_ui.rs`; take the "before" dump
- [ ] T062 [P] [US1] Migrate `crates/horae/src/pages/admin.rs`, `admin/people.rs`, `admin/add_person.rs`, `admin/rates.rs`, `admin/workspace.rs` into the "People and permissions" section
- [ ] T063 [P] [US1] Migrate `crates/horae/src/pages/admin/permission_editor.rs` and `admin/permission_editor/{draft,recovery,recovery_storage,subjects,templates}.rs` into the "People and permissions" section
- [ ] T064 [P] [US1] Migrate `crates/horae/src/pages/permission_audit.rs` into the "Audit log" section
- [ ] T065 [US1] Lower the area's lines in `crates/dioxus-lint/baseline.toml`, take the "after" dump, compare, and write the PR description
- [ ] T066 Checkpoint: `nix flake check` passes; PR I-8 opened

______________________________________________________________________

## Phase 9: I-9, Reports

- [ ] T067 [US1] Check open design branches against `crates/horae/src/pages/reports.rs` and `crates/horae/src/pages/reports/*` (PR-5 filters, PR-6 dates and durations)
- [ ] T068 [US1] Add an HTML dump test to `crates/horae/tests/scoped_reports_ui.rs` and to the `tests` module of `crates/horae/src/pages/reports.rs`; take the "before" dump
- [ ] T069 [US1] Migrate `crates/horae/src/pages/reports.rs`, `reports/scoped.rs` and `reports/scoped/*` into the Reports section; the grouping labels ("Team", "Person") keep today's words, which PR-10 changes later (FR-011)
- [ ] T070 [US1] Lower the area's lines in `crates/dioxus-lint/baseline.toml`, take the "after" dump, compare, and write the PR description
- [ ] T071 Checkpoint: `nix flake check` passes; PR I-9 opened

______________________________________________________________________

## Phase 10: I-10, New project

- [ ] T072 [US1] Check open design branches against `crates/horae/src/pages/new_project.rs` and `crates/horae/src/pages/new_project/*` (PR-6 milestone dates, PR-9)
- [ ] T073 [US1] Add HTML dump tests to `crates/horae/tests/new_project_screen.rs` and to the `tests` module of `crates/horae/src/pages/new_project/billing.rs`; take the "before" dump
- [ ] T074 [P] [US1] Migrate `crates/horae/src/pages/new_project.rs`, `new_project/basics.rs`, `new_project/date_field.rs`, `new_project/draft.rs`, `new_project/edit_return.rs`, `new_project/protected.rs`, `new_project/session.rs` into the "New project" section
- [ ] T075 [P] [US1] Migrate `crates/horae/src/pages/new_project/billing.rs` (with `ProjectType::label` moving to a function taking `Tr`, research R-3; remove the core method if nothing else uses it), `new_project/invoice_defaults.rs`, `new_project/tasks.rs`, `new_project/team.rs`, `new_project/catalog.rs` into the "New project" section
- [ ] T076 [US1] Lower the area's lines in `crates/dioxus-lint/baseline.toml`, take the "after" dump, compare, and write the PR description
- [ ] T077 Checkpoint: `nix flake check` passes; PR I-10 opened

______________________________________________________________________

## Phase 11: I-11, Approvals

- [ ] T078 [US1] Check open design branches against `crates/horae/src/pages/approvals.rs` (PR-6 `PeriodNav`, PR-8 inline buttons and "Approve all")
- [ ] T079 [US1] Add an HTML dump test to `crates/horae/tests/approval_labels_ui.rs`; take the "before" dump
- [ ] T080 [US1] Migrate `crates/horae/src/pages/approvals.rs` into the Approvals section, with counts as plural messages
- [ ] T081 [US1] Lower the area's line in `crates/dioxus-lint/baseline.toml`, take the "after" dump, compare, and write the PR description
- [ ] T082 Checkpoint: `nix flake check` passes; PR I-11 opened

______________________________________________________________________

## Phase 12: I-12, Projects and Timesheet

- [ ] T083 [US1] Check open design branches against `crates/horae/src/pages/projects.rs`, `crates/horae/src/pages/timesheet.rs` and `crates/horae/src/pages/timesheet/*` (PR-3, PR-5, PR-6 formatter and `PeriodNav`, PR-7 mobile, PR-8, PR-9); split the PR if one of the two is still blocked
- [ ] T084 [US1] Add HTML dump tests to the `tests` modules of `crates/horae/src/pages/projects.rs` and `crates/horae/src/pages/timesheet.rs`, to `crates/horae/tests/timesheet_week_ui.rs`, and for the project detail to `crates/horae/tests/detail_navigation.rs`; take the "before" dump
- [ ] T085 [P] [US1] Migrate `crates/horae/src/pages/projects.rs` into the Projects section; its duration formatter stays as it is for PR-6 (C-8)
- [ ] T086 [P] [US1] Migrate `crates/horae/src/pages/timesheet.rs`, `timesheet/calendar.rs`, `timesheet/data.rs`, `timesheet/people.rs`, `timesheet/tracking.rs` into the Timesheet section
- [ ] T087 [US1] Lower the area's lines in `crates/dioxus-lint/baseline.toml`, take the "after" dump, compare, and write the PR description
- [ ] T088 Checkpoint: `nix flake check` passes; PR I-12 opened

______________________________________________________________________

## Phase 13: I-13, close the ratchet

**Goal**: no loose text remains in the interface's markup, so the lint fails on any new one
(SC-001), and PR-10 can start (User Story 2).

**Independent test**: quickstart *The increment as a whole*.

- [ ] T089 [US1] Migrate any file the lint still counts (a file added since I-1, or one left behind by a design PR), and confirm `crates/dioxus-lint/baseline.toml` has no `["dioxus/literal-text"]` lines left; then remove the table
- [ ] T090 [US1] Review every `NOT_LANGUAGE` and `EXEMPT_FILES` entry and its reason in `crates/dioxus-lint/src/main.rs`; drop entries no literal matches any more
- [ ] T091 [US1] Check the info count of literals outside markup; migrate the remaining real texts, and list the false positives in the PR description (FR-023)
- [ ] T092 [P] [US1] Measure the WASM on I-1's base commit (master before the increment) and on the head (quickstart *WASM budget*); the difference must be at most 61,440 B brotli (SC-005)
- [ ] T093 [P] [US1] Run the FR-012 check of quickstart (no file under the operations, the sign-in page, the exports, the invoice template or the CLI changed since I-1's base) and the FR-003 check (`horae-core` and `horae-types` do not depend on `horae-i18n`)
- [ ] T094 [P] [US2] Check that every glossary word of `DESIGN.md` the screens show today is a term in `crates/i18n/locales/en/horae.ftl` used by its messages, so PR-10 changes it in one place (FR-007, User Story 2 scenario 2); add any missing term
- [ ] T095 [P] [US2] Update `design/IMPLEMENTATION-GAPS.md` (the catalog is done; PR-10 edits `horae.ftl` and tests) and set `specs/023-i18n/spec.md` *Status* to the first increment delivered
- [ ] T096 Checkpoint: `nix flake check` passes; PR I-13 opened

______________________________________________________________________

## After the first increment (not part of these tasks)

- **PR-10** applies the glossary. SC-003 is checked on its diff (quickstart *The increment as a
  whole*).
- **Later increments**, in the spec's order: server messages with reasons (User Story 4), choosing
  the language (User Story 3), documents (User Story 5). Each gets `/speckit-plan` and
  `/speckit-tasks` of its own, starting from plan.md *Later increments*, data-model.md and
  contracts/language-settings.md.

## Parallel execution examples

- **Within I-1**: T006, T007 (crate tests), T009, T010 (app wiring), T013, T014 (lint fixtures and
  skill) and T016, T017, T018 (Nix and docs) touch different files and can be written together
  once T002 to T005 exist; T015 runs after T011.
- **Within an area PR**: the `[P]` migration tasks of I-3, I-5, I-8, I-10 and I-12 touch different
  files and different parts of the area's section, so two people can split them.
- **Across PRs**: after I-1 merges, any of I-2 to I-12 can be open at once (plan.md *Open
  questions* 1 recommends two). They meet only in `horae.ftl` (separate sections) and
  `baseline.toml` (separate lines).

## Implementation strategy

- **MVP**: I-1 and I-2. Together they prove the infrastructure on real screens, settle the
  conventions on 170 texts and give the first WASM measurement with text.
- **Then** I-3, so later areas find the components already in the catalog and share their
  `action-*` and `common-*` ids, followed by the areas in the order above, each when its design
  overlap is clear.
- **Close** with I-13, then hand over to PR-10.
