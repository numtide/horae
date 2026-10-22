# Tasks: project budget remaining tile

Input: [spec.md](spec.md), [plan.md](plan.md), [research.md](research.md).
Tests come first and must be seen failing before the matching implementation.
All fixtures are disposable. No schema change, dependency, policy activation,
Harvest write, or edit to reference208/212.

Status 2026-10-22: the foundation and server tasks below are implemented
locally and uncommitted, awaiting review. No UI task has started. This is not
delivery of the tile, the budget feature or 004.

## Phase 1 — Setup

- [x] T001 Coordinator decisions recorded in [spec.md](spec.md#clarifications)
  and [plan.md](plan.md#implementation-record):
  - P2 and P3 approved; O3 decided;
  - O1 explicitly deferred by the user; O2 proposal pending;
  - C3 method (existing lock-wait pattern);
  - the build slot and a disposable `mktemp` database on a free non-default
    port.

## Phase 2 — Foundation (blocks all stories)

- [x] T002 [P] `format_hours2` regressions in `crates/core/src/duration.rs`.
  `format_hours2_keeps_ordinary_output_and_sign` checks float equality over
  −100 000..=100 000 only, plus `-30`/`1`.
  `format_hours2_is_exact_where_float_division_is_not` checks
  `9_007_199_254_740_991 → "150119987579016.52"`,
  `i64::MAX → "153722867280912930.12"` and
  `i64::MIN → "-153722867280912930.13"`. Red observed: the float gave `.53`.

- [x] T003 Integer half-up `format_hours2` (unit-free); `format_decimal` and
  `format_hhmm` untouched. Core suite: 193 passed.

- [x] T004 [P] Snapshot tests in
  `crates/horae/src/server_fns/projects/canonical_read_tests/concurrency.rs`
  (only plain reads; authority rows untouched by the data writers):

  - `detail_view_reads_identity_and_workflow_labels_from_one_snapshot`: locks
    `project_tasks`, renames the project and links a task. Red on READ
    COMMITTED: `("Widget", 1)` vs `("Widget", 0)`.
  - `detail_view_reads_identity_and_budget_from_one_snapshot`: locks
    `time_entries`, renames the project, changes the allowance and inserts an
    entry. It ran red only because the stub returned no budget; the READ
    COMMITTED baseline had already been replaced, so this test proves
    coherence green-only.
  - `detail_view_reports_a_conflict_after_three_serialization_failures`
    (C3): organization writer, then actor writer, then an organization writer
    queued behind attempt 2, via `wait_for_blocked` and blocking pids. Red on
    READ COMMITTED (wait timed out); green after T005.
  - `detail_view_snapshot_preserves_inherited_connection_settings`: green on
    both, as a guard.

  The existing organization-wait, actor-wait, reader-first-hold and
  cancellation fences already loop over `Reader::DetailView` and stay green.

- [x] T005 `ReadAccess::begin_snapshot` and the shared `authorize` loader in
  `crates/horae/src/server_fns/projects/read_access.rs`:

  - `REPEATABLE READ, READ WRITE`, then `configure_transaction_limits`;
  - 3 fresh attempts on 40001, then the existing conflict message;
  - `begin` behaviour unchanged; `snapshot::manager` unused.

- [~] T006 Core headline in `crates/core/src/budget.rs`.

  - Done: snake_case serde `BudgetHeadline`, and `headline` (one
    `allocated_totals` call plus an all-blank check). Four red→green tests cover
    the nine arithmetic cases; core suite: 197 passed.
  - Moved: serialization is checked in server tests (core has no
    `serde_json`).
  - **Not implemented:** `remaining_percent`. O1 is user-deferred; no
    percentage rule.

- [x] T007 `spend_in_transaction(connection, org, viewer, permissions, Option<project>, include_money) -> Result<_, sqlx::Error>` in
  `crates/horae/src/server_fns/projects.rs`. `include_money` was added for
  P2B. `fetch_project_spend` keeps its signature and error mapping and passes
  `true`. The spend, privacy and invoice suites are green.

- [x] P1A (review) Lock-only access gate. `db::lock_organization(AccessChange)`
  now writes a same-value row version.

  - Red first through production `remove_assignment`.
  - Added: the canonical editor `report_visibility` revocation (unchanged
    managers), reader-first membership removal, and stored-row-unchanged and
    rollback cases.
  - Behaviour change and cost are recorded in
    [plan.md](plan.md#implementation-record).

- [x] P2B (review) Budget valuation overflow: metadata preflight,
  22003-only savepoint, `fetch_progress(…, allocated_only)` and
  `include_money`. The two reproductions were red first; there are 9 overflow,
  failure and aggregate cases in total.

## Phase 3 — US1: see how much budget is left (P1)

Independent test (server part): an hour budget of 120 minutes against 150
qualifying minutes gives `Exact { 120, 150, -30 }`, equal to the Projects
list's inputs for the same date.

- [x] T008 [US1] `canonical_read_tests/detail_budget.rs` now has 23 tests. The
  first 12 were red against the `None` stub, then green:

  - legacy policy 0 hours (raw lifetime) and fees (parity with
    `fetch_project_spend`, 9 000), plus blank → `Unallocated`;
  - rounding (15 min up) and `include_nonbillable`, with parity against
    `progress_for_viewer`;
  - monthly 2028-02-29 / 2028-03-01 boundaries;
  - task blank/zero/overrun/all-blank, and task allowance overflow;
  - person allowances;
  - configured kind `none`;
  - no scope details in the JSON;
  - unchanged `xmin`, notification and outbox counts.

  `archived_project_keeps_its_configured_budget` was added later; green (2/2
  gate). The P2B cases are listed above.

- [x] T009 [US1] `project_budget` in `crates/horae/src/server_fns/budgets.rs`,
  per [data-model.md](data-model.md#derivation).

- [x] T010 [US1] `ProjectDetailView.budget: Option<ProjectDetailBudget>`.
  `fetch(…, today)` uses `begin_snapshot` and loads the budget before commit.
  `get_project_detail_view` captures the UTC date once; its public API is
  unchanged. All 22 test callers pass the fixed `today()` (2026-09-07). Both
  SSR fixtures (`tests/detail_navigation.rs` and
  `tests/detail_navigation/detail.rs`) gain `budget: None`. The second was
  found by review.

- [ ] T011 [P] [US1] Pure `budget_tile` mapping with unit tests in
  `crates/horae/src/pages/projects.rs`, per the
  [UI contract](contracts/budget-summary.md#ui-contract). Blocked on review of
  the UI pattern, and on O1 for any percentage.

- [ ] T012 [US1] Render `ProjectBudgetTile` (see contract). Blocked as T011.

## Phase 4 — US2: absent or unavailable budget (P1)

- [x] T013 [P] [US2] Authorization in `detail_budget.rs`, all green:

  - `fee_figures_require_current_financial_authority` covers ReadAll vs
    BillableRateReadAll, managed with and without designation, and hours
    without money authority;
  - `unallocated_fee_budget_is_still_withheld_without_financial_authority`;
  - `legacy_fee_figures_follow_spend_redaction_for_canonical_readers`;
  - withheld JSON is exactly
    `{"kind":"amount","currency":"EUR","period_key":"lifetime","headline":"unavailable"}`.

  Unreadable and foreign-org projects remain covered by the existing
  `detail.rs` tests. `foreign_client_parent_hides_configured_and_legacy_budgets`
  uses the existing `canonical_project_reads_exclude_foreign_client_parent_links`
  fixture and expects not found for configured and legacy budgets; green.

- [ ] T014 [P] [US2] SSR cases in `crates/horae/tests/detail_navigation.rs`.

- [ ] T015 [US2] No-budget and unavailable UI states. Blocked as T011.

## Phase 5 — US3: trust the tile under change (P1)

- [x] T016 [US3] Audit recorded in [plan.md](plan.md#implementation-record).
  After `begin_snapshot`, every data statement is a plain SELECT. The only
  other statements are the budget savepoint controls, which neither lock nor
  write business data. The T004 coherence tests and the fence tests (including
  the new P1A cases) are green.

## Phase 6 — Polish and delivery

- [ ] T017 Browser suite `crates/horae/tests/browser/project-budget-tile.cjs`.

- [x] T018 Coordinated build for the foundation and server scope, recorded in
  [plan.md](plan.md#implementation-record):

  - `.sqlx` regenerated;
  - offline full suite green (1535 + 15 integration targets, including
    `detail_navigation`);
  - core, server and WASM clippy green;
  - `nix fmt` applied and `--ci` clean.

  Rerun on the rebased exact head before publishing.

- [~] T019 Independent reviews of the foundation and server source are
  bounded-approved, and Spec Kit analysis has been rerun. UI review is
  outstanding.

- [ ] T020 Publish only with coordinator approval and green required CI.

## Dependencies

T011/T012/T015 wait for UI-pattern review; percentages wait for O1. T014
needs T011. T017 needs T012/T015. T018 needs a granted build slot. One author;
no concurrent heavy builds or browsers.

## Traceability

| Requirement | Tasks |
|-------------|-------|
| BFR-001 | T011, T012, T015, T017 (percentage: O1 deferred) |
| BFR-002 | T006, T007, T008, T009 |
| BFR-003 | T002, T003, T006, T008, T011, T013 |
| BFR-004 | T008, T009, T010 |
| BFR-005 | T004, T005, T010, T016 |
| BFR-006 | T009, T013 |
| BFR-007 | T014, T015, T017 |
| BFR-008 | T012, T017 |
| BFR-009 | T002, T003, T011 |
| BSC-001 | T008, T013 |
| BSC-002 | T004, T013, T016 |
| BSC-003 | T014, T017, T018, T019, T020 |
