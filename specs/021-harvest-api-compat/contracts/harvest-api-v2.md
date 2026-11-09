# Contract: Horae's Harvest-compatible API v2

Supersedes [001's read-only contract](../../001-time-tracking-invoicing/contracts/harvest-api.md)
step by step: each delivery step below replaces the matching part of it. Field lists follow
Harvest's public API v2 reference; this document records only where Horae differs or decides.

## Base paths

- Canonical: `/harvest/v2/...`.
- Aliases served identically (FR-026): `/v2/...`, `/api/v2/...`, and any path with a `.json`
  suffix. A client configured with base URL `https://<host>` or `https://<host>/api` therefore
  works with a one-line host change.

## Authentication (step 2)

| Input | Accepted as |
| --- | --- |
| `Authorization: Bearer <token>` | personal access token (OAuth2 access token from step 7) |
| `access_token=<token>` query parameter | same |
| `Harvest-Account-Id` header or `account_id` query | optional; when present it must equal the organization's account id, else 403 |
| Session cookie | reads only (D-7) |
| `User-Agent` | not required (FR-008) |

| Case | Status |
| --- | --- |
| No credentials | 401 |
| Unknown, revoked or expired token, or inactive owner | 401 |
| Valid token, wrong account id | 403 |
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

Integers, per organization and kind. Imported records use their Harvest id. During the
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
| 3 | `POST /time_entries` | duration mode (`hours`) or start/end mode (`started_time`, `ended_time`) per the company's `wants_timestamp_timers`; without `hours` or times, starts a timer (D-6) |
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

- Each write runs the same shared operation as the web app (`commands::apply` for time entries)
  with the token's person and organization. The API holds no validation of its own.
- A write that the web app would refuse is refused with the mapped status: validation 422,
  permission 403, not found 404, conflict (daily cap, locked entry) 409 until recording says
  otherwise.
- Parameters are read from a JSON body, a form body or the query string (FR-025).
