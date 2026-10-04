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

## Phase 3: Independent increments and full-policy integration gate

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
- [x] T031 Reauthorize and lock the active same-organization administrator inside the access-change transaction in `crates/horae/src/server_fns/users.rs`, after the organization lock; keep user creation and post-commit events on the same boundary.
- [x] T032 Run real PostgreSQL user/concurrency and affected regressions, regenerate `.sqlx/`, check server/core builds and formatting, and record evidence in `specs/015-scoped-permissions/quickstart.md`.

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
explicit administrative identity and schema require the storage-specific design
review described under T035/T036; no database work is unlocked by these pure tests.

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

T006/T009 must bind OP48 project duplication/deletion to its owning domain
requirements. OP47 now follows FR-027/028/029/031 for writer authority, eligibility,
retention and self-relationship rejection, separately from FR-026 project
delegation. Feature 010's archive-only increment does not remove those operations
from full web parity. T012/T013 cover relationship
authority and revisions; T014/T015 cover source projection, dependent resources,
revocation and atomic denial. Do not infer destructive grants or copy legacy
API access-role side effects into canonical permissions.

## Phase 4: US1 — Six profiles and safe assignments (P1)

### Authenticated editor integration

T125 → T126 → T127 → T128 → T129 follows `contracts/permission-editor.md`.
Sequential shared-file work; backend acceptance does not close UI or activation.

- [x] T126 [US1] Add failing actor/target/survivor activity and inherited-settings tests in `crates/horae/src/server_fns/permissions/tests/profiles.rs` and `tests/templates.rs`; protect these decisions through completion in `permissions/profiles.rs` and `permissions/templates.rs` (FR-010/011).
- [x] T127 [US1] Add editor tests in `crates/horae/src/server_fns/permissions/tests/editor.rs`; share preview/save effects in `permissions/profiles.rs`, add DTOs in `models/permission_editor.rs` and reads in `permissions/editor.rs` (FR-004/010/011/015/025/029/030).
- [x] T128 [US1] Expose session-derived editor/profile/template functions in `crates/horae/src/server_fns/permission_editor.rs`, with HTTP tests in `server_fns/importers/authorization_tests/permission_editor.rs`; preserve replay/error contracts and update module/operation wiring (FR-004/010/011/013/018/032).
- [x] T129 [US1] Verify command/HTTP/regression suites, complete `.sqlx/`, offline server/WASM lint and formatting, adversarial review and scoped analysis; record evidence in feature 015 `quickstart.md` and `progress.md` before unsigned publication to draft #212 (FR-018).

Requester-bound recovery prerequisite (FR-010/011/013/018):

- [x] T130 [US1] Reproduce cross-login saves through registered HTTP functions in `server_fns/importers/authorization_tests/permission_editor.rs`; cover person/template commands, same/foreign organization, omitted binding and unchanged state on denial.
- [x] T131 [US1] Return the session-derived requester from the authorized editor read; require that pair on both save transports in `server_fns/permission_editor.rs` and retain it in existing person/template UI calls. Preserve internal commands, audit/receipt shapes and runtime policy.
- [x] T132 [US1] Verify registered HTTP and UI regressions, offline native/WASM lint, formatting and requester/replay source review. Record evidence and browser-recovery limits in `quickstart.md` and `progress.md`; no activation or migration.

T129 → T130 → T131 → T132 is sequential transport/consumer work supporting
T018's durable reload recovery, not a replacement for that remaining acceptance.

Independent test: all six profiles allow/deny correctly; concurrent demotions preserve an active administrator.

- [ ] T010 [US1] Add failing profile, stale-edit and concurrent administrator tests in `crates/horae/tests/integration.rs` and pure grant tests under `crates/core/src/permissions/` (FR-001/003/010/011).
- [ ] T011 [US1] Implement verified grants, revisioned persistence and atomic assignment mutations in `crates/core/src/permissions.rs`, `crates/horae/migrations/`, `crates/horae/src/models/` and `crates/horae/src/server_fns/users.rs`; refresh `.sqlx/` (FR-001/010/011/013).

T010/T011 are acceptance work packages refined below. Runtime activation remains
gated on T006–T009. T035/T036 now follow the reviewed non-activating
`contracts/permission-storage.md`: FR-032 closes creation-name equivalence;
canonical grants, independent administrative identity and provenance are stored
without treating presentation as authority. Use disposable PostgreSQL only. T037/T038
also require resolved command predicates and T042's transaction hierarchy. A
proposal's existence is not approval to invent those decisions or run migrations.

- [x] T035 [US1] Add failing persisted-state tests in `crates/horae/src/server_fns/permissions/tests/storage.rs`: tenant constraints, six profile selections, independent Administrator identity/provenance, unknown grants, no silent normalization, legacy preservation and FR-032 name conflicts/bounds (FR-001/006/011/014/015/032).
- [x] T036 [US1] Add reviewed additive schema in `crates/horae/migrations/0042_scoped_permission_state.sql`, server-only typed read models in `crates/horae/src/models/permissions.rs` and trusted storage helpers in `crates/horae/src/server_fns/permissions.rs`; keep legacy mode unchanged with no automatic mapping (FR-001/010/014/017/032). Recheck migration numbering against the implementation base before creating the file.
- [ ] T037 [US1] Add failing command/replay/concurrency tests in `crates/horae/src/server_fns/permissions/tests/changes.rs`: stale revisions, request identity, revoke-vs-write in both orders, last-admin races and state/audit rollback (FR-010/011/013).
- [ ] T038 [US1] Implement the reviewed typed access-change transaction and authorized outcome lookup in `crates/horae/src/server_fns/permissions/changes.rs`, authenticated wrappers in `crates/horae/src/server_fns/users.rs`, and regenerate `.sqlx/`; do not expose a partially enforced policy (FR-010/011/013/017).

