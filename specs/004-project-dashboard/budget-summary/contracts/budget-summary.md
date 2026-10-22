# Contract: Project Detail budget headline

## Server function

`get_project_detail_view(project_id, expected_requester)` is unchanged in
signature, binding and errors. Its `ProjectDetailView` response gains one
field, `budget`, with the types in [data-model.md](../data-model.md).

JSON examples (snake_case external tagging, as implemented and asserted in
`detail_budget.rs`; amounts in minutes or cents):

```json
{ "budget": null }
{ "budget": { "kind": "hours", "currency": "EUR", "period_key": "lifetime",
  "headline": { "exact": { "budget": 120, "spent": 150, "remaining": -30 } } } }
{ "budget": { "kind": "amount", "currency": "EUR", "period_key": "lifetime",
  "headline": "unavailable" } }
{ "budget": { "kind": "hours", "currency": "EUR", "period_key": "lifetime",
  "headline": "unallocated" } }
```

## Authorization

- Visibility is the detail view's existing project-read rule under policy 0
  or 1. Unreadable, foreign-organization and tenant-inconsistent projects are
  not found, as today.
- A fee budget's figures are `Unavailable` unless money is visible. Under
  policy 0, project readers see money. Under policy 1 the viewer needs
  `BillableRateReadAll`, or `BillableRateReadManaged` and management of the
  project. This is the predicate `fetch_progress` and `fetch_project_spend`
  already use.
- Hour budgets are visible to every project reader.
- Withheld and overflowed figures share `Unavailable`. Authorization is proven
  by tests that read the same project as authorized (`Exact`) and unauthorized
  (`Unavailable`, with no number in the JSON) viewers.
- No scope kind, scope label, per-scope allowance or consumption is sent.
- Identity, authority, settings and consumption come from one REPEATABLE READ
  snapshot. Exhausted 40001 retries return the existing conflict error.

## UI contract

Not implemented; the UI pattern awaits review. P1/P4 rest on official
Harvest screenshots (R6). P2, P3 and O3 are decided. O1 is user-deferred:
there is no percentage until it is verified. O2 is a pending proposal.

| `budget` / `headline` | Label | Value | Row | Bar |
|-----------------------|-------|-------|-----|-----|
| `null` | "Budget remaining" | "No budget is set on this project." (exact handoff copy) | "Set up a budget" link when `can_edit` | — |
| `exact`, budget > 0 | "Budget remaining"; "(NN%)" waits for O1 | signed remaining; `text-danger` when negative | "Monthly budget" or "Total budget" + allowance | `<progress>` = `used_percent`; accessible text names spent and allowance |
| `exact`, budget = 0 (O3) | "Budget remaining" | signed remaining | allowance `0` | none |
| `unallocated` | "Budget remaining" | "—" | — | — |
| `unavailable` | "Budget remaining" | "—" with accessible label "unavailable" (P3) | — | — |

- A monthly budget adds a caption above the label: a human month and year
  from `period_key` (`2026-10` → "October 2026", chrono `%B %Y`; the app is
  English-only). Lifetime budgets have no caption.
- Hours are written `format_hours2(m) + " h"`, with "hours" in each
  accessible name (P2). Money uses `format_cents(value, currency)`.
- The bar reuses the list's native `progress` element and `.proj-bar` styling.
  A scoped `.pd-summary .proj-bar { width: 100% }` rule widens it; the list's
  rule is unchanged.
- O2 proposal: a minimal disclosure listing only "Total budget" and "Budget
  spent", from the existing `exact` figures. No Expenses row; the expense
  split depends on feature 016.
- "Set up a budget" links to
  `Route::EditProject { id, return_to: EditReturn::detail(id) }`.
