# Reusable-template command boundary

This is the create/delete portion of FR-004/010/011/013/015/032 and
T016/T017/T037/T038. It does not apply profiles to people, change grants or
activate policy. The broader tasks remain open.

## Authority and integration boundary

The internal commands take a server-supplied actor and organization, never UI
inputs carrying trusted authority. Their session-authenticated consumers follow
`permission-editor.md`; the original T053–T055 increment exposed none. Require policy
version 1, an active same-organization user and a strictly loaded permission state
with explicit Administrator identity. Legacy/future policy, missing state,
malformed grants and equivalent non-admin grants cannot authorize a command.
Only disposable tests set policy version 1; no activation path ships here.

The confirmed [Harvest permission guide](https://support.getharvest.com/hc/en-us/articles/44171549176077-Permissions)
supplies administrator-managed create/delete and the 50-profile limit. FR-032
supplies the approved naming comparison and FR-015/C01 supplies grant-preserving
deletion; no new product choice is needed. Replay/audit are Horae mechanics, not
claims about Harvest's internal implementation.

## Local lock order

Open a fresh READ COMMITTED, READ WRITE transaction with bounded transaction-local
waits under `permission-editor.md`. Lock organization FOR UPDATE before any other
row lock, then the current actor FOR SHARE through commit, canonical state and
request receipt. Do not issue user UPDATE locks or user/business-row DML, or lock
projects, assignments, invoices, approval or job rows.
For deletion lock the template, then affected person states ordered by user ID.
Detach provenance, delete the template, increment revisions and insert the durable
receipt/audit in the same transaction. No external work occurs under the lock.

Project finalization/editor paths now take the organization gate first (T068–T070).
Actor SHARE also remains compatible with already held actor SHARE and FK KEY SHARE;
PostgreSQL documents their compatibility in its
[row-lock table](https://www.postgresql.org/docs/17/explicit-locking.html#LOCKING-ROWS).
Existing legacy role/active mutations already take organization UPDATE first.
Migration 0042's affected state rows have no parent-writing project/user triggers.

This closes the local order for these isolated commands only, not T042 or live
coexistence with an activated policy. Any writer added for permissions/templates
must take the same gate and increment the organization access revision for every
real change; taking the gate alone does not fence a confirmed affected set.
User deactivation, assignment changes, profile application,
operator writers and other delivery paths keep their existing integration gates.

## Typed intent and atomic history

Each command carries request UUID, expected organization revision, and either:

- Create: proposed name and the complete confirmed grant selection at the current
  catalog version. Trim the name, validate FR-032, strictly validate the selection
  without adding prerequisites, and sort its identifiers for intent comparison.
- Delete: tenant-local template UUID and expected template revision. The confirmed
  organization revision fences the affected person set. Cancellation sends no command.

Authenticate before looking up a receipt or returning details. An exact canonical
intent retry returns its historical result before checking now-stale revisions
or requiring the deleted template to exist. Changed intent with the same key
conflicts. A revoked caller cannot replay or retrieve a privileged receipt.

Use a UUID-v7, org-scoped receipt containing principal identity, request UUID,
versioned typed canonical intent, typed historical result, timestamp and typed
before/after audit. User identity uses a composite tenant FK. Keep the separately
specified operator principal variant distinct and structurally exclusive; these
commands expose no operator entry point. Historical deleted-template IDs are
snapshot values, never restrictive/cascading template FKs. Store no credentials,
email, financial fields or unrelated configuration.

Create checks the expected organization revision and current count under the
exclusive gate; 50 existing templates rejects another. The unique name index is
the final conflict boundary. Insert the template at revision zero, advance the
organization revision once and persist its creation audit and receipt atomically.
No person state changes merely because a reusable template was saved.

Delete checks both expected revisions, loads and strictly validates affected
person states, then converts each source to individual while preserving exact
grants, catalog version and administrative identity. Increment each affected
person revision once, retain the previous source and unchanged grants/identity in
audit, and delete only the reusable template. Increment organization revision
once. No membership, management relationship, active status or business row changes.
Missing/foreign templates fail without a success receipt; no implicit same-name
replacement or reassignment. Every increment checks overflow before writes.

Audit, receipt or any state-write failure rolls back the complete transaction.
These two successful commands always create a real change; future unchanged
person saves retain their separately specified no-op rules. No denial audit is
fabricated inside a rolled-back success transaction.

## Required executable cases

| Case | Expected result |
| --- | --- |
| Create then load/replay | Trimmed display name, exact grants, one template/revision/audit; historical replay changes nothing |
| Non-admin, inactive, missing/foreign actor; legacy/future mode; invalid stored authority | Denied without state or privileged receipt disclosure |
| Blank/long name, incomplete/duplicate/unknown grants, stale revision | Atomic rejection, no normalization or successful audit |
| Equivalent names; two creators at count 49; 51st template | At most one valid concurrent change, count never above 50 |
| Delete with multiple adjusted assignees, including explicit admin identity | Exact grants/identity and business rows preserved, independent source detachment/revisions audited |
| Delete replay after original removal or same-name recreation | Original historical outcome; replacement and current person state untouched |
| Stale template/organization revision; foreign template; revision exhaustion | No partial detach/delete/revision or audit |
| Actor revocation or deactivation wins organization gate | Waiting command reloads authority and denies; no privileged replay |
| Audit insert forced to fail | Template/state/revisions roll back together |
| Existing actor SHARE lock held during command | Command completes without requesting a conflicting user lock |

Use production command helpers and real PostgreSQL fixtures. Authenticated
wrappers follow `permission-editor.md`. Full profile/assignment integration,
operator commands, audit browsing, browser confirmation and policy activation
remain separate work.
