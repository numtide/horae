# Data model: project budget remaining tile

No stored entity, column or migration changes. These are read projections
built from existing rows; sources are in [research.md](research.md) R2/R2a.

## Entities

### `BudgetHeadline` (`crates/core/src/budget.rs`, serde)

Whole-project figures for one period, in the budget's unit (minutes or minor
currency units).

| Variant | Meaning |
|---------|---------|
| `Unallocated` | No applicable allowance is set (every scope blank, or no scope). |
| `Unavailable` | No exact figure may be shown: an overflow, or money the viewer may not see. |
| `Exact { budget, spent, remaining }` | All three are exact `i64`s; `remaining = budget - spent` may be negative. |

### `ProjectDetailBudget` (`crates/horae/src/models/project.rs`)

| Field | Type | Rule |
|-------|------|------|
| `kind` | `BudgetKind` | `hours` or `amount`; never `none` (that is `budget: None`). |
| `currency` | `String` | The project's currency. |
| `period_key` | `String` | `YYYY-MM` of the server UTC date when monthly-reset, else `lifetime`. |
| `headline` | `BudgetHeadline` | See derivation. |

`ProjectDetailView.budget: Option<ProjectDetailBudget>`; `None` means no
budget is set.

## Derivation

`headline(scopes: &[(Option<i64>, i64)]) -> BudgetHeadline`:

1. If no scope has `Some` allowance → `Unallocated`.
1. Otherwise call `allocated_totals(scopes)` once. `(Some(budget), Some(spent))` with `budget.checked_sub(spent) = Some(remaining)` gives
   `Exact`; anything else gives `Unavailable`.

The has-allocation fact is needed because `allocated_totals` returns
`(None, Some(_))` both for all-blank and for allowance overflow.

Server derivation (`budgets::project_budget`), inside the detail read's
snapshot after its authority locks:

1. **Metadata preflight** (one plain SELECT, no valuation):

   - `kind` and `currency`;
   - `configured` (a `project_settings` row exists, never a scope count) and
     `monthly_reset`;
   - `can_view_money`, using the unchanged predicates: configured is legacy
     reader, `BillableRateReadAll`, or `BillableRateReadManaged` plus
     management; legacy is the spend query's `can_view_progress` or the same
     canonical grants;
   - `allocated` (any applicable allowance is non-null, using the consumption
     queries' tenant and membership joins): project scope or legacy uses the
     unit's project column; task scope uses linked tasks' settings; person
     scope uses member budgets, hours only.

1. **Precedence:**

   | Condition | Result |
   |-----------|--------|
   | kind `none` | `None` |
   | fee and `!can_view_money` | `Unavailable`, with nothing valued |
   | `!allocated` | `Unallocated`, with nothing valued |
   | otherwise | consumption (step 3) |

1. **Consumption** in one savepoint (sqlx nested transaction). The authority
   locks stay outside it.

   | Path | Query | Result |
   |------|-------|--------|
   | configured | `fetch_progress(…, allocated_only = true)` | project scope: `headline(&[(budget, consumed)])`; task/person: `headline` over every scope row |
   | legacy hours | `spend_in_transaction(…, include_money = false)` | `headline(&[(budget_minutes, spent_minutes)])`, lifetime raw minutes; no money computed |
   | legacy fee | `spend_in_transaction(…, include_money = true)` | `headline(&[(budget_amount_cents, spent_cents)])` |

   - `allocated_only` makes an entry that no non-null scope can count
     contribute 0 without being valued. Explicit-zero allowances still count.
   - Success releases the savepoint.
   - SQLSTATE 22003 alone rolls back to it and gives `Unavailable`; the
     preserved metadata supplies kind, currency and period.
   - Any other error, a rollback error, or a missing expected row (including a
     visible fee whose `spent_cents` is absent) is a server error. It never
     becomes zero or falls back to the legacy path.

Withholding comes before every arithmetic state, so an unauthorized viewer
cannot tell an allocated, unallocated or overflowing fee budget apart.

## Display derivations (from `Exact` only; R6)

| Value | Rule | Source |
|-------|------|--------|
| Bar fill | `used_percent(spent, budget)`, clamped 0..=100 | existing core |
| Label percent | not implemented: the exact-half tie rule (O1) is user-deferred | open |
| Zero allowance | no percentage and no bar; signed remaining and the zero allowance are shown | O3, decided |

`Unallocated` and `Unavailable` have no percent or bar.

## Validation rules

- Allowances and tracked minutes are non-negative in the schema; `headline`
  still treats any `i64` totally.
- No scope label, per-scope allowance or per-scope consumption is part of the
  projection.
