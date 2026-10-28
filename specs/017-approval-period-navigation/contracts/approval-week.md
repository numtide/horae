# Contract: one approval week

This covers `Route::Approvals`, `pages/approvals.rs` and `list_approvals`.
Anything not listed here keeps its behaviour at `8a796361`.

## Address

| Input `?week=` | Selected week start |
|----------------|---------------------|
| absent, empty or not `YYYY-MM-DD` | `week_bounds(today_utc, first_day).0` |
| any valid date D | `week_bounds(D, first_day).0` (no redirect) |
| D whose week is not representable | none: inline error "This week is outside the supported date range.", no list request, no pager; only a "This week" link |

Prev, This week and Next push `/approvals?week=<canonical start>`.

- For a representable week, Prev is disabled when the start minus 7 days has
  no representable week.
- For a representable week, Next is disabled when the start plus 7 days has no
  representable week.
- The sidebar link targets `Route::Approvals { week: String::new() }`.

## Server: `list_approvals(status: Option<String>, week_start: String)`

Checks run in this order, after `require_manager()` (403 for non-managers, as
today). The first failure returns:

1. `week_start` is not `YYYY-MM-DD`: `BAD_REQUEST`, "Invalid week_start (use YYYY-MM-DD)". This is mapped locally; the shared `parse_date` returns `INTERNAL_ERROR`, and it is not changed.
1. `week_start`'s weekday is not the organization's first weekday: `BAD_REQUEST`, "week_start must match the organization's first weekday". This uses the same numeric comparison as `submit_user_week`.
1. `week_start + 6 days` is not representable: `BAD_REQUEST`, "week_start is out of range". For an aligned start this equals `week_bounds(week_start, first_day)` being `None`.
1. `status` is not an entry state: the existing `parse_enum` error, unchanged.

Result rows are `a.org_id = manager.org_id AND a.period_start = week_start`,
filtered by the optional `status`, ordered by `u.name, a.id`. Each row keeps the
existing `ApprovalSummary` fields and its exact integer `total_minutes` and
`billable_minutes`. There is no overlap matching and no widening.

`approve_submission`, `approve_submissions` and `reject_submission` are
unchanged, including:

- `require_manager` and `ensure_can_approve` / `can_transition`;
- the org filter;
- the Submitted→Approved state checks;
- the whole-period reopen and invoiced-entry preservation;
- plugin events.

## Page states

| Condition | Shown | Requests |
|-----------|-------|----------|
| `get_me` loading | "Loading…" | no approval list |
| not manager | existing "Manager or admin access is required…" card | no `list_approvals` |
| manager, `get_week_start` loading | "Loading…" | no `list_approvals` |
| manager, `get_week_start` failed | error alert with its message | no `list_approvals` |
| manager, week unrepresentable | the inline error and "This week" link above, no pager | no `list_approvals` |
| list response for a different start than selected | "Loading…" (never the other week's rows) | the selected week's request |
| list loaded, empty | "No approvals found." | — |
| list loaded | the total card, rows, and bulk bar for that week | — |

The pager reuses the timesheet pager classes (`ts-pager`, `ts-pager-btn prev/next`,
`ts-pager-range`). Its label reads "This week" when the selected start equals
today's week start, and "Week" otherwise. The range text uses the
`period_label` format below. No new CSS.

## Rows and totals

- **Period label**: `period_label(start, end)`. When the years are equal it
  renders "%d %b – %d %b %Y" (e.g. "13 Jul – 19 Jul 2026"). Otherwise it
  renders "%d %b %Y – %d %b %Y". It appears on every row, next to the teammate,
  and in the pager.
- **Totals**: total, billable, non-billable and the pending count are integer
  sums over the rendered rows only.

## Actions

- **Busy**: while any action is in flight, every Approve, Reopen and "Approve
  visible timesheets" button is `disabled`. The flag clears when the action
  resolves (success or error), then `refresh` reloads the selected week.
- **Bulk ids**: exactly the `Submitted` rows rendered for the selected week. The
  page submits them, and they are never recomputed from another response.
- **Bulk result**: after `approve_submissions` returns `n` for `m` submitted
  ids, the page shows "Approved n of m timesheets" (`alert-success`), keyed to
  that week's start. It is hidden under any other week and cleared when the next
  action starts.
- **Errors**: action errors keep the existing `alert-danger` behaviour.
