# Phase 0 Research: Harvest Data Importer

Decisions resolving the open questions in the Technical Context. Each is stated as Decision / Rationale / Alternatives considered.

## 1. Source format for v1: Harvest detailed time-report CSV

- **Decision**: The v1 input is Harvest's **detailed time-report CSV**. Its rows are denormalized — each time-entry row carries the client name, project name, project code, task name, person, notes, decimal hours, billable/invoiced flags, and rate/amount/currency columns. All four Horae entity levels (client, project, task, time entry) are derived from this single file. Optional dedicated Harvest **clients** and **projects** CSV exports MAY be accepted later as supplementary enrichment (client address, project budget/dates) but are not required for v1.
- **Rationale**: One file an administrator can produce in Harvest with no OAuth gives the complete client→project→task→entry graph. Deriving the parent entities from the denormalized rows avoids requiring three separate exports for the common case. The exact column list is pinned in `contracts/csv-format.md`.
- **Alternatives considered**: Requiring separate per-entity CSVs (more setup burden, more files to keep consistent); requiring the OAuth API immediately (out of scope, needs credentials, larger surface).

## 2. Field mapping authority: reuse the existing `/harvest/v2` shape

- **Decision**: Treat `crates/horae/src/harvest/` (the read-only Harvest-compatible API) as the authoritative reference for what each Harvest field means and how it corresponds to Horae columns. The importer performs the **inverse** of the transforms that module already applies when emitting Harvest JSON.
- **Rationale**: That module already encodes the mapping: `hours = minutes / 60`, `rate = cents / 100`, project `bill_by`/`budget_by`, task `billable_by_default`/`default_hourly_rate`, client `currency`/`address`. Inverting the same, already-reviewed mapping keeps import and export symmetric and avoids re-deriving field semantics.
- **Alternatives considered**: Re-deriving the mapping from Harvest's public docs independently (risks drift from the shape the codebase already commits to).

## 3. Exact conversions: hours → integer minutes, money → integer cents

- **Decision**: Convert decimal hours to whole minutes as `round(hours * 60)` (round half up) and decimal money to minor units as `round(amount * 100)` using integer/decimal-safe arithmetic, both implemented in `horae-core`. Tests assert round-trip reconciliation against the existing `minutes/60` and `cents/100` export transforms and that a full-file import reconciles to zero drift (SC-003, SC-007). Any hours value that does not land on a whole minute is adjusted by this single rule and the adjustment is countable in the summary.
- **Rationale**: Constitution I forbids float storage; a single, centralized, tested rounding rule makes conversions deterministic and re-import-stable (the same input always yields the same minutes/cents). Parsing the decimal string carefully (rather than through `f64`) avoids binary-float representation error near the half-minute/half-cent boundary.
- **Alternatives considered**: Storing raw hours (violates Constitution I); truncating instead of rounding (loses up to a minute per entry, breaks reconciliation); per-call ad-hoc rounding (drift risk, untestable in isolation).

## 4. Natural keys for idempotency

- **Decision**: Match incoming rows to existing rows using these org-scoped natural keys (all string comparisons trimmed and case-folded):
  - **Client**: `name`.
  - **Project**: `code` when present; else `(client, name)`.
  - **Task**: `name` (Horae's task catalog is org-level, so one task record is shared across projects and enabled per project via `project_tasks`).
  - **Time entry**: composite `(user, project, task, spent_date, minutes, notes)`. This keeps two genuinely distinct entries on the same day while recognizing an exact re-import as the same row.
- **Rationale**: These are the fields a Harvest export reliably carries and that uniquely identify each entity at Horae's grain. Case-fold+trim absorbs incidental export whitespace/casing differences so a re-run matches. The composite entry key needs no schema change and satisfies FR-011 for the common case.
- **Alternatives considered**: Matching clients/projects by a surrogate export id (the time-report CSV does not reliably expose stable ids); matching entries by notes alone (not unique); ignoring notes in the entry key (would collapse two legitimately different same-day/same-duration entries into one).

## 5. Time-entry provenance (exact matching) — deferred, composite key is the default

- **Decision**: Ship v1 with the **composite natural key** (Decision 4) and no schema change. Record the recommendation to persist a **Harvest source identifier** (a provenance/mapping record keyed by org + Harvest entry id) as a follow-up that would make entry matching exact and robust against later edits (e.g. a note changed in Horae after import). If adopted, it is a new nullable-linked mapping table specified in data-model.md — it does not alter existing columns.
- **Rationale**: The composite key meets the idempotency requirement for the realistic re-import case (same file, unedited rows) without touching the schema, honoring `ponytail` (smallest thing that works). Provenance is strictly better for edited-after-import cases but costs a migration and only matters once entries diverge from their source; deferring keeps v1 lean while the mapping/matching code is structured so a provenance lookup can slot in ahead of the composite fallback.
- **Alternatives considered**: Adding an `external_ref` column to `time_entries` now (migration + sqlx cache churn for a case v1 may not hit); a generic import-batch audit only (does not help matching).

## 6. User resolution: match by email, never provision

- **Decision**: Resolve each time-entry row's person to an existing Horae user by **email** (the Harvest export's person email, or first/last name mapped to an email column when present). If no user matches, the row is a per-row **error** and the run continues; the importer never creates user accounts.
- **Rationale**: User provisioning already has an owner (OIDC / admin `user create`) with its own authorization and identity rules; duplicating it in the importer would risk creating shadow accounts and bypassing that path (Constitution IV). Erroring unmatched rows surfaces the gap so the admin provisions the user and re-imports just those rows.
- **Alternatives considered**: Auto-creating placeholder users (pollutes the user table, breaks sign-in/identity assumptions); attaching orphan entries to the importing admin (silently misattributes time).

