# Contract: `horae-jobs`

The crate's public surface, the job kinds `horae-server` registers, and their policies. Signatures
are indicative; names and behaviour are the contract.

## Enqueue

```rust
pub struct Enqueue<'a> {
    pub org_id: Uuid,
    pub kind: &'a str,
    pub payload: serde_json::Value,       // wrapped in the existing {version, payload} envelope
    pub idempotency_key: Option<&'a str>, // existing (org_id, kind, idempotency_key) uniqueness
    pub concurrency_key: Option<&'a str>, // FR-005
    pub run_at: Option<DateTime<Utc>>,    // FR-008, default now
    pub priority: i16,                    // FR-008, default 0
    pub max_attempts: i32,                // required; usually the kind's RetryPolicy::max_attempts
}

pub enum Enqueued { Created(Uuid), Existing(Uuid) }
pub enum EnqueueError { Conflict, Contended, Database(sqlx::Error) }

pub async fn enqueue_in(tx: &mut PgConnection, request: Enqueue<'_>) -> Result<Enqueued, EnqueueError>;
```

- Runs inside the caller's transaction, so a job exists only if the caller's change commits.
- `max_attempts` is required because enqueue has no registry to look the kind's policy up in.
- The insert uses `ON CONFLICT DO NOTHING`, which covers both keys. When it inserts nothing, enqueue
  looks the conflicting job up by idempotency key, then by concurrency key. A concurrent enqueue
  with the same key waits for the first transaction and then finds its job.
- A concurrency-key conflict returns `Existing` with the queued or running job's id.
- An idempotency-key conflict with a different payload is `EnqueueError::Conflict`, as
  `RequestConflict` is today.
- If the job holding the key finishes between the insert and the lookup, the insert is tried
  again, at most three times; then enqueue returns `EnqueueError::Contended`.
- A job whose cancellation was requested gives its key up at once, even while its handler winds
  down: enqueue then creates a new job. The cancelling attempt cannot write anything any more,
  because every fence checks `NOT cancellation_requested`.
