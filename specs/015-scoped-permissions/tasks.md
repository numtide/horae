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

### Current administrator mutation boundary (FR-010/011)

- [x] T030 Pass authenticated actor IDs into user role/activation/create transaction helpers in `crates/horae/src/server_fns/users.rs`; add failing stale/revoked/foreign actor and concurrent-revocation tests in `users/tests/authority.rs`, retaining existing last-admin tests.
- [x] T031 Reauthorize and lock the active same-organization administrator inside the access-change transaction, after the organization lock; keep user creation and post-commit events on the same boundary.
- [x] T032 Run real PostgreSQL user/concurrency and affected regressions, regenerate the complete SQLx cache, check server/core builds and formatting, and record evidence.

These tasks close a current access-change race required by the eventual six-profile
mutations. They do not replace T010/T011's profile persistence or revision/audit tests.

### Persistence protocol planning (not policy activation)

The renewed implementation request permits a strict pure loading boundary before
the database work below. This reuses the confirmed catalog and rejects malformed
saved grants without choosing schema, profile provenance or unresolved policy.

- [x] T047 Add failing strict-restoration tests in `crates/core/src/permissions/catalog/stored_tests.rs`: all six profiles, custom selections, unsupported catalog versions, missing floor/prerequisites, duplicates and unchanged saved grants (FR-001/011/015).
- [x] T048 Add versioned, fallible restoration to `PermissionSelection` in `crates/core/src/permissions/catalog.rs`; never normalize stored grants, infer administrative identity or activate runtime consumers.
- [x] T049 Run focused/full core tests, Clippy, formatting and a mutation check; record limits in `quickstart.md` and `progress.md`.

T029 → T047 → T048 → T049 is the pure portion of T035's loading validation.
It does not complete T035/T036: tenant constraints, trusted database loading,
explicit administrative identity and schema remain gated on T006–T009.

- [x] T033 Define proposed stored entities, tenant constraints, explicit administrative identity, revisions/receipts/audit and cross-surface lock ordering in `specs/015-scoped-permissions/data-model.md` and `contracts/permission-state.md` (FR-005/007/010/011/013/014/017).
- [x] T034 Adversarially review the proposal against current writers, record findings/gates in `specs/015-scoped-permissions/research.md` and `progress.md`, and refine dependent tests without marking T006–T009 complete.
- [ ] T042 Complete the cross-command resource hierarchy and trigger/FK lock inventory in `specs/015-scoped-permissions/contracts/permission-state.md` from the finalized operation matrix before closing T009 or executing T039/T040.

Partial T042 evidence now identifies the current writer orders, parent-writing
triggers/cascades, READ ONLY incompatibility and network waits in inline imports.
The follow-up source inventory covers credential/identity/job-maintenance paths
and identifies the legacy-report converter's job-to-organization FK inversion.
Validate the corrected candidate hierarchy and maintenance exceptions; do not
mark it complete from source inspection alone.

Inventory evidence is in `contracts/current-access.md`; browser limitations and unresolved reference cases are in `contracts/harvest-evidence.md`. Neither T006 nor T007 is complete until the remaining verification/migration review is done.

- [ ] T006 Complete operation-level parity matrix and custom prerequisite/approval contracts in `specs/015-scoped-permissions/contracts/`, with documented/observed/conflicting/unverified evidence in `research.md` (FR-002/003/005/009/015/019/020).

`contracts/operation-matrix.md` covers the 80 public async server-function symbols
inspected at `b7e730c` and additional delivery surfaces. Its C/U cells still need
resolved predicates and executable allowed/denied cases; coverage alone does not
complete T006.

- [ ] T007 Inventory role checks across `crates/horae/src/` and legacy core transitions; document migration grant/revocation differences and obtain review in `specs/015-scoped-permissions/contracts/migration.md` (FR-014/017).
- [ ] T008 Amend the three-role constraint through `.specify/memory/constitution.md` governance and reconcile dependent specs; finalize persistence/revocation design in `specs/015-scoped-permissions/data-model.md`.

Partial cross-feature reconciliation and test ownership are recorded in
`contracts/dependent-spec-reconciliation.md`. Read the listed branch revisions
before integration; the older Project Detail copy in this branch is not its
latest acceptance contract. The shared editor has a conditional FR-022 transition
note, but neither this nor the existing constitution amendment completes T008.

