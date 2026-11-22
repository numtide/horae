# Research: Import Harvest invoices and link the time they billed

Each item records a decision, why, and what else was weighed. Harvest facts come from its public API
v2 documentation (invoices, invoice line items, invoice payments, time entries); nothing here copies
Harvest's wording.

## R-1 What Harvest gives

- `GET /v2/invoices` lists invoices of every state, paged, filterable by `updated_since`, with
  `line_items` embedded. Each invoice has `id`, `client {id, name}`, `number`, `purchase_order`,
  `amount` (total), `due_amount`, `tax`, `tax_amount`, `tax2`, `tax2_amount`, `discount`,
  `discount_amount`, `subject`, `notes`, `currency`, `state` (draft, open, paid, closed),
  `period_start`, `period_end`, `issue_date`, `due_date`, `paid_date`, `sent_at`, `closed_at`,
  `created_at`, `updated_at`, plus `estimate`, `retainer` and `recurring_invoice_id`, which this
  feature ignores.
- A line item has `id`, `kind` (an account-defined name such as Service or Product), `description`,
  `quantity`, `unit_price`, `amount`, `taxed`, `taxed2` and an optional `project {id, name}`.
- `GET /v2/invoices/{id}/payments` lists payments. Not read (R-9).
- A time entry has `is_billed` and, when billed, `invoice {id, number}`. Line items do not point to
  entries.

The Harvest client crate (`crates/harvest/src/v2.rs`) gains `ApiInvoice`, `ApiInvoiceLine` and the
`invoice` field on `ApiTimeEntry`, with the same `serde_json::Number` decimals the importer already
reads exactly.

## R-2 Where imported invoices live

**Decision**: imported invoices are rows of `invoices` with `source = 'harvest'`; their Harvest-only
values sit in a one-to-one table `harvest_invoices`; their lines go in a new table
`harvest_invoice_lines`, not in `invoice_line_items`.

**Rationale**: every surface that lists invoices, the `time_entries.invoice_id` foreign key, the
unique number and the last-invoice default already read `invoices`, so an imported invoice must be a
row there. Horae's own lines carry invariants that Harvest lines cannot meet: each line is exactly a
time entry or a fee occurrence, amounts are non-negative, discount is allocated per line, and fee
balances sum fee lines. A separate line table keeps those invariants and their code untouched.

**Alternatives**: one more line source in `invoice_line_items` (relaxes five constraints and every
query that assumes two sources); a fully separate `imported_invoices` table (every list, report and
the entry foreign key would need a union).

## R-3 Amounts and the exact checks on `invoices`

**Decision**: an imported invoice stores Harvest's subtotal, discount, tax and total amounts as
integer minor units. The three exact-rounding checks (`invoice_discount_exact`,
`invoice_tax1_exact`, `invoice_tax2_exact`) apply only to `source = 'horae'`. Its `discount_bps`,
`tax1_bps`, `tax2_bps` and `tax2_name` stay 0 and NULL; the rates are kept as exact decimals in
`harvest_invoices` for display. A new check keeps `subtotal = total + discount − tax1 − tax2` (already
generated) and the import checks that the lines sum to the subtotal (FR-005) in `horae-core`.

**Rationale**: Harvest taxes only the lines flagged `taxed`/`taxed2` and rounds its own way, so
Horae's formula (rate × (subtotal − discount), half up) would refuse real invoices. Recomputing would
change what the client was billed. Rates such as 8.875% do not fit basis points.

**Alternatives**: store the rates as basis points and refuse invoices whose rates have more than two
decimals (loses real invoices); recompute amounts (changes billed amounts).

## R-4 Converting decimals

**Decision**: a new pure function in `horae-core` (`importers::harvest::convert::money_exact`)
converts Harvest's decimal text to minor units of a given currency, refusing more decimals than the
currency has and allowing a sign (credit lines). The existing `money_to_cents` rounds half up, assumes
two decimals and refuses negatives, so it is wrong here. The currency's decimals come from the table
`horae_core::money` already uses. Quantities and rates are stored as PostgreSQL `numeric` (exact
decimal, never a float) and are display-only.

## R-5 Status mapping and Closed

**Decision** (Q1, Q2, decided 2026-11-22): open → `sent`, paid → `paid`, closed → new `closed`; drafts are read
but not imported. `invoice_status` gains `closed`; `horae_core::types::InvoiceStatus` gains `Closed`.
`set_status` accepts Sent → Paid, Sent → Closed and Sent → Void for imported invoices, and keeps its
current transitions for Horae invoices (Closed is not offered for them in this feature).

**Rationale**: closed in Harvest means written off: nothing more is due and its time stays billed.
Void in Horae reopens the time, which would put it back on Uninvoiced; Paid would overstate revenue.
Drafts are work in progress that, after a migration, is finished in Horae; leaving their time open
lets the administrator bill it from Horae.