### Internal person-profile transaction

The reviewed `contracts/person-profile-commands.md` closes the local identity,
inactive-target and FK/trigger questions without claiming full T042 closure.
T055 → T056 (RED) → T057 (GREEN) → T058 refines T037/T038 and the FR-025/029
portion of T012/T013. No parallel code tasks: schema, commands and fixtures overlap.
The shared request namespace also requires template commands to reject another
command kind as conflicting intent rather than trying to decode its shape.

- [x] T056 [US1] Add failing production-command tests in `crates/horae/src/server_fns/permissions/tests/profiles.rs` for every local acceptance case in `contracts/person-profile-commands.md`, including management-table tenant/self constraints (FR-004/010/011/013/015/025/029/030).
- [x] T057 [US1] Add isolated management relations in `crates/horae/migrations/0044_permission_management_assignments.sql` and internal audited profile changes in `crates/horae/src/server_fns/permissions/profiles.rs`; wire the module, preserve strict loads and shared receipt conflicts in `permissions/templates.rs`, with no activation/backfill or authenticated wrapper (FR-004/010/011/013/015/025/029/030).
- [x] T058 [US1] Verify focused and affected PostgreSQL regressions, regenerate `.sqlx/` without incremental compilation, run offline all-targets server Clippy and formatting, adversarially review command/rollback/concurrency behavior and record evidence in `specs/015-scoped-permissions/quickstart.md` and `progress.md`.

## Phase 5: US2 — Managed work and scoped approvals (P1)

### Internal project delegation

T058 → T059 → T060 → T061 implements the closed local contract in
`contracts/project-management-commands.md`. These sequential tasks share fixtures
and commands; no parallel code work. Person-management lifecycle, editor wiring,
full T042 and runtime activation remain separate gates.

- [x] T059 [US2] Add failing production-command tests in `crates/horae/src/server_fns/permissions/tests/project_management.rs` for the authority, eligibility, preservation, replay, rollback and concurrency cases in `contracts/project-management-commands.md` (FR-005/010/011/013/017/026).
- [x] T060 [US2] Implement internal atomic manager-set replacement in `crates/horae/src/server_fns/permissions/project_management.rs`, using existing canonical loaders, 0044 relations and receipts; prevent the legacy project-parent lock cycle without policy activation or membership changes (FR-005/010/011/013/017/026).
- [x] T061 [US2] Verify focused and affected PostgreSQL regressions, complete `.sqlx/` regeneration, offline all-targets server Clippy and formatting; adversarially review the implementation and record actual evidence in `specs/015-scoped-permissions/quickstart.md` and `progress.md`.

Activity-fence follow-up, reusing the same FR-010/026 contract and existing
administration transaction settings; no new grant or lifecycle rule:

- [x] T133 [US2] Reproduce concurrent actor/new-manager deactivation and inherited READ ONLY failure in `crates/horae/src/server_fns/permissions/tests/project_management.rs`; cover both race orders, retained inactive managers and cancellation rollback.
- [x] T134 [US2] Hold actor and added-manager activity with SHARE through receipt commit in `permissions/project_management.rs`, under the existing organization gate; reuse `configure_administration`, preserving the project NOWAIT rollback and all eligibility/replay rules.
- [x] T135 [US2] Verify project/profile/template command regressions, SQLx cache, offline server lint and formatting; adversarially review lock ordering, retained-manager behavior and rollback, recording evidence in `quickstart.md` and `progress.md`.

Independent test: two projects/two approvers with overlapping people scope, filtered dates, empty cells and withdrawal; no unrelated changes.

### Completed increment: pure relationship prerequisites

The user authorized removing unnecessary planning dependencies on 2026-10-03.
Local readiness is in `checklists/person-management-validation.md`; its closed
contract is `contracts/person-management-validation.md`. This is the pure portion
of T012/T013, not the complete story or a new assignment mutation endpoint.
Preserve existing task IDs and completion history; new IDs extend the sequence.

- [x] T050 [US2] Add failing exhaustive compatibility and self-link tests in `crates/core/src/permissions/person_management/tests.rs`, following the independent acceptance table in `specs/015-scoped-permissions/contracts/person-management-validation.md`; cover all catalog grants, mixed sets, last-grant loss, self-link batches and immutable inputs (FR-028/029/031).
- [x] T051 [US2] Implement the pure compatible-grant predicate and fallible proposed-set self-link check in `crates/core/src/permissions/person_management.rs`, exposed from `crates/core/src/permissions.rs`; reuse `PermissionSelection`, add no runtime consumers or I/O and keep writer authority separate (FR-027/028/029/031).
- [x] T052 [US2] Run focused and full core tests, core Clippy and formatting; adversarially review against `specs/015-scoped-permissions/contracts/person-management-validation.md` and record actual results/limits in `specs/015-scoped-permissions/quickstart.md` and `progress.md`. Confirm no server guard, schema or account data changed.

T049 → T050 (RED) → T051 (GREEN) → T052 completed without T006–T009.
No full-story marker or runtime acceptance follows from it. The transactional
work below retains its command/storage/concurrency dependencies.

### Remaining transaction and approval work

The [coverage/lifecycle boundary](contracts/approval-coverage.md) maps production
symbols, AC01–AC06 fixtures, schema-affecting reference discriminators and the
actual feature 016 dependency. Resolve its local coverage gates before T013;
the expense worktree currently supplies reference evidence, not runtime fixtures.
T012 must exercise actual production transactions, not a second SQL-only model.

Closed FR-024 record guard, independent of the unresolved lifecycle/activation
contracts: see `contracts/approval-visibility.md`.

- [x] T136 [US2] Add failing scope/visibility/authority and tenant/provenance tests in `crates/core/src/permissions/approvals/tests.rs` for the complete supplied time/expense set (FR-006/024).

