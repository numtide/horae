# Feature Specification: Expense Tracking and Categories

**Feature Branch**: `feat/expense-parity`

**Created**: 2026-10-01

**Status**: Draft — reference investigation and shared contracts incomplete

**Input**: Complete Harvest web parity, including expenses, using Horae's design language; specify before implementation. Native applications and new integrations are excluded.

## Clarifications

### Session 2026-10-01

- Q: May screens without a dedicated mockup be composed from Horae's existing design system, preserving Harvest behavior? → A: Yes, authorized. Reuse current components and tokens; a new handoff is not a prerequisite for these surfaces.
- Q: May reference checks create, modify and delete isolated test records in Harvest? → A: Yes, authorized. Limit them to disposable clients/projects, categories, expenses, receipts and draft invoices. Preserve existing records; no messages, payments, seat purchases or owner changes.

## User Scenarios & Testing

### User Story 1 - Record and correct a project expense (Priority: P1)

A teammate records a dated expense against permitted work and can correct it without losing the original entry on failure.

**Why this priority**: Expenses must be captured before approval, reporting or billing can use them.

**Independent Test**: Create, reopen and correct an expense; verify cancel and failed save leave stored records unchanged.

**Acceptance Scenarios**:

1. **Given** available work and a category, **When** a person saves a date and amount, **Then** the entry retains its project, category, owner and optional note/receipt.
1. **Given** invalid input or a failed request, **When** saving, **Then** field errors are actionable, the draft remains available and no success is claimed.
1. **Given** a person outside the authorized scope, **When** opening an expense or its receipt directly, **Then** no private content or identifying metadata is disclosed.
1. **Given** a EUR client and a USD project override, **When** an ordinary expense of `1.235` is saved, **Then** the expense uses USD 1.24; a correction to `-1.235` persists USD -1.24 without changing the client's currency.
1. **Given** an existing receipt, **When** a replacement at or above 10,485,760 bytes fails, **Then** the prior receipt and expense fields remain unchanged; a valid PDF of 10,485,759 bytes is accepted.
1. **Given** an existing receipt, **When** removal is selected and cancelled, **Then** the receipt remains; selecting removal and saving instead removes its internal download route without deleting the expense.

### User Story 2 - Maintain expense categories (Priority: P1)

An authorized administrator maintains categories without erasing historical expenses.

**Why this priority**: Category lifecycle and rate changes affect the correctness of every subsequent expense.

**Independent Test**: Exercise rename, rate change, archive, restore and attempted deletion using an existing entry.

**Acceptance Scenarios**:

1. **Given** a category with entries, **When** archived, **Then** history remains and new entries cannot use it; eligible existing entries remain editable.
1. **Given** a category in use, **When** deletion is attempted, **Then** it is refused without deleting expenses.
1. **Given** a unit-priced entry, **When** the category rate changes, **Then** its recorded amount remains until an authorized resave applies the current rate.
1. **Given** a USD project and unit rate `0.125`, **When** quantity `1.235` is saved, **Then** quantity is first rounded to `1.24` and its total to USD `0.16`; `-1.235` yields `-1.24` and USD `-0.16`.
1. **Given** quantity 2 saved at rate `0.125`, **When** the rate becomes `0.5`, **Then** the old USD `0.25` remains until Edit and Save, even with no fields changed, applies USD `1.00`.

### User Story 3 - Review expenses across approval and billing (Priority: P1)

Reviewers and billing staff use the same expense identities without confusing approval, billing and reimbursement.

**Why this priority**: Combining these states would create incorrect balances or unauthorized edits.

**Independent Test**: Compare a mixed-project expense selection across approval, detailed reporting and invoice eligibility using the agreed shared contracts.

**Acceptance Scenarios**:

1. **Given** a non-billable expense, **When** preparing a client invoice, **Then** it is excluded from tracked-expense billing and uninvoiced value.
1. **Given** multiple independent locks, **When** one is removed, **Then** the other protections remain effective.
1. **Given** two currencies, **When** displaying totals, **Then** they remain separate and reconcile exactly to their constituent entries.
1. **Given** an expense incurred in a different currency from the expense form's applicable currency, **When** recording it, **Then** the user enters a manually converted amount; Horae does not fetch an exchange rate. The applicable-currency contract remains gated by FR-011.
1. **Given** an invoice containing tracked expenses, **When** its line amount is edited, **Then** the source amounts stay unchanged and their full billed status remains; invoice value and tracked value are not required to be equal.
1. **Given** a draft invoice with an attached expense report, **When** a tracked-expense line is removed and the invoice saved, **Then** the regenerated report excludes that line's expenses; recipient access to that report does not grant access to internal expense records.
1. **Given** a multi-page expense report with no rows checked, **When** choosing a billed/unbilled action, **Then** confirmation identifies only the current page's targets; cancellation changes nothing and Projects' empty-selection Actions remain disabled.

