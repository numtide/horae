# Tasks: Client archive and restore

Input: [spec.md](spec.md), [plan.md](plan.md). Tests come before the code they
cover and must fail first. Paths are under `crates/horae/` unless noted. No task
authorizes builds, databases or browsers before the coordinator hands them over.

## Phase 1: Setup

- [x] T001 Record governance, clarifications and readiness in specs/012-clients-design/increments/lifecycle/spec.md and checklists/requirements.md (LC-001–LC-010).
- [x] T002 Record writer/lock scope, isolation and error contract in specs/012-clients-design/increments/lifecycle/plan.md.
- [x] T003 Run cross-artifact analysis; record the result at the end of this file.

## Phase 2: Foundation

- [x] T004 After coordinator handoff, start a disposable PostgreSQL in a fresh `mktemp -d /tmp/horae-c03.XXXXXX` (role with CREATEDB) and apply migrations with `sqlx migrate run --source crates/horae/migrations`. Record commands and isolation at the end of this file (LC-010).
- [x] T005 Expose `authorize_legacy_status_write` as `pub(super)` with the policy-not-0 message as a parameter in src/server_fns/projects.rs; keep the project message unchanged (LC-003).

## Phase 3: US1 — Archive a finished client (P1)

Independent check: zero/archived-only/active linked projects; actor and policy changes; snapshots of projects, time, invoices and import records.

- [x] T006 [US1] Add failing tests in src/server_fns/clients/tests.rs:
  - an active linked project refuses archive; the message has no project name, ID or count;
  - zero and archived-only projects archive successfully; a repeated archive is a no-op with no transition;
  - each of these leaves project, time, invoice and import records unchanged;
  - member, inactive actor, policy 1 and foreign actor fail closed, including a role or policy change while waiting;
  - a foreign client ID returns the same not-found as an unknown one;
  - authority is checked before project state, so a forbidden caller never sees the conflict message.
    (LC-001, LC-003, LC-007, LC-008, LC-SC-002, LC-SC-003)
- [x] T007 [US1] In src/server_fns/clients.rs, start `set_client_active_record` with `SET TRANSACTION ISOLATION LEVEL READ COMMITTED, READ WRITE`, pass in the actor, call the shared authority, and refuse an archive transition while a linked project is active. Update the existing callers in src/server_fns/clients/tests.rs (LC-001, LC-003, LC-004, LC-007).

## Phase 4: US2 — Restore a client, then its projects (P1)

Independent check: client restore leaves projects unchanged; single and bulk project restore before and after client restore.

- [x] T008 [P] [US2] Add failing tests:
  - client restore changes only the client, with all snapshots unchanged (src/server_fns/clients/tests.rs);
  - single and bulk project restore under an archived client is refused, the bulk changes nothing and every snapshot is unchanged;
  - an already-active project stays a no-op;
  - project restore succeeds after the client is restored (src/server_fns/projects/bulk_tests.rs).
    (LC-002, LC-005, LC-008, LC-SC-002)
- [x] T009 [US2] Lock the client `FOR SHARE` after `lock_project` and require it active on an inactive → active transition in `set_project_active_in_transaction`, src/server_fns/projects.rs (LC-005).

## Phase 5: US3 — Concurrency and import (P1)

Independent check: both commit orders for each writer, at the server default and with the database default set to REPEATABLE READ; import dry run versus commit; re-run after restore.

