# Tasks: Client archive and restore

Input: [spec.md](spec.md), [plan.md](plan.md). Tests come before the code they
cover and must fail first. Paths are under `crates/horae/` unless noted. No task
authorizes builds, databases or browsers before the coordinator hands them over.

## Phase 1: Setup

- [x] T001 Record governance, clarifications and readiness in specs/012-clients-design/increments/lifecycle/spec.md and checklists/requirements.md (LC-001–LC-010).
- [x] T002 Record writer/lock scope, isolation and error contract in specs/012-clients-design/increments/lifecycle/plan.md.
- [x] T003 Run cross-artifact analysis; record the result at the end of this file.

## Phase 2: Foundation

- [ ] T004 After coordinator handoff, start a disposable PostgreSQL in a fresh `mktemp -d /tmp/horae-c03.XXXXXX` (role with CREATEDB) and apply migrations with `sqlx migrate run --source crates/horae/migrations`. Record commands and isolation at the end of this file (LC-010).
- [ ] T005 Expose `authorize_legacy_status_write` as `pub(super)` with the policy-not-0 message as a parameter in src/server_fns/projects.rs; keep the project message unchanged (LC-003).

## Phase 3: US1 — Archive a finished client (P1)

Independent check: zero/archived-only/active linked projects; actor and policy changes; snapshots of projects, time, invoices and import records.

- [ ] T006 [US1] Add failing tests in src/server_fns/clients/tests.rs:
  - an active linked project refuses archive; the message has no project name, ID or count;
  - zero and archived-only projects archive successfully; a repeated archive is a no-op with no transition;
  - each of these leaves project, time, invoice and import records unchanged;
  - member, inactive actor, policy 1 and foreign actor fail closed, including a role or policy change while waiting;
  - a foreign client ID returns the same not-found as an unknown one;
  - authority is checked before project state, so a forbidden caller never sees the conflict message.
  (LC-001, LC-003, LC-007, LC-008, LC-SC-002, LC-SC-003)
- [ ] T007 [US1] In src/server_fns/clients.rs, start `set_client_active_record` with `SET TRANSACTION ISOLATION LEVEL READ COMMITTED, READ WRITE`, pass in the actor, call the shared authority, and refuse an archive transition while a linked project is active. Update the existing callers in src/server_fns/clients/tests.rs (LC-001, LC-003, LC-004, LC-007).

## Phase 4: US2 — Restore a client, then its projects (P1)

Independent check: client restore leaves projects unchanged; single and bulk project restore before and after client restore.

- [ ] T008 [P] [US2] Add failing tests:
  - client restore changes only the client, with all snapshots unchanged (src/server_fns/clients/tests.rs);
  - single and bulk project restore under an archived client is refused, the bulk changes nothing and every snapshot is unchanged;
  - an already-active project stays a no-op;
  - project restore succeeds after the client is restored (src/server_fns/projects/bulk_tests.rs).
  (LC-002, LC-005, LC-008, LC-SC-002)
- [ ] T009 [US2] Lock the client `FOR SHARE` after `lock_project` and require it active on an inactive → active transition in `set_project_active_in_transaction`, src/server_fns/projects.rs (LC-005).

## Phase 5: US3 — Concurrency and import (P1)

Independent check: both commit orders for each writer, at the server default and with the database default set to REPEATABLE READ; import dry run versus commit; re-run after restore.

- [ ] T010 [US3] Add failing race tests in src/server_fns/clients/tests.rs, using the existing blocker and `wait_for_blocked` pattern, each in both orders: archive against project restoration, draft finalization and editor reassignment. Exactly one side succeeds, and no archived client ends with a newly active project. Run the restoration pair a second time after `ALTER DATABASE … SET default_transaction_isolation = 'repeatable read'` on the throwaway database. In the order where archive waits while the writer commits a project and leaves the client row unchanged, archive must still refuse; this test fails without T007's explicit isolation. A writer that loses with a serialization error must leave nothing written (LC-004, LC-SC-001).
- [ ] T011 [P] [US3] Add failing import tests in src/importers/harvest/engine_tests.rs (CSV) and src/importers/harvest/sync_tests.rs (API):
  - a new active project under an archived matched client fails with the reason;
  - API: its time entries fail as dependents ("project N failed to import", like `invalid_parent_blocks_dependent_time_but_other_catalog_records_survive`);
  - CSV: each line for that project is reported with its own line number and reason under the existing per-row accounting (`apply_row` → `errored(EntityType::Project, …)`), with no deduplication or suppression;
  - other records import; summary counts reconcile (FR-021); the client stays archived;
  - dry-run and commit reports match;
  - restoring the client then re-running creates each record once;
  - an incoming archived project and an existing match are unchanged;
  - import against archive in both commit orders, at the default and at REPEATABLE READ.
  (LC-004, LC-006, LC-SC-001, LC-SC-004)
- [ ] T012 [US3] Make the currency lookup in `resolve_project` read `currency, active … FOR SHARE` and return a `RowFailure` for an active project under an archived client, in src/importers/harvest/resolve.rs (LC-004, LC-006).

