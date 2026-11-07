# Contract: Harvest section routes and server functions (PR B)

Every route sits under `AdminShell`, and every server function calls `require_admin` (013 FR-002).

## Routes

| Route | Page |
|---|---|
| `/admin/importers` | `ImporterIndex`: Harvest API and Harvest CSV only, with the connection badge |
| `/admin/importers/harvest` | `HarvestOverview`: connection card, preview, latest 5 runs |
| `/admin/importers/harvest/oauth` | `HarvestOAuthApp`: OAuth client settings |
| `/admin/importers/harvest/csv` | `HarvestCsv`: file picker, preview |
| `/admin/importers/harvest/runs?:before` | `HarvestRuns`: paginated history |
| `/admin/importers/harvest/runs/:id` | `HarvestRun`: header, progress, summary, error groups, missing people, created people |

The OAuth callback redirects to `/admin/importers/harvest?connected=1` on success and
`/admin/importers/harvest?error=1` on failure.

## Server functions

| Function | Result | Refusals |
|---|---|---|
| `harvest_import_error_summary(job_id)` | `ErrorSummary` (≤ 50 groups, ≤ 500 people, plus omitted counts) | `NOT_FOUND` for another organization's job; `FORBIDDEN` for a non-admin |
| `commit_harvest_import_preview(job_id)` | `JobStatus` of the commit job; idempotent per preview | `CONFLICT` when the job is not a succeeded preview, a newer job of the same source exists, the account changed, or the upload expired |
| `add_harvest_people(people: Vec<NewPerson>, activation: Activation)` | the created `User`s | `BAD_REQUEST` for a blank or invalid email; `CONFLICT` for an existing email; one transaction, all or nothing |

Unchanged and reused: connection, client settings, start, get and list, cancel, retry, and the
NDJSON error download.