### Edge Cases

- Save races with permission revocation, archiving, approval, invoicing or a rate change.
- A replaced receipt fails to upload; an old download link is used after access revocation.
- Resubmission after a timeout must not duplicate the expense.
- Currency transitions, other currency exponents, locale and true numeric bounds still require reference fixtures. The tested USD precision/sign cases are recorded in FR-011/017.
- Deleting or changing an already invoiced expense must not silently rewrite a sent invoice or orphan its attribution.
- Expense-report bulk actions with no checked rows target the current page and require confirmation; they must not target other pages or change the confirmed disabled-empty Actions behavior on Projects.

## Requirements

### Functional Requirements

- **FR-001**: Provide expense capture, listing, correction and deletion for authorized users, plus category administration. Scope follows feature 015, not a hard-coded legacy Manager label.
- **FR-002**: Capture date, person, project, category and money amount or unit quantity; allow notes and a receipt. Default billability from the project while allowing the reference-supported override. [Tracking expenses](https://support.getharvest.com/hc/en-us/articles/360048687611-Tracking-expenses).
- **FR-003**: Support one receipt per expense, attached by picker or drag/drop, including PNG, GIF, PDF and JPEG, with replacement and explicit removal. Accept at most 10,485,759 bytes; reject 10,485,760 bytes or more without replacing the old receipt or partially saving accompanying field changes. Cancelling removal retains the receipt; saving removal makes its internal download route unavailable without deleting the expense. Internal receipt access MUST use the expense's current authorization; invoice-recipient artifacts follow FR-015 separately. Define content validation and retention behavior in the final contract; do not assume a public source-file URL is acceptable. [Persisted boundary checks](receipt-evidence.md).
- **FR-004**: Support ordinary and unit-priced categories, rename, rate changes, archive, restore and deletion only when unused. Renames affect historical labels; rate updates do not automatically reprice old entries, but resaving can. [Categories](https://support.getharvest.com/hc/en-us/articles/360048686731-Managing-expense-categories).
- **FR-005**: Keep approval status, billed status, invoice association and lock reasons distinct. Apply the shared submission/partial-approval contract from 015 to expenses without widening project/person/date scope. [Expense reference](https://help.getharvest.com/api-v2/expenses-api/expenses/expenses/).
- **FR-006**: Preserve ordinary own-entry editing and separately authorized editing for others. Support the documented privileged correction of locked notes/amounts and deletion, while preventing locked project/category reassignment. Resolve invoice consequences before accepting this flow. [Editing expenses](https://support.getharvest.com/hc/en-us/articles/4406054281101-How-to-edit-expenses).
- **FR-007**: Account for invoice date-range locks separately for time and expenses, alongside approval, archive and company-wide locks. A time-only invoice MUST NOT alone lock expense dates. Shared lock ownership belongs to 015 and the billing specification. [Locking](https://support.getharvest.com/hc/en-us/articles/360048687491-Unlocking-time-and-expenses).
- **FR-008**: Respect workspace expense-module and reimbursement preferences. Reimbursable and billable MUST remain independent; marking reimbursement MUST NOT imply transferring money. Feature 013 owns configuration. [Preferences](https://support.getharvest.com/hc/en-us/articles/360048179912-Customizing-account-preferences).
- **FR-009**: Provide detailed expense reporting with date/client/project/person/category filters, billing filters, authorized receipt access, printing and CSV/XLSX/PDF export, including supported receipt images in PDF and customizable CSV/XLSX columns. Preserve archived history; distinguish its inclusion in results from showing archived choices in filters. Shared saved/shared/recurring reporting belongs to the reporting package. [Detailed reports](https://support.getharvest.com/hc/en-us/articles/360048687171-Detailed-time-and-detailed-expense-reports).
- **FR-010**: Define billable-expense invoice selection, manual billed/unbilled marking and source attribution with the billing owner. Non-billable expenses MUST NOT be manually marked billed. Editing invoice description, quantity or unit price MUST NOT change its source expenses or partially release their billed status. Removing a saved tracked-expense line or deleting its invoice releases the associated billing status/lock, not independent approval or administrative protection. Free-form lines MUST NOT create source expenses. Do not equate manual billed status with an invoice. [Invoice editing](https://support.getharvest.com/hc/en-us/articles/360048181012-Editing-and-deleting-invoices-and-estimates) and detailed reports under FR-009.
- **FR-011**: Preserve exact currency-specific amounts and historical attribution. Prefer a project currency override ahead of client currency in the expense form, rows and weekly totals; the persisted EUR-client/USD-project fixture verifies that override for new expenses. Users manually convert foreign-currency receipts before recording amounts; do not add exchange-rate retrieval or automatic conversion. Category rates use the same numeric value in the project's applicable currency without conversion in the tested fixture. [Currency reference](https://support.getharvest.com/hc/en-us/articles/360055384512-Does-Harvest-support-multiple-currencies) and dated [persisted evidence](numeric-evidence.md). [NEEDS CLARIFICATION: verify currency exponents beyond USD, true numeric bounds and input locale, historical currency-change effects and invoice consequences of privileged source-expense correction/deletion.]
- **FR-012**: Validate permission, identity, project/category availability and business state at save/download time. Reject stale conflicting edits visibly without partial success. Logging MUST not expose receipt contents or private notes.
- **FR-013**: Supply keyboard-operable forms, labelled controls, explicit empty/loading/error/pending states and recoverable validation. Compose expense surfaces without dedicated mockups from Horae's existing design system, preserving verified Harvest behavior. Record the screen composition and control/state mapping during planning; do not change shared defaults or require a new handoff. Verify shared project/time/settings screens for regressions under SC-005.
- **FR-014**: Use feature 015's six expense read/write scopes: own, managed people/projects and organization-wide. Built-in defaults are own for Member, own plus managed for Project Manager, and organization-wide for People Admin, Accounting, Executive Manager and Administrator. Customization follows 015's Member floor and grant prerequisites. [NEEDS CLARIFICATION: finalize category, receipt, deletion, manual billing, approval and privileged locked-entry operation mapping with 015; observed ordinary read/write grants do not establish these lifecycle permissions.]
- **FR-015**: Support a client-facing expense report attached to an invoice containing expenses. Generate it on invoice save from the included expenses and regenerate an attached report when editing and saving a draft. Recipient access MUST remain distinct from internal source-expense access. Resolve sent-invoice and later source-edit/receipt-removal behavior with the billing owner before finalizing retention. [Invoice attachments](https://support.getharvest.com/hc/en-us/articles/9864825272589-Attaching-files-and-reports-to-invoices).
- **FR-016**: For expense-report billed/unbilled bulk actions, use explicitly checked entries, or the current report page when none are checked, matching the documented reference. Require a confirmation identifying the target set and preserve cancellation without changes. Do not extend the selection to unseen pages or change Projects' disabled-empty Actions behavior or shared defaults. Apply FR-010/012 to every target. [Detailed reports](https://support.getharvest.com/hc/en-us/articles/360048687171-Detailed-time-and-detailed-expense-reports).
- **FR-017**: Preserve the verified numeric rules: round unit quantities to two decimal places before multiplication; retain category rates to three decimal places; round USD expense totals to two decimal places. Tested ties round away from zero. Accept negative ordinary amounts and quantities; reject malformed numeric values and nonpositive category rates. Do not reject stored zero totals/quantities merely because the web input clears zero on blur and its required-field check blocks submission: preserve that observed UI/domain distinction. Rate changes do not automatically reprice historical entries; an authorized resave applies the current rate, even without changed fields. Exact arithmetic remains mandatory. Tiny rates rounding to zero, upper bounds and other currency exponents remain gated by FR-011. [Numeric cases](numeric-evidence.md).

### Key Entities

- **Expense**: Organization-owned identity, responsible person, work date, project/category, amount and currency, optional quantity/rate basis, notes and independent approval/billing facts.
- **Expense category**: Named active/archived classification with an optional unit and rate.
- **Receipt**: Attachment associated with one expense, with filename, type, size and controlled access.
- **Expense attribution**: Relationship to invoice contributions and approval coverage, retaining the distinction between source work and issued billing.

## Success Criteria

### Measurable Outcomes

- **SC-001**: All agreed capture/category scenarios pass, including cancellation, failures and historical rate cases, without unintended data changes.
- **SC-002**: Scope and revocation scenarios expose zero unauthorized records, receipt bytes or derived totals.
- **SC-003**: Every currency subtotal equals the sum of its entries across list, report and export fixtures; invoice attribution reconciles under the agreed billing contract.
- **SC-004**: Approval and invoice-lock fixtures preserve unrelated dates/projects and demonstrate every authorized correction and denial.
- **SC-005**: Every expense control works with keyboard-only navigation at 320, 390, 768 and 1440 pixel widths in both supported themes; shared project/time/settings screens remain unchanged.

## Assumptions

- Scope confirmation includes this domain despite older omissions. Existing import is preserved, not expanded to expense ingestion by this specification.
- Feature 015 owns authorization and approval; 013 owns workspace preferences. Invoice/payment and reporting owners must be assigned before dependent contracts are finalized.
- Current handoff inventory and `DESIGN.md` constrain appearance. The user authorized composing missing surfaces from existing components and tokens; this is not permission to omit expenses, invent behavior or redesign shared controls.
- Research evidence and unresolved questions are in [research.md](research.md), with discriminating reference checks in [reference-validation.md](reference-validation.md). No feature readiness, runtime verification or complete Harvest parity is claimed.
