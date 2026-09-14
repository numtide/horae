# Data Model: Harvest Jobs CLI

## Session file

`{ "server_origin": "https://horae.example", "session_id": "<existing id cookie value>" }`

Regular private file, maximum 16 KiB, no symlink or group/other permissions on Unix. Never echo its contents. Origin is the sole destination, without URL credentials, query, fragment or path prefix. Send only the `id` cookie; never follow redirects. File provisioning/renewal follows existing web login, not server secret access.

## Request identity

UUIDv7, generated or explicit. Validate age ≤24 hours and future skew ≤five minutes. Namespace is organization plus source kind. Existing payload carries mode/scope; CSV adds a server-computed SHA-256 digest not exposed in status. Atomic enqueue returns the original job only for identical input, preserving policy/upload/outcomes. Reject conflicting or expired identities. Existing uniqueness and lifecycle remain; no migration planned.

## Job and report

Reuse `models::JobStatus` and core `ImportReport`. Queued/running are active; succeeded/failed/cancelled terminal. Cancellation request and acknowledgement are distinct. Reports follow feature-005 retention and captured error boundaries.

## Result envelope

Fields: `version: 1`, `operation`, `outcome`, optional `job_id`, `request_id`, `data`, and safe `error`. Errors contain a stable category and optional HTTP status, never untrusted remote bodies or cookies. File results identify the destination and byte count only after publication.

Exit codes: 0 successful command (including completed partial imports), 1 command/transport failure, 2 invalid arguments/configuration, 3 failed terminal job, 4 cancelled terminal job, 5 wait deadline, 6 indeterminate submission, 130 interrupted observation. Reading a failed job succeeds (0); waiting on it returns 3.
