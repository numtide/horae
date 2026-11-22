# Quickstart: validating the Harvest invoice import

## Prerequisites

- The dev stack: `process-compose up` (PostgreSQL, migrations, the app on :8080).
- For automated checks, a database role with `CREATEDB` and `DATABASE_URL` set inside
  `nix develop -c env DATABASE_URL=… …` (the shell re-exports its own URL otherwise).
- For the manual check against Harvest: the test (evidence) account connected on the Harvest import
  screen, with the user's authorization for this task recorded in `specs/harvest-parity.md`. Never
  the production account, and never create, edit or delete Harvest data outside disposable fixtures
  tracked by id.

## 1. Domain rules

```sh
cargo test -p horae-core importers::harvest
```

Expect passing tests for: exact conversion per currency (refusing `10.005` in EUR and `10.5` in JPY,
accepting `-25.00`), the adds-up check (a one-cent difference is reported with its size), the state
mapping (draft → skip, unknown → error), and every row of the link table (research R-6) including the
property that `decide` never links an entry on a Horae-made invoice.

## 2. Importer against the Harvest test server

```sh
cargo test -p horae-server importers::harvest::invoices
cargo test -p horae-server importers::harvest::sync_tests
```

The fixture has: three open/paid/closed invoices, one draft, one partly paid invoice, one invoice with
per-line taxes and a credit line, one with a number a Horae invoice already uses, one whose amounts do
not add up, and entries billed on each, plus an entry billed on an invoice the list does not return.
Expect after a commit:

- invoices: created 5, errored 2, drafts skipped 1;
- entries on imported invoices are `invoiced` and linked; entries on the draft are `open`; entries on
  the two failed invoices and the missing one are `invoiced` with no invoice;
- the preview before it reports the same counts and leaves every table unchanged;
- a second identical run counts every invoice skipped and links nothing;
- changing the fixture's paid state and unbilling one entry, then running incrementally, counts one
  invoice updated and one entry unlinked.

## 3. Billed means invoiced

```sh
cargo test -p horae-harvest-api is_billed
cargo test -p horae-api exports
DATABASE_URL=… cargo test -p horae --features server invoices
```

Expect `is_billed: true` for an entry invoiced with no invoice; Uninvoiced and invoice previews
exclude it; editing, duplicating or saving a draft of an imported invoice is refused; Sent → Closed is
accepted for an imported invoice and refused for a Horae one; a new Horae invoice number skips one
taken by an import.

## 4. In the app

1. Sign in as Admin, open the Harvest import, run a preview, then commit it.
1. The run page shows the invoice counts and link counts ([contracts/import-report.md](contracts/import-report.md)).
1. Invoices lists the imported invoices with their Harvest numbers. Open a partly paid one: its lines,
   rates, Paid and Due match Harvest's; the editor and Duplicate are not offered; download its PDF
   and compare the amounts.
1. Uninvoiced shows, for a client, the same unbilled time as Harvest's own uninvoiced view for the
   same period (SC-001).
1. Start a new invoice for a client with imported invoices: the period starts after the last imported
   invoice's time.

## 5. Assumption check against the test account

Before relying on incremental runs (research R-7), confirm in the test account that billing and
unbilling an entry moves its `updated_at`: create a disposable invoice for one disposable entry,
read the entry through the API before and after, then delete both and record their ids in
`specs/harvest-parity.md`.

## 6. Gate

```sh
nix fmt
nix flake check
```
