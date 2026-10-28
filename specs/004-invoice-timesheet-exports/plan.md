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

## Acceptance Increment: Legacy Report Export Reconciliation (2026-10-25)

Scope: the spec section of the same name. This adds tests only: no product code, query,
column, migration or dependency changes. The Phase 0 decisions are in `research.md` (L1–L4),
the run guide is in `quickstart.md` (Scenario 5), and the tasks are in `tasks.md`.

### Technical Context

- **Surface under test**: `pages/reports.rs` `LegacyReports` (rendered while
  `permission_policy_version = 0`) and its Export CSV/XLSX links to
  `/api/reports/export/{csv,xlsx}`. CSV is streamed by `reports/streaming.rs::entries`;
  XLSX is built by `limits::time::entries` and `entries_xlsx`.
- **Values**: Hours = `te.minutes`; Rounded = `effective_minutes(minutes, frozen, org.round_minutes, org.round_dir)`. On the Time tab, Billable hrs sums rounded minutes
  where the entry is billable. The page and CSV show `format_hours2` (half-up to two
  decimals); XLSX writes `minutes / 60` as a number.
- **Harnesses**: the existing browser runner `tests/browser/run-design-checks.sh`
  (disposable `/tmp/horae-browser.*` PostgreSQL, seeded, on port 8093, Playwright) and the existing
  Rust HTTP harness `server_fns/importers/authorization_tests` (`Api`: real export routes,
  session cookies, `#[sqlx::test]` throwaway database).
- **Readers**: CSV with the `csv` crate (Rust) and an exact expected-text comparison
  (browser). XLSX with the existing `zip` dev-dependency and the existing test-only cell
  helpers (`xlsx_part`, `assert_cell`, `assert_text_cell`), extracted from the
  `reports/limits.rs` tests into a shared `#[cfg(test)]` module. No new reader, helper or
  dependency.

### Constitution Check (pre- and post-design)

| Principle | Assessment |
|---|---|
| I. Exactness | ✅ for the new tests: expectations are integer fixture minutes, totals are summed as integers, and XLSX number cells are compared as exact text literals, so tests do no float arithmetic. ⚠️ **Open analyze finding D1, not waived**: the existing `entries_xlsx` writes durations as `f64` (`minutes / 60`). This increment does not change that behavior and does not certify it; it is left for independent review. |
| II. Domain Purity | ✅ No domain logic is added. |
| III. Single Datastore | ✅ PostgreSQL only; fixtures live in disposable databases; no migration. `.sqlx/` is regenerated for the new test queries. |
| IV. Mutations Through Server Functions | ✅ Read-only download routes are only read; fixture SQL runs in test setup. |
| V. Reproducible Builds | ✅ Runs under the existing flake checks (`tests`, `browser`); `nix fmt` gate. |

Result: PASS for the test-only work. Analyze finding D1 stays open: it is a pre-existing deviation, not one introduced here, so this increment does not justify it in Complexity Tracking.

### Fixture (expectations shared by both layers)

Each layer builds its own parallel copy of this fixture in its own disposable database:
the same names, minutes, flags and order, but different UUIDs (only the ascending order
of entry IDs matters). Every name is created by the fixture: clients "Export client A" and "Export client B",
projects "Export alpha" and "Export beta" (client A) and "Export gamma" (client B), task
"Export task", and user "Export colleague". The actor is the session user: "Admin User"
in the browser (the demo seed's admin) and "Test User" in Rust (`test_seed::seed`,
Manager). Organization rounding: 15 minutes, nearest. The browser asserts the seed's
setting; Rust sets it. Period: 2041-03-04 to 2041-03-06. Rows are ordered by date, project
name, task name, then entry ID; fixture IDs ascend E1 → E6, which fixes E1 before E2 and E3
before E4.

| Entry | Date | Client / project | User | Raw | Rounded | Billable | XLSX E / F text | Notes |
|---|---|---|---|---|---|---|---|---|
| E1 | 03-04 | A / Export alpha | actor | 50 | 45 | Yes | `0.8333333333333334` / `0.75` | `Legacy export, "one"` |
| E2 | 03-04 | A / Export alpha | Export colleague | 50 | 45 | Yes | `0.8333333333333334` / `0.75` | `Legacy export <two> & more` |
| E3 | 03-05 | A / Export beta | actor | 53 | 60 | No | `0.8833333333333333` / `1` | `Legacy export three` |
| E4 | 03-05 | A / Export beta | actor | 7 | 0 | Yes | `0.11666666666666667` / `0` | none (screen "—", file empty) |
| E5 | 03-06 | B / Export gamma | actor | 90 | 90 | Yes | `1.5` / `1.5` | `Legacy export excluded` |
| E6 | 03-07 | A / Export alpha | actor | 60 | 60 | Yes | n/a | `Legacy export outside` (out of period) |

The XLSX text is what the existing writer emits for `minutes / 60` (Rust `f64` display).
T004's first run confirms the strings. A different string is either a test-literal
correction, reviewed before it is accepted, or a product change to report; it is never
tolerance-matched.

Filter states and expected entries (raw / rounded / billable-rounded minutes → displayed):

- No filter: E1–E5, 250 / 240 / 180 → 4.17 / 4.00 / 3.00. By project: alpha
  100/90/90 → 1.67/1.50/1.50, beta 60/60/0 → 1.00/1.00/0.00, gamma 90/90/90 → 1.50/1.50/1.50.
- Project only (gamma, no client): E5, 90 / 90 / 90 → 1.50 / 1.50 / 1.50.
- Client A only: E1–E4, 160 / 150 / 90 → 2.67 / 2.50 / 1.50. The raw row values 0.83 + 0.83 +
  0.88 + 0.12 add up to 2.66, which is not 2.67; the total is correct because it is summed in minutes.
