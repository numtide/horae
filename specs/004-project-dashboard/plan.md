# Implementation Plan: Project Detail Dashboard

> Historical plan, pending reconciliation with the expanded 2026-09-29 spec.
> Do not execute its old rate, permission, no-chart or no-migration assumptions.
> Planning resumes after PD-001 is clarified; this document is not acceptance.

## Independent implementation increment — reporting periods and activity

The complete dashboard plan below remains historical. The ongoing delivery may
implement this independent foundation while destructive/manual-link policies
remain open; this is not acceptance of those actions or completion of Spec Kit's
full planning gate.

- Scope: FR-005, FR-006 and FR-021. Add `crates/core/src/project_activity.rs`
  for inclusive work-date intervals and exact weekly/cumulative tracked minutes.
- Reuse `week::week_start`, `chrono` and checked integer arithmetic. No new
  dependency, schema, rate rule, import rule, financial mutation or CSS change.
- Month/quarter/year bounds use calendar dates, including leap years. Reversed
  or unrepresentable ranges return explicit errors.
- Weekly buckets follow the supplied workspace week start, include empty weeks
  and clip the first/last buckets to the chosen interval. Cumulative values start
  at zero at the selected interval, not at project inception.
- The caller supplies an explicit bucket bound; exceeding it fails before
  allocation. Never silently truncate, saturate minutes or shift dates via UTC.
- Test first: calendar boundaries, configured weeks, inclusive filtering,
  empty weeks, deterministic input order, exact split/cumulative reconciliation,
  negative data, overflow and bucket limits. Run the whole core suite and Clippy
  in the pinned Nix dev shell.
- The server projection in `server_fns/projects/activity.rs` authorizes through
  `project_read_access`, derives all-time bounds or accepts inclusive dates, then
  feeds daily SQL aggregates into the domain series. It reads actual minutes,
  caps intervals at 5,200 weeks before fetching daily values, and uses a
  repeatable-read/read-only transaction with a five-second statement timeout.
- Shared DTOs in `models/project.rs` contain only interval, configured week start
  and minute buckets. Reuse the timesheet's weekday conversion. No schema or
  business mutation is needed for this increment.
- Follow-up integration remains required: design-matched charts and accessible
  table, exports and browser verification. Passing domain/database suites do not
  prove those requirements.

Constitution pre/post-design check for this increment: integer minutes only;
pure `horae-core`; PostgreSQL-only, macro-checked, org-authorized reads; no new
mutation surface or dependencies; verification through Nix. No exception
required. Full-feature gates stay open.

### Activity UI integration

Connect the authorized projection to a scoped Project Detail panel. Use the
existing menu, input and table components and segmented-control styles. Reporting
presets and an explicitly applied custom interval feed the same response to the
chart and its accessible table. Pending/error responses must not reuse previous
project or period data. Chart mode changes are local and must not refetch data.
SVG geometry uses integer scaling with widened intermediates; no chart dependency
or money-to-hours conversion is required. Selected-period cumulative totals must
be labelled separately from lifetime metrics. Verify date boundaries, zero/large
values, route/request races, both build targets and the real browser.

This does not waive the separate chart week navigation, applicable hour-budget
overlay, complete dashboard layout, summaries, breakdowns, exports or remaining
delivery slices. Those remain required for acceptance.

### Tasks/Team reporting toolbar

Place the existing reporting menu below the breakdown tabs, beside its period
heading, as in the handoff. Keep the toolbar and selected tab mounted when a
breakdown read is pending or fails; remove stale rows/counts and permit retry
without resetting the selected period. Invoice history remains independently
accessible. Hide, rather than reset, the report toolbar on the Invoices tab.
Custom validation/cancellation must not dispatch reads. Chart-window navigation
remains local; applying a reporting interval refreshes both authorized views.
Verify placement, keyboard focus, tab continuity, error recovery and responsive
layout using render tests and the disposable-database browser runner. This UI
increment changes no SQL, permissions, schema or shared CSS; scoped report links
and working exports remain separate required work.

### Contextual report navigation and downloads

The existing manager-only report/export pipeline needs an optional task filter
before task hours can link to a genuinely scoped report. Add the filter to the
grouped query, shared detailed/CSV row stream and XLSX size preflight together;
omitting it retains current behavior. The Reports selector must show its active
value even if catalog loading fails, without clearing or widening the selection.
Do not change progress permissions into permission to read entry notes or costs.
Verify conjunction with project/person/dates, foreign/unknown identities,
size-check parity, invalid UUIDs and revoked report authority. Regenerate the
SQLx cache against the existing migrated schema; no migration is needed.