- [x] T137 [US2] Implement the pure borrowed-input guard in `crates/core/src/permissions/approvals.rs`, reusing catalog/scopes without runtime integration, writes or new dependencies (FR-006/024).

- [x] T138 [US2] Verify focused/full core tests, core Clippy, formatting and adversarial scope/empty-input review; record actual evidence and remaining transactional acceptance in `quickstart.md` and `progress.md`.

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
FR-031 adds T012/T013 cases for direct and mixed-batch self-link rejection, even
with Administrator authority and compatible grants; unchanged own/all access;
and valid changes to the Administrator's own set of other people. Reject the
complete invalid batch without silently filtering it or recording a success audit.
This does not decide self-approval or authorize historical data cleanup.

FR-030's [keep-project-access cases](contracts/keep-project-access.md) extend
T010/T011 permission transactions, T012/T013 relationship tests and T016/T018
editor tests. Cover unchecked removal, explicit read/write opt-in, read-only
retention without opt-in, combined person losses, cancellation, failed/stale
previews, revocation, rollback and no automatic reassignment. Reuse the shared
editor and existing grant/revision/audit mechanics; no new grant or bypass.

- [ ] T043 [US2] Finalize the calendar/execution and correction cases in `specs/015-scoped-permissions/contracts/company-locks.md`; add failing injected-clock boundary tests in `crates/core/src/permissions/company_lock/tests.rs` for the finalized modes, timezone/week-start/month-end/DST rules and nondecreasing automatic cutoff (FR-019/023). Pure calculation code needs this local contract closed first; unrelated full-matrix rows are not prerequisites. T044–T046 remain gated on storage/command review and T042 before integration.

- [ ] T044 [US2] Add failing production-transaction tests in `crates/horae/src/server_fns/approvals/company_lock_tests.rs` for configuration revocation, worker/manual races, replay, stopped timers, submission without notifications, separate approval/invoice locks and privileged correction. Include feature 016 expense fixtures before combined acceptance (FR-007/010/013/019/023).

- [ ] T045 [US2] Implement the reviewed shared calculation in `crates/core/src/permissions/company_lock.rs`, persisted organization configuration/cutoff under `crates/horae/migrations/`, authorized commands under `crates/horae/src/server_fns/approvals/company_lock.rs` and due execution from `crates/horae/src/scheduler.rs`; reuse the existing transaction/audit/job infrastructure and regenerate `.sqlx/` (FR-019/023).

- [ ] T046 [US2] Expose the same next-run/cutoff calculation and real settings/actions in `crates/horae/src/pages/approvals.rs`, `crates/horae/src/pages/settings.rs` and shared models; add browser checks under `crates/horae/tests/browser/` for authorized/forbidden actions, disable/clear/cancel states and errors using existing design tokens (FR-012/016/019/023).

T043 → T044 → T045 → T046 refines T012/T013. No new scheduler is authorized
before the existing policy/migration gates. T045 depends on the final T042
hierarchy and audited transactions; combined expense behavior depends on 016.

## Phase 6: US3 — Enforcement on every delivery path (P1)

### Budget email preparation authority

T113 → T114 → T115 → T116 follows `contracts/budget-email-authority.md`.
Sequential shared-file work; preserve the existing recipient predicate without
claiming OP37 or full policy activation. Independent acceptance is preparation
under current eligibility/claim ownership with all locks released before transport.

- [x] T114 [US3] Add failing production-delivery concurrency tests in `crates/horae/src/notifications/tests/authority.rs`, wired from `notifications/tests.rs`, for revocation during organization/recipient/project waits and changed claims/leases (FR-006/007/010/018).
- [x] T115 [US3] Extract bounded current-authority preparation in `crates/horae/src/notifications.rs` or its `notifications/preparation.rs` submodule; preserve trusted claim payloads, recipient predicate, sanitized terminal outcomes, transport and acknowledgement outside locks (FR-006/007/010/017).
- [x] T116 [US3] Extend `crates/horae/src/notifications/tests/authority.rs` for current payload, inherited settings, cancellation, exact claim fencing and size-one-pool/blocked transport; run notification/outbox/full server regressions, regenerate complete `.sqlx/`, verify offline lint/WASM/format, adversarial review and scoped analysis; record actual evidence in feature 015 `quickstart.md` and `progress.md` (FR-017/018, SC-006 subset).

### Interrupted import session disposal

T089 → T090 → T091 implements the closed cleanup contract in
`contracts/permission-state.md`, a T042 worker prerequisite (FR-007/010,
FR-017/018 preservation and SC-006 import regression). Shared source and
fixtures require sequential execution; no new execution policy is selected.

- [x] T089 [US3] Reproduce pending missing-savepoint cleanup failure with real PostgreSQL fixtures in `crates/horae/src/importers/harvest/engine_tests.rs`, checking precommitted-data preservation, incomplete rollback and immediate one-connection retry.
- [x] T090 [US3] Drain interrupted rollback responses in shared `crates/horae/src/importers/harvest.rs::release_import`, recover only the defined savepoint condition, roll back before unlocking and preserve error propagation and connection disposal.
- [x] T091 [US3] Cover untracked transaction cleanup and nonrecoverable connection errors in `crates/horae/src/importers/harvest/engine_tests.rs`, run actual API/CSV cancellation and server regressions, regenerate `.sqlx/`, verify offline Clippy/formatting and record analysis/review in feature 015 artifacts.

### Original import requester provenance

T086 → T087 → T088 follows the independent storage contract in
`contracts/permission-state.md` (FR-007/010 prerequisite, FR-017/018 preservation
and verification). Shared files require sequential execution. No worker authority,
legacy backfill, retry delegation or full policy activation is implied.

