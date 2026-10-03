# Implementation Plan: Scoped Roles and Permissions

**Branch**: `feat/scoped-permissions` | **Date**: 2026-09-30 | **Spec**: [spec.md](spec.md)

## Summary

Deliver Harvest parity: six built-in profiles, reusable custom profiles, per-person adjustments and scoped approvals. No fixed-role-only or whole-week-only substitute is accepted.

This is an incremental plan. The independent record-scope foundation is executable; the full feature's research and policy design are not complete. Implementing that foundation does not authorize replacing runtime role checks. FR-002's complete operation matrix, adoption of the included governance amendment, dependent-spec reconciliation and reviewed migration remain prerequisites to the policy cutover.

## Technical Context

- Rust edition 2024, existing `horae-core` and Dioxus/Axum application crates.
- Existing UUID, serde and chrono dependencies; no new crate, dependency or policy engine.
- Pure scope evaluation has no I/O or persistence. Later persistence uses PostgreSQL, organization foreign keys and UUID v7 primary keys.
- Foundation validation: exhaustive unit tests, core regression suite, Clippy and Nix formatting.
- Runtime validation: SQLx integration tests, cross-surface authorization tests and browser comparisons with Harvest using disposable fixtures.
- Scope evaluation borrows assignments and allocates nothing. No new performance SLA is invented.

## Constitution Check

The [profile-application contract](contracts/profile-application.md) refines
FR-004's explicit command intent and T010/T011/T016–T018 acceptance. Preserve
canonical grants on unchanged saves and commit reviewed individual edits even
when the selected profile identity is unchanged. FR-032 settles creation-name
equivalence. The reviewed [storage contract](contracts/permission-storage.md)
separates authoritative grants/identity/provenance from computed presentation;
unverified Harvest backend classification does not block this additive storage.

| Gate | Foundation | Runtime cutover |
| --- | --- | --- |
| Exact integer time/money | No arithmetic changes | Preserve totals and historical values |
| Pure domain rules | Scope evaluator in `horae-core`, no I/O | Shared decisions, trusted facts loaded by server |
| PostgreSQL, org isolation, UUID v7 | No schema changes; isolation tested | Required in migrations and every assignment |
| Server-function mutations | No new mutation surface | All access mutations session-authenticated |
| Reproducible validation | Nix core tests, Clippy and formatting | Full flake and integration checks before merge |
| Authorization target and transition | Runtime roles unchanged | Constitution 1.1.0 amendment included; matrix, migration and cross-surface acceptance still required before cutover |

Foundation design passes these gates. The full-feature post-design check is still open, not waived. No complexity exception is requested.

## Project Structure

```text
specs/015-scoped-permissions/
  spec.md
  plan.md
  research.md
  data-model.md
  contracts/record-scope.md
  quickstart.md
  tasks.md
crates/core/src/
  permissions.rs
  permissions/tests.rs
crates/horae/src/
  server_fns.rs
  server_fns/
  models/
  pages/
crates/horae/tests/integration.rs
```

## Execution and Dependencies

### Increment readiness versus activation

The 2026-10-03 user authorization repairs planning dependencies without reducing
parity. A confirmed increment needs its own closed contract, requirement-to-test
mapping, dependency review and adversarial review. It does not require already
passing full-feature runtime outcomes. Unrelated open predicates remain open.

| Work | Entry gate | Exit gate / what it does not authorize |
| --- | --- | --- |
| Completed: pure person-management validation, T050–T052 | Completed catalog/restoration, FR-027/028/029/031 and `contracts/person-management-validation.md`; no unresolved local predicate | Red/green tests, core regressions, Clippy, formatting and focused review; no server consumer, schema, assignment write or complete US2 acceptance |
| Non-activating storage, T035/T036 | FR-032 and reviewed `contracts/permission-storage.md`; schema/tenant/loader checks and isolated PostgreSQL tests | Evidence for storage only; no inferred mappings, authenticated command acceptance or automatic runtime consumers |
| Replacing guards / activating policy | T006–T009, T042, reviewed migration and concurrency/recovery design, complete cross-surface integration | Full allowed/denied, revocation, migration and browser acceptance plus Nix gates; no endpoint-by-endpoint fallback to old roles |