Then wire reproducible project/task/person/all-time/custom-date route context and
the project export menu. PDF summary rendering and bounded-download explanations
remain required; the task-filter foundation alone does not satisfy FR-016.

### Chart window navigation

Keep the reporting interval shared by charts, Tasks and Team. Add a separate
26-week chart viewport with a configured-week anchor, previous/next week and a
return to this week action. Do not navigate beyond the current week. This is
display-only navigation over the already-authorized series, not a new report
filter or a new read. Clip the viewport to the selected report dates and label
empty intersections explicitly. Keep the complete selected-period table and
total available; cumulative values retain their selected-period origin when
the beginning is outside the viewport. Reset the anchor when applying another
report period and when switching projects. Verify week/year/leap boundaries,
cumulative carry-in, no extra requests, loading/failure, native keyboard controls
and responsive bounds. Calendar selection and budget overlays remain separate
acceptance requirements; this increment does not waive them.

### Chart calendar selection

Reuse DatePicker and the native calendar-popover behavior already used by New
Project. The chart's week label opens a configured-week calendar and remounts
its browsed month on each opening. Add an optional inclusive calendar bound;
only the chart supplies the end of the current week, so future scheduling in
Timesheet and New Project stays available. Preserve focus on selection/Escape,
light dismissal and viewport positioning through the existing shared script.
Selections change only the chart anchor, never report filters or reads. Verify
inclusive bounds, default unbounded behavior, month navigation, keyboard and
short/mobile/zoom layouts, plus the existing scheduling calendar regression.
The shared calendar's existing nominal dimensions use scalable tokens; its
heading/footer reflow and narrow padding prevent hidden overflow at enlarged
text sizes. Compare the eight-screen style baseline before accepting this shared
change, and inspect the calendar's inner surface/controls as well as its popover.

**Branch**: `feat/project-dashboard` | **Date**: 2026-09-01 | **Spec**: [spec.md](./spec.md)

### Project identity integration

Replace the generic detail title and metadata card with the handoff header:
back navigation, client link, code/name, type, status and manager-only edit link.
Keep saved dates, currency, tags and administrator-only notes. Extend the existing
authorized details projection with client ID, project type and active status;
do not widen progress access or expose additional private fields. Reuse the edit
route and utility CSS. Wire existing archive/reactivate through the shared menu
and native confirmation dialog, with duplicate-submit protection, visible errors,
safe retry and explicit success. No additional lifecycle policy or mutation API.
Test header identity, role gating, archived projects,
failed/pending route transitions and navigation highlighting; regenerate SQLx
metadata and verify both targets and the browser. Pin/duplicate/delete actions, financial
summaries and all other dashboard/delivery requirements remain open.

### Hours, budgets and internal costs

Implement the corresponding three handoff tiles while invoice attribution and
external-billing policy are reconciled separately. A single authorized,
repeatable-read, read-only projection returns lifetime actual minute splits,
current configured budget scopes (or the legacy lifetime budget) and permitted
internal costs. Reuse configured budget SQL and parameterize the existing spend
query for a single project; do not fork financial rules or fetch every project.
Pure checked budget aggregation belongs in `horae-core` and must preserve
unallocated scopes, zero allowances and individual overruns. Costs use actual
minutes and workspace currency, distinguish absent from explicit zero rates,
and omit the complete cost payload for members or managers whose total would
include private project overrides. Test current permissions, empty/archived and
foreign projects, configured monthly rounding versus actual hours, list/detail
parity, missing/zero/private costs and route loading/error identity. No schema,
dependency or shared CSS default changes. Invoice tiles, tabs, exports, full
visual verification and all remaining delivery slices remain required.

**Input**: Feature specification from `specs/004-project-dashboard/spec.md`

### Reciprocal task and team breakdowns

Use one authorized reporting snapshot for task/person intersections, with actual
minute splits and per-entry internal costs in workspace currency. Aggregate each
intersection once in PostgreSQL, then use checked core arithmetic for reciprocal
parent/grand totals. Keep all currently enabled/assigned and historical identities,
including zero-time and inactive rows; never include unrelated organization users
or task catalog entries. Reuse the summary's cost policy: members and managers
with private contributing overrides receive no costs. Missing rates remain
unknown and cannot become a partial subtotal.

Share the chart's selected interval with both tabs. Tasks expand into people and
people into tasks; use semantic tables, keyboard-operable disclosure controls,
real counts and stable sorting by exact minutes, name and ID. Reset disclosure
state when project or interval changes; loading/error states must not retain old
rows. Preserve management through the existing authorized controls until the
shared editor covers their complete behavior. Bound rows explicitly and fail
instead of silently truncating. Invoice attribution, exports and whole-delivery
acceptance remain required independent follow-ups.

