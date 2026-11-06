# Contract: Harvest section routes and server functions (PR B)

Every route sits under `AdminShell`, which keeps "Importers" highlighted on each of them, and every
server function calls `require_admin` (013 FR-002).

## Routes

| Route | Page |
|---|---|
| `/admin/importers` | `ImporterIndex`: Harvest API and Harvest CSV only, with the connection badge |
| `/admin/importers/harvest` | `HarvestOverview`: connection card, preview, latest 5 runs |
| `/admin/importers/harvest/oauth` | `HarvestOAuth`: OAuth client settings |
| `/admin/importers/harvest/csv` | `HarvestCsv`: file picker, preview |
| `/admin/importers/harvest/runs?before={id}` | `HarvestRuns`: 20 runs a page; `before` is the last run of the previous page, absent for the newest |
| `/admin/importers/harvest/runs/{id}` | `HarvestRun`: header and actions, progress, summary, error groups, people not imported |

Starting a preview, a commit or a re-sync opens the new run's page; a retry stays on its run. The
OAuth callback redirects to `/admin/importers/harvest?connected=1` on success and
`/admin/importers/harvest?error=1` on failure.

## Server functions

| Function | Result | Refusals |
|---|---|---|
| `harvest_import_error_summary(job_id)` | `ErrorSummary`: ≤ 50 groups and ≤ 500 people, with `omitted_errors` and `omitted_people` | `NOT_FOUND` for another organization's job; `FORBIDDEN` for a non-admin |
| `commit_harvest_import_preview(job_id)` | `JobStatus` of the commit job; the same job for a repeated call (key `preview-commit:{job_id}`) | `CONFLICT` when the job is not a succeeded preview, a newer job of the same source exists, the account changed, or the upload expired; `NOT_FOUND`; `FORBIDDEN` |
| `add_harvest_people(people: Vec<NewPerson>, activation: Activation)` | the created `User`s, members, active only for `Activation::Active` | `BAD_REQUEST` for 0 or more than 500 people, a missing or invalid email, or an email listed twice; `CONFLICT` for an email any account uses, ignoring case; `FORBIDDEN`. One transaction, all or nothing |

`JobStatus` gains `mode`, `source` and `commit_availability`
(`available | not_preview | unavailable_state | newer_import | previous_account | missing_upload | unknown`),
all read from the job request, so history labels a run before it has a report.

Unchanged and reused: connection, client settings, start, get and list, cancel, retry, and the
NDJSON error download.
