# Interactive time-writer activity fence

FR-010/018; prerequisite for OP04 integration, not canonical time-write scope or
approval-policy activation. Source inspection at `c80233b` finds six production
transaction entry points in `server_fns/time_entries.rs`: insert (manual/timer),
stop, update, delete, reschedule and reorder. Each accepts the session-resolved
person ID but starts only the advisory submission barrier. Revocation after
`require_user` is therefore not rechecked by that shared transaction prefix.

## Contract

- Derive organization from the server-supplied person ID within a bounded
  READ COMMITTED, READ WRITE transaction. This first lookup is routing only.
- Acquire organization SHARE before actor/resource locks. Reload the same
  active person in that organization with SHARE, then acquire the existing
  shared timesheet barrier. A missing/inactive person is forbidden; never
  acquire another person's authority from browser input.
- Hold activity and organization fences until commit/rollback. Deactivation
  first denies the pending mutation; a mutation admitted first may finish,
  then later mutations are denied. Failed/no-op operations do not bypass this.
- Use the prefix for every interactive time mutation, keeping current tracking,
  task, ownership, business-state, billability, duration and submission checks.
  Stopping an active person's own timer after task access removal stays allowed;
  that exception does not admit an inactive account.
- Do not change the lower-level advisory helper used by service imports and
  barrier tests into a user-authorization API. Service authority is separate.
- Organization-first fencing follows the existing access-change hierarchy.
  Entry writes do not promote their organization lock. Existing submission and
  invoice FKs require compatible KEY SHARE, not an organization lock upgrade.
  Full cross-command T042 and delegated writes remain separate gates.

## Verification

Reproduce a post-session inactive-owner edit before fixing it. Cover every
interactive mutation with inactive and missing owners, retaining complete source
rows on denial. Verify direct deactivation and the production organization
access-change gate in both directions, cancellation, inherited transaction defaults, and existing
submission/task-revocation/invoice behavior. Regenerate SQLx and verify native
and WASM consumers. Tests use only the owned disposable PostgreSQL.

This work neither resolves teammate picker candidates nor implements delegated
editing, submitted-entry editability, locked corrections or combined approvals.
