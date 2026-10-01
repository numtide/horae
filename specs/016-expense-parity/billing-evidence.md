# Draft-invoice source expense evidence

Checked 2026-10-01 in the authenticated Harvest web UI, as account owner, using
only the authorized disposable client, project, expense and a new draft invoice.
No invoice was sent or marked sent, and no payment or notification was created.
Do not extrapolate owner results to scoped non-owner permissions or sent invoices.

## Invoice preparation

The fixture project had no tracked time and one unsubmitted, billable expense:
quantity 2 at rate 0.5, total USD 1.00. Invoice preparation warned about unapproved
or unsubmitted work but offered all expenses, approved only, or approved plus
pending approval. All expenses included the fixture without requiring submission.
Detailed display produced one line with quantity 2 and unit price 0.50.

Although the project and expense used USD, the initial invoice review defaulted
to its client's EUR and showed numeric total 1.00 without conversion. This is a
UI observation, not a persisted cross-currency invoice test. USD was explicitly
selected before saving the fixture draft to keep the source-correction experiment
independent of currency conversion. Default private notes were replaced with
disposable-test text; the invoice used a unique test number and subject.

## Source correction

After Save invoice, the resulting invoice showed Draft and USD 1.00. The source
expense editor warned that it was invoiced and editing would not update the
invoice. Date, project, category and billability were disabled. Notes and quantity
remained editable for the owner.

Changing source quantity from 2 to 4 and replacing its note saved successfully;
the expense row showed quantity 4, the new note and USD 2.00. Reloading the draft
invoice still showed its original description, quantity 2, unit price 0.50 and
USD 1.00 total. Thus source correction does not automatically rewrite this draft's
line or amount. This is the reverse direction from the previously documented
invoice-line-to-source independence.

## Source deletion

The owner could choose Delete on the same invoiced expense. The UI requested
confirmation of permanent expense deletion. Confirming removed that fixture
from the expense list. Reloading its draft invoice still showed the original
description, quantity 2 and USD 1.00 total. Source deletion did not cascade into
invoice-line or invoice deletion. Internal attribution/tombstone representation
has not been established; do not infer it from visible line retention.

## Evidence and limits

Local snapshots in `.scratch/playwright-windows/`:

- `expense-invoice-projects-selected-20261001.md`
- `expense-invoice-review-20261001.md`
- `expense-invoice-draft-confirmed-20261001.md`
- `expense-invoiced-source-corrected-20261001.md`
- `expense-invoice-after-source-correction-20261001.md`
- `expense-invoiced-source-delete-confirm-20261001.md`
- `expense-source-deleted-20261001.md`
- `expense-invoice-after-source-deletion-20261001.md`

The earlier `expense-invoice-draft-saved-20261001.md` is a failed pre-save attempt
caused by currency selection; only the subsequent `draft-confirmed` snapshot is
evidence of a persisted invoice. Exact fixture IDs and cleanup state stay in the
private ledger. Invoice report attachment was requested during preparation, but
its generation/content was not verified; no recipient artifact test has passed.

Still open: sent-invoice behavior, report regeneration/retention, non-owner
correction/deletion authorization, attribution after source deletion, and source
lock release when removing an invoice line or deleting the draft invoice.
