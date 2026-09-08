# Contract: Importer Interface (server functions + OAuth callback + CLI)

The importer is exposed through the existing authorized server-side path (Constitution IV) — admin-only Dioxus `#[server]` functions and/or a server-binary CLI — plus one plain Axum **OAuth callback** route (a browser redirect target, so it cannot be a `#[server]` fn). All import surfaces call the **same source-agnostic engine**, so behavior is identical regardless of source (API or CSV) or surface. Signatures are intent-level (types abbreviated); server errors use the repo's `ServerFnError` with named status codes (e.g. `FORBIDDEN`, `NOT_FOUND`), not integer literals. A two-state `mode` is the named enum `ImportMode { DryRun, Commit }`, never `Option<bool>`.

## 1. Connect Harvest (OAuth2) — admin-only

```
harvest_connect_start() -> Result<AuthorizeUrl, ServerFnError>   // returns the Harvest authorize URL to redirect to
```

- Admin-only; rejects non-administrators with `FORBIDDEN` (FR-001).
- Builds Harvest's authorization-code URL (configured client id, redirect URL, `state`, PKCE). The `state` is a **random nonce generated per start, tied to the initiating admin's session** (stored server-side/session-bound) for callback validation. The SPA redirects the browser there.

```
GET /auth/harvest/callback?code=…&state=…      (plain Axum route, beside auth::router())
```

- The browser redirect target after the admin authorizes on Harvest (cannot be a `#[server]` fn).
- **Validates `state`** against the value stored at connect-start for this session, rejecting a missing or mismatched `state` **without exchanging the code** — this prevents CSRF / forged or replayed callbacks. Only on a valid `state` does it proceed.
- Exchanges `code` for `access_token` + `refresh_token`, resolves the Harvest **account id**, and stores the tokens **encrypted at rest** alongside the account ID in `harvest_credentials`, scoped to the org (FR-022). Tokens are never returned to the browser or logged; the account ID is non-secret metadata.
- On success, redirects back into the admin "Import from Harvest" screen showing a connected state.

The organization is permanently bound to the first connected Harvest account.
Reconnect may replace credentials for that same account (including after key
rotation), but cannot switch accounts. Account mismatch or unidentified legacy
provenance returns a plain-text `409 Conflict` with recovery instructions; only
these known policy errors and the import-busy message are exposed, never tokens
or arbitrary upstream errors. Return to the import screen and reconnect the
original account, or ask the operator to verify legacy identity.

Credential storage and disconnect acquire the same organization lock as imports.
If an import is running, they reject with `CONFLICT` rather than replacing or
removing credentials underneath its token refresh or data transaction. OAuth
code exchange/account discovery precedes credential storage; a rejected callback
must start a fresh connect attempt when the conflicting operation has finished.

```
harvest_disconnect() -> Result<(), ServerFnError>
```

Admin-only. Removes stored OAuth credentials but preserves account binding,
provenance and imported records. It does not revoke tokens at Harvest. A later
connection must use the original account. Disconnect removes the watermark with
the credential row; the next sync replays data through existing provenance.

```
harvest_connection_status() -> Result<ConnectionStatus, ServerFnError>   // connected? account id, token freshness — never the tokens
```

## 2. Import from the Harvest API (PRIMARY) — admin-only

API and CSV imports share one PostgreSQL advisory lock per organization, across
server processes. Both previews and committing runs hold it. A competing run is
rejected with `CONFLICT` and a retry message before resolving or applying rows; API
runs acquire it before loading credentials, refreshing tokens, or fetching data.
Other organizations can import independently. This does not impose uniqueness
on time-entry fields: distinct Harvest IDs and repeated CSV occurrences remain
distinct entries.

The lock uses one pooled connection for the whole run, including token refresh
outside the data transaction. The connection is closed after the run and on
errors or cancellation, so a session lock cannot return to the shared pool.
Cancellation rolls back uncommitted data; a blocking HTTP call already in flight
keeps the lock until it finishes, but its cancelled importer cannot apply the
result. Successfully persisted refreshed OAuth tokens remain stored even if
later data work is rolled back. Use a direct PostgreSQL connection or a pooler
with session affinity; transaction-mode pooling cannot preserve session locks.

```
import_harvest_api(
    mode: ImportMode,        // DryRun | Commit
    sync: SyncScope,         // Full | Incremental  — Incremental uses the stored updated_since watermark (FR-025)
) -> Result<ImportReport, ServerFnError>
```