T050 → T051 → T052 is implemented and verified. Its validation uses the existing catalog and
trusted facts, not saved-profile classification or approval execution rules.
The requirements checklist measures specification coverage; T020 measures actual
runtime outcomes. Neither local readiness nor an incomplete full checklist marks
the entire feature ready. See `checklists/person-management-validation.md`.
T035/T036 and T053–T055 are complete for isolated storage and reusable-template
create/delete commands under `contracts/template-commands.md`. Real command,
replay and audit tests pass without exposing a new policy. Next refine the person
profile/application and assignment transactions in T037/T038 and reconcile their
T042 lock dependencies. T042 and T008's broader integration gates remain open;
only disposable fixtures enable version 1.

T056–T058 now refine the next internal person-profile transaction under
`contracts/person-profile-commands.md`. Explicit command intent determines
Administrator transitions; inactive targets retain their activation state.
Separate management relations avoid legacy membership cascades. This bounded
post-design constitution check passes: existing dependencies, typed grants,
tenant FKs, UUID v7, atomic audit and disposable PostgreSQL only. It does not
close T042, replace legacy guards or authorize mixed-policy operation.

### Existing increments and remaining integration

T083–T085 isolate durable CSV parser waits from SQL transactions under
`contracts/permission-state.md`, preparing bounded worker authorization without
selecting historical-job or retry policy. Reuse the parser's one-row channel,
500-record checkpoints, reserved connection and existing cancellation/preview
logic. Local constitution review requires no schema, dependency, grant mapping or
real-data change. Do not infer that this implements execution-time permission
checks; requester provenance and reviewed activation remain separate.

T080–T082 implement the bounded error-download contract in
`contracts/permission-state.md`. Share the existing importer authority guard,
authorize metadata and each archive page in short transactions, and reauthorize
the captured inline tail before release. Preserve snapshot bytes and bounded
buffering without locks across client consumption. The local constitution check
needs no schema, dependency, grant mapping or real-data change. Full policy
activation and worker effect authorization remain separate gates.

T077–T079 close the current importer job-control/status boundary under
`contracts/permission-state.md`. Extract pool-injected server helpers; authorize
under READ COMMITTED organization SHARE then current Administrator SHARE, mutate
through caller-owned queue transactions and project status before commit. Queue
primitives remain separate from user authority; no worker receives an invented
actor. Buffer CSV outside locks. The local constitution check passes without new
schema, dependency, grant mapping or real-data change. Execution-time authority,
report downloads and full T042/cutover remain separate gates.

T074–T076 implement the current connection-management authority contract in
`contracts/permission-state.md`: nonblocking import reservation, explicit READ
COMMITTED, organization SHARE, current active same-tenant Administrator under
user SHARE, then existing generation/credential writes. Actor IDs come only from
the authenticated wrapper or validated OAuth attempt. This local constitution
check passes without schema, dependency, grant mapping or real-data changes.
Service token refresh/import execution and full OP28/T042 remain separate gates.

T071–T073 close the current branding writer's admission-to-commit gap under
`contracts/permission-state.md`. Preserve Manager/Admin policy, organization
UPDATE first and post-commit change events; reload the active tenant-bound actor
under SHARE and explicit READ COMMITTED before any result or write. This bounded
constitution check needs no schema, dependency, mapping or real-data change.
Tests cover revocation waits and denied no-op disclosure as well as actual writes.
Full OP27, T042 and runtime cutover remain open.

