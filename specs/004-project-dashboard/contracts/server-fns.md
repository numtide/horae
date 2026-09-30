# Contract: Dashboard Server Functions (read-only)

## Current activity contract

`get_project_activity(project_id: String, interval: Option<ProjectActivityInterval>)`
returns `ProjectActivity` from `server_fns/projects/activity.rs`. This increment
implements the time-series part only; the historical financial contracts below
remain pending reconciliation.

- Require an active session and current `project_read_access.can_view_progress`.
  An inaccessible or foreign project returns non-disclosing `NOT_FOUND`, before
  date-range/bucket validation. No progress access follows merely from own history.
- `Some({from,to})` uses inclusive work dates; `None` derives both bounds from
  actual recorded dates. Empty all-time has no invented interval or buckets.
- Use a repeatable-read/read-only transaction for authorization, interval and
  daily aggregate. Set a five-second statement timeout.
- Week boundaries follow the stored organization setting; reject invalid settings.
- Aggregate actual tracked minutes by original billable flag, without invoice
  rounding or entry notes. Include inactive historical projects/tasks/contributors.
- Return interval, week start and weekly billable/non-billable/cumulative minutes;
  no people, financial fields or private notes are part of this payload.
- Bound to 5,200 weekly buckets and reject excessive ranges before fetching daily
  values. The UI must explain the limit and request a shorter interval on error;
  it must not silently change the selected interval or truncate the series.
- Invalid/reversed ranges are `BAD_REQUEST`; invalid stored configuration and
  unexpected arithmetic/database failures are errors, not zero totals.
- No schema or mutation change. Tests use disposable PostgreSQL databases.

## Report task-filter extension

The existing `report_time` and `report_detailed` server functions accept an
additional `task_id: Option<String>`, parsed with the same strict UUID helper as
their other entity filters. CSV/XLSX export query parameters accept the same
optional task UUID. All five entity filters are conjunctive; a missing task means
all tasks, not a different permission scope. Detailed rows and CSV still share
one query; XLSX preflight applies the same task predicate before checking limits.
Unknown/foreign tasks produce no rows, and malformed UUIDs fail explicitly.
The current active-manager gates and organization/cost privacy stay unchanged.
No schema or financial-rule change is part of this extension.

## Report period extension

`report_time`, `report_detailed` and timesheet CSV/XLSX accept optional `from`
and `to`. Both absent means all recorded work dates, including future entries;
both present means an inclusive interval. Partial, empty, malformed or reversed
ranges return `BAD_REQUEST`, never an unbounded fallback. Parsing shares
`ActivityRange::parse_optional` in core. All entity predicates, current-manager
checks, currency/privacy rules and bounded-download limits remain unchanged.

Reports still defaults to the current month through today. Selecting **All time**
explicitly omits the dates, disables date editing and preserves the custom pair
for returning to it. Invalid custom dates hide stale rows and disable downloads.
This enables, but does not yet implement, the contextual Project Detail links.

> Historical contract, pending reconciliation with the 2026-09-29 specification.
> Its visibility and accounting assumptions predate feature 011; it is not the
> current implementation contract.

All new surfaces are Dioxus `#[server]` **read** functions in `crates/horae/src/server_fns/projects.rs`. They are session-authenticated like the rest of `server_fns` (`session_user_id().await?`), org-scoped, and issue **no mutations** (Constitution IV — the feature adds zero write paths). No Axum route, no Harvest v2 change. Money-visibility follows Horae's existing policy; where money is hidden the cents fields are returned as `None`.

Types below are the view models from `models/dashboard.rs` (see `data-model.md`). `ServerFnError` codes use the repo's named constants (e.g. `NOT_FOUND`), not integer literals.

## 1. `get_project_dashboard`

```rust
#[server]
pub async fn get_project_dashboard(project_id: String)
    -> Result<ProjectDashboard, ServerFnError>;
```