**Alternatives**: import drafts as read-only drafts (blocks their time behind an invoice nobody can
edit or send); closed → void keeping its time (Void's meaning would depend on the source).

## R-6 Linking entries

**Decision**: invoices are applied after the catalog (clients, projects, tasks, people) and before
time entries in the same run. The run cache maps each Harvest invoice id to one of: an imported Horae
invoice, a skipped draft, or a failure. Each entry row carries `harvest_invoice_id`; after the entry
is created or matched, `apply_row` sets the link in the same savepoint:

| Harvest says | Horae entry now | Result |
| --- | --- | --- |
| billed on imported invoice I | not linked, open or approved | `invoice_id = I`, `invoiced`, rounding frozen; linked |
| billed on imported invoice I | linked to I | nothing; skipped |
| billed on imported invoice I | linked to another imported invoice | relinked to I; linked |
| billed on imported invoice I, which was voided in Horae | open | nothing; Horae's void stands until Harvest changes I |
| billed on any invoice | linked to a Horae-made invoice | kept; conflict |
| billed on a skipped draft | anything not Horae-linked | open, no invoice |
| billed, invoice failed or not listed | not Horae-linked | `invoiced`, no invoice; billed without invoice |
| not billed | import-linked or invoiced without invoice | open, no invoice, rounding cleared; unlinked |
| not billed | open or Horae-linked | nothing |
| billed | submitted (someone submitted it in Horae) | nothing; conflict, since `can_transition` forbids Submitted → Invoiced |

The decision table is a pure function in `horae-core` (`importers::harvest::link`), tested there; the
server only reads the inputs and writes its result.

"Import-linked" means linked to an invoice with `source = 'harvest'`, or invoiced with no invoice:
Horae itself never makes the second. No new column is needed to tell import links from Horae links.

Rounding is frozen with the same `effective_minutes` rule generation uses, so reports show the same
minutes before and after.

## R-7 Incremental runs and earlier imports

Entries imported before this feature have no links, and an incremental run only fetches entries
Harvest changed. **Decision**: the watermark gains an `invoice` key. When it is absent (the first run
with invoices), the run reads invoices and time entries in full whatever scope was asked, and the
report says so. Later incremental runs read invoices and entries changed since their own watermarks.
This relies on Harvest moving an entry's `updated_at` when it is billed or unbilled (assumption in
the spec; the quickstart checks it against the test account).

## R-8 Numbering

Harvest numbers are kept (FR-015). `generate.rs` numbers new invoices `INV-YYYYMM-NNN` from a count
of numbers with that prefix, which collides once an imported number matches the pattern or a number
is skipped. **Decision**: generation takes the highest numeric suffix for the prefix plus one, and
still retries on the unique key. An imported number already used by a Horae-made invoice is an
invoice error; one used by another imported invoice cannot happen (Harvest numbers are unique per
account, and provenance updates the same row).

## R-9 Payments

**Decision**: not imported as records (spec Assumptions, Q6). `harvest_invoices` stores `due_cents`
and `paid_on`, which are enough to show paid and due amounts now and to create one payment record per
invoice when Invoices step 3 adds payments. Reading every invoice's payments would add one request
per paid invoice for data no screen shows yet.

## R-10 What "billed" means elsewhere

Today `invoice_id IS NOT NULL` and `state = 'invoiced'` are equivalent: generation sets both, voiding
clears both, and the Harvest API's write sets both. FR-011 entries break that. **Decision**: the
places that read "billed" from `invoice_id` read `state = 'invoiced'` instead:

- `crates/harvest-api/src/lib.rs` (`is_billed`, and the billable expression of the two entry reads),
- `crates/server/src/ops/reports.rs`, `ops/time_reads/reports.rs`, `ops/time_reads/reports/groups.rs`,
- `crates/api/src/exports/limits/time.rs`, `limits/time/groups.rs`,
  `exports/streaming/cursor/time.rs`, `cursor/groups.rs`,
- `crates/server/src/ops/projects/reads.rs` and `ops/projects/budgets.rs` (billable expression only;
  their void-invoice joins are unchanged).

`ops/invoices/entries.rs`, `generate.rs` and PR-I1's `uninvoiced.rs` already require `state IN ('open', 'approved')`, so they need no change for FR-011. The Harvest API's `invoice` object on an
entry stays absent for entries with no invoice.

## R-11 Last invoice and summaries (PR-I1)

`uninvoiced.rs` computes a client's last invoice's `covered_to` from its lines' time entries. Imported
invoices have no such lines. **Decision**: `covered_to` also takes `max(spent_date)` of entries whose
`invoice_id` is the invoice (FR-025). PR-I1's list summary (outstanding, overdue) uses the total of
sent invoices; for imported invoices it uses `due_cents` (a partly paid invoice is outstanding only
for what is due). Closed invoices are neither outstanding nor overdue.

## R-12 Read only, PDF and UI

- `ops::invoices` refuses the draft editor, Duplicate and any line change for `source = 'harvest'`,
  with a named conflict error.
- The detail and PDF show the imported lines (kind, description, quantity, unit price, amount) and a
  breakdown built from Harvest's rates as written and the stored amounts, plus paid and due.
- The detail marks the invoice as imported from Harvest. Interface text goes in the catalog
  (`crates/i18n/locales/en/horae.ftl`); none of it copies Harvest's wording.

## R-13 CSV

Harvest's detailed time CSV has an "Invoiced?" column and no invoice id or number. **Decision** (Q5):
CSV rows with "Invoiced?" yes become invoiced with no invoice, counted as billed without an imported
invoice. A later API import can link them (R-6, "billed on imported invoice I" over "invoiced, no
invoice" is a link, not a conflict).

## R-14 Mutation path

The import job writes invoices, lines and links directly, as it writes every other imported record:
an administrator authorizes the job through the `start_harvest_*` server functions (004, 005, 019).
Status changes on imported invoices go through `ops::invoices::set_status`. No new exception to
Principle IV.

## R-15 Plugin events

Imports raise no invoice events (FR-023): the invoices were sent and paid in Harvest, and a plugin
that emails on `InvoiceSent` must not send hundreds of old invoices. A Sent → Closed change made in
Horae raises no event either; there is no closed event today, and adding one is outside this
feature.

## R-16 Performance

An account of 2,000 invoices with about 5 lines each is 10,000 line rows and 20 Harvest pages. Lines
are inserted in one statement per invoice. Linking adds one indexed update per billed entry inside
the savepoint the entry already has. The spec's bound (SC-006, at most a fifth more time) is checked
with the existing scale fixture extended with invoices.
