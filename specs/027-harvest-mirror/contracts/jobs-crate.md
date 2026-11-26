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
    pub max_attempts: Option<i32>,        // default from the kind's policy
}

pub enum Enqueued { Created(Uuid), Existing(Uuid) }

pub async fn enqueue_in(tx: &mut PgConnection, request: Enqueue<'_>) -> Result<Enqueued, EnqueueError>;
```

- Runs inside the caller's transaction, so a job exists only if the caller's change commits.
- A concurrency-key conflict returns `Existing` with the queued or running job's id.
- An idempotency-key conflict with a different payload is `EnqueueError::Conflict`, as
  `RequestConflict` is today.

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

The delay before attempt `k + 1` is `min(first_delay × multiplier^(k-1), max_delay)`.

## Worker and scheduler

```rust
pub fn spawn_worker(pool: PgPool, registry: Arc<Registry>) -> Worker;       // Worker::shutdown(grace) as today
pub fn spawn_scheduler(pool: PgPool, zones: Arc<dyn ZoneResolver>) -> Scheduler;
pub trait ZoneResolver: Send + Sync { fn zone(&self, org_id: Uuid, person_id: Option<Uuid>) -> BoxFuture<'_, anyhow::Result<Tz>>; }
pub async fn upsert_schedule(tx: &mut PgConnection, schedule: Schedule) -> anyhow::Result<Uuid>;
pub async fn set_schedule_enabled(tx: &mut PgConnection, org_id: Uuid, name: &str, enabled: bool) -> anyhow::Result<()>;
pub async fn delete_schedule(tx: &mut PgConnection, org_id: Uuid, name: &str) -> anyhow::Result<()>;
pub fn next_due(cadence: &Cadence, zone: Tz, after: DateTime<Utc>) -> DateTime<Utc>;   // pure
```

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
| `harvest_api_import` | server (existing) | `HORAE_JOB_MAX_ATTEMPTS` (default 5), 1 s, ×2, 300 s (today's curve) | none | 0 |
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
