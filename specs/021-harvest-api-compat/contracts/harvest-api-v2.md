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
| 3 | `GET /users/me/project_assignments` | the pairs the Timesheet lets the caller track, grouped by project, with nested `task_assignments`; rates, budgets and timestamps `null`. An assignment's `id` is its project's id and a task assignment's `id` its task's id (decided 2026-11-19): Horae numbers neither, and an administrator tracks without an assignment row. They are unique within the list but not across people, a known gap against FR-015. Step 5 closes it (*Step 5* below) |
| 5 | `GET /users/{id}/project_assignments` | *Step 5* below |
| 4 | all list fidelity above | |
| 5 | `GET /users/{id}`, `/users/{id}/teammates`, `/user_assignments`, `/task_assignments` (account-wide), `/projects/{id}/user_assignments[/{id}]`, `/projects/{id}/task_assignments[/{id}]`, `/invoices`, `/invoices/{id}`, `/invoices/{id}/payments`, `/reports/time/{clients,projects,tasks,team}`, `/reports/uninvoiced`, `/reports/project_budget` | *Step 5* below |
| 6 | `POST/PATCH/DELETE` for clients, projects, tasks, user and task assignments; `POST/PATCH /users`; invoice status changes Horae supports (draft, sent, closed) | administrators and managers, per their grants |
| 7 | OAuth2 authorize, token and `GET /accounts` on Horae's own identity paths | OQ-1 |

Everything else in Harvest's reference (estimates, expenses, contacts, payments, roles, invoice
messages that send email) stays out of scope (spec *Out of scope*).

## Step 5: catalogue and report reads (FR-015, FR-044 items 2–4, FR-045)

Planned, not implemented. Lists follow step 4: keyed by id newest first with `cursor` and
`updated_since` under the commit horizon, unless a section says otherwise. Ids are each record's
`harvest_id` ([data-model.md](../data-model.md) *Step 5*).

### Assignments and the FR-015 fix

- A **user assignment** is an `assignments` row (a person on a project); a **task assignment** is
  a `project_tasks` row (a task on a project). Each has its own `harvest_id`, so a user
  assignment's id is unique across people, and a task assignment's id is the same for everyone
  on the project, as on Harvest.
- `GET /users/me/project_assignments` and `GET /users/{id}/project_assignments` return one entry
  per user assignment of that person whose project they can track now (the Timesheet's rule:
  active project, client and task). `id` is the user assignment's; `task_assignments` holds the
  task assignments the person may track, so a restricted task appears only for its members.
  The lists take `cursor` and `updated_since` from step 5; step 4 refused both with 422.
- `updated_since` and the entry's `updated_at` are the user assignment's, as on Harvest, where
  the project assignment is the user assignment object. A change to a nested task assignment
  moves that task assignment's own `updated_at`, not the entry's; a client syncing task changes
  reads `/task_assignments?updated_since=…`. A change to who may track a restricted task moves
  neither, so a client that needs it re-reads the list.
- The ids differ from the ones step 3 served (project and task ids). Clients that cached them
  get new ones once; the release notes say so.
- **An administrator's projects without an assignment row are left out** (decided by the
  maintainer on 2026-11-23). Horae lets an administrator track any project; Harvest does not, so
  such pairs have no assignment to number. The lists show only `assignments` rows, as Harvest
  would; the administrator still writes time to those projects by project and task id. This
  partly reverses the 2026-11-19 decision in step 3's row above, which listed every pair the
  Timesheet offers. With real assignment ids, keeping those pairs would need a table of
  administrator and project pairs kept by triggers: more schema for a case Harvest does not
  have. The demo administrator has assignment rows. Imported people get theirs from the Harvest
  API import, which brings Harvest's user assignments as `assignments` rows (decided
  2026-11-23, task T080), so an imported administrator sees the list they had on Harvest. An
  organization imported earlier gets the rows on its next API import. The CSV import carries
  no assignments and creates none, so a CSV-imported organization has empty lists until people
  are assigned in Horae.