T068–T070 implement the reviewed project-family prefix in
`contracts/permission-state.md`: shared organization gates for drafts/reads,
NO KEY UPDATE for project/assignment/task-link changes, then current actor
reauthorization. Preserve editor isolation and its existing project UPDATE;
new revision-only parent prelocks use NO KEY UPDATE for FK compatibility.
The local constitution check passes without a schema, dependency, grant or
real-data change. This closes a named integration boundary, not full T042.

T065–T067 repair the concrete legacy-report conversion inversion documented in
`contracts/permission-state.md`. Discover without a row lock, acquire organization
SHARE, then recheck and lock the exact tenant/job under explicit READ COMMITTED.
Keep discovery and conversion on one connection and commit before rediscovery.
Preserve bounded chunks, rollback and lease fencing; a job already bounded by a
worker must retain its lease. Independent lock research reviewed this local path.
The constitution check passes without new schema, dependencies, policy activation
or real-data operations. Full T042 and gated import authorization remain open.

T062–T064 implement the internal audit-read contract in `contracts/audit-lookup.md`
using receipt-ID lookup, organization SHARE and strict historical projections.
The bounded constitution check passes: no mutations, new storage, dependency or
active endpoint; canonical current Administrator authority precedes disclosure.
Historical wire DTOs do not become trusted authorization facts. Independent
review closes the local lock path, not full T042 or the history UI.

T059–T061 implement internal project delegation under
`contracts/project-management-commands.md`. The bounded post-design check passes:
reuse 0044, canonical grants, organization revisions and atomic receipts; no new
dependency, public endpoint or legacy data change. Independent lock research found
an INSERT FK cycle with the existing editor; parent KEY SHARE NOWAIT and complete
rollback on contention close this local edge. Editor reconciliation and full T042
remain mandatory before activation. Person-management inactive-target predicates
are not silently inferred from the separate FR-026 project rule.

The renewed implementation request also permits T047–T049: strict restoration of
the confirmed saved grant selection in the pure core. Validate catalog version,
duplicates, Member floor and prerequisite closure without adding grants on load.
Keep this separate from editor normalization. No schema, source-classification,
administrative identity or runtime authorization is introduced by this increment;
the persistence and cutover gates below remain unchanged.

The 2026-10-02 implementation request permits advancing the confirmed catalog and
profile/dependency model with local tests while reference conflicts remain isolated.
Follow [grant-catalog.md](contracts/grant-catalog.md) and T027–T029 before runtime
integration. This does not reduce the requested feature, resolve C01–C07 by guess,
approve legacy mappings or waive the runtime cutover checks below.

1. Implement the independent FR-006 scope predicate with failing tests first: own, managed people, managed projects and organization; union without privilege inference; fail closed for inactive or mismatched identities/organizations.
1. Finish the Harvest reference matrix, custom prerequisite graph, approval/withdrawal semantics and access-path inventory. Inspect current runtime checks and review migration differences. Review the included constitution amendment and reconcile dependent feature contracts before cutover.
1. Finalize capability/profile and assignment persistence contracts. Add transactional authorization, stale-edit rejection, audit and concurrent last-administrator protection.
1. Integrate current permissions across server functions, lists/aggregates, exports/downloads, compatibility API, CLI, jobs and plugins. Do not activate a partially migrated policy.
1. Replace whole-week-only approval storage with verified date/project coverage, including empty-cell locks, submission editing and independent locks.
1. Implement Settings/Workspace permissions with shared descriptions, then verify full parity and migration fixtures. Keep the PR draft until all acceptance gates pass.

Steps 2–6 need detailed contracts for the affected work before coding. The local gates above allow confirmed pure increments without waiting for unrelated decisions. Replacing guards still requires the complete matrix, migration and concurrent activation/recovery review. Step 1 does not satisfy any full user story by itself.

The [dependent-spec reconciliation](contracts/dependent-spec-reconciliation.md)
records current source revisions and accepted cross-feature obligations for
T008. The shared editor's cost/notes contract is reconciled conditionally for
the reviewed cutover; other feature worktrees remain untouched. Legacy Clients
MVP acceptance is deliberately distinct from full six-profile acceptance.
Remaining operation predicates and migration still prevent completing T008.

