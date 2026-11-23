# Data model: Task rate templates

Follows the maintainer's answers recorded in `research.md` (R2: zero is no rate; R3: foreign-currency
rates keep their number). The currency precision and line amount ship in the rounding PR; the rate
template ships in the templates PR (plan.md, Delivery).

## Rate template (`tasks.default_rate_cents`)

| | Before | After |
| --- | --- | --- |
| Column | `default_rate_cents bigint`, `>= 0` or null | same name, `> 0` or null |
| Meaning | money, in hundredths of `default_rate_currency` | a template: hundredths of "one unit of whatever the project bills in" |
| Currency | `default_rate_currency text`, `^[A-Z]{3}$` or null | dropped |
| Core type | `i64` | `horae_core::money::RateTemplate` (positive `i64`) |

Validation: at most two decimals when typed (`RateTemplate::parse`), positive, fits in `i64`. Who
may read or write it is unchanged (billable rate permissions).

A template has no state; it changes only through the task editor, the importers and the seed.

## Project task rate (`project_tasks.rate_cents`), unchanged

Hundredths of the project's currency (`projects.currency NOT NULL`). Set once when the task is added
to the project, from the typed rate or `RateTemplate::apply(project currency)`; later edits to the
template do not reach it.

## Currency precision (new, `horae-core`)

| Currency decimals (ISO 4217) | Examples | Step in hundredths | 1.5 h × 80.50 |
| --- | --- | --- | --- |
| 0 | JPY, KRW, CLP | 100 | 12100 (121) |
| 2 | EUR, USD, GBP, CHF | 1 | 12075 (120.75) |
| 3 | BHD, KWD, OMR | 1 | 12075 (120.750) |

`money::currency_decimals` (made public) is the only table in Rust. `currency_amount_step(text)` in
SQL lists the same zero-decimal codes; a test compares the two.

## Line amount

```text
line_amount(rate, minutes, currency) = floor((rate × minutes + 30 × step) ÷ (60 × step)) × step
```

`rate` and the result are hundredths, `minutes` the rounded minutes billed. Computed in `i128`; a
result beyond `i64` is an error, as today. Rust: `horae_core::invoice::line_amount_cents(rate, minutes, currency)`. SQL: `line_amount_cents(bigint, integer, text)`, `IMMUTABLE STRICT PARALLEL SAFE`, replacing the two-argument function.

Totals are sums of line amounts (unchanged).

## Migrations (numbers: next free when each PR is written)

Rounding PR, `NNNN_currency_line_amounts.sql`:

```text
currency_amount_step(text), line_amount_cents(bigint, integer, text) created
line_amount_cents(bigint, integer) dropped
```

Templates PR, `NNNN_task_rate_templates.sql`:

```text
tasks.default_rate_cents = 0           → NULL            (R2)
tasks.default_rate_cents, any currency → same number      (R0, R3)
tasks.default_rate_currency            → column dropped
CHECK default_rate_cents > 0
COMMENT ON COLUMN tasks.default_rate_cents
```

Neither touches `project_tasks`, `projects`, `clients`, `users`, `assignments`, `invoices`,
`invoice_line_items` or `time_entries`.

## Wire types (`horae-types`)

| Type | Change |
| --- | --- |
| `task::TaskRateEdit::Set` | `{ amount_cents, currency }` → `{ amount_cents }` (template hundredths) |
| `task::TaskCatalogEntry` | drops `default_rate_currency` |
| `task::TaskCatalog` | drops `rate_currency` |
| `task::Task` | unchanged (`default_rate_cents` only); doc says template |
| `project_creation::CreationTask` | drops `default_rate_currency` |

## External shapes

| Surface | Change |
| --- | --- |
| Harvest API task `default_hourly_rate` | none in shape; always two decimals |
| `/api/v1` | none |
| Plugin `TaskCreated.default_rate_cents` | none in shape; now a template |
| Import preview checkpoints | old ones keep working; new ones have no task currency |
