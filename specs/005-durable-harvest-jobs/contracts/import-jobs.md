# Import Jobs Contract

All operations are authenticated server functions and restricted to organization administrators.

| Operation | Input | Result |
|---|---|---|
| `start_harvest_api_import` | mode, sync scope | job ID + initial status |
| `start_harvest_csv_import` | mode, upload | job ID + initial status |
| `get_import_job` | job ID | status, progress, report/error |
| `list_import_jobs` | optional limit/cursor | recent jobs for current organization |
| `cancel_import_job` | job ID | updated status |
| `retry_import_job` | job ID | new attempt/status, subject to policy |

The contract never returns credentials or raw CSV contents. Status reads must reject foreign-organization job IDs as not found or forbidden according to the existing server-function convention.

`list_harvest_import_jobs(before)` returns up to 20 jobs, ordered by creation
time and then UUID, both descending. Pass the last job ID as `before` to fetch
the next page, or omit it for the latest jobs. The cursor is resolved inside the
current organization; an unknown, expired, or foreign cursor returns no rows.
Refreshing without a cursor returns to the latest retained history.

## Cancellation and acknowledgement

A successful cancel request means the request was accepted, not that an active
import has already stopped. Queued jobs become `cancelled` immediately. Running
jobs remain `running` with phase `cancelling`; retry is rejected until they reach
the terminal `cancelled` state. The worker stops the SQL consumer, joins its
source producer, and flushes rollback before recording that acknowledgement.

If the worker crashes before acknowledging, recovery marks the expired claim
cancelled instead of executing it again. Each claim has a new UUIDv7 token.
Import commits and terminal reports are conditional on that token, a live lease,
and no cancellation request, in the same database transaction as domain writes.
An obsolete execution cannot confirm writes or overwrite the replacement's job
state, even if its heartbeat never observes the loss.
