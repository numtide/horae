# Implementation Plan: Import Harvest invoices and link the time they billed

**Branch**: `spec/harvest-invoice-import` (documents); implementation on `feat/harvest-invoice-import`
from master after PR-I1 (`design/invoices-ui`) merges | **Date**: 2026-11-22 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/025-harvest-invoice-import/spec.md`

## Summary

The Harvest API import reads the account's invoices after its catalog and before its time entries.
Open, paid and closed invoices become Horae invoices with `source = 'harvest'`, keeping Harvest's
number, amounts (converted exactly to minor units and checked to add up) and lines, which go in their
own table so Horae's line invariants stay as they are. Drafts are read only to know which entries to
leave open. Each time entry row carries its Harvest invoice id, and a pure decision table in
`horae-core` sets, keeps or clears its link inside the entry's savepoint. Entries Harvest billed on
an invoice Horae could not import become invoiced with no invoice, so nothing billed is offered
again. Re-imports update imported invoices through provenance and never touch Horae's own. The report
gains invoice counts and link counts; the Invoices screens show imported invoices read only, with
their lines, paid and due amounts, and a PDF. "Billed" reads the entry state instead of the invoice
link across reports, exports and the Harvest-compatible API.

## Technical Context

**Language/Version**: Rust 2024 from the Nix dev shell; Dioxus 0.7.9 fullstack

**Primary Dependencies**: sqlx (compile-time checked macros), axum, serde with `arbitrary_precision`
(exact Harvest decimals), ureq (Harvest HTTP), Typst (invoice PDF), the `jobs` lease and checkpoint
machinery

**Storage**: PostgreSQL. One migration (`0006_harvest_invoices.sql` or the next free number at
implementation): `invoice_status` gains `closed`, `harvest_entity_type` gains `invoice`, `invoices`
gains `source`, the three exact-rounding checks are limited to Horae invoices, and two tables are
added (`harvest_invoices`, `harvest_invoice_lines`). See [data-model.md](data-model.md).

**Testing**: `cargo test -p horae-core` (conversion, adds-up check, state mapping, link table with
proptest), `cargo test -p horae-server` (`#[sqlx::test]` importer runs against the Harvest HTTP test
server and fixtures, re-imports, dry runs), `cargo test -p horae-harvest-api` (`is_billed`),
`cargo test -p horae-api` (exports' billed column), `cargo test -p horae --features server` (server
functions refusing edits on imported invoices, invoice detail), browser checks for the detail.

**Target Platform**: Linux server and WASM client

**Project Type**: Web application (app crate, backend crate, pure domain crate)

**Performance Goals**: SC-006, at most a fifth more import time for 2,000 invoices and 15,000 entries

**Constraints**: dry runs write nothing; durable checkpoints resume between batches (invoices are one
checkpointed phase before entries); old reports and checkpoints still load (`serde(default)` on new
fields); no float anywhere in conversion; Harvest is called only through fakes in tests

**Scale/Scope**: the observed production account has about 13,700 entries; its invoice count is
measured in the quickstart before the scale test is sized

## Constitution Check

*GATE: passed before Phase 0 and re-checked after Phase 1.*

- **I. Exactness**: Pass. Amounts become integer minor units through exact integer conversion that
  refuses extra decimals (R-4). Quantities and rates are stored as exact `numeric` for display and
  never used in a calculation. The import checks that lines sum to the subtotal and that the
  components give the total, in integers.
- **II. Domain Purity**: Pass. Decimal conversion, the adds-up check, the state mapping and the link
  decision table are pure functions in `horae-core` with unit and property tests. `InvoiceStatus`
  gains `Closed` there.
- **III. Single Datastore**: Pass. New tables have UUID v7 keys and `org_id`; PostgreSQL only.
- **IV. Mutations Through Shared Operations**: Pass with no amendment. The import job writes
  invoices, lines and links as it writes every imported record, authorized when an administrator
  starts the job through the `start_harvest_*` server functions (004, 005, 019). Status changes on
  imported invoices go through `ops::invoices::set_status`. No API gains a write.
