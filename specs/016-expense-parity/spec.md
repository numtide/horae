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
1. **Given** an existing expense of 1.24 and a project inheriting its EUR client's currency, **When** that client's currency becomes GBP, **Then** the existing row displays £1.24 without exchange conversion; restoring a USD project override displays $1.24.
1. **Given** an existing receipt, **When** a replacement at or above 10,485,760 bytes fails, **Then** the prior receipt and expense fields remain unchanged; a valid PDF of 10,485,759 bytes is accepted.
1. **Given** an existing receipt, **When** removal is selected and cancelled, **Then** the receipt remains; selecting removal and saving instead removes its internal download route without deleting the expense.
1. **Given** decimal `.` and thousands `,` configuration and a category rate of 0.5, **When** quantity `1,25` is entered and saved, **Then** it persists as 125 units / USD 62.50, not 1.25 units. Input presentation must make the configured number format clear.

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
1. **Given** quantity 3 / USD 1.50 at rate 0.5, **When** the category becomes ordinary and the unchanged expense is resaved, **Then** its total remains 1.50 and quantity becomes 1; restoring unit pricing leaves the amount unchanged until a further expense resave produces USD 0.50 for that one unit.
1. **Given** an expense retaining an archived category, **When** it is changed to an active category and then reassigned back, **Then** reassignment is rejected until the archived category is restored. A stale enabled Delete control MUST NOT permit deleting a category now in use.

### User Story 3 - Review expenses across approval and billing (Priority: P1)

Reviewers and billing staff use the same expense identities without confusing approval, billing and reimbursement.

**Why this priority**: Combining these states would create incorrect balances or unauthorized edits.

**Independent Test**: Compare a mixed-project expense selection across approval, detailed reporting and invoice eligibility using the agreed shared contracts.

**Acceptance Scenarios**:

1. **Given** a non-billable expense, **When** preparing a client invoice, **Then** it is excluded from tracked-expense billing and uninvoiced value.
1. **Given** an expense manually marked billed on an archived project, **When** its billing mark is cleared, **Then** it becomes unbilled with no invoice association but remains archive-locked until project restoration. Other approval/administrative combinations remain distinct acceptance cases.
1. **Given** 1,000 report rows loaded and an eligible expense moved into the same project/date filter after the no-selection confirmation opens, **When** confirming, **Then** all 1,001 current matches are marked billed while an expense outside the dates remains unchanged. Selecting exactly two IDs for clearing instead changes only those two; any project-period protection remains distinct until its blocking conditions are removed.
1. **Given** one client's USD 1.50 and GBP 0.50 expenses, **When** displaying weekly or detailed-report totals, **Then** USD 1.50 and GBP 0.50 remain separate without conversion. A mixed-currency client group shows N/A rather than an invalid combined subtotal; report headline/footer totals still reconcile by currency.
1. **Given** a USD 100 Total project fees budget with expense inclusion enabled and no tracked time, **When** a USD 1.50 expense is recorded, **Then** list and detail show USD 98.50 remaining and the expense report totals USD 1.50. Making the expense non-billable does not change budget consumption; disabling inclusion restores USD 100 remaining without removing expense costs.
1. **Given** that fee budget resets monthly, **When** the expense work date moves from October 1 to September 30, **Then** October has USD 100 remaining while All time expense costs remain USD 1.50; moving it back restores USD 98.50. A 100-hour budget with no time instead retains 100 hours regardless of that expense amount.
1. **Given** an archived project with an existing expense, **When** its owner corrects quantity or billability, **Then** those corrections can persist without removing the archive lock; new expense capture remains rejected. Its detailed report includes the expense unless Active projects only is selected.
1. **Given** an expense incurred in a different currency from the expense form's applicable currency, **When** recording it, **Then** the user enters a manually converted amount; Horae does not fetch an exchange rate. The applicable-currency contract remains gated by FR-011.
1. **Given** an invoice containing tracked expenses, **When** its line amount is edited, **Then** the source amounts stay unchanged and their full billed status remains; invoice value and tracked value are not required to be equal.
1. **Given** a draft invoice for a tracked expense of USD 1.00, **When** an authorized source correction changes the expense to USD 2.00 or deletes it, **Then** the draft retains its original line description, quantity and USD 1.00 total; source editing displays a warning that the invoice will not be updated.
1. **Given** a draft invoice with an attached expense report, **When** a tracked-expense line is removed and the invoice saved, **Then** the regenerated report excludes that line's expenses; recipient access to that report does not grant access to internal expense records.
1. **Given** a report containing one billable expense with no rows checked, **When** confirming the invoiced action, **Then** that expense becomes billed and locked without creating an invoice; explicitly selecting it and confirming uninvoiced removes that billing lock. Cancellation changes nothing. Other project-period or approval/archive locks can still prevent editing; target-set semantics follow FR-016; Projects' empty-selection Actions remain disabled.

