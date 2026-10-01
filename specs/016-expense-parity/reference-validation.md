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
A fixture expense and unit category have now been saved and reused for numeric,
repricing and synthetic receipt experiments. A draft invoice was subsequently
saved; correcting and then permanently deleting the fixture source expense left
its original invoice line and amount unchanged. The draft remains unsent. Chrome
still reports hidden visibility, but focused keyboard and guarded DOM actions
successfully save records; visibility is no longer a blocker. Reuse the ledger's
exact surviving fixtures after connection interruptions; the deleted source
expense cannot be reused. Cleanup of the draft/client/project/category is pending.

Independent follow-up reused that client/project and created a new expense.
Currency changes and inheritance were exercised then restored to client EUR /
project USD. A second EUR draft retained the USD expense's numeric 1.24; deleting
that disposable draft released the source billing lock and preserved its receipt.
The first draft remains. Report Preview hit an explicit paid-plan/Stripe gate;
no upgrade or integration was attempted. See the private ledger for current IDs.

## Persisted numeric follow-up

[Numeric evidence](numeric-evidence.md) records successful UI saves and separate
form-endpoint tests. The persisted EUR-client/USD-project case follows project
currency. USD amounts round to two decimals, quantities to two decimals before
multiplication, and category rates to three. Tested ties round away from zero.
Negative amounts/quantities persist; malformed inputs fail. Zero is accepted by
the endpoint but cleared and blocked by the UI's required-field validation.
Changing a rate leaves old expenses unchanged until resaved, even when no field
is changed. These findings supersede the inconclusive unsaved cases below only
where a matching persisted case exists. Later [currency checks](currency-evidence.md)
verify two-decimal response amounts for JPY/BHD, JPY zero-decimal list display,
and historical relabeling without exchange conversion after project/client
currency changes. Current-locale inputs and category mode changes are now
verified. Bounded follow-up establishes quantity/rate limits, but ordinary
amount errors conflict with actual acceptance and one response loses cents;
that conflict needs an exact-money contract, not more escalating probes.
Other locales, exports and remaining billing effects remain separate questions.

## Initial unsaved browser evidence

The authenticated expense list was initially empty. The new-expense form was opened without
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

This asset inspection resolves the **current client currency priority**, not
historical currency changes, stored representation or invoice currency. The
subsequent persisted fixture separately confirms project-override acceptance.
Horae must preserve exact arithmetic, not copy the reference's floating-point
implementation. The older client-only expense FAQ on the
[currency page](https://support.getharvest.com/hc/en-us/articles/360055384512-Does-Harvest-support-multiple-currencies)
must not override this newer client evidence without a discriminating fixture.

## Documented attachment and billing boundaries

The [receipt launch announcement](https://www.getharvest.com/blog/2010/05/upload-expense-receipts-in-harvest)
advertised 10 MB in 2010. A current search excerpt for the marketing upload page
also says 10 MB, but opening that page returned 404. Neither establishes today's
exact byte boundary; do not silently pick 10,000,000 or 10,485,760 bytes.

Subsequent authorized [receipt experiments](receipt-evidence.md) resolve the
boundary: 10,485,759-byte PDFs save, whereas 10,485,760 and 10,485,761 fail with
422\. An invalid replacement preserves the prior downloadable receipt and the
expense note. A `.txt` file is rejected without persisting its other field changes.
Cancelling removal retains the receipt; saving removal makes the old internal
receipt route return 404 while preserving the expense. Later benign content
tests accepted real PNG/GIF/JPEG, rejected empty/disguised-text files and
normalized PNG MIME. Follow-up accepted a nine-byte PDF header without a full
document but rejected a signature-only PNG; acceptance is not structural
validity. Rendering failure, storage retention and access lifecycle remain open;
the size limit is no longer inferred from the old article.

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
Effects on this report of source edits without an invoice save, sent-invoice
regeneration and subsequent receipt deletion remain unverified.

The later [draft-invoice fixture](billing-evidence.md) verifies source-to-invoice
line independence: changing source quantity/note or deleting it does not update
the saved draft's original line or total. The owner editor disables date,
project, category and billability and warns that source edits will not update
the invoice. No report content or sent-invoice behavior was verified; these
remain separate from the proven draft line/amount behavior.

[Detailed reports](https://support.getharvest.com/hc/en-us/articles/360048687171-Detailed-time-and-detailed-expense-reports)
include archived history; explicitly selecting archived filter options requires
including them in the filter picker. Billable expenses can be manually marked
billed; non-billable expenses cannot. CSV/XLSX support column customization and
PDF includes receipt images. The guide describes no-selection bulk actions as
targeting entries on the page, not an empty set. However, the live form submits
filters without IDs; [report evidence](report-evidence.md) records this unresolved
page-versus-filter discrepancy. FR-016 no longer claims page scope is verified.
Projects' separately confirmed disabled-empty Actions and shared table defaults
remain unchanged.

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
| E-NUM / FR-002/004/011 | USD/JPY/BHD precision, tiny rates, current locale and quantity/rate bounds verified; ordinary boundary messages contradict some responses, including lost cents | Resolve exact ordinary bound contract; other locale configurations and exports remain; preserve lexical exactness and zero UI/server distinction |
| E-RATE / FR-004/011 | Active repricing, archive/resave/new-capture rejection, restore, mode changes, archived endpoint rate editing, change-away/return and stale Delete rejection verified | Independent named category cases complete; archived list has Restore only, endpoint acceptance does not add an Edit UI; non-owner authority remains E-AUTH |
| E-CUR / FR-011 | Project override/inheritance, historical relabeling, client EUR→GBP and draft-linked source USD→GBP verified; original currencies restored | No observed conversion; draft currency/value remains independent; mixed-currency reports/exports and sent-state effects remain open |
| E-BILL / FR-006/010 | Draft source correction/deletion and invoice-price/source independence verified; deleting a saved tracked line releases source billing and retains receipt, leaving an empty draft | Finish independent multiple-lock case; owner-only draft evidence does not prove non-owner authorization, report retention or sent behavior; sent-state evidence requires separately safe access |
| E-REC / FR-003/012 | Size/removal and benign content matrix verified, including accepted header-only PDF and rejected signature-only PNG | Define rendering failure, access/revocation and retention; do not claim full parser/sanitizer or provider-backup erasure |
| E-AUTH / FR-005/006/014 | Use an editable non-owner reference person with own, managed and all read/write combinations; repeat after scope revocation | Allow/deny matrix for expenses, receipts, categories, locks, billing and approval; account currently exposes only an immutable owner |
| E-ARCH / FR-003/004/009 | Category and project archive/correction/restore, receipt retention, default archived report inclusion and active-only exclusion verified | Project archive permits owner billability correction unlike invoice lock; person archive needs suitable access, portable archive depends on export authority; do not infer non-owner behavior |
| E-REP / FR-009/010/015/016 | Single-row report reconciliation and manual billing verified; export controls inspected; empty-selection page scope contradicted by form shape | Multi-page scope needs discrimination. Generation not submitted because delivery can email; no export bytes certified. Invoice Preview separately requires paid plan/Stripe; no upgrade authorized |

Until these gates are resolved, no final expense plan, complete operation matrix
or implementation-ready status is claimed. Additional read-only documentation
can narrow the experiments but cannot certify unobserved persistence.
