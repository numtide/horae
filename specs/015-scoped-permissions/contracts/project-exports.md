# Materialized project export authorization

T107–T109 refine US3 / FR-006/007/010/017/018 and SC-006. This integrates
current legacy project scope into XLSX materialization and response release;
it does not activate canonical grants or complete CSV streaming acceptance.
Reuse migration 0035's `project_read_access` predicate without broadening or
narrowing it. No new Harvest behavior or product decision is inferred.

## Loading boundary

Start explicit READ COMMITTED, READ WRITE before any query. Apply existing
five-second statement and ten-second idle-transaction deadlines, preserving
stricter inherited lock timeouts. Lock organization SHARE, then the active
same-tenant actor SHARE. Missing organization/actor or inactivity denies with
403; an active Member with no matching projects receives the existing empty
spreadsheet, not an invented Manager requirement.

After those waits, one SQL statement computes size and payload from the same
snapshot. A limited MATERIALIZED CTE applies tenant, current project visibility
and the existing active/budgeted/archived filter before taking at most 10,001
rows. Compute the existing row, aggregate-byte and field-byte limits from it.
Return payload only when all limits pass, otherwise a stats-only sentinel.
Check stats before decoding nullable fields; nullable project ID distinguishes
the empty sentinel from a valid row with absent code/budget. Oversized strings
must not cross the database wire into application memory.

Preserve exact integer budgets, seven spreadsheet columns, filter fallbacks and
`client_name, name, id` ordering. Capture each project ID privately; never add it
to spreadsheet columns. Commit before rendering. READ COMMITTED is intentional:
the payload statement sees completed scope gains/losses and newly finalized
assigned projects after the organization wait, even when legacy writers do not
advance `organizations.access_revision`. Plain multi-query READ COMMITTED size
and payload reads are not acceptable.

## Release boundary

After bounded rendering, acquire the same fresh organization/actor boundary.
Lock every distinct captured project parent SHARE in UUID order within the
original tenant. Then, in a separate statement, require current
`project_read_access.can_view_progress` for every captured ID. Missing, deleted
or inaccessible projects deny the whole body with 403. Empty exports still
require the active tenant-bound actor. A demoted Manager who retains access to
every captured project may still download; profile-name ordering is not a scope
check. Release checks do not change the previously materialized business values.

Do not merge the parent lock and relationship check into one statement: the
latter must see child changes committed during a parent-lock wait. Migration
0039's assignment/settings triggers update the parent, so a winning direct
relationship mutation is visible after that wait. No child-row locks are taken.
Production assignment/editor/finalization writers already acquire the conflicting
organization access-change gate. No database lock spans rendering, a body
lifetime or browser backpressure. Authorization is at response release, not a
promise to recall already delivered bytes.

## Verification

- Active Manager/Admin, assigned Member, `lead`/`admin`, absent settings and
  manager-only visibility; inactive/foreign/missing actors and historical-only
  identity. Tenant and filtering precede all size checks.
- Winning assignment addition/removal, visibility expansion/restriction,
  missing-settings insertion and new assigned-project finalization. Actor
  revocation after either organization or actor wait cannot disclose payloads.
- Rendering allows concurrent revocation; release checks every captured ID.
  A parent-lock wait followed by direct child revocation denies. Reader-first
  locks delay that commit only until authorization completes, not body delivery.
- Empty, exactly 10,000 and 10,001 rows; multibyte fields and aggregate limits;
  coherent size/payload under concurrent edits, unchanged ordering and columns.
- Cancellation, timeout, inherited isolation/read-only defaults, single-pool
  reuse and both admission permits; actual session-authenticated HTTP delivery.
- Full server/export regressions, complete SQLx cache, offline Clippy and WASM,
  formatting, adversarial review and scoped analysis before publication.

CSV retains its separately reviewed streaming contract until its own integration;
this materialized design must not add locks to browser-paced streams.