- [x] T086 [US3] Add failing session-command requester assertions and duplicate/lifecycle checks in `crates/horae/src/server_fns/importers/commands/tests/requester.rs`; cover registered HTTP actor provenance in `crates/horae/src/server_fns/importers/authorization_tests.rs`.
- [x] T087 [US3] Add nullable tenant-bound requester storage in `crates/horae/migrations/0045_import_job_requester.sql`, pass trusted actors through `crates/horae/src/server_fns/importers/commands.rs` into `crates/horae/src/jobs.rs` inserts, and preserve historical fixtures and all conflict/retry paths.
- [x] T088 [US3] Add populated upgrade/FK/rollback preservation tests in `crates/horae/src/server_fns/importers/commands/tests/requester.rs` and extend the historical report preservation assertions in `crates/horae/src/jobs/report/tests.rs`, run importer and server regressions, regenerate `.sqlx/`, verify offline Clippy/formatting, and record analysis/review in feature 015 `quickstart.md`, `research.md` and `progress.md`.

### Durable CSV preparation outside authorization transactions

T083 → T084 → T085 implements the closed input-boundary contract in
`contracts/permission-state.md` (FR-007/010 transaction prerequisite, FR-017/018
preservation/verification). Execute sequentially; worker provenance, historical
jobs and full permission activation remain separately gated.

- [x] T083 [US3] Add a failing real-parser/SQL transaction observation in `crates/horae/src/importers/harvest/engine_tests/csv_streaming.rs`, covering partial first/subsequent durable batches and Commit/DryRun without sleeps or production hooks.
- [x] T084 [US3] Buffer only the next existing checkpoint batch before beginning SQL in `crates/horae/src/importers/harvest/csv_source/upload.rs`; preserve resumed offsets, single-connection exclusion, preview rollback, cancellation and the test-only unleased contract.
- [x] T085 [US3] Verify CSV recovery/cancellation/lease/report regressions and server tests; refresh `.sqlx/`, run offline all-targets Clippy/formatting and record scoped review/analysis in `specs/015-scoped-permissions/quickstart.md`, `research.md` and `progress.md`.

### Bounded import error downloads

T080 → T081 → T082 follows the closed download contract in
`contracts/permission-state.md` (FR-007/010/017/018). Shared files require
sequential execution; no full policy cutover is implied.

- [x] T080 [US3] Extract the production response builder and add failing preparation/stream revocation tests in `crates/horae/src/jobs/report/stream_tests.rs`, covering page and inline/empty-tail boundaries without changing snapshot or retention semantics.
- [x] T081 [US3] Share importer authorization in `crates/horae/src/jobs/access.rs` and wire `server_fns/importers/commands.rs` plus `jobs/report.rs` to short metadata/page/tail transactions; carry only session-trusted actor identity and abort denied bodies without private context.
- [x] T082 [US3] Verify both wait orders, reader-first locks, tenant isolation, single-connection/lazy bounded streaming and HTTP/CLI regressions in `jobs/report/stream_tests.rs` and `server_fns/importers/authorization_tests.rs`; run server tests, regenerate `.sqlx/`, run offline Clippy/formatting and record review/evidence in feature 015.

### Import job command and status authority

T077 → T078 → T079 follows the closed job-control contract in
`contracts/permission-state.md` (FR-007/010/017/018). Shared files require
sequential execution. This preserves current Administrator policy, not cutover.

- [x] T077 [US3] Extract production importer helpers and add failing authority tests in `crates/horae/src/server_fns/importers/commands/tests.rs`: API/CSV submission, cancel/retry, status/history, completed/waiting revocation, idempotent/no-op denial, foreign records, retained payloads, writer-first authority, rollback and single-connection pools.
- [x] T078 [US3] Wire `server_fns/importers.rs` to `server_fns/importers/commands.rs` with trusted actor IDs, organization SHARE/current actor SHARE under READ COMMITTED; refactor `crates/horae/src/jobs.rs` mutations to caller-owned transactions and reads to executor parameters, preserving queue semantics and returning status within the authorization transaction. Keep uploads outside locks and worker authority separate.
- [x] T079 [US3] Run focused command, importer HTTP/CLI and affected server regressions; regenerate complete `.sqlx/`, verify offline all-targets Clippy and formatting, review all production callers/lock/FK/error paths and record evidence in `specs/015-scoped-permissions/quickstart.md` and `progress.md`.

### Harvest connection transaction authority

T074 → T075 → T076 refines FR-007/010/017/018 under the connection contract in
`contracts/permission-state.md`. Sequential shared-file work; preserve current
Administrator policy and account identity/version/history protections.

- [x] T074 [US3] Add failing production-writer tests in `crates/horae/src/importers/harvest/credentials/authority_tests.rs` for connect/disconnect/change: inactive/non-admin/foreign/missing actors, revocation during organization/actor waits, write-first authority retention, rollback and no-op denial. Retain reservation, generation and history regressions.
- [x] T075 [US3] Pass trusted actor identity from `server_fns/importers.rs` and OAuth completion in `importers/harvest.rs` into `credentials.rs` and `account_switch.rs`; share a READ COMMITTED organization-first transaction check, retain existing locks/version checks, and map revocation to secret-free forbidden errors.
- [x] T076 [US3] Verify focused/full affected server tests, regenerate complete `.sqlx/`, run offline all-targets Clippy and formatting, review callers/lock ordering/error projections and record actual results in `specs/015-scoped-permissions/quickstart.md` and `progress.md`.

### Branding transaction authority

T071 → T072 → T073 refines FR-007/010/018 and the bounded branding contract in
`contracts/permission-state.md`. These tasks share files and run sequentially;
they preserve current role semantics rather than activating OP27's target policy.

