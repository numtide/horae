# Tasks: Import Harvest invoices and link the time they billed

**Input**: [spec.md](spec.md), [plan.md](plan.md), [research.md](research.md),
[data-model.md](data-model.md), [contracts/](contracts/), [quickstart.md](quickstart.md)

**Prerequisites**: PR-I1 (`design/invoices-ui`) merged; the maintainer's answers to Q1–Q3. Tasks that
depend on an answer name it; a different answer changes only those tasks.

**Tests**: required. The constitution requires `horae-core` tests and `#[sqlx::test]` integration
tests for correctness-critical changes, and this feature changes billing. Write each test first and
see it fail.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an unfinished task)
- **[Story]**: US1 to US4 from the spec; setup and foundational tasks have none
- PR letters (A, B, C) are the delivery split in plan.md

______________________________________________________________________

## Phase 1: Setup

- [ ] T001 Create branch `feat/harvest-invoice-import` from up-to-date master after PR-I1 merges, and
  re-read `ops/invoices/uninvoiced.rs` and `0005_invoice_subject.sql` as merged (the plan cites them
  from the PR branch)
- [ ] T002 [P] Record in `specs/harvest-parity.md` the user's authorization for disposable fixtures in
  the test account for this feature, and run quickstart step 5 to confirm that billing an entry moves
  its `updated_at` (research R-7); stop and report if it does not

______________________________________________________________________

## Phase 2: Foundational (PR A, blocks every story)

### Domain (`horae-core`)

- [ ] T003 [P] Test then add `InvoiceStatus::Closed` (`"closed"`) to display, parse and `pg_enum` in
  `crates/core/src/types.rs` (Q2)
- [ ] T004 [P] Test then add `money_exact(text, currency)` to
  `crates/core/src/importers/harvest/convert.rs`: signed, refuses decimals beyond the currency's and
  unknown currencies, reuses the decimals table in `crates/core/src/money.rs` (make it `pub(crate)`)
- [ ] T005 [P] Test then add `crates/core/src/importers/harvest/invoice.rs`: `map_state` (draft →
  skip per Q1, open → Sent, paid → Paid, closed → Closed per Q2, other → error) and `check_amounts`
  (lines sum to subtotal, components give total, `0 ≤ due ≤ total`, `total ≥ 0`, errors carry the
  difference)
- [ ] T006 [P] Test then add `crates/core/src/importers/harvest/link.rs`: `decide(Billing, Current) -> LinkAction` for every row of research R-6, with a proptest that it never links or unlinks an entry
  on a Horae-made invoice or a submitted entry
- [ ] T007 Extend `crates/core/src/importers/harvest/types.rs`: `EntityType::Invoice`,
  `ImportSummary.invoices`, `LinkCounts`, `ImportReport.link_conflicts` (bounded, with an omitted
  count), `invoices_skipped_drafts`, `invoice_scan`, `RowError.invoice_kind`,
  `SourceRow.harvest_invoice_id`, all `serde(default)`; a test loads a report stored before the change

### Schema

- [ ] T008 Write the migration `crates/server/migrations/00NN_harvest_invoices.sql` per
  [data-model.md](data-model.md): enum values, `invoice_source`, `invoices.source`, the three checks
  limited to Horae invoices, `invoices_closed_imported`, `invoices_harvest_no_rates`,
  `harvest_invoices`, `harvest_invoice_lines`; a `#[sqlx::test]` shows each check refuses a Horae
  invoice it refused before
- [ ] T009 [P] Add the account-switch business snapshot's two new tables in
  `crates/server/src/importers/harvest/account_switch.rs` tests

### Shared behaviour

- [ ] T010 Test then change invoice numbering in `crates/server/src/ops/invoices/generate.rs` to the
  highest numeric suffix for the month plus one (research R-8), keeping the unique-key retry
- [ ] T011 Test then read "billed" from `state = 'invoiced'` in the places listed in research R-10
  (`harvest-api/src/lib.rs`, `ops/reports.rs`, `ops/time_reads/reports.rs` and `groups.rs`,
  `api/src/exports/limits/time.rs` and `groups.rs`, `exports/streaming/cursor/time.rs` and
  `groups.rs`, `ops/projects/reads.rs`, `ops/projects/budgets.rs`); a test with an entry invoiced
  without an invoice covers each surface
