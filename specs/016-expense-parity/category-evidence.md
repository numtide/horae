# Category archive and minimum-rate evidence

Checked 2026-10-01 using the authorized disposable unit category and expense,
as account owner. Existing categories were not edited or archived.

## Minimum nonzero rate

Against the observed category form endpoint, `0.0004` returned 422 with a
positive-rate validation error; `0.0005` returned 200 and rate `0.001`. Restoring
the fixture's rate to `0.5` returned 200. This discriminates three-decimal rounding
before positivity validation rather than merely checking the raw input's sign.
It complements the previous `0.1235` → `0.124` tie case.

## Archive with existing expenses

With a quantity-2 expense at rate 0.5, the category list disabled Delete and
explained that a category with tracked expenses cannot be deleted. This is UI
enforcement evidence, not a tested server deletion response.

Archiving the fixture through its own Archive control succeeded. The existing
expense remained at USD 1.00. Its editor identified the category as archived and
warned that changing away would require restoring it before changing back.
Changing quantity to 3 and saving through the UI succeeded, giving USD 1.50.
The original synthetic PDF receipt still downloaded successfully with 431 bytes.

A new expense submitted to that same archived category through the observed web
form endpoint returned 422 and required restoring the category first. No new
expense identity was returned or persisted by this rejected request. Thus archive
blocks new capture without blocking eligible corrections to existing entries.

The archived-categories view exposed Restore for the fixture. Restoring it and
reopening the active category list brought back Edit/Archive controls and its
0.5 rate. The fixture category was left active, not archived.

Private evidence: `expense-category-before-archive-20261001.md`,
`expense-category-archived-20261001.md` and
`expense-archived-category-resaved-20261001.md`, plus
`expense-category-restored-20261001.md`. Fixture IDs are in the scratch
ledger. The follow-up below resolves archived-category endpoint rate editing,
changing away/back and server delete enforcement. Non-owner authorization and
portable exports remain separate open cases.

## Archived-category and stale-control follow-up

The archived list exposed Restore but no Edit. A scoped request to the already
observed category form endpoint nevertheless accepted rate 0.75 while keeping
the category archived. Resaving its existing quantity-3 expense returned USD
2.25. This is endpoint behavior, not evidence for adding an Edit control to the
archived-category UI.

Changing that expense to an active ordinary category succeeded. Trying to
assign the archived category again returned 422 and required restoration first.
Restoring the category through the UI, then its rate to 0.5 and the expense to
quantity 3, returned the original USD 1.50.

A category list loaded while the fixture category was unused retained a stale
enabled Delete control after the expense was assigned back. Clicking that exact
fixture control sent DELETE and returned 422 with a not-removable message.
The category and expense survived. This follow-up establishes server-side
delete-in-use enforcement, not merely a disabled button. Non-owner enforcement
and portable export remain separate open checks.

## Switching category mode with historical entries

Starting with quantity 3 / USD 1.50 at rate 0.5, unchecking the category's unit
price option and saving through its editor removed its rate display. Reloading
the expense retained USD 1.50 but hid the unit suffix; its editor showed ordinary
amount 1.50. Saving that expense without changing fields retained total 1.50 and
returned units 1 with null category rate/name fields.

Restoring the category's unit name and rate 0.5 did not reprice the expense:
its row now displayed one unit but still USD 1.50. Opening and saving without
changing fields then produced one unit / USD 0.50. Thus a category-mode change
does not immediately recompute historical totals, but a subsequent expense
save follows the current mode and can replace the quantity basis.

The fixture was restored to quantity 3 / USD 1.50, billable, unbilled/unlocked
with no invoice. The category remained active at 0.5. Private response:
`expense-category-mode-ordinary-response-20261001.json`; numeric read-backs in
`expense-independent-results-20261001.md`.