## Phase 6: US4 — Clear lifecycle controls (P2)

Independent check: keyboard-only list and detail; Cancel/Escape; delayed and failed responses; Active and Archived filters; narrow screens; both themes.

- [ ] T013 [US4] Update browser checks before changing the UI:
  - tests/browser/clients.cjs, list and detail:
    - archive confirmation text has no count; Cancel and Escape change nothing;
    - a refused archive of a client with active projects stays in the dialog with a projects link;
    - a client without active projects archives; restore is direct with no dialog;
    - project and invoice snapshots are unchanged; a delayed response blocks duplicates;
    - focus returns to the live control, or falls back to `clients-heading` / `client-back` when the row leaves the filter;
    - focus is left alone after the user moves it.
  - action-errors.cjs, clients-errors.cjs and clients-permissions.cjs: the new labels and the dialog step.
  (LC-009, LC-SC-005)
- [ ] T014 [US4] Build the archive dialog in new src/pages/clients/status.rs on `Modal`. In src/pages/clients.rs: Archive opens the dialog, Restore calls directly, a per-client pending guard, and focus per plan decision 7, including the one-expression fallback in `restore_opener_focus`. src/pages/clients/filters.rs: "Archived clients", with its unit test. src/pages/clients/detail.rs: the `client-status` header action, the Archived badge, the same dialog and a detail refresh. No other detail change (LC-009).

## Phase 7: Verification and delivery

- [ ] T015 Run in order, one at a time, against T004's database:
  - `touch crates/horae/src/main.rs crates/horae/tests/*.rs && cargo sqlx prepare --workspace -- --features server --all-targets`, then stage `.sqlx/`;
  - `SQLX_OFFLINE=true cargo test -p horae --features server --all-targets --no-run`;
  - `DATABASE_URL=… cargo test -p horae --features server --bin horae server_fns::clients::`;
  - `DATABASE_URL=… cargo test -p horae --features server --bin horae server_fns::projects::`;
  - `DATABASE_URL=… cargo test -p horae --features server --bin horae importers::harvest::`;
  - `DATABASE_URL=… cargo test -p horae --features server --test detail_navigation`;
  - `SQLX_OFFLINE=true cargo clippy -p horae --features server --all-targets -- -D warnings`;
  - `cargo check -p horae --features web --target wasm32-unknown-unknown`;
  - `nix fmt -- --ci`.
  Rerun the focused tests on the committed head after any formatting change. Record exact results here (LC-010).
- [ ] T016 Coordinator browser/native pass: `run-design-checks.sh clients action-errors clients-errors clients-permissions` plus native checks in both themes, keyboard only, at 320/390/768/1440px and 200% text, with delayed and failed responses. Record the evidence (LC-009, LC-SC-005).
- [ ] T017 Two independent reviews; fix critical/high findings and rerun the affected checks.
- [ ] T018 Final local `nix flake check` on the candidate head with a single build owner. Small unsigned commits. A PR only after coordinator acceptance, and its exact head must pass mandatory CI. No merge here.
- [ ] T019 After T015–T017 pass, replace the sentence in specs/015-scoped-permissions/contracts/client-permissions.md that says the legacy operation "changes only the client without this prerequisite". The replacement records, as implemented and tested evidence, that policy 0 now enforces the FR-036 prerequisite under the legacy role. No other 015 change.

## Dependencies

T001–T003 → T004 → T005 → US1 (T006–T007) → US2 (T008–T009) → US3 (T010–T012) →
US4 (T013–T014) → T015–T019. US2 and the import half of US3 depend only on T005,
not on US1. T010 needs T007, T009 and T012. Shared files (`clients/tests.rs`,
`projects.rs`, `pages/clients.rs`) are edited one task at a time.

Parallel: T008 and T011 write different test files; T013 can be written alongside
T011. Database runs stay serial on the one disposable instance.

MVP: US1 + US2 + US3 together. Archive without the restore and import checks
would let new active projects appear under archived clients, so none ships alone.

## Analysis

`speckit-analyze` rerun, 2026-10-25, read-only, after the coordinator-approved
remediation pass. The first run's findings are resolved:

- G1: T008 now cites LC-008 and LC-SC-002.
- G2: T006 now cites LC-SC-003 and asserts that foreign and unknown client IDs look the same.
- U1: the import race moved to T011, which writes the importer test files.
- L1: T015 lists the exact commands.

Coordinator findings are also folded in:

- Archive confirmation and the detail entry point (spec Governance, US4, LC-009; plan decision 6).
- Restore stays direct, because no approved source asks to confirm it.
- Explicit READ COMMITTED and the writer-side reasoning (plan decision 5; T007, T010, T011).
- Focus fallback for rows leaving the filter (plan decision 7; T013, T014).
- Authority before state disclosure (plan error contract; T006).
- CSV per-line errors under the existing accounting (T011).
- The 015 sentence only as post-test evidence (T019).

Result: 15/15 requirement and criteria keys are cited by tasks. No constitution
conflict, duplicate, placeholder, open clarification or unmapped task. One LOW
item stays open: the plan's archive wording is draft copy, to be checked in T016.
