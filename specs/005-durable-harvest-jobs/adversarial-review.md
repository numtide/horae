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

Still open: cooperative running cancellation, fencing of import writes after
lease loss, durable checkpoints and live progress, configurable enqueue policy,
and history restoration/error recovery in the UI. Baseline CI success must not
be used to close these findings.
