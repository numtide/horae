# Feature Specification: Timesheet Exports for Invoicing Transparency

**Feature Branch**: `feat/invoice-timesheet-exports`

**Created**: 2026-09-01

**Status**: Draft

**Input**: User description: "Timesheet exports for invoicing transparency. Two related capabilities: (a) Filtered detailed timesheet exports — scope the existing CSV/XLSX detailed timesheet export by client and/or project (in addition to the existing date range), for ad-hoc reporting. (b) Invoice backup — from an invoice, download the timesheet detail behind it (the hours the customer is paying for) to attach for transparency, in CSV, XLSX, and a client-facing PDF grouped by project/task with hours (and optionally amount) subtotals. Keep v1 scoped to: CSV/XLSX filtered detailed export + invoice backup exporting the EXACT time entries referenced by invoice_lines.time_entry_id + a basic client-facing PDF grouped by project then task with hours subtotals (amounts only when rates are available). Defer fancy branding, scheduled/emailed backups, and per-user breakdowns."

## Clarifications

### Session 2026-09-01

- Q: For the invoice backup, how is the set of "hours the customer is paying for" determined — by a date window, or by the exact entries billed? → A: By the **exact time entries** the invoice's lines reference. Each invoice line already points at one billed time entry, so the backup is the set of those entries — no proxy date window, no drift from re-querying a period. The invoice carries no stored period, so any window-based approach would be an approximation.
- Q: When must a monetary amount appear in the backup (CSV/XLSX/PDF)? → A: Only when a billed line carries a rate. The amount shown is the amount already recorded on the invoice line, never recomputed. When a line has no rate (unbilled/zero-rate context), the backup shows hours only for that line and omits its amount, and amount subtotals sum only the lines that have amounts.
- Q: How is the filtered detailed export scoped beyond the existing date range? → A: By optional client and/or project selectors that narrow the existing detailed export. Absent selectors mean "all" (today's behavior). Choosing a client narrows to that client's projects; choosing a project narrows to that project; the two combine (a project outside the chosen client yields an empty-but-valid export).

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Attach the hours behind an invoice (Priority: P1)

A person who has sent (or is about to send) an invoice wants to give the customer a transparent, itemized record of exactly the work being billed. From the invoice they download a timesheet backup — a client-facing PDF that lists every billed time entry grouped by project and then by task, with hours subtotals per task and per project and a grand total, plus a header naming the client, the invoice number, and the period the work spans. They attach it to the invoice email so the customer can see precisely which hours they are paying for.

**Why this priority**: This is the headline value — "invoicing transparency." An invoice total on its own is opaque; the backup is the evidence behind it. Every other capability here is a supporting or ad-hoc variant of producing this detail.

**Independent Test**: Open an existing invoice that has line items, download the PDF backup, and confirm it lists exactly the time entries the invoice billed, grouped project → task with correct hours subtotals and a grand total that equals the invoice's billed hours, and a header showing client, invoice number, and the work's date span.

**Acceptance Scenarios**:

1. **Given** an invoice whose lines reference several time entries across two projects, **When** the user downloads the PDF backup, **Then** the document groups the entries by project, then by task within each project, and shows an hours subtotal for each task, each project, and a grand total.
1. **Given** that same invoice, **When** the user downloads the PDF backup, **Then** the total hours in the backup equal the sum of the billed hours on the invoice's lines (no entry added, dropped, or double-counted).
1. **Given** an invoice with billed lines that carry rates, **When** the user downloads the PDF backup, **Then** each such line shows its amount and the task/project/grand subtotals include an amount column whose figures equal the amounts already recorded on the invoice lines.
1. **Given** an invoice whose billed lines carry no rate, **When** the user downloads the PDF backup, **Then** the document shows hours only (no amount column or an empty one) and remains internally consistent.
1. **Given** an invoice, **When** the user downloads the backup as CSV or as XLSX instead of PDF, **Then** the file contains the same set of billed time entries in a per-entry detail form suitable for a spreadsheet.

______________________________________________________________________

### User Story 2 - Export a detailed timesheet filtered by client and/or project (Priority: P2)

A person preparing an ad-hoc report or a per-client record opens the reporting/export surface, picks a date range as today, and additionally narrows the detailed timesheet export to one client and/or one project. They download the result as CSV or XLSX and get only the matching entries — for example, "all of Acme's hours in August" or "just the Website Redesign project last quarter."

**Why this priority**: It reuses the existing detailed export and adds the most-requested scoping (by who the work is for and what it's on). It is valuable on its own but secondary to the invoice-anchored backup, and it is a smaller, additive change.

**Independent Test**: On the export surface, choose a date range plus a client (and optionally a project), download CSV and XLSX, and confirm every row belongs to the chosen client/project and date range, and that omitting the selectors reproduces today's unfiltered export.

**Acceptance Scenarios**:

1. **Given** entries for several clients in a date range, **When** the user exports the detailed timesheet filtered to one client, **Then** the file contains only that client's entries within the range.
1. **Given** a client with multiple projects, **When** the user additionally filters to one project, **Then** the file contains only that project's entries within the range.
1. **Given** no client or project selected, **When** the user exports, **Then** the output matches the existing date-range-only detailed export exactly (backward compatible).
1. **Given** a project filter naming a project that does not belong to the selected client, **When** the user exports, **Then** the result is a valid file with no entry rows (an empty, well-formed export rather than an error).

______________________________________________________________________

### User Story 3 - Reach the backup from the invoice, alongside the existing invoice download (Priority: P3)

Looking at an invoice's detail page, the user sees — next to the existing "Download PDF" (the invoice itself) — a clear way to download the timesheet backup in each format (CSV, XLSX, PDF). The two are visibly distinct: one is the invoice the customer pays; the other is the hours behind it.

**Why this priority**: The capability in Story 1 is only useful if it is discoverable where invoices live. It is a thin presentation layer over Story 1, so it follows once the backup exists.

**Independent Test**: Open an invoice detail page and confirm there is a labelled control to download the timesheet backup in CSV, XLSX, and PDF, distinct from the existing invoice PDF download, and that each produces the corresponding backup file.

**Acceptance Scenarios**:

1. **Given** an invoice detail page, **When** the user views the actions area, **Then** a clearly labelled "timesheet backup" download is present and distinguishable from the existing invoice PDF download.
1. **Given** that control, **When** the user chooses a format, **Then** the corresponding CSV, XLSX, or PDF backup for that invoice downloads.

______________________________________________________________________

### Edge Cases

- **Invoice with no lines**: the backup is a valid, empty document — headers/grouping present, zero entries, zero totals — not an error.
- **Mixed rated and unrated lines**: hours subtotals always include every line; amount subtotals include only lines that carry an amount, and the presence of any unrated line is not treated as an error.
- **Billed entry later edited or deleted**: the backup reflects what the invoice recorded (the line's stored minutes/amount and the referenced entry's descriptive detail); if a referenced entry no longer exists, the backup still accounts for the billed line rather than failing the whole export. (Exact display of an orphaned entry is a design detail; the invariant is that totals stay consistent with the invoice.)
- **Currency**: an invoice is single-currency; amounts in the backup use that invoice's currency. Amount subtotals are only summed within that one currency.
- **Filtered export with an empty result**: a valid file with headers and no rows (never an error).
- **Filter combination that cannot match** (project not under the chosen client): empty-but-valid file, per US2.
- **Rounding**: hours shown are derived from the same whole-minute values the app already stores; subtotals are summed from minutes and only then presented as hours, so displayed subtotals equal the sum of their parts.
- **Access control**: only users allowed to see an invoice / the reporting surface can download its backup or a filtered export (same authorization as viewing the underlying data today).

## Requirements *(mandatory)*

### Functional Requirements

#### Invoice timesheet backup (US1)

- **FR-001**: The system MUST let a user download, from a specific invoice, a timesheet backup representing the exact set of time entries that invoice bills.
- **FR-002**: The backup's set of entries MUST be defined by the time entries the invoice's line items reference — not by a proxy date window or a re-query of a period. Each billed line contributes its referenced entry.
- **FR-003**: The system MUST offer the backup in three formats: CSV, XLSX, and a client-facing PDF.
- **FR-004**: The PDF backup MUST group entries by project, and within each project by task, and MUST show an hours subtotal for each task, an hours subtotal for each project, and a grand total of hours.
- **FR-005**: The PDF backup MUST include a header identifying the client, the invoice number, and the period the billed work spans (the earliest to latest work date among the billed entries).
- **FR-006**: When billed lines carry a rate, the PDF backup MUST show a per-line amount and include amount subtotals (per task, per project, grand total); the amounts MUST equal the amounts already recorded on the invoice lines and MUST NOT be recomputed.
- **FR-007**: When a billed line carries no rate, the backup MUST present that line's hours without an amount, and amount subtotals MUST sum only the lines that have amounts. A mix of rated and unrated lines MUST NOT produce an error.
- **FR-008**: The CSV and XLSX backups MUST contain the same billed entries in a per-entry detail form (one row per billed line/entry) suitable for spreadsheet use, carrying at least date, project, task, hours, and — where present — amount.
- **FR-009**: The total hours in any backup format MUST equal the sum of the billed hours on the invoice's lines, with no entry added, dropped, or double-counted.
- **FR-010**: An invoice with no lines MUST produce a valid, empty backup in every format rather than an error.

#### Filtered detailed timesheet export (US2)

- **FR-011**: The detailed timesheet export MUST accept an optional client filter and an optional project filter, in addition to the existing date range.
- **FR-012**: With no client or project selected, the export output MUST be identical to the current date-range-only detailed export (backward compatible).
- **FR-013**: Selecting a client MUST narrow the export to entries whose project belongs to that client; selecting a project MUST narrow to that project; the two filters MUST combine (an entry must satisfy both when both are set).
- **FR-014**: A filter combination that matches no entries (including a project outside the chosen client) MUST produce a valid, well-formed file with headers and zero rows, not an error.
- **FR-015**: Both CSV and XLSX filtered exports MUST honor the same filters identically.

#### Presentation & access (US3, cross-cutting)

- **FR-016**: The invoice detail view MUST present a clearly labelled way to download the timesheet backup in each format, visibly distinct from the existing invoice-document PDF download.
- **FR-017**: The filtered-export controls MUST surface on the existing reporting/export surface where the detailed timesheet export already lives, so a user can set the client/project filters alongside the date range.
- **FR-018**: A user MUST only be able to download an invoice's backup, or a filtered export, when they are already permitted to view that invoice / the underlying reporting data.

#### Correctness invariants (cross-cutting)

- **FR-019**: All durations in every export MUST derive from the whole-minute values the system already stores; subtotals MUST be summed in minutes and only then presented as hours, so every displayed subtotal equals the exact sum of its parts.
- **FR-020**: All monetary amounts MUST use integer minor units with the invoice's ISO currency code and MUST NOT be produced via floating-point accumulation; amounts are only summed within a single currency.

### Key Entities *(include if feature involves data)*

- **Invoice**: the bill sent to a client. Carries a client, an invoice number, a single currency, a status, and a total. It has **no stored work period** and **no single project** — it can span multiple projects and tasks — which is why the backup's period is derived from the billed entries and its grouping comes from those entries, not from the invoice header.
- **Invoice line**: one billed item on an invoice. References exactly one time entry and records that entry's billed minutes, its rate (which may be absent), and the resulting amount. The set of an invoice's lines defines the backup's contents; the recorded minutes and amount are the source of truth for the backup's totals.
- **Time entry**: a unit of tracked work on a date for a project and task, with whole-minute duration and descriptive detail (notes, user). The billed entries — reached through the invoice lines — supply the descriptive detail (project name, task name, date) the backup groups and lists.
- **Timesheet backup**: the generated artifact (CSV, XLSX, or PDF) for one invoice: the billed entries, grouped and subtotaled, with a header. It is a read-only projection — generating it never changes any invoice, line, or entry.
- **Filtered detailed export**: the existing detailed timesheet export (CSV/XLSX over a date range) extended with optional client and project scoping. A read-only projection of time entries.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: From an invoice, a user can produce and download a client-ready PDF backup of the hours behind it in under 15 seconds and without leaving the invoice.
- **SC-002**: For every invoice, the grand total of hours in each backup format (CSV, XLSX, PDF) equals the sum of the invoice's billed line hours — verified to the minute — 100% of the time.
- **SC-003**: For every invoice whose lines carry rates, the sum of amounts in the PDF backup equals the sum of the invoice lines' recorded amounts exactly (integer cents, no drift).
- **SC-004**: 100% of PDF backups group entries project → task with a correct hours subtotal at each level and a header naming client, invoice number, and the derived work period.
- **SC-005**: A detailed export filtered by client and/or project returns only matching entries within the date range in 100% of cases; the same export with no filters is byte-for-byte equivalent to today's date-range-only export.
- **SC-006**: An invoice with no lines, and a filter combination that matches nothing, each yield a valid downloadable file (empty content, correct headers) rather than an error, 100% of the time.

## Assumptions

- **Period accuracy via exact entries (decision)**: the invoice backup is built from the exact time entries the invoice's lines reference, because those lines already point at the billed entries and the invoice stores no work period. This is preferred over (a) using the invoice's issued/due dates as a proxy (inaccurate — those are billing dates, not work dates), (b) deriving a MIN/MAX work-date window and re-querying entries in it (would re-include unbilled entries and drift from what was billed), and (c) persisting a period on the invoice (a schema change that still would not capture which specific entries were billed). The header's displayed period is derived as the earliest-to-latest work date among the billed entries, for human context only; it never defines the entry set.
- **Amounts are recorded, not recomputed**: the backup shows the amount already stored on each invoice line and sums those; it never re-derives amounts from rates at export time. This keeps the backup in exact agreement with the invoice and honors the integer-cents money invariant.
- **Amount visibility**: amounts appear only for lines that carry an amount; an invoice or line without rates yields an hours-only backup. This avoids showing misleading zeroes and matches "optionally amount subtotals."
- **PDF grouping depth**: v1 groups two levels deep — project then task — with hours (and, where available, amount) subtotals at each level and a grand total. Deeper breakdowns (e.g. per user, per day) are out of scope for v1.
- **Filtered export placement and defaults**: the client/project filters are optional selectors on the existing export surface; omitting them preserves current behavior. This keeps the change additive and backward compatible.
- **Basic PDF presentation**: v1 uses a clean, readable client-facing layout reusing the app's existing document-rendering approach; elaborate branding/theming (logos, custom color, letterhead beyond what invoices already carry) is deferred.
- **Single currency per invoice**: amounts and amount subtotals in a backup are within the invoice's one currency; no cross-currency conversion is introduced.
- **Authorization reuse**: downloading a backup or a filtered export requires the same permission as viewing the invoice / reporting data today; no new roles or sharing model is introduced.
- **Scope of this feature**: it targets exporting/reporting of existing time-entry and invoice data. It does not add new fields to invoices, does not change how invoices are generated or how rates cascade, and does not alter any total.

## Out of Scope (Deferred)

- Scheduled or emailed backups (automatic generation/delivery on a cadence).
- Per-user (per-teammate) breakdowns or grouping dimensions beyond project → task in the PDF.
- Rich branding/theming of the PDF beyond the app's existing document presentation.
- Persisting a billing period on invoices, or any invoice schema change.
- Additional export formats beyond CSV, XLSX, and PDF.
- Filtering the detailed export by dimensions other than client and project (e.g. task, teammate, billable flag) — may follow later.

## Dependencies

- Builds on the existing detailed timesheet export (CSV/XLSX over a date range) and its reporting surface.
- Builds on the existing invoice model and invoice line items, which already reference the billed time entries and record billed minutes, rate, and amount.
- Reuses the existing document/PDF generation approach already used for invoice PDFs.
- Relies on the existing client/project/time-entry data and the current authorization for viewing invoices and reports.
