# Contract: legacy Time report total cells (R02)

Scope: the Billable amount and Cost cells of `.report-total-row` in the legacy
(policy 0) Time tab. Every other cell and surface is unchanged.

## Core helper (`horae_core::money`)

```rust
/// Some amount in the set is not visible to the viewer.
#[derive(Debug, PartialEq, Eq)]
pub struct Withheld;

/// Exact per-currency totals, ordered by currency code. Amounts in different
/// currencies are never combined. Any withheld (`None`) amount makes the whole
/// result `Err(Withheld)`, whatever else the set contains, so no partial total
/// can leak. Within a currency, `None` marks a final sum that does not fit in
/// `i64`; summing in `i128` makes that independent of input order.
pub fn sum_by_currency<'a>(
    amounts: impl IntoIterator<Item = (&'a str, Option<i64>)>,
) -> Result<Vec<(&'a str, Option<i64>)>, Withheld>
```

Currency codes are compared and ordered exactly as stored, by byte order, with
no case folding, trimming or merging. Display is a separate step: in-range
labels come from the existing `money` formatter, which trims the code.

## Cell text (`pages/reports.rs`, `money_totals`)

Rules are applied in this order. Each output line is one block element in the
existing cell.

| Input | Lines |
|-------|-------|
| no rows | `—` (the view never reaches this: empty reports show the empty state) |
| any amount withheld | `Restricted` only |
| otherwise, per currency, in range | `money(sum, code)`, for example `EUR 350.00` |
| otherwise, per currency, out of range | `{code} amount exceeds supported range`, with the code as stored |

When there is a single currency and its total is in range, the cell text is
exactly what it is today. The out-of-range wording is new, for one currency or
several. The single-currency guarantee applies only to in-range totals
(SC-R02-003).
