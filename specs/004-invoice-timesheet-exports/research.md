# Phase 0 Research: Timesheet Exports for Invoicing Transparency

Decisions that resolve the open design questions from the spec. Each records what was chosen, why, and what was rejected.

## D1 — Period accuracy for the invoice backup

**Decision**: Build the backup from the **exact time entries referenced by `invoice_line_items.time_entry_id`** (option C). Join each line back to its time entry for descriptive detail (project, task, date, notes) and group those. The header's displayed period is the derived **MIN/MAX `spent_date`** of the billed entries — for human context only, never to define the entry set.

**Rationale**:

- Each invoice line already points at exactly one billed time entry, so the billed set is known precisely with no re-query and no drift.
- The invoice header carries **no** `period_start`/`period_end` and **no** single `project_id` (verified in `models/invoice.rs`), so any window-based approach is an approximation.
- Hours and amounts can be taken from the line's own recorded `minutes`/`amount_cents`, keeping the backup in exact agreement with the invoice total (Constitution I).

**Alternatives rejected**:

- **(A) Proxy `issued_on`→`due_on`**: those are billing dates, not work dates — would include unbilled entries and miss billed work outside the window. Inaccurate.
- **(B) Derive MIN/MAX(spent_date) from the lines' entries, then re-query all entries in that window**: re-includes unbilled or other-client entries that happen to fall in the range; drifts from "what was billed." (We still compute MIN/MAX for the *header label* only, not to select entries.)
- **(D) Persist `period_start`/`period_end` on invoices (migration)**: a schema change that still would not identify *which specific entries* were billed, and violates the "no schema change needed" simplicity here. Deferred as unnecessary.

## D2 — Amounts: recorded vs recomputed

**Decision**: Show the **`amount_cents` already recorded on each invoice line**, summed in integer cents within the invoice's single currency. Never recompute from rates at export time.

**Rationale**: The line is the source of truth for what was billed (rate cascade already resolved at invoice generation). Recomputing risks disagreeing with the invoice total and reintroduces float/rounding risk. Honors Constitution I (exact integer cents, no float accumulation) and FR-006/FR-020.

**Amount visibility**: A line's amount appears only when it carries one; amount subtotals sum only lines that have amounts (FR-007). An invoice/line with no rate produces an **hours-only** backup. This matches the "optionally amount" requirement and avoids misleading zeroes.

**Alternatives rejected**: Recompute `minutes × resolved_rate` at export (drift + float risk); always show a zero amount for unrated lines (misleading).

## D3 — PDF grouping and subtotals

**Decision**: Group **project → task**, two levels deep. Show an **hours subtotal per task**, an **hours subtotal per project**, and a **grand total of hours**; add matching **amount subtotals at each level** when amounts are present. Header shows client name, invoice number, and the derived work period (D1).

**Rationale**: Two-level grouping matches how customers reason about a bill ("what project, what kind of work"), is the level `projects`/`tasks` joins naturally provide, and keeps v1 small. Deeper dimensions (per user, per day) are explicitly deferred (spec Out of Scope).

**Subtotal correctness**: Roll up in **minutes** and **cents**, only converting minutes→decimal hours at render time, so every displayed subtotal equals the exact sum of its parts (FR-019). This roll-up is the pure `horae-core::export_backup` function, unit-tested in isolation.

**Alternatives rejected**: Flat entry list with only a grand total (loses the "grouped by project/task" transparency the feature is named for); client→project→task three-level (an invoice is single-client, so the client level is redundant — it belongs in the header).

## D4 — Rendering approach for the PDF

**Decision**: Reuse the existing **Typst** pipeline (`render.rs`, `typst-as-lib`, embedded template) with a **new `templates/invoice-backup.typ`** and a `render_invoice_backup_pdf(...)` function alongside `render_invoice_pdf`. Template embedded at compile time; fonts from `typst-kit` as today.

**Rationale**: Typst is already a dependency and already renders invoice PDFs deterministically; a second template is the smallest change and inherits the same byte-identical-output property. No new dependency (Constitution V, `ponytail`).

**Alternatives rejected**: A new PDF crate (needless dependency); HTML→PDF (no such pipeline exists here).

## D5 — Filtered detailed export: parameters and surfacing

**Decision**: Add optional `client_id` and `project_id` **query parameters** to the existing `/api/reports/export/{csv,xlsx}` routes, honored with the `($N::uuid IS NULL OR column = $N)` optional-filter SQL pattern already used by `server_fns/reports.rs::report_time`. Absent params = "all" = today's output (backward compatible).

**Surfacing**: The Reports/export page (`pages/reports.rs`) **already** has `client_filter` and `project_filter` signals (they drive the grouped `report_time` view and its client→project narrowing), but the `export_csv_url`/`export_xlsx_url` it builds pass only `from`/`to`. The change is to **append the selected `client_id`/`project_id` to those export URLs** when set, and to honor them server-side. No new UI controls are needed — the selectors exist.

**Rationale**: Reuses an established pattern and existing UI state; the only real work is threading two optional UUIDs through `ExportParams` and the `fetch_entries` SQL, and extending the URL builder. Client filter maps to `p.client_id` (the `projects` join already exists in `fetch_entries`).

