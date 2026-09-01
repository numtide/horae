# Phase 1 Data Model: Harvest Data Importer

Derived from the spec's Key Entities and `research.md`. The importer creates rows in Horae's existing schema (`crates/horae/migrations/0001_init.sql`) and adds **two new additive tables** in v1 — `harvest_credentials` (OAuth connection) and `harvest_import_map` (provenance). Neither alters existing columns. This document defines the source-side model, the mapping onto existing Horae columns, the two new tables, the idempotency keys, and the in-memory run/report structures. Everything is scoped to the single `organizations` row; every created row carries `org_id`. Primary keys are UUID v7. Time is stored as **integer minutes**, money as **integer minor units (cents) + ISO 4217 currency** — never floats.

## Source-side entities (transient, not persisted)

### SourceRow

The normalized record both source adapters produce; the engine is source-agnostic (research.md §1). A `SourceRow` carries the parent entity fields alongside the entry, **plus optional Harvest ids** used for provenance matching (present for the API source, absent for CSV).

- Harvest ids (API source only; `None` for CSV): `harvest_client_id`, `harvest_project_id`, `harvest_task_id`, `harvest_time_entry_id`, `harvest_user_id`.
- Client: `client_name`.
- Project: `project_name`, `project_code` (optional).
- Task: `task_name`.
- Person: `user_email` (API: resolved from the pulled Harvest users by `harvest_user_id`; CSV: the email column or first/last name).
- Entry: `spent_date`, `hours` (API: JSON number; CSV: decimal string), `notes` (optional), `billable` (flag), `invoiced` (flag, informational only).
- Money: `billable_rate`, `billable_amount`, `cost_rate`, `cost_amount` (optional), `currency` (ISO code).
- Provenance freshness (API only): `harvest_updated_at` — Harvest's `updated_at` for the record, stored on the mapping to drive `updated_since` incremental sync.

**How each adapter fills it**:

- **API source (primary)**: fetches each Harvest collection (`clients`, `projects`, `tasks` + `task_assignments`, `users`, `time_entries`) page by page; a time-entry object references its client/project/task/user by id, which the adapter joins against the already-fetched collections to assemble a `SourceRow` with all ids populated.
- **CSV source (secondary)**: parses each denormalized detailed-time-report row into a `SourceRow` with all Harvest ids `None`; see `contracts/csv-format.md`.

**Validation before resolution**: `spent_date` is a valid date; `hours` is a non-negative number/decimal; `currency` is a 3-letter code; required name fields are non-empty. A record failing validation becomes a `RowOutcome::Errored` with its source location (Harvest id or CSV line) and reason and does not proceed.

### ImportRun

A single execution against one source. Fields: `source` (`HarvestApi` | `Csv`), `mode` (`DryRun` | `Commit`), `actor_user_id` (the admin), `org_id`, source descriptor (Harvest account id, or filename/size). Produces an `ImportSummary` and a `Vec<RowOutcome>`. Conceptually the unit that must be idempotent when repeated.

## New persisted tables (additive migrations)

### `harvest_credentials` — the OAuth connection (FR-022, FR-024, FR-025)

One row per organization (v1 supports a single connected Harvest account). Added by its own migration; does not touch existing tables.

| Column | Type | Notes |
|---|---|---|
| `id` | uuid | UUID v7, PK |
| `org_id` | uuid | FK `organizations(id)`; **unique** (one connection per org in v1) |
| `harvest_account_id` | text | the Harvest account the tokens authorize |
| `access_token_enc` | bytea | access token, **encrypted at rest** (AEAD, deployment-supplied key) |
| `refresh_token_enc` | bytea | refresh token, **encrypted at rest** |
| `token_expires_at` | timestamptz | when the access token expires; drives transparent refresh |
| `scope` | text | granted scope, informational |
| `synced_watermark` | jsonb | per-entity `updated_since` high-water marks for incremental re-sync (FR-025) |
| `created_at` | timestamptz | default `now()` |
| `updated_at` | timestamptz | bumped on token refresh / successful sync |

- Tokens are never returned to the browser or logged (FR-022). Decryption happens only server-side when calling Harvest.
- The `synced_watermark` is updated **only on a successful committing run**, never in a dry-run (FR-014).

### `harvest_import_map` — provenance (FR-012, FR-026)

The exact, edit-robust match key for API-sourced records; looked up ahead of the composite natural key. Added by its own migration; does not touch existing tables.

| Column | Type | Notes |
|---|---|---|
| `org_id` | uuid | FK `organizations(id)` |
| `harvest_entity_type` | enum/text | `client` | `project` | `task` | `time_entry` |
| `harvest_id` | bigint | the Harvest record id |
| `horae_id` | uuid | the Horae record it maps to (client/project/task/time_entry id) |
| `harvest_updated_at` | timestamptz | Harvest `updated_at` seen at last sync (informational / re-sync aid) |
| `created_at` | timestamptz | default `now()` |
| **PK** | | `(org_id, harvest_entity_type, harvest_id)` |

- Written **only on commit**, as part of the same all-or-nothing unit that creates/updates the record (FR-020, FR-026); never in a dry-run.
- Looked up first during resolution; a hit means the Harvest record already maps to a Horae record, so the incoming record is matched exactly regardless of any edit to the natural-key fields on either side.
- Not populated for the CSV source (no Harvest ids); those runs rely solely on the natural key.

## Target mapping (source → existing Horae columns)

### Client → `clients`

| Horae column | Source | Notes |
|---|---|---|
| `id` | generated | UUID v7 |
| `org_id` | run | single org |
| `name` | `client_name` | trimmed |
| `currency` | `currency` | ISO 4217; falls back per FR-013 precedence |
| `address` | Harvest client `address` (API) / supplementary CSV | else NULL |
| `active` | Harvest `is_active` (API) | default `true` |

