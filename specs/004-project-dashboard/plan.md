# Implementation Plan: Project Detail Dashboard

**Branch**: `feat/project-dashboard` | **Date**: 2026-09-01 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/004-project-dashboard/spec.md`

## Summary

Replace the stub `ProjectDetail` page (today: `"Project detail for {id}"` plus an assignments table) with a real, data-honest project dashboard. It shows the project's identity, budget & progress (driven by `budget_kind`), total/billable/non-billable hours, billable amount, invoiced vs uninvoiced money, per-task and per-person breakdowns, the team and enabled tasks, and a recent-entries feed.

Technical approach: **no schema migration**. Every figure is computed from existing tables (`projects`, `time_entries`, `project_tasks`, `assignments`, `users`, `invoices`, `invoice_line_items`) by new **read-only** `#[server]` aggregation functions. Spend (hours + billable amount) reuses the exact FR-024 rate cascade already used by the Projects list — `horae_core::invoice::resolve_rate` + `line_amount_cents` — so the dashboard's "Spent" always matches the list. Invoiced money is read directly from `invoice_line_items.amount_cents` (authoritative: what was actually billed); uninvoiced is the resolved billable value of not-yet-invoiced billable entries. The existing rate-resolution/aggregation math is lifted into a small pure `horae-core` helper so it is unit-tested in isolation and shared by the list rollup and the dashboard. The page renders with existing design tokens/utilities (cards, progress bars, tables); no dedicated detail mockup exists.

## Technical Context

**Language/Version**: Rust (edition 2024)

**Primary Dependencies**: Dioxus 0.7 (fullstack + router, SSR + WASM), Axum, sqlx (compile-time-checked macros), chrono, uuid (v7)

**Storage**: PostgreSQL 15+; **no new migration** — read-only aggregation over existing tables; `.sqlx/` offline cache regenerated for the new queries

**Testing**: `cargo test -p horae-core` (pure aggregation/rate math); `#[sqlx::test]` + `#[serial]` integration in `crates/horae/tests/` for the new server functions (reconciliation and invoiced/uninvoiced)

**Target Platform**: Linux server (Axum) + WebAssembly SPA (Dioxus web)

**Project Type**: Web application (single feature-gated crate `horae`, two targets) + pure `horae-core` domain crate

**Performance Goals**: Dashboard opens without perceptible delay; per-project aggregation over that project's time entries (a bounded set), served by a small number of queries — target a single page load with no N+1 fan-out

**Constraints**: All money is integer minor units (cents) in the project's own currency, no floats and no cross-currency conversion; hours derived from integer `minutes`; totals must reconcile exactly (breakdown rows sum to headline; invoiced + uninvoiced = total billable)

**Scale/Scope**: Single organization; one existing route (`Route::ProjectDetail { id }`); additive change touching `horae-core` (extract/extend rate-aggregation helper), new read-only `#[server]` functions, and the `ProjectDetail` page renderer. No schema change.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Assessment |
|-----------|------------|
| **I. Exactness** | ✅ Hours from integer `minutes`; money as integer cents with the project's currency; billable amount uses the existing integer `line_amount_cents` (banker's rounding) and the FR-024 cascade; invoiced read from stored `amount_cents`. No floats. Reconciliation is a stated success criterion (breakdown rows sum to totals; invoiced + uninvoiced = total billable). |
| **II. Domain Purity** | ✅ The correctness-critical part — resolving each entry's rate and aggregating minutes/amount into totals and by-task/by-person groupings — is factored into a pure `horae-core` helper (no sqlx/axum/dioxus) and unit-tested there. SQL only fetches the raw per-entry rows; grouping/summing stays pure, mirroring how `list_project_spend` already resolves rates in Rust. |
| **III. Single Datastore** | ✅ PostgreSQL only; **no migration** — read-only aggregation over existing tables. PKs untouched; `org_id` already scoped. `.sqlx/` cache regenerated for the new read queries. |
| **IV. Mutations Through Server Functions** | ✅ Feature is entirely **read-only**; it adds `#[server]` read functions and issues no client-side fetches. No new mutation path; existing assignment mutations already on the page are unchanged. |
| **V. Reproducible Builds & Formatting Gate** | ✅ `nix fmt` / `nix flake check` green; `.sqlx` prepare committed; no toolchain assumptions; new `horae-core` unit tests and `#[sqlx::test]` integration tests. |

**Result**: PASS — no violations. Complexity Tracking not required.

## Project Structure

### Documentation (this feature)

```text
specs/004-project-dashboard/
├── plan.md              # This file
├── research.md          # Phase 0 — decisions, data-source map, deferrals
├── data-model.md        # Phase 1 — read model (view models) + derivations, no schema change
├── contracts/
│   └── server-fns.md    # New read-only #[server] signatures + returned view models
├── quickstart.md        # Phase 1 — end-to-end validation guide
├── checklists/
│   └── requirements.md  # Spec quality checklist (from /speckit-specify)
└── tasks.md             # Phase 2 — created by /speckit-tasks
```

### Source Code (repository root)

```text
crates/core/src/
├── invoice.rs           # (exists) resolve_rate + line_amount_cents — reused unchanged
└── project_rollup.rs    # NEW (pure): fold per-entry rows into totals + by-task/by-person
                         #   breakdowns (billable/non-billable minutes, billable cents);
                         #   shared by list_project_spend and the dashboard; unit-tested

crates/horae/src/
├── models/dashboard.rs          # NEW: dashboard view-model DTOs (header, budget, totals,
│                                #   breakdown rows, team/task rows, recent-entry rows)
├── server_fns/projects.rs       # ADD read-only fns: get_project_dashboard (identity+budget+
│                                #   totals+invoiced/uninvoiced), list_project_breakdowns
│                                #   (by task + by person), list_recent_project_entries;
│                                #   refactor list_project_spend onto the shared rollup
└── pages/projects.rs            # Replace ProjectDetail stub body with the dashboard sections;
                                 #   keep the existing assignments block as the team section

crates/horae/assets/css/horae.css   # dashboard card/stat/breakdown-table styles (reuse proj-bar etc.)
crates/horae/tests/integration.rs   # reconciliation (rows sum to totals), invoiced+uninvoiced=billable,
                                     #   empty-state + over-budget + unresolvable-rate cases
```

**Structure Decision**: Reuse the existing two-crate layout. The only genuinely new files are one pure `horae-core` module (the rollup fold), one models file for the view-model DTOs, and the design docs; the server functions extend `server_fns/projects.rs` and the page rewrites the `ProjectDetail` body. Extracting the fold into `horae-core` lets the Projects list rollup and the dashboard share one tested implementation, so their "Spent" figures cannot drift (SC-002). No migration, matching FR-016/SC-007.

## Complexity Tracking

No constitutional violations — section intentionally empty.
