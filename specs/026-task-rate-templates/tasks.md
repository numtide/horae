# Tasks: Task rate templates

**Input**: `specs/026-task-rate-templates/` (spec.md, plan.md, research.md, data-model.md,
contracts/interfaces.md, quickstart.md, constitution-amendment.md)

**Prerequisites**: PR 1 (`fix/unknown-currency`) merged. The maintainer answered the spec's
questions and approved the constitution amendment on 2026-11-23 (spec.md, Clarifications).

**Tests**: requested. The spec asks for property tests of the rounding rule and for migration,
import and screen tests; test tasks come before the code they cover.

**Two deliveries** (plan.md, Delivery): Delivery 1 (T001 to T018) is the rounding PR, User Story 3
only. Delivery 2 (T019 to T054) is the templates PR, User Stories 1, 2, 4 and 5, and starts after
Delivery 1 and the OAuth amendment (constitution 7.1.0) have merged.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an unfinished task)
- **[Story]**: the user story in spec.md (US1 to US5)

______________________________________________________________________

# Delivery 1: Round amounts to the currency's precision (User Story 3)

## Phase 1: Setup

- [x] T001 Check the next free migration number on master (0010 if PR 1's 0009 is the latest) and use it for `crates/server/migrations/NNNN_currency_line_amounts.sql` in every later Delivery 1 task

## Phase 2: Foundational (core rule and SQL twin)

- [x] T002 [P] Tests first in `crates/core/src/money.rs`: `currency_decimals` is 0 for JPY and " JPY", 2 for EUR, 3 for KWD, and 2 for an unknown code and for "jpy" (matching stays trimmed and case-sensitive, as today)
- [x] T003 [P] Tests first in `crates/core/src/invoice.rs`: `line_amount_cents(8050, 90, "JPY") == 12100`, `(500, 30, "JPY") == 300`, `(8050, 90, "EUR") == 12075`, `(8050, 90, "BHD") == 12075`, a JPY case whose exact value is 120.496 gives 12000 (single rounding), `i64::MAX` overflow is an error
- [x] T004 Make `currency_decimals` public in `crates/core/src/money.rs`, with a doc comment that it is the only list of currency decimals in Rust
- [x] T005 Change `line_amount_cents` in `crates/core/src/invoice.rs` to take the currency and use the step of research R1, with truncating integer division as today (not `floor`); keep the doc comment on signed inputs and the existing EUR tests passing unchanged
- [x] T006 [P] Properties in `crates/core/tests/properties.rs`: for any non-negative rate and minutes and any currency, the amount is a multiple of the step, within half a step of the exact value, and equals the old two-decimal formula when the step is 1; any sum of amounts in one currency is a multiple of its step; update the Kani harness `line_amount_is_half_up_for_every_input` in `crates/core/src/proofs.rs` for the step
- [x] T007 Write `crates/server/migrations/NNNN_currency_line_amounts.sql`: create `currency_amount_step(text)` (100 for the zero-decimal codes of `currency_decimals`, else 1, matching `btrim(currency)` case-sensitively as Rust does) and `line_amount_cents(bigint, integer, text)` (`IMMUTABLE STRICT PARALLEL SAFE`, data-model.md formula with `div`, not `floor`), drop `line_amount_cents(bigint, integer)`
- [x] T008 Extend the existing parity test `line_amount_cents_sql_matches_rust` in `crates/horae/tests/integration.rs`: every code `currency_decimals` lists, one unknown code, `' JPY'` and `'jpy'`, and amounts within one hundredth of every half; keep the existing negative cases; SQL equals Rust; update the other Rust callers in that file and in `crates/horae/src/server_fns/reports/tests.rs`
- [x] T009 [P] Allow the new SQL functions and remove the old signature in the plugin role allowlist in `crates/server/src/plugin/database.rs`, in the same PR as T007 (the list casts `'public.line_amount_cents(bigint,integer)'::regprocedure`, which fails at startup once the function is gone); run its existing role test and check that the server boots against the migrated database

**Checkpoint**: the workspace does not compile until T011 to T014 use the new signatures.

## Phase 3: User Story 3 - Amounts round to the currency's precision (P2, ships first)

**Goal**: per-entry amounts use `line_amount_cents` with the currency; totals stay sums of lines.

**Independent test**: quickstart.md "Rounding".

- [x] T010 [P] [US3] Tests first: a JPY invoice preview and invoice with lines 12100 and 300 and subtotal 12400 in `crates/server/src/ops/invoices/` tests; for JPY, EUR and KWD invoices, the preview's and the generated invoice's `subtotal_cents` equal the sum of `line.amount_cents` (SC-003); the PDF from `render_invoice_pdf` in `crates/server/src/render.rs` shows "121.00", "3.00" and "124.00" with JPY, since the display keeps two decimals (SC-004); the time report (`ops::reports::time`, `billable_cents`) and its CSV export, the project figures and the amount budget for the same entries give 12100 and 300; an invoice stored before the change keeps its amounts
- [x] T011 [US3] Pass the invoice currency to `line_amount_cents` in `crates/server/src/ops/invoices/entries.rs` (`EntryWithRates::amount`)
- [x] T012 [P] [US3] Use the three-argument SQL function with the row's currency in `crates/server/src/ops/reports.rs` (billable with the report currency; cost with the cost currency)
- [x] T013 [P] [US3] Same in `crates/server/src/ops/projects/reads.rs`
- [x] T014 [P] [US3] Same in `crates/server/src/ops/projects/budgets.rs`

## Phase 4: Delivery 1 polish

- [ ] T015 [US3] On a copy of a real database (`pg_dump` of a deployment, restored locally and `ANALYZE`d; ask the maintainer before taking the dump), record invoice totals, report totals and project figures before and after the migration; they must be equal apart from uninvoiced per-entry amounts in zero-decimal currencies (FR-011)
- [x] T016 Regenerate `.sqlx/` (touch command and `cargo sqlx prepare --workspace -- --features server --all-targets`), then build offline
- [ ] T017 Run `cargo test -p horae-core`, `cargo kani -p horae-core`, the `horae-server` and `horae --features server` tests, and clippy for `horae-server`, `horae-api`, `horae-harvest-api` and `horae --features server`
- [x] T018 `nix fmt` and `nix flake check`; write the PR body, saying that uninvoiced amounts and amount-budget consumption in zero-decimal currencies now round to whole units (so a budget alert may cross its threshold on upgrade), that discounts, taxes, fees and typed lines are unchanged, and that stored invoices do not change

______________________________________________________________________

# Delivery 2: Rate templates (User Stories 1, 2, 4 and 5)

Starts after Delivery 1 and the OAuth amendment (7.1.0) have merged.

## Phase 5: Setup

- [ ] T019 Apply the approved text (with any wording the maintainer approves for review finding H3; see HANDOFF.md) of `specs/026-task-rate-templates/constitution-amendment.md` to `.specify/memory/constitution.md` verbatim, as MINOR 7.1.0 → 7.2.0, with a Sync Impact Report; if 7.1.0 has not landed, stop and ask
- [ ] T020 Check the next free migration number on master (one after Delivery 1's) and use it for `crates/server/migrations/NNNN_task_rate_templates.sql` in every later task

## Phase 6: Foundational (blocks every Delivery 2 story)

The column drop breaks every query that names it, so the core type, the migration and the wire types
come first.

- [ ] T021 [P] Tests first in `crates/core/src/money.rs`: `RateTemplate::parse` accepts "80", "80.5", "0.01", "1,200.50", refuses "80.505", "-1", "" and "0" (R2); `format` gives "80.50"; `apply` returns the same hundredths for "EUR", "JPY" and "BHD"
- [ ] T022 Implement `pub struct RateTemplate` (positive `i64` of hundredths; `parse`, `hundredths`, `format`, `apply(currency) -> i64`) in `crates/core/src/money.rs`; doc comments say a template is not money until applied (research R5)
- [ ] T023 Write `crates/server/migrations/NNNN_task_rate_templates.sql`: clear zero rates (R2), keep every other number whatever its currency (R0, R3), drop `tasks.default_rate_currency`, replace `tasks_default_rate_cents_nonneg` with a `> 0` check, comment the column as a template in hundredths with no currency
- [ ] T024 Migration test in `crates/server/src/db.rs` (`#[sqlx::test(migrator = "crate::db::MIGRATOR", …)]` stopping before and after the migration): task rates with the workspace currency, another currency, no currency and zero; assert numbers kept, zero cleared, column gone, every `project_tasks` row identical
- [ ] T025 [P] Change the wire types in `crates/types/src/task.rs` and `crates/types/src/project_creation.rs` per data-model.md (drop `default_rate_currency`, `TaskCatalog::rate_currency`, the `currency` of `TaskRateEdit::Set`); swap the transport test cases per research R8; doc comments say template

**Checkpoint**: core and schema done; the workspace does not compile until the server phases drop
the column from their queries.

## Phase 7: User Story 1 - A task's default rate fills projects in any currency (P1, MVP)

**Goal**: adding a task to a project in any currency copies the template as the project's task rate.

**Independent test**: quickstart.md "Applying templates".

- [ ] T026 [P] [US1] Tests first in `crates/horae/src/server_fns/project_creation/tests.rs` and `crates/horae/src/server_fns/project_creation/editing/tests/catalog.rs`: a template of 80.50 added from New Project to EUR and USD projects billed by task rate (New Project accepts only `PROJECT_CURRENCIES`), and from the editor to an existing EUR project and to a JPY project inserted by the test fixture, gives a task rate of 8050 each; a project not billed by task rate gets no rate, as today; clearing an existing link's rate in the editor re-applies the template in a JPY project; a typed rate wins; no template gives no rate; changing the template later leaves the project rate; a manager without rate access gets the rate applied without seeing it
- [ ] T027 [US1] In `crates/server/src/ops/projects/creation/finalize.rs` `save_tasks`, read the template only, apply it with `RateTemplate::apply(project currency)` when the typed rate is empty, and remove the "currency is unknown or incompatible" refusal
- [ ] T028 [US1] Same in `crates/server/src/ops/projects/editor/associations.rs`: apply the template, remove "Enter an explicit task rate in the project currency."
- [ ] T029 [P] [US1] Drop the currency from the reads and redaction in `crates/server/src/ops/projects/creation/options.rs`, `crates/server/src/ops/projects/editor/catalog.rs`, `crates/server/src/ops/projects/editor/projection.rs` and `crates/server/src/ops/projects/editor.rs`
- [ ] T030 [US1] In `crates/horae/src/pages/new_project/tasks.rs` and `crates/horae/src/pages/new_project/draft.rs`, show the applied rate as the placeholder ("EUR 80.50 / h", from the template in the draft's currency) instead of "Enter rate" when currencies differ; confirm the project editor's task rows use the same component (`edit_return.rs`) and cover them too; new text in `crates/i18n/locales/en/horae.ftl`
- [ ] T031 [P] [US1] Update `crates/horae/tests/new_project_screen.rs`, `crates/horae/src/pages/new_project/picker_tests.rs`, `crates/horae/src/server_fns/project_creation/import_tests.rs`, `editing/tests.rs` and `editing/tests/canonical_fields.rs` for the new types and the removed refusals
- [ ] T032 [P] [US1] Update `crates/horae/tests/browser/project-task-rates.cjs` and `new-project-task-errors.cjs`: a cross-currency task fills its rate, the old error does not appear

**Checkpoint**: US1 works with the existing editor writing templates through the new types.

## Phase 8: User Story 2 - The task catalog shows templates without a currency (P1)

**Goal**: the Tasks screen and the task editor treat the rate as a template.

**Independent test**: quickstart.md "Applying templates", steps 1 and 4; `task_catalog_ui`.

- [ ] T033 [P] [US2] Tests first in `crates/horae/src/server_fns/projects/task_edit_tests.rs` and `task_write_tests.rs`: set 80.50, clear, preserve; a workspace currency change does not refuse a later edit; 0 is stored as no rate (R2); permissions unchanged
- [ ] T034 [US2] In `crates/server/src/ops/projects/task_writes.rs`, make `authorize_rate` return the template only: keep the `BillableRateWriteAll` check, drop the workspace currency lookup and the "Task rate currency changed" refusal, store no rate for 0; drop the currency from the inserts and updates
- [ ] T035 [P] [US2] Drop the currency from the catalog read in `crates/server/src/ops/projects/task_catalog.rs`
- [ ] T036 [US2] In `crates/horae/src/pages/tasks/editor.rs`, label the field "Default hourly rate" with no currency, parse with `RateTemplate::parse`, show "Current default: 80.50 / h"; strings in `horae.ftl`
- [ ] T037 [US2] In `crates/horae/src/pages/tasks.rs`, `rate_label` writes "80.50 / h" with the "each project's currency" note and "No rate"; remove PR 1's `tasks-rate-currency-not-set` from `horae.ftl` and add the new message
- [ ] T038 [P] [US2] Update `crates/horae/tests/task_catalog_ui.rs`, `crates/horae/src/server_fns/projects/task_activity_tests.rs`, `mutation_tests.rs`, `crates/horae/src/server_fns/projects/task_catalog.rs` (its test asserts `page.rate_currency`), and `crates/horae/src/server_fns/importers/authorization_tests/{task_edits,task_writes,task_catalog,task_activity}.rs`
- [ ] T039 [P] [US2] Update `crates/horae/tests/browser/task-catalog.cjs` and `task-read-permissions.cjs`: no currency next to a template; 80.505 refused; 0 clears the rate (R2)

## Phase 9: User Story 4 - Imports keep Harvest's number (P2)

**Goal**: importers store templates as given and keep the CSV project-task currency check.

**Independent test**: quickstart.md "Imports and interfaces".

- [ ] T040 [P] [US4] Tests first in `crates/server/src/importers/harvest/sync_tests.rs` and `sync_tests/checkpoints.rs`: API 80.5 → 8050, 0 and null → none; CSV new task with 75.00 USD → template 7500, no currency; CSV project-task rate in another currency still refused; a checkpoint JSON that contains `default_rate_currency` commits; a checkpoint holding `default_rate_cents = 0`, with and without `default_rate_currency`, commits and the task has no rate (FR-018)
- [ ] T041 [US4] In `crates/server/src/importers/harvest/resolve.rs` `resolve_task`, drop `rate_currency` and the column from the insert, and drop the `rate_currency` that `crates/server/src/importers/harvest/resolve/fields.rs` feeds it; leave `ensure_project_task` as it is
- [ ] T042 [US4] In `crates/server/src/importers/harvest/resolve/preview.rs`, drop `default_rate_currency` from the snapshot and restore queries, and restore the rate in `ParentSnapshot::restore` with `NULLIF(default_rate_cents, 0)` so an old zero passes the new `> 0` CHECK as no rate (research R4); replace `checkpoint_restores_known_task_currency_and_keeps_old_currency_unknown` with a test that an old checkpoint restores the rate (research R4)
- [ ] T043 [P] [US4] Check `crates/server/src/importers/harvest/parents.rs` still passes `rate_currency: None` and remove the field if nothing else uses it
- [ ] T044 [P] [US4] Stop writing a task currency in `crates/server/src/seed.rs`

## Phase 10: User Story 5 - Existing rates move without surprises (P2)

**Goal**: interfaces and stored money unchanged after the upgrade.

**Independent test**: T024, T049, and quickstart.md "Before upgrading".

- [ ] T045 [P] [US5] In `crates/harvest-api/src/lib.rs` (`TaskRow`, `task_row_to_harvest`) and `crates/harvest-api/src/task_reads.rs`, drop the currency; write the template with `Amount::new(cents, "")`
- [ ] T046 [P] [US5] Update the task tests in `crates/harvest-api/src/pagination_tests/` for the unchanged shape and a JPY-workspace task written at two decimals
- [ ] T047 [P] [US5] Run `cargo test -p horae-api openapi` without `UPDATE_OPENAPI`; `crates/api/openapi/v1.json` must not change
- [ ] T048 [P] [US5] Update the doc comment of `TaskCreated::default_rate_cents` in `crates/server/src/plugin/event.rs`: a template in hundredths with no currency (FR-021); the shape does not change
- [ ] T049 [US5] On a copy of a real database (`pg_dump` of a deployment, restored locally and `ANALYZE`d; ask the maintainer before taking the dump), record project-task rates, invoice totals and report totals before and after the migration; all must be equal, since the rounding shipped in Delivery 1

## Phase 11: Delivery 2 polish

- [ ] T050 [P] Mark spec 011's catalog denomination as superseded by 026 in `specs/011-new-project-screen/research.md`, `data-model.md`, `quickstart.md` and `tasks.md`, saying what stays (no guessed currency, project rates are copies) and what changes (research R10)
- [ ] T051 [P] Update the "Unknown currency" notes in `crates/horae/src/pages/tasks.rs` doc comments and `AGENTS.md` only if they mention a task currency
- [ ] T052 Regenerate `.sqlx/` (touch command and `cargo sqlx prepare --workspace -- --features server --all-targets`), then build offline; `rg -n "default_rate_currency|rate_currency" crates` must list nothing that refers to a task's currency; the expected leftovers are `task_rate_currency` (the project's currency in `types/project.rs` and `ops/projects/detail.rs`) and names about client rates, such as the test `finalization_rechecks_client_role_revision_and_rate_currency` if it still checks client rates only
- [ ] T053 Run `cargo test -p horae-i18n` (unused messages), `cargo run -p horae-dioxus-lint`, clippy for `horae-server`, `horae-api`, `horae-harvest-api` and `horae --features server`
- [ ] T054 `nix fmt` and `nix flake check`; write the PR body with the upgrade query from quickstart.md

## Dependencies

- Delivery 1: T001 first. T004 after T002, T005 after T003, T007 before T008, T005 and T007 before T010 to T014. T015 to T018 last; T016 after every query change.
- Delivery 2 needs Delivery 1 merged and constitution 7.1.0 landed. T019, T020 first. Phase 6 blocks every story: T022 after T021, T023 before T024, T025 before every server and UI task.
- US1 (T026 to T032) and US2 (T033 to T039) touch different files apart from `horae.ftl` (T030, T036, T037: one at a time).
- US4 needs T025; US5 needs every server task before T049.
- Delivery 2 polish last; T052 after every query change.

## Parallel examples

```text
Delivery 1: T002 (money.rs tests) ‖ T003 (invoice.rs tests); T012 ‖ T013 ‖ T014 after T011
US1:        T029 (reads) ‖ T031 (Rust tests) ‖ T032 (browser scripts), after T027/T028
US5:        T045 ‖ T046 ‖ T047 ‖ T048
```

## Implementation strategy

Delivery 1 is small and lands on its own: core rule, SQL twin, four call sites. Delivery 2's MVP is
Phase 6 plus US1 and US2; they must land together, because the column drop leaves no other
consistent state, and US4 and US5 finish the same PR. Stop at each checkpoint and run the matching
quickstart section.
