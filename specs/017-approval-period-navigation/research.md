# Research: Approval Period Navigation

All findings below were taken read-only at `8a796361`.

## R1 — Week membership is exact `period_start`

- **Decision**: a week lists the approvals whose `period_start` equals the
  selected week's first day.

- **Rationale**: `server_fns/approvals.rs::submit_user_week` is the only
  non-test writer of `approvals`. It rejects a start that is not the
  organization's first weekday and sets `period_end = start + 6`. Nothing
  outside tests updates `organizations.week_start`. Its only writers are:

  - the column default (migration `0001`);
  - `init.rs`, which uses the default;
  - `seed.rs`, which writes 1.

  No importer, CLI command or migration inserts approvals. So every supported
  row aligns with exactly one week.

- **Alternatives considered**: matching by overlap was rejected. If a row were
  ever misaligned, overlap would put dates outside the selected week into
  "Approve visible", contrary to 015 FR-009 ("grouping MUST NOT widen" the
  selection).

## R2 — Default week

- **Decision**: with no usable week in the address, show the week containing
  today, aligned to the organization's first weekday. No indicator for pending
  work in other weeks.
- **Rationale**: a read-only Harvest observation on 2026-10-28 (root checkout
  `.scratch/harvest-approval-default-20261028.md`) found that a bare
  `/approvals` opens the current week, even after the previous week had been
  viewed.
- **Today's date**: taken from `chrono::Utc::now().date_naive()`, as in
  `pages/timesheet.rs` (`Anchor::default`) and `pages/reports.rs`. No new
  timezone policy is introduced.
- **Alternatives considered**: remembering the last week, opening the oldest
  pending week, or showing an older-pending badge. Harvest showed none of these
  in the observed (empty) case, and none is in the approved slice. Whether
  Harvest has such a badge is unverified.

## R3 — Week value in the address

- **Decision**: `/approvals?:week`, with `week: String` on `Route::Approvals`.
  The page parses `YYYY-MM-DD`. An empty or unparseable value means today, and
  any date is normalized to its week. Prev/This week/Next push the canonical
  week start.
- **Rationale**: this follows the existing query-parameter routes
  (`/timesheet/:view/:date?:span&:user`, `/projects/:id/edit?:..return_to`).
  The only caller is `components/sidebar.rs`, and `SideLink` matches the
  variant, not the URL (`route::route_is_active`).
- **Alternatives considered**:
  - Reusing the timesheet `Anchor` type would couple the approvals page and
    its test harness to `pages/timesheet.rs`.
  - Keeping the week in a signal only was rejected because refresh and sharing
    would lose it (SC-004).

## R4 — Week bounds near the representable range

- **Decision**: add one pure helper to `horae-core`:
  `week::week_bounds(date, first_day) -> Option<(NaiveDate, NaiveDate)>`. It
  returns the aligned start and start + 6 days, or `None` when either is not
  representable. The page uses it. The server validates an already aligned
  start with the same `checked_add_days(6)`, after the weekday comparison it
  shares with `submit_user_week`.
- **Rationale**: `week::week_start` already returns `Option` for underflow, but
  nothing checks that the seventh day exists. The submission path does its own
  `checked_add_days(6)`. Keeping the check in core gives it unit tests with no
  I/O (Constitution II).

## R5 — A response for a week that is no longer selected

- **Decision**: the list resource returns `(week_start, result)`. The page
  renders rows only when the returned start equals the selected start; otherwise
  it shows the loading state.
- **Rationale**: this makes FR-010 hold whether or not the resource framework
  keeps its previous value while it restarts. Nothing depends on Dioxus
  internals.

## R6 — Repeated actions and the bulk count

- **Decision**:
  - One page-level `busy` flag disables every Approve, Reopen and bulk button
    while any action is in flight.
  - The bulk ids are taken from the rows shown for the selected week.
  - `approve_submissions` already returns the count it approved. The page shows
    "Approved N of M timesheets", tagged with its week so it is not shown
    under another week.
- **Rationale**: this is the smallest change that prevents a double submit and
  reports the real result. The server already skips non-submitted ids, so it
  stays the authority on what was approved.

## R7 — Test harness

- **Decision**: extend `tests/approval_labels_ui.rs`, which compiles the real
  `pages/approvals.rs` against stub server functions. Add stubs for
  `get_week_start` and a minimal `route` module with a `Routable` `Route`
  holding only `Approvals { week: String }`, so the page's navigation compiles.
  A `Router` with in-memory history hosts the page when a test clicks
  Prev/Next.
- **Risk**: the repository has no existing in-memory-router test. If the
  Dioxus 0.7 router cannot be driven this way, assert navigation through the
  route the click builds and cover the click itself in the native check. Record
  any such deviation in tasks.md.

## R8 — Server listing helper

- **Decision**: move the body of `list_approvals` into
  `list_week(pool, manager, status, week_start)` so `#[sqlx::test]` cases can
  call it directly, as `approve_periods` and `reopen_period` are called today.
  The week is validated against `organizations.week_start` in the same text as
  `get_week_start`'s query, so the existing `.sqlx` descriptor is reused.
- **Ordering**: within one week every row has the same `period_start`, so the
  existing `ORDER BY a.period_start DESC` no longer orders anything. Use
  `ORDER BY u.name, a.id` for a stable table.
