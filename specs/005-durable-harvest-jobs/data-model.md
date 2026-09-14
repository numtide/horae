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
- `phase`, `processed_count`, `total_count`: progress.
- `checkpoint`: versioned importer cursor/checkpoint.
- `report`, `last_error`: terminal report and latest failure details.
- `created_at`, `started_at`, `finished_at`, `updated_at`: lifecycle timestamps.

New jobs snapshot `HORAE_JOB_MAX_ATTEMPTS` (default 5, allowed 1–100) into
`max_attempts`. Duplicate enqueues and manual retries preserve that limit,
even if the server's current policy has changed. Existing rows retain their
stored policy.

## CSV Upload Blob

Stores an administrator-uploaded CSV owned by the same organization and job, with a bounded size. Successful uploads are eligible for deletion after one day. Failed and cancelled uploads remain available for retry until the job's thirty-day retention ends; deleting the job cascades to its upload. Retry refuses CSV jobs with a missing upload.

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
