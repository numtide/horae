# Quickstart: Harvest Data Importer

Runnable validation guide proving the importer works end-to-end. The **primary flow connects Harvest over OAuth2 and pulls via the API**; a CSV import is the secondary, offline fallback. Commands assume the Nix dev shell and a running Postgres (see repo `AGENTS.md`). Contracts: [importer-api.md](./contracts/importer-api.md), [harvest-api.md](./contracts/harvest-api.md), [csv-format.md](./contracts/csv-format.md). Data model: [data-model.md](./data-model.md).

## Prerequisites

```sh
nix develop                                  # dev shell
export DATABASE_URL=postgres://localhost/horae
cargo run -p horae --features server -- migrate run    # applies the two new tables: harvest_credentials, harvest_import_map
# Provision the users whose time will be imported (importer matches by email, never creates users):
cargo run -p horae --features server -- user create --email dev@example.com --name "Dev User" --role member
```

For the API flow, configure the Harvest OAuth app credentials (client id/secret, redirect URL) and the token-encryption key via Horae's configuration, alongside the existing OIDC/session secrets (see `config.rs`). The redirect URL must point at Horae's callback route `/auth/harvest/callback`.

## Scenario 1 — Connect Harvest via OAuth (US1, FR-022)

1. Sign in as an admin and open the "Import from Harvest" screen; click **Connect Harvest**.
1. Horae redirects to Harvest's authorization page; sign in and authorize.
1. Harvest redirects back to `/auth/harvest/callback`; Horae exchanges the code, resolves the Harvest account id, and stores the access + refresh tokens **encrypted** in `harvest_credentials`.

**Expected**: the screen shows a connected state (account id, token freshness) — and the tokens themselves are never shown or logged. No Harvest data is read until an import is run.

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

## Scenario 4 — Re-sync is idempotent and edit-robust (US2, FR-011/FR-026, SC-002)

```sh
cargo run -p horae --features server -- import harvest-api --full      # unchanged data
```

**Expected**: the second run creates **zero** new records — all matched by provenance (Harvest id) and reported as skipped/unchanged. Then edit one imported entry's notes in Horae and re-run: it is **still** matched to the same entry by Harvest id (not duplicated), proving provenance matching survives edits that a pure natural key would miss.

```sh
cargo run -p horae --features server -- import harvest-api --incremental
```

**Expected**: an incremental re-sync fetches only records Harvest changed since the last successful run (`updated_since` from the stored watermark) and applies just those, leaving unchanged records intact (SC-008).

## Scenario 5 — Partial success reconciles (US4, FR-018/020/021, SC-005)

**Expected** (already observable from Scenario 3's report): valid records imported, invalid records skipped with per-record reasons and their source location, no partial fragments (and no dangling provenance rows) left behind, and per entity type `processed = created + updated + skipped + errored`.

## Scenario 6 — CSV fallback (secondary source, US5)

Prepare a small Harvest detailed-time-report CSV fixture (columns per `contracts/csv-format.md`) with a couple of clients/projects/tasks and a handful of entries — including one unknown-user and one malformed-date row.

```sh
cargo run -p horae --features server -- import harvest-csv harvest-sample.csv --dry-run
cargo run -p horae --features server -- import harvest-csv harvest-sample.csv
cargo run -p horae --features server -- import harvest-csv harvest-sample.csv   # re-run: zero creations
```

**Expected**: same FK-safe order, exact conversions, dry-run, and per-row error behavior as the API path; the re-run creates zero duplicates (matched by the composite natural key, since the CSV carries no Harvest ids and writes no provenance).

## Automated tests backing these scenarios

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