- Client A and project beta: E3–E4, 60 / 60 / 0 → 1.00 / 1.00 / 0.00.
- Teammate only (Export colleague, no client or project): E2, 50 / 45 / 45 → 0.83 / 0.75 /
  0.75. By project: alpha 0.83/0.75/0.75. E1 shares E2's date and project, so only the
  person filter excludes it.
- Client A and project gamma (impossible): no rows; direct link only, because the page's
  project list narrows to the chosen client.

### Proof Map

| Proof | Layer | What it shows | Requirements |
|---|---|---|---|
The four visible states (S1 no filter, S2 project gamma only, S3 client A only, S4 client A
and project beta) are covered in both layers. S5, the impossible client A and project gamma
combination, and S6, another organization's client, are Rust only. S7, teammate "Export
colleague" only, is covered in both layers. It is regression coverage for the existing
Teammate filter and traces to no FR; the Requirements column applies to S1–S4.

| Proof | Layer | What it shows | Requirements |
|---|---|---|---|
| P1 | Browser | For S1–S4 and S7, both Export links equal `/api/reports/export/{csv,xlsx}?from=2041-03-04&to=2041-03-06` plus exactly the visible filters (`&project_id=gamma`; `&client_id=A`; `&client_id=A&project_id=beta`; `&user_id=colleague`) and differ only in path | FR-011, FR-013, FR-017 |
| P2 | Browser | For S1–S4 and S7, Time-tab rows (by label) and the total row's Total, Rounded and Billable hrs equal the fixture constants | FR-019, 001 SC-007 |
| P3 | Browser | For S1–S4 and S7, the Detailed-tab rows (8 cells each, in order) equal the expected entries; excluded entries (E5 in S3/S4, E1–E4 in S2, E1 and E3–E5 in S7, E6 always) are absent | FR-013, SC-L03 |
| P4 | Browser | For S1–S4 and S7, clicking Export CSV (real download event) yields `timesheet.csv` whose bytes equal the CSV built from the constants. S1 is the regression proof that the export is unchanged | FR-011–FR-013, FR-012, FR-015, SC-L01, SC-L04 |
| P5 | Browser | For S1–S4 and S7, clicking Export XLSX yields `timesheet.xlsx` starting with `PK`. Contents are not decoded in the browser; see P6 | FR-015 (link only) |
| P6 | Rust | For every state S1–S7 (S5 and S6 hold header rows only; see P8, P9), using the same route and query template as P1 over the Rust fixture's UUIDs: the XLSX row count is entries + 1; A–D, G and H match via `assert_text_cell`; E and F match via `assert_cell` with the exact literal for each entry. Each row maps to one fixture entry, and that entry's integer minutes give the raw, rounded and billable-rounded sums | FR-011–FR-015, FR-019, SC-L01, SC-L02, SC-L04 |
| P7 | Rust | For every state S1–S7, the CSV (parsed with `csv`) equals the literal rows, so each row matches one fixture entry. The minute totals are a fixture self-check: the matched entries' integer minutes are summed and compared with the hand-derived totals, not read from the file | FR-015, SC-L02 |
| P8 | Rust | S5 gives a header-only CSV and a header-only XLSX, both 200. The Rust fixture's organization is asserted to be at policy 0 before any download | FR-014, SC-006 |
| P9 | Rust + existing | S6 (Rust only): `client_id` naming another organization's client that has an entry in the period gives header-only files, both 200; that entry never appears in S1. Existing tests, unchanged: `authorization_tests/time_reports.rs` covers 401/403, role and policy binding (its foreign-ID downloads run after a switch to policy 1), and legacy-path tenant scoping is covered at function level in `reports/streaming/database_tests.rs` and the `reports/limits.rs` tests | FR-018 |

- **G1 (FR-012 / SC-005)**: no file from before this change is stored. The proof that the
  unfiltered export is unchanged is the exact expected S1 content (P4 bytes, P6 cells), fixed
  at the current product behavior. Any later change to the unfiltered export fails it.
- **I1**: the plan Summary and the route contract above are historical text and still
  describe `fetch_entries`. The current paths are the ones in this section's Technical
  Context. The historical text is kept as written.
- **Residual**: the browser proves the XLSX link and that a file downloads. XLSX contents
  are proven by the Rust layer for the same route and query template over its own parallel
  fixture, because the browser environment has no XLSX reader and adding one is outside
  scope. The XLSX number cells are checked as exact text only; that they convert back to
  whole minutes is not claimed (analyze finding D1).

### Structure

```text
crates/horae/src/reports.rs                      # + #[cfg(test)] pub(crate) mod xlsx_cells;
crates/horae/src/reports/xlsx_cells.rs           # NEW: xlsx_part, assert_cell, assert_text_cell moved
                                                 #      verbatim from limits.rs tests (no new helper)
crates/horae/src/reports/limits.rs               # tests import the moved helpers (no behavior change)
crates/horae/src/server_fns/importers/authorization_tests/time_reports.rs
                                                 # + mod legacy_exports; call from check()
crates/horae/src/server_fns/importers/authorization_tests/time_reports/legacy_exports.rs  # NEW (P6–P8)
crates/horae/tests/browser/reports-legacy-exports.cjs                                     # NEW (P1–P5)
crates/horae/tests/browser/run-design-checks.sh  # add the suite after reports-currency-totals
                                                 # (it needs policy 0, before reports-permissions)
.sqlx/                                           # regenerated for the new test queries
```

No change to `nix/checks/browser.nix`, product code or ignore files.
