# Phase 0 Research: Project Detail Dashboard

All spec ambiguities were resolved in the `## Clarifications` session; no `NEEDS CLARIFICATION` remain. This file records the technical decisions and, crucially, the **data-source map** that proves every figure is computable from the current schema (data-honesty), plus what is deferred and why.

## References

- **Harvest gap report** — `.scratch/harvest-gap-analysis.md`, *Projects / budgets / rates* section and top recommendation #1: "Ship the project detail dashboard — progress + per-task billable/non-billable breakdown + invoiced/uninvoiced. Data already exists (spend, tasks, invoices); the page is a stub. Biggest value for least new backend." Harvest's project detail shows: progress vs budget, hours-per-week bar chart, per-task and per-person billable/non-billable breakdown (hours + amount), invoiced/uninvoiced, and rates. This feature adopts the **computable** subset and defers the charting/forecasting subset.
- **Design** — `design/project/app/Horae Projects.dc.html` is the Projects **list** screen (table with Budget / Spent / Budget-remaining columns, progress bar `proj-bar`/`proj-bar-fill`, Actions menu, export modal). There is **no dedicated detail mockup**; the dashboard reuses the list's visual language (cards, `proj-bar`, mono figures, badges) at implementation time per the repo's design-implement rule.
- **Existing code** — `crates/horae/src/pages/projects.rs` (`ProjectDetail` stub + `ProjectList` with `row_spend`), `crates/horae/src/server_fns/projects.rs` (`list_project_spend`), `crates/core/src/invoice.rs` (`resolve_rate`, `line_amount_cents`), `crates/horae/migrations/0001_init.sql` + `0002_invoices.sql`.

## D1 — Data-source map (the data-honesty core)

**Decision**: Every dashboard figure maps to an existing column. Nothing is invented.

| Dashboard figure | Source (existing columns) | Derivation |
|---|---|---|
| Name, code, type, currency, active, start/end | `projects.name/code/project_type/currency/active/starts_on/ends_on` | direct |
| Client | `clients.name` via `projects.client_id` | direct join |
| Budget kind + budget | `projects.budget_kind`, `budget_minutes` (hours), `budget_amount_cents` (amount) | direct |
| Total / billable / non-billable hours | `time_entries.minutes`, `time_entries.billable` (for the project) | sum minutes; split by `billable` |
| Billable amount (spent) | per entry: `project_tasks.rate_cents` → `assignments.rate_cents` → `users.billable_rate_cents`, then `line_amount_cents(rate, minutes)` | FR-024 cascade, summed over billable entries |
| Progress % | spent ÷ budget (hours-spent ÷ `budget_minutes`, or amount-spent ÷ `budget_amount_cents`) | clamp bar to 100% |
| Invoiced amount | `invoice_line_items.amount_cents` for lines whose `time_entry` belongs to the project | sum (authoritative — what was billed) |
| Uninvoiced amount | resolved billable value of billable entries with `invoice_id IS NULL` | cascade + `line_amount_cents` |
| By-task breakdown | group the same per-entry rows by `time_entries.task_id` | sum minutes/billable-split/amount per task |
| By-person breakdown | group by `time_entries.user_id` | sum minutes/billable-split/amount per person |
| Team | `assignments` (already shown today) + `users.name`, role | direct |
| Enabled tasks | `project_tasks` (`billable`, `rate_cents`) + `tasks.name` | direct |
| Recent entries | latest `time_entries` for the project | order by `spent_date desc` (then `created_at desc`), limit N |

**Rationale**: This table is the contract that keeps the feature honest — a reviewer can check each row against `0001_init.sql`/`0002_invoices.sql`. Anything not on this table is deferred (D6), not faked.

## D2 — Spend basis reuses the Projects-list rollup

**Decision**: Hours and billable amount use **precise `minutes`** and the **same FR-024 cascade** already implemented in `list_project_spend` (`resolve_rate` then `line_amount_cents`, summed in Rust). Extract that fold into a pure `horae-core` helper (`project_rollup`) and have both `list_project_spend` and the dashboard call it.

**Rationale**: SC-002 requires the dashboard "Spent" to equal the list "Spent" exactly. Sharing one tested implementation makes drift impossible. `list_project_spend` already resolves rates in Rust after a single flat `LEFT JOIN` query (project_tasks / assignments / users) — the dashboard needs the identical per-entry row plus `task_id`/`user_id`/`billable`/`spent_date`/`invoice_id` to also produce the groupings, so it is the same query shape with a few more selected columns.