- **Purpose**: Header identity + budget/progress + hours totals + invoiced/uninvoiced money for one project (User Stories 1 & 3).
- **Auth**: any authenticated user who may open the project; org-scoped by `org_id`. `NOT_FOUND` if the project is not in the caller's org.
- **Reads**: `projects` + `clients` (identity/budget); a flat per-entry query over `time_entries` LEFT JOINing `project_tasks`/`assignments` and JOINing `users` (the `list_project_spend` shape plus `billable`, `invoice_id`), folded by `horae_core::project_rollup::fold_entries`; `SUM(invoice_line_items.amount_cents)` for lines whose entry's `project_id` matches → `invoiced_cents`.
- **Derivations**: `spent_minutes`/`total_minutes`/billable split and `spent_billable_cents` from the rollup; `progress_pct` from `progress_pct(spent, budget)` chosen by `budget_kind`; `uninvoiced_cents` = rollup `uninvoiced_billable_cents`; `money_incomplete` = rollup `unresolved_rate_minutes > 0`.
- **Guarantees**: `spent_minutes` and `spent_billable_cents` equal the same project's `list_project_spend` values exactly (SC-002 — same shared fold). Money fields `None` when the viewer may not see money.

## 2. `list_project_breakdowns`

```rust
#[server]
pub async fn list_project_breakdowns(project_id: String)
    -> Result<ProjectBreakdowns, ServerFnError>;
```

- **Purpose**: Per-task and per-person breakdown tables (User Story 2).
- **Reads**: the **same** per-entry query as (1), folded once; `by_task`/`by_person` buckets joined to `tasks.name` / `users.name` for labels.
- **Guarantees (SC-003)**: `Σ by_task.total_minutes == Σ by_person.total_minutes == dashboard.total_minutes`; `Σ by_task.billable_cents == Σ by_person.billable_cents == dashboard.spent_billable_cents`. Rows sorted by hours desc (label asc tie-break). Empty vectors for a project with no time.

> Implementation note: (1) and (2) fetch the identical row set. They MAY be exposed as two functions (called by two `use_resource`s) or one combined function; either way both derive from a single `fold_entries` result so they cannot disagree. Splitting keeps each server fn small and lets the header render before the tables.

## 3. `list_project_enabled_tasks`

```rust
#[server]
pub async fn list_project_enabled_tasks(project_id: String)
    -> Result<Vec<EnabledTaskRow>, ServerFnError>;
```

- **Purpose**: Enabled-tasks section — each `project_tasks` row with its `billable` flag and `rate_cents` (User Story 4, FR-011).
- **Reads**: `project_tasks JOIN tasks` for the project, ordered by task name. `rate_cents` is `None` where unset (rendered "—"); returned as `None` for viewers who may not see money.
- Distinct from the existing `list_project_tasks` (which returns bare `Task`s for entry pickers) because the dashboard needs the per-project `billable`/`rate_cents` override columns.

## 4. `list_recent_project_entries`

```rust
#[server]
pub async fn list_recent_project_entries(project_id: String, limit: i64)
    -> Result<Vec<RecentEntryRow>, ServerFnError>;
```

- **Purpose**: Newest-first activity feed (User Story 4, FR-012).
- **Reads**: `time_entries JOIN users JOIN tasks` for the project, `ORDER BY spent_date DESC, created_at DESC LIMIT $limit`. `limit` is clamped to a sane cap server-side (default/typical 10); `note_snippet` truncates `notes`.

## 5. Reused, unchanged

- **`list_assignments(project_id)`** — the team section (already rendered on the page today).
- **`list_project_spend()`** — refactored internally onto `horae_core::project_rollup::fold_entries`; its signature and result are unchanged, but it now shares the fold with the dashboard so figures cannot drift.
- **`get_me()`** — money-visibility / role gating, as elsewhere.

## Error & edge behavior

- Unknown/foreign-org `project_id` → `NOT_FOUND`.
- Malformed `project_id` → the existing `parse_uuid` error path.
- No time / no tasks / no assignments / no invoices → success with zero totals and empty vectors (FR-015); never an error, never a divide-by-zero.
- Over budget → `progress_pct` clamped to 100; remaining computed by the UI may be zero/negative (conveys overage).
