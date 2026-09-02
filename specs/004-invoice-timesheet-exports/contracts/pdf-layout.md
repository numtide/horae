# Contract: PDF Backup Layout

Client-facing PDF for one invoice's timesheet backup. Rendered via the existing Typst pipeline (`render.rs` + a new `templates/invoice-backup.typ`, embedded at compile time, fonts from `typst-kit`), deterministic like the current invoice PDF.

## Inputs (from server → template)

- `client_name` — the invoice's client.
- `invoice_number` — the invoice's number.
- `period_start`, `period_end` — MIN/MAX `spent_date` of the billed entries (header context only; may be equal; absent when there are no entries).
- `currency` — the invoice's ISO currency code (only meaningful when amounts are shown).
- `groups` — the `horae-core::export_backup` roll-up: projects → tasks → entries, with per-level `subtotal_minutes` and optional `subtotal_amount_cents`.
- `grand_minutes`, optional `grand_amount_cents`.
- `has_amounts` — whether any line carried an amount (drives whether the Amount column/subtotals render).

## Layout

```
┌────────────────────────────────────────────────────────────┐
│  Timesheet backup                                          │
│  Client: {client_name}                                     │
│  Invoice: {invoice_number}                                 │
│  Period: {period_start} – {period_end}                     │
├────────────────────────────────────────────────────────────┤
│  ▸ {project_name}                                          │
│      {task_name}                                           │
│        {date}   {notes}            {hours}   [{amount}]    │
│        {date}   {notes}            {hours}   [{amount}]    │
│      Task subtotal                 {Σ hours} [{Σ amount}]  │
│      {task_name_2} …                                       │
│    Project subtotal                {Σ hours} [{Σ amount}]  │
│  ▸ {project_name_2} …                                      │
├────────────────────────────────────────────────────────────┤
│  Total                             {grand hours} [{amount}]│
└────────────────────────────────────────────────────────────┘
```

## Rules

- **Grouping**: project → task, in the fetched order (`project_name, task_name, spent_date`). Two levels only (FR-004).
- **Header**: client name, invoice number, derived period (FR-005).
- **Hours**: rendered from `subtotal_minutes` / `grand_minutes` (minutes summed first, then shown as decimal hours, e.g. 90 → `1.50`), so displayed subtotals equal the sum of their parts (FR-019).
- **Amount column**: shown **only when `has_amounts`** (FR-006, FR-007). When shown, per-line amount appears for rated lines; a line without an amount shows a blank amount cell; subtotals sum only present amounts. Amounts formatted from integer cents in `currency` via `horae_core::money` (FR-020).
- **Single currency**: all amounts are the invoice's one currency; no conversion.
- **Empty invoice**: header renders; no group rows; totals show 0 hours (and no amount) — a valid document (FR-010).
- **Branding**: basic/clean for v1 (reuse the app's existing document styling); logos/letterhead/theming beyond that are deferred (spec Out of Scope).
- **Determinism**: same invoice ⇒ byte-identical PDF (inherits the existing Typst rendering guarantee).