## 7. Dry-run: same engine, no writes

- **Decision**: Dry-run runs the full parse → resolve → plan pipeline and produces the identical per-entity would-create/update/skip/error counts and per-row error report, but the apply stage performs no writes (implemented by running the same code inside a transaction that is rolled back, or by a plan-only path that never enters the write stage). A committing run on the same unchanged input/data reproduces the preview (FR-015, SC-004).
- **Rationale**: Sharing one engine guarantees the preview matches reality (the biggest failure mode of dry-runs is divergence from the real path). Rollback-in-transaction is the simplest way to exercise the real resolution/matching against live data while guaranteeing nothing persists.
- **Alternatives considered**: A separate estimation path (drifts from the real importer); dry-run that only validates parsing (misses conflicts against existing data).

## 8. Resilience and reconciliation: per-row transactional application

- **Decision**: Apply each source row's writes as an **all-or-nothing** unit (a per-row transaction / savepoint) so a mid-row failure leaves no partial fragment (FR-020). A failing row is recorded with its source line and reason and the run continues (FR-018). The summary reports created/updated/skipped/errored per entity type and MUST reconcile: `processed = created + updated + skipped + errored` (FR-021).
- **Rationale**: Migration-scale files always contain some bad rows; aborting the whole import on one is unusable. Savepoint-per-row keeps the datastore consistent while preserving partial success. Reconciliation makes "no row silently lost" verifiable.
- **Alternatives considered**: One transaction for the whole file (one bad row rolls back everything); no transaction boundary per row (risk of orphaned parent without child, or half-written entry).

## 9. Scale: stream rows, batch writes, dedup parents in-run

- **Decision**: Parse the CSV as a streaming record iterator (the `csv` crate's reader) rather than loading the whole file into memory, and cache resolved parent entities (client/project/task) in an in-run map so a client seen on 50,000 rows is resolved/created once (FR-021 edge case, SC-006). Writes are batched in bounded chunks.
- **Rationale**: A ≥100k-row export must import without exhausting memory; streaming + an in-run parent cache keeps memory bounded and avoids re-querying the same parent per row.
- **Alternatives considered**: Loading all rows then grouping (peak memory scales with file size); resolving each parent per row against the DB (redundant queries, slow).

## 10. Delivery surface: admin server function and/or CLI, one shared engine

- **Decision**: Expose the importer through an admin-only Dioxus `#[server]` function (backing an "Import from Harvest" upload screen) taking the uploaded CSV and a `dry_run` flag, and/or a server-binary CLI subcommand (`import harvest <file> [--dry-run]`). Both call the same engine. At least one administrator-invocable surface ships in v1.
- **Rationale**: The `#[server]` path fits the SPA and Constitution IV's single authorized mutation path; the CLI fits an operator doing a one-shot migration on the host (large files, no browser upload limits). Sharing the engine keeps behavior identical. A two-state `dry_run` flag is modeled as a plainly named bool / small enum per the repo's "avoid `Option<bool>`" rule.
- **Alternatives considered**: UI-only (awkward for very large files / server-side operators); CLI-only (misses the in-app migration flow).

## Resolved unknowns

All Technical Context items are resolved: source format (Decision 1), mapping authority (2), exact conversions (3), natural keys (4), provenance posture (5), user resolution (6), dry-run mechanism (7), resilience/reconciliation (8), scale strategy (9), and delivery surface (10). No `NEEDS CLARIFICATION` remains.
