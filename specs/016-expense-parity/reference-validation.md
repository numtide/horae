# Expense rule validation

Checked 2026-10-01. This records reference evidence, not completed Horae tests.
`Documented`, `client-observed` and `persisted` are different evidence levels.
The user authorized disposable reference experiments on 2026-10-01: create,
modify and delete only test clients/projects, categories, expenses, receipts and
draft invoices. Existing business records, messages, payments, seat purchases and
owner changes remain excluded. Authorization is not a successful test result.

A dedicated test client and T&M project were created after authorization. A
read-back of the project editor confirmed EUR on the test client and an explicit
USD project override. Exact IDs and cleanup targets are recorded only in the
main checkout's `.scratch/playwright-windows/expense-fixture-ledger-20261001.md`.
No expense, category, receipt or invoice has yet been saved. The expense tab
currently reports `document.visibilityState = hidden`; tab selection and
`window.focus()` did not restore visibility. The user was asked to restore Chrome
before continuing interactive validation. Reuse these fixtures; do not recreate
them after a connection or visibility interruption.

## Current browser evidence

The authenticated expense list is empty. The new-expense form was opened without
saving. It exposes date, project, category, notes, one receipt, billability,
reimbursement and amount. Choosing Mileage changes the amount suffix to miles.
The picker advertises GIF, JPEG, PNG and PDF; this does not prove server-side
content validation, size limits or successful upload.

There are three Track expenses buttons in the DOM, two with zero-size bounds.
Selecting the visible control opened the form. Some later visible-control
clicks also timed out, so hidden duplicates do not explain every timeout.

An unsaved Mileage experiment set the amount, dispatched input/change and moved
focus. Its first run produced these values:

| Input | Display after blur |
| --- | --- |
| 1.234 | 1.23 |
| 1.235 | 1.24 |
| -1.235 | -1.24 |
| 0 | Empty required field |
| 1,25 | 125.00 |
| 999999999999.99 | 999,999,999,999.99 |

A later run after an unsuccessful category interaction left inputs unchanged,
including nonnumeric text. A fresh-form fill/Tab experiment without a selected
project/category likewise retained `1.235`. Therefore these are **inconclusive
interaction experiments**, not an accepted numeric contract. Native input
validity does not demonstrate Harvest server acceptance. Do not adopt negative
support, two-decimal storage, a maximum, or zero rejection from these results.

Local snapshots remain outside Git in the main checkout's
`.scratch/playwright-windows/`:

- `expenses-open-form-goal-20261001.md`
- `expenses-category-menu-goal-20261001.md`
- `expenses-unsaved-amount-goal-20261001.md`

## Delivered client behavior