### Edge Cases

- Save races with permission revocation, archiving, approval, invoicing or a rate change.
- A replaced receipt fails to upload; an old download link is used after access revocation.
- Resubmission after a timeout must not duplicate the expense.
- Currency display must not be confused with accepted numeric precision: tested JPY values retain two decimals even though its expense list shows none. Current-locale normalization and USD/GBP weekly/report totals are verified; other locales, ordinary numeric bounds, exports and nonzero unlike time/expense-cost aggregation remain open.
- Deleting or changing an already invoiced expense must not silently rewrite a sent invoice or orphan its attribution.
- Expense-report bulk actions with no checked rows act on current filter matches, including an eligible expense entering the filter after the report or confirmation loaded. Explicit row selection remains identity-scoped. Confirmation must communicate that distinction; Projects keeps its confirmed disabled-empty Actions behavior.

## Requirements

### Functional Requirements

- **FR-001**: Provide expense capture, listing, correction and deletion for authorized users, plus category administration. Scope follows feature 015, not a hard-coded legacy Manager label.

- **FR-002**: Capture date, person, project, category and money amount or unit quantity; allow notes and a receipt. Default billability from the project while allowing the reference-supported override. [Tracking expenses](https://support.getharvest.com/hc/en-us/articles/360048687611-Tracking-expenses).

- **FR-003**: Support one receipt per expense, attached by picker or drag/drop, including PNG, GIF, PDF and JPEG, with replacement and explicit removal. Accept at most 10,485,759 bytes; reject 10,485,760 bytes or more without replacing the old receipt or partially saving accompanying field changes. Validate content as well as extension: reject an empty PDF or plain text disguised as PDF/PNG, preserving the previous receipt. A valid PNG named `.png` with declared `text/plain` is accepted and served as `image/png`; the same PNG named `.txt` is rejected. Do not rely solely on the client MIME declaration. Cancelling removal retains the receipt; saving removal makes its internal download route unavailable without deleting the expense. Internal receipt access MUST use the expense's current authorization; invoice-recipient artifacts follow FR-015 separately. A header-only PDF is accepted in the reference, whereas a signature-only PNG is rejected; accepted upload does not imply a structurally valid or renderable document. Finalize rendering-failure and retention behavior; do not assume a public source-file URL is acceptable. [Persisted boundary and content checks](receipt-evidence.md).

- **FR-004**: Support ordinary and unit-priced categories, rename, rate changes, archive, restore and deletion only when unused, enforced at save/delete time even if a stale UI still offers Delete. Existing entries can retain an archived category, but after changing away cannot return to it until restoration. Renames affect historical labels; rate or mode updates do not automatically reprice old entries, but resaving can. Resaving under ordinary mode uses the entered total and resets quantity to 1; switching back to unit pricing preserves that total until an expense resave applies the current quantity and rate. [Categories](https://support.getharvest.com/hc/en-us/articles/360048686731-Managing-expense-categories) and [mode-change evidence](category-evidence.md).

- **FR-005**: Keep approval status, billed status, invoice association and lock reasons distinct. Apply the shared submission/partial-approval contract from 015 to expenses without widening project/person/date scope. [Expense reference](https://help.getharvest.com/api-v2/expenses-api/expenses/expenses/).

- **FR-006**: Preserve ordinary own-entry editing and separately authorized editing for others. Support privileged correction of invoiced notes/amounts and deletion, while preventing date/project/category/billability reassignment under that invoice lock. Distinguish archive locks: the owner can correct amount and billability on an existing expense under an archived project, while date/project/category remain disabled and new capture is rejected. Do not infer that every expense-write grant permits these corrections or that one lock overrides another. For an expense included in a draft invoice, warn that source editing will not update the invoice; correcting or deleting the source MUST NOT automatically rewrite or delete that draft's line, description or amount. Require explicit permanent-deletion confirmation. Resolve sent-invoice, attribution and report consequences before accepting the complete flow. [Editing expenses](https://support.getharvest.com/hc/en-us/articles/4406054281101-How-to-edit-expenses), [owner draft-fixture evidence](billing-evidence.md) and [archive evidence](archive-evidence.md).

- **FR-007**: Account for invoice date-range locks separately for time and expenses, alongside approval, archive and company-wide locks. A time-only invoice MUST NOT alone lock expense dates. An individually cleared expense can remain unbilled with no invoice association yet retain a project-period lock while other expenses in that period are still billed; the named bulk fixture becomes fully unlocked after clearing the remaining matching entries. Do not infer editability from billed status alone. Shared lock ownership and the complete interval/overlap derivation belong to 015 and the billing specification. [Observed report lock sequence](report-scope-evidence.md) and [Locking](https://support.getharvest.com/hc/en-us/articles/360048687491-Unlocking-time-and-expenses).

- **FR-008**: Respect workspace expense-module and reimbursement preferences. Reimbursable and billable MUST remain independent; marking reimbursement MUST NOT imply transferring money. Feature 013 owns configuration. [Preferences](https://support.getharvest.com/hc/en-us/articles/360048179912-Customizing-account-preferences).

- **FR-009**: Provide detailed expense reporting with date/client/project/person/category filters, billing filters, authorized receipt access, printing and CSV/XLSX/PDF export, including supported receipt images in PDF and customizable CSV/XLSX columns. Preserve archived history; distinguish its inclusion in results from showing archived choices in filters. Shared saved/shared/recurring reporting belongs to the reporting package. [Detailed reports](https://support.getharvest.com/hc/en-us/articles/360048687171-Detailed-time-and-detailed-expense-reports).

- **FR-010**: Define billable-expense invoice selection, manual billed/unbilled marking and source attribution with the billing owner. Non-billable expenses MUST NOT be manually marked billed. Editing invoice description, quantity or unit price MUST NOT change its source expenses or partially release their billed status. Removing a saved tracked-expense line or deleting its invoice releases the associated billing status/lock, not independent approval or administrative protection. Free-form lines MUST NOT create source expenses. Do not equate manual billed status with an invoice. [Invoice editing](https://support.getharvest.com/hc/en-us/articles/360048181012-Editing-and-deleting-invoices-and-estimates) and detailed reports under FR-009.

- **FR-011**: Preserve exact amounts and explicit currency attribution while matching the reference's historical behavior. Prefer the project's current currency override, otherwise its client's current currency, in expense forms, rows and weekly grouping. Changing this effective currency relabels existing expenses, including the tested source linked to a draft, without exchange conversion; do not silently freeze their creation-time currency or convert their numeric values. Users manually convert foreign-currency receipts before recording amounts. Category rates use the same numeric value in the project's applicable currency without conversion in the tested fixture. Invoice currency is separate: a draft created with client EUR from a USD expense retained the numeric 1.24 as EUR 1.24, while the source remained USD. Changing a draft-linked source's project currency later relabels that source without rewriting the draft's currency or amount. [Currency reference](https://support.getharvest.com/hc/en-us/articles/360055384512-Does-Harvest-support-multiple-currencies), [currency/history evidence](currency-evidence.md) and [draft-invoice evidence](billing-evidence.md). [NEEDS CLARIFICATION: ordinary numeric bounds and other input locales, export reconciliation, sent-invoice currency changes and other sent-invoice/attribution/report consequences of privileged source-expense correction/deletion.]

- **FR-012**: Validate permission, identity, project/category availability and business state at save/download time. Reject stale conflicting edits visibly without partial success. Logging MUST not expose receipt contents or private notes.

- **FR-013**: Supply keyboard-operable forms, labelled controls, explicit empty/loading/error/pending states and recoverable validation. Compose expense surfaces without dedicated mockups from Horae's existing design system, preserving verified Harvest behavior. Record the screen composition and control/state mapping during planning; do not change shared defaults or require a new handoff. Verify shared project/time/settings screens for regressions under SC-005.

- **FR-014**: Use feature 015's six expense read/write scopes: own, managed people/projects and organization-wide. Built-in defaults are own for Member, own plus managed for Project Manager, and organization-wide for People Admin, Accounting, Executive Manager and Administrator. Customization follows 015's Member floor and grant prerequisites. [NEEDS CLARIFICATION: finalize category, receipt, deletion, manual billing, approval and privileged locked-entry operation mapping with 015; observed ordinary read/write grants do not establish these lifecycle permissions.]

- **FR-015**: Support a client-facing expense report attached to an invoice containing expenses. Generate it on invoice save from the included expenses and regenerate an attached report when editing and saving a draft. Recipient access MUST remain distinct from internal source-expense access. Resolve sent-invoice and later source-edit/receipt-removal behavior with the billing owner before finalizing retention. [Invoice attachments](https://support.getharvest.com/hc/en-us/articles/9864825272589-Attaching-files-and-reports-to-invoices).

- **FR-016**: For expense-report billed/unbilled bulk actions, explicit selection targets only the selected expense identities; no selection targets all eligible entries matching the current report filters at execution, not a snapshot of loaded rows. Recheck current authority and eligibility for every target. Confirm the selected-versus-filter-wide scope and project/date filters; cancellation changes nothing. A fixture with 1,000 loaded rows plus one expense moved into the filter after confirmation opened marked all 1,001, leaving an out-of-filter control unchanged. Explicitly clearing two selected IDs changed only those two. Do not invent a page-only guarantee or require pagination to explain the filter-wide behavior. The tested report rendered 1,001 on one page; this is not a claim of unlimited volume or an observed all-pages control. Keep Projects' disabled-empty Actions and shared defaults unchanged. Apply FR-010/012 to every target. [Detailed reports](https://support.getharvest.com/hc/en-us/articles/360048687171-Detailed-time-and-detailed-expense-reports) and [discriminating target-set evidence](report-scope-evidence.md).

- **FR-017**: Preserve the verified numeric rules: round unit quantities to two decimal places before multiplication; retain category rates to three decimal places; accept two-decimal expense totals in the tested USD, JPY and BHD cases. Tested ties round away from zero. JPY list display uses no decimals, but MUST NOT truncate the underlying fractional amount; BHD list display uses two in the reference fixture. Accept negative ordinary amounts and quantities; reject malformed numeric values and category rates that are nonpositive after rounding (`0.0004` rejected, `0.0005` becomes `0.001`). After rounding, quantities range from -100000 to 999999999.99 inclusive and positive category rates cannot exceed 10000000. Do not reject stored zero totals/quantities merely because the web input clears zero on blur and its required-field check blocks submission: preserve that observed UI/domain distinction. Rate changes do not automatically reprice historical entries; an authorized resave applies the current rate, even without changed fields. Exact arithmetic remains mandatory; reconcile fractional currency/rate representation explicitly during the constitution check. Ordinary-amount bounds remain a contract conflict: the reference's advertised limits disagree with some accepted/rejected inputs and one accepted large value loses cents. Do not copy that precision loss or treat the error text as a verified exact bound. Other untested currencies remain open. [Numeric cases](numeric-evidence.md), [currency cases](currency-evidence.md) and [category cases](category-evidence.md).

- **FR-018**: Interpret numeric input under the configured decimal/thousands separators rather than assuming a user's language implies decimal comma. With decimal `.` and thousands `,`, accept `1,25` as 125 and `1,234.56` as 1234.56; reject `1.234,56` and `1 234.56` under that configuration. The reference web form normalizes `1e3` to the visibly displayed `13.00` in both ordinary amount and unit quantity fields; saving persists 13, not 1000. The separate ordinary-amount endpoint accepts `1e3` as 1000, which MUST NOT be used as evidence of UI scientific-notation support or as authority to add another Horae mutation API. Preserve the visible normalized value at submission. Other locale configurations remain unverified. [Current-locale evidence](numeric-evidence.md).

- **FR-019**: Integrate expenses with project budgets: the project editor owns the option to include billable and non-billable expenses in Total project fees budgets; other budget types MUST NOT silently consume expense amounts. Disabling inclusion removes budget consumption, not expense costs or history. Monthly consumption follows the expense work date; prior-month costs remain in All time totals. Project list and detail MUST agree on budget consumption, reconciling expense contributions with the detailed report without changing invoice eligibility or adding a budget remainder to that report. Enable/disable, both billability states, monthly boundaries, total-hours exclusion and single-currency owner list/detail/report reconciliation have persisted reference evidence. Follow the [shared budget contract](budget-contract.md); task/person variants, cross-currency and non-owner checks remain explicitly unverified. [Budget reference](https://support.getharvest.com/hc/en-us/articles/360052763131-How-do-I-include-expenses-in-my-project-budget) and [live budget evidence](budget-evidence.md).

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
