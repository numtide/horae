# Quickstart: Harvest Data Importer

Runnable validation guide proving the importer works end-to-end. The **primary flow connects Harvest over OAuth2 and pulls via the API**; a CSV import is the secondary, offline fallback. Commands assume the Nix dev shell and a running Postgres (see repo `AGENTS.md`). Contracts: [importer-api.md](./contracts/importer-api.md), [harvest-api.md](./contracts/harvest-api.md), [csv-format.md](./contracts/csv-format.md). Data model: [data-model.md](./data-model.md).

## Prerequisites

```sh
nix develop                                  # dev shell
export DATABASE_URL=postgres://localhost/horae
cargo run -p horae --features server -- migrate run    # includes credentials, provenance and account binding
# Provision the users whose time will be imported (importer matches by email, never creates users):
cargo run -p horae --features server -- user create --email dev@example.com --name "Dev User" --role member
```

For the API flow, configure the Harvest OAuth app credentials (client id/secret, redirect URL) and the token-encryption key via Horae's configuration, alongside the existing OIDC/session secrets (see `config.rs`). The redirect URL must point at Horae's callback route `/auth/harvest/callback`.

## Scenario 1 — Connect Harvest via OAuth (US1, FR-022)

1. Sign in as an admin and open the "Import from Harvest" screen; click **Connect Harvest**.
1. Horae redirects to Harvest's authorization page; sign in and authorize.
1. Harvest redirects back to `/auth/harvest/callback`; Horae exchanges the code, resolves the Harvest account id, and stores the access + refresh tokens **encrypted** in `harvest_credentials`.

**Expected**: the screen shows a connected state (account id, token freshness) — and the tokens themselves are never shown or logged. No Harvest data is read until an import is run.

## Account identity and recovery

Each Horae organization is bound to its first connected Harvest account, including
after **Disconnect**. Reconnecting the same account can replace expired tokens or
recover from encryption-key rotation. Connecting another account returns a clear
conflict without replacing credentials, watermarks, provenance or imported data.
Disconnect removes OAuth secrets and the watermark, not imported records or the
binding; a subsequent same-account sync safely replays records by Harvest ID.

Before deploying the account-binding migration, stop all Horae server instances
and importer processes, apply migrations, then restart all instances on the new
version. The migration binds existing credential rows to their recorded accounts.
It cannot reconstruct the source account of old provenance if credentials were
already deleted, nor repair mappings mixed by an earlier account switch.

For the specific error about **unverified existing import identities**:

1. Back up the database and stop all server/importer processes.
1. Verify the original Harvest account ID against a trusted credential backup or
   import records. Confirm that all of the organization's existing provenance
   belongs to that one account. Do not infer identity from matching numeric IDs
   or client/project names alone. If identity cannot be established, leave the
   connection blocked and retain the data for review.
1. Check that no `harvest_account_bindings` or `harvest_credentials` row exists
   for that organization. Restore only the missing binding with an operator SQL
   `INSERT INTO harvest_account_bindings (org_id, harvest_account_id)` using the
   verified organization UUID and original account ID. Do not overwrite an
   existing binding or delete provenance to get past the guard.
1. Restart Horae, reconnect the verified original account, preview a full sync,
   and review the report before committing it.

This is identity recovery, not an account-switch procedure. Importing a different
account requires a separate Horae deployment/database or an explicitly designed
data migration that reconciles existing identities and monetary history. No
automatic reset, data deletion or cross-account migration is provided.

## Scenario 2 — Dry-run the API import previews without writing (US3, FR-014)

```sh
cargo run -p horae --features server -- import harvest-api --full --dry-run
```

**Expected**: a summary table reporting would-create/update/skip/error per entity type (clients, projects, tasks, time entries); any problem records appear in the error report with their Harvest id + reason; **no rows are written** — `clients`, `time_entries`, `harvest_import_map`, and the `harvest_credentials.synced_watermark` are all unchanged.

## Scenario 3 — First real API import populates Horae (US1, FR-004/005/006/023)

```sh
cargo run -p horae --features server -- import harvest-api --full
```

**Expected**: the importer pages through Harvest (respecting the rate limit), then creates clients first, then projects under them, then tasks (+ per-project enablement), then time entries. A 1.5-hour Harvest entry stores `minutes = 90` exactly; an entry with a billable rate stores integer cents + ISO currency. Provenance rows are written to `harvest_import_map` for every created record, and the summary counts match the dry-run's would-create numbers. The unknown-user rows are reported as errors and skipped (US4).

### Catalog records without time

Include an unused client, a project with no time and an unused task in the API
fixture. All must appear in the catalog and summary even when the entire time
collection is empty. Repeat the import: no duplicates, with each parent counted
once as skipped. A preview must report the same creations without persisting any
catalog/provenance rows; a parent-only commit must not advance the time watermark.