- User assignment fields: `id`; `project` (`id`, `name`, `code`); `user` (`id`, `name`);
  `is_active` true (Horae does not archive an assignment, so `is_active=false` lists nothing);
  `is_project_manager` from `project_management_assignments`; `use_default_rates` true when the
  row has no rate; `hourly_rate` from the row's rate, shown with `BillableRateReadAll`, or with
  `BillableRateReadManaged` on a project the caller manages (the project reads' rule), else
  `null`; `budget` in hours from `project_member_budgets` when the project budgets per
  person, else `null`; `created_at`, `updated_at`.
- Task assignment fields: `id`; `project`; `task` (`id`, `name`); `is_active` and `billable` from
  the row; `hourly_rate` from the row's rate under the same grant; `budget` from
  `project_task_settings` (hours, or money in the project's currency), else `null`;
  `created_at`, `updated_at`.
- Who reads them: the per-project and account-wide lists cover the projects the caller may read
  (`ProjectReadAll`, or the projects they manage). `/users/{id}/project_assignments` needs
  authority over that person (`PeopleReadAll`, or managing them); anyone reads their own through
  `users/me`. A person the caller may not see is 404, as `GET /users/{id}` answers, so the
  status does not reveal who exists; a person they see but lack authority over is 403. A user
  filter naming someone the caller may not see lists nothing.
- Time entries gain `user_assignment` and `task_assignment` (`id` and the nested ids Harvest
  documents), `null` when the row does not exist (an administrator without an assignment, or a
  task removed from the project since).

### Users and teammates

- `GET /users/{id}`: the users list's object and visibility for one person; someone the caller
  may not see is 404.
- `GET /users/{id}/teammates`: the people that user manages (`person_management_assignments`),
  with `id`, `first_name`, `last_name` and `email`; the caller needs `PeopleReadAll` or to be
  that user. A user the caller may not see is 404; one they see without that authority is 403.
- Tags are not served. The public API v2 reference has no tags resource (checked 2026-11-22),
  so there is no documented shape to serve; FR-044 item 4 was amended to match on 2026-11-23.

### Invoices

| Horae status | Harvest `state` | Served |
| --- | --- | --- |
| draft | `draft` | yes |
| sent | `open` | yes |
| paid | `paid` | yes |
| closed (imported only, spec 025) | `closed` | yes |
| void | none | no: left out of lists, 404 by id, and `state=void` is 422 (decided 2026-11-23) |

- Overdue (PR #453) is not a state on Harvest either. An overdue invoice is `open` with a
  `due_date` before the organization's today, which is how a client tells it apart; nothing is
  added for it.
- A voided invoice reopens its time in Horae, which is what deleting an invoice does on Harvest,
  so it is not served. A sync client sees it disappear, as it sees a deleted Harvest invoice.
- Fields: `id`; `client` (`id`, `name`); `number`; `purchase_order` from `po_number`; `amount`
  (total); `due_amount` (imported: Harvest's due amount; Horae's own: 0 once paid, else the
  total); `tax`, `tax2` and `discount` as percentages (Horae's own: basis points as an exact
  decimal, so 2150 is `21.5`, `null` when zero or unset; imported: the rate as Harvest wrote it);
  `tax_amount`, `tax2_amount`, `discount_amount`; `subject`; `notes`; `currency`; `state`;
  `issue_date`, `due_date`; `payment_term` (`upon receipt` for 0 days, `net 15`, `net 30`,
  `net 45` or `net 60`, otherwise `custom`); `period_start`, `period_end` (imported: Harvest's;
  Horae's own: `null`, since Horae stores no period); `paid_date` (imported: Harvest's paid date;
  Horae's own: `null` until payments exist); `sent_at`, `paid_at`, `closed_at`, `creator`,
  `estimate`, `retainer`, `recurring_invoice_id`, `client_key` `null` and `payment_options`
  empty, because Horae records none of them; `created_at`, `updated_at`.
- Money uses the invoice's currency decimals through `Amount`, as everywhere in this API.
- `line_items`, Horae's lines in `id` order (the invoice detail's order; the table has no
  position column) and imported lines by `position`:
  - Horae's time lines (one per time entry today): `kind` `Service`, `description`, `quantity`
    the line's minutes in hours (half up, two decimals), `unit_price` the rate, `amount`,
    `project` of the entry.
  - Horae's fee lines: `kind` `Service`, the line's `description`, `quantity` 1, `unit_price`
    and `amount` the fee, `project` of the fee.
  - On both, `taxed` is true when the invoice has a first tax and `taxed2` when it has a second,
    since Horae taxes the whole subtotal. `amount` is authoritative; `quantity × unit_price`
    may differ from it by rounding, as on Harvest.
  - Imported lines (`harvest_invoice_lines`): their stored kind, description, quantity, unit
    price, amount, taxed flags and project.
- On Harvest, `period_start` and `period_end` bound the dates of the work an invoice bills
  (checked 2026-11-22); imported invoices serve them as Harvest stored them. Horae's own invoices serve
  `null` for both for now (decided 2026-11-23): Horae stores no period, and deriving one from
  the lines' dates can come later.
- Filters: `client_id`, `project_id` (invoices with a line for that project), `state` (one
  value, `draft`, `open`, `paid` or `closed`, as Harvest's filter takes one; checked
  2026-11-22), `from` and `to` (on `issue_date`), `updated_since`.
- Order: newest `issue_date` first, as on Harvest (checked 2026-11-23). The reference states no
  tie-break; invoices with the same issue date run by id, newest first, which is Horae's choice.
  The list is keyed by `(issue_date, id)` as time entries are keyed by `(spent_date, id)`.
- `updated_at` moves when the invoice row or any of its lines changes.
- `GET /invoices/{id}/payments` is an empty list with the full envelope; Horae has no payment
  records yet. An unknown or void invoice is 404.
- Who reads them: invoice read grants (`ops::invoices::authorize_read` with `READ`); otherwise
  403\.
- Time entries gain `invoice` (`id`, `number`) when linked to a served invoice. `is_billed` reads
  the entry's state (`invoiced`), as spec 025 decides, so an entry billed in Harvest on an
  invoice that was not imported is billed with a `null` invoice.

### Reports

- The time reports and the uninvoiced report require `from` and `to`, accept `YYYYMMDD` and
  `YYYY-MM-DD`, and refuse a range over 365 days with 422. The project budget report takes no
  dates. Every report is paged by `page` and `per_page` only (1–2000, default 2000; no cursor,
  no `updated_since`), in the order the operation returns. Checked against the public
  reference on 2026-11-22.
- Totals are integer minutes and minor units until the response (FR-045). Hours are the
  organization's rounded minutes, which billable amounts are priced on (decided 2026-11-23).
- `/reports/time/clients`, `/projects`, `/tasks`, `/team`: one row per group and currency from
  `ops::reports::time` (group `client`, `project`, `task`, `person`), so a client with work in
  two currencies has two rows. Fields: the group's `*_id` and `*_name` (project rows also carry
  `client_id` and `client_name`), `total_hours` (rounded minutes), `billable_hours`,
  `currency`, `billable_amount` (`null` when the caller may not read a rate in the group, the
  report's own rule). Team rows add `is_contractor` false, `weekly_capacity` and `avatar_url`
  `null`. `include_fixed_fee` is an optional string: on Harvest, `true` makes the report
  calculate and include billable amounts for fixed-fee projects (checked 2026-11-23). The
  reference states no default for the time reports, so Horae treats an absent value as `false`
  unless a later check shows otherwise. Horae prices no time on fixed-fee projects, so it has no
  such amount to add and the rows are the same either way. Who reads them: the report's grants (`TimeReadAll` or `TimeReadManaged`).
- `/reports/uninvoiced`: one row per project with billable, uninvoiced time in the range, using
  the eligibility of the Uninvoiced view (PR #453: billable, stopped, not invoiced, open or
  approved). Fields: `client_id`, `client_name`, `project_id`, `project_name`, `currency`,
  `total_hours` (all of the project's time in the range), `uninvoiced_hours`,
  `uninvoiced_expenses` 0, `uninvoiced_amount`. `include_fixed_fee` (default true, as on
  Harvest) adds a fixed-fee project's uninvoiced fees to its `uninvoiced_amount` when `to` is
  on or after the project's start date, and 0 otherwise. The fees counted are the ones the
  Uninvoiced view counts as due, with due dates on or before the earlier of `to` and the
  organization's today. With `include_fixed_fee=false` the amount is time only. Projects of
  inactive clients are left out, as the Uninvoiced view leaves them out. Who reads it: the
  invoice read and preparation grants, as the Uninvoiced view.
- `/reports/project_budget`: one row per project with a budget, from
  `ops::projects::budget_progress`. Its only parameters are `page`, `per_page` and `is_active`. Fields: `project_id`,
  `project_name`, `client_id`, `client_name`, `is_active`, `budget_is_monthly` (monthly reset),
  `budget_by` (`project` for project hours, `project_cost` for project money, `task` and
  `task_fees` for task budgets in hours and money, `person` for person budgets), `budget`,
  `budget_spent`, `budget_remaining` (hours or money by `budget_by`; remaining may be
  negative). As on Harvest (checked 2026-11-22): only projects that have a budget are listed,
  `per_page` runs from 1 to 2000 with a default of 2000, and hours are rounded per the
  account's rounding setting when rounding is on.
- Rate limit: `/reports/*` count against a second window of 100 requests per 15 minutes per token
  (FR-050), beside the general one.

### OAuth scopes (spec 024)

| Routes | Scope |
| --- | --- |
| project assignments, user and task assignments, `/users/{id}`, teammates | `catalog:read` |
| invoices, invoice payments, `/reports/*` | none: OAuth tokens are refused with `insufficient_scope` (024 FR-063) |

Personal access tokens and session reads are unaffected (024 FR-064). No scope covers rates
(024 FR-063), so an OAuth token reads `hourly_rate: null` on user and task assignments whatever
its owner's rate grants. A time entry's `invoice` reference (`id`, `number`) is served to any
caller who can see the entry, an OAuth token with only `time:read` included (decided
2026-11-23). Harvest's time entries carry the reference with no permission rule, and members,
who cannot read invoices, get it too. `GET /invoices/{id}` still needs invoice access.

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
