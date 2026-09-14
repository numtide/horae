# Import Jobs Contract

Job operations are authenticated server functions restricted to organization
administrators. The error download is an additional read-only, session-checked
file response with the same administrator and organization boundary.

| Operation | Input | Result |
|---|---|---|
| `start_harvest_api_import` | mode, sync scope | job ID + initial status |
| `start_harvest_csv_import` | mode, upload | job ID + initial status |
| `get_harvest_import_job` | job ID | status, progress, report/error |
| `list_harvest_import_jobs` | optional limit/cursor | recent jobs for current organization |
| `cancel_harvest_import_job` | job ID | updated status |
| `retry_harvest_import_job` | job ID | new attempt/status, subject to policy |

Start, cancel and retry return the same public `JobStatus` shape as status reads,
including `id`, `status`, progress and retained outcomes.
They do not wait for import completion. The returned snapshot can already show
worker progress; clients continue polling the returned ID. A running cancel
reply can show `running` / `cancelling`, never a promise of completed cleanup.

The contract never returns credentials or raw CSV contents. Status reads must reject foreign-organization job IDs as not found or forbidden according to the existing server-function convention.

The former synchronous `import_harvest_api` and `import_harvest_csv` functions
are not registered. In particular, `/api/import/harvest/csv/{mode}` is no longer
an import endpoint. CSV starts use `POST /api/import/harvest/csv-job/{mode}` with
the `X-Horae-Import: csv` header and the administrator's session. Old browser
tabs must reload after an upgrade; custom clients must use the start/poll
contract above rather than await an `ImportReport` in the start response.

`list_harvest_import_jobs(before, limit)` defaults to 20 jobs; an explicit limit
is clamped to 1–100. Jobs are ordered by creation time and then UUID, both
descending. Pass the last job ID as `before` to fetch
the next page, or omit it for the latest jobs. The cursor is resolved inside the
current organization; an unknown, expired, or foreign cursor returns no rows.
Refreshing without a cursor returns to the latest retained history.

## Reports and failed attempts

`report` contains the last confirmed outcomes, not an assertion that the import
finished. A new Harvest job starts with a zero-outcome report for its source/mode.
Checkpoints publish updated outcomes atomically with progress; a later error or
expired final attempt cannot erase them or include rolled-back work. Only a
`succeeded` job's report covers the completed source. `last_error` describes the
latest attempt failure separately from the report's per-record errors.

Failed/cancelled reports remain readable from status and history throughout job
retention, and manual retry retains confirmed outcomes. The UI marks them as
partial, shows the interruption/error, and disallows confirming an interrupted
preview. Older checkpoint-only reports remain readable without exposing private
source cursors, caches, upload contents or credentials.

## Complete error download

`GET /api/import/harvest/jobs/{job_id}/errors` returns an attachment containing
UTF-8 JSON lines (`application/x-ndjson`), one complete `RowError` per line.
The filename is derived from the job UUID, not imported text. Responses disable
caching and MIME sniffing. The handler checks the user's current active/admin
state; foreign or missing jobs are not found.

Version-2 reports may keep error details in an archive, with a bounded inline
tail. Display totals include both; the UI identifies the inline subset and links
to the complete download. A download captures one confirmed report boundary, so
later progress cannot append errors to that response. Missing fragments fail the
stream instead of producing successful truncated output.

Startup converts oversized legacy reports to the archive before serving these
endpoints. Unsupported or malformed reports fail the upgrade without discarding
their original data. Database constraints prevent older workers from publishing
new oversized report values.

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
