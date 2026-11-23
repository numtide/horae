# Quickstart: validating task rate templates

For whoever implements or reviews spec 026. The data model is in [data-model.md](data-model.md) and
the shapes in [contracts/interfaces.md](contracts/interfaces.md).

The feature ships in two PRs (plan.md, Delivery). The rounding PR is validated by "Domain rules"
(the `line_amount_cents` and `currency_decimals` parts), "Migration" (rounding PR) and "Rounding".
The templates PR is validated by every other section.

## Before upgrading: tasks whose rate changes meaning

For the templates PR. Run this against the database before its migration. Each row is a task whose stored rate was in a
currency other than the workspace's; after the upgrade its number applies in whatever currency a
project uses (research R3).

```sql
SELECT t.id, t.name, t.default_rate_cents, t.default_rate_currency, o.default_currency
FROM tasks t JOIN organizations o ON o.id = t.org_id
WHERE t.default_rate_cents IS NOT NULL
  AND t.default_rate_currency IS NOT NULL
  AND upper(btrim(t.default_rate_currency)) <> upper(btrim(o.default_currency));
```

Tasks with a stored zero also lose their rate (research R2):
`SELECT id, name FROM tasks WHERE default_rate_cents = 0;`

## Setup

```sh
nix develop
process-compose up postgres migrate        # database only, migrated
```

## Domain rules (no database)

```sh
cargo test -p horae-core money             # currency_decimals (rounding PR), RateTemplate (templates PR)
cargo test -p horae-core invoice           # line_amount_cents with a currency
cargo test -p horae-core --test properties # total = sum of lines, single rounding, step multiples
cargo kani -p horae-core                   # line amount proofs
```

Expected: JPY 80.50 × 90 min = 12100, JPY 5.00 × 30 min = 300, EUR 80.50 × 90 min = 12075, and a
rate and duration whose exact amount is 120.496 JPY give 12000.

## Migration

Rounding PR:

```sh
DATABASE_URL=… cargo test -p horae --features server --test integration line_amount_cents
```

Expected: the Rust and SQL line amounts agree for every listed currency and an unknown code.

Templates PR:

```sh
DATABASE_URL=… cargo test -p horae-server task_rate_templates
```

Expected: rates with the workspace currency, another currency and none keep their number; zeros
become no rate; `default_rate_currency` no longer exists; `project_tasks` rows are identical.

## Applying templates

```sh
DATABASE_URL=… cargo test -p horae --features server project_creation
DATABASE_URL=… cargo test -p horae --features server --test new_project_screen
```

Manual check in the running app (`process-compose up`, sign in as Admin):

1. Tasks → edit "Design", set the default rate to 80.5. The list shows "80.50 / h" and the note that
   it takes each project's currency.
1. New Project for a client billed in USD, billing by task rate, add "Design". The row shows
   "USD 80.50 / h" before saving. Save; the project's task rate is USD 80.50.
1. Open an existing EUR project's editor (billing by task rate), add "Design" with the rate empty,
   save: EUR 80.50, no error.
1. Do the same on a JPY project (New Project offers only EUR, CHF, USD and GBP, so import one or set
   `projects.currency = 'JPY'` on a seeded project in your own database): JPY 80.50.
1. Change the task's default to 90.00; every project keeps 80.50.

## Rounding

With a JPY project (import one, or set `projects.currency = 'JPY'` on a seeded project in your own
database), a task rate of 80.50, entries of 90 minutes and, at 5.00, 30 minutes:

1. Invoices → new invoice for the client: the preview lines are JPY 121.00 and JPY 3.00, the
   subtotal JPY 124.00 (the display keeps two decimals for every currency).
1. Create it, download the PDF: the same numbers.
1. Reports → time report for the period, grouped by project or task, and its CSV export: billable
   amounts of 121 and 3 whole yen.

## Imports and interfaces

```sh
DATABASE_URL=… cargo test -p horae-server importers::harvest
DATABASE_URL=… cargo test -p horae-harvest-api tasks
DATABASE_URL=… cargo test -p horae-api openapi   # snapshot unchanged, no UPDATE_OPENAPI
```

Expected: Harvest's 80.5 is stored as 8050, 0 and null as no rate; a CSV task has no currency; a
checkpoint saved with `default_rate_currency` still commits; `/v2/tasks` keeps its shape.

## Whole suite

```sh
cargo clippy -p horae-server --all-targets && cargo clippy -p horae --features server
cargo run -p horae-dioxus-lint
cargo test -p horae-i18n
nix fmt && nix flake check
```