- [x] T071 [US3] Add failing production-helper authority and lock-wait tests in `crates/horae/src/server_fns/organization/tests.rs`; cover denied changed/unchanged saves, tenant/activity/role checks, revocation after organization/actor waits, lock lifetime and rollback, retaining all field/idempotency regressions.
- [x] T072 [US3] Pass authenticated actor identity into `update_org_branding_record` in `crates/horae/src/server_fns/organization.rs`, use explicit READ COMMITTED and reload active same-org Manager/Admin after organization UPDATE with actor SHARE; preserve event dispatch and no-op behavior.
- [x] T073 [US3] Run focused/affected server tests, regenerate complete `.sqlx/`, verify offline all-targets Clippy and formatting, review caller/lock/error paths and record results in `specs/015-scoped-permissions/quickstart.md` and `progress.md`.

### Project-family lock integration

T068 → T069 → T070 refines the reviewed project-family boundary in
`contracts/permission-state.md`. These overlapping files run sequentially;
full T039/T040/T042 and policy activation remain open.

- [x] T068 [US3] Add failing production-path gate/revocation tests in `crates/horae/src/server_fns/project_creation/locking_tests.rs`, `projects/assignment_tests.rs` and `users/tests/authority.rs`; cover task callers, editor snapshots, invoice FK compatibility in `invoices/tests.rs` and time-entry/member cascades in `time_entries/update_tests.rs` (FR-007/010/017/018).
- [x] T069 [US3] Establish organization-first SHARE/NO KEY UPDATE gates in `crates/horae/src/db.rs`, `server_fns/project_creation.rs`, its options/finalize/editing modules and `projects.rs`; reauthorize task callers and prelock assignment/task-link parents without weakening editor isolation or legacy policy.
- [x] T070 [US3] Run affected and full server regressions, regenerate `.sqlx/`, verify offline all-targets Clippy and formatting; review production caller/trigger coverage and record actual results in `quickstart.md` and `progress.md`.

### Legacy report conversion lock integration

- [x] T065 [US3] Reproduce the converter versus actual archive/checkpoint writer deadlock in `crates/horae/src/jobs/report/tests.rs`; cover competing converters, changed/deleted candidates, exact archive preservation and current lease retention (FR-010/017/018; `contracts/permission-state.md`).
- [x] T066 [US3] Reorder `crates/horae/src/jobs/report/legacy.rs` to nonlocking discovery, organization SHARE and exact tenant/job oversized recheck under explicit READ COMMITTED on one connection; preserve atomic conversion and commit before rediscovery.
- [x] T067 [US3] Run converter/worker and affected regressions, regenerate `.sqlx/`, check offline all-targets Clippy and formatting; review lock compatibility, rollback and lease preservation and record results in `quickstart.md` and `progress.md`. This does not close full T039/T040/T042 or activate policy.

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

### Approved rate-field policy

T005/T029/T049 → T092 → T093 → T094 implements the pure financial gate in
`contracts/rate-scope-evidence.md`, not complete T014/T015. No parallel code work:
tests and implementation share the interface. Existing FR-021/022 decisions close
the local rule; no new Harvest account mutation or product choice is needed.

- [x] T092 [US3] Add failing action/owner/scope, profile-default, unrelated-grant, revocation and isolation tests in `crates/core/src/permissions/rates/tests.rs` (FR-006/008/010/021/022).
- [x] T093 [US3] Implement borrowed pure rate-field checks in `crates/core/src/permissions/rates.rs`, exposed from `permissions.rs`; reuse the catalog and scope checks, distinguish person/project/global owners, and keep cost grants independent without activating consumers (FR-021/022).
- [x] T094 [US3] Run focused/full core tests and core Clippy, check formatting, adversarially review the action/owner matrix and record evidence in `specs/015-scoped-permissions/quickstart.md` and `progress.md`; retain full consumer and migration gates.

### Materialized manager snapshot integration

T098 → T099 → T100 implements `contracts/manager-snapshots.md`, following the
existing organization-first gate and reviewed snapshot research. Work is
sequential because shared reader signatures and financial fixtures overlap.
No complete T039/T040 or US3 acceptance follows from this pair of consumers.

- [x] T098 [US3] Reproduce stale-authority reads and add transaction/race/preservation tests in `crates/horae/src/server_fns/invoices/snapshot_tests.rs`, exercising production preview and fee-balance helpers (FR-006/007/010/017/018).
- [x] T099 [US3] Add the shared prelude in `crates/horae/src/server_fns/snapshot.rs`, wire it from `server_fns.rs`, `projects.rs` and `invoices/preview.rs`, and retain the session actor through `invoices.rs` and test adapters; preserve existing snapshot queries and streaming behavior (FR-006/007/010/017).
- [x] T100 [US3] Verify reader/HTTP/financial regressions, bounded retry and cancellation, regenerate `.sqlx/`, run offline checks/lint/format, review adversarially and record evidence in `specs/015-scoped-permissions/quickstart.md` and `progress.md` (FR-018, SC-006 subset).

### Invoice editor snapshot integration

T100 → T101 → T102 → T103 extends `contracts/manager-snapshots.md`. Sequential
work shares the editor and financial fixtures. No new product predicate or
policy activation is introduced; full US3/T039/T040 acceptance stays separate.

- [x] T101 [US3] Add failing inherited-settings, revocation, revision/snapshot, cancellation and preservation tests for both editor readers in `crates/horae/src/server_fns/invoices/snapshot_tests/editor.rs` (FR-006/007/010/017/018).
- [x] T102 [US3] Reuse the manager prelude in `crates/horae/src/server_fns/invoices/editing.rs` load/review only; preserve business validation, DTOs and mutation guards (FR-006/007/010/017).
- [x] T103 [US3] Extend registered HTTP coverage in `crates/horae/src/server_fns/importers/authorization_tests/financial_snapshots.rs`, run reader/financial/server regressions and cache/offline/lint/format gates, adversarial review and analysis; record evidence in `specs/015-scoped-permissions/quickstart.md` and `progress.md` (FR-018, SC-006 subset).

