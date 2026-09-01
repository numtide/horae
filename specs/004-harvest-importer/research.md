# Phase 0 Research: Harvest Data Importer

Decisions resolving the open questions in the Technical Context. Each is stated as Decision / Rationale / Alternatives considered.

## 1. Source priority: Harvest REST API (OAuth2) primary, CSV secondary — one source-agnostic engine

- **Decision**: The v1 primary source is a **live pull from Harvest's REST API over OAuth2**. A Harvest **CSV export** is a secondary, offline adapter. Both are **source adapters** that normalize into the same internal `SourceRow` stream feeding a single mapping/matching/apply engine; the engine never knows which adapter produced a record. The API adapter fetches each Harvest entity level as its own paginated collection (`clients`, `projects`, `tasks` + task assignments, `users` for reference, `time_entries`); the CSV adapter parses the denormalized detailed-time-report and derives the four levels from each row.
- **Rationale**: The API gives every Harvest record a **stable id**, which the CSV cannot; those ids make matching exact and edit-robust (Decision 5) and enable incremental re-sync (Decision 11) — the properties a real cut-over migration needs. It also removes the export-a-file step for the common case. Keeping the engine source-agnostic means the CSV path costs only an adapter, and future sources (or Harvest API changes) do not ripple into the mapping/matching rules.
- **Alternatives considered**: CSV-only for v1 (no stable ids → matching limited to a composite natural key, no reliable incremental sync, and an extra manual export step); API-only (drops the fully-offline migration path some operators need). The seam is explicit: `csv_source.rs` and `api_source.rs` both yield the same `SourceRow`.

## 2. Field mapping authority: invert the existing `/harvest/v2` shape

