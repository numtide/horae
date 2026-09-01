# Contract: Export Routes

Plain Axum routes (binary downloads with custom `Content-Type`), consistent with the existing exports in `reports.rs`. No `#[server]` functions (exports are the sanctioned non-mutating Axum surface). All routes are read-only and require the same authorization as viewing the underlying data (FR-018).

## 1. Filtered detailed timesheet export (extends existing)

Existing routes, gaining two optional query parameters. **No new route.**

```
GET /api/reports/export/csv?from=YYYY-MM-DD&to=YYYY-MM-DD[&client_id=UUID][&project_id=UUID]
GET /api/reports/export/xlsx?from=YYYY-MM-DD&to=YYYY-MM-DD[&client_id=UUID][&project_id=UUID]
```

**Query params**

| Param | Required | Meaning |
|-------|----------|---------|
| `from`, `to` | yes | existing inclusive date range |
| `client_id` | no | restrict to `projects.client_id = client_id` |
| `project_id` | no | restrict to `time_entries.project_id = project_id` |

**Behavior**

- No `client_id`/`project_id` ⇒ **byte-for-byte identical** to today's output (FR-012, SC-005).
- Both set ⇒ AND of the two (FR-013).
- No matching rows (incl. project outside client) ⇒ HTTP 200 with a well-formed file: header row present, zero data rows (FR-014, SC-006).
- Invalid UUID in a param ⇒ 400 (bad request); invalid/missing date ⇒ same handling as today.
- Columns, headers, `Content-Type`, and `Content-Disposition` filename are unchanged from the current detailed export.

**Implementation note**: extend `ExportParams` with `client_id: Option<Uuid>` / `project_id: Option<Uuid>`; thread into `fetch_entries` using the `($N::uuid IS NULL OR …)` pattern from `report_time`. The `projects p` join (with `p.client_id`) already exists.

**UI**: `pages/reports.rs` already holds `client_filter`/`project_filter` signals; append their values to `export_csv_url`/`export_xlsx_url` when non-empty. No new controls.

## 2. Invoice timesheet backup (new)

```
GET /api/invoices/{id}/backup/csv
GET /api/invoices/{id}/backup/xlsx
GET /api/invoices/{id}/backup/pdf
```

Named `…/backup/…` to stay distinct from the existing `…/export/{csv,xlsx,pdf}` (which export the invoice **lines**, i.e. the invoice document). These export the **timesheet detail behind** the invoice.

**Path param**: `id` — invoice UUID.

**Entry set**: exactly the time entries referenced by that invoice's `invoice_line_items.time_entry_id` (research D1). No date-window re-query.

**Responses**

| Format | `Content-Type` | `Content-Disposition` filename |
|--------|----------------|-------------------------------|
| csv | `text/csv` | `invoice-{number}-timesheet.csv` |
| xlsx | `application/vnd.openxmlformats-officedocument.spreadsheetml.sheet` | `invoice-{number}-timesheet.xlsx` |
| pdf | `application/pdf` | `invoice-{number}-timesheet.pdf` |

**CSV / XLSX shape**: one row per billed line/entry — at minimum `Date, Project, Task, Notes, Hours` and, when the line is rated, `Amount` (and the invoice currency). Hours = `line.minutes / 60` formatted as today's exports do; amount = `line.amount_cents` as integer cents formatted via `horae_core::money`.

**PDF shape**: see `pdf-layout.md`.

**Behavior**

- Unknown invoice `id` ⇒ 404 (matches existing invoice-export handlers).
- Invoice with no lines ⇒ 200, valid empty document (headers/grouping present, zero rows, zero totals) (FR-010, SC-006).
- Grand total hours == Σ billed line hours; amount total == Σ line `amount_cents` (rated lines only) (FR-009, SC-002, SC-003).
- Read-only: no writes performed.

## Route registration (`main.rs`)

Add beside the existing invoice exports:

```
.route("/api/invoices/{id}/backup/csv",  get(reports::export_invoice_backup_csv))
.route("/api/invoices/{id}/backup/xlsx", get(reports::export_invoice_backup_xlsx))
.route("/api/invoices/{id}/backup/pdf",  get(reports::export_invoice_backup_pdf))
```

The two `/api/reports/export/*` routes are unchanged (filters ride as query params).