### Materialized export authorization

T103 → T104 → T105 → T106 extends the same snapshot contract. These tasks are
sequential: signatures, shared fixtures and real-route harness overlap. The
independent check is current authority at materialization and response release,
not complete canonical US3 enforcement. No new grant predicate is inferred.

- [x] T104 [US3] Reproduce revoked-manager disclosure and add reader/race/timeout/cancellation/render-release tests in `crates/horae/src/reports/limits/tests/authorization.rs`, reusing existing limit fixtures (FR-006/007/010/017/018).
- [x] T105 [US3] Integrate the shared snapshot prelude and retained actor IDs in `crates/horae/src/reports.rs`, `reports/limits.rs`, `server_fns.rs` and `server_fns/snapshot.rs`; recheck after bounded rendering, preserve deadlines and keep CSV/Member paths unchanged (FR-006/007/010/017).
- [x] T106 [US3] Verify real export routes in `crates/horae/src/server_fns/importers/authorization_tests/exports.rs` and its existing harness; run full server/export regressions, SQLx/offline/lint/format gates, adversarial review and analysis; record evidence in `specs/015-scoped-permissions/quickstart.md` and `progress.md` (FR-018, SC-006 subset).

### Materialized project export authorization

T106 → T107 → T108 → T109 implements `contracts/project-exports.md`. Work is
sequential because the private row type, bounded query and HTTP fixtures overlap.
Scope gains/losses and final parent-wait freshness are mandatory, not just actor
revocation. This does not close CSV, canonical US3 or T039/T040/T042.

- [x] T107 [US3] Add failing actor/scope/freshness/size/render-release tests in `crates/horae/src/reports/limits/tests/project_authorization.rs`, reusing disposable fixtures and real writer boundaries (FR-006/007/010/017/018).
- [x] T108 [US3] Implement bounded fresh project materialization and captured-ID release authorization in `crates/horae/src/reports/limits/project.rs`, wired through `reports/limits.rs` and `reports.rs`; preserve existing CSV, columns and limits (FR-006/007/010/017).
- [x] T109 [US3] Extend real-cookie project XLSX checks in `crates/horae/src/server_fns/importers/authorization_tests/exports.rs` and its `exports/projects.rs` submodule, including a real finalize/editor writer winning the organization gate; update only the inactive-actor expectation in `crates/horae/src/reports/privacy_tests.rs`; run focused/full regressions, cache/offline/lint/format checks, adversarial review and analysis, recording evidence in `specs/015-scoped-permissions/quickstart.md` and `progress.md` (FR-018, SC-006 subset).

### CSV delivery authorization

T109 → T110 → T111 → T112 → T113 implements `contracts/csv-exports.md`.
All four tasks are sequential because source signatures, migration, buffer and
fixtures overlap. Independent acceptance requires all three CSV families, not
just the native helper. Full-policy activation remains separately gated.

- [x] T110 [US3] Reproduce inactive-actor acceptance in `crates/horae/src/reports/streaming/database_tests.rs` before modifying producers; cover initial missing/foreign/role-denied cases as interfaces are retained (FR-006/007/010/018).
- [x] T111 [US3] Add `crates/horae/migrations/0046_csv_export_cursor.sql` and checked native projections in `crates/horae/src/reports/streaming/cursor.rs`; integrate current-authority savepoints, bounded batches/output and retained actors through `reports/streaming.rs`, `reports.rs` and `reports/limits/project.rs`, reusing invoice adjustment formatting in `models/invoice.rs` (FR-006/007/010/017).
- [x] T112 [US3] Add deterministic backpressure/revocation, snapshot, byte/row/native-type and cleanup tests in `crates/horae/src/reports/streaming/database_tests/authorization.rs` and update the invoice snapshot fixture in `database_tests.rs` to pause after capture, preserving exact original-value assertions (FR-006/007/010/017/018).
- [x] T113 [US3] Register and verify all three real-cookie CSV routes in `crates/horae/src/server_fns/importers/authorization_tests.rs` and `authorization_tests/exports.rs`; run full server/export regressions, SQLx/cache/offline/lint/WASM/format gates, adversarial review and scoped analysis, recording results in `specs/015-scoped-permissions/quickstart.md` and `progress.md` before unsigned publication to existing draft #212 (FR-018, SC-006 subset).

## Phase 7: US4 — Custom profiles and permission explanations (P2)

### Isolated create/delete command implementation

T035/T036 → T053 → T054 → T055 implements the reviewed local boundary in
`contracts/template-commands.md`, without a public endpoint or activation.
Its organization/template/person-state lock set does not include legacy business
writers or user-row updates. T042 and full T037/T038 remain open for broader
commands and runtime integration; this subset does not declare them complete.

- [x] T053 [US4] Add failing command tests in `crates/horae/src/server_fns/permissions/tests/templates.rs` for every case in `contracts/template-commands.md`, using actual helpers and disposable PostgreSQL (FR-004/010/011/013/015/032).
- [x] T054 [US4] Implement typed internal create/delete commands and replay in `crates/horae/src/server_fns/permissions/templates.rs`, with additive org/principal-scoped receipt/audit storage in `crates/horae/migrations/0043_permission_change_receipts.sql`; keep runtime consumers and legacy data untouched, and regenerate `.sqlx/` (FR-004/010/011/013/015/032).
- [x] T055 [US4] Run permission and affected user/project regressions, offline server Clippy, formatting and focused adversarial review; record actual evidence and limits in `specs/015-scoped-permissions/quickstart.md` and `progress.md`. No full-story, T042 or activation completion follows from this subset.

