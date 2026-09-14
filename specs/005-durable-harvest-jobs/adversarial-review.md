# Durable jobs adversarial review

Reviewed baseline: `86e93fcf1cbe2ed6650d4ebf11272d99870c6e0e`.

The baseline CI passed, but the following requirements are not established by
those checks. The PR must remain open until these cases are addressed.

## Findings

| Severity | Trigger and consequence | Evidence | Required regression |
|---|---|---|---|
| High | Cancel a running import: the row becomes cancelled while the importer continues applying work. Retrying can race the old execution. | `jobs::cancel` clears ownership without signalling or checking inside the importer. | Cancel during execution; prove no further batch begins after acknowledgement and retry cannot overlap it. |
| High | Lose a lease during an import: the old handler keeps running while another instance can reclaim the job. | `execute` logs heartbeat errors, ignores zero affected rows, and does not stop the handler. | Expire and reclaim a live lease; prove the stale execution cannot commit or acknowledge subsequent work. |
| High | Missing Harvest configuration or upload leaves the job running instead of recording a retryable error. | `execute` uses `?` before reaching its result handler. Payload decoding in `claim` has a similar failure path. | Exercise missing configuration, missing upload and invalid stored payload through claim and execution. |
| High | Shutdown drops the Tokio runtime before the worker has drained; deployment SIGTERM is not handled. | `spawn` discards its join handle; `main` only sends a watch notification after HTTP exits. | Await a blocked active job, enforce a drain deadline, stop claiming, and handle SIGTERM. |
| High | Crash after processing pages: retry repeats the whole import; live progress stays at zero. | `checkpoint` is never read or written. `processed_count` only changes on success; importers use an import-wide transaction. | Interrupt between durable batches, resume without repeating completed work, and observe progress before completion. |
| Medium | Repeated crashes or invalid payloads can exceed the attempt limit; terminal errors lack retention timestamps. | `claim` does not check `max_attempts`; the error update does not set `finished_at`. | Exhaust crash and ordinary error retries; assert terminal status, timestamp and cleanup behavior. |
| Medium | Reopen the importer: previously submitted jobs and reports are not displayed. A polling error leaves the screen running indefinitely. | `active_job` starts empty; the history endpoint has no UI caller; `use_effect` ignores polling errors. | Reopen, select history, resume polling, retry, and recover from a failed status request. |
| Medium | A stale outbox consumer can acknowledge or reschedule a newer claim. | Delivery updates filter only by ID and undelivered state, with no claim token. | Reclaim an event, reject the old consumer's acknowledgement and retry, and verify transaction rollback. |
| Medium | Future payload changes cannot be distinguished from old persisted jobs. | `JobPayload` has a kind tag but no payload version. | Decode supported versions and record a bounded terminal error for unsupported versions. |
| Medium | Retrying a CSV after upload retention expires cannot succeed. | Cleanup deletes uploads after one day while failed/cancelled jobs remain retryable for thirty days. | Retry at the retention boundary, including concurrent cleanup. |

## Coverage corrections

- The baseline synthetic test only serializes and deserializes an enum. It does
  not execute a job through the worker boundary.
- The baseline cancellation test only changes a queued row; despite its name,
  it does not test a foreign organization or running cancellation.
- The baseline outbox test verifies duplicate database acknowledgements, not
  idempotent external delivery, stale claims, or transactional rollback.
- Future webhook/email/plugin consumers are explicitly deferred by FR-018.
  Their absence alone is not a first-release defect; the shared outbox semantics
  still require the tests above.

## Follow-up verification

The follow-up retains the worker join handle, drains it before runtime exit,
enforces a deadline, and wires SIGTERM into the shutdown path. It also routes
configuration and upload lookup failures through the persisted error/retry path.
The subsequent recovery fixes below also address invalid payload decoding and
terminal retry bookkeeping.

The first follow-up passed 11 `jobs::tests` with a temporary PostgreSQL instance, including:

- Waiting for active work and joining it after a drain deadline.
- Leaving queued jobs unclaimed when shutdown is requested.
- A full synthetic claim/execute/completion, with its persisted report checked.
- Missing configuration and missing upload returning to queued state with errors.
- Rejecting foreign-organization status, list, cancel and retry operations.

The configuration regression failed before the fix with `Harvest is not configured` escaping from `execute`. SIGTERM is wired in the server entry point;
these tests exercise worker shutdown directly, not operating-system signalling
against a deployed server. Passing them does not close the remaining findings.

## Recovery and outbox follow-up

The expanded suite passes 18 tests against PostgreSQL. It verifies that expired
final attempts become failed without being claimed again, terminal failures
receive retention timestamps, manual retry resets the attempt budget, and
malformed or unsupported stored payloads reach a recorded terminal error.

Payloads now have a version-1 envelope with legacy read compatibility. CSV
uploads remain available throughout the retryable retention window; deleting an
expired terminal job also deletes its upload.

