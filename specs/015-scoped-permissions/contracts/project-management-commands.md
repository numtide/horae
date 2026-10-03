# Internal project-management replacement

Implements FR-005/010/011/013/017/026 inside the inactive policy boundary.
T059–T061 refine T012/T013, not complete US2 or T042. Existing migration 0044
supplies the independent relations; no schema change or runtime endpoint is needed.

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

Start READ COMMITTED, lock organization FOR UPDATE, validate policy version and
current actor authority. Exact canonical receipt replay requires that same current
project authority, including after self-removal; then precedes stale-revision
checks. Changed intent, including another command kind, conflicts before decoding
its result. Outcomes contain only project ID, access revision and changed status;
target grants and audit contents are never returned to project editors.

For a new request check the expected organization revision, then lock the tenant
project FOR KEY SHARE NOWAIT. PostgreSQL 55P03 causes explicit whole-transaction
rollback and a retryable busy outcome, never an in-transaction retry. This prevents
the concrete legacy-editor cycle: project UPDATE → organization SHARE versus
organization UPDATE → project FK KEY SHARE. Parent protection then covers child
inserts. Plain user reads and FK KEY SHARE do not conflict with existing user SHARE
locks. Do not add project writes, legacy membership writes or parent-write triggers.

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

Read-only adversarial contract review found no unresolved high finding in this
subset. Full T042, activation, UI and editor integration remain open: the legacy
editor still reads/writes `assignments.role` and uses `projects.edit_revision`.
Integration must use canonical relations and the access revision, not pretend this
internal command already updates that UI or add a trigger to synchronize two truths.