**Alternatives considered**: rounded/locked `rounded_minutes` — rejected; the list uses precise `minutes`, and mixing bases would make the two screens disagree. Aggregating money in SQL (`SUM`) — rejected; rate resolution is a first-non-null cascade across three nullable sources plus banker's-rounding `line_amount_cents`, which is exactly the pure logic the constitution (II) wants in `horae-core`, not duplicated in SQL.

## D3 — Invoiced is authoritative from line items; uninvoiced is derived

**Decision**: Invoiced = `SUM(invoice_line_items.amount_cents)` for lines linking the project's entries. Uninvoiced = resolved billable value (cascade) of the project's billable entries with `time_entries.invoice_id IS NULL`.

**Rationale**: `invoice_line_items.amount_cents` is what was actually billed and must not be re-derived (rates may change after invoicing). Uninvoiced work has no line item yet, so it must be estimated at current resolvable rates — the same basis as "spent". Invoiced + uninvoiced then reconciles to total billable amount for entries at stable rates (SC-004). The link is unambiguous: `time_entries.invoice_id` is set when an entry is invoiced (0002 adds the FK), and `invoice_line_items.time_entry_id` references the entry.

**Edge**: an entry could in principle be billable, invoiced, yet its current resolved rate differs from the stored line amount. Invoiced always uses the stored amount; only uninvoiced uses live rates — so the number a manager acts on ("still to bill") is a current estimate while "already billed" is historical fact.

## D4 — Unresolvable rates and money visibility read as "—"

**Decision**: An entry whose rate is `None` at every cascade level contributes 0 to amount but still counts its hours; the section flags that money may be incomplete. Where the viewer is not entitled to see money (reusing Horae's existing money-visibility policy), all amounts render as "—" while hours/identity stay visible.

**Rationale**: FR-014 — showing a confident `0` for missing rates would mislead. "—" plus a flag is honest. This mirrors how the Projects list already tolerates missing rates (`resolve_rate(...).unwrap_or(0)` sums 0) but the dashboard additionally surfaces the gap so a manager knows to set rates rather than trust an understated total.

## D5 — Project-to-date totals; recent list is the only bounded section

**Decision**: All totals/breakdowns/money cover the project's entire history (no date filter), matching `list_project_spend`. "Recent entries" shows the latest N (small fixed count, e.g. 10) ordered newest-first.

**Rationale**: Simplicity and consistency with the list's lifetime spend. Date-range filtering (D6) is deferred.

## D6 — Deferrals (data-honesty: not computable now or out of v1 scope)

| Deferred | Spec ID | Why deferred |
|---|---|---|
| Burn-down / hours-per-week bar chart | D-001 | Needs a per-period time series + charting; the headline progress bar covers "how far into budget". |
| Forecasting / projected completion/overspend | D-002 | Modelling not backed by stored data. |
| Cost-based margin / profit | D-003 | `users.cost_rate_cents` exists, but per-entry cost accounting + profit is a separate concern; v1 reports billable value only. |
| Rate-editing UI | D-004 | Viewing resolved rates is in scope; editing is a separate feature (gap report rec #2). |
| Date-range filter / per-day timeline | D-005 | v1 is project-to-date + a recent-entries feed. |
| Retainer per-period accounting, fixed-fee milestones | D-006 | No per-period budget reset or milestone schedule in the schema. |
| Dashboard-specific export | D-007 | List export already exists; not part of v1. |

## D7 — No migration; read-only server functions

**Decision**: Add read-only `#[server]` functions only (see `contracts/server-fns.md`); no schema change; regenerate `.sqlx/` for the new queries.

**Rationale**: FR-016 / SC-007. Every figure is derivable from existing tables (D1). This keeps the change additive and inside the constitution's single-mutation-path rule (IV — the feature adds no mutations at all) and single-datastore rule (III — no migration).

## D8 — Rendering with existing design language

**Decision**: Build the page from existing tokens/utilities — cards, the `proj-bar`/`proj-bar-fill` progress bar already used by the list, mono monetary figures, `badge` pills, and standard tables — since no detail mockup exists.

**Rationale**: The repo's design-implement rule recreates visuals in the existing CSS at implementation time. The list screen (`Horae Projects.dc.html`) is the visual reference; the detail layout groups the same primitives into a header + budget/hours/money summary + breakdown tables + team/tasks + recent feed.