Include a malformed task default rate and a time entry referencing that task.
Both the task and dependent entry must have clear errors, while unrelated catalog
records import successfully. Correct the task and retry: it and its time entry
are created, previously imported parents are skipped, and only the error-free
run can advance the watermark. A project with an unresolved client must not
invent a placeholder client or leave behind partial provenance.

## Scenario 4 — Re-sync is idempotent and edit-robust (US2, FR-011/FR-026, SC-002)

```sh
cargo run -p horae --features server -- import harvest-api --full      # unchanged data
```

**Expected**: the second run creates **zero** new records — all matched by provenance (Harvest id) and reported as skipped/unchanged. Then edit one imported entry's notes in Horae and re-run: it is **still** matched to the same entry by Harvest id (not duplicated), proving provenance matching survives edits that a pure natural key would miss.

```sh
cargo run -p horae --features server -- import harvest-api --incremental
```

**Expected**: an incremental re-sync filters time entries using `updated_since` from the stored watermark; parent collections are still fetched in full. Only the time-entry watermark advances, bounded by both the latest source timestamp and the start of the fetch, with a one-second overlap for timestamp precision and boundary records. Provenance keeps repeated boundary records duplicate-free. An empty response or any missing time-entry timestamp leaves the previous watermark unchanged (SC-008).

## Scenario 5 — Partial success reconciles (US4, FR-018/020/021, SC-005)

**Expected** (already observable from Scenario 3's report): valid records imported, invalid records skipped with per-record reasons and their source location, no partial fragments (and no dangling provenance rows) left behind, and per entity type `processed = created + updated + skipped + errored`.

For the API path, include an older entry whose user is absent from Horae and a newer valid entry. After the partial import, the previous watermark must remain unchanged. Create the missing user and run an incremental sync: the failed entry is now created, the already-imported entry is skipped, and the error-free run can advance the watermark. No retry queue is needed; failed runs deliberately retain the earlier fetch boundary. Successful data writes and their watermark update commit in one transaction, so a failed watermark write rolls back that run's data and provenance as well.

If a previous version already advanced past failed records, correct their reported errors and run a full sync once; retaining the current watermark cannot recover records that an older run already excluded.

## Scenario 6 — CSV fallback (secondary source, US5)

Prepare a small Harvest detailed-time-report CSV fixture (columns per `contracts/csv-format.md`) with a couple of clients/projects/tasks and a handful of entries — including one unknown-user and one malformed-date row.

```sh
cargo run -p horae --features server -- import harvest-csv harvest-sample.csv --dry-run
cargo run -p horae --features server -- import harvest-csv harvest-sample.csv
cargo run -p horae --features server -- import harvest-csv harvest-sample.csv   # re-run: zero creations
```

**Expected**: same FK-safe order, exact conversions, dry-run, and per-row error behavior as the API path; the re-run creates zero duplicates (matched by the composite natural key, since the CSV carries no Harvest ids and writes no provenance).

Also exercise a name-only export: remove the email column, add `First Name` / `Last Name`, and match an existing user's full name. Dry-run creates nothing, commit assigns the correct user, and re-import skips the stored entries (including repeated identical rows). A blank email permits the same fallback; an unknown nonblank email does not. Add a second user with the same normalized full name, including an inactive user: the name-only row must error, while a unique email still selects the intended user. Two emails that differ only by ASCII case must be rejected as ambiguous in both CSV and API imports. Users with the same name in another organization do not affect matching. See the CSV contract for alias and header validation.

## Automated tests backing these scenarios

Lookup-query counts, cold-index measurements and remaining scale limits are documented in [Import lookup measurements](performance.md).

- `cargo test -p horae-core` — pure conversions and natural-key normalization: `hours → minutes`, `money → cents` (round-trip vs. the export transforms), trim/case-fold key equality.
- `cargo test -p horae --features server` (`#[sqlx::test]`, `#[serial]`) — FK-safe insertion, provenance-based idempotent re-run (including edited-after-import), dry-run-writes-nothing (rows, provenance, and watermark unchanged), incremental-watermark behavior, unknown-user row errors, and summary reconciliation. The API adapter is exercised against **stubbed HTTP fixtures** (paged JSON, a 429 backoff, a token refresh) so no live Harvest account is needed; fixture CSVs under `crates/horae/tests/` drive the secondary-source tests.

## Formatting / cache gate before merge

```sh
nix fmt
cargo clippy -p horae --features server
# The new migrations add queries, so regenerate the sqlx cache:
cargo sqlx prepare --workspace -- --features server --all-targets && git add .sqlx/
nix flake check
```
