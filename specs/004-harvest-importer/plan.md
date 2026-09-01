# Implementation Plan: Harvest Data Importer

**Branch**: `004-harvest-importer` | **Date**: 2026-09-01 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/004-harvest-importer/spec.md`

## Summary

Let an organization administrator migrate existing Harvest data — clients, projects, tasks, and time entries — into Horae so a team switching from Harvest does not start on an empty install. The **primary source is a live pull from Harvest's REST API over OAuth2**: the admin connects their Harvest account (authorization-code flow), and the importer fetches each entity collection (clients → projects → tasks/assignments → users-for-reference → time entries) and feeds it through a **source-agnostic engine**. A **Harvest CSV export is a secondary, offline adapter** feeding the identical engine; it derives the four levels from denormalized rows. Both adapters normalize into one `SourceRow` stream — the engine never knows the source. All writes go through a single authenticated, role-checked server-side path into PostgreSQL, in FK-safe order.

Matching is **provenance-first**: a new persisted table maps `(org, Harvest entity type, Harvest id) → Horae id`, written on commit for API-sourced records and looked up ahead of a composite natural key. This gives exact, edit-robust idempotency and powers **incremental re-sync** (`updated_since` watermark). The composite natural key remains the fallback and the only matcher for the id-less CSV source. Correctness-critical conversions live in `horae-core`: Harvest decimal **hours → exact integer minutes** and decimal money **→ integer minor units (cents) + ISO currency**, the exact inverse of what `crates/horae/src/harvest/` already does when it emits the Harvest-compatible API. The importer is **idempotent**, offers a **dry-run** that reports would-create/update/skip/error without writing (no data, no provenance, no watermark), and is **resilient** (a bad record is skipped and reported, never aborting the run) with a reconciling summary.

Two additive migrations are in scope: `harvest_credentials` (encrypted OAuth tokens + Harvest account id + re-sync watermark) and `harvest_import_map` (the provenance table). Neither alters existing columns.

## Technical Context

**Language/Version**: Rust (edition 2024); the app is Dioxus fullstack (server + web targets). The importer's engine is server-side and pure-domain where it counts.

**Primary Dependencies**: `csv` (already a workspace dependency, used by `reports.rs`) for the secondary source; an async HTTPS client (`reqwest`, already transitively present via the OIDC/`openidconnect` stack — confirm and reuse rather than adding) for the Harvest API pull; `serde`/`serde_json` for Harvest JSON; `sqlx` 0.8 (Postgres) for FK-safe writes and the two new tables; `uuid` v7 for keys; `chrono` for dates and token expiry. OAuth2 authorization-code exchange reuses the crate already backing OIDC (`openidconnect`/`oauth2`) where practical; token encryption at rest uses an established AEAD (e.g. the `aes-gcm`/`chacha20poly1305` family) keyed from config — pick one small, audited crate per `ponytail`. Conversions reuse `horae-core`. Delivery via the existing `clap` CLI and/or Dioxus `#[server]` functions + an admin screen, plus one plain Axum route for the OAuth callback.

**Storage**: PostgreSQL 15+ via `sqlx`, existing schema in `crates/horae/migrations/`. **Two new additive migrations** in scope (neither alters existing columns): `harvest_credentials` (encrypted access/refresh tokens, Harvest account id, per-entity `updated_since` watermark) and `harvest_import_map` (provenance: `(org_id, harvest_entity_type, harvest_id) → horae_id`). Detailed in data-model.md.

**Testing**: `cargo test -p horae-core` for the pure conversion/matching helpers (hours→minutes, money→cents, natural-key normalization). `cargo test -p horae --features server` with `#[sqlx::test]` (throwaway DB, `#[serial]`) for FK-safe insertion, provenance-based idempotent re-runs (including edited-after-import), dry-run-writes-nothing (rows, provenance, and watermark all unchanged), incremental-watermark behavior, and partial-failure reconciliation. The Harvest API adapter is tested against a **recorded/stubbed HTTP layer** (fixture JSON pages exercising pagination, a 429 backoff, and a token refresh) so tests need no live Harvest account; fixture CSVs drive the secondary-source tests.