- **V. Reproducible Builds & Formatting Gate**: Pass. `nix fmt`, clippy, the `.sqlx` cache, the Dioxus
  lint ratchet, the i18n unused-message test and `nix flake check`.
- **Data constraints**: Pass. `org_id` on both new tables; the relaxed checks apply only to rows
  marked `source = 'harvest'`, which only the import writes.

Re-check after design: unchanged. Complexity Tracking records the one design choice that loosens an
existing invariant.

## Project Structure

### Documentation (this feature)

```text
specs/025-harvest-invoice-import/
├── spec.md
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── import-report.md      # report and CLI JSON additions
│   └── invoice-surfaces.md   # invoice reads, status changes, PDF, Harvest-compatible is_billed
├── checklists/requirements.md
└── tasks.md
```

### Source Code (repository root)

```text
crates/core/src/
├── types.rs                          # InvoiceStatus::Closed
└── importers/harvest/
    ├── convert.rs                    # money_exact (currency decimals, signed, no rounding)
    ├── invoice.rs                    # new: state mapping, adds-up check
    ├── link.rs                       # new: entry link decision table
    └── types.rs                      # EntityType::Invoice, ImportSummary.invoices, LinkCounts, SourceRow.harvest_invoice_id
crates/harvest/src/v2.rs              # ApiInvoice, ApiInvoiceLine, ApiTimeEntry.invoice
crates/server/migrations/00NN_harvest_invoices.sql
crates/server/src/importers/harvest/
├── api_source.rs                     # carry harvest_invoice_id onto rows
├── streaming.rs (+ durable)          # fetch invoices after users; invoice phase before entries
├── invoices.rs                       # new: apply one invoice in a savepoint (create/update/skip/error)
├── apply.rs                          # link step after create or match
├── resolve.rs                        # RunCache: harvest invoice id → outcome
├── csv_source.rs / apply.rs          # Invoiced? yes → invoiced without invoice
├── credentials.rs                    # invoice watermark; full entry scan when absent
└── report.rs                         # invoice errors and link counts
crates/server/src/ops/invoices.rs (+ editing.rs, generate.rs, uninvoiced.rs)
                                      # read only for imported; Closed; numbering; covered_to; due amounts
crates/server/src/render.rs, templates/invoice.typ   # imported lines and breakdown
crates/server/src/ops/{reports.rs,time_reads/…,projects/…}, crates/api/src/exports/…,
crates/harvest-api/src/lib.rs        # billed = state 'invoiced'
crates/types/src/invoice.rs           # Invoice.imported, ImportedInvoiceLine, InvoiceDetailView
crates/horae/src/pages/invoices*.rs, pages/importers*.rs, crates/i18n/locales/en/horae.ftl
```

**Structure Decision**: the existing crates; one new module each in `horae-core` (`invoice`, `link`)
and the importer (`invoices`).

## Delivery

Three PRs, each green on `nix flake check`:

1. **A, data and domain**: migration, `horae-core` additions, `Closed`, numbering fix, "billed" as
   state (R-10). No behaviour visible yet except numbering.
1. **B, import** (US1, US3, US4): fetch, invoice phase, links, CSV flag, report, watermark, and the
   `ops` refusals that keep imported invoices read only, so no imported invoice is ever editable,
   and Uninvoiced's `covered_to` (FR-025), which US1 needs.
1. **C, screens** (US2): detail, PDF, list summary, import report UI.

Between B and C an imported invoice shows in the current detail without its lines; that is why the
three should ship in the same release.

## Complexity Tracking

| Choice | Why needed | Simpler alternative rejected because |
| --- | --- | --- |
| The three exact-rounding checks on `invoices` hold only for `source = 'horae'` | Harvest taxes per line and rounds its own way; its stated amounts are what the client was billed | Recomputing changes billed amounts; refusing such invoices drops real history |
| A second line table for imported invoices | Horae lines must be a time entry or a fee, non-negative, with an allocated discount | One table would loosen five checks and every fee-balance query |
