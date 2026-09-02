# Quickstart: Timesheet Exports for Invoicing Transparency

End-to-end validation for the two capabilities. Assumes the standard dev setup (Nix shell, Postgres, `migrate run`, `seed`, `dx serve` — see repo AGENTS.md), signed in as Admin with `DEV_LOGIN=1`. Seed data provides clients, projects, time entries, and at least one invoice with line items.

## Prerequisites

- Server running with `--features server`; a client with projects and tracked time; at least one generated invoice whose lines reference time entries.
- `DATABASE_URL` set; `.sqlx/` cache regenerated after the query changes (`cargo sqlx prepare --workspace -- --features server --all-targets`).

## Scenario 1 — Filtered detailed export (US2)

1. Open the Reports page. Set a date range covering seeded entries.
1. Click **Export CSV** with no client/project selected. Save the file.
1. Select a **client** (the project selector narrows to that client). Click **Export CSV** again.
1. Additionally select a **project**. Click **Export XLSX**.

**Expected**

- Step 2 output equals the current date-range-only export (backward compatible — FR-012, SC-005).
- Step 3 file contains only the chosen client's rows within the range (FR-013).
- Step 4 file contains only the chosen project's rows within the range (FR-013).
- Selecting a project that is not under the chosen client yields a file with headers and **zero data rows**, not an error (FR-014, SC-006).

**Direct URL check** (bypassing UI):

```
GET /api/reports/export/csv?from=2026-08-01&to=2026-08-31&client_id={CLIENT}&project_id={PROJECT}
```

returns 200 with a CSV scoped to that client+project+range.

## Scenario 2 — Invoice timesheet backup, PDF (US1 + US3)

1. Open an invoice's detail page.
1. In the actions area, confirm a **Timesheet backup** control distinct from the existing **Download PDF** (the invoice document).
1. Download the backup as **PDF**.

**Expected**

- The PDF header shows the client, the invoice number, and a period equal to the earliest–latest work date among the billed entries (FR-005, research D1).
- Entries are grouped **project → task**, with an hours subtotal per task, per project, and a grand total (FR-004).
- Grand total hours == sum of the invoice's billed line hours, to the minute (FR-009, SC-002).
- If the invoice's lines are rated, an amount column appears; task/project/grand amount subtotals equal the sum of the invoice lines' `amount_cents` (FR-006, SC-003). If unrated, the PDF shows hours only (FR-007).

## Scenario 3 — Backup as CSV / XLSX (US1)

1. From the same invoice, download the backup as **CSV**, then **XLSX**.

**Expected**

- Both contain one row per billed line/entry (date, project, task, notes, hours, and amount when rated) — the same entry set as the PDF (FR-008).
- Row hours sum to the same grand total as the PDF (SC-002).

## Scenario 4 — Empty / edge cases

1. Backup an invoice with **no lines** (or construct one): every format returns a valid, empty document — headers/grouping present, zero rows, zero totals — not an error (FR-010, SC-006).
1. Request a backup for an unknown invoice id: 404.

## Automated coverage

- **`horae-core` unit tests** (`export_backup`): roll-up correctness — minute subtotals equal the sum of parts; cent subtotals sum only present amounts and stay `None` when absent; empty input ⇒ zero groups / `None` amount (R1–R5 in data-model.md).
- **Integration tests** (`#[sqlx::test]`, `#[serial]`): filtered-export SQL honors client/project (incl. the empty-but-valid mismatch); invoice-backup entry set equals the invoice's line `time_entry_id`s; backup grand hours == Σ line minutes and amount == Σ line `amount_cents`.
