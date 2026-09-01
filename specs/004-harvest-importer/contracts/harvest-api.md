# Reference: Existing Harvest-Compatible API (`/harvest/v2/*`)

This is **not a surface this feature adds** — it is the authority the importer inverts. Horae already ships a read-only Harvest-compatible API in `crates/horae/src/harvest/` (`mod.rs`, `types.rs`, `auth.rs`) that emits Horae data in Harvest's JSON shape. The importer maps in the **opposite direction** (Harvest → Horae), reusing that module's field semantics so import and export stay symmetric (research.md §2).

## What the existing module already encodes

From `crates/horae/src/harvest/`:

- **Time entry** (`HarvestTimeEntry`): `hours = minutes / 60`, `rounded_hours` from org rounding config, `spent_date`, `notes`, `billable`, `billable_rate = cents / 100`, `cost_rate = cents / 100`, refs to `user` / `client` / `project` / `task`.
- **Project** (`HarvestProject`): `code`, `name`, `is_active`, `is_billable`, `bill_by`, `budget_by`, `budget` (from `budget_minutes / 60` or `budget_amount_cents / 100`), `client` ref.
- **Client** (`HarvestClient`): `name`, `is_active`, `address`, `currency`.
- **Task** (`HarvestTask`): `name`, `is_active`, `billable_by_default`, `default_hourly_rate = cents / 100`.
- **User** (`HarvestUser`): `first_name`, `last_name`, `email`, rates — the importer **reads** this shape to match users by email but never creates them.

## Inverse transforms the importer performs

| Export (existing, Horae → Harvest) | Import (this feature, Harvest → Horae) |
|---|---|
| `hours = minutes / 60` | `minutes = round(hours * 60)` |
| `rate = cents / 100` | `cents = round(rate * 100)` |
| `budget = budget_minutes / 60` or `budget_amount_cents / 100` | (supplementary; default `none` in v1) |
| `client.currency` passthrough | `clients.currency` from `Currency` column |
| enabled task → `billable_by_default` | `tasks.billable_default` + `project_tasks.billable` |

## Boundary

- The importer **reads** `crates/horae/src/harvest/` only as a mapping reference; it does not modify it and does not route writes through it (that module is read-only by design — Constitution IV).
- A future OAuth2 API pull would consume this same Harvest JSON shape as an alternate source adapter, feeding the identical mapping/matching engine (research.md §10). Not built in v1.
