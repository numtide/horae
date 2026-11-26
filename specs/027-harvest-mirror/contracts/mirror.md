# Contract: mirror operations, server functions, read-only error and configuration

## Operations (`crates/server/src/ops/harvest_mirror.rs`)

Each takes `(tx, actor, input)`, requires `actor.administrator()`, writes the audit log and returns
`Done<T>`. Each one is wrapped by a server function and reached by the CLI over HTTP.

| Operation | Input | Effect | Refusals |
| --- | --- | --- | --- |
| `status` (read) | none | `MirrorStatus` (below) | not administrator |
| `enable` | `interval_secs` | `state = on`, authority = actor, schedules created, a first pass enqueued | not connected; no completed full import; spec 026 delivery 2 not applied; a running import; already on |
| `pause` | none | `state = paused`, `pause_reason = administrator`, schedules disabled | not on |
| `resume` | none | `state = on`, authority = actor, schedules enabled, a pass enqueued | not paused; connection refused and not renewed |
| `run_pass` | `scope: incremental \| full` | enqueues a pass (or returns the existing one) | not on |
| `set_interval` | `interval_secs` (60..=3600) | updates the pass schedule | out of range |
| `run_parity` | `months: nightly \| all` | enqueues a parity job | mirror off |
| `cut_over` | `override_unexplained: Option<u32>` | final pass, final parity, then `state = off`, schedules deleted, write-through enrollments removed | last parity has unexplained findings and `override_unexplained` is not that count |
| `set_write_through` (PR 10) | `person_id`, `enabled: bool` | enrollment row | `HORAE_HARVEST_WRITE_THROUGH` not set; person has no Harvest provenance |

`cut_over` runs as a job (`harvest_mirror_cutover`, one attempt, concurrency key
`harvest:<org>:cutover`) because it waits for a pass and a parity run; the server function returns
its job id.

## Server functions

POST paths beside the import ones (spec 006's convention), so the CLI can call them:

| Function | Path | JSON fields |
| --- | --- | --- |
| `harvest_mirror_status` | `/api/import/harvest/mirror/status` | none |
| `enable_harvest_mirror` | `/api/import/harvest/mirror/enable` | `interval_secs` |
| `pause_harvest_mirror` | `/api/import/harvest/mirror/pause` | none |
| `resume_harvest_mirror` | `/api/import/harvest/mirror/resume` | none |
| `run_harvest_mirror_pass` | `/api/import/harvest/mirror/run` | `scope` |
| `set_harvest_mirror_interval` | `/api/import/harvest/mirror/interval` | `interval_secs` |
| `run_harvest_parity` | `/api/import/harvest/parity/run` | `months` |
| `list_harvest_parity_runs` | `/api/import/harvest/parity/runs` | `before`, `limit` |
| `cut_over_harvest_mirror` | `/api/import/harvest/mirror/cutover` | `override_unexplained` |
| `set_harvest_write_through` | `/api/import/harvest/mirror/write-through` | `person_id`, `enabled` |

Errors use named status codes: 403 not administrator, 409 for state refusals, 422 for invalid input.

### `MirrorStatus` (`crates/types/src/harvest_mirror.rs`)

```text
state: on | paused | off
pause_reason: Option<administrator | connection_refused | authority_lost>
interval_secs, reconcile_cadence, parity_local_time
authority: { id, name }
last_pass: Option<{ job_id, started_at, finished_at, status, per_entity: [{ entity, created, updated, skipped, unsupported, errored }] }>
lag_secs: Option<i64>                 # now − oldest entity watermark
watermarks: [{ entity, at }]
recent_errors: [{ job_id, at, message }]          # last 10 passes
next_pass_at, next_reconcile_at, next_parity_at
rate_limit_wait_until: Option<timestamp>
clock_skew_secs: Option<i64>
latest_parity: Option<ParityRunSummary>           # contracts/parity.md
days_since_unexplained_difference, days_since_unexplained_error
cut_over: Option<{ at, by, parity_run_id }>
```

## Status page

In the Harvest section (`pages/importers`), visible to administrators only. It shows `MirrorStatus`,
with Pause, Resume, Start a pass, Start a full pass, Change interval, Run parity, Download detail (per
parity run) and Cut over. Cut over opens a `ConfirmDialog` stating the last parity run's counts; with
unexplained findings it asks for the count to be typed. All text goes in `horae.ftl` under the
Harvest import section.

## Read-only refusal (FR-020)

`OpError::Conflict` with code `harvest_mirror_read_only` and the message "Synced from Harvest: edit
this in Harvest while the mirror is on." Mapped as:

- server functions: 409 with the message;
- `/api/v1`: `application/problem+json`, status 409, `type` `…/problems/harvest-mirror-read-only`;
- `/harvest/v2`: 422 with `{"message": …}`, Harvest's error shape.

Guarded operations: create, update, archive, restore and delete of clients, projects (including
tasks, task settings, managers, member budgets and rates on them), tasks, people (profile fields
Harvest owns: name, email, active, rates), time entries (including timers, submit and approve) and
invoices (including status changes). Not guarded: permissions, profiles, sign-in bindings, API
tokens, OAuth, organization settings, plugins, notification preferences.

The UI shows a "Synced from Harvest" badge on mirrored records' detail and rows and disables their
edit actions with the same text as a tooltip. The read DTOs gain `synced_from_harvest: bool`.

## Configuration

| Variable | Default | Meaning |
| --- | --- | --- |
| `HORAE_HARVEST_MIRROR` | `0` | `1` lets administrators turn the mirror on; `0` keeps every mirror kind unregistered and the page hidden |
| `HORAE_HARVEST_WRITE_THROUGH` | `0` | `1` lets administrators enroll pilot people (phase 3) |

Interval, reconciliation cadence and parity time are per organization, in `harvest_mirrors`.
Turning `HORAE_HARVEST_MIRROR` off on a server whose mirror is on leaves its jobs queued (no
handler) and the records read only; the status page says so.