- [x] T010 [US3] Add failing race tests in src/server_fns/clients/tests.rs, using the existing blocker and `wait_for_blocked` pattern, each in both orders: archive against project restoration, draft finalization and editor reassignment. Exactly one side succeeds, and no archived client ends with a newly active project. Run the restoration pair a second time after `ALTER DATABASE … SET default_transaction_isolation = 'repeatable read'` on the throwaway database. In the order where archive waits while the writer commits a project and leaves the client row unchanged, archive must still refuse; this test fails without T007's explicit isolation. A writer that loses with a serialization error must leave nothing written (LC-004, LC-SC-001).
- [x] T011 [P] [US3] Add failing import tests in src/importers/harvest/engine_tests.rs (CSV) and src/importers/harvest/sync_tests.rs (API):
  - a new active project under an archived matched client fails with the reason;
  - API: its time entries fail as dependents ("project N failed to import", like `invalid_parent_blocks_dependent_time_but_other_catalog_records_survive`);
  - CSV: each line for that project is reported with its own line number and reason under the existing per-row accounting (`apply_row` → `errored(EntityType::Project, …)`), with no deduplication or suppression;
  - other records import; summary counts reconcile (FR-021); the client stays archived;
  - dry-run and commit reports match;
  - restoring the client then re-running creates each record once;
  - an incoming archived project and an existing match are unchanged;
  - import against archive in both commit orders, at the default and at REPEATABLE READ.
    (LC-004, LC-006, LC-SC-001, LC-SC-004)
- [x] T012 [US3] Make the currency lookup in `resolve_project` read `currency, active … FOR SHARE` and return a `RowFailure` for an active project under an archived client, in src/importers/harvest/resolve.rs (LC-004, LC-006).

## Phase 6: US4 — Clear lifecycle controls (P2)

Independent check: keyboard-only list and detail; Cancel/Escape; delayed and failed responses; Active and Archived filters; narrow screens; both themes.

- [x] T013 [US4] Update browser checks before changing the UI:
  - tests/browser/clients.cjs, list and detail:
    - archive confirmation text has no count; Cancel and Escape change nothing;
    - a refused archive of a client with active projects stays in the dialog with a projects link;
    - a client without active projects archives; restore is direct with no dialog;
    - project and invoice snapshots are unchanged; a delayed response blocks duplicates;
    - focus returns to the live control, or falls back to `clients-heading` / `client-back` when the row leaves the filter;
    - focus is left alone after the user moves it.
  - action-errors.cjs, clients-errors.cjs and clients-permissions.cjs: the new labels and the dialog step.
    (LC-009, LC-SC-005)
- [x] T014 [US4] Build the archive dialog in new src/pages/clients/status.rs on `Modal`. In src/pages/clients.rs: Archive opens the dialog, Restore calls directly, a per-client pending guard, and focus per plan decision 7, including the one-expression fallback in `restore_opener_focus`. src/pages/clients/filters.rs: "Archived clients", with its unit test. src/pages/clients/detail.rs: the `client-status` header action, the Archived badge, the same dialog and a detail refresh. No other detail change (LC-009).

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

## Run evidence

Logs are in `/tmp/horae-c03.l9DeCi` (a disposable socket-only cluster). Every run used
`nix develop -c env DATABASE_URL='postgres://horae@localhost/horae?host=/tmp/horae-c03.l9DeCi'`.

- **T004:** the identity check passed and 48 migrations were applied (`migrate-attempt4.log`).
  Earlier failed attempts are kept: `migrate-attempt1.txt` (an incident:
  `/tmp/horae-c03-migration-incident-20261025.md`) and `migrate-attempt3.log`.
- **T006–T012, behavioral RED:**
  - `test-red1-*` and `test-red2-*`. Setup failures in red1 were fixed before any RED was
    claimed.
  - `test-noiso*` shows the REPEATABLE READ case failing without the explicit isolation.
- **GREEN:**
  - `test-green3-*` (clients 53, projects 199, importers::harvest 190 plus 8 ignored);
  - `test-green4-*` (project_creation 103);
  - `gate4-*` (`detail_navigation` 74, clients 53, projects 199, project_creation 103,
    wasm, clippy).
- **T013, T014:** headless suites against the package built from `00440476`:
  `browser1-{action-errors,clients-errors,clients-permissions,project-detail-header}.log`
  and `browser2-clients.log`, all exit 0. The coordinator accepted this evidence.
- **T015:** `gate5-*` (cache regeneration, offline all-targets build, affected tests,
  wasm, clippy, `fmt --ci`). After the native corrections, `gate7-*` reran wasm, clippy
  and the affected tests.