Read-only inspection of the public
[expense asset](https://cache.harvestapp.com/static/expenses-VQ26GITA.js), SHA-256
`9329004b39e0ec48d2281f7683cd686f3863b2a08d763fe429c961c36e87bef2`, found:

- Form currency, expense-row currency and weekly currency grouping prefer a
  project's currency override and otherwise use its client's currency.
- Unit-priced categories send quantity; ordinary categories send total cost.
- Receipt selection/replacement and explicit receipt deletion are separate form
  states serialized on save; selection alone is not a completed upload.
- Save failures re-enable the form controls. The client inspection did not
  establish server numeric bounds or a receipt-size validator.

This resolves the **current client currency priority**, not historical currency
changes, stored representation, invoice currency or server-side enforcement.
Horae must preserve exact arithmetic, not copy the reference's floating-point
implementation. The older client-only expense FAQ on the
[currency page](https://support.getharvest.com/hc/en-us/articles/360055384512-Does-Harvest-support-multiple-currencies)
must not override this newer client evidence without a discriminating fixture.

## Documented attachment and billing boundaries

The [receipt launch announcement](https://www.getharvest.com/blog/2010/05/upload-expense-receipts-in-harvest)
advertised 10 MB in 2010. A current search excerpt for the marketing upload page
also says 10 MB, but opening that page returned 404. Neither establishes today's
exact byte boundary; do not silently pick 10,000,000 or 10,485,760 bytes.

The [expense API](https://help.getharvest.com/api-v2/expenses-api/expenses/expenses/)
documents one receipt and explicit deletion. Its broader locked-update wording
does not establish which fields the web form permits, nor does its `integer`
quantity label establish browser fractional behavior. These sources do not
authorize adding a write API to Horae.

[Invoice attachments](https://support.getharvest.com/hc/en-us/articles/9864825272589-Attaching-files-and-reports-to-invoices)
establish a separate client-facing expense report. It is generated when the
invoice is saved, excludes lines removed before save, and is regenerated when an
attached draft invoice is edited and saved. Its files are accessible to invoice
recipients. This is not general access to the underlying expense or receipt URL.
Source edits without an invoice save, sent-invoice regeneration and subsequent
receipt deletion effects remain unverified.

[Detailed reports](https://support.getharvest.com/hc/en-us/articles/360048687171-Detailed-time-and-detailed-expense-reports)
include archived history; explicitly selecting archived filter options requires
including them in the filter picker. Billable expenses can be manually marked
billed; non-billable expenses cannot. CSV/XLSX support column customization and
PDF includes receipt images. Its no-selection bulk-action behavior targets all
entries on the page, not an empty set. FR-016 follows that documented report
behavior under the confirmed parity scope while preserving Horae's separately
confirmed disabled-empty Actions behavior on Projects. This is a screen-specific
contract, not a change to shared table defaults.

## Permission evidence and owner

Feature 015 owns the canonical matrix. Its
`contracts/expense-permissions-evidence.md` records the six expense grants and
profile defaults observed in the authenticated editor configuration. Evidence
does not yet map category administration, locked correction, deletion, manual
billing, reports or approval to every custom grant combination. A time-approval
permission is not automatically an expense-write permission or vice versa.

## Remaining discriminating checks

These are authorized reference experiments requiring suitable account access,
not unchecked Horae implementation tasks. Capture before/after values and relevant
errors; identify fixture records before any mutation. Do not touch existing
business records, send invoices, notify clients, pay, purchase seats or change
the account owner. Use only explicitly authorized disposable fixtures.

| ID / requirement | Reference experiment | Evidence needed to close |
| --- | --- | --- |
| E-NUM / FR-002/004/011 | Save ordinary and unit expenses with fractional values, half-way products, zero, negative, malformed and boundary values; reopen and export | Accepted precision/range/sign, input locale, stored quantity/rate/total and rounding stage; include supported currency exponents |
| E-RATE / FR-004/011 | Change a fixture rate and resave an old expense, including a notes-only edit and an archived category | Old versus new total, applicable rate and resave behavior; category mode/zero-rate handling |
| E-CUR / FR-011 | Compare client currency with an explicit project override; change only disposable client/project currencies after saving | New-entry currency, historical totals/labels, override inheritance and unchanged versus converted numbers |
| E-BILL / FR-006/010 | Link fixture expenses to a draft invoice, then correct/delete a source; inspect invoice lines, totals, source links and attached report | Source-to-invoice effects, distinct from invoice-to-source rules already documented; sent-state evidence requires separately safe access |
| E-REC / FR-003/012 | Upload synthetic valid files around both possible 10 MB boundaries; replace/remove; attempt an invalid replacement | Exact size/type/error rules and preservation of original expense/receipt on failure |
| E-AUTH / FR-005/006/014 | Use an editable non-owner reference person with own, managed and all read/write combinations; repeat after scope revocation | Allow/deny matrix for expenses, receipts, categories, locks, billing and approval; account currently exposes only an immutable owner |
| E-ARCH / FR-003/004/009 | Archive fixture project/person/category and inspect historical expense, receipt, reporting and portable archive | Retained history and access rules; category archive must not itself prohibit editing existing expenses |
| E-REP / FR-009/010/015 | Compare explicit selection with no selection; save a draft expense report, remove a source line and save again | Bulk-action target/confirmation, regenerated report scope and recipient versus internal receipt access |

Until these gates are resolved, no final expense plan, complete operation matrix
or implementation-ready status is claimed. Additional read-only documentation
can narrow the experiments but cannot certify unobserved persistence.
