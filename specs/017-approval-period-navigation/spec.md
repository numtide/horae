# Feature Specification: Approval Period Navigation

**Feature Branch**: `feat/approval-period-navigation`

**Created**: 2026-10-28

**Status**: Draft

**Input**: User description: "Weekly approval period identity and navigation on the existing Approvals page. Bounded child of the 001 approval exclusion and the 015 approval contracts."

## Scope and parent contracts

This is a narrow refinement of the **existing weekly** approval surface. It
makes every approval row identify its work period, lets a manager move between
weeks, and keeps every total and action limited to the week being shown.

- [001 spec](../001-time-tracking-invoicing/spec.md), Assumptions: the existing
  approval surface is "neither required nor removed" by 001. This feature
  refines that surface and does not introduce a new approval workflow.
- [015 approval coverage](../015-scoped-permissions/contracts/approval-coverage.md)
  owns the target flexible-approval lifecycle (custom ranges, project portions,
  empty-cell coverage, scoped withdrawal, U-AC08). None of it is decided,
  activated or relied on here. Its replacement map stays valid. This feature
  changes how approvals are read and displayed, not what `submit`, `approve` or
  `reopen` write.
- [015 approval visibility](../015-scoped-permissions/contracts/approval-visibility.md)
  (user-approved FR-024) and the approved FR-009 rule ("grouping MUST NOT widen
  [the selection]") constrain this feature: a selected week must never widen
  into dates outside it.

### Out of scope

Day/Month/custom period units; flexible or custom-range submission; partial
project/person approval; client, project, role or teammate filters; withdrawal
semantics, or renaming or changing reject/reopen; expenses; the FR-024 guard;
activating the scoped-permission policy; cross-person timesheet links (they
exist only under the inactive scoped policy); full design parity with
`design/project/app/06_Approvals.dc.html`.

## Clarifications

The membership rule below rests on code evidence. The owner's answers are
recorded under the session that follows it.

- Week membership: an approval belongs to the selected week exactly when its
  period starts on that week's first day. Evidence at `8a79636`, which holds
  for rows written since `a55b3a47` (2026-09-09):

  - `submit_user_week` is the only non-test writer of `approvals`. It
    validates that the start is the organization's first weekday and that the
    end is start + 6.
  - Nothing outside tests changes `organizations.week_start`. Its writers are
    the column default, `init` (default), `seed` (1) and the branding update
    (not that column).
  - No importer, CLI command or migration inserts approvals.

  Before `a55b3a47`, `submit_week` accepted any start date (with end = start +
  6), and no migration normalizes those rows. A database that took submissions
  then may hold approvals whose start is not the organization's first weekday.
  Exact matching shows such a row under no week, so it cannot be approved or
  reopened from this page. The owner made a read-only check for such rows a
  mandatory deployment gate (see Session 2026-10-28 and quickstart.md,
  "Deployment gate"). No overlap, normalization or migration is adopted.

  Overlap matching is not used: it could include dates outside the selected
  week in a bulk approval (015 FR-009).

### Session 2026-10-28

- Q: Which week opens when the address names none (or an unparseable one), and
  are pending approvals in other weeks surfaced? → A: The current week (the
  week containing today), aligned to the organization's first weekday. No
  indicator for pending approvals in other weeks and no oldest-pending default
  are added; a manager reaches other weeks with previous/next.
  - Basis: a read-only 2026-10-28 observation in the owner's Harvest account.
    Bare `/approvals` redirected to the current week (26 Oct – 1 Nov 2026, Week,
    Pending Approval, Person). After stepping to the previous week, a second
    bare visit again opened the current week, not the remembered one.
  - Evidence: notes and snapshots recorded locally during that observation.
    They hold account identity and are not kept in the repository.
  - Limitation: the observed current week was empty. Whether Harvest shows an
    indicator for older pending work is **unverified**. This feature neither
    adds one nor claims Harvest lacks one.
- Q: Must deployment require zero misaligned historical approvals, or must this
  feature handle them? → A: "Sí, exigir esa comprobación antes del despliegue"
  ("Yes, require that check before deployment"). Immediately before deploying,
  run the read-only check for approvals whose start is not the organization's
  first weekday or whose end is not start + 6.
  - Any returned row means: do not deploy, and keep the previous version
    running.
  - No migration, normalization or other data change is made, whatever the
    check finds.
  - The check is a deployment gate. It has not been run against any real
    database.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Know which week each approval covers (Priority: P1)

A manager reviewing approvals sees, on every row, the exact dates of the work
period that row covers. Two rows for the same teammate in different weeks are
unmistakably different.

**Why this priority**: Without the period, a manager cannot know what they are
approving or reopening. This is the core MVP defect.

**Independent Test**: Render the Approvals page with two submissions for one
teammate in consecutive weeks. Each row shows its own first and last date.

**Acceptance Scenarios**:

1. **Given** a submitted week 13–19 July 2026, **When** the manager views that
   week, **Then** the row shows "13 Jul – 19 Jul 2026" next to the teammate.
1. **Given** a week that spans two years, **When** it is displayed, **Then**
   both dates carry their year: "28 Dec 2026 – 03 Jan 2027", in the pager
   and on rows. The same-year form "13 Jul – 19 Jul 2026" matches the
   timesheet pager.

______________________________________________________________________

### User Story 2 - Move between weeks (Priority: P1)

The manager steps to the previous or next week, or returns to the current
week. The selected week is part of the page address, so a refresh or a shared
link opens the same week.

**Why this priority**: Pending work from different weeks must be reviewable
one week at a time. Today the page lists every returned week's rows together
without dates.

**Independent Test**: Open the page, step back one week, refresh. The same
week is shown, and only that week's approvals are listed.

**Acceptance Scenarios**:

1. **Given** today is 28 Oct 2026 in a Monday-start organization, **When** the
   manager opens Approvals with no week, or an unparseable one, in the address,
   **Then** "26 Oct – 01 Nov 2026" is shown and labelled as this week.
1. **Given** a different week is shown, **When** the manager selects the
   current week, **Then** "26 Oct – 01 Nov 2026" is shown and the address names
   2026-10-26.
1. **Given** the shown week, **When** the manager selects previous/next,
   **Then** the week 7 days earlier/later is shown and the address changes to it.
1. **Given** an address naming any date, **When** opened, **Then** the week
   containing that date is shown (the address names the week's first day).
1. **Given** an organization whose week starts on Sunday, **When** navigating,
   **Then** every shown week starts on a Sunday.

______________________________________________________________________

### User Story 3 - Totals and actions apply only to the shown week (Priority: P1)

The total-time card, the pending count and "Approve visible timesheets" cover
exactly the approvals listed for the shown week. A row action affects only
that row's period.

**Why this priority**: The current table shows every returned row, but
without dates. "Approve visible timesheets" therefore approves every listed
pending week, and the manager cannot tell those weeks apart. The defect is
the scope of a mutation, not just its presentation.

**Independent Test**: With pending submissions in two adjacent weeks, approve
visible on one week. The other week stays pending and its entries are
unchanged.

**Acceptance Scenarios**:

1. **Given** pending weeks W and W−1, **When** the manager approves visible on
   W, **Then** only W becomes approved. W−1 and its entries are unchanged.
1. **Given** approved weeks W and W+1, **When** the manager reopens W's row,
   **Then** only W's entries reopen. W+1 remains approved and locked.
1. **Given** shown rows of 7:30 and 2:15, **Then** the total card shows 9:45,
   the exact integer-minute sum of the shown rows.
1. **Given** three shown pending rows, one of which stops being pending before
   bulk approval commits, **Then** the page shows "Approved 2 of 3
   timesheets", using the count the server returned (FR-013).

______________________________________________________________________

### User Story 4 - Safe loading, errors and denials (Priority: P2)

The page behaves predictably while loading, on errors, with invalid addresses,
under rapid navigation and on repeated clicks.

**Independent Test**: Exercise each edge case below against a controlled
response harness.

**Acceptance Scenarios**:

1. **Given** a non-manager, **When** opening Approvals for any week, **Then**
   the existing access-required message is shown and no approval data is
   requested or displayed.
1. **Given** the organization's first weekday cannot be loaded, **Then** an
   error is shown and no approval list is requested for a guessed week.
1. **Given** the manager navigates quickly from W to W+1, **When** W's response
   arrives after W+1 is selected, **Then** W's rows are never shown under W+1's
   label.
1. **Given** an action is in flight, **Then** no approve or reopen button can be
   pressed again until it completes. The action affects only the approvals it
   was pressed for, and afterwards the list reloads for the week then
   selected.
1. **Given** a week whose first or last day is not representable, **When**
   navigation would reach it, **Then** that direction is disabled, and an
   address that names it shows an inline error without requesting approvals.

### Edge Cases

- Unparseable week in the address: treated like a missing week (the current
  week), without a server error.
- Week at the calendar boundary (first or last representable date, or a week
  whose seventh day overflows): an inline error, no request, no panic.
- Pending approvals in other weeks: not indicated (see Clarifications). The
  status filter and previous/next are the way to reach them.
- A week-start date that is not the organization's first weekday, sent
  directly to the server: rejected as a bad request (as submission already
  does), never silently widened.
- Status filter (Pending/Approved/All) combined with the week: both apply.
- Navigation does not change what counts as "today": the existing UTC date
  convention of the timesheet and reports pages is reused.
- Historical misaligned approval (written before `a55b3a47`): it would appear
  under no week. The page does not handle it. Instead, the deployment gate in
  Clarifications blocks deployment while any such row exists.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Every approval row MUST display its period's first and last date.
- **FR-002**: The Approvals page MUST show exactly one week at a time. That
  week is defined by its first day, which is aligned to the organization's
  first weekday.
- **FR-003**: The selected week MUST be part of the page address. Any valid
  date selects the week containing it. A missing or unparseable value selects
  the current week (FR-012).
- **FR-004**: The manager MUST be able to move to the previous week, the next
  week and the current week.
- **FR-005**: The approval list for a week MUST contain exactly the approvals
  whose period starts on that week's first day, within the manager's
  organization, filtered by the chosen status.
- **FR-006**: The server MUST reject a requested week start that is not the
  organization's first weekday, or whose last day is not representable.
- **FR-007**: The total, billable and non-billable figures and the pending count
  MUST be the exact integer sums over the shown rows only.
- **FR-008**: "Approve visible timesheets" MUST submit only the pending
  approvals of the shown week, and MUST report the number actually approved.
- **FR-009**: Approve, bulk-approve and reopen MUST keep their existing
  authorization checks, state transitions, locks, invoiced-entry preservation
  and plugin events. Only which ids the page offers changes.
- **FR-010**: Responses for a week that is no longer selected MUST NOT be
  displayed. Action controls MUST be disabled while their action is in flight.
- **FR-011**: Non-managers MUST see the existing denial and MUST NOT trigger
  approval reads.
- **FR-012**: With no usable week in the address, the page MUST show the
  current week: the week containing today's date (existing UTC convention),
  aligned to the organization's first weekday. The page MUST NOT add an
  indicator for, or default to, pending approvals in other weeks.
- **FR-013**: After "Approve visible timesheets", the page MUST show
  "Approved N of M timesheets", where N is the count the server returned and M
  is the number of pending approvals it submitted.

### Key Entities

- **Approval period**: an existing approval row's person, first date and last
  date (first + 6 days), plus its state. It is not changed by this feature.
- **Selected week**: the first day of the week the page shows. It is derived
  from the address and the organization's first weekday.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In a fixture with three weeks of submissions for one person, a
  manager identifies the correct week's row on first view in 100% of trials.
- **SC-002**: Bulk approval from any shown week changes zero approvals or
  entries outside that week (verified on adjacent-week fixtures).
- **SC-003**: For every shown week, the displayed total equals the exact sum of
  the shown rows' minutes, with zero rounding difference.
- **SC-004**: Refreshing or sharing the page address reproduces the same week
  in 100% of cases.

## Assumptions

- The weekly storage (one approval row per person per week start) remains as
  is. 015 may replace it later, and this feature adds no migration.
- The organization's first weekday has no supported setter today (defaults:
  Monday). The design must still honour any stored value.
- "Today" uses the existing UTC date convention of the timesheet and reports.
- Existing shared styles (the timesheet pager) are reused. No new shared CSS
  is planned.
- Harvest's approvals API returns inclusive date-range buckets. Its support
  article ([Submitting and approving timesheets](https://support.getharvest.com/hc/en-us/articles/360048181832-Submitting-and-approving-timesheets))
  describes period-based review. Neither defines a rule for historical
  misaligned rows. Horae can no longer create them, but older databases may
  hold some. Per the owner (see Clarifications), deployment requires the
  read-only check to return none, and no data rule is adopted.
- Harvest default navigation (observed 2026-10-28, see Clarifications): a bare
  visit opens the current week with Pending Approval, which matches Horae's
  existing default status. Harvest's indicator for older pending work, if any,
  remains unverified. So do Harvest's behaviour for other roles, invalid
  addresses and time zones.