- [ ] T009 Refine remaining work packages into executable file-level tasks in `specs/015-scoped-permissions/tasks.md`, complete requirement checks and repeat analysis before replacing runtime authorization.

T006/T009 must finish OP47 self-assignment under FR-027's approved
Administrator-only writer boundary, separately from FR-026 project delegation, and bind OP48 project duplication/deletion to
its owning domain requirements. Feature 010's archive-only increment does not
remove those operations from full web parity. T012/T013 cover relationship
authority and revisions; T014/T015 cover source projection, dependent resources,
revocation and atomic denial. Do not infer destructive grants or copy legacy
API access-role side effects into canonical permissions.

## Phase 4: US1 — Six profiles and safe assignments (P1)

Independent test: all six profiles allow/deny correctly; concurrent demotions preserve an active administrator.

- [ ] T010 [US1] Add failing profile, stale-edit and concurrent administrator tests in `crates/horae/tests/integration.rs` and pure grant tests under `crates/core/src/permissions/` (FR-001/003/010/011).
- [ ] T011 [US1] Implement verified grants, revisioned persistence and atomic assignment mutations in `crates/core/src/permissions.rs`, `crates/horae/migrations/`, `crates/horae/src/models/` and `crates/horae/src/server_fns/users.rs`; refresh `.sqlx/` (FR-001/010/011/013).

T010/T011 are acceptance work packages refined below. Schema/runtime steps remain
gated on T006–T009; the proposal's existence does not make them executable yet.

- [ ] T035 [US1] Add failing persisted-state tests in `crates/horae/src/server_fns/permissions/tests/storage.rs`: tenant constraints, six profile selections, explicit Administrator identity, unknown grants and no silent normalization on load (FR-001/006/011/015).
- [ ] T036 [US1] Add reviewed additive schema in `crates/horae/migrations/0042_scoped_permission_state.sql`, DTOs in `crates/horae/src/models/permissions.rs` and trusted storage helpers in `crates/horae/src/server_fns/permissions.rs`; keep legacy mode unchanged with no automatic mapping (FR-001/010/014/017). Recheck migration numbering against the implementation base before creating the file.
- [ ] T037 [US1] Add failing command/replay/concurrency tests in `crates/horae/src/server_fns/permissions/tests/changes.rs`: stale revisions, request identity, revoke-vs-write in both orders, last-admin races and state/audit rollback (FR-010/011/013).
- [ ] T038 [US1] Implement the reviewed typed access-change transaction and authorized outcome lookup in `crates/horae/src/server_fns/permissions/changes.rs`, authenticated wrappers in `crates/horae/src/server_fns/users.rs`, and regenerate `.sqlx/`; do not expose a partially enforced policy (FR-010/011/013/017).

## Phase 5: US2 — Managed work and scoped approvals (P1)

Independent test: two projects/two approvers with overlapping people scope, filtered dates, empty cells and withdrawal; no unrelated changes.

- [ ] T012 [US2] Add management-assignment and scoped approval/withdrawal/lock concurrency tests in `crates/horae/tests/integration.rs`. Cover every FR-024 case in `contracts/approval-visibility.md`: readable/unreadable time and expenses, missing approval authority, truly expense-free selections, revocation/new-record races, non-disclosing errors, direct requests and atomic full-selection effects. Combined acceptance requires feature 016 expense fixtures, not mocks alone (FR-005/006/009/010/019/024).

- [ ] T013 [US2] Implement verified assignment storage, date/project approval coverage and transitions in `crates/horae/migrations/`, `crates/horae/src/server_fns/approvals.rs`, relevant project/person server functions and `crates/core/src/state.rs`; refresh `.sqlx/`. Enforce FR-024's complete-set visibility and atomic denial under the reviewed transaction protocol, without silent filtering, implicit grants or extrapolating withdrawal semantics. Apply FR-025's existing-designation retention/read-loss rules atomically with permission revision and audit (FR-005/006/009/010/013/017/019/024/025).

