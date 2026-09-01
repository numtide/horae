# Feature Specification: Project Detail Dashboard

**Feature Branch**: `feat/project-dashboard`

**Created**: 2026-09-01

**Status**: Draft

**Input**: User description: "Turn Horae's stub project detail page into a real project dashboard, using Harvest's project detail as the reference for WHAT to show, scoped strictly to what Horae's existing PostgreSQL schema already supports (data-honest). Show project identity, budget & progress, hours (billable vs non-billable), amounts from stored rates, invoiced vs uninvoiced, breakdowns by task and by person, the project's team & enabled tasks, and recent time entries. Keep v1 to the real, computable dashboard; defer anything needing new persisted data or heavy charting."

## Clarifications

### Session 2026-09-01

- Q: Which columns/fields can the dashboard actually show without inventing data the backend can't produce? → A: Only fields that exist in the current schema (`projects`, `clients`, `tasks`, `project_tasks`, `assignments`, `users`, `time_entries`, `invoices`, `invoice_line_items`). "Spent" money is derived from the existing FR-024 rate cascade (`project_tasks.rate_cents` → `assignments.rate_cents` → `users.billable_rate_cents`) applied to billable minutes; "invoiced" money is read directly from `invoice_line_items.amount_cents`. Everything else Harvest shows (burn-down chart, hours-per-week bars, forecasting, cost margin/profit) is deferred because it needs new persisted data, new UI machinery, or data the schema does not carry per entry.
- Q: Does this feature need a database migration? → A: No. Every number is computable from the existing tables. At most it adds new read-only server functions (aggregation queries); it introduces no new columns, tables, or enum values. If a future refinement wants cached rollups that would be an additive migration, but v1 requires none.
- Q: Should "Spent" and the progress bar use precise tracked minutes or the rounded/locked minutes? → A: Use the same basis the rest of Horae already uses for spend — precise tracked `minutes` — so the dashboard's totals match the Projects list "Spent" column (`list_project_spend`) exactly and never disagree with it. Invoiced amounts still come from the stored line-item amounts.
- Q: Who can view the dashboard, and does everyone see money? → A: Any authenticated user who can already open the project detail route can view the identity, budget/hours, team, tasks, and recent entries. Monetary figures (amounts, invoiced/uninvoiced, rates) follow the same visibility rule Horae already applies elsewhere; where a viewer is not entitled to see money or no rate is resolvable, money reads as "—" rather than a wrong or zero number.
- Q: What time window does the dashboard cover? → A: Project-to-date (all time) for every total and breakdown, matching how the Projects list reports lifetime spend. "Recent entries" is the only time-bounded section and simply shows the latest N entries by date. Date-range filtering is deferred.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - See a project's budget and progress at a glance (Priority: P1)

A manager opens a project from the Projects list and lands on a dashboard that immediately answers "how is this project doing against its budget?". For an hours-budgeted project they see hours spent, the hours budget, a progress bar with a percentage, and hours remaining. For a fee-budgeted project they see the fee spent, the fee budget, the same style of progress bar, and the amount remaining. For a project with no budget they still see the totals (hours and, where visible, billable amount) without a progress bar. The header names the project (with its code if it has one), its client, its type, its currency, its active/archived state, and its start/end dates when set.

**Why this priority**: This is the single most valuable thing the page is missing today — the detail page is a stub, while budget/spend only appear on the list row. It turns the page from a placeholder into the reason a manager visits a project. It is also fully computable from data Horae already stores, so it is the natural MVP.

**Independent Test**: Open the detail page for an hours-budgeted project that has tracked time and confirm the header identity, the hours spent/budget/remaining, and a progress bar whose percentage equals spent ÷ budget (clamped to 100%). Repeat for a fee-budgeted project (fee figures) and for a budget-less project (totals, no bar). Confirm the "Spent" figures match that project's row on the Projects list exactly.

**Acceptance Scenarios**:

