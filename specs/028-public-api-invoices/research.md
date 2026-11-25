# Research: Invoices in the public API

## R1. Who may read invoices over the API

**Decision**: the web app's rule, unchanged: `ops::invoices::authorize_read(actor, READ)`, the
whole-invoice read grant. `reads::page` and `reads::require` call it; `reads::get` calls the detail
page's own `ops::invoices::detail`.

**Why**: 022 FR-020 and FR-021. The managed invoice grants authorize nothing in the web app yet
(015 T006), so they authorize nothing here.

## R2. OAuth tokens

**Decision**: the three operations declare `security(("token" = []))` only. `v1::scopes` maps an
operation without an `oauth2` requirement to "refused" (024 FR-062), so an OAuth token gets 403
`insufficient_scope` before the handler runs.

**Why**: the scope ceiling of 024 (`scopes_let_through_only_time_and_catalog_reads` and its
properties) lets only time and catalogue grants through an OAuth token. An `invoices:read` scope
means a new scope, a wider ceiling, its proofs and the consent screen's wording: its own feature.

**Alternatives**: a scope now (rejected, above); declaring `catalog:read` (rejected: it would let
the token reach the handler, whose actor holds no invoice grant, and answer 403 `forbidden`, which
misstates why).

## R3. "Updated since"

**Finding**: `invoices` has `created_at` and `edit_revision`, no update time. Its
`advance_invoice_edit_revision` trigger compares `OLD IS DISTINCT FROM NEW`, which in a BEFORE
trigger sees the generated columns unset, so it advances on every update, including no-op saves.

**Decision**: migration 0011 adds `updated_at` (default `clock_timestamp()`, existing rows take
`created_at`) and a BEFORE UPDATE trigger running `set_updated_at()` when the stored columns differ,
named one by one, because a WHEN clause may not read generated columns. `edit_revision` is not in
the list. An index on `(org_id, updated_at)` serves the filter.

**Consequence**: a line added or removed without changing the amounts does not move the update
time. A draft's line edit recomputes its amounts, so it does in practice.

## R4. Order and pagination

**Decision**: `id` order with the signed cursor of `crate::cursor`, list name `invoices`, filters
serialized as JSON (as `/time_entries`), so a cursor is bound to its filters.

## R5. The PDF

**Decision**: one function, `exports::invoice_pdf(pool, caller, id)`, serves the web app's
download and the API. `begin_invoice_export`, `render_invoice_export` and `limits::{invoice, pdf}`
take the `Caller` instead of a session's `(org_id, user_id)`; the session routes build
`Caller::session`. The handler first runs `reads::require` in a snapshot read, so refusals have the
API's statuses (404 for an unknown invoice, where the session download answers 403 to avoid
confirming it exists); the export then checks again, as before, and its statuses map to problems:
`too_large` (413), `unavailable` (503), `timeout` (504).

**Why**: the same bytes (spec SC-003); `render_invoice_pdf` is already deterministic (no timestamp).

## R6. Public types

**Decision**: `Invoice` (list), `InvoiceDetail` (`Invoice` flattened, plus `lines`),
`InvoiceLine`, `InvoiceStatus`, `InvoiceSource`. Money as `Money` (hundredths and currency). The
second tax's name is `null` without one. An imported line gives description and amount only.
`client_name` is the name now, as the Invoices page shows it.
