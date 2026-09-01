# Phase 1 Data Model: Harvest Data Importer

Derived from the spec's Key Entities and `research.md`. The importer creates rows in Horae's existing schema (`crates/horae/migrations/0001_init.sql`) — it introduces **no new persisted tables in v1**. This document defines the source-side model, the mapping onto existing Horae columns, the idempotency keys, and the in-memory run/report structures. Everything is scoped to the single `organizations` row; every created row carries `org_id`. Primary keys are UUID v7. Time is stored as **integer minutes**, money as **integer minor units (cents) + ISO 4217 currency** — never floats.

## Source-side entities (transient, not persisted)

### SourceRow

One parsed row of a Harvest detailed-time-report CSV. Denormalized — carries the parent entity fields alongside the entry. Fields (see `contracts/csv-format.md` for exact columns):

- Client: `client_name`.
- Project: `project_name`, `project_code` (optional).
- Task: `task_name`.
- Person: `user_email` (or first/last name resolved to an email).
- Entry: `spent_date`, `hours` (decimal string), `notes` (optional), `billable` (flag), `invoiced` (flag, informational only).
- Money: `billable_rate`, `billable_amount`, `cost_rate`, `cost_amount` (decimal strings, optional), `currency` (ISO code).

**Validation at parse time**: `spent_date` parses as a date; `hours` parses as a non-negative decimal; `currency` is a 3-letter code; required name columns are non-empty. A row failing parse validation becomes a `RowOutcome::Errored` with its source line and reason and does not proceed to resolution.

### ImportRun

A single execution against one source file. Fields: `mode` (`DryRun` | `Commit`), `actor_user_id` (the admin), `org_id`, source descriptor (filename/size). Produces an `ImportSummary` and a `Vec<RowOutcome>`. Conceptually the unit that must be idempotent when repeated.

## Target mapping (source → existing Horae columns)

### Client → `clients`

| Horae column | Source | Notes |
|---|---|---|
| `id` | generated | UUID v7 |
| `org_id` | run | single org |
| `name` | `client_name` | trimmed |
| `currency` | `currency` | ISO 4217; falls back per FR-013 precedence |
| `address` | (supplementary clients CSV, if provided) | else NULL |
| `active` | — | default `true` |

### Project → `projects`

| Horae column | Source | Notes |
|---|---|---|
| `id` | generated | UUID v7 |
| `org_id` / `client_id` | run / resolved client | FK-safe: client created/resolved first |
| `code` | `project_code` | optional |
| `name` | `project_name` | trimmed |
| `project_type` | — | default `time_and_materials` (Harvest `bill_by` may refine later) |
| `currency` | client currency | project currency defaults from client (matches existing data-model default) |
| `starts_on` / `ends_on` | (supplementary projects CSV, if provided) | else NULL |
| `budget_kind` | — | default `none` (supplementary budget → `amount`/`hours`) |
| `active` | — | default `true` |

### Task → `tasks` + `project_tasks`

Horae keeps an **org-level task catalog** (`tasks`) with **per-project enablement** (`project_tasks`). The importer:

1. Resolves/creates one `tasks` row per distinct `task_name` (org-scoped catalog) — `billable_default` from the row's billable flag on first sight; `default_rate_cents` from `billable_rate` when present.
1. Ensures a `project_tasks` link `(project_id, task_id)` exists for every project a task's entries reference, carrying `billable` (from the row) and optional `rate_cents` (from `billable_rate`). This satisfies FR-009 so an imported entry's task is always valid for its project.

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

Money on the entry (Harvest `billable_rate`/`billable_amount`/`cost_rate`/`cost_amount`) is converted to cents for validation/reconciliation and surfaced through the project-task/user rate fields; Harvest's per-entry billed/invoiced fact is informational only.

## Idempotency: natural keys (org-scoped)

String comparisons are trimmed and case-folded (normalization lives in `horae-core`).

| Entity | Natural key | Rationale |
|---|---|---|
| Client | `name` | export reliably carries the client name |
| Project | `code` if present, else `(client, name)` | code is the stable Harvest project identity when set |
| Task | `name` | org-level catalog; shared across projects |
| Time entry | `(user, project, task, spent_date, minutes, notes)` | distinguishes two real same-day entries while recognizing an exact re-import |

**Deferred (research.md §5)**: persisting a Harvest source identifier as a provenance/mapping record would make entry matching exact and edit-robust. If adopted, it is a **new nullable mapping table** (org_id + Harvest entry id → time_entry id) added by its own migration — it does not alter existing columns and is looked up ahead of the composite fallback. Not built in v1.

## Run/report structures (in-memory)

### RowOutcome

Per source row: one of `Created`, `Updated`, `Skipped` (matched, unchanged), or `Errored { source_line, reason }`. The raw material of both the summary and the error report (FR-019).

### ImportSummary

Per entity type (clients, projects, tasks, time entries): counts `created`, `updated`, `skipped`, `errored`. Invariant: `processed = created + updated + skipped + errored` per type (FR-021, SC-005). In `DryRun` mode the same counts are reported as would-create/update/skip/error and nothing is written (FR-014).

## Ordering & transactional rules

- **FK-safe order** per run: resolve/create client → project → task (+ project_task link) → time entry (FR-004). An in-run cache resolves each distinct parent once (research.md §9).
- **Per-row atomicity**: each source row's writes apply as an all-or-nothing unit (savepoint/transaction) so a mid-row failure leaves no partial fragment (FR-020).
- **Dry-run**: full resolve/plan against live data, zero writes — via a rolled-back transaction or a plan-only path (research.md §7).

## Cross-cutting validation

- Durations and money converted and stored as exact integers; a full-file import reconciles to zero drift against the Harvest source (FR-005/FR-006, SC-003/SC-007) — asserted in `horae-core` unit tests and integration reconciliation tests.
- Currency conflicts across levels resolve by a single defined precedence and are recorded as a fallback rather than failing the row (FR-013).
- Unmatched user → row errored, run continues (FR-010/FR-018); the importer never writes to `users`.
