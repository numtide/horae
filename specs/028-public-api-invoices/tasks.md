# Tasks: Invoices in the public API

**Input**: `specs/028-public-api-invoices/` (spec.md, plan.md, research.md, data-model.md,
contracts/api-v1-invoices.md, quickstart.md)

**Tests**: requested (spec, User Stories 1 to 3); test tasks come before the code they cover.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an unfinished task)
- **[Story]**: the user story in spec.md (US1 to US3)

______________________________________________________________________

## Phase 1: Foundational

- [x] T001 Write `crates/server/migrations/0011_invoice_updated_at.sql` (data-model.md): the column, the backfill from `created_at`, the trigger over the named stored columns, the index
- [x] T002 [P] [US2] Tests first in `crates/server/src/ops/invoices/reads/tests.rs`: a no-op save keeps `updated_at`, a status change moves it, and `updated_since` returns exactly the changed invoice
- [x] T003 [P] [US1] [US3] Tests first in the same file: the page is in `id` order with `limit + 1` rows and continues after `after`; status and client narrow it; `get` reads the detail with `updated_at`; a member is refused by `page`, `get` and `require`; another organization's invoice is not found
- [x] T004 Write `crates/server/src/ops/invoices/reads.rs`: `InvoiceQuery`, `InvoiceRecord`, `InvoiceDetailRecord`, `page`, `get` (over `invoices::detail`), `require`

## Phase 2: User Story 1 and 3 - Read invoices with a token, as the web app allows (P1)

- [x] T005 [US1] Make `begin_invoice_export`, `render_invoice_export` and `limits::{invoice, pdf}` in `crates/api/src/exports.rs` and `crates/api/src/exports/limits.rs` take a `Caller`; extract `invoice_pdf(pool, caller, id)` from `export_invoice_pdf`; update the export tests' callers with `Caller::session`
- [x] T006 [P] [US1] Public types in `crates/api/src/v1/types.rs`: `Invoice`, `InvoiceDetail`, `InvoiceLine`, `InvoiceStatus`, `InvoiceSource`, with their conversions
- [x] T007 [US1] [US3] Tests first in `crates/api/src/v1/invoices/tests.rs`: an administrator's list and detail answer as documented with the expected figures; the PDF is `application/pdf`, named after the number and identical twice; a member gets 403 `forbidden` on all three; another organization's invoice is 404; an OAuth token with every scope gets 403 `insufficient_scope`
- [x] T008 [US1] Handlers in `crates/api/src/v1/invoices.rs` (`listInvoices`, `getInvoice`, `getInvoicePdf`, the `Pdf` response) and their routes, tag and response in `crates/api/src/v1.rs`
- [x] T009 [US3] Let `every_token_operation_declares_its_oauth_scope` in `crates/api/src/v1/tests.rs` accept the token-only invoice operations, and count 23 operations

## Phase 3: User Story 2 - Incremental sync (P1)

- [x] T010 [US2] In `crates/api/src/v1/invoices/tests.rs`: status filter, an unknown status refused, a cursor walk of `limit=1` returns every invoice once, a cursor reused with other filters is `invalid_cursor`, `updated_since` returns the invoice just paid, `client_id` narrows

## Phase 4: Polish

- [x] T011 `UPDATE_OPENAPI=1 cargo test -p horae-api openapi` and review the diff of `crates/api/openapi/v1.json` (additions only)
- [x] T012 Regenerate `.sqlx/` (AGENTS.md, *sqlx query cache*)
- [x] T013 `nix fmt`, clippy for `horae-server` and `horae-api`, `nix flake check`