The 2026-10-02 persistence proposal in [data-model.md](data-model.md) and
[permission-state.md](contracts/permission-state.md) details canonical saved
grants, explicit administrative identity, composite tenant references, revisions,
atomic audit/outcome storage and one organization-first lock protocol. T033/T034
review these mechanics; T035–T041 refine storage and integration. The separately
reviewed 2026-10-03 storage contract permits additive T035/T036 work in disposable
databases. It does not complete Spec Kit Phase 1 or the full post-design check;
no policy activation, existing-data mapping or cross-surface gate is waived.

Storage stays in the existing app (`server_fns/permissions.rs` plus
`permissions/`); typed read models live in `models/permissions.rs` and remain
server-only until separately reviewed UI projections are needed. No new crate,
repository framework or policy engine is proposed. Reuse tenant keys and
transaction patterns after reconciling lock order across all writers. Saved
template behavior is not selected by the storage representation.

The 2026-10-02 user clarification now selects C01 deletion semantics: detach the
template while preserving assignees' effective grants/scope as person-specific
configurations. Apply it to US4 tests and persistence/audit design. This resolves
that product choice only; FR-032 subsequently settles creation-name equivalence.
Unverified saved classification remains a presentation acceptance limitation,
not a requirement to mirror Harvest's private schema. Other open work includes
remaining approval/withdrawal predicates, migration mappings and full runtime readiness.

The user also selected C02 option A: authorize financial report projections and
their matching exports through the report grant, without adding ordinary rate
grants or requiring them. T014/T015 must distinguish report-only access from
direct source/rate access, test the reciprocal rate-only denial, and preserve
current scope/revocation checks. This is an approved Horae contract, not verified
restricted-user Harvest enforcement or permission to activate an incomplete policy.

The user selected C03's resource-specific rate scope (FR-021): person defaults
follow person management; project rates and person/task overrides follow project
management. Both require the matching financial permission. T014/T015 must cover
the independent person/project cross-product, inherited-rate versus history
projection, read-only denial and revocation. C04 now follows approved FR-022:
explicit organization-wide cost read/write, Accounting/Executive read-only,
Administrator read/write and custom effective grants. Apply the same separation
to ordinary cost payloads and supported project overrides, preserving unrelated
resource authority. These decisions do not approve migration mappings or activate
runtime policy; local integration and restricted-user reference checks differ.

The [operation matrix](contracts/operation-matrix.md) maps all 80 inspected public
async server-function symbols plus HTTP/authentication, worker and operator
surfaces to target dimensions and remaining predicates.

C05's dependency is now documented by current Harvest guides, not a new product
choice: implement the [company-lock contract](contracts/company-locks.md) under
FR-023. T043–T046 refine calendar calculations, persistence/serialization,
settings/banner UI and worker integration. Retain the existing infrastructure;
configuration and due execution must share current organization state. Full
implementation remains gated, including feature 016 for combined expense behavior.

The user accepted C06's approval-visibility boundary in FR-024 and
[approval visibility](contracts/approval-visibility.md): require current approval
authority and visibility over all selected time/expense records, or deny the
complete command. T012/T013 cover policy and transactional checks, including
revocation and concurrent additions; feature 016 supplies expense fixtures.
Do not add a catalog dependency, silently filter expenses, infer ordinary expense
editing or extend this approval decision to the unresolved withdrawal contract.

The user selected C07 retention option A (FR-025): effective managed/all project
read retains existing designations without project editing. T012/T013 must cover
confirmed atomic read-loss removal, stale previews, cancellation, preserved
membership/history and recomputed scope across delivery paths. Do not silently
restore permissions or conflate retention with new assignment, promotion or
the explicit keep-access action, now specified by FR-030 and
[its contract](contracts/keep-project-access.md). Use existing revision/audit mechanics.