### Invoice history integration

Read stored `net_before_tax_cents` through both time-entry and fee-occurrence
sources, scoped to the project and organization. Do not join through the current
`time_entries.invoice_id`: a voided invoice keeps its lines after the entry is
released or billed again. Current application writes do not move existing
time-entry/fee sources between projects, and imported existing entries are
skipped. A future source-move feature must preserve historical attribution before
changing this invariant.

Return all invoice history, labelled independently of chart work dates; preserve
the chart interval while switching tabs. Keep draft reservations, sent/paid
contributions and void history distinct and group totals by stored invoice
currency. Recheck current organization-manager/admin authority, active status and
project access within a bounded read-only snapshot. Exceeding the row or integer
limit fails without partial results. Reuse exact checked domain sums and real
invoice routes; do not reprice historical lines or attribute a whole mixed invoice.

The model does not currently store invoice subject or payment date. Their addition
to the invoice workflow is a pending product choice; do not substitute private
notes, issue date or an invented payment timestamp. Invoice creation with project
context, those metadata fields, financial tiles and remaining acceptance are
separate required follow-ups, not satisfied by a read projection.

### Contextual invoice creation

Reuse `PrepareInvoice` and the current recovery gate from a project-specific
route. Load authorized project/client identity, then fix both source IDs for this
flow; the general Invoices screen keeps its client/project picker unchanged.
Do not initialize dates from unrelated issue dates or lifetime invoice history.
Preview, fee refresh, generation and recovery must retain the same single-project
scope. Cancelling returns to Project Detail without a mutation; generation keeps
the existing pending-navigation guard and idempotent recovery. Verify direct and
changed routes, denied/missing identity, source payloads, cancellation and generic
invoice preparation regression. No new billing policy, schema or mutation API.

### Invoiced summary

Lift the existing authorized invoice resource to the keyed project page and share
its state/retry with the summary tile and history. Reuse stored non-void totals;
count only non-void invoices per stored currency and label draft reservations,
discounts, tax exclusion and lifetime scope explicitly. Empty, private, pending
and failed results are not monetary zero. Keep the tile readable independently
of a failed work-summary request, and do not add another server query or change
external-billing eligibility. Use the existing auto-fit tile grid and verify
route races, permissions, one-read behavior, browser reconciliation and bounds.

## Summary

Replace the stub `ProjectDetail` page (today: `"Project detail for {id}"` plus an assignments table) with a real, data-honest project dashboard. It shows the project's identity, budget & progress (driven by `budget_kind`), total/billable/non-billable hours, billable amount, invoiced vs uninvoiced money, per-task and per-person breakdowns, the team and enabled tasks, and a recent-entries feed.

Technical approach: **no schema migration**. Every figure is computed from existing tables (`projects`, `time_entries`, `project_tasks`, `assignments`, `users`, `invoices`, `invoice_line_items`) by new **read-only** `#[server]` aggregation functions. Spend (hours + billable amount) reuses the exact FR-024 rate cascade already used by the Projects list — `horae_core::invoice::resolve_rate` + `line_amount_cents` — so the dashboard's "Spent" always matches the list. Invoiced money is read directly from `invoice_line_items.amount_cents` (authoritative: what was actually billed); uninvoiced is the resolved billable value of not-yet-invoiced billable entries. The existing rate-resolution/aggregation math is lifted into a small pure `horae-core` helper so it is unit-tested in isolation and shared by the list rollup and the dashboard. The page renders with existing design tokens/utilities (cards, progress bars, tables); no dedicated detail mockup exists.

## Technical Context

**Language/Version**: Rust (edition 2024)

**Primary Dependencies**: Dioxus 0.7 (fullstack + router, SSR + WASM), Axum, sqlx (compile-time-checked macros), chrono, uuid (v7)

**Storage**: PostgreSQL 15+; **no new migration** — read-only aggregation over existing tables; `.sqlx/` offline cache regenerated for the new queries

**Testing**: `cargo test -p horae-core` (pure aggregation/rate math); `#[sqlx::test]` + `#[serial]` integration in `crates/horae/tests/` for the new server functions (reconciliation and invoiced/uninvoiced)

**Target Platform**: Linux server (Axum) + WebAssembly SPA (Dioxus web)

**Project Type**: Web application (single feature-gated crate `horae`, two targets) + pure `horae-core` domain crate

