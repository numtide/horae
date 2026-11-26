# Data model: Harvest mirror mode, parity checks and the durable jobs crate

Every new table has a UUID v7 `id` and an `org_id` referencing `organizations` (Constitution III).
Column types follow the baseline's conventions. Migration numbers are taken when each PR is written.

## Jobs (owned by `horae-jobs`)

### `horae_jobs` (existing; PR 2 adds columns)

| Column | Type | Notes |
| --- | --- | --- |
| `concurrency_key` | text, null | At most one queued or running job per `(org_id, kind, concurrency_key)` |
| `priority` | smallint, default 0 | Claimed highest first, then `available_at`, then `created_at` |
| `heartbeat_at` | timestamptz, null | Written every 15 s while running |
| `stale_after_secs` | integer, default 60 | Set from the kind's handler at enqueue; `CHECK (stale_after_secs BETWEEN 15 AND 3600)` |

New index: `UNIQUE (org_id, kind, concurrency_key) WHERE concurrency_key IS NOT NULL AND status IN ('queued', 'running')`.

`lease_until` stays and is renewed with each heartbeat to `heartbeat_at + stale_after_secs`, so every
fence that checks it today keeps working. The claim's recovery condition becomes `status = 'running' AND lease_until < now()`, which is now at most a minute after the last heartbeat.

Columns the server owns and the crate does not interpret: `account_generation`,
`original_requester_id`, `error_kind`, `report`, `checkpoint`, `phase`, `processed_count`,
`total_count`. `available_at`, `attempts`, `max_attempts`, `idempotency_key` keep their meaning.

State transitions are unchanged: `queued → running → succeeded | failed | cancelled`, and `running → queued` on a retryable failure. A retry's `available_at` comes from the kind's policy (below) instead
of the fixed `2^attempts` seconds capped at 300.

### `horae_job_attempts` (PR 2)

| Column | Type | Notes |
| --- | --- | --- |
| `id` | uuid | v7 |
| `org_id` | uuid | FK with `job_id` to `horae_jobs (id, org_id)` ON DELETE CASCADE |
| `job_id` | uuid | |
| `attempt` | integer | `CHECK (attempt >= 1)`, `UNIQUE (job_id, attempt)` |
| `worker_id` | text | |
| `started_at` | timestamptz | |
| `finished_at` | timestamptz, null | Null while running or when the worker died |
| `outcome` | text, null | `succeeded`, `failed`, `retrying`, `cancelled`, `interrupted` |
| `error` | text, null | Bounded to 4 KiB |

Deleted with their job by the existing 30-day cleanup.

### `horae_job_schedules` (PR 3)

| Column | Type | Notes |
| --- | --- | --- |
| `id` | uuid | v7 |
| `org_id` | uuid | |
| `name` | text | Unique per organization, for example `harvest_mirror_pass` |
| `kind` | text | The job kind it enqueues |
| `payload` | jsonb | Versioned payload, as for jobs |
| `cadence` | text | `every`, `daily`, `weekly` |
| `every_secs` | integer, null | For `every`: 60 to 86400 |
| `local_time` | time, null | For `daily` and `weekly` |
| `weekday` | smallint, null | For `weekly`: 1 (Monday) to 7 |
| `zone_source` | text | `utc`, `organization`, `person` |
| `person_id` | uuid, null | Required when `zone_source = 'person'`; FK `(org_id, person_id)` to `users` |
| `concurrency_key` | text, null | Passed to each job |
| `enabled` | boolean | |
| `next_due_at` | timestamptz | The next firing, in UTC |
| `last_fired_at` | timestamptz, null | |

Checks tie the nullable columns to `cadence` and `zone_source`. A firing enqueues with idempotency
key `schedule:<id>:<next_due_at as RFC 3339>`, then sets `next_due_at` to the first due time after
now. A daily time that does not exist on a spring-forward day fires at the first instant after the
gap; a repeated autumn time fires once, at its first occurrence.

### `horae_outbox` (existing; moves into the crate unchanged)

## Mirror (owned by `horae-server`)

### `harvest_mirrors` (PR 4)

