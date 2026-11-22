# Interface contracts: Task rate templates

## Server functions (`horae-types`)

`TaskRateEdit` (internally tagged by `action`, `deny_unknown_fields`):

```json
{"action": "preserve"}
{"action": "clear"}
{"action": "set", "amount_cents": 8050}
```

`amount_cents` is the template in hundredths, positive (recommended R2; `>= 0` if zero stays a
template). Refused: `{"action": "set", "amount_cents": 8050, "currency": "EUR"}`, a fractional or
negative `amount_cents`, and any other key.

`TaskCatalogEntry` and `CreationTask` lose `default_rate_currency`; `TaskCatalog` loses
`rate_currency`. `default_rate_cents` stays, omitted when the reader may not see rates.

Errors removed from the operations:

- "Task rate: currency is unknown or incompatible; enter an explicit rate in the project currency"
  (`ops/projects/creation/finalize.rs`)
- "Enter an explicit task rate in the project currency." (`ops/projects/editor/associations.rs`)
- "Task rate currency changed. Reload the task and enter its rate again"
  (`ops/projects/task_writes.rs`)

## Harvest-compatible API

`GET /v2/tasks`, `GET /v2/tasks/{id}` (and `/harvest/v2/…`): unchanged shape.

`default_hourly_rate` stays a JSON number or `null`. It is the template written by the existing
`Amount` serializer with no currency (two decimals), never rounded to a currency because it has
none.

## `/api/v1`

No change. Tasks expose no rate; `crates/api/openapi/v1.json` stays byte-identical.

## Plugin events

`TaskCreated { default_rate_cents: Option<i64>, … }`: unchanged shape; the value is a template.

## SQL available to the plugin read role

`line_amount_cents(bigint, integer)` is replaced by `line_amount_cents(bigint, integer, text)` and
`currency_amount_step(text)`, both added to the allowlist in `crates/server/src/plugin/database.rs`.
