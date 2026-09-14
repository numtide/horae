# Data Model: Durable Harvest Import Jobs

## Import Job

Represents one administrator-requested API or CSV import.

- `id`: UUIDv7 primary key.
- `org_id`: organization owner; foreign key and authorization boundary.
- `kind`: API or CSV import kind.
- `payload`: JSON envelope `{ "version": 1, "payload": { ... } }` without credentials or raw request streams. Reads also accept the original unversioned tagged enum; unknown versions enter the bounded failure path.
- `status`: queued, running, succeeded, failed, or cancelled.
- `attempts`, `max_attempts`: retry accounting. Both execution errors and expired claims consume the persisted attempt budget; manual retry starts a new budget on the same job.
- `available_at`: earliest retry/start time.
- `lease_until`, `worker_id`: active claim lease.
- `claim_token`: UUIDv7 assigned on every claim; fences commits and acknowledgements independently of worker identity or reset attempt counts.
- `cancellation_requested`: durable request flag. Running jobs remain running until the worker stops or their lease expires; retry is unavailable before terminal acknowledgement.
- `phase`, `processed_count`, `total_count`: progress.
- `checkpoint`: versioned importer cursor/checkpoint.
- `report`, `last_error`: terminal report and latest failure details.
- `created_at`, `started_at`, `finished_at`, `updated_at`: lifecycle timestamps.

New jobs snapshot `HORAE_JOB_MAX_ATTEMPTS` (default 5, allowed 1–100) into
`max_attempts`. Duplicate enqueues and manual retries preserve that limit,
even if the server's current policy has changed. Existing rows retain their
stored policy.

### CSV commit checkpoint, version 1

Durable CSV commits save a checkpoint every 500 source records, including
record-level errors. The checkpoint contains the next byte/record position,
validated headers, the captured default currency, the accumulated report, and
the run's resolution cache. The cache preserves parent and user identities,
project/task links, failed parents, and occurrence counters for identical CSV
entries. Email and full-name identities remain distinct when serialized.

The checkpoint and processed count are written inside the same transaction as
the batch's domain changes, with a current-lease/cancellation fence. Recovery
reads that checkpoint under the new claim and discards the stored CSV's byte
prefix without parsing or applying completed records again. A failed or cancelled
attempt retains the checkpoint for retry. Successful completion retains the
report and final total count but clears the now-unneeded checkpoint cache.

Dry-runs do not yet use resumable checkpoints. They retain
their rollback-only domain transaction; source and simulation checkpoint support
remains required before FR-007 is complete. Snapshot size and large-import
throughput also remain part of the final checkpoint validation.

### API pagination cursor, version 1

The HTTP adapter exposes a serializable cursor containing the next unconsumed
URL and constant-space pagination cycle state (`anchor`, `distance`, `window`).
An explicit null next URL means the collection is complete; an absent field is
invalid. The cursor contains no supplied account credentials. Resuming validates
both stored URLs against the configured collection endpoint before sending any
authenticated request, and rejects unsupported versions or invalid cycle state.

The consumer receives the proposed next cursor with each page. The in-memory
cursor advances only after the consumer accepts that page. A durable consumer
must save this cursor atomically with its applied batch, not when HTTP finishes
downloading or buffering it.

### API commit checkpoint, version 1

Durable API commits checkpoint each downloaded catalog page before moving on
to parent application. The checkpoint retains the catalog, collection and HTTP
cursor, original account ID, sync scope, incremental filter, currency fallback
and capture start time. Resume rejects unsupported versions, another account or
sync scope, and inconsistent parent offsets. Credentials are loaded afresh and
are not part of the checkpoint.

Once the catalog is complete, clients/projects/tasks apply in dependency order
in batches of up to 500 entities. Each batch commits its parent offset,
resolution cache, report and progress atomically with domain writes. Subsequent
time-entry pages each commit with their next cursor and accumulated report/cache.
Every checkpoint update checks the current organization, claim token, lease
deadline and cancellation state, locking the job until the batch commits.

The maximum observed source timestamp and any missing-timestamp flag survive
retries. The watermark advances only after all pages finish without row errors
or missing timestamps, capped by the original capture start time. Watermark and
terminal report commit together. A finalization failure retains an EOF cursor,
so retry can finish without downloading or applying completed pages again.
Inline imports and dry-runs keep their existing import-wide transactions.

## CSV Upload Blob

Stores an administrator-uploaded CSV owned by the same organization and job, with a bounded size. Successful uploads are eligible for deletion after one day. Failed and cancelled uploads remain available for retry until the job's thirty-day retention ends; deleting the job cascades to its upload. Retry refuses CSV jobs with a missing upload.

The composite foreign key `(job_id, org_id)` references the job's `(id, org_id)`;
independent foreign keys alone do not enforce this ownership invariant. Migration
0026 rejects existing mismatches for operator review rather than silently
reassigning or deleting uploads.

## Invariants

- A job can have at most one active lease.
- A terminal job cannot be claimed again.
- Job and upload organization IDs must match.
- Payloads and reports must be bounded and schema-versioned.

## Outbox Delivery Claim

`claim_token` is a UUIDv7 created for each delivery attempt. Claiming also
increments `attempts` and moves `available_at` to the lease deadline. Delivery
and failure acknowledgements require the event ID, organization ID, current
claim token, and an unexpired lease. Both acknowledgements clear the token;
failure schedules bounded backoff and preserves `last_error`.

This fences stale database acknowledgements, not external side effects. Future
consumers must pass the stable event ID as an idempotency key to destinations
that support it; a crash after an external effect can still cause redelivery.