**Alternatives rejected**: A new server function (exports are plain-Axum binary routes by design, not `#[server]`); a separate filtered route (duplicates the handler); free-text name filters (ambiguous — IDs are exact and already available in the UI).

## D6 — Access control

**Decision**: Reuse the existing authorization for viewing invoices / reporting data; downloading a backup or filtered export requires the same permission as viewing the underlying data (FR-018). No new role or sharing model.

**Rationale**: The data exposed is exactly what the user can already see; keeping one authorization path avoids a second policy surface (Constitution IV spirit — one place for authorization).

## D7 — Orphaned billed entry (edge case)

**Decision**: The invoice line is the source of truth for billed **minutes and amount**; descriptive detail (project/task/date) comes from the referenced entry. If a referenced entry no longer exists, the backup still accounts for the line's minutes/amount so totals stay consistent with the invoice; exact display of an orphaned line (e.g. "(entry removed)") is a rendering detail left to implementation. In normal operation invoiced entries are not deleted, so this is a defensive path, not a primary flow.

**Rationale**: Guarantees FR-009 (totals equal the invoice) even under data anomalies, without over-engineering the display. Marked as the one deliberately-open rendering detail (see checklist notes).

## Legacy report export acceptance (2026-10-25)

These decisions cover the acceptance increment in `spec.md` and `plan.md` only.

## L1 — Evidence layers

**Decision**: Use a browser suite in the existing disposable runner for the screen, both
links and CSV contents, plus a Rust HTTP-harness test for XLSX and CSV contents over the
same routes and query template (`from`, `to`, `client_id`, `project_id`, `user_id`). Each layer builds
its own parallel fixture with the same names, minutes and order; the UUIDs differ.

**Rationale**: Only the browser can prove what the page shows and which links it
builds. The browser runner (`nix/checks/browser.nix`) has bash, coreutils, curl, node,
PostgreSQL and Playwright, but no XLSX reader. The Rust harness already reads XLSX through
the `zip` dev-dependency.

**Alternatives considered**: Rust alone (rejected: it cannot see the page); decoding XLSX
in node (rejected: it needs a handwritten ZIP/XML parser or a new dependency); adding
`unzip` to the browser check (rejected: a new tool, and cells would still need a new parser).

## L2 — Independent expectations

**Decision**: Expected rows and totals are literal constants derived by hand from the
fixture minutes and the organization's rounding (15, nearest), and written as both minutes
and two-decimal strings. Neither layer takes expectations from a report response or from
rows the page has loaded.

**Rationale**: The R02 currency-totals suite and the scoped export tests compare
against their own responses, which proves agreement between surfaces, not correctness.

## L3 — Decimal display and XLSX numbers

**Decision**: Totals are reconciled in integer minutes. Displayed values are compared as
exact two-decimal strings. The fixture deliberately includes rows (50, 50, 53 and 7 minutes)
whose displayed values add up to 2.66 while the correct total is 2.67, and the test asserts both.
XLSX number cells are compared as the exact `<v>` text the existing writer emits (plan.md
fixture table). Each matched row maps to its fixture entry, and the totals are summed from
those entries' integer minutes. The tests do no float parsing or arithmetic.

**Rationale**: `format_hours2` rounds half-up per value, so adding displayed decimals is
not a valid check; summing minutes is (Constitution I, FR-019). No rounding behavior
changes. The writer's `f64` step is XLSX-D1 (spec, plan; not this file's D1): an approved
owner decision under Constitution 2.0.0 whose conformance check is pending. These tests do not
certify it.

## L4 — XLSX reading utility

**Decision**: Move the existing test-only helpers `xlsx_part`, `assert_cell` and
`assert_text_cell` from the `reports/limits.rs` test module, verbatim, into
`reports/xlsx_cells.rs` (`#[cfg(test)]`). No helper is added.

**Rationale**: One shared copy of the existing reading pattern, with no new parser and no
new dependency. The other inline `zip` readers in the authorization tests are left as
they are (out of scope).

## L5 — Excel duration cells (XLSX-D1, 2026-10-28)

**Decision**: The owner chose to keep numeric decimal hours in Excel duration cells (timesheet
Hours and Rounded Hours, invoice Hours). Constitution 2.0.0 Principle I permits the one
floating-point step this needs: converting final integer minutes to hours where the
spreadsheet library writes the cell. Verifying that the existing writers meet the exception's
conditions is still pending.

**Rationale**: Spreadsheet number cells are binary floating point, so n/60 hours cannot be
stored exactly in any numeric cell. Users need hours they can sum, and Harvest's detailed
Excel export was observed to use numeric hours cells. Integer minutes stay the authority for
storage, rounding and totals, and each cell is only an approximate presentation of them.

**Alternatives considered**: exact two-decimal text matching the CSV (rejected: a spreadsheet
cannot sum it); integer minutes as numbers (rejected: the unit and headers would differ from
hours); a `=minutes/60` formula (rejected: the file still holds a floating-point duration,
and readers that do not recalculate show the cached value).
