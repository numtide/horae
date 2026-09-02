# Implementation Plan: Timesheet Exports for Invoicing Transparency

**Branch**: `feat/invoice-timesheet-exports` | **Date**: 2026-09-01 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/004-invoice-timesheet-exports/spec.md`

## Summary

Two additive, read-only export capabilities layered on the existing export routes:

1. **Filtered detailed timesheet export** — extend the existing detailed CSV/XLSX export (`export_csv`/`export_xlsx` in `reports.rs`, today filtered only by `from`/`to`) with optional `client_id` and `project_id` query parameters, using the same `($N::uuid IS NULL OR …)` optional-filter SQL pattern already used by `server_fns/reports.rs::report_time`.

1. **Invoice timesheet backup** — from an invoice, download the timesheet detail behind it in CSV, XLSX, and a new client-facing PDF. The backup's entry set is the **exact time entries referenced by `invoice_line_items.time_entry_id`** — not a proxy date window — joined back to their project/task/date/notes for grouping. Hours come from the whole-minute values already stored; amounts are the `amount_cents` already recorded on each line (never recomputed); the header period is the derived MIN/MAX `spent_date` of the billed entries. The PDF groups project → task with per-task, per-project, and grand hours (and, where rated, amount) subtotals, reusing the existing Typst rendering approach (`render.rs`, already a dependency).

Technical approach: all work lives in the plain-Axum export layer (`reports.rs` + a new Typst template + new routes in `main.rs`) plus a small pure grouping/subtotal helper in `horae-core`. No schema change, no new mutation path, no change to how invoices are generated or how amounts are computed.

## Technical Context

**Language/Version**: Rust (edition 2024)

**Primary Dependencies**: Axum (plain routes for binary downloads), sqlx (compile-time-checked macros), `csv`, `rust_xlsxwriter`, `typst`/`typst-as-lib` (already used for invoice PDFs), chrono, uuid (v7); Dioxus for the UI controls

**Storage**: PostgreSQL 15+; **no migration** — reads existing `time_entries`, `projects`, `clients`, `tasks`, `users`, `invoices`, `invoice_line_items`. `.sqlx/` offline cache regenerated for the new/changed queries.

**Testing**: `cargo test -p horae-core` (pure grouping/subtotal helpers); `#[sqlx::test]` + `#[serial]` integration in `crates/horae/tests/` (filtered-export SQL, invoice-backup entry set + totals)

**Target Platform**: Linux server (Axum) + WebAssembly SPA (Dioxus web) for the download controls

**Project Type**: Web application (single feature-gated crate `horae`, two targets) + pure `horae-core` domain crate

**Performance Goals**: One-shot export/render on demand; data volume is one invoice's lines or one date-range's entries (hundreds of rows). No streaming required.

**Constraints**: Durations exact **integer minutes**, summed in minutes then presented as hours; money exact **integer cents** in the invoice's ISO currency, summed only within that one currency; **no floating-point accumulation** for any subtotal. The backup is a read-only projection — it MUST NOT mutate any invoice, line, or entry.

**Scale/Scope**: Single organization; touches `reports.rs`, a new Typst template, new routes in `main.rs`, one new `horae-core` grouping helper, and two UI surfaces (invoice detail actions, reports/export filters).

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Assessment |
|-----------|------------|
| **I. Exactness** | ✅ Hours derive from stored whole minutes and are summed in minutes before display; amounts are the invoice lines' recorded integer `amount_cents`, summed only within the invoice's single currency, never recomputed and never via float. Grand totals equal the sum of parts by construction (FR-009, FR-019, FR-020). |
| **II. Domain Purity** | ✅ The correctness-critical part — grouping billed entries project → task and computing minute/cent subtotals — is a pure function in `horae-core` (input: rows of {project, task, minutes, amount_cents?}; output: nested subtotaled groups), unit-tested in isolation. CSV/XLSX/PDF serialization and SQL stay in the `horae` app crate. |
| **III. Single Datastore** | ✅ PostgreSQL only; **no migration** and no schema change (the exact-entries decision deliberately avoids persisting a period). Reads existing tables; `.sqlx/` cache regenerated for changed queries. |
| **IV. Mutations Through Server Functions** | ✅ No mutations at all — every capability is a read-only export. New surfaces are plain-Axum download routes, exactly the "non-mutating export surface" the constitution permits (like the existing `export_csv`/`export_invoice_pdf`). No second mutation path is introduced. |
| **V. Reproducible Builds & Formatting Gate** | ✅ `nix fmt` / `nix flake check` green; `.sqlx` prepare committed; new Typst template embedded at compile time like the existing `templates/invoice.typ`; no toolchain assumptions. |

**Result**: PASS — no violations. Complexity Tracking not required.

## Project Structure

### Documentation (this feature)

```text
specs/004-invoice-timesheet-exports/
├── plan.md              # This file
├── research.md          # Phase 0 — decisions & rationale (period accuracy, amounts, grouping)
├── data-model.md        # Phase 1 — read model, projections, subtotal types & rules
├── contracts/
│   ├── export-routes.md # Axum route + query-param contracts (filtered export, invoice backup)
│   └── pdf-layout.md    # PDF backup layout contract (grouping, subtotals, header)
├── quickstart.md        # Phase 1 — end-to-end validation guide
└── tasks.md             # Phase 2 — created by /speckit-tasks
```

### Source Code (repository root)

```text
crates/core/src/
├── money.rs             # (exists) integer-cent formatting reused for amount cells
└── export_backup.rs     # NEW: pure grouping (project → task) + minute/cent subtotal roll-up (no I/O)

crates/horae/src/
├── reports.rs           # extend ExportParams with optional client_id/project_id + optional-filter SQL;
│                        #   add invoice-backup fetch (join invoice_line_items → time_entries → project/task)
│                        #   and export_invoice_backup_{csv,xlsx,pdf} handlers
├── render.rs            # add render_invoice_backup_pdf(...) alongside render_invoice_pdf
├── main.rs              # register /api/invoices/{id}/backup/{csv,xlsx,pdf}; reuse /api/reports/export/*
│                        #   (filters ride existing routes as query params)
└── pages/invoices.rs    # InvoiceDetail page-actions: add "Timesheet backup" downloads (CSV/XLSX/PDF),
                         #   distinct from the existing invoice "Download PDF"
                         # + reports/export page: optional client & project selectors on the detailed export

crates/horae/templates/
└── invoice-backup.typ   # NEW: client-facing backup layout (header + project→task groups + subtotals)

crates/horae/tests/integration.rs   # filtered-export scoping; backup entry set == invoice lines; totals equal
```

**Structure Decision**: Reuse the existing two-crate layout and the established plain-Axum export seam. The only genuinely new artifacts are one pure `horae-core` module (grouping/subtotals), one Typst template, and new backup routes/handlers; everything else extends existing modules (`reports.rs`, `render.rs`, `main.rs`, `pages/invoices.rs`). This keeps the change additive and inside established seams (Constitution II/III/IV) and mirrors how the current invoice PDF export is built.

## Complexity Tracking

No constitutional violations — section intentionally empty.