FR-026 separately resolves delegation: current project editors may add/remove
manager designations within their authorized projects, without changing global
grants. Adding evaluates existing target project-read grants against the proposed
assignment; read-only actors cannot delegate. T012/T013 cover atomic scope-only
changes, revocation, eligible-target checks and non-disclosing responses. Project
creation remains a separate gate. FR-031 now forbids person-management self-links
even for Administrators, while preserving independent own/all access.
FR-027 now reserves person-management relationship writes to
active same-organization Administrators, independently of PeopleWriteAll and
FR-026 project delegation. T012/T013 must cover add/remove/replace, direct and
self-set requests, revoked authority, stale revisions, atomic audit and preservation
of profiles, grants, project membership and history. T016–T018 must distinguish
read-only explanations from assignment controls.

FR-028 resolves new person-management assignment eligibility: require existing
grants applicable to managed people, evaluating scope with the proposed assignment
without adding permissions. T012/T013 must check every compatible grant family,
unrelated-only denial and revocation before commit. This permits read-only
compatible grants and does not require people-directory/edit access. It does not
make the complete operation matrix ready. FR-029 now supplies retention: keep
relationships while any compatible grant remains, otherwise preview and confirm
atomic permission/assignment/revision/audit changes. T012/T013 cover rollback,
stale confirmations, preserved incoming relationships and history, and no automatic
restoration of removed assignments. FR-031's identity restriction is separate from
self-approval and applies to add/replace commands without excluding an
Administrator's own set of other people.

The
[concrete lock inventory](contracts/permission-state.md#concrete-lock-inventory-t042-partial)
records current ordering, trigger/FK effects and a candidate common hierarchy.
It identifies READ ONLY transaction incompatibility and network-paced imports
that cannot simply acquire the proposed gate. These are T006/T042 inputs, not
completed operation predicates or verified concurrency integration.

The [migration contract](contracts/migration.md) now defines the reviewable access
diff, stale-preview/atomic-cutover acceptance and historical provenance gaps for
step 2. Mappings and compatibility policy remain unapproved; the artifact's
existence does not complete T007 or authorize policy activation.

### Independent tenant-isolation repair

The access inventory found missing organization filters in the existing approval/reopen mutations. Correct these without introducing new roles or changing same-organization weekly semantics. Extract the existing transactions into pool-injected helpers so database tests exercise the production SQL; retain plugin dispatch after commit in the server wrappers. Test Manager and Admin callers against foreign pending/approved weeks, mixed-ID bulk requests, same-org success and invoice-lock preservation. Regenerate the SQLx cache and run approval regression tests. This closes an existing invariant violation; it does not bypass the matrix/governance gates for the new policy.

The same inventory identifies unscoped legacy assignment creation/removal. Preserve their existing administrator-only policy, but validate both project and person organization and reload active administrator authority under a transaction lock. Tests must call the production helpers, cover both foreign-ID directions, unknown IDs, duplicate creation, repeat removal, and completed/concurrent revocation. Return no foreign assignment details and emit events only for committed changes. This requires no schema or role migration and does not settle the future assignment-authority matrix.

User creation, role changes and activation changes also need transactional
reauthorization. Share the existing organization lock for last-administrator
protection, then reload and lock the active same-organization actor through commit.
Exercise completed revocation, waits on both organization and actor locks,
foreign/unknown actors, creation rollback and the existing concurrent last-admin
cases against the actual helpers. This closes the current FR-010/011 boundary;
it does not replace profile persistence, revisions or durable audit.

## Workflow Notes

The checked-in `setup-plan.sh --json` and `setup-tasks.sh --json` were executed again for the authorized dependency repair, preserving existing artifacts. Their feature identifier is `015-scoped-permissions`; the actual Git branch remains `feat/scoped-permissions`. No extension hooks or `update-agent-context.sh` exist here; no agent-context generation is claimed. Requirements checklist remains 12/16; local readiness is evaluated separately, without claiming complete Phase 0/1 or full-feature Analyze.
