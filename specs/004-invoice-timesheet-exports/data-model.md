# Phase 1 Data Model: Timesheet Exports for Invoicing Transparency

**No schema change.** This feature adds no tables, columns, or migrations. It defines read-only projections over existing tables and one pure grouping/subtotal type in `horae-core`.

## Existing tables read (no modification)

- **`time_entries`** — `spent_date`, `minutes`, `rounded_minutes`, `billable`, `notes`, `project_id`, `task_id`, `user_id`, `org_id`.
- **`projects`** `p` — `id`, `name`, `client_id`. (The `p.client_id` join key is what the client filter uses; the join already exists in `reports::fetch_entries`.)
- **`clients`** `c` — `id`, `name`, `currency`, `address`, `tax_id`.
- **`tasks`** `t` — `id`, `name`.
- **`users`** `u` — `id`, `name`.
- **`invoices`** — `id`, `client_id`, `number`, `currency`, `total_cents`, `status`, `issued_on`, `due_on`, `org_id`. **No period, no project_id** (this is why the backup uses exact line entries — see research D1).
- **`invoice_line_items`** — `id`, `invoice_id`, `time_entry_id`, `description`, `minutes`, `rate_cents`, `amount_cents`. One line ↔ one billed time entry; source of truth for billed minutes/amount.

## Projection A — Filtered detailed export row

Extends the existing `DetailedReportRow` fetch. **No new type** — the same row shape; only the query gains optional filters.

`reports::ExportParams` gains two optional fields:

| Field | Type | Meaning |
|-------|------|---------|
| `from` | `String` (date) | existing — range start (inclusive) |
| `to` | `String` (date) | existing — range end (inclusive) |
| `client_id` | `Option<Uuid>` | NEW — restrict to `projects.client_id = client_id` |
| `project_id` | `Option<Uuid>` | NEW — restrict to `time_entries.project_id = project_id` |

**Filter SQL** (mirrors `server_fns/reports.rs::report_time`):

```
WHERE te.spent_date BETWEEN $1::date AND $2::date
  AND ($3::uuid IS NULL OR p.client_id = $3)
  AND ($4::uuid IS NULL OR te.project_id = $4)
```

**Rules**: both filters optional and independent; both null ⇒ identical to today's export (FR-012); both set ⇒ AND (FR-013); a project not under the chosen client ⇒ zero rows, valid file (FR-014).

## Projection B — Invoice backup billed entry

One row per invoice line, joined to its billed time entry. Selected by `invoice_line_items.invoice_id = $1`, ordered for stable grouping (`p.name, t.name, te.spent_date`).

| Field | Source | Notes |
|-------|--------|-------|
| `spent_date` | `time_entries.spent_date` | for the date column; feeds header period MIN/MAX |
| `project_name` | `projects.name` | grouping level 1 |
| `task_name` | `tasks.name` | grouping level 2 |
| `notes` | `time_entries.notes` / `invoice_line_items.description` | line detail |
| `minutes` | `invoice_line_items.minutes` | **billed** minutes — source of truth for hours (not `time_entries.minutes`) |
| `amount_cents` | `invoice_line_items.amount_cents` | present only when the line is rated |
| `rate_cents` | `invoice_line_items.rate_cents` | drives "has amount?" |

**Why billed minutes/amount from the line, not the entry**: the line records exactly what was billed after rounding and rate resolution; using it guarantees the backup total equals the invoice (FR-009, research D2).

## Core type — grouped subtotals (`horae-core::export_backup`)

Pure, no I/O. Input: a slice of `{ project_name, task_name, minutes: i64, amount_cents: Option<i64> }`. Output: nested groups with rolled-up subtotals.

```
ProjectGroup {
    project_name: String,
    tasks: Vec<TaskGroup>,
    subtotal_minutes: i64,          // Σ task minutes
    subtotal_amount_cents: Option<i64>, // Σ task amounts; None if no task had an amount
}
TaskGroup {
    task_name: String,
    entries: Vec<Entry>,            // ordered as fetched
    subtotal_minutes: i64,          // Σ entry minutes
    subtotal_amount_cents: Option<i64>,
}
BackupTotals {
    groups: Vec<ProjectGroup>,
    grand_minutes: i64,
    grand_amount_cents: Option<i64>,
    currency: Option<String>,       // the invoice currency; None only when no amounts
}
```

**Rules (unit-tested in `horae-core`)**:

- **R1 (exact minutes)**: every subtotal is the integer sum of its parts in **minutes**; hours are derived only at render time. `grand_minutes == Σ all entry minutes` (FR-009, FR-019).
- **R2 (exact cents)**: amount subtotals sum in **integer cents**; a level's `subtotal_amount_cents` is `Some` iff at least one descendant has an amount, and equals the sum of present amounts (FR-006, FR-007, FR-020).
- **R3 (single currency)**: amounts are summed only within one currency (the invoice's); the type carries no cross-currency addition (FR-020).
- **R4 (no float)**: the type stores and sums only `i64`; no `f64` appears in roll-up.
- **R5 (empty)**: empty input ⇒ zero groups, `grand_minutes = 0`, `grand_amount_cents = None` — a valid empty backup (FR-010).

## Read-only guarantee

Every projection and the core type are read-only. Generating any backup or export performs **no** write to `invoices`, `invoice_line_items`, or `time_entries` (Constitution IV; spec "read-only projection").
