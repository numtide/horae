# Tasks: Scoped Roles and Permissions

**Input**: [spec.md](spec.md), [plan.md](plan.md), [research.md](research.md), [data-model.md](data-model.md), [record-scope contract](contracts/record-scope.md)

Status: executable foundation tasks; later phases are required work packages to refine after reference verification. This is not a completed full-feature task breakdown. No user story is delivered by the foundation alone.

## Phase 1: Setup

- [x] T001 Record confirmed parity scope and independent foundation boundaries in `specs/015-scoped-permissions/plan.md` and `contracts/record-scope.md`.
- [x] T002 Analyze foundation consistency and report unresolved full-feature gates against `specs/015-scoped-permissions/spec.md`, `plan.md` and `tasks.md` before code changes.

## Phase 2: Independent foundation (FR-006)

- [x] T003 Write record-scope truth-table, isolation, missing-ID, assignment-removal and union-law tests in `crates/core/src/permissions/tests.rs`; expose the module in `crates/core/src/lib.rs` and observe failure before implementation.
- [x] T004 Implement allocation-free scope union and coverage in `crates/core/src/permissions.rs`, without changing existing role types, schema or runtime guards.
- [x] T005 Run focused/core tests, core Clippy and formatting; record results and limitations in `specs/015-scoped-permissions/quickstart.md`.

## Phase 3: Full-policy gate (blocks all subsequent runtime work)

### Confirmed catalog implementation (independent of runtime cutover)

The user requested implementation without paying for or modifying Harvest on
2026-10-02. Execute this confirmed pure-model increment while unresolved operation
and migration cases remain isolated, not waived. It does not complete T006–T020.

- [x] T027 Add failing closed-catalog, profile-default, prerequisite/floor, removal and unknown-wire-value tests in `crates/core/src/permissions/catalog/tests.rs` (FR-001/003/004/015; `contracts/grant-catalog.md`).
- [x] T028 Implement the typed catalog, six profiles and normalized editable selection in `crates/core/src/permissions/catalog.rs`; expose it from `permissions.rs`, without changing legacy roles or runtime guards.
- [x] T029 Verify focused and full core tests, Clippy and formatting; adversarially review grant escalation/dependency loss and record evidence in `quickstart.md` and `progress.md`.

T027 → T028 → T029 builds on T005. It supplies pure grant logic for T010/T011/T016,
but persistence, administrative identity, scope/field enforcement and migration
still need their own tests and cannot be inferred from a selected grant set.

Inventory evidence is in `contracts/current-access.md`; browser limitations and unresolved reference cases are in `contracts/harvest-evidence.md`. Neither T006 nor T007 is complete until the remaining verification/migration review is done.

- [ ] T006 Complete operation-level parity matrix and custom prerequisite/approval contracts in `specs/015-scoped-permissions/contracts/`, with documented/observed/conflicting/unverified evidence in `research.md` (FR-002/003/005/009/015/019/020).
- [ ] T007 Inventory role checks across `crates/horae/src/` and legacy core transitions; document migration grant/revocation differences and obtain review in `specs/015-scoped-permissions/contracts/migration.md` (FR-014/017).
- [ ] T008 Amend the three-role constraint through `.specify/memory/constitution.md` governance and reconcile dependent specs; finalize persistence/revocation design in `specs/015-scoped-permissions/data-model.md`.
- [ ] T009 Refine remaining work packages into executable file-level tasks in `specs/015-scoped-permissions/tasks.md`, complete requirement checks and repeat analysis before replacing runtime authorization.

## Phase 4: US1 — Six profiles and safe assignments (P1)

Independent test: all six profiles allow/deny correctly; concurrent demotions preserve an active administrator.

- [ ] T010 [US1] Add failing profile, stale-edit and concurrent administrator tests in `crates/horae/tests/integration.rs` and pure grant tests under `crates/core/src/permissions/` (FR-001/003/010/011).
- [ ] T011 [US1] Implement verified grants, revisioned persistence and atomic assignment mutations in `crates/core/src/permissions.rs`, `crates/horae/migrations/`, `crates/horae/src/models/` and `crates/horae/src/server_fns/users.rs`; refresh `.sqlx/` (FR-001/010/011/013).

## Phase 5: US2 — Managed work and scoped approvals (P1)

Independent test: two projects/two approvers with overlapping people scope, filtered dates, empty cells and withdrawal; no unrelated changes.

