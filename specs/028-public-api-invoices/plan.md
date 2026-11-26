# Implementation Plan: Invoices in the public API

**Branch**: `feat/api-invoices` | **Date**: 2026-11-25 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/028-public-api-invoices/spec.md`

## Summary

Three read operations on `/api/v1`: `GET /invoices`, `GET /invoices/{id}` and
`GET /invoices/{id}/pdf`. The list and the read are thin `utoipa` handlers over new shared reads in
`ops::invoices::reads`, which use the Invoices page's authorization and the detail page's read. The
PDF goes through the web app's export (`exports::invoice_pdf`), which now takes the request's
`Caller` instead of a session's ids. Migration 0011 gives invoices an `updated_at` with a trigger,
for "updated since". The operations declare the bearer token only, so OAuth tokens are refused.

## Technical Context

**Language/Version**: Rust 2024 (workspace toolchain from the flake)

**Primary Dependencies**: Axum, utoipa / utoipa-axum, sqlx (compile-time checked queries), typst
(the PDF, unchanged)

**Storage**: PostgreSQL; one migration, `0011_invoice_updated_at.sql`

**Testing**: `#[sqlx::test]` in `horae-server` (the reads) and `horae-api` (the routes, every JSON
answer checked against the document by `v1::testing::send`); the OpenAPI snapshot;
`nix flake check` (clippy, fmt, `.sqlx`, `api-compat`, `api-v1`)

**Project Type**: web application (virtual workspace)

**Constraints**: v1 changes only by additions (`api-compat`); `.sqlx/` regenerated

## Constitution Check

| Principle | Check | Result |
| --- | --- | --- |
| I. Exactness | Amounts are the stored integers in hundredths with the invoice's currency; imported lines' decimal quantities are left out | Pass |
| II. Domain purity | No rule added; `is_overdue` stays in `horae-core` | Pass |
| III. Single datastore | One PostgreSQL migration, a column, a trigger and an index | Pass |
| IV. Shared operations | Reads only, through `ops::invoices` with the web app's grants; no write | Pass |
| V. Builds and formatting | `nix fmt`, clippy, `.sqlx`, `nix flake check` | Pass |

## Project Structure

```text
crates/server/migrations/0011_invoice_updated_at.sql   updated_at, trigger, index
crates/server/src/ops/invoices/reads.rs                page, get, require (+ reads/tests.rs)
crates/api/src/exports.rs                              invoice_pdf(pool, caller, id)
crates/api/src/exports/limits.rs                       invoice(), pdf() take a Caller
crates/api/src/v1/invoices.rs                          the three handlers (+ invoices/tests.rs)
crates/api/src/v1/types.rs                             Invoice, InvoiceDetail, InvoiceLine, …
crates/api/src/v1.rs                                   routes, tag, Pdf response
crates/api/openapi/v1.json                             snapshot
```

## Complexity Tracking

None.
