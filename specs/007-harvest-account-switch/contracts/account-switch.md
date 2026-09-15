# Contract: Harvest Account Switching

## Administrator operations

- `harvest_connection_status`: stable POST `/api/import/harvest/connection`; returns current bound identity, connection flags, generation/revision and eligibility.
- `harvest_change_account(expected_account, expected_generation, expected_revision)`: typed administrator server function; rejects missing binding, changed context, imported API provenance or queued/running Harvest work. Success removes only credentials/binding and advances both counters; reports remain retained. The UI then starts normal OAuth.
- `harvest_connect_start`: captures fresh context with nonce. Callback rechecks the context when storing credentials. A stale callback cannot replace a connection.
- `harvest_disconnect`: retains source binding/account generation, advances connection revision and removes credentials/watermark only.

## Work compatibility

`start_harvest_api_import(mode, sync, generation: Option<i64>)` keeps `/api/import/harvest/start`. Missing generation is interpreted as zero, never as current generation. Updated UI/CLI capture status before submission; unchanged ordinary submissions in generation zero retain backward compatibility. Rebuild server/web together; upgrade the CLI before using a switched connection.

Stop/drain older servers and workers before enabling this version; mixed-version workers and OAuth callbacks do not implement these fences. This is a coordinated upgrade, not a rolling compatibility guarantee.

Identity replay remains bound to the captured generation and original payload. Old retained API reports remain readable; their jobs cannot be retried under another generation. The existing 30-day retention exceeds the accepted identity's 24-hour lifetime plus future skew, so expired history cannot turn a valid old identity into new work.

The CLI does not automatically retry uncertain submissions. A user resubmitting with the same request ID reads current status again, but the server rejects any mismatch with the generation of the previously accepted identity. An unaccepted request has no persisted association; legacy or explicitly stale generation still fails closed.

## UX

Change account has explicit confirmation and cancel actions. Explain that the existing connection is removed, business data/reports are retained, and old attempts cannot run against another account. Display blocking reasons; never suggest that Disconnect removes imported data or frees the binding. Failure to authorize afterward leaves a normal disconnected state with Connect available.

Use the existing labelled native Modal, busy state, focus restoration, Escape cancellation and visible errors. Hide/disable stale actions while submitting; server checks remain authoritative.
