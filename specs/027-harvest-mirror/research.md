# Research: Harvest mirror mode, parity checks and the durable jobs crate

Sources: Harvest's public API v2 documentation (read 2026-11-25), the `underway` 0.2.0 crate source
(MIT or Apache-2.0, read for ideas; nothing is copied), and Horae at `f4e142bd`.

## R-1. Harvest API limits

Read from the public docs:

| Fact | Source |
| --- | --- |
| General requests: 100 per 15 seconds | `api-v2/introduction/overview/general/` |
| Reports API requests: 100 per 15 minutes | same page |
| Over the limit: HTTP 429, with the seconds to wait in `Retry-After` | same page |
| A request without `User-Agent` gets 400 | same page |
| `per_page` defaults to and tops out at 2000 (invoices default to 100); a larger value is 422 | `overview/pagination/`, `invoices/invoices/` |
| Follow the `links.next` URL instead of building page URLs; `cursor` beats `page` | `overview/pagination/` |
| Time reports (`/v2/reports/time/clients`, `projects`, `tasks`, `team`) need `from` and `to`, at most 365 days apart | `reports-api/reports/time-reports/` |
| They return `total_hours`, `billable_hours`, `currency`, `billable_amount`; the amounts only for administrators and managers who can see billable rates | same page |
| `include_fixed_fee=true` adds billable amounts for fixed-fee projects | same page |
| `/v2/reports/uninvoiced` needs `from` and `to` (365 days at most); returns per project `total_hours`, `uninvoiced_hours`, `uninvoiced_expenses`, `uninvoiced_amount`, `currency` | `reports-api/reports/uninvoiced-report/` |
| `/v2/invoices` needs an administrator, or a manager who can create and edit invoices | `invoices-api/invoices/invoices/` |
| Payments are listed per invoice only (`/v2/invoices/{id}/payments`) | `invoices-api/invoices/invoice-payments/` |

Decision: one limiter per Harvest account in the server process with two buckets, general (90 per
15 s, leaving room for a person browsing Harvest's own API tools) and reports (90 per 15 min). Every
job that calls Harvest takes tokens from it. The existing HTTP client already retries 429, 502, 503
and 504 with `Retry-After` (`crates/harvest/src/http.rs`), so the limiter only needs to keep a pass
from tripping the limit, not to recover from it.

Rationale: the docs do not say whether the two limits share a counter. Two buckets are right if they
do not, and the general bucket's margin covers the reports calls if they do.

Alternatives considered: a limiter in the database, shared across server instances. Shipyard runs one
instance; if a second appears, the 429 handling still holds and the margin halves. Not worth a table.

## R-2. `updated_since` semantics and the watermark

Every list the mirror needs takes `updated_since`: users, clients, projects, tasks, user assignments,
task assignments, time entries, invoices, and payments per invoice. The docs give the type as an ISO
8601 UTC date and time and do not say whether the bound is inclusive. They mention no deletion feed
and no webhooks for v2.

Decision:

- A step's watermark is the time the step started (Horae's clock) minus a margin of 5 minutes. The
  margin covers the unknown inclusivity, clock skew up to a few minutes, and records committed in
  Harvest while the step paged through. Re-reading a record costs a skip (FR-014).
- The watermark is stored per entity in `harvest_credentials.synced_watermark`, the store the
  importer already uses for time entries and invoices. The mirror writes the other six keys.
- A step that ends with record errors keeps its old watermark, as the importer does, so the next
  pass tries those records again.
- The status page shows clock skew by comparing Harvest's `Date` response header with Horae's clock,
  and warns above 2 minutes.

Rationale: the importer already advances its watermark to its capture start; the mirror keeps that
rule and adds the margin.

Alternatives considered: the greatest `updated_at` seen in a step. It uses Harvest's clock, but a
record committed late with an earlier `updated_at` than one already seen would be skipped forever.

Not confirmed from the docs, to check in the rehearsal (T073):

- whether a payment moves its invoice's `updated_at` (spec 025 relies on `due_amount` changing; if the
  invoice does not move, the invoice step also lists invoices whose state is open, which are few);
- whether moving a project to another client moves the project's time entries' `updated_at`;
- whether archiving a project or changing a task assignment's rate moves anything but the
  assignment;
- whether `updated_since` is inclusive (harmless either way with the margin).

