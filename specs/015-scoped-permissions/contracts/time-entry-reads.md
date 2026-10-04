# Scoped time-entry reads (OP03)

This implements the confirmed read portion of FR-006/007/008/010/018, not time
editing, approval, timer ownership or policy activation. Harvest's current
[permission reference](https://support.getharvest.com/hc/en-us/articles/44171549176077-Permissions)
(checked 2026-10-04) distinguishes own, managed people/projects and all time
entries. Its role defaults do not replace effective grants.

## Boundary

- `list_visible_time_entries(query)` derives organization and actor from the
  session. Policy 1 and an active actor with valid stored grants are required;
  policy 0 is forbidden and unsupported/missing permission state is unavailable.
- `TimeReadOwn` admits own rows; `TimeReadManaged` admits rows whose person OR
  project is explicitly managed by the actor; `TimeReadAll` admits all rows in
  the organization. These sets combine without duplicates. Ordinary membership,
  unrelated grants, legacy roles and the administrator flag confer no scope.
- Qualify the entry, person, project, task and client by organization. Historical
  entries remain visible when their person/project/task is archived; current
  task tracking restrictions do not remove historical read authority.
- Return explicit entry facts, including notes, integer minutes, persisted
  rounded minutes, state, timer/calendar placement and creation ordering, plus
  contextual person/project/task/client IDs and names. Do not return rates,
  costs, currency/financial totals, invoice identity, user email, authentication
  metadata or private project notes. These labels do not grant directory access.

## Query and delivery

- Require inclusive `date_from` and `date_to`, ordered from earliest to latest.
  Optional person/project IDs only narrow authorized rows; a nonexistent,
  foreign or invisible filter yields the same empty result.
- Use descending `(spent_date, created_at, id)` keyset order, at most 500 rows
  and an explicit next cursor. The cursor is an exclusive ordering bound, not
  an identity lookup, permission assertion or reusable authorization snapshot.
  Reject a cursor whose date is outside the requested range.
- The page size is a Horae transport bound, not a claimed Harvest product limit.
  No arbitrary maximum historical date interval is introduced. There are no
  partial-page totals: consumers must exhaust pages before claiming full-period
  totals. Each page reauthorizes; concurrent entry edits are not a cross-page
  repeatable snapshot.
- Reuse organization SHARE then active-actor SHARE locks and bounded READ
  COMMITTED transactions. Existing permission/relationship writers conflict
  through the organization lock; direct account deactivation conflicts through
  the actor lock. Commit after materializing the authorized projection.
- Preserve current legacy readers and consumers until reviewed all-surface
  cutover. Reading another person's running entry never permits stopping it.

## Acceptance

Prove six-profile and custom-grant scope, person/project union without duplicate
rows, foreign and malformed parent exclusion, archived history, date/filter and
keyset boundaries, populated sensitive-field omission, policy/state/actor denial
and revocation ordering. Exercise the registered session endpoint as well as
database transactions. Full shell/Timesheet integration remains a separate
required delivery step, not something this reader alone completes.

## Existing time-entry response boundary (T168–T170)

The legacy `TimeEntry` response is shared by lists, current timer, start/stop,
create/update and rescheduling. No page or component reads its `invoice_id`;
Timesheet uses `state` to distinguish locked entries. FR-008 therefore permits
removing this unused invoice identity from every serialization of the model,
including null placeholders, without changing any action grant or business lock.
Keep the database field and SQLx decoding intact for internal billing checks.
Ignore incoming invoice identities during model deserialization: the browser
cannot reconstruct billing authority by supplying a hidden field. Preserve all
other response facts. Invoice/report and Harvest-compatibility projections are
separate types and remain unchanged.

Verify the real own-entry response with a populated invoice link for each legacy
role, session-only identity, exact field omission and unchanged stored relation.
Exercise shared serialization/deserialization and existing timer/invoice tests.
This repair does not activate canonical policy or finish Timesheet integration.

## Timesheet integration findings

The existing `TimesheetContent` fetches an entire week and sums it in Day, Week
and Calendar views. Its name maps come from the actor's tracking catalogs, not
the selected teammate's authorized entries. Its grid, dialog, drag/reorder,
delete, timer and submission callbacks all assume the session user. Wiring the
new list into these callbacks alone would be incorrect: selected-person context,
read-only affordances, complete-period loading and independently authorized
writes must be integrated together. Do not fabricate full `TimeEntry` records
from the minimal scoped projection or fall back to legacy reads after denial.

The handoff `04_Timesheet.dc.html` shows own-time entry controls but no teammate
selector. Harvest's [other-person timesheet guide](https://support.getharvest.com/hc/en-us/articles/360048687591-How-to-view-edit-and-submit-another-person-s-timesheet)
documents a Teammates selector and a return-to-own-timesheet link. Its legacy
role wording does not establish the exact candidate set for new custom grants,
archived people or people without entries; that boundary still needs evidence.
Its submission flow can include expenses, so the current time-only submit action
must not be presented as complete combined-submission parity.
