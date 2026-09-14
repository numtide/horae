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
- `report`, `last_error`: last confirmed outcomes and latest attempt failure details. Only `status = succeeded` means the report covers the completed source.
- `created_at`, `started_at`, `finished_at`, `updated_at`: lifecycle timestamps.

New jobs snapshot `HORAE_JOB_MAX_ATTEMPTS` (default 5, allowed 1–100) into
`max_attempts`. Duplicate enqueues and manual retries preserve that limit,
even if the server's current policy has changed. Existing rows retain their
stored policy.

Harvest jobs initialize an empty source/mode-specific report when enqueued, so
configuration failures or invalid uploads before the first checkpoint still have
an inspectable zero-confirmed-work result. Duplicate enqueue never replaces an
existing report. Each checkpoint updates the public report in the same fenced
transaction as its cursor and progress. Failed or cancelled batches cannot publish
unconfirmed outcomes; previous confirmed outcomes survive bounded retry exhaustion,
lease expiry, cancellation and manual retry. Final success replaces the partial
report with the complete one and clears the checkpoint.

Status and history also read the report embedded in older checkpoints, without
returning the private source cursor or simulation cache. This supports jobs written
before separate partial-report persistence. Report data follows the job's existing
thirty-day retention policy; there is no separate report expiry.

### Import report schemas

Reports include `version: 1` alongside their source, mode, summary and per-record
errors. The shared report deserializer rejects unsupported or malformed explicit
versions. Legacy reports without the field retain their existing version-1
meaning, including all counts and error details. This applies to final reports,
partial reports and reports embedded in importer checkpoints.

Version 2 adds `error_archive: { count, chunks }` when error details have been
archived. `count` is the number of archived errors; `chunks` is the exclusive
end of their ordered byte stream. Summary error totals must equal archived
errors plus the inline `row_errors` tail. A version-1 report cannot claim an
archive. Unknown versions, invalid archive metadata and inconsistent error
counts are rejected on read.

New checkpoint/final reports serialize to at most 16 KiB. Workers archive above
8 KiB of compact JSON, reserving room for PostgreSQL's JSONB rendering whitespace
within the same 16 KiB database limit. The worker appends pending details within
the checkpoint/completion transaction, then clears that inline list. Failed
transactions publish neither fragments nor an advanced archive boundary.
Preview archives are written after rollback of their domain simulation.

API and CSV checkpoints containing archived errors use outer checkpoint version
2 as well. Older workers that predate report-version validation already reject
that checkpoint version, so they cannot resume using an incomplete inline error
list. Readers still accept version-1 checkpoints without archived errors, but
reject archives mislabelled as checkpoint version 1.

The startup upgrade converts existing oversized reports before exposing the
server or starting its worker. It locks one job per transaction and transfers at
most sixteen 64 KiB error fragments per read, never a complete legacy report or
checkpoint. The checkpoint's cursor/cache and independent summary totals remain
intact. Conversion invalidates the old claim and expires its running lease;
normal recovery retains the attempt budget and cancellation request. Failed
conversions roll back their fragments and leave the original report intact.

Migration 0028 checks both report columns on new writes immediately, including
writes from older workers. Startup validates those constraints after converting
old rows. Malformed/unsupported reports fail startup rather than discard data.
PostgreSQL still decodes the existing JSONB internally during conversion; the
bounded transfers describe application memory, not PostgreSQL's legacy decoding
cost. The one-connection upgrade/rollback regression passes; broader legacy
state coverage and archive stress/failure-path coverage remain open in T021.

### Report error chunks

`horae_job_report_error_chunks` stores the complete error JSON-lines byte stream.
Each row has a UUIDv7 primary key, `job_id`, an `org_id` foreign key, a unique
zero-based per-job `sequence`, `schema_version = 1`, and 1–65,536 `body` bytes.
A composite foreign key enforces job/organization ownership. Job deletion
cascades to every fragment; errors share the job's retention window.

Fragments may split a JSON record or UTF-8 character. Consumers concatenate the
ordered bytes before decoding records; writers never truncate long reasons.
Reads fetch at most 16 fragments (1 MiB) within the captured report's archive
boundary and reject missing sequences. A download appends that report's inline
tail once, rather than observing the job's changing live archive end.

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

### CSV preview checkpoint, version 1

CSV previews use the same cursor, report and occurrence cache, with an additional
`preview` snapshot of cached clients, projects, tasks and project/task links. Each
500-record batch captures that simulation state, rolls back its domain transaction,
then persists the checkpoint in a new lease-fenced transaction. The next batch
restores parent identities and importer-relevant attributes only inside another
rollback-only transaction. Existing domain rows are never updated by restoration.

CSV time entries carry no source IDs or provenance: the occurrence counter already
identifies which existing duplicate the next row should match. Simulated time
entries need not be stored or replayed. Recovery skips the checkpointed source
prefix and resumes with original counts and errors. At EOF the final domain
transaction rolls back before the terminal job report commits under its lease.
Commit checkpoints remain compatible and never contain preview state.

CSV snapshots currently scale with the cached parent/link set and are restored at
each batch boundary; their size and large-import throughput still need measurement
alongside the existing full resolution-cache snapshots.

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
Inline imports keep their existing import-wide transactions.

### API preview checkpoint, version 1

Durable API previews use the same catalog, parent-batch and entry-page cursors,
with an additional `preview` field. Parent snapshots reuse the CSV representation.
An entry map retains successful Harvest-ID to Horae-ID associations across pages,
including adoption of real entries from an earlier CSV import. Restoration keeps
all associations to real entries reserved against other source IDs, plus virtual
associations whose source IDs appear on the upcoming page. Other virtual entries
have no row left to adopt after rollback, so their mappings remain only in the
checkpoint until needed by a repeated ID. This avoids replaying every old virtual
mapping on every fresh page without recreating simulated time-entry rows. Failed
row savepoints do not publish new simulated associations.

Each batch applies inside a nested transaction. The parent snapshot and successful
entry associations are captured before rolling that transaction back; only the
checkpoint and progress commit in the outer lease-fenced transaction. Recovery
restores simulated parents and associations inside the next rollback-only batch,
not as permanent domain data. Finalization commits the report and clears the
checkpoint without changing the sync watermark. An EOF checkpoint also allows a
failed finalization to retry without downloading or applying any source pages.

Loading rejects a mode/preview-state mismatch. Existing version-1 committing
checkpoints remain readable without the optional preview field. Preview state
grows with cached parents and distinct entry IDs; snapshot size and repeated
restoration cost remain subject to large-import validation.

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