T012 also covers FR-025's acceptance table in
`contracts/current-account-investigation.md`: absent/managed/all project reads,
editing removed or retained, confirmed removal versus cancel, direct-request
bypass, stale-preview/revocation races, preserved membership/history and
independent scope. T013 must pass these cases without automatic promotion or
restoring grants. T012/T013 also cover FR-026 project-editor delegation:
managed/all actor authority, read-only denial, compatible target grants evaluated
with the proposed project scope, no automatic profile changes, removal without
requiring target grants, same-organization active identities, atomic multi-person
saves, revocation and negative permission payloads. Target grant changes remain
Administrator-only. Project-creation remains separate. T012/T013 also enforce
FR-027 for person-management add/remove/replace: Administrator success with valid
subjects, non-admin PeopleWriteAll/People Admin/Executive Manager/custom denial,
own managed-set attempts, cross-org IDs, revocation/stale revisions, atomic batches
and audit. Preserve profiles, global grants, project membership and history.
T016–T018 distinguish assignment controls from independently authorized ordinary
person editing and own-access explanations. FR-028 now requires existing compatible
grants for newly added person-management relationships. T012/T013 cover each
compatible grant family, own/unrelated-only denial, proposed scope without prior
assignments, revoked eligibility before commit, mixed valid/invalid replacement
additions, and unchanged global grants/history. Include read-only compatible
grants without requiring people-directory/edit access. FR-029 adds retention with
any compatible grant and confirmed atomic removal on last-compatible-grant loss.
T012/T013 cover cancellation, missing/stale confirmation, revoked Administrator,
concurrent relationship changes, audit/write rollback, unchanged incoming
relationships and history, and no automatic reassignment after grants return.
Project-manager designations remain unchanged unless FR-025 independently requires
removal; test combined FR-025/029 losses with confirmation of both affected sets
and all-or-nothing commit.
Self-assignment remains a separate unresolved predicate, not waived by eligibility.

- [ ] T043 [US2] Finalize the calendar/execution and correction cases in `specs/015-scoped-permissions/contracts/company-locks.md`; add failing injected-clock boundary tests in `crates/core/src/permissions/company_lock/tests.rs` for the finalized modes, timezone/week-start/month-end/DST rules and nondecreasing automatic cutoff (FR-019/023). T006–T009 gate code; this is not an independent permission foundation.

- [ ] T044 [US2] Add failing production-transaction tests in `crates/horae/src/server_fns/approvals/company_lock_tests.rs` for configuration revocation, worker/manual races, replay, stopped timers, submission without notifications, separate approval/invoice locks and privileged correction. Include feature 016 expense fixtures before combined acceptance (FR-007/010/013/019/023).

- [ ] T045 [US2] Implement the reviewed shared calculation in `crates/core/src/permissions/company_lock.rs`, persisted organization configuration/cutoff under `crates/horae/migrations/`, authorized commands under `crates/horae/src/server_fns/approvals/company_lock.rs` and due execution from `crates/horae/src/scheduler.rs`; reuse the existing transaction/audit/job infrastructure and regenerate `.sqlx/` (FR-019/023).

- [ ] T046 [US2] Expose the same next-run/cutoff calculation and real settings/actions in `crates/horae/src/pages/approvals.rs`, `crates/horae/src/pages/settings.rs` and shared models; add browser checks under `crates/horae/tests/browser/` for authorized/forbidden actions, disable/clear/cancel states and errors using existing design tokens (FR-012/016/019/023).

T043 → T044 → T045 → T046 refines T012/T013. No new scheduler is authorized
before the existing policy/migration gates. T045 depends on the final T042
hierarchy and audited transactions; combined expense behavior depends on 016.

## Phase 6: US3 — Enforcement on every delivery path (P1)

Independent test: replay forbidden direct reads/writes and downloads; revoke between preview/execution/download; inspect returned payloads.

- [ ] T014 [US3] Add cross-surface negative payload, aggregation and revocation tests in `crates/horae/tests/integration.rs` and surface-specific test modules. Include approved C02: report-only grants allow defined financial report fields and matching exports, but not ordinary rate reads/history/edits or unrelated reports; rate-only grants do not authorize financial reports. Cover neither grant, out-of-scope/foreign records, forged report-family selection and revocation before generation/download (FR-007/008/010/018).

- [ ] T015 [US3] Integrate trusted permission loading and enforcement across the completed entry-point inventory in `crates/horae/src/server_fns/`, `reports.rs`, `harvest/`, jobs, CLI and plugin hosts; activate only after all paths are covered (FR-007/008/010/017/018).

