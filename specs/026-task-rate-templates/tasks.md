# Tasks: Task rate templates

**Input**: `specs/026-task-rate-templates/` (spec.md, plan.md, research.md, data-model.md,
contracts/interfaces.md, quickstart.md, constitution-amendment.md)

**Prerequisites**: PR 1 (`fix/unknown-currency`) merged; the maintainer has answered the spec's open
questions and approved the constitution amendment.

**Tests**: requested. The spec asks for property tests of the rounding rule and for migration,
import and screen tests; test tasks come before the code they cover.

**Open questions**: the tasks follow the recommended answers. A different answer changes these
tasks: FR-006 zero templates (R2): T003, T006, T007, T020, T021, T023, T026; FR-011 rounding scope
(R1): T029 to T031 drop; FR-012 foreign-currency rates (R3): T006, T007.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an unfinished task)
- **[Story]**: the user story in spec.md (US1 to US5)

## Phase 1: Setup

- [ ] T001 Apply the approved text of `specs/026-task-rate-templates/constitution-amendment.md` to `.specify/memory/constitution.md` verbatim, with a Sync Impact Report and the version bump the maintainer chose
- [ ] T002 Check the next free migration number on master (0010 when written; PR 1 owns 0009) and use it for `crates/server/migrations/0010_task_rate_templates.sql` in every later task

## Phase 2: Foundational (blocks every story)

The column drop breaks every query that names it, so the core types, the migration and the wire
types come first.

- [ ] T003 [P] Tests first in `crates/core/src/money.rs`: `RateTemplate::parse` accepts "80", "80.5", "0.01", "1,200.50", refuses "80.505", "-1", "" and (per R2) "0"; `format` gives "80.50"; `apply` returns the same hundredths for "EUR", "JPY" and "BHD"; `currency_decimals` is 0 for JPY, 2 for EUR, 3 for KWD and 2 for an unknown code
- [ ] T004 Implement `pub struct RateTemplate` (positive `i64` of hundredths; `parse`, `hundredths`, `format`, `apply(currency) -> i64`) and make `currency_decimals` public in `crates/core/src/money.rs`; doc comments say a template is not money until applied (research R5)
- [ ] T005 [P] Tests first in `crates/core/src/invoice.rs`: `line_amount_cents(8050, 90, "JPY") == 12100`, `(500, 30, "JPY") == 300`, `(8050, 90, "EUR") == 12075`, `(8050, 90, "BHD") == 12075`, a JPY case whose exact value is 120.496 gives 12000 (single rounding), `i64::MAX` overflow is an error
- [ ] T006 Write `crates/server/migrations/0010_task_rate_templates.sql`: clear zero rates (R2), keep every other number whatever its currency (R0, R3), drop `tasks.default_rate_currency`, replace `tasks_default_rate_cents_nonneg` with a `> 0` check, comment the column as a template in hundredths with no currency, create `currency_amount_step(text)` (100 for the zero-decimal codes of `currency_decimals`, else 1) and `line_amount_cents(bigint, integer, text)` (`IMMUTABLE STRICT PARALLEL SAFE`, data-model.md formula), drop `line_amount_cents(bigint, integer)`
- [ ] T007 Migration test in `crates/server/src/db.rs` (`#[sqlx::test(migrator = "crate::db::MIGRATOR", …)]` stopping before and after 0010): task rates with the workspace currency, another currency, no currency and zero; assert numbers kept, zero cleared, column gone, every `project_tasks` row identical
- [ ] T008 Change `line_amount_cents` in `crates/core/src/invoice.rs` to take the currency and use the step of research R1; keep the existing EUR tests passing unchanged
- [ ] T009 [P] Properties in `crates/core/tests/properties.rs`: for any rate, minutes and currency, the amount is a multiple of the step, within half a step of the exact value, equals the old two-decimal formula when the step is 1, and an invoice subtotal from `invoice_amounts` equals the sum of its lines for 0, 2 and 3 decimal currencies; update the Kani harness `line_amount_is_half_up_for_every_input` in `crates/core/src/proofs.rs` for the step
- [ ] T010 SQL parity test in `crates/server/src/db.rs`: for every code `currency_decimals` lists, one unknown code, and amounts within one hundredth of every half, SQL `line_amount_cents` equals the Rust function
- [ ] T011 [P] Allow the new SQL functions and remove the old signature in the plugin role allowlist in `crates/server/src/plugin/database.rs`; run its existing role test
- [ ] T012 [P] Change the wire types in `crates/types/src/task.rs` and `crates/types/src/project_creation.rs` per data-model.md (drop `default_rate_currency`, `TaskCatalog::rate_currency`, the `currency` of `TaskRateEdit::Set`); swap the transport test cases per research R8; doc comments say template

**Checkpoint**: core and schema done; the workspace does not compile until the server phases drop
the column from their queries.

## Phase 3: User Story 1 - A task's default rate fills projects in any currency (P1, MVP)