**Target Platform**: Linux server (self-hosted); administrator connects and imports via an admin screen in the WASM SPA and/or the CLI on the host. Outbound HTTPS to Harvest is required for the primary source.

**Project Type**: Web application — Dioxus fullstack app plus the pure-domain `horae-core` crate; this feature adds a server-side importer module (two source adapters + shared engine), an OAuth connect flow, two migrations, and a delivery surface.

**Performance Goals**: Import a dataset of ≥100,000 time-entry records to completion without exhausting memory (SC-006) — stream API pages / iterate CSV rows and batch writes rather than materializing everything at once; page through Harvest and respect its rate limit. No interactive latency target; this is a batch operation.

**Constraints**: Exactness is non-negotiable — durations as integer minutes, money as integer minor units + ISO currency, never floats (Constitution I). UUID v7 keys; Postgres-only; every created row carries `org_id`. `horae-core` stays free of `sqlx`/`axum`/`dioxus`/HTTP. All domain writes go through the existing authorized server-side path (Constitution IV); the OAuth callback is a read/exchange route that persists only the connection credentials, not domain data. OAuth tokens are encrypted at rest and never surface to the browser or logs. Idempotency, per-record resilience, pagination, rate-limit backoff, and token refresh are hard requirements (FR-011, FR-018, FR-022–FR-026).

**Scale/Scope**: Single organization, one Harvest connection. Four entity levels (clients, projects, tasks, time entries) plus the org-level task catalog's per-project enablement. One import engine, two source adapters (API primary, CSV secondary). Provenance + incremental re-sync in scope; scheduled/automatic re-sync and multi-account are deferred.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

Checked against the ratified project constitution (`.specify/memory/constitution.md`, **v1.0.0**):

- **I. Exactness (non-negotiable)**: Harvest decimal hours/money are converted to integer minutes and integer minor units + ISO currency in `horae-core`, with round-trip reconciliation asserted in tests (FR-005, FR-006, SC-003). No float is stored. ✅
- **II. Domain purity**: hours→minutes, money→cents, and natural-key normalization (trim/case-fold) live in `horae-core` with no I/O deps and are unit-tested in isolation. HTTP, OAuth, and token encryption stay entirely in the server crate, out of `horae-core`. ✅
- **III. Single datastore**: PostgreSQL only; created rows use UUID v7 and carry `org_id`; the two new tables are additive and `org_id`-scoped; FK-safe insertion order matches the existing schema's foreign keys. ✅
- **IV. Mutations through server functions**: the import writes through the existing session-authenticated, role-checked server-side path (a `#[server]` function and/or the server-binary CLI sharing the same domain/DB layer). The one new plain Axum route is the **OAuth callback** — browsers must redirect to it, so it cannot be a `#[server]` fn; it performs the token exchange and persists only the Harvest connection credentials, not domain-table mutations, and sits beside the existing `auth::router()`. The read-only Harvest-compatible export API stays read-only and is referenced only for field-shape semantics. No second domain mutation path. ✅
- **V. Reproducible builds & formatting gate**: work stays in the Nix dev shell; `nix fmt` / `nix flake check` green before merge; sqlx query cache regenerated and committed for the new tables' queries. Any new dependency (HTTP client if not already transitive, AEAD crate) is added deliberately and pinned; prefer reusing what the OIDC stack already pulls in. ✅

No violations to justify (Complexity Tracking empty). The OAuth callback route is the only non-`#[server]` surface added; it is justified above (redirect target) and confined to credential exchange, consistent with the existing `auth/` router precedent.

## Project Structure

### Documentation (this feature)

```text
specs/004-harvest-importer/
├── plan.md              # This file
├── research.md          # Phase 0 — key decisions (source priority, OAuth2, provenance, conversions, rate limits, incremental sync)
├── data-model.md        # Phase 1 — source + target model, provenance & credentials tables, idempotency keys
├── quickstart.md        # Phase 1 — runnable validation guide (connect via OAuth, import, re-sync, CSV fallback)
├── contracts/           # Phase 1 — interface contracts
│   ├── harvest-api.md    # the real Harvest REST API consumed (OAuth2, headers, pagination, rate limit) + inverse-of-exporter mapping
│   ├── importer-api.md   # server functions (connect + import_harvest_api/import_harvest_csv) + CLI contract
│   └── csv-format.md     # secondary adapter: expected Harvest CSV columns → Horae fields
└── tasks.md             # Phase 2 — created by /speckit-tasks (not here)
```

