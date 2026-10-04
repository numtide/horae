# Internal project-management replacement

Implements FR-005/010/011/013/017/026 inside the inactive policy boundary.
T059–T061 refine T012/T013, not complete US2 or T042. Existing migration 0044
supplies the independent relations; no schema change or runtime endpoint is needed.

The internal increment above is complete. The following delivery increment
connects it only behind the existing policy-1 boundary; it does not enable that
policy, replace legacy form saves or imply complete runtime enforcement.

## Authenticated delivery (T189–T191)

`load_project_managers(project_id, expected_requester)` derives the actor and
tenant from the session. Optional expected requester pins subsequent reads;
mismatch denies without loading another account's selection. Return exactly
requester, project ID, organization access revision and the complete retained
manager set ordered by UUID, each with ID/name/activity only. No rates, email,
grants, legacy role, provenance, receipt or project financial data is returned.
Retained archived or no-longer-compatible managers remain represented without
loading their permission state; otherwise saving the visible set could silently
remove them. Candidate search remains the separate active-only project picker.

The read uses bounded READ COMMITTED, organization SHARE and actor SHARE, then
the same current project-edit grant/designation predicate as replacement. Policy
0 is unavailable, with no legacy fallback; no Administrator-identity or people
grant shortcut. Materialize the selection before releasing the transaction.
An archived project remains editable under the existing contract. Neither reads
nor denials update revisions, relationships or receipts.

`save_project_managers(command, expected_requester)` requires a matching original
requester pair and delegates to the existing transaction. Keep the requester
envelope outside its serialized intent so old receipts retain exact replay.
Do not synthesize project edit revisions or authorize through picker results.
Return only the existing outcome; current project authority is rechecked even on
replay. After managed-only self-removal, successful save is acknowledged but
subsequent load/replay denies. Permission audit remains Administrator-only even
for the command's author; it is not a bypass for recovery after revocation.

Authentication failure is 401; requester/current authority failure is 403;
missing/foreign project is non-disclosing; duplicate/ineligible selection is 400;
stale, busy and changed-request intent are 409. Storage, malformed state, receipt
decoding and revision exhaustion use a fixed sanitized 500 message. No IDs,
grants or database diagnostics appear in public failure text.

Acceptance uses actual PostgreSQL readers/commands and the existing registered
server/session test harness. Cover all/managed/read-only scope, active and
retained archived targets, no target state, foreign IDs, both requester fields,
policy/version/storage failures, same-cookie revocation, exact replay and
atomic invalid batches. Reader concurrency tests use real blockers in both
revocation orders and cancellation cleanup. The frontend must integrate this
canonical set in the design's existing project editor once its complete field
contract is closed; no new ad-hoc manager page or modal is introduced here.

## Authority and intent

An authenticated same-organization active actor needs canonical `ProjectWriteAll`,
or `ProjectWriteManaged` with a current designation on the requested project.
Neither a proposed designation, legacy membership/role nor administrative identity
alone substitutes for these grants. Strict loading rejects malformed state.

The command replaces one project's complete manager-ID set with an expected
organization access revision and principal-local request ID. Sort IDs; reject
duplicates instead of dropping them. Newly added managers must be active, in the
same organization and already have canonical managed/all project-read grants.
No promotion, grant normalization or tracking membership change is allowed.
Eligibility applies only to additions, not retained or removed relationships.
Empty replacement and authorized self-removal are valid. There is no minimum
manager count or restriction on otherwise authorized project self-designation.

Archived projects follow existing project-editor availability: do not change
activation or introduce an active-project-only restriction. Person-management
inactive-target lifecycle remains unresolved and outside this command.

## Atomicity and replay

Use the existing administration setup: READ COMMITTED, READ WRITE and local
statement/idle limits of at most 5/10 seconds, preserving stricter inherited
limits without changing session defaults. Lock organization FOR UPDATE, then
read actor activity FOR SHARE and validate policy version/current authority.
Hold actor SHARE through receipt lookup or commit. Exact canonical receipt replay requires that same current
project authority, including after self-removal; then precedes stale-revision
checks. Changed intent, including another command kind, conflicts before decoding
its result. Outcomes contain only project ID, access revision and changed status;
target grants and audit contents are never returned to project editors.

For a new request check the expected organization revision, then lock the tenant
project FOR KEY SHARE NOWAIT. PostgreSQL 55P03 causes explicit whole-transaction
rollback and a retryable busy outcome, never an in-transaction retry. This prevents
the concrete legacy-editor cycle: project UPDATE → organization SHARE versus
organization UPDATE → project FK KEY SHARE. Parent protection then covers child
inserts. Read every newly added manager's activity FOR SHARE in sorted ID order
before loading their grants. Hold those locks through receipt commit: FK KEY SHARE
alone does not protect the active flag against a non-key update. Existing user
SHARE locks remain compatible. Do not require activity for retained/removed
managers, add user/project writes, legacy membership writes or parent-write triggers.

Read current relationships in deterministic order. Validate every addition before
writing; preserve retained IDs/revisions, delete removed edges and insert UUID-v7
new edges. Advance organization revision exactly once for an actual change,
checking overflow first. Exact no-op preserves revisions and writes an unchanged
receipt, not a fabricated change. Audit added/removed IDs, managers and revisions
with the same commit; any audit failure rolls everything back. All future access
writers must participate in this organization gate/revision protocol.

## Required production-command tests

- All/managed actor success, read-only or undesignated denial; no identity bypass.
- Compatible managed/all-read additions; incompatible, inactive, foreign, missing
  or malformed additions reject the entire replacement and preserve prior state.
- Retained inactive/incompatible targets and removal without eligibility; exact
  no-op, duplicate rejection, archived projects and membership/history preservation.
- Exact reordered replay, cross-command request conflict, stale revision and
  overflow; current revoked/inactive/foreign actor denial including replay.
- Authorized self-removal followed by denied replay; no grant-bearing outcome.
- Concurrent replacements, revocation winning the organization gate, and a held
  legacy-style project UPDATE produce no stale commit or lock cycle.
- Audit failure preserves links, permissions, revision and receipts atomically.
- Actor/new-manager deactivation first makes the locking check wait and then
  reject; command first holds activity through receipt commit. Exercise actual
  PostgreSQL blockers, not assumed task timing. Cancellation releases the gate
  and rolls back relationship/revision changes. Inherited READ ONLY/repeatable-read
  defaults do not alter command semantics or get overwritten at session level.

Read-only adversarial contract review found no unresolved high finding in this
subset. Full T042, activation, UI and editor integration remain open: the legacy
editor still reads/writes `assignments.role` and uses `projects.edit_revision`.
Integration must use canonical relations and the access revision, not pretend this
internal command already updates that UI or add a trigger to synchronize two truths.
