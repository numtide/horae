# Phase 1 Data Model: Project Detail Dashboard

## Schema change

**None.** This feature is read-only. No new columns, tables, enum values, or migrations (FR-016, SC-007). It reads existing tables from `0001_init.sql` and `0002_invoices.sql`:

`projects`, `clients`, `tasks`, `project_tasks`, `assignments`, `users`, `time_entries`, `invoices`, `invoice_line_items`.

The `.sqlx/` offline cache is regenerated for the new read queries (`cargo sqlx prepare --workspace -- --features server --all-targets`) — a cache refresh, not a schema change.

## Existing columns consumed (no change)

| Table | Columns used | Role |
|---|---|---|
| `projects` | `id, client_id, code, name, project_type, currency, starts_on, ends_on, budget_kind, budget_amount_cents, budget_minutes, active` | Header + budget |
| `clients` | `id, name` | Header (client name) |
| `time_entries` | `project_id, user_id, task_id, minutes, billable, invoice_id, spent_date, notes, created_at` | All hours/amount aggregation + recent feed |
| `project_tasks` | `project_id, task_id, billable, rate_cents` | Enabled-tasks section + cascade step 1 |
| `assignments` | `project_id, user_id, role, rate_cents` | Team section + cascade step 2 |
| `users` | `id, name, billable_rate_cents` | Person names + cascade step 3 |
| `tasks` | `id, name` | Task names |
| `invoice_line_items` | `time_entry_id, amount_cents` | Invoiced amount |

(`users.cost_rate_cents` exists but is intentionally **not** read in v1 — deferral D-003.)

## Core domain (`horae-core`, new `project_rollup.rs`) — pure, unit-tested

The correctness-critical fold. No sqlx/axum/dioxus (Constitution II). SQL fetches raw per-entry rows; this module resolves each rate and aggregates into totals + groupings, reusing `invoice::resolve_rate` and `invoice::line_amount_cents` so the math is identical to invoicing and the current `list_project_spend`.

### Input row (one per time entry, as fetched)

```
EntryRow {
    task_id: Uuid,
    user_id: Uuid,
    minutes: i32,
    billable: bool,
    invoiced: bool,            // time_entries.invoice_id IS NOT NULL
    task_rate_cents: Option<i64>,       // project_tasks.rate_cents
    assignment_rate_cents: Option<i64>, // assignments.rate_cents
    user_rate_cents: Option<i64>,       // users.billable_rate_cents
}
```

### Output aggregate

```
Rollup {
    total_minutes: i64,
    billable_minutes: i64,
    nonbillable_minutes: i64,
    billable_cents: i64,          // Σ line_amount_cents(resolved_rate, minutes) over billable rows
    uninvoiced_billable_cents: i64, // same, but only rows where !invoiced
    unresolved_rate_minutes: i64, // billable minutes whose rate was None at every level (flag for "—")
    by_task: Map<Uuid, Bucket>,
    by_person: Map<Uuid, Bucket>,
}

Bucket {           // per task or per person
    total_minutes, billable_minutes, nonbillable_minutes, billable_cents
}
```

### Functions (intent)

| Function | Signature (intent) | Rules |
|---|---|---|
| `fold_entries` | `impl Iterator<Item = EntryRow> -> Rollup` | Single pass; for each row add `minutes` to total and to billable/non-billable by `billable`; for billable rows resolve rate via `resolve_rate(task, assignment, user)`, add `line_amount_cents(rate, minutes)` to `billable_cents` (and to `uninvoiced_billable_cents` when `!invoiced`); `None` rate → add 0 and count `unresolved_rate_minutes`; accumulate the same into `by_task[task_id]` and `by_person[user_id]`. |
| `progress_pct` | `(spent: i64, budget: i64) -> Option<u8>` | `budget > 0` → `(spent*100/budget)` clamped 0..=100; else `None`. (Mirrors `row_spend`'s `pct_of`; extract so bar math is shared and tested.) |

**Invariants (unit-tested — these are the reconciliation guarantees):**

- `billable_minutes + nonbillable_minutes == total_minutes`.
- `Σ by_task[*].total_minutes == total_minutes` and same for `by_person` (SC-003).
- `Σ by_task[*].billable_cents == billable_cents` and same for `by_person`.
- With stable rates, `invoiced_cents + uninvoiced_billable_cents == billable_cents` (invoiced comes from line items in the server fn; the identity is asserted in integration tests, SC-004).
- All money integer cents; no floats (Constitution I, SC-006).

`list_project_spend` is refactored to call `fold_entries` and read `total_minutes`/`billable_cents`, so the list and dashboard cannot drift (SC-002).

## View-model DTOs (`models/dashboard.rs`) — compiled for both targets

Plain serde structs returned by the server functions and consumed by the page. Money is `Option<i64>` cents (None → render "—"); hours derived from minutes in the UI via the existing `format_decimal`. Illustrative shape:

```
ProjectDashboard {
    // header
    project: Project,          // reuse existing DTO
    client_name: String,
    // budget & progress (budget_kind drives which fields are Some)
    budget_kind: BudgetKind,
    budget_minutes: Option<i64>,
    budget_amount_cents: Option<i64>,
    spent_minutes: i64,
    spent_billable_cents: Option<i64>,
    progress_pct: Option<u8>,
    // hours
    total_minutes: i64,
    billable_minutes: i64,
    nonbillable_minutes: i64,
    // money
    invoiced_cents: Option<i64>,
    uninvoiced_cents: Option<i64>,
    money_incomplete: bool,    // some billable minutes had no resolvable rate
}

BreakdownRow {                 // one per task or per person
    id: Uuid,
    label: String,             // task or person name
    total_minutes: i64,
    billable_minutes: i64,
    nonbillable_minutes: i64,
    billable_cents: Option<i64>,
}

ProjectBreakdowns { by_task: Vec<BreakdownRow>, by_person: Vec<BreakdownRow> }

EnabledTaskRow { task_id: Uuid, name: String, billable: bool, rate_cents: Option<i64> }

RecentEntryRow {
    id: Uuid, spent_date: NaiveDate, person: String, task: String,
    minutes: i32, billable: bool, note_snippet: Option<String>,
}
```

(The team/assignments section reuses the existing `Assignment` DTO and `list_assignments`, unchanged.)

## Validation / behavior rules (from FR)

- **FR-005 / FR-006 / Constitution I**: money is integer cents via `line_amount_cents`; hours from integer `minutes`; spend basis = precise `minutes` + FR-024 cascade (identical to the list).
- **FR-003**: budget section shape is chosen by `budget_kind` — `hours` uses `budget_minutes`, `amount` uses `budget_amount_cents`, `none` shows totals with `progress_pct = None`.
- **FR-007 / D3**: invoiced from `invoice_line_items.amount_cents`; uninvoiced from live-resolved billable value of `invoice_id IS NULL` entries.
- **FR-008 / FR-009 / SC-003**: by-task/by-person buckets are produced in the same fold as the totals, so they reconcile by construction.
- **FR-014 / D4**: unresolved rate → 0 amount + `money_incomplete = true`; hidden money → all cents fields `None` ("—").
- **FR-013 / FR-015**: empty inputs yield zero totals, empty vectors, `progress_pct = None` — no divide-by-zero.
- **FR-016 / FR-017**: no migration; all figures in `projects.currency`, no conversion.

## Entity relationships

Unchanged. The dashboard is a **read projection** over `projects → {time_entries, project_tasks, assignments}` and `time_entries → invoice_line_items`. The "timed vs untimed", "billable vs non-billable", and "invoiced vs uninvoiced" splits are all derived from existing scalar columns, not new state.