- **Native first pass (T016, coordinator) on `00440476`:** 23 functional and 27+27 visual
  checks passed. It found two MEDIUM issues:
  - Undo refusal focus left on BODY, with the alert below the fold.
  - Archive dialog button labels breaking into letter columns at 390px with 200% text.
    Both have RED logs against that package (`browser3red-*`) and are fixed in `07e55f8e`
    and `25de7f9f`.
- **Recorded as LOW, not changed in this increment:** focus lands on BODY after a refused
  archive inside its dialog, after a failed list restore, and after a refused
  reactivation in the Project Detail dialog.
- **Open:** T016 needs the remaining native keyboard configurations and the Undo retest.
  T017's final review, T018's full flake check and CI, and T019 are pending.

## Evidence notes

- Race coverage combines two kinds of test; neither alone runs both full
  production paths at once:
  - Writer first: `clients/lifecycle_tests.rs`
    `archive_waiting_for_a_project_writer_sees_its_committed_project` holds hand-written SQL
    with each writer's lock shape (Create, Reassign, Restore) and runs the real
    `set_client_active_record` against it.
  - Archive first: the real writers (`set_project_active_record`,
    `set_projects_active_records`, `finalize_draft_record`, `editing::save_editable_project`
    and the CSV import) run against held SQL with archive's lock shape:
    `projects/client_lifecycle_tests.rs`, `project_creation/client_lifecycle_tests.rs` and
    `engine_tests.rs`.
  - The archive-first tests show that each real writer takes a lock conflicting with
    archive's before it commits, since each blocks until archive commits. Restore and
    import block on the client row. Finalization and editor save block earlier, on the
    organization gate (`AccessChange` against archive's shared gate). They don't show that
    this lock comes before the writer's project write, and no test runs both production
    paths together.
  - Editor save runs at SERIALIZABLE (`editing/save.rs`); loading the editor uses
    REPEATABLE READ (`editing.rs`). So reassignment loses with "The project changed while
    saving. Reload before retrying." at every session default. A restore under a
    REPEATABLE READ session default fails with a 500 database error, never the
    archived-client refusal. The test checks the status code, not PostgreSQL's wording.
- Import under a non-default REPEATABLE READ: a lost race gives a per-record
  `RowFailure` reading "database error: could not serialize access…", not the
  archived-client reason. Later rows needing that client in the same batch fail the same
  way. No project is created and the run continues. The default isolation is READ
  COMMITTED, which reports the archived-client reason. The error mapping stays unchanged
  here.
- Draft finalization and editor reassignment already refused an archived client before
  this increment. Their race test is regression coverage, not a RED.
- Project Detail (`pages/projects.rs`) is a caller affected by LC-005. From the status
  dialog and from Undo, it now shows the "restore the client first" refusal with a link
  to the client, instead of a retry (US2 scenario 2). Coverage:
  `tests/browser/project-detail-header.cjs`. This is a required caller fix, not a new
  feature.
- Success feedback reuses the existing `Toast`: "{name} archived" or "{name} restored",
  as in the client-detail prototype's toast. It is shown only beside a settled, managed
  view. Restore shows "Restoring…" or "Restoring client…" while pending, and each client
  has its own pending guard.
- An ordinary same-binding refresh, such as another row's restore finishing, leaves an
  archive confirmation open, including while it is pending. A failed, denied or rebound
  reload closes it. A pending archive's result and error belong to the client it was sent
  for: a late error never appears in another client's confirmation. While an archive
  request is outstanding, every Archive opener on the list and detail is disabled, even
  after a failed reload was retried. Coverage: the
  busy-overlap, target-change and policy-change steps in `tests/browser/clients.cjs`, and
  `tests/detail_navigation/clients.rs` for rebinding.
- Focus after a refused archive or a failed restore is only observed for now: the browser
  checks print `OBSERVE:` lines. Behavior changes only if native checks show focus is lost.
- Deferred by LC-009 ("no other detail change"): Client Detail's Projects band still says
  "inactive" and shows "Inactive" badges for projects.

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