- [ ] T012 Regenerate `.sqlx/` (AGENTS.md "sqlx query cache") and run the offline test build

**Checkpoint**: PR A. `nix flake check` green; no visible change except numbering and `is_billed`.

______________________________________________________________________

## Phase 3: US1, billed history stays billed (P1, PR B) MVP

### Tests

- [ ] T013 [P] [US1] Extend the Harvest test server fixtures in
  `crates/server/src/importers/harvest/engine_tests.rs` (or a new `invoice_tests.rs`) with invoices of
  each state, a draft, entries billed on each, and an entry billed on an unlisted invoice (quickstart
  §2)
- [ ] T014 [P] [US1] `#[sqlx::test]`: a commit links entries on imported invoices, leaves draft time
  open, marks the rest invoiced without invoice, and queues no invoice plugin event (FR-023); a
  preview reports the same counts and writes nothing; a member without invoice grants cannot read the
  imported invoices (FR-020)
- [ ] T015 [P] [US1] `#[sqlx::test]`: entries imported before invoices existed are linked by the next
  run, which reads entries in full and reports `invoice_scan: "full"`
- [ ] T016 [P] [US1] `#[sqlx::test]` in `ops/invoices`: after an import, Uninvoiced, preview and
  generation exclude every billed entry, and a client's last invoice `covered_to` is the latest linked
  entry (FR-025)

### Implementation

- [ ] T017 [US1] Add `ApiInvoice`, `ApiInvoiceLine` and `ApiTimeEntry.invoice` to
  `crates/harvest/src/v2.rs` with deserialization tests
- [ ] T018 [US1] Fetch invoices (all states, `updated_since` from the `invoice` watermark) after users
  in `crates/server/src/importers/harvest/streaming.rs` and carry them in the catalog page; add the
  invoice phase to the durable checkpoint in `streaming/durable`
- [ ] T019 [US1] Carry `harvest_invoice_id` from `ApiTimeEntry.invoice` onto `SourceRow` in
  `api_source.rs`
- [ ] T020 [US1] New `crates/server/src/importers/harvest/invoices.rs`: apply one invoice in a
  savepoint (resolve client and line projects, convert, check, insert `invoices` + `harvest_invoices`
  - lines + provenance) and record its outcome in `RunCache` (`resolve.rs`); run it after
    `parents.rs`, before entries
- [ ] T021 [US1] Link step in `apply.rs` after an entry is created or matched: read the entry's
  current state and invoice source, call `link::decide`, apply the action (freeze or clear rounding
  with `effective_minutes`), count it
- [ ] T022 [US1] Watermark: store the `invoice` key on commit and force a full entry scan when it is
  absent, in `credentials.rs` and the job planner
- [ ] T023 [US1] Refuse the draft editor, draft save, Duplicate and line changes for
  `source = 'harvest'` in `crates/server/src/ops/invoices.rs` and `editing.rs`, and accept only
  Sent → Paid, Closed, Void for them in `set_status` (Q3); Paid sets the imported invoice's
  `due_cents` to 0 and `paid_on` to the organization's today
- [ ] T024 [US1] `covered_to` in `ops/invoices/uninvoiced.rs` also takes entries linked by
  `invoice_id` (research R-11)

**Checkpoint**: an import leaves no billed Harvest time on Uninvoiced.

______________________________________________________________________

## Phase 4: US3, re-imports keep invoices current (P2, PR B)

- [ ] T025 [P] [US3] `#[sqlx::test]`: an unchanged re-run counts every invoice skipped and writes
  nothing (compare a snapshot of the invoice tables and entry links)
- [ ] T026 [P] [US3] `#[sqlx::test]`: a paid change updates status and due; a new invoice links two
  older entries; an unbilled entry is unlinked and open; a Horae-linked entry is kept and listed as a
  conflict; a Horae status change survives a run where Harvest did not change the invoice
- [ ] T027 [US3] Update path in `invoices.rs`: compare `harvest_updated_at`, rewrite the invoice, its
  details and lines when it moved, count updated or skipped; a vanished invoice is left and reported