### Project → `projects`

| Horae column | Source | Notes |
|---|---|---|
| `id` | generated | UUID v7 |
| `org_id` / `client_id` | run / resolved client | FK-safe: client created/resolved first |
| `code` | `project_code` | optional |
| `name` | `project_name` | trimmed |
| `project_type` | Harvest `bill_by`/billing flags (API) | default `time_and_materials` |
| `currency` | client currency | project currency defaults from client |
| `starts_on` / `ends_on` | Harvest project dates (API) / supplementary CSV | else NULL |
| `budget_kind` | Harvest `budget_by` (API) | default `none` |
| `active` | Harvest `is_active` (API) | default `true` |

### Task → `tasks` + `project_tasks`

Horae keeps an **org-level task catalog** (`tasks`) with **per-project enablement** (`project_tasks`). The importer:

1. Resolves/creates one `tasks` row per distinct Harvest task (matched by provenance for the API, by `task_name` for CSV) — `billable_default` from the Harvest task's `billable_by_default` (API) or the row's billable flag (CSV) on first sight; `default_rate_cents` from the Harvest `default_hourly_rate` / `billable_rate` when present.
1. Ensures a `project_tasks` link `(project_id, task_id)` exists for every project a task's entries reference (from Harvest `task_assignments` for the API, or derived from rows for CSV), carrying `billable` and optional `rate_cents`. This satisfies FR-009.

### Time Entry → `time_entries`

| Horae column | Source | Notes |
|---|---|---|
| `id` | generated | UUID v7 |
| `org_id` / `user_id` / `project_id` / `task_id` | run / resolved | user matched by email (FR-010); parents resolved first |
| `spent_date` | `spent_date` | parsed date |
| `minutes` | `round(hours * 60)` | exact integer minutes via `horae-core` (FR-005) |
| `rounded_minutes` | — | left NULL (persisted at lock, not at import) |
| `notes` | `notes` | optional |
| `billable` | `billable` | flag |
| `is_running` | — | always `false` (imported entries are historical) |
| `started_at` | — | NULL (no live timer) |
| `state` | — | `open` — never `invoiced` from a Harvest billed flag (FR-016) |
| `invoice_id` | — | NULL — not coupled to any Horae invoice (FR-016) |

For an API-sourced entry, a `harvest_import_map` row `(org, 'time_entry', harvest_time_entry_id) → time_entries.id` is written on commit (FR-026). Money on the entry is converted to cents for validation/reconciliation; Harvest's per-entry billed/invoiced fact is informational only.

## Idempotency: provenance first, natural key fallback (org-scoped)

Resolution order (FR-012): **provenance lookup**, then **composite natural key**. String comparisons in the natural key are trimmed and case-folded (normalization lives in `horae-core`).

| Entity | Provenance key (API) | Natural-key fallback (CSV + first import) |
|---|---|---|
| Client | `(org, 'client', harvest_client_id)` | `name` |
| Project | `(org, 'project', harvest_project_id)` | `code` if present, else `(client, name)` |
| Task | `(org, 'task', harvest_task_id)` | `name` |
| Time entry | `(org, 'time_entry', harvest_time_entry_id)` | `(user, project, task, spent_date, minutes, notes)` |

- **API source**: provenance is authoritative — a hit matches exactly even after the record was edited in Horae or Harvest (FR-026, SC-002). On first import a record has no provenance yet, so it resolves by natural key (avoiding a duplicate when a matching row already exists), then a provenance row is written on commit.
- **CSV source**: no Harvest ids, so only the natural key applies (US5).

## Run/report structures (in-memory)

### RowOutcome

Per source record: one of `Created`, `Updated`, `Skipped` (matched, unchanged), or `Errored { source_location, reason }`, where `source_location` is a Harvest record id (API) or a CSV line number (CSV). The raw material of both the summary and the error report (FR-019).

### ImportSummary

Per entity type (clients, projects, tasks, time entries): counts `created`, `updated`, `skipped`, `errored`. Invariant: `processed = created + updated + skipped + errored` per type (FR-021, SC-005). In `DryRun` mode the same counts are reported as would-create/update/skip/error and nothing is written — no data, no `harvest_import_map` rows, no `harvest_credentials.synced_watermark` update (FR-014).

## Ordering & transactional rules

- **FK-safe order** per run: resolve/create client → project → task (+ `project_tasks` link) → time entry (FR-004). An in-run cache resolves each distinct parent once (research.md §9).
- **Per-record atomicity**: each source record's writes — including its `harvest_import_map` provenance row — apply as an all-or-nothing unit (savepoint/transaction) so a mid-record failure leaves no partial fragment and no dangling mapping (FR-020).
- **Dry-run**: full resolve/plan against live data (including provenance lookups), zero writes — via a rolled-back transaction or a plan-only path (research.md §7).
- **Watermark update**: `harvest_credentials.synced_watermark` advances only after a successful committing API run, enabling the next incremental `updated_since` pull (FR-025, SC-008).

## Cross-cutting validation

- Durations and money converted and stored as exact integers; a full import reconciles to zero drift against the Harvest source (FR-005/FR-006, SC-003/SC-007) — asserted in `horae-core` unit tests and integration reconciliation tests.
- Currency conflicts across levels resolve by a single defined precedence and are recorded as a fallback rather than failing the record (FR-013).
- Unmatched user → record errored, run continues (FR-010/FR-018); the importer never writes to `users`.
- OAuth tokens in `harvest_credentials` are encrypted at rest and never surfaced to the browser or logs (FR-022); an expired token is refreshed transparently, and a failed refresh rejects the run up front (FR-024).