- **Decision**: Treat `crates/horae/src/harvest/` (Horae's own read-only Harvest-**compatible** API) as the authoritative reference for what each Harvest field means and how it corresponds to Horae columns. The importer performs the **inverse** of the transforms that module already applies when emitting Harvest JSON. Note the direction carefully: that module maps Horae → Harvest (an exporter); this feature consumes the **real** Harvest API and maps Harvest → Horae. They are inverses of each other, not the same surface.
- **Rationale**: That module already encodes the mapping: `hours = minutes / 60`, `rate = cents / 100`, project `bill_by`/`budget_by`, task `billable_by_default`/`default_hourly_rate`, client `currency`/`address`. Inverting the same, already-reviewed mapping keeps import and export symmetric and avoids re-deriving field semantics.
- **Alternatives considered**: Re-deriving the mapping from Harvest's public docs independently (risks drift from the shape the codebase already commits to). Where the real Harvest API exposes a field the local exporter omits, the public Harvest docs are the tie-breaker.

## 3. Exact conversions: hours → integer minutes, money → integer cents

- **Decision**: Convert decimal hours to whole minutes as `round(hours * 60)` (round half up) and decimal money to minor units as `round(amount * 100)` using integer/decimal-safe arithmetic, both implemented in `horae-core`. This is source-independent: the API returns `hours`/`rate` as JSON numbers and the CSV as decimal strings, but both are converted by the same tested helpers. Tests assert round-trip reconciliation against the existing `minutes/60` and `cents/100` export transforms and that a full import reconciles to zero drift (SC-003, SC-007).
- **Rationale**: Constitution I forbids float storage; a single, centralized, tested rounding rule makes conversions deterministic and re-import-stable. Parsing carefully (avoiding `f64` accumulation near the half-minute/half-cent boundary) avoids binary-float representation error.
- **Alternatives considered**: Storing raw hours (violates Constitution I); truncating instead of rounding (loses up to a minute per entry, breaks reconciliation); per-call ad-hoc rounding (drift risk, untestable in isolation).

## 4. Matching keys: provenance first, composite natural key as fallback

- **Decision**: Match an incoming record to an existing Horae record in this order:
  1. **Provenance** — a stored `(org, harvest_entity_type, harvest_id) → horae_id` mapping (Decision 5). Exact and edit-robust; used for every API-sourced record.
  1. **Composite natural key** — the fallback, and the only key available for the CSV source (no ids). Org-scoped, all string comparisons trimmed and case-folded:
     - **Client**: `name`.
     - **Project**: `code` when present; else `(client, name)`.
     - **Task**: `name` (org-level catalog; shared across projects, enabled per project via `project_tasks`).
     - **Time entry**: composite `(user, project, task, spent_date, minutes, notes)`.
- **Rationale**: Provenance makes API re-syncs correct even after a record is edited on either side, which a pure natural key cannot (a changed note would look like a new entry). The composite natural key remains a working matcher for the id-less CSV source and as a first-import fallback before provenance exists. Case-fold+trim absorbs incidental whitespace/casing differences.
- **Alternatives considered**: Natural key only (breaks on edit-after-import; the original spec's known weakness); provenance only (leaves the CSV source with nothing to match on).

## 5. Provenance/mapping table — IN SCOPE for v1

- **Decision**: Add a new persisted table mapping `(org_id, harvest_entity_type, harvest_id) → horae_id` (plus the Harvest `updated_at` seen at last sync), by its **own migration**. It does **not** alter existing columns. It is written **only on commit** for API-sourced records (never in a dry-run), as part of each record's all-or-nothing write, and is looked up ahead of the composite natural key (Decision 4). Specified in data-model.md.
- **Rationale**: Now that the primary source is the API — which hands us stable Harvest ids for free — provenance is cheap and high-value: it is what makes re-sync exact and edit-robust (FR-026, SC-002) and what powers incremental sync (Decision 11). This reverses the earlier "defer provenance" posture, which only made sense while CSV (no ids) was the sole source. It stays within the constitution (a new additive table, UUID-linked, `org_id`-scoped) and honors `ponytail` because it removes a whole class of matching bugs rather than adding speculative surface.
- **Alternatives considered**: Adding an `external_ref` column to each existing table (alters existing schema, spreads Harvest-specific concerns across core tables); a generic import-audit log only (does not help matching); deferring provenance (the earlier plan) — rejected now that the API makes ids available and re-sync a first-class story.

## 6. User resolution: match by email, never provision

- **Decision**: Resolve each time-entry record's person to an existing Horae user by **email**. For the API source, Harvest `users` are pulled as reference data (id → email) so an entry's `user_id` resolves to an email and then to a Horae user; for CSV, the person's email column (or name fallback) is used. If no user matches, the record is a per-record **error** and the run continues; the importer never creates user accounts and never writes Harvest users into `users`.
- **Rationale**: User provisioning already has an owner (OIDC / admin `user create`) with its own authorization and identity rules; duplicating it in the importer would risk shadow accounts and bypass that path (Constitution IV). Erroring unmatched records surfaces the gap so the admin provisions the user and re-runs just those.
- **Alternatives considered**: Auto-creating placeholder users (pollutes the user table, breaks sign-in/identity assumptions); attaching orphan entries to the importing admin (silently misattributes time).

## 7. Dry-run: same engine, no writes (no provenance, no watermark)

- **Decision**: Dry-run runs the full pull/parse → resolve → plan pipeline and produces the identical per-entity would-create/update/skip/error counts and per-record error report, but the apply stage performs no writes — **including no provenance rows and no re-sync watermark update** (implemented by running the same code inside a transaction that is rolled back, or by a plan-only path that never enters the write stage). A committing run on the same unchanged input/data reproduces the preview (FR-015, SC-004).
- **Rationale**: Sharing one engine guarantees the preview matches reality. Rollback-in-transaction is the simplest way to exercise real resolution/matching (including provenance lookups) against live data while guaranteeing nothing persists. A dry-run against the API still performs read-only GETs to Harvest — reads, not writes — which is consistent with "writes nothing to storage".
- **Alternatives considered**: A separate estimation path (drifts from the real importer); dry-run that only validates parsing (misses conflicts against existing data and provenance).

## 8. Resilience and reconciliation: per-record transactional application

- **Decision**: Apply each source record's writes as an **all-or-nothing** unit (a per-record transaction / savepoint), and include that record's **provenance row** in the same unit so a mid-record failure leaves no partial fragment and no dangling mapping (FR-020). A failing record is recorded with its source location (Harvest id or CSV line) and reason and the run continues (FR-018). The summary reports created/updated/skipped/errored per entity type and MUST reconcile: `processed = created + updated + skipped + errored` (FR-021).
- **Rationale**: Migration-scale datasets always contain some bad records; aborting the whole import on one is unusable. Savepoint-per-record keeps the datastore consistent while preserving partial success, and binding provenance into the same unit keeps the mapping table consistent with the data.
- **Alternatives considered**: One transaction for the whole run (one bad record rolls back everything); no per-record boundary (risk of orphaned parent, half-written entry, or a provenance row pointing at a rolled-back record).

## 9. Scale: stream records, batch writes, dedup parents in-run

- **Decision**: For the API source, consume Harvest **pages as a stream** (fetch a page, process it, fetch the next) rather than accumulating the whole dataset; for CSV, parse as a streaming record iterator (the `csv` crate's reader). Cache resolved parent entities (client/project/task) in an in-run map so a client seen on 50,000 records is resolved/created once. Writes are batched in bounded chunks.
- **Rationale**: A ≥100k-record dataset must import without exhausting memory; streaming pages + an in-run parent cache keeps memory bounded and avoids re-querying the same parent per record.
- **Alternatives considered**: Fetching all pages then grouping (peak memory scales with dataset size); resolving each parent per record against the DB (redundant queries, slow).

## 10. OAuth2 connection & credential storage

- **Decision**: Connect via Harvest's **OAuth2 authorization-code flow**: an admin-only "Connect Harvest" action redirects to Harvest's authorization endpoint; Harvest redirects back to a Horae callback with a code; Horae exchanges it for an access + refresh token and resolves the **Harvest account id**, then stores all three **encrypted at rest** in a new `harvest_credentials` table (its own migration, one connection per org for v1). Data-API calls send `Authorization: Bearer <token>`, `Harvest-Account-Id: <id>`, and a `User-Agent`. Encryption uses a deployment-supplied key (config, alongside existing secrets). Tokens are never sent to the browser or logged. The OAuth client id/secret come from configuration.
- **Rationale**: OAuth2 is Harvest's supported way for a third-party app to read another account's data; storing the refresh token lets Horae re-sync later without re-authorizing. Encryption at rest keeps a DB dump from leaking live Harvest access. Reusing Horae's existing config/secrets plumbing (the OIDC secrets already live there) avoids a new secret-management surface.
- **Alternatives considered**: Personal access tokens (simpler but per-user, not the "connect your account" flow, and long-lived static secrets); storing tokens in plaintext (a DB compromise leaks Harvest access); a browser-side token flow (would expose the token to the SPA, violating "never surfaced to the browser").

## 11. Rate limits, pagination, and incremental re-sync

- **Decision**: The API adapter (a) **paginates** through every collection to completion using Harvest's paging (page/`next` links, bounded `per_page`); (b) **respects the rate limit** — on an HTTP 429 it waits per the response's retry-after guidance and retries, and otherwise paces requests to stay under Harvest's published ceiling — rather than failing the run; (c) **refreshes** an expired access token transparently mid-run using the stored refresh token, and rejects the run up front with a "reconnect Harvest" message if refresh fails; (d) supports **incremental re-sync** by persisting a per-entity `updated_since` watermark on each successful committing run and sending it on the next run so only changed records are fetched, while a full re-sync remains available.
- **Rationale**: A real account exceeds one page and will trip the rate limit on a large first pull; handling both is required for SC-006. Transparent refresh keeps long runs from dying on token expiry (FR-024). The watermark makes cut-over re-syncs cheap and is the API-source counterpart to idempotency (FR-025, SC-008); the provenance table (Decision 5) is what makes those incremental updates land on the right existing rows.
- **Alternatives considered**: Fixed sleeps between calls (wastes time or still trips limits); no incremental sync (every re-sync re-pulls everything — slow, and pointless once provenance exists); failing on 429 (unusable at migration scale).

## 12. Delivery surface: admin server function + connect flow, and/or CLI, one shared engine

- **Decision**: Expose the importer through admin-only Dioxus `#[server]` functions backing an "Import from Harvest" screen — a **connect step** (start OAuth / handle callback) plus `import_harvest_api(mode)` for the API pull and `import_harvest_csv(file, mode)` for the CSV upload — and/or a server-binary CLI (`import harvest-api [--dry-run]`, `import harvest-csv <file> [--dry-run]`). Both surfaces call the same engine. A two-state `mode` is a plainly named enum (`ImportMode { DryRun, Commit }`), never `Option<bool>`, per the repo convention. At least one administrator-invocable surface ships in v1.
- **Rationale**: The `#[server]` path fits the SPA and Constitution IV's single authorized mutation path; the OAuth callback is a small server route alongside the existing auth router. The CLI fits an operator doing a one-shot host-side migration. Sharing the engine keeps behavior identical across surfaces and across sources.
- **Alternatives considered**: UI-only (awkward for very large host-side pulls); CLI-only (misses the in-app connect + migration flow). The OAuth **callback** must be a plain server route (browsers redirect to it), not a `#[server]` function — the only non-`#[server]` addition, and it performs the token exchange, not a data mutation of domain tables.

## Resolved unknowns

All Technical Context items are resolved: source priority and adapter seam (Decision 1), mapping authority (2), exact conversions (3), matching keys (4), provenance now in scope (5), user resolution (6), dry-run mechanism (7), resilience/reconciliation (8), scale strategy (9), OAuth2 + credential storage (10), rate limits/pagination/incremental sync (11), and delivery surface (12). No `NEEDS CLARIFICATION` remains.
