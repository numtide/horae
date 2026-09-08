# Contract: Harvest REST API (consumed) + inverse-of-exporter mapping

This feature **consumes Harvest's real REST API** (Harvest → Horae) as its primary source. Do not confuse it with Horae's own read-only Harvest-**compatible** exporter in `crates/horae/src/harvest/` (Horae → Harvest); that exporter is the **inverse reference** the importer inverts, not a surface this feature adds. This contract pins (A) the real Harvest API surface the importer calls, and (B) the field mapping, taken by inverting the exporter.

## A. Real Harvest API consumed (primary source)

### Authorization — OAuth2 authorization-code flow

- **Authorize** (browser redirect): Harvest's identity host authorization endpoint (`id.getharvest.com`), with the configured client id, redirect URL, and `response_type=code`. The admin signs in to Harvest and authorizes; Harvest redirects back to Horae's callback with a `code`.
- **Token exchange** (server-side): POST the `code` to Harvest's identity-host token endpoint with client id/secret and redirect URL; receive `access_token`, `refresh_token`, and `expires_in`.
- **Account id**: resolve the Harvest **account id** (identity host accounts endpoint) — required on every data call.
- **Refresh**: when `access_token` is expired, POST the stored `refresh_token` (`grant_type=refresh_token`) to the token endpoint for a fresh pair. A failed refresh (revoked/expired) → reject the run with a "reconnect Harvest" message (FR-024).
- **Storage**: `access_token`, `refresh_token`, `expires_at`, and `harvest_account_id` are stored **encrypted at rest** in `harvest_credentials` (data-model.md), never returned to the browser or logged (FR-022).

### Required request headers (every data call)

- `Authorization: Bearer <access_token>`
- `Harvest-Account-Id: <harvest_account_id>`
- `User-Agent: <Horae identifier + contact>` (Harvest requires a meaningful user agent)
- `Accept: application/json`

Data base host: Harvest's API v2 data host (`api.harvestapp.com/v2`).

### Endpoints pulled (in FK-safe order)

| Order | Harvest endpoint | Feeds Horae |
|---|---|---|
| 1 | `GET /clients` | `clients` |
| 2 | `GET /projects` | `projects` (references `client`) |
| 3 | `GET /tasks` and `GET /task_assignments` (or per-project `GET /projects/{id}/task_assignments`) | `tasks` + `project_tasks` |
| 4 | `GET /users` | reference only — resolve each entry's user to a Horae user by email (FR-010); **never written to `users`** |
| 5 | `GET /time_entries` | `time_entries` (references user/client/project/task by id) |

Each object carries a stable numeric `id` (→ provenance) and an `updated_at` (→ incremental watermark).

### Pagination

- Responses are paginated; follow `links.next` to completion, including cursor URLs whose numeric `next_page` is null. The importer requests 100 records and rejects responses exceeding 2,000 records or 10 MiB of decompressed JSON. Next links must retain the authenticated origin and collection path; redirects, cycles, user information and fragments are rejected. See Harvest's [pagination documentation](https://help.getharvest.com/api-v2/introduction/overview/pagination/).
- Time-entry pages use a capacity-one queue between blocking HTTP and async SQL. Catalog metadata is retained for reference resolution; it is not constant-memory storage. A later fetch failure rolls back the data transaction. See [performance and remaining limits](../performance.md).

### Rate limiting & backoff

- Harvest's [general-endpoint limit](https://help.getharvest.com/api-v2/introduction/overview/general/) is 100 requests per 15 seconds. The importer spaces requests by at least 160 ms and retries an HTTP `429` according to the full `Retry-After` delay. It permits six attempts per page; delays above five minutes fail for a later retry, rather than being shortened. Other consumers of the same account can still cause throttling. Data-page connect/request deadlines are 10/30 seconds, but the existing client's non-interruptible system DNS lookup can exceed the request deadline.

### Incremental sync

- Only time entries use `updated_since`; catalogs are fetched in full. An error-free committing run with source timestamps advances the time-entry watermark atomically with data/provenance, capped at the start of capture with a one-second overlap. Empty responses, missing timestamps, row errors and preview leave it unchanged. A full re-sync remains available. Provenance keeps repeated boundary records duplicate-free.
- **Deletions are not reported (known limitation)**: `updated_since` returns only changed or new records — Harvest never lists deletions this way. Re-sync is therefore **additive/updating only, not a mirror**: a record deleted in Harvest after import stays in Horae. A future "mirror-delete" mode (diffing Harvest's full id set against `harvest_import_map` to remove upstream-deleted records) is deferred (spec.md Out-of-Scope).

## B. Field mapping — invert the existing `/harvest/v2` exporter

Horae's exporter (`crates/horae/src/harvest/` — `mod.rs`, `types.rs`, `auth.rs`) already encodes the Horae↔Harvest correspondence when it emits Harvest JSON. The importer performs the **opposite** transform, reusing those field semantics so import and export stay symmetric (research.md §2).

### What the exporter already encodes

- **Time entry** (`HarvestTimeEntry`): `hours = minutes / 60`, `rounded_hours` from org rounding config, `spent_date`, `notes`, `billable`, `billable_rate = cents / 100`, `cost_rate = cents / 100`, refs to `user` / `client` / `project` / `task`.
- **Project** (`HarvestProject`): `code`, `name`, `is_active`, `is_billable`, `bill_by`, `budget_by`, `budget` (from `budget_minutes / 60` or `budget_amount_cents / 100`), `client` ref.
- **Client** (`HarvestClient`): `name`, `is_active`, `address`, `currency`.
- **Task** (`HarvestTask`): `name`, `is_active`, `billable_by_default`, `default_hourly_rate = cents / 100`.
- **User** (`HarvestUser`): `first_name`, `last_name`, `email`, rates — the importer **reads** this shape (via `GET /users`) to match users by email but never creates them.

### Inverse transforms the importer performs

| Export (existing, Horae → Harvest) | Import (this feature, Harvest → Horae) |
|---|---|
| `hours = minutes / 60` | `minutes = round(hours * 60)` |
| `rate = cents / 100` | `cents = round(rate * 100)` |
| `budget = budget_minutes / 60` or `budget_amount_cents / 100` | `budget_minutes` / `budget_amount_cents` from Harvest `budget` + `budget_by` |
| `client.currency` passthrough | `clients.currency` from the Harvest client `currency` (API) / `Currency` column (CSV) |
| enabled task → `billable_by_default` | `tasks.billable_default` + `project_tasks.billable` (from Harvest `task_assignments`) |

**Precision caveat**: `minutes = round(hours * 60)` recovers the exact original minutes only when the source supplies sufficient-precision `hours` (Harvest's API `hours` and a well-formed CSV decimal both do — `0.25` → 15, `1.5` → 90). The zero-drift reconciliation (SC-003/SC-007) assumes this; a source that pre-rounded hours to too few decimals could differ by a minute. This is a property of the source data, not the conversion.

## Boundary

- The importer **reads** `crates/horae/src/harvest/` only as a mapping reference; it does not modify it and does not route writes through it (that module is read-only by design — Constitution IV).
- The real Harvest API is reached over outbound HTTPS from the server; the OAuth **callback** is a plain Axum route beside the existing `auth::router()` (a browser redirect target, so it cannot be a `#[server]` fn) and persists only the connection credentials, not domain data (plan.md, Constitution Check).
