# Data Model: Durable Harvest Import Jobs

## Import Job

Represents one administrator-requested API or CSV import.

- `id`: UUIDv7 primary key.
- `org_id`: organization owner; foreign key and authorization boundary.
- `kind`: API or CSV import kind.
- `payload`: versioned JSON without credentials or raw request streams.
- `status`: queued, running, succeeded, failed, or cancelled.
- `attempts`, `max_attempts`: retry accounting.
- `available_at`: earliest retry/start time.
- `lease_until`, `worker_id`: active claim lease.
- `phase`, `processed_count`, `total_count`: progress.
- `checkpoint`: versioned importer cursor/checkpoint.
- `report`, `last_error`: terminal report and latest failure details.
- `created_at`, `started_at`, `finished_at`, `updated_at`: lifecycle timestamps.

## CSV Upload Blob

Stores an administrator-uploaded CSV until its asynchronous job finishes. It is owned by the same organization and job, has a bounded size, and is deleted by terminal-job retention cleanup.

## Invariants

- A job can have at most one active lease.
- A terminal job cannot be claimed again.
- Job and upload organization IDs must match.
- Payloads and reports must be bounded and schema-versioned.