| Column | Type | Notes |
| --- | --- | --- |
| `id` | uuid | v7 |
| `org_id` | uuid | `UNIQUE` |
| `state` | text | `on`, `paused`, `off`; `CHECK` |
| `pause_reason` | text, null | `administrator`, `connection_refused`, `authority_lost` |
| `interval_secs` | integer | 60 to 3600, default 300 |
| `reconcile_cadence` | text | `hourly` (default) or `daily` |
| `parity_local_time` | time | Default 02:30, organization time zone |
| `authority_id` | uuid | The administrator whose authority passes use; FK to `users` |
| `enabled_at` | timestamptz | |
| `last_pass_job_id` | uuid, null | |
| `last_success_at` | timestamptz, null | |
| `alert_run_id` | uuid, null | FK to `harvest_parity_runs`; set while an unexplained run is open |
| `cut_over_at` | timestamptz, null | Set with `state = 'off'` by the cutover |
| `cut_over_by` | uuid, null | |
| `cut_over_parity_run_id` | uuid, null | |
| `updated_at` | timestamptz | |

States: `off → on` (enable, needs a completed full import and spec 026's delivery 2 migration);
`on → paused` (administrator, or automatic with a reason); `paused → on` (administrator); `on | paused → off` (cutover). A row that reached `off` through a cutover may be turned on again only by a
new full import, which is outside this feature.

Turning the mirror on creates three schedules (`harvest_mirror_pass`, `harvest_mirror_reconcile`,
`harvest_parity_check`), each with concurrency key `harvest:<org_id>:<kind>`. Pausing disables them;
cutover deletes them.

### Watermarks

No new table. `harvest_credentials.synced_watermark` (jsonb, per connection) gains the keys `person`,
`client`, `project`, `task`, `task_assignment`, `user_assignment` beside the existing `time_entry` and
`invoice`. Values are RFC 3339 strings, as today.

### Provenance

`harvest_import_map` is unchanged in shape. `harvest_entity_type` gains `task_assignment` and
`user_assignment`. A record is mirrored when it has a row here and the organization's mirror is `on`
or `paused`.

### `harvest_write_through` (PR 10)

| Column | Type | Notes |
| --- | --- | --- |
| `id` | uuid | v7 |
| `org_id` | uuid | |
| `person_id` | uuid | `UNIQUE (org_id, person_id)`; FK to `users` |
| `harvest_user_id` | bigint | From the person's provenance; required |
| `enabled_by` | uuid | |
| `enabled_at` | timestamptz | |

## Parity (owned by `horae-server`)

### `harvest_parity_runs` (PR 7)

| Column | Type | Notes |
| --- | --- | --- |
| `id` | uuid | v7 |
| `org_id` | uuid | |
| `job_id` | uuid | The parity job |
| `started_at`, `finished_at` | timestamptz | |
| `watermarks` | jsonb | The `synced_watermark` compared against (FR-027) |
| `months` | daterange[] | The months layer 2 covered |
| `counts` | jsonb | Per layer and class: `{"records": {"equal": n, "missing_in_horae": n, …}, …}` |
| `unexplained` | integer | Confirmed findings without a rule |
| `pending` | integer | First sightings |
| `status` | text | `succeeded`, `failed`, `partial` (a layer could not finish) |

Kept for at least a year.

### `harvest_parity_findings` (PR 7)

| Column | Type | Notes |
| --- | --- | --- |
| `id` | uuid | v7 |
| `org_id` | uuid | |
| `run_id` | uuid | FK `(run_id, org_id)` ON DELETE CASCADE |
| `layer` | text | `records`, `totals`, `invoices` |
| `entity` | text | `person`, `client`, `project`, `task`, `task_assignment`, `user_assignment`, `time_entry`, `invoice`, or a report (`report_client`, `report_project`, `report_task`, `report_person`, `uninvoiced`) |
| `harvest_id` | bigint, null | Records and invoices |
| `horae_id` | uuid, null | |
| `aggregate_key` | jsonb, null | Totals: `{"month": "2026-10", "client": 123, "currency": "EUR"}` |
| `class` | text | `missing_in_horae`, `extra_in_horae`, `different`, `known` |
| `rule` | text, null | For `known`: `rounding`, `unsupported`, `deletion_pending`, `in_flight` |
| `fields` | jsonb | `[{"field": "minutes", "harvest": "90", "horae": "75"}]`, values as exact text |
| `fingerprint` | bytea | SHA-256 of layer, entity, ids or key, class and fields |
| `confirmed` | boolean | The fingerprint was in the previous completed run |
| `first_seen_run_id` | uuid | |

Equal pairs are counted only. Findings are deleted 90 days after their run.

Validation: a `known` finding has a `rule`; other classes have none. `different` has at least one
field.
