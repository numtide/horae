# Contract: Horae API v1

The readable contract of `/api/v1`. The authoritative description is the OpenAPI document the
server generates (`GET /api/v1/openapi.json`, snapshot `crates/api/openapi/v1.json`); the draft in
[openapi-v1.draft.yaml](openapi-v1.draft.yaml) fixes the shapes before code exists. Where the two
disagree after PR F, the generated document wins and this file is corrected.

## Conventions

| Topic | Rule |
| --- | --- |
| Base path | `/api/v1` on the instance's host |
| Authentication | `Authorization: Bearer horae_pat_…` on every operation except `openapi.json`. No cookie, no query-string token (FR-010) |
| Media type | `application/json` in and out; errors `application/problem+json` |
| Ids | UUID v7 strings (FR-041) |
| Durations | integer minutes, fields ending in `_minutes` (FR-042) |
| Money | `{ "amount_minor": 1250, "currency": "EUR" }` (FR-042) |
| Instants | RFC 3339 in UTC, e.g. `2026-11-18T09:30:00Z` (FR-043) |
| Dates | `YYYY-MM-DD`; a time entry's date is the person's calendar day, as in the timesheet |
| Unknown query parameters | ignored |
| Unknown body fields | refused, 422 `unknown_field` |
| Compatibility | within v1 only additions: operations, optional parameters, response fields, error codes. Clients MUST ignore unknown response fields and treat unknown `code` values as their status (FR-050) |

### Lists

- Query: `limit` (1–1000, default 100), `cursor` (from `next_cursor`), plus each list's filters.
- Response: `{ "data": [ … ], "next_cursor": "…" }`; `next_cursor` is `null` on the last page.
- Order: by `id` ascending (creation order). A walk returns each record once while nothing
  changes underneath (SC-007).
- `updated_since` (RFC 3339) returns records whose `updated_at` is at or after it. For incremental
  sync, pass the time the previous run **started**.
- A cursor is valid for 24 hours, for the same list and the same filters. Otherwise 400
  `invalid_cursor`.

### Errors

```json
{
  "type": "about:blank",
  "title": "Conflict",
  "status": 409,
  "detail": "This person already has a running timer",
  "code": "conflict",
  "field": null
}
```

| Status | `code` | When |
| --- | --- | --- |
| 400 | `bad_request`, `invalid_cursor` | malformed JSON, parameter of the wrong type, bad cursor |
| 401 | `unauthenticated` | no token, unknown, revoked or expired token, owner inactive (one answer for all; FR-004) |
| 403 | `forbidden` | the owner may not do this in the web app |
| 404 | `not_found` | missing, or hidden by the owner's scope (a record outside scope is not found, as in the web app) |
| 409 | `conflict` or a reason code | the state forbids it (locked, approved, invoiced, no running timer, …) |
| 422 | `invalid`, `unknown_field` or a reason code | validation; `field` names the input |
| 429 | `rate_limited` | over the token's limit; `Retry-After` header in seconds |
| 500 | `internal` | `detail` withheld |

`detail` carries the reason the web app shows for the same refusal (FR-044).

### Limits

100 requests per 15 seconds per token by default, shared with the Harvest-compatible API
(FR-051). Responses carry no rate headers other than `Retry-After` on 429.

## Operations

### Description

| Method and path | Auth | Answer |
| --- | --- | --- |
| `GET /api/v1/openapi.json` | none | 200, the OpenAPI 3.1 document |

### Identity

| Method and path | Answer | Notes |
| --- | --- | --- |
| `GET /api/v1/me` | 200 `Me` | the token's owner, organization and token |
| `GET /api/v1/me/trackable` | 200 `Page<Trackable>` | project and task pairs the owner may track today (`ops::time::tracking`); `person_id` filter for a person the owner may write for |
| `GET /api/v1/me/timer` | 200 `TimeEntry` or 204 | the owner's running timer, if any |

`Me`:

```json
{
  "person": { "id": "…", "name": "Ada Lovelace", "email": "ada@example.org" },
  "organization": { "id": "…", "name": "Example Ltd" },
  "administrator": false,
  "profile": "Member",
  "token": { "id": "…", "name": "calendar sync", "expires_at": null }
}
```