**Goal**: adding a task to a project in any currency copies the template as the project's task rate.

**Independent test**: quickstart.md "Applying templates".

- [ ] T013 [P] [US1] Tests first in `crates/horae/src/server_fns/project_creation/tests.rs` and `crates/horae/src/server_fns/project_creation/editing/tests/catalog.rs`: a template of 80.50 added from New Project to EUR, USD and JPY projects and from the editor to an existing EUR project gives a task rate of 8050 each; a typed rate wins; no template gives no rate; changing the template later leaves the project rate; a manager without rate access gets the rate applied without seeing it
- [ ] T014 [US1] In `crates/server/src/ops/projects/creation/finalize.rs` `save_tasks`, read the template only, apply it with `RateTemplate::apply(project currency)` when the typed rate is empty, and remove the "currency is unknown or incompatible" refusal
- [ ] T015 [US1] Same in `crates/server/src/ops/projects/editor/associations.rs`: apply the template, remove "Enter an explicit task rate in the project currency."
- [ ] T016 [P] [US1] Drop the currency from the reads and redaction in `crates/server/src/ops/projects/creation/options.rs`, `crates/server/src/ops/projects/editor/catalog.rs`, `crates/server/src/ops/projects/editor/projection.rs` and `crates/server/src/ops/projects/editor.rs`
- [ ] T017 [US1] In `crates/horae/src/pages/new_project/tasks.rs` and `crates/horae/src/pages/new_project/draft.rs`, show the applied rate as the placeholder ("EUR 80.50 / h", from the template in the draft's currency) instead of "Enter rate" when currencies differ; new text in `crates/i18n/locales/en/horae.ftl`
- [ ] T018 [P] [US1] Update `crates/horae/tests/new_project_screen.rs`, `crates/horae/src/pages/new_project/picker_tests.rs`, `crates/horae/src/server_fns/project_creation/import_tests.rs`, `editing/tests.rs` and `editing/tests/canonical_fields.rs` for the new types and the removed refusals
- [ ] T019 [P] [US1] Update `crates/horae/tests/browser/project-task-rates.cjs` and `new-project-task-errors.cjs`: a cross-currency task fills its rate, the old error does not appear

**Checkpoint**: US1 works with the existing editor writing templates through the new types.

## Phase 4: User Story 2 - The task catalog shows templates without a currency (P1)

**Goal**: the Tasks screen and the task editor treat the rate as a template.

**Independent test**: quickstart.md "Applying templates", steps 1 and 4; `task_catalog_ui`.

- [ ] T020 [P] [US2] Tests first in `crates/horae/src/server_fns/projects/task_edit_tests.rs` and `task_write_tests.rs`: set 80.50, clear, preserve; a workspace currency change does not refuse a later edit; 0 is stored as no rate (R2); permissions unchanged
- [ ] T021 [US2] In `crates/server/src/ops/projects/task_writes.rs`, make `authorize_rate` return the template only: keep the `BillableRateWriteAll` check, drop the workspace currency lookup and the "Task rate currency changed" refusal, store no rate for 0; drop the currency from the inserts and updates
- [ ] T022 [P] [US2] Drop the currency from the catalog read in `crates/server/src/ops/projects/task_catalog.rs`
- [ ] T023 [US2] In `crates/horae/src/pages/tasks/editor.rs`, label the field "Default hourly rate" with no currency, parse with `RateTemplate::parse`, show "Current default: 80.50 / h"; strings in `horae.ftl`
- [ ] T024 [US2] In `crates/horae/src/pages/tasks.rs`, `rate_label` writes "80.50 / h" with the "each project's currency" note and "No rate"; remove PR 1's `tasks-rate-currency-not-set` from `horae.ftl` and add the new message
- [ ] T025 [P] [US2] Update `crates/horae/tests/task_catalog_ui.rs`, `crates/horae/src/server_fns/projects/task_activity_tests.rs`, `mutation_tests.rs`, and `crates/horae/src/server_fns/importers/authorization_tests/{task_edits,task_writes,task_catalog,task_activity}.rs`
- [ ] T026 [P] [US2] Update `crates/horae/tests/browser/task-catalog.cjs` and `task-read-permissions.cjs`: no currency next to a template; 80.505 refused; 0 clears the rate (R2)

## Phase 5: User Story 3 - Amounts round to the currency's precision (P2)

**Goal**: per-entry amounts use `line_amount_cents` with the currency; totals stay sums of lines.

**Independent test**: quickstart.md "Rounding".

- [ ] T027 [P] [US3] Tests first: a JPY invoice preview and invoice with lines 121 and 3 and subtotal 124 in `crates/server/src/ops/invoices/` tests; the detailed report and the project figures for the same entries show 121 and 3; an invoice stored before the change keeps its amounts
- [ ] T028 [US3] Pass the invoice currency to `line_amount_cents` in `crates/server/src/ops/invoices/entries.rs` (`EntryWithRates::amount`)
- [ ] T029 [P] [US3] Use the three-argument SQL function with the row's currency in `crates/server/src/ops/reports.rs` (billable with the report currency; cost with the cost currency)
- [ ] T030 [P] [US3] Same in `crates/server/src/ops/projects/reads.rs`
- [ ] T031 [P] [US3] Same in `crates/server/src/ops/projects/budgets.rs`

## Phase 6: User Story 4 - Imports keep Harvest's number (P2)

**Goal**: importers store templates as given and keep the CSV project-task currency check.

**Independent test**: quickstart.md "Imports and interfaces".

- [ ] T032 [P] [US4] Tests first in `crates/server/src/importers/harvest/sync_tests.rs` and `sync_tests/checkpoints.rs`: API 80.5 → 8050, 0 and null → none; CSV new task with 75.00 USD → template 7500, no currency; CSV project-task rate in another currency still refused; a checkpoint JSON that contains `default_rate_currency` commits
- [ ] T033 [US4] In `crates/server/src/importers/harvest/resolve.rs` `resolve_task`, drop `rate_currency` and the column from the insert; leave `ensure_project_task` as it is
- [ ] T034 [US4] In `crates/server/src/importers/harvest/resolve/preview.rs`, drop `default_rate_currency` from the snapshot and restore queries; replace `checkpoint_restores_known_task_currency_and_keeps_old_currency_unknown` with a test that an old checkpoint restores the rate (research R4)
- [ ] T035 [P] [US4] Check `crates/server/src/importers/harvest/parents.rs` still passes `rate_currency: None` and remove the field if nothing else uses it
- [ ] T036 [P] [US4] Stop writing a task currency in `crates/server/src/seed.rs`

## Phase 7: User Story 5 - Existing rates move without surprises (P2)

**Goal**: interfaces and stored money unchanged after the upgrade.

**Independent test**: T007, T040, and quickstart.md "Before upgrading".

- [ ] T037 [P] [US5] In `crates/harvest-api/src/lib.rs` (`TaskRow`, `task_row_to_harvest`) and `crates/harvest-api/src/task_reads.rs`, drop the currency; write the template with `Amount::new(cents, "")`
- [ ] T038 [P] [US5] Update the task tests in `crates/harvest-api/src/pagination_tests/` for the unchanged shape and a JPY-workspace task written at two decimals
- [ ] T039 [P] [US5] Run `cargo test -p horae-api openapi` without `UPDATE_OPENAPI`; `crates/api/openapi/v1.json` must not change
- [ ] T040 [US5] On a copy of a real database (`pg_dump` of a deployment, restored locally and `ANALYZE`d), record project-task rates, invoice totals and report totals before and after the migration; they must be equal apart from per-entry JPY report amounts (FR-011)

## Phase 8: Polish

- [ ] T041 [P] Mark spec 011's catalog denomination as superseded by 026 in `specs/011-new-project-screen/research.md`, `data-model.md`, `quickstart.md` and `tasks.md`, saying what stays (no guessed currency, project rates are copies) and what changes (research R10)
- [ ] T042 [P] Update the "Unknown currency" notes in `crates/horae/src/pages/tasks.rs` doc comments and `AGENTS.md` only if they mention a task currency
- [ ] T043 Regenerate `.sqlx/` (touch command and `cargo sqlx prepare --workspace -- --features server --all-targets`), then build offline
- [ ] T044 Run `cargo test -p horae-i18n` (unused messages), `cargo run -p horae-dioxus-lint`, clippy for `horae-server`, `horae-api`, `horae-harvest-api` and `horae --features server`
- [ ] T045 `nix fmt` and `nix flake check`; write the PR body with the upgrade query from quickstart.md

## Dependencies

- T001, T002 first. Phase 2 blocks everything: T004 after T003, T008 after T005, T006 before T007 and T010, T012 before every server and UI task.
- US1 (T013 to T019) and US2 (T020 to T026) need Phase 2; they touch different files apart from `horae.ftl` (T017, T023, T024: one at a time).
- US3 (T027 to T031) needs the SQL functions of T006 and T008 only. To ship it alone first, split those functions into their own migration (plan.md Delivery).
- US4 needs T012; US5 needs every server task before T040.
- Polish last; T043 after every query change.

## Parallel examples

```text
Phase 2: T003 (money.rs tests) ‖ T005 (invoice.rs tests) ‖ T012 (types)
US1:     T016 (reads) ‖ T018 (Rust tests) ‖ T019 (browser scripts), after T014/T015
US3:     T029 ‖ T030 ‖ T031 after T028
US5:     T037 ‖ T038 ‖ T039
```

## Implementation strategy

MVP is Phase 2 plus US1 and US2: they must land together, because the column drop leaves no other
consistent state. US3 can go first as its own PR, or with the rest. US4 and US5 finish the same PR.
Stop at each checkpoint and run the matching quickstart section.
