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

As implemented in step 4:

- Order. Clients, projects, tasks and users run newest first, by creation (their UUID v7 id), as
  Harvest orders them by creation date. Time entries run by `spent_date`, latest first, then by
  id. Before step 4 the catalogue lists ran by name; callers that showed them as received now
  show the newest first.
- Cursors. `links.next` and `links.previous` carry a `cursor` on all five lists; `first` and
  `last` carry page numbers. A cursor is a signed position: the id of the row a page continues
  after (or ends before), with its `spent_date` on time entries. Harvest's own cursor is likewise
  an "after id" position. An edit never moves a record, so a cursor walk skips nothing and
  repeats nothing whatever is edited or deleted meanwhile; a record created during a walk may
  land behind the cursor, and `updated_since` finds it next time. A cursor the server did not
  sign is 400, on the empty kinds too, and a cursor wins over `page`. Unlike `/api/v1`'s
  cursors these do not expire and are not tied to a list or its filters: a position is
  meaningful on any of them. They are signed, not encrypted, and show the row's internal id.
- Page numbers stay filled on cursor pages; a page reached by cursor may start between page
  boundaries, and then counts the partial page before it. A page-number walk is an offset walk,
  as on Harvest: a record deleted or created before the current page during the walk shifts
  later pages by one, so it can skip or repeat a record. Follow the links to avoid that.
- Update times. A project, client or task changes on any edit of its own row; a user also when
  their stored permission state (which holds `is_admin`) changes. A save that changes nothing
  keeps the time. Names shown from another record (a project's client, a time entry's project
  or task) do not move the showing record's time; a sync picks them up from that record's list.
  Times are stamped when the row is written (`clock_timestamp()`), not when its transaction
  began.
- Horizon. A list serves only records stamped before the start of the oldest transaction that
  is still writing (and before the request). A record a list returns therefore has a final
  time: no record can later commit with an earlier one, so a client that takes the latest
  `updated_at` it saw as its next `updated_since` needs no overlap. A record written by a
  transaction still running, or stamped after the oldest running write began, appears once
  those transactions end; with the 5 s statement limit on Horae's writes that is normally
  milliseconds, but a long import delays recent changes in lists until it finishes. Reads of
  one record by id are not delayed. The horizon reads `pg_stat_activity`, which shows other
  sessions' transactions only to the same database role, so every writer must use Horae's role.
- Time-entry filters: `user_id`, `client_id`, `project_id`, `task_id`, `is_billed`,
  `is_running`, `approval_status` (`unsubmitted`, `submitted`, `approved`, where approved
  includes invoiced; another value is 422), `from`, `to`, `updated_since`.
  `external_reference_id` matches nothing, because Horae keeps no external references.
- Time-entry fields: `approval_status` is `unsubmitted`, `submitted` or `approved`. A running
  timer's `hours` and `rounded_hours` include the time since it last started, and
  `hours_without_timer` is the time before that. `user_assignment`, `task_assignment`,
  `invoice` and `external_reference` are null until Horae numbers those records.
- `GET /users/me/project_assignments` is paged by number only, and refuses `updated_since` and
  `cursor` with 422 (FR-042): assignments have no update time and no id of their own, and a
  person's list fits one page.
- Empty lists: `expenses`, `expense_categories`, `estimates`, `estimate_item_categories`,
  `contacts`, `invoice_item_categories` and `roles`. Invoice payments arrive with invoices
  (step 5), since they are listed under an invoice.
- Every response carries `Cache-Control: private, no-store`: it is per caller, and list links
  echo the request's `Host` and `X-Forwarded-Proto`.

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
| 3 | `GET /company` | `wants_timestamp_timers` false (Horae stores durations), `time_format` `hours_minutes`, `clock` `24h`, `week_start_day` from the organization; settings Horae lacks (domain, date format, plan, currency display, capacity) are `null`; `invoice_feature`, `approval_feature` and `team_feature` true, `expense_feature` and `estimate_feature` false |
| 3 | `GET /users/me/project_assignments` | the pairs the Timesheet lets the caller track, grouped by project, with nested `task_assignments`; rates, budgets and timestamps `null`. An assignment's `id` is its project's id and a task assignment's `id` its task's id (decided 2026-11-19): Horae numbers neither, and an administrator tracks without an assignment row. They are unique within the list but not across people, a known gap against FR-015 |
| 5 | `GET /users/{id}/project_assignments` | |
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

## Compatibility results (FR-072)

Measured by the `harvest-compat` check; clients and pins are in
`nix/checks/harvest-compat/clients/README.md`.

| Client | Identity and picker | Time workflow | Sync |
| --- | --- | --- | --- |
| becoded/go-harvest (Go library) | passes: `users/me`, `projects`, `company`, `users/me/project_assignments` | passes: log, edit, start and end times, start (stopping the running timer), stop, restart, delete | passes: time entries, clients, projects, tasks and users by page and by `links.next` (cursors on the catalogue lists); `updated_since` after one edit |