`Trackable`: `{ "project_id", "project_name", "task_id", "task_name", "billable" }`.

### Time entries

| Method and path | Answer | Shared operation |
| --- | --- | --- |
| `GET /api/v1/time_entries` | 200 `Page<TimeEntry>` | `ops::time_reads::list` |
| `GET /api/v1/time_entries/{id}` | 200 `TimeEntry` | `ops::time_reads::get` |
| `POST /api/v1/time_entries` | 201 `TimeEntry`, `Location` | `ops::time::apply(Create)` |
| `PATCH /api/v1/time_entries/{id}` | 200 `TimeEntry` | `ops::time::patch` (merge, then `Update` rules) |
| `DELETE /api/v1/time_entries/{id}` | 204 | `ops::time::apply(Delete)` |
| `POST /api/v1/timers` | 201 `TimeEntry` | `ops::time::apply(StartTimer)`; stops a running timer first (D-6) |
| `POST /api/v1/time_entries/{id}/stop` | 200 `TimeEntry` | `ops::time::apply(StopTimer)` |
| `POST /api/v1/time_entries/{id}/restart` | 200 `TimeEntry` | `ops::time::apply(RestartTimer)`; stops a running timer first (D-6) |

List filters: `person_id`, `project_id`, `task_id`, `client_id`, `from`, `to` (spent date,
inclusive), `is_running`, `updated_since`. Without `person_id` the list holds every entry the
owner may read (own, managed or all, by their scope).

`TimeEntry`:

```json
{
  "id": "…", "person_id": "…", "project_id": "…", "task_id": "…",
  "spent_date": "2026-11-18",
  "minutes": 90, "rounded_minutes": null,
  "notes": "Review", "billable": true,
  "state": "open",
  "is_running": false, "timer_started_at": null,
  "start_minute": 540,
  "created_at": "2026-11-18T09:00:00Z", "updated_at": "2026-11-18T10:30:00Z"
}
```

`state` is one of `open`, `submitted`, `approved`, `invoiced` (Horae's entry states).

`NewTimeEntry` (`POST /time_entries`): `project_id`, `task_id`, `spent_date`, `minutes`
(≥ 0) required; `notes`, `billable` (default `true`; the task's eligibility still applies),
`start_minute` (0–1439), `person_id` (default the owner; another person needs the owner's write
scope over them) optional.

`TimeEntryPatch`: any of `project_id`, `task_id`, `spent_date`, `minutes`, `notes` (null
clears), `billable`, `start_minute` (null clears).

Timer start (`POST /timers`): `project_id`, `task_id` required; `notes`, `person_id` optional.
The entry is dated today in the person's time zone.

Restart is allowed on a stopped, open entry dated today in the person's time zone; otherwise 409.
Stop on an entry that is not running is 409.

### Catalogue (read-only)

| Method and path | Answer | Shared operation | Filters |
| --- | --- | --- | --- |
| `GET /api/v1/projects` | 200 `Page<Project>` | `ops::projects::reads::page` | `client_id`, `active`, `updated_since` |
| `GET /api/v1/projects/{id}` | 200 `Project` | `ops::projects::reads::get` | |
| `GET /api/v1/tasks` | 200 `Page<Task>` | `ops::projects::task_catalog::visible_page` | `project_id`, `active`, `updated_since` |
| `GET /api/v1/tasks/{id}` | 200 `Task` | `ops::projects::task_catalog::visible_get` | |
| `GET /api/v1/clients` | 200 `Page<Client>` | `ops::clients::page` | `active`, `updated_since` |
| `GET /api/v1/clients/{id}` | 200 `Client` | `ops::clients::get` | |
| `GET /api/v1/people` | 200 `Page<Person>` | `ops::people::page` | `active`, `updated_since` |
| `GET /api/v1/people/{id}` | 200 `Person` | `ops::people::get` | |

`active` defaults to `true`; `active=false` lists archived records and needs the same permission
the web app needs to see them. Fields that the web app shows only to some readers (client
address and tax id, project budgets and rates) are present only for them, and absent otherwise.
The OpenAPI document marks them optional.

## Not in v1

Writes to the catalogue, reports, invoices, approvals, OAuth2 and webhooks (spec FR-031, FR-053).
They arrive as additions.