**Performance Goals**: Dashboard opens without perceptible delay; per-project aggregation over that project's time entries (a bounded set), served by a small number of queries — target a single page load with no N+1 fan-out

**Constraints**: All money is integer minor units (cents) in the project's own currency, no floats and no cross-currency conversion; hours derived from integer `minutes`; totals must reconcile exactly (breakdown rows sum to headline; invoiced + uninvoiced = total billable)

**Scale/Scope**: Single organization; one existing route (`Route::ProjectDetail { id }`); additive change touching `horae-core` (extract/extend rate-aggregation helper), new read-only `#[server]` functions, and the `ProjectDetail` page renderer. No schema change.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Assessment |
|-----------|------------|
| **I. Exactness** | ✅ Hours from integer `minutes`; money as integer cents with the project's currency; billable amount uses the existing integer `line_amount_cents` (banker's rounding) and the FR-024 cascade; invoiced read from stored `amount_cents`. No floats. Reconciliation is a stated success criterion (breakdown rows sum to totals; invoiced + uninvoiced = total billable). |
| **II. Domain Purity** | ✅ The correctness-critical part — resolving each entry's rate and aggregating minutes/amount into totals and by-task/by-person groupings — is factored into a pure `horae-core` helper (no sqlx/axum/dioxus) and unit-tested there. SQL only fetches the raw per-entry rows; grouping/summing stays pure, mirroring how `list_project_spend` already resolves rates in Rust. |
| **III. Single Datastore** | ✅ PostgreSQL only; **no migration** — read-only aggregation over existing tables. PKs untouched; `org_id` already scoped. `.sqlx/` cache regenerated for the new read queries. |
| **IV. Mutations Through Server Functions** | ✅ Feature is entirely **read-only**; it adds `#[server]` read functions and issues no client-side fetches. No new mutation path; existing assignment mutations already on the page are unchanged. |
| **V. Reproducible Builds & Formatting Gate** | ✅ `nix fmt` / `nix flake check` green; `.sqlx` prepare committed; no toolchain assumptions; new `horae-core` unit tests and `#[sqlx::test]` integration tests. |

**Result**: PASS — no violations. Complexity Tracking not required.

## Project Structure

### Documentation (this feature)

```text
specs/004-project-dashboard/
├── plan.md              # This file
├── research.md          # Phase 0 — decisions, data-source map, deferrals
├── data-model.md        # Phase 1 — read model (view models) + derivations, no schema change
├── contracts/
│   └── server-fns.md    # New read-only #[server] signatures + returned view models
├── quickstart.md        # Phase 1 — end-to-end validation guide
├── checklists/
│   └── requirements.md  # Spec quality checklist (from /speckit-specify)
└── tasks.md             # Phase 2 — created by /speckit-tasks
```

### Source Code (repository root)

```text
crates/core/src/
├── invoice.rs           # (exists) resolve_rate + line_amount_cents — reused unchanged
└── project_rollup.rs    # NEW (pure): fold per-entry rows into totals + by-task/by-person
                         #   breakdowns (billable/non-billable minutes, billable cents);
                         #   shared by list_project_spend and the dashboard; unit-tested

crates/horae/src/
├── models/dashboard.rs          # NEW: dashboard view-model DTOs (header, budget, totals,
│                                #   breakdown rows, team/task rows, recent-entry rows)
├── server_fns/projects.rs       # ADD read-only fns: get_project_dashboard (identity+budget+
│                                #   totals+invoiced/uninvoiced), list_project_breakdowns
│                                #   (by task + by person), list_recent_project_entries;
│                                #   refactor list_project_spend onto the shared rollup
└── pages/projects.rs            # Replace ProjectDetail stub body with the dashboard sections;
                                 #   keep the existing assignments block as the team section

crates/horae/assets/css/horae.css   # dashboard card/stat/breakdown-table styles (reuse proj-bar etc.)
crates/horae/tests/integration.rs   # reconciliation (rows sum to totals), invoiced+uninvoiced=billable,
                                     #   empty-state + over-budget + unresolvable-rate cases
```

**Structure Decision**: Reuse the existing two-crate layout. The only genuinely new files are one pure `horae-core` module (the rollup fold), one models file for the view-model DTOs, and the design docs; the server functions extend `server_fns/projects.rs` and the page rewrites the `ProjectDetail` body. Extracting the fold into `horae-core` lets the Projects list rollup and the dashboard share one tested implementation, so their "Spent" figures cannot drift (SC-002). No migration, matching FR-016/SC-007.

## Complexity Tracking

No constitutional violations — section intentionally empty.