## R-3. The importer's current shape and what the mirror reuses

- `run_api_import_with_http` takes the import lock (an advisory lock per organization), refreshes the
  token, reads every catalog list in full on every run, streams time entries (with `updated_since` on
  incremental runs) and invoices, and advances the time-entry and invoice watermarks on an
  error-free commit.
- Mapping and resolution live in `importers/harvest/{resolve,apply,parents,invoices}.rs`; project task
  links are derived during resolution (`resolve.rs` inserts `project_tasks`), not read from Harvest's
  task assignments.
- Provenance is `harvest_import_map` (entity type, Harvest id, Horae id, Harvest `updated_at`, Horae
  `updated_at`), which is how an unchanged record is skipped.

Decision: the mirror pass is a new driver over the same resolve and apply code. It differs from an
incremental import in three ways: every catalog step uses `updated_since`; task and user assignments
become steps of their own, with `harvest_entity_type` gaining `task_assignment` and
`user_assignment`; and each step saves its watermark when it finishes instead of once at the end.

Horae has no general project membership. A task assignment maps onto `project_tasks` (billable,
rate, active). A user assignment maps onto what Horae models: `project_management_assignments` when
`is_project_manager` is set, and `project_member_budgets` when the assignment has a budget. Its
other fields (the person's hourly rate on the project, `use_default_rates`) have no Horae column
yet; they are counted as unsupported fields and left out of parity, and the plan's open item asks
whether to add them. The import lock stays, so a pass and a manual
import can never run together.

Rationale: FR-012 asks for the importer's mapping, and the provenance table already makes upserts
idempotent.

Alternatives considered: running the existing incremental import on a schedule. It reads every
catalog in full and has no assignment step; at 5-minute intervals the full catalog reads would be
the bulk of all requests.

## R-4. Inventory of background work

What Horae runs in the background today, and what the backlog and the roadmap need. It justifies the
crate's mechanisms; this feature implements only the mirror and parity kinds.

| Work | Kind of job | Exists? | Needs |
| --- | --- | --- | --- |
| Forgotten timers | periodic per organization | yes, `scheduler.rs` | schedule, single leader |
| Budget alerts | periodic | yes, `notifications::budgets::sweep` | schedule, single leader |
| Expired OAuth codes and tokens | maintenance | yes, `ops::oauth::delete_expired` | schedule |
| Missing-time and submit-week reminders | per person at a local time | no | person time zone schedules, idempotency per person and week |
| Overdue invoice reminders | per organization daily | no | organization time zone, idempotency per invoice and day |
| Recurring invoices | per schedule | out of scope | time zone schedules, idempotency per occurrence |
| Mirror passes, reconciliation, parity | per organization | this feature | concurrency key, heartbeat, rate limit per service |
| Email delivery | reliable delivery | budget email only | retries per kind, idempotency key |
| Plugin events and webhooks | reliable delivery | yes, the outbox | retries per kind |
| Push notifications (Android app) | reliable delivery | no | retries, idempotency |
| Harvest API and CSV imports | heavy, on demand | yes | progress, cancel, checkpoints |
| Large CSV, XLSX and PDF exports, invoice PDFs | heavy, on demand | synchronous today | progress, cancel, retention of the result |
| Account data export and deletion | heavy, on demand | no | progress, retention |
| Calendar and accounting syncs | external sync | no | concurrency key, rate limit per service |
| Expired sessions, import error files, finished jobs, audit log retention | maintenance | partly (`jobs::cleanup`) | schedule, retention per kind |

Requirements it implies: schedules in an organization's or person's time zone; concurrency keys;
idempotency keys (exist); retry policy per kind; delayed jobs and priorities; a rate limit per
external service; progress, cancel and retention per kind.

## R-5. What to take from `underway` 0.2.0

Read: `migrations/*.sql`, `src/queue.rs`, `src/worker.rs`, `src/scheduler.rs`.

| Idea | How underway does it | Horae's version |
| --- | --- | --- |
| Concurrency key | A partial unique index on `(queue, concurrency_key)` where the state is pending or in progress; the worker takes a transaction-scoped advisory lock on the key (or the task id) for the whole run | The same partial unique index on `(org_id, kind, concurrency_key)` for `queued` and `running`; enqueue inserts with `ON CONFLICT DO NOTHING` and returns the existing job found by a bounded lookup. No advisory lock: the index already admits one queued or running job per key, so a claim cannot start a second one. Underway holds its lock for the run because it runs a task inside one transaction; Horae's handlers commit in batches, so the claim token and the heartbeat fence the run instead |
| Heartbeat | Columns `heartbeat` (interval, default 30 s) and `last_heartbeat_at`; a spawned task writes the heartbeat; dequeue also takes in-progress tasks whose heartbeat is older than the interval | Horae already renews a 300 s lease every 100 s (`jobs/lease.rs`). The change: a `heartbeat_at` column written every 15 s by the existing monitor, a stale limit of 60 s per kind (written at claim from the handler), claims look at the heartbeat, and the fixed lease goes. A long pass is never reclaimed while it beats; a dead one is reclaimed after a minute instead of up to five |
| Retry policy and attempts | A composite type (max attempts, initial interval, max interval, coefficient) per task, and a `task_attempt` table | A `RetryPolicy` per kind in code (not per row), a `horae_job_attempts` table, and the existing `max_attempts` column kept for the import kinds' configured value (`HORAE_JOB_MAX_ATTEMPTS`) |
| Delay and priority | `delay` interval and `priority` integer columns | `available_at` exists; enqueue gains `run_at`. A `priority` smallint, claimed in descending order |
| Scheduler | One schedule per queue (cron expression and IANA zone, via `jiff` and `jiff-cron`); singleton by a session advisory lock keyed on the queue name; sleeps until the next firing | Many schedules in a table, each enqueuing a kind; cadences limited to every N seconds, daily at a time, weekly on a day at a time; zones from `chrono-tz` (already a dependency), resolved from UTC, the organization or a person; singleton by `pg_try_advisory_lock` on a dedicated connection; each firing enqueued with the idempotency key `schedule:<id>:<due time>` |
| LISTEN/NOTIFY | A trigger notifies `task_change` on pending rows; workers listen and also poll | Optional (T029). The 2-second poll is fine for the mirror's cadence |
| Dead-letter queue | A second queue per queue | Not taken. Failed jobs stay listed and retryable (`horae jobs retry`) |
| Multi-step jobs | Steps chained through queue rows | Not taken. The mirror resumes from watermarks |
| Task timeout | Per task, default 15 minutes | Not taken as a hard limit: a full pass can take longer. Cancellation already exists |

Decision: write these in `horae-jobs` (Rust 2024, sqlx macros), no dependency on underway, jiff or a
cron parser.

Rationale: the maintainer ruled out an external queue (Clarifications). A cron parser would serve
three cadences; `chrono-tz` already resolves zones for `timezone.rs`.

## R-6. Crate boundary

Decision: `crates/jobs` (`horae-jobs`) depends on sqlx, tokio, serde, serde_json, uuid, chrono,
chrono-tz, anyhow, thiserror and tracing. It owns the generic columns of `horae_jobs`,
`horae_job_attempts`, `horae_job_schedules` and `horae_outbox`, the worker loop, claim, heartbeat,
cancel, retry, status, list and the scheduler loop. It exposes:

- `Registry` with `register(handler)`, and `Handler` (kind, retry policy, stale limit, `run(ctx, payload)`), see [contracts/jobs-crate.md](contracts/jobs-crate.md);
- `JobContext` with the lease operations that exist today (`load_checkpoint`, `save_checkpoint`,
  `run`, cancellation) and the pool;
- `enqueue_in(tx, Enqueue { org_id, kind, payload, idempotency_key, concurrency_key, run_at, priority, max_attempts })`.

What stays in `horae-server`: `JobPayload` for the import kinds and its handlers, account generations
(`check_account_generation`), uploads, report chunks, `error_kind`, retry availability and the import
start functions. The columns the server owns stay in `horae_jobs`; the crate leaves them alone, and
the handler's error carries an optional `error_kind` string that the crate writes as given (the
check constraint still validates it).

