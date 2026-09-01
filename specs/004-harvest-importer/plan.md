# Implementation Plan: Harvest Data Importer

**Branch**: `004-harvest-importer` | **Date**: 2026-09-01 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/004-harvest-importer/spec.md`

## Summary

Let an organization administrator migrate existing Harvest data — clients, projects, tasks, and time entries — into Horae from Harvest's own CSV exports, so a team switching from Harvest does not start on an empty install. The approach parses a Harvest detailed-time-report CSV (denormalized rows carrying client/project/task/entry fields), resolves each row against existing Horae data by defined **natural keys**, and writes in FK-safe order (clients → projects → tasks → time entries) through a single authenticated, role-checked server-side path into PostgreSQL.

Correctness-critical conversions live in `horae-core`: Harvest decimal **hours → exact integer minutes** and decimal money **→ integer minor units (cents) + ISO currency**, the exact inverse of what `crates/horae/src/harvest/` already does when it emits the Harvest-compatible API (`hours = minutes/60`, `rate = cents/100`). The importer is **idempotent** (re-running never duplicates), offers a **dry-run** that reports would-create/update/skip/error without writing, and is **resilient** (a bad row is skipped and reported, never aborting the run) with a reconciling summary. A Harvest REST API (OAuth2) pull is acknowledged as a later source mode and the mapping/matching core is structured to accept either CSV rows or API records without a rewrite.

## Technical Context

**Language/Version**: Rust (edition 2024); the app is Dioxus fullstack (server + web targets). The importer's engine is server-side and pure-domain where it counts.

**Primary Dependencies**: `csv` (already a workspace dependency, used by `reports.rs` for export) for parsing; `sqlx` 0.8 (Postgres) for the FK-safe writes; `uuid` v7 for keys; `chrono` for dates; `serde` for row structs. Conversions reuse `horae-core` (`rounding`, money/duration helpers). Delivery surface via the existing `clap` CLI and/or a Dioxus `#[server]` function + admin upload page. No new heavy dependency is anticipated (per `ponytail`: reuse `csv` rather than adding a parser).

**Storage**: PostgreSQL 15+ via `sqlx`, existing schema in `crates/horae/migrations/`. Whether a new provenance/mapping migration is added is decided in Phase 0 (research.md) and, if adopted, specified in data-model.md.

**Testing**: `cargo test -p horae-core` for the pure conversion/matching helpers (hours→minutes, money→cents, natural-key normalization); `cargo test -p horae --features server` with `#[sqlx::test]` (throwaway DB, `#[serial]`) for FK-safe insertion, idempotent re-runs, dry-run-writes-nothing, and partial-failure reconciliation. Fixture CSVs (small, hand-authored) drive integration tests.

**Target Platform**: Linux server (self-hosted); administrator invokes via CLI on the host and/or an admin screen in the WASM SPA.

**Project Type**: Web application — Dioxus fullstack app plus the pure-domain `horae-core` crate; this feature adds a server-side importer module and a delivery surface.

**Performance Goals**: Import an export of ≥100,000 time-entry rows to completion without exhausting memory (SC-006) — stream/iterate rows and batch writes rather than materializing the whole file and result set at once. No interactive latency target; this is a batch operation.

**Constraints**: Exactness is non-negotiable — durations stored as integer minutes, money as integer minor units + ISO currency, never floats (Constitution I). UUID v7 keys; Postgres-only; every created row carries `org_id`. `horae-core` stays free of `sqlx`/`axum`/`dioxus`. All writes go through the existing authorized server-side mutation path — the importer introduces no second write path (Constitution IV). Idempotency and per-row resilience are hard requirements (FR-011, FR-018).

**Scale/Scope**: Single organization. Four entity levels (clients, projects, tasks, time entries) plus the org-level task catalog's per-project enablement. One import engine, two source adapters conceptually (CSV now, API later — only CSV built).

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

Checked against the ratified project constitution (`.specify/memory/constitution.md`, **v1.0.0**):