Outbox claims now carry a UUIDv7 token. Acknowledgements require a current,
unexpired token and matching organization. Tests reject stale, duplicate and
foreign acknowledgements, retain failure details with backoff, and verify that
rolling back the enqueue transaction publishes no event. This does not provide
exactly-once external delivery.

At that stage, open findings included cooperative running cancellation, fencing of import writes after
lease loss, durable checkpoints and live progress,
and history restoration/error recovery in the UI. Baseline CI success must not
be used to close these findings.

## Configurable execution policy

API and CSV enqueue now persist the configured attempt limit rather than relying
on the database default. `HORAE_JOB_MAX_ATTEMPTS` accepts 1–100 attempts, including
the initial execution, and defaults to 5. Validation runs both at startup and at
the enqueue boundary. Idempotent requests preserve the original job's limit and
CSV body; manual retry resets consumed attempts without replacing the policy.

The PostgreSQL jobs suite passes 21 tests. New regressions exercise API failures
through a non-default attempt budget, a replacement server with different
configuration, duplicate CSV enqueue, and rejection before any job or upload is
stored. The configuration regression was observed failing before the fix: the
requested environment value was ignored. This follow-up closes T004, not the
remaining cancellation, lease-fencing, checkpoint, or UI findings.

All 11 configuration tests pass, including the default, supported boundaries,
and rejection of empty, out-of-range, overflowing, and non-numeric values. Server
clippy passes with all targets and warnings denied; SQLx metadata is regenerated.

## Execution fencing and cooperative cancellation

Every claim now receives a UUIDv7 token separate from worker identity. Running
cancellation is a persisted request, not an immediate terminal acknowledgement.
The worker signals its SQL consumer, joins the blocking source producer, and
flushes rollback before recording cancellation. Retry is unavailable while that
cleanup is pending. Recovery honours a cancellation requested before a crash.

Both API and CSV committing imports now write their terminal report in the same
transaction as domain changes. The conditional update checks organization,
claim token, running state, cancellation, and the current lease deadline. It
locks the job through commit, preventing cancellation or reclaim from racing a
separate ownership check. Lease checks use `clock_timestamp()`, not the import
transaction's potentially old `now()`. Heartbeat monitoring is an owned future,
so aborting the worker cannot leave a detached renewal task alive.

Verification passes 26 jobs tests and 115 importer tests against PostgreSQL;
three existing 100,000-record benchmarks remain explicitly ignored. Regressions
cover same-name worker replacement, expired renewal, an expired lease inside an
older transaction, cross-organization adapter calls, and cancellation recovery.
Real CSV and API pipelines reject stale commits with heartbeat deliberately
disabled, including API watermark writes. Running cancellation tests prove the
producer stops before acknowledgement and preserve previously committed data;
CSV retry then succeeds without duplicating those records. The initial
cancellation-state regression was observed failing before this fix. Server
clippy passes with all targets and warnings denied.

This closes the worker lifecycle/fencing gaps in T006. Checkpoints and live
progress are still missing: the current import uses one transaction, so a crash
still restarts its uncommitted work. T007, T009, T012 and T016 remain open for
checkpoint/resume behavior, the final authorization audit, and history/retry/error
UI. The cancellation and fencing tests must also cover future batch boundaries
before the feature is ready to merge.

## Import history and status recovery

The importer now restores queued/running work on reopening, displays stored
phase/count/total/last-error values, and lets administrators select retained
reports. History uses 20-row cursor pages ordered by creation time and UUID.
Foreign, missing, and expired cursor IDs cannot expose another organization's
history. Manual retry clears the previous cancellation phase.

Status lookup failures and missing jobs stop the loading indicator and offer
resuming monitoring without enqueuing another import. Results are tied to a
request generation, so a previous result cannot overwrite a newer selection.
Cancel and retry failures are visible; accepting a cancellation does not imply
worker cleanup has finished. Pending mutations reject duplicate submissions.

Historical previews are read-only. Newly submitted previews keep their original
source for confirmation, including when the CSV picker changes while the
preview runs. The report labels the original CSV filename.

Ten production-component tests exercise these interactions through controlled
server-function responses, including pagination, request replacement, and CSV
source preservation. The initial reopening regression failed before the fix.
Five existing admin-shell tests also pass. These are VirtualDom interaction
tests, not a live browser/database end-to-end run.

The PostgreSQL suites pass 27 jobs tests and 115 importer tests; three explicit
scale benchmarks remain ignored. New database coverage checks tied-timestamp
pagination, timestamp precedence, foreign/missing cursors, and retry phase reset.
Server clippy passes with all targets and warnings denied. The WebAssembly target
checks successfully, with existing warnings for InvoiceLine, OrgBranding, and
PluginWidget. SQLx metadata is regenerated; only the two obsolete changed-query
entries are removed.

