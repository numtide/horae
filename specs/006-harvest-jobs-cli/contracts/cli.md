# Contract: Harvest Jobs CLI

## Commands

Existing server-feature binary; `--session-file PATH` or `HORAE_SESSION_FILE` selects credentials. Global `--json` and `--session-file` can appear before or after subcommands. Help requires no credentials. These commands never load deployment configuration or open a database connection.

```text
horae import harvest-api [--dry-run] [--full | --incremental] [--request-id UUID] [--wait [--timeout SECONDS]]
horae import harvest-csv FILE [--dry-run] [--request-id UUID] [--wait [--timeout SECONDS]]
horae jobs status UUID
horae jobs list [--before UUID] [--limit 1..100]
horae jobs report UUID
horae jobs errors UUID --output FILE [--force]
horae jobs cancel UUID
horae jobs retry UUID [--wait [--timeout SECONDS]]
horae jobs wait UUID [--timeout SECONDS]
```

Default commit, preview via `--dry-run`; default API Incremental with full fallback without a watermark. Timeout is an integer from 1 to 86400 seconds for waiting only, and caps pending network requests. Poll every second. Interrupting/closing the CLI never implicitly cancels jobs.

Print the generated request ID to stderr before submission and include it in results. Identical resubmission with `--request-id` within 24 hours returns the same job. Keys are scoped to organization and API/CSV source kind; changed mode/scope/content conflicts. Expired keys fail; use `jobs retry` for existing failed/cancelled jobs.

## HTTP contract

Explicit POST paths for existing administrator-checked Dioxus functions:

| Function | Path | JSON fields |
|---|---|---|
| start_harvest_api_import | `/api/import/harvest/start` | mode, sync |
| get_harvest_import_job | `/api/import/harvest/status` | job_id |
| list_harvest_import_jobs | `/api/import/harvest/history` | before, limit |
| cancel_harvest_import_job | `/api/import/harvest/cancel` | job_id |
| retry_harvest_import_job | `/api/import/harvest/retry` | job_id |

CSV remains raw `POST /api/import/harvest/csv-job/{DryRun|Commit}` with `X-Horae-Import: csv`. Both starts accept optional `X-Horae-Idempotency-Key`; absence retains fresh web submissions. Errors remain `GET /api/import/harvest/jobs/{job_id}/errors`.

No inline endpoint restored. Rebuild web bundle with the server when upgrading routes; old tabs reload. CLI/server must support this contract; version/route mismatch fails, never falls back to local execution.

## Security and output

See [data-model.md](../data-model.md) for file schema and exits. HTTPS except loopback, no redirects, no raw secrets in arguments, active admin/org checks per request. Missing/expired/foreign jobs never create replacements.

Success is the raw Dioxus result value. Classify failures by HTTP status without printing arbitrary remote bodies. JSON mode emits one version-1 envelope to stdout; diagnostics/progress go to stderr. Completed imports with record errors are explicitly partial success.

`jobs report` identifies confirmed summaries as incomplete unless succeeded. `jobs errors` requires `--output` even in JSON mode. Stream to a private sibling temporary file; publish only after successful EOF. Never overwrite without `--force`; failure leaves an existing destination unchanged.
