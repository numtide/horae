# Feature Specification: Invoices in the public API

**Feature Branch**: `feat/api-invoices`

**Created**: 2026-11-25

**Status**: Draft, for the maintainer's review

**Input**: User description: "Add invoices to the personal-access-token API: list them (filters:
status, updated since, pagination as the other lists), get one, and export one as PDF, checked
against the same permissions as the web app. The use is an accounting system that imports the
invoices a person has issued."

Builds on [022 Public API v1](../022-public-api-v1/spec.md), which left invoices out of v1 and
named them a later, backward-compatible addition (022 FR-031, FR-050). The OAuth scopes of
[024 OAuth with PKCE](../024-oauth-pkce/spec.md) do not cover invoices; this feature adds no scope.

## Background

A freelancer or a small company bills its clients from Horae and keeps its books somewhere else.
Today the books learn about an invoice when someone downloads its PDF from the invoice page and
uploads it by hand, and types its number, client, dates and amounts again. A personal access token
can already read time and the catalogue; it cannot read invoices.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - The books import issued invoices (Priority: P1)

The person who issues invoices creates a personal access token named after their accounting
system. Every few minutes the accounting system asks Horae for the invoices that changed since its
last run, takes the ones that have been sent, paid or written off, and for each one stores its PDF
with its number, client, currency, amounts and dates. An invoice it has seen before changes
nothing in the books when it comes back.

**Why this priority**: it is the request. Everything else in this feature serves it.

**Independent Test**: as an administrator, issue an invoice in the web app, then with a token list
the invoices, read the one issued and download its PDF; compare the figures with the invoice page
and the bytes with the web app's PDF download.

**Acceptance Scenarios**:

1. **Given** a token of a person who may view all invoices, **When** the client lists invoices,
   **Then** it gets every invoice of the organization in a stable order, each with its id, number,
   status, client, subject, issue and due dates, currency, subtotal, discount, taxes, total,
   purchase order, notes, whether it is overdue, where it was made, and when it was created and
   last changed.
1. **Given** the same token, **When** the client reads one invoice, **Then** it also gets the
   invoice's lines, each with its description and amount, and for billed time its minutes, rate
   and time entry.
1. **Given** the same token, **When** the client asks for an invoice's PDF, **Then** it gets the
   same document the web app's download gives for that invoice, named after its number.
1. **Given** an invoice that has not changed, **When** its PDF is asked for twice, **Then** the two
   answers are byte for byte the same, so a store that keeps documents by content keeps one.

______________________________________________________________________

### User Story 2 - An incremental sync sees what changed (Priority: P1)

The accounting system remembers when its last run started. On the next run it asks only for
invoices changed since then: a draft that was sent, an invoice that was paid or voided, a draft
that was edited. It narrows the list to one status or one client when it only needs those.

**Why this priority**: without it, every run reads every invoice, and a paid invoice cannot be
told from an unchanged one.

**Independent Test**: list with `updated_since` set to a moment, mark one invoice paid, list again
from the same moment and see exactly that invoice.

**Acceptance Scenarios**:

1. **Given** an invoice whose status, dates, amounts, texts or client change, **When** the list is
   asked for invoices updated since a moment before the change, **Then** the invoice is in it.
1. **Given** a save that changes nothing, **When** the same list is asked for, **Then** the
   invoice's update time has not moved.
1. **Given** a list longer than a page, **When** the client follows the cursor, **Then** it gets
   every invoice once, and a cursor used with other filters is refused.

______________________________________________________________________

### User Story 3 - Only those who may see invoices read them (Priority: P1)

A member who may not view invoices in the web app creates a token. Their script cannot list, read
or download any invoice through the API either. An app the person authorized with OAuth cannot
read invoices, whatever scopes it holds.

**Why this priority**: invoices disclose clients, rates and amounts. The API must not widen who
sees them (022 FR-003, FR-021).

**Independent Test**: with a member's token, ask for the list, an invoice and its PDF; with an
administrator's OAuth token holding every scope, ask again.

**Acceptance Scenarios**:

1. **Given** a token whose owner may not view all invoices, **When** it asks for any invoice
   operation, **Then** it is refused with the reason the web app gives.
1. **Given** an invoice of another organization or an unknown id, **When** it is read or
   downloaded, **Then** the answer is not found.
1. **Given** an OAuth access token, **When** it asks for any invoice operation, **Then** it is
   refused as not covered by its scopes.

______________________________________________________________________

### Edge Cases

- **Invoices imported from Harvest** (spec 025) are listed like Horae's own, marked as imported.
  Their lines carry Harvest's quantity and unit price as decimals; the API gives only each line's
  amount, as no decimal amounts appear in v1 (022 FR-042).
- **Drafts and voided invoices** are listed with their status. A client that wants issued
  invoices filters by status or skips them.
- **Written off** (`closed`) exists only for imported invoices.
- **A PDF that is too large or too slow** to make, or made while too many others are, is refused
  with the export's own limits, as a problem with its own code.
- **An unknown status value** in the filter is refused as a bad request (022, *Unknown query
  parameters*).
- **Money** is in hundredths with the invoice's currency, as every amount of v1.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: `/api/v1` MUST list the organization's invoices, in `id` order, paginated with the
  signed cursor and the page size of the other lists (022 FR-045), filtered by status, client and
  "updated since" (022 FR-046).
- **FR-002**: `/api/v1` MUST read one invoice with its lines.
- **FR-003**: `/api/v1` MUST answer an invoice's PDF, made by the same code and with the same
  limits as the web app's download, and refused the same way.
- **FR-004**: The three operations MUST require exactly what the web app's invoice pages and
  downloads require (the whole-invoice read grant), read through the shared operations, and
  answer an invoice outside the organization as not found (022 FR-020, FR-021).
- **FR-005**: No OAuth scope covers invoices. OAuth access tokens MUST be refused on the three
  operations with `insufficient_scope` (024 FR-062). A scope for invoices is a later addition.
- **FR-006**: Invoices MUST carry a real update time that moves when the invoice's own values
  change (status, dates, amounts, texts, client) and not on a save that changes nothing.
  Existing invoices take their creation time.
- **FR-007**: The new operations, parameters and types are additions to v1: no existing operation,
  field or meaning changes (022 FR-050), and the published document describes them (022 FR-047).

### Key Entities

- **Invoice (public)**: id, number, status, source, client id and name, subject, issue and due
  dates, overdue, currency, subtotal, discount, tax, second tax name and amount, total, purchase
  order, notes, created and updated times.
- **Invoice line (public)**: description, amount, and for billed time its minutes, hourly rate and
  time entry.

## Success Criteria *(mandatory)*

- **SC-001**: An accounting system imports every invoice issued in Horae with no manual typing of
  its figures.
- **SC-002**: A person who cannot see invoices in the web app gets none through the API, and no
  OAuth token gets any (checked by tests).
- **SC-003**: An unchanged invoice's PDF is byte-identical between requests (checked by tests).
- **SC-004**: Every new answer conforms to the published document, and the contract check finds
  only additions.

## Assumptions

- Which invoices a project manager manages is still open (015, T006); until it is decided the
  managed grants authorize no invoice read, in the web app and here alike.
- A line added or removed without changing the invoice's amounts does not move its update time;
  editing a draft's lines changes its amounts in practice.

## Out of scope

- Writing invoices over the API (creating, editing, sending, paying, voiding).
- An OAuth scope for invoices.
- The CSV and XLSX exports over the API.
- Harvest's decimal quantities and unit prices of imported lines.