- **Authorization**: rejects non-administrators with `FORBIDDEN` (FR-001). Reads the acting admin + single org from the session/`AppState`.
- **Precondition**: requires a usable Harvest connection; with none, rejects up front with a clear "connect Harvest" message and no writes (FR-003). Refreshes an expired access token transparently; a failed refresh → reject with "reconnect Harvest" (FR-024).
- **Pull**: fetches clients → projects → tasks/assignments → users(reference) → time entries, following pagination and respecting the rate limit (FR-023), then feeds the normalized rows through the shared engine.

API clients, projects and tasks are applied as independent catalog records before
time entries, including records with no time. They reuse the same provenance-first
resolvers as denormalized time/CSV rows. The catalog and time phases share one
run cache, report and outer transaction: parents are counted once, dry-run rolls
back both phases, and a watermark-write failure also rolls back catalog writes.
An import containing only parents does not invent a time-entry watermark.

Each catalog record has its own savepoint. A failed parent is reported by type
and Harvest ID; dependent projects/time entries fail visibly instead of creating
placeholder parents. Independent records continue. A project whose client is
absent from the client collection may resolve existing provenance or use its
embedded client name. An ID without either is not enough to create a client.
Parent errors retain the previous incremental watermark, so a corrected retry
can recover dependent time. Existing matched records remain unchanged (FR-017).

- **`mode = DryRun`**: full pull → resolve → plan against live data, returns the report with **zero writes** — no data, no provenance, no watermark update (FR-014). `mode = Commit`: applies the plan and writes provenance + advances the watermark on success (FR-015, FR-025, FR-026).
- **`SyncScope`** is a plainly named two-state enum (not `Option<bool>`); `Incremental` sends `updated_since` from `harvest_credentials.synced_watermark`.

## 3. Import from a Harvest CSV (SECONDARY, offline) — admin-only

```
import_harvest_csv(
    mode: ImportMode,       // DryRun | Commit, encoded in the route
    file: CsvUpload,        // native browser file, sent as the raw request body
) -> Result<ImportReport, ServerFnError>
```

- Same authorization and dry-run/commit semantics as the API import; the only difference is the source adapter (research.md §1).
- **Validation**: rejects an unrecognized/empty file up front with a clear message and no writes (FR-003).
- Because a CSV carries no Harvest ids, matching uses the composite natural key only; no provenance rows are written (data-model.md).

The CSV function is `POST /api/import/harvest/csv/DryRun` (or `/Commit`).
It remains a Dioxus server function with the same active-admin session check;
the organization and default currency come from the server, never the file or
request parameters. Its body is raw CSV, not a JSON byte array. Requests must
include `X-Horae-Import: csv`; simple cross-site form submissions cannot supply
that header. Do not enable credentialed cross-origin CORS on this endpoint.
The route has exactly one mode segment; an invalid mode returns `BAD_REQUEST`
without reading the upload body.

The browser retains its native file handle for preview and commit without
reading the whole file into WASM memory. The server's blocking CSV parser feeds
a one-record queue consumed by the existing async row/savepoint pipeline. SQL
backpressure stops further parsing and body reads. Headers and the first record
are checked before opening the data transaction. Record-level validation errors
are reported in input order; a transport/read failure rejects the entire run and
rolls back its writes. Only successful parsing through EOF permits a commit.
Cancellation wakes a parser waiting for more bytes or a full queue; it retains
the organization-lock connection until it exits.

This bounds queued records, not total process memory. A record or body frame can
be large, occurrence/cache keys and the complete error report still grow with
input, and one outer transaction still spans the run. See [performance.md](../performance.md).

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

## Decimal limits

Decimal conversion is exact and rounds half up. The parser accepts up to 38
fractional digits with an unscaled numerator no larger than `i128::MAX`;
converted minutes/cents must fit in `i64`. The entry writer additionally checks
that stored minutes fit in PostgreSQL's non-negative `integer` range.
High fractional precision does not overflow the scaling intermediate.
An out-of-range value or precision produces a typed conversion error, reported
against that source row. Its savepoint is rolled back and subsequent rows are
still processed; values are never clamped or silently wrapped.

## Out of scope (v1)

- **Propagating Harvest deletions** — a "mirror-delete" re-sync mode that removes records deleted in Harvest. Re-sync is additive/updating only (`updated_since` never reports deletions), so upstream-deleted records remain in Horae until an admin removes them manually (FR-025, harvest-api.md).
- Scheduled / automatic re-sync jobs — this version re-syncs only when an admin runs `import_harvest_api`.
- Connecting more than one Harvest account per organization.
- Importing Harvest entities beyond clients/projects/tasks/time entries (invoices, estimates, expenses, users-as-accounts, roles, teams).