- [ ] T012 [US2] Add management-assignment and scoped approval/withdrawal/lock concurrency tests in `crates/horae/tests/integration.rs` (FR-005/006/009/019).
- [ ] T013 [US2] Implement verified assignment storage, date/project approval coverage and transitions in `crates/horae/migrations/`, `crates/horae/src/server_fns/approvals.rs`, relevant project/person server functions and `crates/core/src/state.rs`; refresh `.sqlx/` (FR-005/006/009/017/019).

## Phase 6: US3 — Enforcement on every delivery path (P1)

Independent test: replay forbidden direct reads/writes and downloads; revoke between preview/execution/download; inspect returned payloads.

- [ ] T014 [US3] Add cross-surface negative payload, aggregation and revocation tests in `crates/horae/tests/integration.rs` and surface-specific test modules (FR-007/008/010/018).
- [ ] T015 [US3] Integrate trusted permission loading and enforcement across the completed entry-point inventory in `crates/horae/src/server_fns/`, `reports.rs`, `harvest/`, jobs, CLI and plugin hosts; activate only after all paths are covered (FR-007/008/010/017/018).

## Phase 7: US4 — Custom profiles and permission explanations (P2)

Independent test: template creation/application/deletion and person-specific adjustments; deletion-preservation expectation remains gated on resolving [C01](contracts/current-account-investigation.md) in T006; both screens explain identical effective access. Do not implement a destructive alternative from source warnings.

- [ ] T016 [US4] Add custom dependency, unknown-grant, template lifecycle and audit tests in `crates/core/src/permissions/` and `crates/horae/tests/integration.rs` (FR-004/011/013/015).
- [ ] T017 [US4] Implement verified custom-template lifecycle and audit in `crates/horae/src/server_fns/`, `models/` and migrations; refresh `.sqlx/` (FR-004/013/015).
- [ ] T018 [US4] Align permission controls/descriptions in `crates/horae/src/pages/` Settings and Workspace using existing components and `design/project/app/08_Settings.dc.html` / `09_Workspace.dc.html`; apply design skills and browser viewport/theme/keyboard checks (FR-012/016).

## Phase 8: US5 — Data-preserving transition (P1)

Independent test: populated migration fixture, reviewed access differences, safe retry and no import-driven privilege overwrite.

- [ ] T019 [US5] Add migration, import and identity-linking fixtures in `crates/horae/tests/integration.rs`; implement only the reviewed migration in `crates/horae/migrations/` and import/auth consumers (FR-014/017).

## Phase 9: Acceptance

- [ ] T020 Verify every matrix row, regression surface and migration case; record evidence in `specs/015-scoped-permissions/quickstart.md`, complete all requirement checks and run full Nix gates before requesting merge (FR-018/020, SC-001–009).

## Dependencies and parallelism

### Independent repair of an existing invariant

- [x] T021 Add two-organization approval/reopen tests against pool-injected production transactions in `crates/horae/src/server_fns/approvals/isolation_tests.rs` and observe the tenant-isolation failures.
- [x] T022 Enforce organization scope in approval/reopen SQL in `crates/horae/src/server_fns/approvals.rs`, preserving same-organization weekly behavior and post-commit events.
- [x] T023 Run approval regression tests, regenerate `.sqlx/`, run server Clippy/formatting and record evidence in `specs/015-scoped-permissions/quickstart.md`.

T021–T023 repair an existing tenant boundary without introducing new policy; they can run while the reference/migration gate is open. They do not complete US2 or the new approval contract.

- [x] T024 Extract the legacy assignment mutations into pool-injected helpers and reproduce cross-organization creation/removal and stale-administrator failures in `crates/horae/src/server_fns/projects/assignment_tests.rs`.
- [x] T025 Constrain both assignment endpoints to same-organization resources and revalidate active administrator authority under a transaction lock in `crates/horae/src/server_fns/projects.rs`, preserving post-commit events and same-org behavior.
- [x] T026 Run assignment/project regressions, regenerate `.sqlx/`, run server Clippy/formatting and record limits in `specs/015-scoped-permissions/quickstart.md`.

T024–T026 close the existing assignment boundary identified in `contracts/current-access.md`. They neither introduce new assignment authority nor activate any part of the six-profile policy. Profile, migration and approval gates remain mandatory.

T001 → T002 → T003 (RED) → T004 (GREEN) → T005. T006–T009 are mandatory before T010–T020 and must not be marked complete using foundation-only tests. Each story's tests precede its implementation. US3 depends on the permission/assignment model; UI depends on shared effective grants; cutover requires every delivery path and migration acceptance. No parallel code tasks are designated because the shared model and integration fixture are overlapping. Evidence gathering may run independently; no independent feature acceptance is implied.