1. **Given** a project with an hours budget of 100h and 47h tracked, **When** the user opens its dashboard, **Then** the budget section shows 47h spent, 100h budget, 53h remaining, and a progress bar at 47%.
1. **Given** a project with a fee (amount) budget and billable time logged at resolvable rates, **When** the user opens its dashboard, **Then** the budget section shows the billable amount spent, the fee budget, the amount remaining, and a progress bar at spent ÷ budget.
1. **Given** a project with `budget_kind = none`, **When** the user opens its dashboard, **Then** total hours (and billable amount where money is visible) are shown with no progress bar and no "remaining" figure.
1. **Given** a project whose tracked time exceeds its budget, **When** the user opens its dashboard, **Then** the progress bar is capped at 100% while the numeric percentage and a negative/zero "remaining" convey the overage.
1. **Given** the same project shown on the Projects list, **When** the user compares the list "Spent" with the dashboard "Spent", **Then** the two figures are identical.

______________________________________________________________________

### User Story 2 - Break down hours and amounts by task and by person (Priority: P2)

The manager scrolls to see where the time and money went: a per-task table (each enabled task's total hours, its billable vs non-billable hours, and its billable amount) and a per-person table (each contributor's total hours, billable vs non-billable hours, and billable amount). Totals across each table reconcile with the project's headline hours and amount.

**Why this priority**: Budget/progress tells you *whether* a project is on track; the breakdowns tell you *why*. They are the core analytical value of Harvest's project detail and are computable by grouping the same time entries used for the headline totals — no new data, just aggregation. They come after P1 because the page is already useful with the headline figures alone.

**Independent Test**: For a project with entries spanning two tasks and two people, some billable and some not, open the dashboard and confirm the by-task and by-person tables each sum (hours and amount) to the project's headline totals, with the billable/non-billable split adding up per row.

**Acceptance Scenarios**:

1. **Given** a project with time on two tasks, **When** the user views the by-task breakdown, **Then** each task row shows its total hours, its billable and non-billable hours, and its billable amount, and the rows sum to the project totals.
1. **Given** a project with time from two people, **When** the user views the by-person breakdown, **Then** each person row shows their total hours, billable/non-billable hours, and billable amount, and the rows sum to the project totals.
1. **Given** a non-billable entry, **When** it appears in either breakdown, **Then** it counts toward total and non-billable hours but contributes zero to the billable amount.
1. **Given** an entry whose rate cannot be resolved through the cascade, **When** it appears in a breakdown, **Then** its hours still count and its amount contribution is treated as zero (and the row/section signals that money may be incomplete rather than showing a misleading figure).

______________________________________________________________________

### User Story 3 - See invoiced vs uninvoiced money (Priority: P2)

