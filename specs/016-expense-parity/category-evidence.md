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
ledger. Non-owner authorization, archived-category rate editing, changing away
and back, portable exports and delete enforcement at the server remain distinct
untested cases.