- [ ] T028 [US3] Unlink and relink actions in the link step, and the `link_conflicts` list in
  `report.rs`

______________________________________________________________________

## Phase 5: US4, the report explains failures (P3, PR B)

- [ ] T029 [P] [US4] `#[sqlx::test]`: number taken, amounts mismatch, too many decimals, unknown
  currency and negative total each give one invoice error with its `invoice_kind`, write nothing of
  that invoice, and leave its entries invoiced without invoice
- [ ] T030 [US4] Map invoice failures to `RowError` with `invoice_kind` in `invoices.rs` and include
  them in the error summary and download (`report.rs`, `crates/api/src/import_errors.rs`)
- [ ] T031 [US4] CSV: "Invoiced?" yes → invoiced without invoice, counted, in `csv_source.rs` and
  `apply.rs` (Q5), with a test
- [ ] T032 [US4] Show invoice counts, link counts and conflicts on the run page in
  `crates/horae/src/pages/importers*.rs`, text in `crates/i18n/locales/en/horae.ftl`
- [ ] T033 [US4] `horae jobs report` prints the new counts in text and JSON
  (`crates/horae/src/cli*`), per [contracts/import-report.md](contracts/import-report.md)
- [ ] T034 Regenerate `.sqlx/`; run quickstart §1–§3

**Checkpoint**: PR B.

______________________________________________________________________

## Phase 6: US2, imported invoices read like Horae's (P1, PR C)

- [ ] T035 [P] [US2] Wire types: `Invoice.imported`, `ImportedInvoiceLine`,
  `InvoiceDetailView.imported_lines` in `crates/types/src/invoice.rs`, with serde tests
- [ ] T036 [US2] Read `harvest_invoices` and `harvest_invoice_lines` in the invoice list and detail
  queries in `crates/server/src/ops/invoices.rs`; imported breakdown per
  [contracts/invoice-surfaces.md](contracts/invoice-surfaces.md)
- [ ] T037 [US2] PR-I1's list summary: outstanding and overdue use `due_cents` for imported invoices;
  Closed is neither (research R-11)
- [ ] T038 [US2] Detail page: imported marker, imported lines table, Paid and Due, Closed badge, no
  editor or Duplicate actions, Mark paid / Close / Void in the menu, in
  `crates/horae/src/pages/invoices*.rs`; all text in the catalog; Dioxus lint ratchet green
- [ ] T039 [US2] PDF: imported lines and breakdown in `crates/server/src/render.rs` and
  `crates/server/templates/invoice.typ`, with a render test comparing amounts
- [ ] T040 [P] [US2] Browser check of an imported invoice's detail in `crates/horae/tests/browser/`
- [ ] T041 [US2] Run quickstart §4

**Checkpoint**: PR C.

______________________________________________________________________

## Phase 7: Polish

- [ ] T042 [P] Scale test: extend the import benchmark fixture with 2,000 invoices and measure
  against SC-006; record the result in `specs/025-harvest-invoice-import/`
- [ ] T043 [P] Mark 004 FR-016 as amended by 025 in `specs/004-harvest-importer/spec.md`, and add the
  invoice import to `specs/harvest-parity.md`
- [ ] T044 Update AGENTS.md only if a command or surface it lists changed; `nix fmt`;
  `nix flake check`

## Dependencies & Execution Order

- Phase 2 blocks everything. T003–T007 are independent of T008–T011 except T007 (types) before T020.
- US1 (Phase 3) before US3 and US4, which extend the same modules. US2 (Phase 6) needs only Phase 2 and
  T020 (imported rows to show), so it can start once T020 lands.
- T023 lands in PR B even though it belongs to read-only behaviour, so no imported invoice is ever
  editable.

## Parallel opportunities

- T003, T004, T005, T006 (separate `horae-core` files).
- T013–T016 tests once the fixtures exist.
- T035 and T040 beside the server work of Phase 6.

## Implementation strategy

MVP is PR A + Phase 3: billed history stays billed. US3 and US4 complete PR B. PR C makes the imported
invoices pleasant to read; release A, B and C together (plan.md, Delivery).
