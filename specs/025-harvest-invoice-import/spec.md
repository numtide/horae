# Feature Specification: Import Harvest invoices and link the time they billed

**Feature Branch**: `spec/harvest-invoice-import` (documents); implementation on its own branch after
the Invoices screens (PR-I1, branch `design/invoices-ui`) merge

**Created**: 2026-11-22

**Status**: Draft

**Input**: User description: "Import a Harvest account's invoices into Horae and link each imported
time entry Harvest billed to its imported invoice, so Horae never offers to bill again time that
Harvest already billed."

Amends [004 Harvest Importer](../004-harvest-importer/spec.md) FR-016 ("never `invoiced` from
Harvest"), which this feature replaces for the API source and for the CSV source.
Builds on 005 (durable jobs), 006 (jobs CLI), 007 (account switch), 008 (import UX) and 019 (people,
the Harvest section).

## Background

The importer brings clients, projects, tasks, people and time entries. It creates every entry as
`open`, ignores Harvest's billed flag and never creates an invoice. The Invoices screens (PR-I1)
start a new invoice's period after the client's last invoice and list each client's unbilled time on
Uninvoiced. After an import, years of time Harvest already billed appear there as unbilled, the last
invoice is missing, and preparing an invoice from the earliest date would bill it a second time.

Harvest's public API gives each invoice its client, number, state (draft, open, paid, closed), issue
and due dates, currency, purchase order, subject, notes, discount and two tax rates with their
amounts, the total and the amount still due, and its line items (kind, description, quantity, unit
price, amount, whether each tax applies, project). Payments are a separate list per invoice. Each
time entry says whether it is billed and, when it is, carries the id and number of its invoice.
Harvest's line items do not say which time entries they cover; only the entry points to its invoice.

## Clarifications

### Session 2026-11-22

- Q1: Import Harvest drafts, or skip them and leave their time open? → A: Skip; their time stays open
  (FR-001).
- Q2: Map Harvest's closed (written off) state to a new Closed status, to Void with its time kept
  invoiced, or to Paid? → A: A new Closed status, for imported invoices only (FR-002).
- Q3: Are imported invoices fully read only, or read only apart from Mark paid, Close and Void? → A:
  Read only apart from Mark paid, Close and Void (FR-016).
- Q4: Time Harvest billed on an invoice Horae could not import: invoiced with no invoice, or left
  open and reported? → A: Invoiced with no invoice, and reported (FR-011).
- Q5: Should the CSV import mark entries whose "Invoiced?" column says yes as invoiced with no
  invoice, changing 004's FR-016 for CSV too? → A: Yes (FR-022).
- Q6: Store payments as records now, or only the amount due and the paid date until Invoices step 3?
  → A: Only the amount due and the paid date (Assumptions).
- Q7: When a later run finds an entry no longer billed in Harvest, unlink and reopen it, or only
  report it? → A: Unlink and reopen, for links the import made only (FR-012).
- Q8: A Harvest number already used by a Horae invoice: report the Harvest invoice as an error, or
  import it under a suffixed number? → A: Error (FR-015).

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Billed history stays billed after an import (Priority: P1)

An administrator migrating from Harvest previews and commits an API import. Horae imports the
account's invoices together with the clients, projects, tasks, people and time. Every imported time
entry that Harvest billed on an imported invoice is linked to that invoice and counts as invoiced.
Uninvoiced shows only what Harvest had not billed, and a new invoice for a client starts after the
client's last imported invoice.

**Why this priority**: Without it every migrated organization sees its whole history as unbilled,
and the first invoice prepared from Horae can bill a client twice. It is the reason for the feature.

**Independent Test**: Import a Harvest fixture with billed and unbilled entries across several
invoices. Check that billed entries are linked and invoiced, unbilled ones are open, Uninvoiced
lists exactly the unbilled time, and preparing an invoice offers none of the billed entries.

**Acceptance Scenarios**:

1. **Given** a Harvest entry billed on an invoice that the run imports, **When** the import commits,
   **Then** the entry is invoiced, linked to that imported invoice, and no Horae invoice preview,
   generation or Uninvoiced total includes it.
1. **Given** a Harvest entry that is not billed, **When** the import commits, **Then** the entry is
   open with no invoice, as today.
1. **Given** a client whose newest Harvest invoice was issued on 2026-09-30 and billed time up to
   2026-09-28, **When** an administrator starts a new invoice for that client, **Then** the default
   period starts after the time that invoice covered, the same as after a Horae invoice.
1. **Given** entries imported by an earlier run, before this feature existed, **When** the import
   runs again, **Then** the entries Harvest billed are linked to their invoices although the entries
   themselves are unchanged and counted as skipped.
1. **Given** a preview (dry run), **When** it completes, **Then** it reports the invoices it would
   create and update and the entries it would link, and nothing is written.

______________________________________________________________________

### User Story 2 - Imported invoices read like Horae's own (Priority: P1)

After the import, the Invoices list and each client's page show the imported invoices beside Horae's
own, with their Harvest number, status, dates, amounts and lines. The detail shows the lines as
Harvest had them, the discount, both taxes and the total, and how much was paid. The PDF of an
imported invoice renders from the same data.

**Why this priority**: Outstanding Harvest invoices still have to be chased after the migration, and
an administrator needs the history to answer a client's question without going back to Harvest.

**Independent Test**: Import a fixture invoice with two service lines, one product line, a discount,
two taxes on some lines only and a partial payment. Compare every amount on the detail and in the PDF
with the fixture.

**Acceptance Scenarios**:

1. **Given** an imported invoice, **When** an administrator opens it, **Then** it shows Harvest's
   number, client, subject, purchase order, notes, issue and due dates, currency, each line's
   description, quantity, unit price and amount, the subtotal, discount, each tax and the total, all
   exactly as Harvest stated them, and marks the invoice as imported from Harvest.
1. **Given** an imported invoice with a total of 1,000.00 and 400.00 still due, **When** it is shown,
   **Then** its status is Sent and it shows 600.00 paid and 400.00 due.
1. **Given** an imported invoice, **When** its PDF is downloaded, **Then** it contains the same lines
   and amounts as the detail.
1. **Given** an imported invoice, **When** an administrator opens the draft editor, duplicates it or
   edits its lines, **Then** Horae refuses (Q3): imported invoices are read only, apart from the
   status changes in FR-016.

______________________________________________________________________

### User Story 3 - Re-imports keep the invoices current (Priority: P2)

The administrator keeps Harvest running for a while and imports again, incrementally or fully. A
Harvest invoice paid since the last run becomes Paid in Horae; one written off becomes Closed; time
billed on a new Harvest invoice is linked to it. Nothing changes twice, and invoices made in Horae are
never touched.

**Why this priority**: Migrations overlap, and the import already supports repeated and incremental
runs. Invoices that freeze at the first run would drift from Harvest within weeks.

**Independent Test**: Import a fixture, change an invoice's state and payments, add a new invoice
billing two previously unbilled entries, and import again. Check the updates, the links, and that a
third identical run changes nothing.

**Acceptance Scenarios**:

1. **Given** an imported invoice that Harvest has since marked paid, **When** the import runs again,
   **Then** the Horae invoice is Paid and the report counts it as updated.
1. **Given** a run with no Harvest changes since the last one, **When** it commits, **Then** every
   invoice is counted as skipped and no invoice, line or link changes.
1. **Given** a new Harvest invoice that bills two entries imported earlier as open, **When** the
   import runs again, **Then** both entries are linked to the new imported invoice and invoiced.
1. **Given** an imported entry that Harvest no longer bills (its invoice was deleted in Harvest),
   **When** the import runs again, **Then** the entry is unlinked and open again, and the report
   counts it as unlinked.
1. **Given** an entry linked to an invoice made in Horae, **When** Harvest reports it billed on a
   Harvest invoice, **Then** the Horae link is kept and the entry is reported as billed in both.

______________________________________________________________________

### User Story 4 - The report explains what could not be imported (Priority: P3)

The import report counts invoices like the other levels (created, updated, skipped, errored) and adds
the entries it linked, unlinked, and marked billed without an imported invoice. Each invoice error
names the Harvest invoice number and the reason.

**Why this priority**: An invoice that fails leaves its time billed without a visible invoice; the
administrator needs to know which ones and why.

**Independent Test**: Import a fixture with an invoice whose number a Horae invoice already uses, an
invoice whose amounts do not add up, and a currency Horae does not know. Check the errors and the
state of their entries.

**Acceptance Scenarios**:

1. **Given** a Harvest invoice whose number a Horae-made invoice already uses, **When** the import
   runs, **Then** that invoice is an error naming the number, and its entries are invoiced without an
   invoice (FR-011), so they are still never offered for billing.
1. **Given** a Harvest invoice whose lines, discount, taxes and total do not add up exactly, **When**
   the import runs, **Then** that invoice is an error naming the difference, and nothing of it is
   written.
1. **Given** a run, **When** it finishes, **Then** the report shows invoice counts and the linked,
   unlinked and billed-without-invoice entry counts, in the preview and the commit alike.

### Edge Cases

- A Harvest draft invoice: not imported, and its time stays open so it can be billed from Horae (Q1).
- A Harvest invoice in the closed state (written off): imported as Closed; its time stays invoiced
  (Q2).
- An entry billed in Harvest whose invoice failed to import or is no longer listed: the entry is
  invoiced with no invoice and counted (FR-011). Time on a skipped draft is not this case.
- An invoice for a client that has no time entries: the client is imported as a catalog record first,
  as clients without time already are.
- An invoice in a currency other than its client's: kept in the invoice's own currency.
- An invoice with no lines, or lines whose amounts are negative (credits): imported when the
  invoice's total is zero or more and its amounts add up; an invoice with a negative total is an
  error.
- Lines for expenses, products or any kind Horae has no source for: imported as imported lines with
  their kind as text; they are not expenses in Horae and do not consume project fees.
- A tax applied to some lines only: Harvest's tax amounts are kept as stated; Horae does not
  recompute them.
- A tax or discount rate with more decimals than two: kept for display exactly as Harvest wrote it.
- An amount with more decimals than the currency allows: the invoice is an error, never rounded.
- An entry billed on an invoice for another client than the entry's project's client (projects moved
  between clients in Harvest): linked as Harvest says.
- A Horae-made invoice voided after the import: its entries reopen as today; imported links are
  unaffected.
- An imported invoice voided in Horae: its entries reopen, and a later import does not link them to
  it again while Harvest has not changed that invoice (FR-016).
- A running timer that Harvest reports billed: not possible in Harvest; the timer rule (skip running
  entries) applies first anyway.
- Two runs at once: prevented by the existing import lock.
- An entry that was edited in Horae after the first import and that Harvest later bills: linked; the
  link freezes it as a Horae invoice would.

## Requirements *(mandatory)*

### Functional Requirements

**Which invoices**

- **FR-001**: An API import MUST read every Harvest invoice of the connected account, of every state,
  and import those in the open, paid and closed states. Draft invoices MUST NOT be imported, and the
  time Harvest attached to a draft MUST stay open in Horae.
- **FR-002**: Harvest's states MUST map as open → Sent, paid → Paid, closed → Closed, where Closed is
  a new invoice status meaning written off: no longer due, not paid, and its time stays invoiced.
  Closed MUST exist only for imported invoices in this feature.
- **FR-003**: An incremental import MUST read the invoices Harvest changed since the last run, with
  the same watermark as the other records; a full import reads them all.

**Amounts**

- **FR-004**: Every amount MUST be converted from Harvest's decimal to integer minor units of the
  invoice's currency with exact integer arithmetic. An amount with more decimals than the currency
  allows MUST make the invoice an error, never be rounded.
- **FR-005**: An imported invoice MUST store Harvest's own subtotal, discount amount, first and second
  tax amounts, total and amount due, and MUST NOT recompute them from rates. Before writing, the
  import MUST check that the lines' amounts sum to the subtotal and that subtotal − discount + taxes =
  total, exactly; a mismatch MUST make the invoice an error.
- **FR-006**: The discount and tax rates MUST be kept exactly as Harvest wrote them, for display only,
  and never used in a calculation.
- **FR-007**: An invoice in a currency Horae does not recognize MUST be an error.

**Lines**

- **FR-008**: Each Harvest line item MUST become one imported line holding its kind, description,
  quantity, unit price, amount, whether each tax applied, and its project when Harvest names one.
  Imported lines MUST NOT reference a time entry or a fee occurrence, MUST NOT count toward a
  project's agreed fee balance, and MAY have a negative amount.

**Linking time**

- **FR-009**: Each imported time entry that Harvest reports billed on an imported invoice MUST be
  linked to that invoice and set to invoiced, with its rounding frozen as a Horae invoice freezes it.
  This MUST apply to entries created by the run and to entries an earlier run created.
- **FR-010**: An entry already linked to an invoice made in Horae, or submitted for approval in
  Horae, MUST keep its state and link; the conflict MUST be counted and listed in the report.
- **FR-011**: An entry Harvest reports billed whose invoice is not imported (an invoice error, or an
  invoice Harvest no longer lists) MUST be set to invoiced with no invoice, so Horae never offers it
  for billing, and MUST be counted as billed without an imported invoice. Time on a skipped draft is
  the exception in FR-001.
- **FR-012**: An entry linked by an earlier import, or marked billed without an invoice by one, that
  Harvest now reports not billed MUST be unlinked and set to open, and counted as unlinked. Entries
  linked to invoices made in Horae MUST NOT be touched.

**Identity and re-imports**

- **FR-013**: Each imported invoice MUST record its Harvest id as provenance, so a later run updates
  the same Horae invoice instead of creating another. An unchanged invoice MUST be counted as skipped
  and MUST NOT be rewritten.
- **FR-014**: A later run MUST update an imported invoice's state, amounts due, dates, texts and lines
  to Harvest's. An imported invoice that Harvest no longer lists MUST be left as it is and reported.
  Invoices made in Horae MUST never be changed by an import.
- **FR-015**: An imported invoice MUST keep Harvest's number. A Harvest number that a Horae-made
  invoice already uses MUST make that Harvest invoice an error. Horae's own numbering MUST skip every
  number already taken, imported or not.

**What an administrator can do with an imported invoice**

- **FR-016**: Imported invoices MUST be read only: no draft editing, no line changes, no Duplicate.
  The only status changes allowed are Sent → Paid, Sent → Closed and Sent → Void; voiding an imported
  invoice reopens its time as voiding a Horae invoice does. A status change made in Horae MUST be
  kept by later imports only until Harvest changes the invoice again (Harvest stays the source while
  it is in use). Marking an imported invoice Paid MUST set its amount due to zero and its paid date
  to the organization's today.
- **FR-017**: An imported invoice MUST render to PDF with the same template as Horae's invoices,
  showing its imported lines, Harvest's rates as written and its stored amounts.

**Preview, report, permissions and other sources**

- **FR-018**: The preview MUST report the invoice counts (created, updated, skipped, errored) and the
  linked, unlinked, billed-without-invoice and conflicting entry counts that the commit would
  produce, and MUST write nothing.
- **FR-019**: Each invoice error MUST name the Harvest invoice number and id and the reason, and MUST
  appear in the run's error list and error download beside the entry errors.
- **FR-020**: Starting, previewing and committing an import MUST stay limited to administrators, as
  today. Seeing imported invoices MUST need the same invoice permissions as seeing any invoice.
- **FR-021**: The account switch MUST treat imported invoices as provenance of the connected account,
  so an organization with imported invoices cannot switch accounts, as with the other imported
  records.
- **FR-022**: A CSV import MUST NOT create invoices (Harvest's time CSV carries no invoice data). An
  entry whose "Invoiced?" column says yes MUST be set to invoiced with no invoice and counted as
  billed without an imported invoice (FR-011).
- **FR-023**: An import MUST NOT raise the invoice sent, paid or voided plugin events for imported
  invoices.

**What changes elsewhere**

- **FR-024**: Uninvoiced, invoice previews and generation, and the reports' billed and unbilled
  figures MUST treat linked entries and entries invoiced without an invoice as billed, and imported
  invoices as invoices of their client, including for the "since the last invoice" default.
- **FR-025**: The time an imported invoice covered, for the "since the last invoice" default, MUST be
  the latest date of the entries linked to it, or its issue date when none is linked.

### Key Entities

- **Imported invoice**: an invoice whose source is Harvest. It has everything a Horae invoice has
  (client, number, status, dates, currency, purchase order, subject, notes, amounts) plus its Harvest
  id, the amount still due, the paid date when Harvest gives one, and the discount and tax rates as
  Harvest wrote them.
- **Imported line**: a line of an imported invoice: kind, description, quantity, unit price, amount,
  the two tax flags and an optional project. It has no time entry or fee occurrence.
- **Invoice provenance**: the link between a Harvest invoice id and the Horae invoice, with Harvest's
  last update time, as for the other imported records.
- **Billed time entry**: an entry linked to an invoice, or invoiced with no invoice when Harvest billed
  it on an invoice Horae does not have.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: After importing an account, the time Uninvoiced lists for each client equals the time
  Harvest reports unbilled for that client, to the minute.
- **SC-002**: No entry Harvest reports billed can be selected in a Horae invoice preview after the
  import.
- **SC-003**: Every imported invoice's subtotal, discount, taxes, total and amount due equal Harvest's
  to the minor unit, and its lines sum to its subtotal.
- **SC-004**: A second import with no Harvest changes creates, updates and links nothing.
- **SC-005**: The preview's invoice and link counts equal those of the commit that follows it.
- **SC-006**: Importing an account with 2,000 invoices and 15,000 entries adds no more than a fifth
  to the import's run time measured without invoices.

## Assumptions

- Harvest's time entries carry the id of their invoice when billed, and invoicing or uninvoicing an
  entry moves its update time, so incremental runs see the change.
- Payments are not imported as records. The amount still due and the paid date are stored on the
  imported invoice, which is what Invoices step 3 needs to create a payment record from it later.
- The importer keeps writing through its own job path, authorized when an administrator starts the
  job, as 004, 005 and 019 do.
- Estimates, retainers, recurring invoice schedules, expenses and invoice messages are out of scope.
- The implementation lands after the Invoices screens (PR-I1) merge; it builds on their Uninvoiced
  view, last-invoice default and Subject field.