### Remaining complete-story acceptance

Independent test: template creation/application/deletion and person-specific adjustments; C01 is resolved by the 2026-10-02 user decision: deleting a template preserves all assignees' effective grants/scope as person-specific configurations, including individually adjusted assignees. Test confirmation/cancellation, unavailable deleted templates and separate explicit revocation; both screens explain identical effective access. Remaining T006 predicates are still open.

- [ ] T016 [US4] Add custom dependency, unknown-grant, template lifecycle and audit tests in `crates/core/src/permissions/` and `crates/horae/tests/integration.rs` (FR-004/011/013/015/032).
- [ ] T017 [US4] Implement verified custom-template lifecycle and audit in `crates/horae/src/server_fns/`, `models/` and migrations; refresh `.sqlx/` (FR-004/013/015/032).

T010/T011/T016/T017/T018 share the ten
[profile-application cases](contracts/profile-application.md): distinguish load,
unchanged save, explicit selection/reset and final individual edits; use current
template/person revisions and confirmed relationship effects. Verify exact grants
after reload and unchanged other assignees. No profile-ID shortcut may discard
explicit edits or normalize stored grants. FR-032 and `permission-storage.md`
now close the storage-specific name/provenance gate. T016/T017 still test the
50-template limit and concurrent creation via authenticated commands. In-place
template update/rename is not an evidenced mandatory lifecycle operation.

Classification acceptance also varies available templates and equal-grant source
order without changing the person. Loading must not reinterpret presentation as
stored assignment provenance, mutate grants or emit an access-change audit.

- [ ] T041 [US4] Add and pass audit/receipt disclosure tests in `crates/horae/src/server_fns/permissions/tests/audit.rs`: current Administrator reads, revoked/non-admin/foreign denial, distinct operator attribution, sanitized outcomes and no false success audit after rollback (FR-011/013).

T061 → T062 → T063 → T064 follows `contracts/audit-lookup.md` and refines the
internal record-lookup portion of T041. No parallel code tasks: the reader, wire
contract and fixtures overlap. Full T041 remains open for authenticated history
delivery and integration, rather than claiming an internal helper completes it.

- [x] T062 [US4] Add failing production-reader and historical-decoder tests in `crates/horae/src/server_fns/permissions/tests/audit.rs`, using actual template/profile/delegation receipts and the allowed/denied, malformed, operator, rollback and revocation cases in `contracts/audit-lookup.md` (FR-010/011/013).

- [x] T063 [US4] Implement bounded Administrator-only receipt-ID audit lookup and strict historical wire projection in `crates/horae/src/server_fns/permissions/audit.rs`; wire it from `permissions.rs`, preserve trusted models and avoid raw intent/result disclosure or policy activation (FR-010/011/013).

- [x] T064 [US4] Run focused/affected PostgreSQL and decoder regressions, regenerate complete `.sqlx/`, verify offline server Clippy/formatting, adversarially review and record evidence in `specs/015-scoped-permissions/quickstart.md` and `progress.md`.

- [ ] T018 [US4] Align permission controls/descriptions in `crates/horae/src/pages/` Settings and Workspace using existing components and `design/project/app/08_Settings.dc.html` / `09_Workspace.dc.html`; apply design skills and browser viewport/theme/keyboard checks (FR-012/016).

T018's person editor consumes T126–T129 on the existing AdminUsers surface. Its
local draft uses the core prerequisite graph, keeps explicit profile intent and
invalidates confirmation on every edit. Test malformed loads, joint losses,
explicit keep-project access and identical-command retry. Reuse Modal, Checkbox,
form utilities and shared descriptions; no global CSS or active-policy change.
Template create/delete controls extend this dialog in `permission_editor/templates.rs`.
The controlled-response suite covers exact grants, blank/duplicate/limit errors,
identity separation, cancellation retaining the person draft, affected-person
confirmation, mismatched previews, stale saves, denial hiding and identical retry.
Creation/deletion does not also save the person; reload explicitly discards their
unsaved changes. Legacy shell/directory replacement, improved recovery and full
browser acceptance remain separate unfinished parts of T015/T017/T018.

Navigation protection extends T018 without closing it: verify dirty/reverted
grants, provenance and identity; preserve dirty person state through template
operations; block pending initial/reload/preview/save requests; and require
confirmation for dirty Close/Cancel/Escape/backdrop exits. Keep shared
project/invoice history and scroll regression cases. Controlled handlers and
Node script tests are not browser evidence. Durable same-request recovery after
forced reload, bound to the original requester/workspace, remains required.

The durable consumer in `permission_editor/recovery_storage.rs` and `recovery.rs`
extends the same T018 acceptance, not a replacement feature. Map FR-004/012/016/018
to store-before-send, exact retry, requester isolation, no automatic submission,
conditional cleanup, denied-retry retention and explicit checked discard. Test
both person/template production controls, remount with no selected person,
malformed/misbound/oversized storage and successful-response cleanup failure in
`tests/permission_editor_ui.rs`; execute the shipped JS with
`tests/browser/permission-recovery-storage.cjs`. Complete server/WASM lint,
formatting and adversarial review; full rendered recovery, keyboard, viewport
and theme acceptance still must pass before checking T018.

The real `permission-editor-recovery.cjs` browser suite now adds matching server/
WASM, session-cookie and disposable PostgreSQL evidence for person/template lost
responses, no-auto-submit reload, one-receipt replay, changed-account isolation,
same-user reauthentication, authority denial/restoration, deleted-template replay
and browser storage failures. It checks keyboard access, background inertness,
dirty Escape/focus restoration and wide-dark/narrow-light layouts. It does not
close full T018 or canonical shell/directory integration. See quickstart for the
exact evidence and remaining browser/feature boundaries.