T014/T015 also cover approved C03 (FR-021): cross managed/unmanaged people with
managed/unmanaged projects for general person rates and project/person/task
overrides; require the corresponding read/write financial grant. Test global
task defaults, inherited-rate display versus unrelated history, read-only
mutation denial, removal of either grant or relationship, foreign organizations,
and person-default propagation without changing project overrides. Use production
paths when integrating; passing pure selection tests does not establish enforcement.

For C04/FR-022, T014/T015 also cover built-in and custom cost read/write grants,
write-implies-read dependencies, ordinary cost fields/history and supported
project overrides, read-only mutation denial, revocation and foreign organizations.
Preserve independent resource authority and distinguish report-only disclosure
from ordinary cost access; neither profile names nor billable grants authorize costs.

- [ ] T039 [US3] Add cross-command lock-order/revocation tests in `crates/horae/tests/integration.rs` and affected `server_fns/` test modules for project creation/editing, membership/management, activation and settings; include organization-row updates, trigger/FK lock paths, snapshot readers waiting across revocation, whole-transaction retry, bounded import commits, credential writes, report conversion and caller-aware download pages (FR-005/007/010/018).

- [ ] T040 [US3] Apply T042's hierarchy by reconciling organization-first locking in `crates/horae/src/server_fns/project_creation/`, `server_fns/projects.rs`, `server_fns/users.rs`, `cli.rs` and all remaining access-affecting writers in the final inventory; no shared-to-exclusive upgrade or fabricated operator user (FR-007/010/017/018).

T039/T040 refine T014/T015's serialization prerequisite, not all report/export/job/
plugin authorization. Exercise production transactions rather than duplicate SQL.

## Phase 7: US4 — Custom profiles and permission explanations (P2)

Independent test: template creation/application/deletion and person-specific adjustments; C01 is resolved by the 2026-10-02 user decision: deleting a template preserves all assignees' effective grants/scope as person-specific configurations, including individually adjusted assignees. Test confirmation/cancellation, unavailable deleted templates and separate explicit revocation; both screens explain identical effective access. Remaining T006 predicates are still open.

- [ ] T016 [US4] Add custom dependency, unknown-grant, template lifecycle and audit tests in `crates/core/src/permissions/` and `crates/horae/tests/integration.rs` (FR-004/011/013/015).
- [ ] T017 [US4] Implement verified custom-template lifecycle and audit in `crates/horae/src/server_fns/`, `models/` and migrations; refresh `.sqlx/` (FR-004/013/015).
- [ ] T041 [US4] Add and pass audit/receipt disclosure tests in `crates/horae/src/server_fns/permissions/tests/audit.rs`: current Administrator reads, revoked/non-admin/foreign denial, distinct operator attribution, sanitized outcomes and no false success audit after rollback (FR-011/013).
- [ ] T018 [US4] Align permission controls/descriptions in `crates/horae/src/pages/` Settings and Workspace using existing components and `design/project/app/08_Settings.dc.html` / `09_Workspace.dc.html`; apply design skills and browser viewport/theme/keyboard checks (FR-012/016).

## Phase 8: US5 — Data-preserving transition (P1)

Independent test: populated migration fixture, reviewed access differences, safe retry and no import-driven privilege overwrite.

- [ ] T019 [US5] Add migration, import and identity-linking fixtures in `crates/horae/tests/integration.rs`; implement only the reviewed migration in `crates/horae/migrations/` and import/auth consumers (FR-014/017).

T007/T019 include migration cases M01–M08 in `contracts/migration.md`: parent
tenancy without child rows, cascading cost/budget/task membership preservation,
NULL/zero rate resolution, distinct role namespaces, SQL-view consumers,
revision exhaustion/stale editors, historical approval attribution and unknown
job requesters versus tenant-safe artifacts. Compare identities and values, not
just counts. Fixtures must use isolated databases; preflight is read-only and no
data repair, role mapping or runtime activation is authorized by these cases.

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

Persistence refinement: T033 → T034 informs T008/T009. After T006–T009 pass,
T035 → T036 → T037 → T038. T042 informs T009; T042 → T039 → T040.
T041 follows audited mutations before
exposing audit reads. File-level subtasks do not replace story acceptance gates.
