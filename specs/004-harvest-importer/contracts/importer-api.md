# Contract: Importer Interface (server functions + OAuth callback + CLI)

The importer is exposed through the existing authorized server-side path (Constitution IV) — admin-only Dioxus `#[server]` functions and/or a server-binary CLI — plus one plain Axum **OAuth callback** route (a browser redirect target, so it cannot be a `#[server]` fn). All import surfaces call the **same source-agnostic engine**, so behavior is identical regardless of source (API or CSV) or surface. Signatures are intent-level (types abbreviated); server errors use the repo's `ServerFnError` with named status codes (e.g. `FORBIDDEN`, `NOT_FOUND`), not integer literals. A two-state `mode` is the named enum `ImportMode { DryRun, Commit }`, never `Option<bool>`.

## 1. Connect Harvest (OAuth2) — admin-only

```
harvest_connect_start() -> Result<AuthorizeUrl, ServerFnError>   // returns the Harvest authorize URL to redirect to
```

- Admin-only; rejects non-administrators with `FORBIDDEN` (FR-001).
- Builds Harvest's authorization-code URL (configured client id, redirect URL, state/PKCE). The SPA redirects the browser there.

```
GET /auth/harvest/callback?code=…&state=…      (plain Axum route, beside auth::router())
```

- The browser redirect target after the admin authorizes on Harvest (cannot be a `#[server]` fn).
- Exchanges `code` for `access_token` + `refresh_token`, resolves the Harvest **account id**, and stores all three **encrypted at rest** in `harvest_credentials`, scoped to the org (FR-022). Tokens are never returned to the browser or logged.
- On success, redirects back into the admin "Import from Harvest" screen showing a connected state.

```
harvest_connection_status() -> Result<ConnectionStatus, ServerFnError>   // connected? account id, token freshness — never the tokens
```

## 2. Import from the Harvest API (PRIMARY) — admin-only

```
import_harvest_api(
    mode: ImportMode,        // DryRun | Commit
    sync: SyncScope,         // Full | Incremental  — Incremental uses the stored updated_since watermark (FR-025)
) -> Result<ImportReport, ServerFnError>
```

- **Authorization**: rejects non-administrators with `FORBIDDEN` (FR-001). Reads the acting admin + single org from the session/`AppState`.
- **Precondition**: requires a usable Harvest connection; with none, rejects up front with a clear "connect Harvest" message and no writes (FR-003). Refreshes an expired access token transparently; a failed refresh → reject with "reconnect Harvest" (FR-024).
- **Pull**: fetches clients → projects → tasks/assignments → users(reference) → time entries, following pagination and respecting the rate limit (FR-023), then feeds the normalized rows through the shared engine.
- **`mode = DryRun`**: full pull → resolve → plan against live data, returns the report with **zero writes** — no data, no provenance, no watermark update (FR-014). `mode = Commit`: applies the plan and writes provenance + advances the watermark on success (FR-015, FR-025, FR-026).
- **`SyncScope`** is a plainly named two-state enum (not `Option<bool>`); `Incremental` sends `updated_since` from `harvest_credentials.synced_watermark`.

## 3. Import from a Harvest CSV (SECONDARY, offline) — admin-only

```
import_harvest_csv(
    file: Vec<u8>,          // uploaded CSV bytes
    mode: ImportMode,       // DryRun | Commit
) -> Result<ImportReport, ServerFnError>
```

- Same authorization and dry-run/commit semantics as the API import; the only difference is the source adapter (research.md §1).
- **Validation**: rejects an unrecognized/empty file up front with a clear message and no writes (FR-003).
- Because a CSV carries no Harvest ids, matching uses the composite natural key only; no provenance rows are written (data-model.md).

## 4. CLI subcommands (operator, large-file / host-side)

```
horae import harvest-api  [--full | --incremental] [--dry-run]
horae import harvest-csv  <FILE> [--dry-run]
```

- Run on the `server` binary, sharing the same engine and DB layer as the server functions. `harvest-api` requires an existing connection (established via the UI's OAuth flow).
- `--dry-run` selects `ImportMode::DryRun`; default is `Commit`. `--incremental` uses the stored watermark; default for `harvest-api` is `--incremental` when a watermark exists, else full.
- Prints the summary table and the per-record error report; exits non-zero if the source is rejected up front (bad file, no/expired connection), zero when the run completes even with per-record errors (partial success is success — FR-018).

## Return shape: `ImportReport`

```
ImportReport {
    source: HarvestApi | Csv,
    mode: ImportMode,
    summary: {
        clients:      { created, updated, skipped, errored },
        projects:     { created, updated, skipped, errored },
        tasks:        { created, updated, skipped, errored },
        time_entries: { created, updated, skipped, errored },
    },
    row_errors: [ { source_location, entity, reason }, ... ],   // source_location = Harvest id (API) or CSV line (FR-019)
}
```

- Per entity type, `processed = created + updated + skipped + errored` MUST hold (FR-021, SC-005).
- In `DryRun` the counts are the would-create/update/skip/error preview and MUST match a subsequent `Commit` on the same unchanged input/data (FR-015, SC-004).

## Behavioral guarantees (cross-referenced)

- **FK-safe order**: clients → projects → tasks (+ `project_tasks`) → time entries (FR-004).
- **Idempotent**: matched rows are skipped/updated, never duplicated; a second identical run reports zero creations (FR-011). Matching is provenance-first (API), composite natural key fallback (CSV + first import) — data-model.md.
- **Edit-robust re-sync**: an API record edited in Horae or Harvest after import is still matched by Harvest id, not duplicated (FR-026, SC-002).
- **Resilient**: a bad record is skipped and reported, run continues; each record's writes (incl. its provenance row) are all-or-nothing (FR-018, FR-020).
- **Exact**: durations → integer minutes, money → integer cents + ISO currency, no floats; full reconciliation to zero drift (FR-005/FR-006, SC-003).
- **User matching only**: users resolved by email; never provisioned; unmatched → record error (FR-010).
- **No invoice coupling**: entries import as `open`, never `invoiced` from Harvest's billed flag (FR-016).
- **Credentials protected**: OAuth tokens stored encrypted, never returned to the browser or logged (FR-022).

## Out of scope (v1)

- Scheduled / automatic re-sync jobs — this version re-syncs only when an admin runs `import_harvest_api`.
- Connecting more than one Harvest account per organization.
- Importing Harvest entities beyond clients/projects/tasks/time entries (invoices, estimates, expenses, users-as-accounts, roles, teams).