This completes the UI implementation in T012. It does not implement live
checkpoint production: T007 and its crash/resume and batch cancellation coverage
in T016 remain open, as does the final authorization audit in T009. The PR remains
a draft until those requirements and the complete acceptance walkthrough hold.

## CSV commit checkpoints

Durable CSV commits now save version-1 checkpoints every 500 source records.
Cursor, accumulated report, currency fallback, and resolution/occurrence cache
commit with the batch's domain writes. The job update is fenced by organization,
claim token, current lease deadline, running state, and cancellation request.
Recovery skips the original upload's byte prefix and resumes parsing at the next
record. Successful completion clears the checkpoint and records the final total.

The initial production-pipeline regression failed because no commit occurred
before EOF. Follow-up tests now interrupt an import after its first batch and
verify that crash recovery and manual retry preserve counts, errors, and
legitimate repeated CSV entries. A deliberately unmonitored obsolete worker
cannot commit the next batch after its lease is reclaimed. Cancellation waits
for producer cleanup and retains completed batches. A dry-run exceeding the
batch size leaves no domain rows or durable checkpoint.

Parser regressions cover multiline Unicode/CRLF records, original error
locations after resume, EOF checkpoints, and offsets beyond the upload. Cache
round-trip coverage includes distinct email/full-name namespaces, failed parents,
project/task links, and occurrence counters.

Verification passes 122 importer tests, 27 jobs tests, 86 domain tests, 10 importer
UI tests, and five admin-shell tests. Three explicit scale benchmarks remain
ignored. Server clippy passes with all targets and warnings denied; WebAssembly
checks with the same three existing warnings. SQLx metadata is regenerated and
the obsolete completion-query entry is replaced.

T007 and T016 remain open: API and dry-run checkpoint integration is still
missing, and full-cache snapshot size/large-import throughput needs validation.
The final authorization audit in T009 and complete acceptance walkthrough also
remain required. This follow-up does not establish the whole feature's readiness
to merge.

## Resumable HTTP pagination prerequisite

The HTTP adapter now exposes a versioned, serializable cursor for its next
unconsumed page and constant-space cycle detector. Replacement clients resume
from the provider's exact cursor URL, using current credentials, and completed
cursors issue no further requests. Unsupported versions, invalid cycle state,
oversized URLs and destinations outside the configured collection endpoint are
rejected before any authenticated request. A consumer error leaves its cursor
unchanged.

Five new regressions cover serialized recovery, EOF, consumer failure, invalid
stored state, and cycles spanning repeated restarts. The missing-next-field
regression failed before the deserializer was corrected: an absent optional
field was silently treated as EOF. Only an explicit null now marks completion.
All 127 importer tests pass against PostgreSQL, including the existing live
API pipeline tests; three scale benchmarks remain ignored. Server clippy passes
with all targets and warnings denied. No queries, migrations or dependencies
changed in this follow-up.

This is an HTTP adapter prerequisite, not completed durable API integration.
The importer must still persist this cursor with catalog, account identity,
report/cache and watermark state at its fenced batch boundaries. T007 and T016
remain open, along with dry-run recovery, scale validation, T009 and the complete
acceptance walkthrough.

## Durable API commit checkpoints

Leased API commits now persist each catalog page, then apply parent entities in
500-record batches and time entries one provider page at a time. The checkpoint
retains the original account, sync scope, filter, currency and capture time,
alongside the catalog, parent offset, HTTP cursor, report and resolution cache.
Every checkpoint commits under the same lease/cancellation fence as its domain
writes. Source buffering alone never advances the persisted cursor.

Retries preserve earlier row errors, missing timestamps and the maximum source
timestamp. Watermark advancement remains gated by the accumulated result and
capped by the original capture time; it commits with the terminal report. If
that final transaction fails, an EOF checkpoint permits retry without fetching
or applying any completed page. Inline commits and previews retain their
existing whole-import transaction behavior.

Eight production-HTTP/PostgreSQL regressions cover partial catalog recovery,
entry-page recovery, errors and missing timestamps across retries, replacement
of a live claim, cancellation followed by manual retry, finalization retry, and
failure after a 500-parent batch commits. The first two tests failed before the
implementation because completed API pages had no durable checkpoint. A stale
worker is tested without heartbeat monitoring, so rejection relies on the
database commit fence. The parent-batch test also verifies original counts and
catalog precision after checkpoint deserialization.

The importer suite passes 135 tests, with three explicit scale benchmarks still
ignored; all 27 jobs tests pass. Server clippy passes with warnings denied.
SQLx metadata is regenerated, adding only three failure-injection DDL queries
used by tests. No production queries, migrations or dependencies change.

T007 and T016 remain open for dry-run recovery and complete checkpoint
validation. Snapshot size and large-import throughput, the final authorization
audit in T009, and the complete acceptance walkthrough are still required.
The feature is not ready to merge.
