# Contract: Horae's Harvest-compatible API v2

Supersedes [001's read-only contract](../../001-time-tracking-invoicing/contracts/harvest-api.md)
step by step: each delivery step below replaces the matching part of it. Field lists follow
Harvest's public API v2 reference; this document records only where Horae differs or decides.

## Base paths

- Canonical: `/harvest/v2/...`.
- Aliases served identically (FR-026): `/v2/...` and any path with a `.json` suffix. A client
  configured with base URL `https://<host>` therefore works with a one-line host change.
- Amended by spec 022 FR-052: the `/api/v2/...` alias is removed, because `/api/` belongs to
  Horae's own API. A client configured with base URL `https://<host>/api` switches to the host
  root or `https://<host>/harvest`.

## Authentication (step 2)

| Input | Accepted as |
| --- | --- |
| `Authorization: Bearer <token>` | personal access token (OAuth2 access token from step 7) |
| `access_token=<token>` query parameter | same |
| `Harvest-Account-Id` header or `account_id` query | names the organization (`organizations.api_account_id`). Optional while the token's account has one membership; with several and no header, 403 `account_required`. When present it must name one of the account's memberships, else 403; on a tenant host it must also be the host's tenant, else 403 (plan.md *Layering*) |
| Session cookie | reads only (D-7) |
| `User-Agent` | not required (FR-008) |

| Case | Status |
| --- | --- |
| No credentials | 401 |
| Unknown, revoked or expired token, or inactive owner | 401 |
| Valid token, wrong account id | 403 |
| Valid token, several memberships, no account id | 403 `account_required` |
| Cookie only, on a write | 401 |
| Permission denied by the owner's role or grants | 403 |

## Error bodies (FR-024)

| Status | Body |
| --- | --- |
| 401 | `{"error": "invalid_token", "error_description": "<text>"}` |
| 403, 404 | `{"error": "<code>", "error_description": "<text>"}` |
| 422 | `{"message": "<text>"}` |
| 429 | `{"message": "<text>", "retry_after": <seconds>}` plus `Retry-After` header |

Provisional until recorded from the test account (research R-11). The text is Horae's own.

## Units (FR-020, FR-021)

- Hours: JSON number, minutes ÷ 60 rounded half up to 2 decimals. Input accepts any decimal and
  rounds half up to whole minutes.
- Money: JSON number in major units with the currency's own decimals (JPY 0, EUR 2, BHD 3).
  Horae stores every currency in hundredths, so a zero-decimal amount that has hundredths keeps
  them (`JPY 1000.50`) rather than lose them. A task rate of unknown currency has two decimals.
  Input with more decimals than the currency allows is 422.
- No floating point on either side (research R-4).

## Ids (step 1, FR-015–FR-018)

Integers, per organization and kind: each record's `harvest_id`, the id in this API (Horae's
own identity stays the UUID). Imported records use their Harvest id when no record of the kind
has it; otherwise they get a fresh id and the import report says so. A number never names
another record. During the
transition period (D-1) a UUID is also accepted wherever an id is accepted; responses carry
integers only.

## Lists (step 4, FR-040–FR-043)

- Envelope: `{<kind>: [...], per_page, total_pages, total_entries, next_page, previous_page, page, links: {first, next, previous, last}}`. Links are absolute URLs on the request's host.
- `per_page` 1–2000, default 2000; outside that range 422.
- Catalogue lists (clients, contacts, projects, tasks, users, assignments, invoices, estimates,
  roles and their categories) also accept `cursor`; every envelope field stays filled.
- `updated_since` (ISO 8601) is honoured on every kind that Harvest documents it for.
- Kinds Horae does not have return an empty list with the full envelope (D-2), and
  `GET /company` reports their feature flags as off.

## Rate limits (FR-050)

100 requests per 15 seconds per token; 100 per 15 minutes for `/reports/*`. Exceeding it is 429.

## Endpoints by delivery step

| Step | Method and path | Notes |
| --- | --- | --- |
| 1 | existing reads with integer ids | `users/me`, `users`, `time_entries`, `projects`, `clients`, `tasks` list and get |
| 2 | same, with token auth | |
| 3 | `POST /time_entries` | duration mode (`hours`) or start/end mode (`started_time`, `ended_time`) whatever the company setting (FR-031); without `hours` or times, starts a timer (D-6) |
| 3 | `PATCH /time_entries/{id}` | |
| 3 | `DELETE /time_entries/{id}` | |
| 3 | `PATCH /time_entries/{id}/stop`, `/restart` | restart stops any other running timer (D-6) |
| 3 | `GET /company` | `wants_timestamp_timers` false (Horae stores durations), feature flags for absent domains off |
| 3 | `GET /users/me/project_assignments`, `/users/{id}/project_assignments` | with nested `task_assignments` |
| 4 | all list fidelity above | |
| 5 | `GET /users/{id}`, `/projects/{id}/user_assignments`, `/task_assignments`, `/user_assignments`, `/task_assignments` (account-wide), `/users/{id}/teammates`, `/invoices`, `/invoices/{id}`, `/reports/time/*`, `/reports/uninvoiced`, `/reports/project_budget` | |
| 6 | `POST/PATCH/DELETE` for clients, projects, tasks, user and task assignments; `POST/PATCH /users`; invoice status changes Horae supports (draft, sent, closed) | administrators and managers, per their grants |
| 7 | OAuth2 authorize, token and `GET /accounts` on Horae's own identity paths | OQ-1 |

Everything else in Harvest's reference (estimates, expenses, contacts, payments, roles, invoice
messages that send email) stays out of scope (spec *Out of scope*).

## Write semantics (step 3, FR-030–FR-032)

- Each write runs the same shared operation as the web app (`ops::time::apply` and
  `ops::time::patch`) with the token's person and organization. The API holds no validation of
  its own.
- A request authenticated only by the session cookie is refused with 401 `invalid_token` before
  anything is read or written.
- A write that the web app would refuse is refused with the mapped status: validation 422,
  permission 403, not found 404, conflict (daily cap, locked entry, a restart of another day's
  entry, a stop of a stopped timer) 409 until recording says otherwise. An entry the token's owner
  may not read is 404, as in the web app.
- Parameters are read from a JSON body, a form body or the query string (FR-025); a body value
  wins over a query value of the same name. Ids are integers (or UUIDs, D-1); a number no record
  has is passed on and refused by the operation.
- Times of day: `started_time` is stored as the entry's start minute and `ended_time` gives the
  minutes from that start (decided 2026-11-19, no migration). `ended_time` before `started_time`
  (which is also how a span past midnight reads) is the operation's 422 `Minutes cannot be negative`. With `ended_time`, `hours` is ignored; `started_time` with `hours` keeps both;
  `started_time` alone is 422; on `PATCH`, `ended_time` alone keeps the entry's start. The
  Timesheet snaps a start to its 15-minute grid, so a start off the grid comes back moved.
- A timer is filed on the person's own today, whatever `spent_date` names; `spent_date` is still
  required, as on Harvest.
- `POST` answers 201 with the entry; `PATCH`, `stop` and `restart` 200 with the entry; `DELETE`
  200 with an empty body. A stop the daily limit cut short adds `horae_notice`, which clients may
  ignore.