- **I. Exactness (non-negotiable)**: Harvest decimal hours/money are converted to integer minutes and integer minor units + ISO currency in `horae-core`, with round-trip reconciliation asserted in tests (FR-005, FR-006, SC-003). No float is stored. ✅
- **II. Domain purity**: hours→minutes, money→cents, and natural-key normalization (trim/case-fold) live in `horae-core` with no I/O deps and are unit-tested in isolation. ✅
- **III. Single datastore**: PostgreSQL only; created rows use UUID v7 and carry `org_id`; FK-safe insertion order matches the existing schema's foreign keys. ✅
- **IV. Mutations through server functions**: the import writes through the existing session-authenticated, role-checked server-side path (a `#[server]` function and/or the server-binary CLI, which shares the same domain/DB layer); the read-only Harvest API stays read-only and is referenced only for field-shape semantics. No second mutation path. ✅
- **V. Reproducible builds & formatting gate**: work stays in the Nix dev shell; `nix fmt` / `nix flake check` green before merge; sqlx query cache regenerated and committed if any macro changes. ✅

No violations to justify (Complexity Tracking empty). One open design choice — persisting a Harvest source identifier for exact entry matching — is a Phase 0 decision, not a constitution deviation; either resolution stays within these principles.

## Project Structure

### Documentation (this feature)

```text
specs/004-harvest-importer/
├── plan.md              # This file
├── research.md          # Phase 0 — key decisions (natural keys, conversions, provenance, streaming)
├── data-model.md        # Phase 1 — import entities, target mapping, idempotency keys
├── quickstart.md        # Phase 1 — runnable validation guide (dry-run, import, re-run)
├── contracts/           # Phase 1 — interface contracts
│   ├── csv-format.md     # expected Harvest CSV columns → Horae fields
│   ├── importer-api.md   # server function + CLI subcommand contract
│   └── harvest-api.md    # reference: how the existing /harvest/v2 shape informs the mapping
└── tasks.md             # Phase 2 — created by /speckit-tasks (not here)
```

### Source Code (repository root)

```text
crates/
├── core/                # horae-core: pure domain
│   └── src/
│       └── harvest_import/   # NEW — pure conversions & matching:
│                             #   hours_to_minutes, money_to_cents (with round-trip guarantees),
│                             #   natural-key normalization (trim/casefold), row-outcome types
└── horae/
    ├── migrations/      # existing schema; a provenance/mapping migration added here ONLY if research.md adopts it
    ├── tests/           # #[sqlx::test] integration tests + small fixture CSVs
    └── src/
        ├── harvest_import.rs      # NEW [server] — importer module root (module = foo.rs + foo/ per conventions)
        ├── harvest_import/        # NEW [server] — submodules:
        │   ├── csv_source.rs      #   parse a Harvest CSV into normalized source rows (streaming)
        │   ├── resolve.rs         #   match source rows to existing rows by natural key
        │   ├── apply.rs           #   FK-safe insert/skip/update; per-row transactional writes
        │   └── report.rs          #   summary + per-row error report types
        ├── server_fns.rs          # + import_harvest_csv(...) #[server] fn (admin-only, dry_run mode)
        ├── cli.rs                 # + `import harvest <file> [--dry-run]` subcommand
        ├── harvest/               # existing read-only Harvest API — referenced, not modified
        └── pages/                 # + optional admin "Import from Harvest" upload screen (P1 delivery)
```

**Structure Decision**: Keep the two-crate split. Pure, correctness-critical conversions and natural-key normalization go in a new `crates/core/src/harvest_import/` so they are unit-tested without a DB (Constitution II). The I/O-bound engine — CSV parsing, DB resolution, FK-safe application, reporting — lives in a new server-only `crates/horae/src/harvest_import.rs` (+ `harvest_import/` submodules), following the repo's `foo.rs` + `foo/` module convention. The administrator invokes it through a new admin-only `#[server]` function (upload path) and/or a `cli.rs` subcommand; both call the same engine so behavior is identical regardless of surface. The existing `crates/horae/src/harvest/` module is read only as the authority on Harvest field semantics.

## Complexity Tracking

No constitution violations require justification. The only non-trivial design decision — whether to persist a Harvest source identifier as import provenance to make time-entry idempotency exact (versus relying on the composite natural key) — is deliberately deferred to research.md; both options stay within the constitution, and the composite key is a working default so the feature is not blocked on it.
