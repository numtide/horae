# Feature Specification: Expense Tracking and Categories

**Feature Branch**: `feat/expense-parity`

**Created**: 2026-10-01

**Status**: Draft — reference investigation and shared contracts incomplete

**Input**: Complete Harvest web parity, including expenses, using Horae's design language; specify before implementation. Native applications and new integrations are excluded.

## Clarifications

### Session 2026-10-01

- Q: May screens without a dedicated mockup be composed from Horae's existing design system, preserving Harvest behavior? → A: Yes, authorized. Reuse current components and tokens; a new handoff is not a prerequisite for these surfaces.

## User Scenarios & Testing

### User Story 1 - Record and correct a project expense (Priority: P1)

A teammate records a dated expense against permitted work and can correct it without losing the original entry on failure.

**Why this priority**: Expenses must be captured before approval, reporting or billing can use them.

**Independent Test**: Create, reopen and correct an expense; verify cancel and failed save leave stored records unchanged.

**Acceptance Scenarios**:

1. **Given** available work and a category, **When** a person saves a date and amount, **Then** the entry retains its project, category, owner and optional note/receipt.
1. **Given** invalid input or a failed request, **When** saving, **Then** field errors are actionable, the draft remains available and no success is claimed.
1. **Given** a person outside the authorized scope, **When** opening an expense or its receipt directly, **Then** no private content or identifying metadata is disclosed.

### User Story 2 - Maintain expense categories (Priority: P1)

An authorized administrator maintains categories without erasing historical expenses.

**Why this priority**: Category lifecycle and rate changes affect the correctness of every subsequent expense.

**Independent Test**: Exercise rename, rate change, archive, restore and attempted deletion using an existing entry.

**Acceptance Scenarios**:

1. **Given** a category with entries, **When** archived, **Then** history remains and new entries cannot use it; eligible existing entries remain editable.
1. **Given** a category in use, **When** deletion is attempted, **Then** it is refused without deleting expenses.
1. **Given** a unit-priced entry, **When** the category rate changes, **Then** its recorded amount remains until an authorized resave applies the current rate.

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

### Edge Cases

- Save races with permission revocation, archiving, approval, invoicing or a rate change.
- A replaced receipt fails to upload; an old download link is used after access revocation.
- Resubmission after a timeout must not duplicate the expense.
- Currency changes, fractional quantities, zero/negative values and rounding boundaries require reference fixtures before final planning.
- Deleting or changing an already invoiced expense must not silently rewrite a sent invoice or orphan its attribution.

## Requirements

### Functional Requirements

- **FR-001**: Provide expense capture, listing, correction and deletion for authorized users, plus category administration. Scope follows feature 015, not a hard-coded legacy Manager label.
- **FR-002**: Capture date, person, project, category and money amount or unit quantity; allow notes and a receipt. Default billability from the project while allowing the reference-supported override. [Tracking expenses](https://support.getharvest.com/hc/en-us/articles/360048687611-Tracking-expenses).
- **FR-003**: Support receipt attachment by picker and drag/drop, including PNG, GIF, PDF and JPEG. Receipt access MUST use the expense's current authorization. Define size, replacement, failure and retention behavior in the final contract; do not assume a public file URL is acceptable.
- **FR-004**: Support ordinary and unit-priced categories, rename, rate changes, archive, restore and deletion only when unused. Renames affect historical labels; rate updates do not automatically reprice old entries, but resaving can. [Categories](https://support.getharvest.com/hc/en-us/articles/360048686731-Managing-expense-categories).
- **FR-005**: Keep approval status, billed status, invoice association and lock reasons distinct. Apply the shared submission/partial-approval contract from 015 to expenses without widening project/person/date scope. [Expense reference](https://help.getharvest.com/api-v2/expenses-api/expenses/expenses/).
- **FR-006**: Preserve ordinary own-entry editing and separately authorized editing for others. Support the documented privileged correction of locked notes/amounts and deletion, while preventing locked project/category reassignment. Resolve invoice consequences before accepting this flow. [Editing expenses](https://support.getharvest.com/hc/en-us/articles/4406054281101-How-to-edit-expenses).
- **FR-007**: Account for invoice date-range locks separately for time and expenses, alongside approval, archive and company-wide locks. A time-only invoice MUST NOT alone lock expense dates. Shared lock ownership belongs to 015 and the billing specification. [Locking](https://support.getharvest.com/hc/en-us/articles/360048687491-Unlocking-time-and-expenses).
- **FR-008**: Respect workspace expense-module and reimbursement preferences. Reimbursable and billable MUST remain independent; marking reimbursement MUST NOT imply transferring money. Feature 013 owns configuration. [Preferences](https://support.getharvest.com/hc/en-us/articles/360048179912-Customizing-account-preferences).
- **FR-009**: Provide detailed expense reporting with date/client/project/person/category filters, billing filters, authorized receipt access, printing and CSV/XLSX/PDF export, including supported receipt images in PDF. Shared saved/shared/recurring reporting belongs to the reporting package. [Detailed reports](https://support.getharvest.com/hc/en-us/articles/360048687171-Detailed-time-and-detailed-expense-reports).
- **FR-010**: Define billable-expense invoice selection, manual billed/unbilled marking and source attribution with the billing owner. Editing invoice description, quantity or unit price MUST NOT change its source expenses or partially release their billed status. Removing a saved tracked-expense line or deleting its invoice releases the associated billing status/lock, not independent approval or administrative protection. Free-form lines MUST NOT create source expenses. Do not equate manual billed status with an invoice. [Invoice editing](https://support.getharvest.com/hc/en-us/articles/360048181012-Editing-and-deleting-invoices-and-estimates).
- **FR-011**: Preserve exact currency-specific amounts and historical attribution. Users manually convert foreign-currency receipts before recording amounts in the applicable expense currency; do not add exchange-rate retrieval or automatic conversion. [Currency reference](https://support.getharvest.com/hc/en-us/articles/360055384512-Does-Harvest-support-multiple-currencies). [NEEDS CLARIFICATION: resolve the reference's project-currency override versus client-currency expense guidance, supported quantity precision, monetary rounding, historical currency-change effects and invoice consequences of privileged source-expense correction/deletion.]
- **FR-012**: Validate permission, identity, project/category availability and business state at save/download time. Reject stale conflicting edits visibly without partial success. Logging MUST not expose receipt contents or private notes.
- **FR-013**: Supply keyboard-operable forms, labelled controls, explicit empty/loading/error/pending states and recoverable validation. Compose expense surfaces without dedicated mockups from Horae's existing design system, preserving verified Harvest behavior. Record the screen composition and control/state mapping during planning; do not change shared defaults or require a new handoff. Verify shared project/time/settings screens for regressions under SC-005.
- **FR-014**: Finalize the operation-level expense/receipt/category matrix with feature 015, including privileged locked-entry operations and custom grants. [NEEDS CLARIFICATION: the approved six-profile matrix must cover this newly scoped domain; legacy Harvest role wording is not sufficient evidence.]

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
- Research evidence and unresolved questions are in [research.md](research.md). No feature readiness, runtime verification or complete Harvest parity is claimed.