The manager wants to know how much of the billable work has been billed. The dashboard shows the invoiced amount (the sum of the project's invoice line-item amounts) and the uninvoiced amount (the billable value of tracked, not-yet-invoiced work), so they can see what is still to bill.

**Why this priority**: Invoiced vs uninvoiced is a headline number on Harvest's project detail and directly drives billing decisions. Horae already links time entries to invoice line items, so it is computable now. It sits alongside the breakdowns as high-value analytics on top of the P1 headline.

**Independent Test**: For a project where some billable entries have been invoiced and others have not, open the dashboard and confirm the invoiced amount equals the sum of that project's line-item amounts and the uninvoiced amount equals the resolved billable value of the remaining (uninvoiced) billable entries.

**Acceptance Scenarios**:

1. **Given** a project with three billable entries, two of them invoiced, **When** the user views the money summary, **Then** the invoiced amount equals the sum of the two entries' invoice line-item amounts.
1. **Given** the same project, **When** the user views the money summary, **Then** the uninvoiced amount equals the resolved billable value of the one remaining uninvoiced billable entry.
1. **Given** a project with no invoices, **When** the user views the money summary, **Then** invoiced reads as zero (or "—") and uninvoiced equals the project's total billable amount.
1. **Given** a viewer not entitled to see money, **When** they open the dashboard, **Then** invoiced/uninvoiced and all amounts read as "—" while hours and identity remain visible.

______________________________________________________________________

### User Story 4 - See the team, enabled tasks, and recent activity (Priority: P3)

The manager reviews who is on the project and what work is loggable: the existing assignments table (person and project role), the project's enabled tasks with each task's billable flag and rate, and a short list of the most recent time entries (date, person, task, hours, billable, a note snippet) so they can sanity-check what is being tracked.

**Why this priority**: These round out the dashboard and match Harvest's Tasks & Team tabs plus a recent-activity feed, but the project is already understandable from budget, breakdowns, and money. The assignments table already exists today, so this story mostly adds the enabled-tasks view and the recent-entries list.

**Independent Test**: For a project with assignments, enabled tasks, and recent entries, open the dashboard and confirm the team list, the enabled-tasks list (with billable flag and rate), and a recent-entries list ordered newest-first are all shown and consistent with the underlying data.

**Acceptance Scenarios**:

1. **Given** a project with two assigned people, **When** the user views the team section, **Then** both people appear with their project role (preserving today's assignments behavior).
1. **Given** a project with three enabled tasks, **When** the user views the tasks section, **Then** each enabled task appears with its billable flag and its rate (where money is visible), or "—" where no rate is set.
1. **Given** a project with recent entries, **When** the user views the recent-activity list, **Then** the latest entries appear newest-first with date, person, task, hours, billable indicator, and a note snippet.
1. **Given** a project with no tracked time yet, **When** the user opens the dashboard, **Then** every section renders an empty state (zero totals, no bar, empty lists) without error.

______________________________________________________________________

### Edge Cases

- **No budget set** (`budget_kind = none`): show totals only; no progress bar, no "remaining".
- **Over budget**: progress bar clamps at 100%; the numeric percentage may exceed 100% and "remaining" may be zero or negative to convey the overage.
- **No tracked time**: all totals are zero, breakdown tables are empty, and the page still renders (no divide-by-zero, no error).
- **Unresolvable rate**: an entry with no rate anywhere in the cascade counts toward hours but contributes zero to billable amount; the section signals that money may be understated rather than showing a confidently wrong number.
- **Non-billable project or entries**: non-billable time counts toward total and non-billable hours and contributes nothing to billable amount; a `non_billable` project shows hours-centric figures.
- **Archived project**: the dashboard still renders for an archived project, marked archived in the header.
- **Mixed currency**: all monetary figures are shown in the project's own currency; the dashboard does not convert between currencies. (Rates stored against other entities are assumed to be in the project's currency, consistent with how the Projects list already sums spend.)
- **Deactivated person or task with historical time**: entries logged by a now-inactive user or against a now-disabled task still appear in the breakdowns and recent list (history is preserved).
- **Rounded vs precise minutes**: spend uses precise tracked minutes to match the Projects list; the rounded/locked minutes are not used for the headline spend.
- **Retainer / recurring projects**: shown like other projects using their budget kind; recurring-period accounting (per-period reset) is out of scope for v1.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The project detail page MUST present a dashboard for a single project instead of the current placeholder, showing the sections defined below, and MUST render for any project the viewer is allowed to open (active or archived, with or without tracked time).
- **FR-002**: The dashboard MUST show a header identifying the project by name (and code when present), its client, its project type, its currency, its active/archived state, and its start and end dates when those are set.
- **FR-003**: The dashboard MUST show a budget & progress section driven by the project's budget kind:
  - hours budget → hours spent, hours budget, hours remaining, and a progress bar (spent ÷ budget, clamped to 100%);
  - amount (fee) budget → billable amount spent, fee budget, amount remaining, and the same style of progress bar;
  - no budget → totals only, with no progress bar and no "remaining".
- **FR-004**: The dashboard MUST show total tracked hours for the project and split them into billable and non-billable hours.
- **FR-005**: The dashboard MUST compute the project's billable amount from stored rates by resolving each billable entry's rate through the existing rate cascade (task rate on the project → the person's assignment rate → the person's default billable rate) and MUST represent all money as integer minor units (cents) with the project's currency — never floating point.
- **FR-006**: "Spent" hours and amounts on the dashboard MUST use the same basis Horae already uses for project spend (precise tracked minutes and the same rate cascade) so the dashboard's totals match the Projects list "Spent" figure for the same project.
- **FR-007**: The dashboard MUST show an invoiced amount equal to the sum of the project's invoice line-item amounts, and an uninvoiced amount equal to the resolved billable value of the project's tracked-but-not-yet-invoiced billable work.
- **FR-008**: The dashboard MUST show a per-task breakdown: for each of the project's enabled tasks that has tracked time, its total hours, its billable and non-billable hours, and its billable amount; the rows MUST reconcile with the project's headline hours and amount.
- **FR-009**: The dashboard MUST show a per-person breakdown: for each person who logged time on the project, their total hours, billable and non-billable hours, and billable amount; the rows MUST reconcile with the project's headline hours and amount.
- **FR-010**: The dashboard MUST show the project's team (its assignments: person and project role), preserving the existing assignments capability.
- **FR-011**: The dashboard MUST show the project's enabled tasks with each task's billable flag and rate (rate shown where money is visible, otherwise "—").
- **FR-012**: The dashboard MUST show a short list of the most recent time entries for the project, ordered newest-first, each with its date, person, task, hours, billable indicator, and a note snippet.
- **FR-013**: Every headline total, breakdown, and money figure MUST be computed project-to-date (all tracked time), matching how the Projects list reports lifetime spend; the recent-entries list is the only time-bounded section and simply shows the latest entries.
- **FR-014**: Where a viewer is not entitled to see money, or where an entry's rate is unresolvable, the dashboard MUST render money as "—" (and signal that an amount may be incomplete) rather than showing a zero or otherwise misleading monetary value; hours and identity MUST remain visible regardless.
- **FR-015**: The dashboard MUST render correct, non-erroring empty states for a project with no budget, no tracked time, no assignments, no enabled tasks, or no invoices.
- **FR-016**: The feature MUST NOT require any database migration: every figure MUST be derivable from the existing schema, adding at most new read-only aggregation queries and no new columns, tables, or enum values.
- **FR-017**: The dashboard MUST show all monetary figures in the project's own currency and MUST NOT convert between currencies.

### Deferred (explicitly out of scope for v1)

- **D-001**: Burn-down / budget-over-time chart and hours-per-week bar chart (need per-period series and charting machinery; the headline progress bar covers "how far into budget").
- **D-002**: Forecasting / projected completion or projected overspend (needs modelling not backed by stored data).
- **D-003**: Cost-based margin / profit (cost rates exist on `users.cost_rate_cents`, but per-entry cost accounting, blended cost, and profit are a separate concern; v1 reports billable value only).
- **D-004**: Rate-editing UI (viewing resolved rates is in scope; editing task/assignment/user rates is a separate feature per the gap report).
- **D-005**: Date-range / period filtering and per-day timelines (v1 is project-to-date plus a recent-entries list).
- **D-006**: Retainer per-period (monthly reset) accounting and milestone/fixed-fee payment schedules.
- **D-007**: Export of the dashboard itself (the Projects list already offers CSV/XLSX export; a dashboard-specific export is not part of v1).

### Key Entities *(include if feature involves data)*

- **Project**: the subject of the dashboard. Carries identity (name, code, client, type, currency, active flag, start/end dates) and budget configuration (budget kind, budget minutes for hours budgets, budget amount in cents for fee budgets). No new attributes.
- **Time Entry**: the source of every hours and spend figure. Provides its project, its person, its task, its billable flag, and its precise minutes, plus a link to an invoice when it has been invoiced. Aggregated by task, by person, and in total.
- **Project Task (enabled task)**: an org-level task enabled on the project, carrying the per-project billable flag and optional rate override used both for the tasks section and as the first step of the rate cascade.
- **Assignment**: a person's membership on the project, carrying their project role and optional per-project rate override (the second step of the rate cascade). Basis for the team section.
- **User**: a contributor; provides the default billable rate (the last step of the rate cascade) and the display name for the by-person breakdown and recent-entries list. (Cost rate exists but is out of scope for v1.)
- **Invoice Line Item**: links an invoiced time entry to its billed amount; the sum of a project's line-item amounts is the invoiced figure, and its complement over billable work is the uninvoiced figure.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A viewer opening a project can, within a few seconds and without leaving the page, read the project's budget status (spent, budget, remaining, percentage) for hours-, fee-, and no-budget projects.
- **SC-002**: The dashboard's "Spent" hours and amount match the same project's Projects-list "Spent" figure exactly for 100% of projects (no drift, no double counting).
- **SC-003**: For any project, the by-task and by-person breakdowns each reconcile to the project's headline hours and billable amount (row sums equal the totals) in 100% of cases.
- **SC-004**: For any project, the invoiced amount equals the sum of that project's invoice line-item amounts, and invoiced + uninvoiced equals the project's total billable amount, in 100% of cases.
- **SC-005**: The dashboard renders without error for edge-case projects — no budget, no tracked time, over budget, unresolvable rates, archived — showing sensible empty/zero states.
- **SC-006**: Every monetary figure shown is exact to the cent (integer minor units) with no floating-point rounding error, in the project's own currency.
- **SC-007**: Shipping the feature requires zero database migrations (verifiable: no new migration file, no schema change).

## Assumptions

- **Data-honest scope**: the dashboard shows only what the current schema can produce. Anything Harvest shows that would require new persisted data (burn-down series, forecasts, per-entry cost, retainer period accounting) is deferred, not faked.
- **Spend basis matches the list**: hours and billable amount use precise tracked minutes and the existing FR-024 rate cascade, so the dashboard never disagrees with the Projects list "Spent" column.
- **Invoiced is authoritative from line items**: the invoiced amount comes from stored `invoice_line_items` amounts (what was actually billed), not from re-deriving rates, so it reflects the real invoice even if rates later change.
- **Uninvoiced is derived**: uninvoiced = resolved billable value of billable entries not yet linked to an invoice; it is an estimate at current resolvable rates, consistent with how spend is computed.
- **Project-to-date totals**: all totals cover the project's entire history; there is no date-range filter in v1. "Recent entries" shows the latest N entries (a small fixed count) purely as an activity feed.
- **Money visibility reuses existing rules**: who may see monetary figures follows the same policy Horae already applies; this feature does not introduce a new permission model. Where money is hidden or unresolvable, it reads as "—".
- **Single currency per project**: figures are in the project's currency; rates stored elsewhere are assumed to be in that currency, and no cross-currency conversion is performed (matching the existing spend rollup).
- **No dedicated detail mockup**: only the Projects *list* screen has a design mockup (`design/project/app/Horae Projects.dc.html`); the detail layout reuses the existing design tokens/utilities (cards, progress bars, tables) and is a design detail settled at implementation time.
- **At most additive read queries**: the feature may add new read-only server functions for the aggregations; it changes no existing behavior and adds no schema.

## Dependencies

- Builds on the existing Projects list and its spend rollup (`list_project_spend`) and the shared rate cascade (`horae_core::invoice::resolve_rate` / `line_amount_cents`), reusing them so figures stay consistent.
- Relies on the existing assignments feature (already shown on the detail page today) and the project-tasks link for the team and enabled-tasks sections.
- Relies on the existing invoices and invoice line items for the invoiced/uninvoiced figures.
- References the Harvest gap report (`.scratch/harvest-gap-analysis.md`, Projects section — recommendation #1: "ship the project detail dashboard") for the intended feature set, and the Projects list design (`design/project/app/Horae Projects.dc.html`) for visual language; no dedicated detail mockup exists.
