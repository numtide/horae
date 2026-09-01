# Quickstart: Harvest Data Importer

Runnable validation guide proving the importer works end-to-end. Commands assume the Nix dev shell and a running Postgres (see repo `AGENTS.md`). Contracts: [importer-api.md](./contracts/importer-api.md), [csv-format.md](./contracts/csv-format.md). Data model: [data-model.md](./data-model.md).

## Prerequisites

```sh
nix develop                                  # dev shell
export DATABASE_URL=postgres://localhost/horae
cargo run -p horae --features server -- migrate run
# Provision the users whose time will be imported (importer matches by email, never creates users):
cargo run -p horae --features server -- user create --email dev@example.com --name "Dev User" --role member
```

Prepare a small Harvest detailed-time-report CSV fixture (columns per `contracts/csv-format.md`), e.g. `harvest-sample.csv` with a couple of clients, projects, tasks, and a handful of entries — including one row with an unknown user email and one with a malformed date to exercise per-row errors.

## Scenario 1 — Dry-run previews without writing (US3, FR-014)

```sh
cargo run -p horae --features server -- import harvest harvest-sample.csv --dry-run
```

**Expected**: a summary table reporting would-create/update/skip/error per entity type (clients, projects, tasks, time entries); the bad rows appear in the error report with source line + reason; **no rows are written** — confirm counts in Horae are unchanged (e.g. `SELECT count(*) FROM clients;` before and after are equal).

## Scenario 2 — First real import populates Horae (US1, FR-004/005/006)

```sh
cargo run -p horae --features server -- import harvest harvest-sample.csv
```

**Expected**: clients created first, then projects under them, then tasks (+ per-project enablement), then time entries. A 1.5-hour Harvest row stores `minutes = 90` exactly; a row with a billable rate stores integer cents + ISO currency. Summary counts match the dry-run's would-create numbers. Valid rows imported; the unknown-user and malformed-date rows are reported as errors and skipped (US4).

## Scenario 3 — Re-run is idempotent (US2, FR-011, SC-002)

```sh
cargo run -p horae --features server -- import harvest harvest-sample.csv
```

**Expected**: the second run creates **zero** new clients, projects, tasks, or time entries — all matched by natural key and reported as skipped/unchanged. Row counts in Horae are identical to after Scenario 2. Add a new row to the CSV and re-run: only the new row is created; everything else stays skipped.

## Scenario 4 — Partial success reconciles (US4, FR-018/020/021, SC-005)

**Expected** (already observable from Scenario 2's report): valid rows imported, invalid rows skipped with per-row reasons, no partial fragments left behind, and per entity type `processed = created + updated + skipped + errored`.

## Automated tests backing these scenarios

- `cargo test -p horae-core` — pure conversions and natural-key normalization: `hours → minutes`, `money → cents` (round-trip vs. the export transforms), trim/case-fold key equality.
- `cargo test -p horae --features server` (`#[sqlx::test]`, `#[serial]`) — FK-safe insertion, idempotent re-run (zero creations), dry-run-writes-nothing (row counts unchanged), unknown-user row errors, and summary reconciliation, driven by small fixture CSVs under `crates/horae/tests/`.

## Formatting / cache gate before merge

```sh
nix fmt
cargo clippy -p horae --features server
# If any sqlx query macro changed:
cargo sqlx prepare --workspace -- --features server --all-targets && git add .sqlx/
nix flake check
```