Migrations stay in `crates/server/migrations` (AGENTS.md: one migrator), so `horae-jobs`'s tests use
`#[sqlx::test(migrator = …)]` with the server's migrations through a small `test-support` path, as
`horae-server` does.

Rationale: the import kinds are where all the special cases live (account switch, previews, uploads);
keeping them in the server is what makes the crate small.

Build time: measured as the types/server/api split measured it, warm, after touching one file.
Commands: `cargo build -p horae --features server` after touching `crates/server/src/jobs.rs` (then
`crates/jobs/src/lib.rs`), and `cargo test -p horae-server --no-run` after the same touch. Record
before and after in PR 1 (FR-004).

## R-7. Read-only enforcement

Decision: the shared operations check a single predicate, `mirror::guard(tx, org_id, entity, id)`,
that refuses when the mirror is on and the record has Harvest provenance, or when a new record of a
mirrored kind is being created. It returns `OpError::Conflict` with a stable code
(`harvest_mirror_read_only`) that the server functions, `/api/v1` (problem type) and `/harvest/v2`
(422 with Harvest's error shape) map like other conflicts. The mirror's driver and write-through call
the importer's writers and their own operations, which do not call the guard.

Rationale: Principle IV puts every rule in the operations; one guard used by each mutating operation
of the six mirrored kinds keeps the surfaces honest.

The list of operations to guard is the mutating functions under `ops/{clients,projects,people,time, invoices}` that create or change Harvest-owned fields. Permissions, profiles, tokens and settings are
not guarded.

## R-8. Parity design

- Layer 1 lists every entity from Harvest in full (time entries in yearly `from`/`to` windows, 2000
  per page). For 100,000 entries that is about 50 requests plus the catalogs, well inside the general
  limit.
- Layer 2 needs 4 time reports and 1 uninvoiced report per month: 5 requests per month. The nightly
  window (current month and the 3 before) is 20 requests. Rotating older months at 12 per night (60
  requests) keeps one night under 100 and covers 3 years within a month (FR-035).
- Layer 3 reads invoices from the same lists as layer 1.
- Horae's side of layer 2 uses `ops::reports::time` and the uninvoiced view under the mirror
  administrator's actor, so the comparison runs the same code users see.
- Each finding has a fingerprint (hash of layer, entity, ids or aggregate key, class and the values).
  A finding is unexplained when its fingerprint was also in the previous completed run (FR-032).
- Equal pairs are only counted; storing them would write a row per record per night.
- Fixed-fee projects: Harvest's report adds fixed-fee amounts only with `include_fixed_fee=true`.
  Horae's reports compute hourly amounts. The checker requests `include_fixed_fee=false` and
  compares fixed fees in the invoice layer. To confirm in the rehearsal.

Rounding rule (FR-031), for an invoice with `n` time lines in a currency with minor unit `u`: the
divergence is known when `|horae_total - harvest_total| <= n * u / 2` and `horae_total` equals the sum
of Horae's rounded lines. For a report aggregate over `n` entries, the same bound with entries. The
comparison is in integer minor units (Constitution I): `2 * |difference| <= n` in units.

## R-9. Alerts

Decision: an unexplained run sets `alert_run_id` on the mirror row. The app's navigation shows a dot
on the Harvest section to administrators while it is set, the status page a banner, and when
`MailConfig` is present one email per alerted run goes to the organization's administrators through
the existing mail transport. A clean run clears it.

Alternatives considered: a plugin event. Possible later through the outbox; no consumer needs it now.

## R-10. Write-through

- Harvest accepts `user_id` on `POST /v2/time_entries` for administrators and managers, and has
  `PATCH`, `DELETE`, `PATCH …/restart` and `PATCH …/stop`.
- The time operations, for an enrolled person, call Harvest inside the operation before their own
  SQL, then write with the returned id and `updated_at` into provenance, so the next pass skips it.
- If Horae's commit fails after Harvest accepted, the next pass brings the record in (Harvest wins).
  If Harvest times out without an answer, Horae refuses; the pass then shows whether Harvest applied
  it.
- The operation runs the HTTP call off the async runtime (`spawn_blocking`, as the importer does) with
  a 10-second timeout.

## R-11. Exit criterion (OQ-3)

Decided on 2026-11-26: four consecutive weeks with zero unexplained differences, one complete month close in that
period with reports and invoices reconciled, and zero unexplained mirror errors. "Unexplained mirror
error" means a pass error not caused by Harvest's availability (5xx, timeouts) or by a record Harvest
itself holds invalid. The status page counts days since the last unexplained difference and since the
last unexplained error, so the criterion can be read off it.
