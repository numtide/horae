# Data model: Currency-separated totals (R02)

There are no schema, query or wire-format changes.

- **`ReportRow`** (`crates/horae/src/models.rs`) is unchanged:
  - `billable_cents: i64` with its billing `currency`;
  - `cost_cents: Option<i64>` (absent when withheld) with `cost_currency`, the
    organization's default currency.
- **Currency total** is derived for display only and is never stored or
  exported. It is keyed by the exact stored currency code, and in-range totals
  are labelled through the existing trimming formatter (see
  [research](research.md#currency-identity-is-the-exact-stored-code)) and
  either an exact `i64` minor-unit sum or "out of range".
- **Withheld** is the result for a whole column when any of its amounts is
  absent.

[contracts/total-cells.md](contracts/total-cells.md) defines how these become
cell text.