- Manual retry (`horae-server`'s `retry_in`) refuses a job whose key another queued or running job
  holds, with `ConcurrencyKeyHeld` (409 through `ops`), and maps the index's unique violation to
  the same error when the holder appears after the check. `is_concurrency_key_conflict` tells that
  violation apart.

Known limits:

- `enqueue_in` assumes READ COMMITTED, PostgreSQL's default. Under REPEATABLE READ or
  SERIALIZABLE a concurrent enqueue of the same key fails with a serialization error, and a job
  committed after the caller's snapshot is not found (ending in `Contended`).
- A running job whose lease expired with its attempts used up keeps its key until the next claim
  pass fails it (every 2 s while a worker runs). Enqueue returns that job meanwhile.

## Handlers

```rust
pub trait Handler: Send + Sync + 'static {
    fn kind(&self) -> &'static str;
    fn policy(&self) -> RetryPolicy;
    fn stale_after(&self) -> Duration { Duration::from_secs(60) }
    fn run<'a>(&'a self, ctx: &'a JobContext, payload: serde_json::Value)
        -> BoxFuture<'a, Result<Completed, JobError>>;
}

pub struct RetryPolicy { pub max_attempts: i32, pub first_delay: Duration, pub multiplier: u32, pub max_delay: Duration }
pub struct Completed { pub report: serde_json::Value, pub processed_count: i64 }
pub struct JobError { pub error: anyhow::Error, pub permanent: bool, pub error_kind: Option<&'static str> }
```

`JobContext` carries the job id, organization, attempt number, pool and the lease operations that
exist today: `load_checkpoint`, `save_checkpoint`, `run` (fenced work), and a cancellation receiver.
The heartbeat runs beside the handler in the existing monitor; a handler does nothing to keep it.
It beats every 15 s, or four times per stale limit when that limit is shorter.

A claim writes the job's `stale_after_secs` from the claiming worker's handler, clamped to
15..3600 (the column's CHECK). Claims take no advisory lock: the partial unique index admits one
queued or running job per key, so a claim cannot start a second job for a key, and the claim
token and the heartbeat fence the run.

Attempt rows open at claim and close when the attempt ends. An attempt whose worker died ends as
`interrupted` when its job is reclaimed or failed, and the worker's cleanup also closes, as
`interrupted`, any open attempt of a finished job (an older binary in a rolling upgrade finishes
jobs without touching attempts). Attempts are numbered from the first row the table has for the
job, so a job that was running when the migration ran starts its history at attempt 1.

The delay before attempt `k + 1` is `min(first_delay × multiplier^(k-1), max_delay)`.
`RetryPolicy::max_attempts` is the value enqueuers pass; the worker stops at the job's stored
`max_attempts`.
`RetryPolicy::default()` is 5 attempts, 2 s, ×2, 300 s: the curve the queue ran before policies.

## Worker and scheduler

```rust
pub fn spawn_worker(pool: PgPool, registry: Arc<Registry>) -> Worker;       // Worker::shutdown(grace) as today
pub fn spawn_scheduler(pool: PgPool, zones: Arc<dyn ZoneResolver>) -> Scheduler;
pub trait ZoneResolver: Send + Sync { fn zone(&self, org_id: Uuid, person_id: Option<Uuid>) -> BoxFuture<'_, anyhow::Result<Tz>>; }
pub async fn upsert_schedule(tx: &mut PgConnection, schedule: Schedule) -> anyhow::Result<Uuid>;
pub async fn set_schedule_enabled(tx: &mut PgConnection, org_id: Uuid, name: &str, enabled: bool) -> anyhow::Result<()>;
pub async fn delete_schedule(tx: &mut PgConnection, org_id: Uuid, name: &str) -> anyhow::Result<()>;
pub fn next_due(cadence: &Cadence, zone: Tz, after: DateTime<Utc>) -> DateTime<Utc>;   // pure
pub async fn fire_due(pool: &PgPool, zones: &dyn ZoneResolver, now: DateTime<Utc>) -> anyhow::Result<usize>; // one leader tick
```

The scheduler takes a connection out of the pool and holds
`pg_try_advisory_lock(hashtextextended('horae.job-scheduler', 0))` on it; a follower asks again
every tick, and the leader checks its connection still answers. The lock ends with the session, so a
leader that dies is replaced at the next tick of another instance. `Scheduler::is_leader` reports
the last tick's answer. The lock's connection sets `tcp_keepalives_idle=30`, `_interval=10` and
`_count=3`, so PostgreSQL ends a vanished leader's session within about a minute. Each check on it is
bounded by a tick, so a half-open connection is dropped instead of blocking the loop. A session
advisory lock needs a session: behind PgBouncer, use session pooling, not transaction pooling.

The server runs the sweeps on a worker of their own (`jobs::spawn_sweeps`) beside the import worker
(`jobs::spawn`). A worker runs one job at a time, so a long import would otherwise delay them for as
long as it runs. A worker's retention cleanup runs at most once a minute.

A change to the organization's zone, or to a person's own zone, sets `next_due_at` to null on the
daily and weekly schedules read in it, so they are planned again in the new zone.

A job whose kind has no registered handler stays queued: the claim only selects kinds the worker's
registry holds, so the job is never claimed, its attempts never grow and it is never failed. The
worker logs each such kind once per start. A rolling upgrade therefore cannot fail jobs that a newer
server knows, and a server started with `HORAE_HARVEST_MIRROR=0` leaves mirror jobs waiting rather
than failing them. `horae jobs list` still shows them as queued. A job that was running when its
server stopped and whose kind the next server lacks stays `running` with a stale heartbeat until a
worker that has the handler reclaims it.

Status, list, cancel and retry keep their current signatures and move with the crate.

## Kinds and policies

| Kind | Registered by | Policy (attempts, first delay, multiplier, max) | Concurrency key | Priority |
| --- | --- | --- | --- | --- |
| `harvest_api_import` | server (existing) | `HORAE_JOB_MAX_ATTEMPTS` (default 5), 2 s, ×2, 300 s (today's curve) | none | 0 |
| `harvest_csv_import` | server (existing) | same | none | 0 |
| `long_timer_sweep` | server, PR 3 | 1, n/a | `sweep:long_timer` | 0 |
| `budget_alert_sweep` | server, PR 3 | 1, n/a | `sweep:budget` | 0 |
| `oauth_cleanup` | server, PR 3 | 1, n/a | `sweep:oauth` | -1 |
| `harvest_mirror_pass` | server, PR 4 | 3, 30 s, ×2, 5 min | `harvest:<org>:pass` | 10 |
| `harvest_mirror_reconcile` | server, PR 5 | 3, 1 min, ×2, 15 min | `harvest:<org>:reconcile` | 5 |
| `harvest_parity_check` | server, PR 7 | 2, 10 min, ×1, 10 min | `harvest:<org>:parity` | 0 |
| `harvest_mirror_cutover` | server, PR 9 | 1, n/a | `harvest:<org>:cutover` | 10 |

Sweeps run with one attempt because the next firing is the retry. The three Harvest kinds share the
per-account rate limiter (research R-1), and a pass and a reconciliation never run together because
both take the importer's lock for the organization.

### Payloads

```json
{"version": 1, "payload": {"kind": "harvest_mirror_pass", "value": {"scope": "incremental"}}}
{"version": 1, "payload": {"kind": "harvest_mirror_reconcile", "value": {}}}
{"version": 1, "payload": {"kind": "harvest_parity_check", "value": {"months": "nightly"}}}
```

`scope` is `incremental` or `full`; `months` is `nightly` (current and three previous, plus the
rotation) or `all`.

### Reports

A pass's `report` is the importer's `ImportReport` with one summary per entity (created, updated,
skipped, unsupported, errored) and the watermarks it advanced. A reconciliation's report counts
removed, archived and kept records per entity. A parity job's report is the run's counts and its run
id. All stay under the existing 16 KiB report budget; detail goes to the error chunks (pass,
reconciliation) or to `harvest_parity_findings`.