### Source Code (repository root)

```text
crates/
├── core/                # horae-core: pure domain
│   └── src/
│       └── harvest_import/   # NEW — pure conversions & matching:
│                             #   hours_to_minutes, money_to_cents (with round-trip guarantees),
│                             #   natural-key normalization (trim/casefold), source-row + row-outcome types
└── horae/
    ├── migrations/      # + 000N_harvest_credentials.sql  (encrypted tokens + account id + watermark)
    │                    # + 000N_harvest_import_map.sql    (provenance: org+entity+harvest_id → horae_id)
    ├── tests/           # #[sqlx::test] integration tests, stubbed-HTTP API fixtures + small fixture CSVs
    └── src/
        ├── harvest_import.rs      # NEW [server] — importer module root (foo.rs + foo/ per conventions)
        ├── harvest_import/        # NEW [server] — submodules:
        │   ├── api_source.rs      #   PRIMARY: pull Harvest REST API → normalized SourceRow stream
        │   │                      #     (pagination, rate-limit backoff, updated_since incremental)
        │   ├── csv_source.rs      #   SECONDARY: parse a Harvest CSV → the same SourceRow stream (streaming)
        │   ├── oauth.rs           #   OAuth2 authorization-code flow: start + callback token exchange
        │   ├── credentials.rs     #   load/store encrypted tokens + account id + watermark (harvest_credentials)
        │   ├── provenance.rs      #   lookup/persist harvest_import_map; provenance-first matching
        │   ├── resolve.rs         #   match source rows to existing rows (provenance → natural key)
        │   ├── apply.rs           #   FK-safe insert/skip/update; per-record transactional writes (+ provenance row)
        │   └── report.rs          #   summary + per-record error report types
        ├── server_fns.rs          # + connect-start + import_harvest_api(mode) + import_harvest_csv(file, mode) #[server] fns (admin-only)
        ├── cli.rs                 # + `import harvest-api [--dry-run]` and `import harvest-csv <file> [--dry-run]`
        ├── config.rs              # + Harvest OAuth client id/secret, redirect URL, token-encryption key
        ├── auth/                  # OAuth callback route registered beside auth::router() (browser redirect target)
        ├── harvest/               # existing read-only Harvest EXPORTER — referenced as inverse, not modified
        └── pages/                 # + admin "Import from Harvest" screen (connect + run + report)
```

**Structure Decision**: Keep the two-crate split. Pure, correctness-critical conversions and natural-key normalization go in `crates/core/src/harvest_import/` so they are unit-tested without a DB or network (Constitution II). The I/O-bound engine — the two source adapters, OAuth, credential storage, DB resolution (provenance-first), FK-safe application, reporting — lives in a new server-only `crates/horae/src/harvest_import.rs` (+ `harvest_import/` submodules), following the repo's `foo.rs` + `foo/` module convention. The **source adapter seam** is explicit: `api_source.rs` and `csv_source.rs` both yield the same `SourceRow` stream, and `resolve.rs`/`apply.rs`/`report.rs` are source-agnostic. The administrator connects and invokes through admin-only `#[server]` functions (+ the plain OAuth callback route) and/or `cli.rs` subcommands; all call the same engine so behavior is identical regardless of surface or source. The existing `crates/horae/src/harvest/` module (the Horae→Harvest exporter) is read only as the authority on Harvest field semantics and inverted.

## Complexity Tracking

No constitution violations require justification. Two design elements deserve a note:

- **Provenance table (now in scope)** — reversing the earlier "defer" posture is justified in research.md §5: the API source hands us stable Harvest ids for free, so provenance is a small additive table that removes an entire class of matching bugs (edit-after-import) and enables incremental re-sync. It stays within the constitution (additive, `org_id`-scoped, UUID-linked).
- **OAuth callback as a plain Axum route** — the single non-`#[server]` surface added, unavoidable because browsers redirect to it; scoped to credential exchange only and placed beside the existing `auth::router()`, so it does not become a second domain-mutation path (Constitution IV). Justified in the Constitution Check.
