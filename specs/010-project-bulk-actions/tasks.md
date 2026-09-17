# Tasks: Project bulk actions

**Input**: spec.md, plan.md, research.md, data-model.md, contracts/bulk-project-status.md
**Tests**: Required by FR-010; tests precede implementation.

## Phase 1: Setup

- [x] T001 Create isolated worktree/branch and feature artifacts in specs/010-project-bulk-actions/.
- [x] T002 Resolve scope, design and transaction decisions in specs/010-project-bulk-actions/research.md.

## Phase 2: Foundation

- [x] T003 Capture unchanged-page baseline with crates/horae/tests/browser/shared-style-audit.cjs and verify isolated test tooling.

## Phase 3: US1 — Select visible projects (P1)

**Goal**: Accurate visible-only selection using shared controls.
**Independent test**: Toggle individual/all/mixed states and change filters without mutations.

- [x] T004 [P] [US1] Add failing checkbox default/mixed/compact/disabled tests in crates/horae/tests/trigger_utilities.rs (FR-002, FR-009).
- [x] T005 [P] [US1] Add failing selection, filters, member and oversized cases in crates/horae/tests/browser/project-bulk-actions.cjs (FR-001–FR-004).
- [x] T006 [US1] Extend optional Checkbox states in crates/horae/src/components/controls.rs, preserving defaults.
- [x] T007 [US1] Implement visible-row selection/count/Actions in crates/horae/src/pages/projects.rs with opt-in grid structure in crates/horae/assets/css/horae.css (FR-002–FR-004, FR-009).
- [x] T008 [US1] Adapt production-page fixture wiring in crates/horae/tests/detail_navigation.rs and retain browser layout assertions in crates/horae/tests/browser/projects-design.cjs and responsive-layout.cjs.

## Phase 4: US2 — Archive/reactivate selection (P1)

**Goal**: Authorized atomic status changes with explicit confirmation and recovery.
**Independent test**: Cancel, archive two, reactivate two, retry after failure; unauthorized and invalid batches change nothing.

- [x] T009 [P] [US2] Add failing validation/rollback/no-op/concurrency/preservation tests in crates/horae/src/server_fns/projects/bulk_tests.rs (FR-006–FR-007, SC-002).
- [x] T010 [P] [US2] Add real-server authorization and confirmation/busy/failure/retry browser cases in crates/horae/tests/browser/project-bulk-actions.cjs (FR-001, FR-005, FR-008).
- [x] T011 [US2] Implement authorized batch mutation, sorted transaction locks and postcommit dispatch in crates/horae/src/server_fns/projects.rs (FR-001, FR-006–FR-007).
- [x] T012 [US2] Implement mounted confirmation, pending/error/success and resource refresh in crates/horae/src/pages/projects.rs (FR-005, FR-008, SC-001).
- [x] T013 [US2] Regenerate and verify .sqlx/ with server/all-targets against disposable data if any query macros change.

## Phase 5: Polish and verification

- [x] T014 Wire the new browser suite into crates/horae/tests/browser/run-design-checks.sh (FR-010).
- [x] T015 Run Rust/server/WASM checks and all browser suites; audit shared-style snapshots for eight pages at three widths and both-theme selection keyboard flows. Record evidence in specs/010-project-bulk-actions/quickstart.md (FR-009–FR-010, SC-003–SC-004).
- [ ] T016 Review CSS/framework/security diff, update DESIGN.md as needed, format, commit and open one scoped PR with deviations and verification results in its body.

## Dependencies and execution order

T001–T003 → US1 → US2 integration → T014–T016. T004/T005 are independent tests; T009/T010 are independent server/browser tests. T011 depends on T009 and can be implemented independently of UI; T012 depends on T007/T010/T011. Shared-file edits remain sequential.

## Implementation strategy

Validate selection first, then atomic mutations and confirmation. Both stories are required for this delivery; selection alone is not completion. No bulk task/tag/edit/delete/pin features. No live imported-data mutations or automatic PR merge.