Relationship-loss labels extend the authorized preview in `permissions/editor.rs`,
not its stored audit or the command confirmation shape. Cover names, inactive
subjects, other managers/tenants, no-loss proposals, denial, strict historical
decoding and real HTTP delivery in the existing editor tests. The UI renders
escaped names with the existing wrapping utility and still submits exact IDs.

### Authenticated own-permission projection

T035/T036/T057 → T095 → T096 → T097 implements `contracts/own-permissions.md`.
Sequential work: the reader, DTO and existing HTTP matrix share the test boundary.
No full US4 or activation completion follows from this read-only slice.

- [x] T095 [US4] Add failing strict/legacy/own-scope/revocation reader tests in `crates/horae/src/server_fns/permissions/tests/own.rs` and real-cookie route cases in `server_fns/importers/authorization_tests/own_permissions.rs` (FR-006/007/010/012/017/018).
- [x] T096 [US4] Implement `server_fns/permissions/own.rs`, a separate shared `models/own_permissions.rs` DTO and no-argument `get_my_permissions` wrapper in `server_fns/auth.rs`; update module wiring and operation inventory without policy activation or changing existing guards (FR-012).
- [x] T097 [US4] Verify production-reader and registered-route tests, affected regressions, complete SQLx cache, offline server/web builds, lint/format and adversarial review; record results in `specs/015-scoped-permissions/quickstart.md` and `progress.md`.

### Authenticated audit delivery

T064 → T123 → T124 → T125 refines T041 under `contracts/audit-lookup.md`.
Sequential work shares the historical DTO, reader and real-route harness.
No history browser, privilege mutation or policy activation is included.

- [x] T123 [US4] Add failing registered-route disclosure tests in `crates/horae/src/server_fns/importers/authorization_tests/permission_audit.rs` and reader deactivation/settings/cleanup tests in `server_fns/permissions/tests/audit.rs` (FR-010/011/013/018).
- [x] T124 [US4] Move historical wire types to `crates/horae/src/models/permission_audit.rs`, retain strict server decoding, fence current requester reads in `server_fns/permissions/audit.rs` and expose the session-derived single-receipt wrapper in `server_fns/auth.rs`; update module wiring and operation inventory (FR-010/011/013).
- [x] T125 [US4] Verify real HTTP, permission/decoder regressions, complete SQLx cache, offline server/WASM lint and formatting; perform adversarial review and scoped analysis, recording evidence in feature 015 `quickstart.md` and `progress.md` (FR-018).

### Own-permission Settings integration

T097 → T120 → T121 → T122 follows the Settings consumer contract in
`contracts/own-permissions.md`. Sequential tests and UI wiring share the same
component. No new policy predicate, mutation or activation is introduced.

- [x] T120 [US4] Add failing SSR state, exact-grant, independent-identity and disclosure tests in `crates/horae/src/pages/settings/own_permissions/tests.rs` (FR-012/016/018).
- [x] T121 [US4] Integrate the read-only resource in `crates/horae/src/pages/settings/own_permissions.rs` and `settings.rs`, with shared descriptions in `components/permission_description.rs`; preserve existing settings, use existing classes and remove the DTO's obsolete unused-consumer annotation (FR-012/016).
- [x] T122 [US4] Exercise resource refresh wiring in `crates/horae/tests/own_permissions_ui.rs`, run focused tests, server/WASM lint and formatting, review exact projection and CSS consistency, and record evidence/visual limitations in feature 015 `quickstart.md` and `progress.md` (FR-018). Do not close full T018 or claim activation readiness.

## Phase 8: US5 — Data-preserving transition (P1)

Independent test: populated migration fixture, reviewed access differences, safe retry and no import-driven privilege overwrite.

### Read-only source preflight

T117–T119 refine the diagnostic portion of M01/M07/M08 under
`contracts/migration-preflight.md`. They depend on the existing schema/current
Administrator boundary, not an unreviewed target mapping. Sequential tests →
reader → verification; no activation, repair or full US5 acceptance follows.

- [x] T117 [US5] Add failing diagnostic/authority/revocation/preservation tests in `crates/horae/src/server_fns/permissions/tests/preflight.rs` for the closed preflight contract (FR-006/010/014/017/018).
- [x] T118 [US5] Implement the internal count-only reader in `crates/horae/src/server_fns/permissions/preflight.rs`, wire it in `permissions.rs` and regenerate `.sqlx/`; preserve all existing data and active authorization.
- [x] T119 [US5] Run focused/permission regressions, complete SQLx preparation, offline lint/build and formatting; review the diagnostic boundary adversarially and record evidence in `specs/015-scoped-permissions/quickstart.md` and `progress.md`.

### Reviewed migration and activation

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

T001 → T002 → T003 (RED) → T004 (GREEN) → T005. T006–T009 are mandatory before replacing legacy authorization and full-feature acceptance, not before every confirmed pure subtask of T010–T020. They must not be marked complete using foundation-only tests. Each increment needs a closed local contract and tests before implementation. T049 → T050 → T051 → T052 is complete; T035/T036 now follow the reviewed storage-specific contract and FR-032. US3 depends on the permission/assignment model; UI depends on shared effective grants; cutover requires every delivery path and migration acceptance. No parallel code tasks are designated because the shared model and integration fixture overlap. Reference/migration documentation can proceed independently of pure model work; no independent full-story acceptance is implied.

Persistence refinement: T033 → T034 informs T008/T009. After the storage-specific
gates above pass, T035 → T036; resolved command predicates and T042 additionally
gate T037 → T038. Storage is tested only in isolated databases; this does not
authorize real-data migration or activation. T042 informs T009; T042 → T039 → T040.
T041 follows audited mutations before
exposing audit reads. File-level subtasks do not replace story acceptance gates.
