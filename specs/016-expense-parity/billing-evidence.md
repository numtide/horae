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
correction/deletion authorization and attribution after source deletion. The
subsequent draft-deletion and line-removal checks below resolve source billing
release for those cases without independent locks.

## Independent follow-up: currency and draft deletion

A second disposable draft was created from the new USD 1.24 expense, this time
retaining the client's EUR default. The saved draft showed EUR 1.24 with source
quantity 1 and price 1.24, without conversion. This supersedes the earlier
UI-only cross-currency limitation for this exact case; it does not establish
exchange-rate behavior or mixed-project currency reconciliation.

The second draft was deleted through its own More actions menu, exact invoice
number confirmation and required DELETE text. The source expense remained with
its receipt. Its date/project/category/billability controls became editable,
and a subsequent successful fixture update returned `is_billed=false`,
`is_locked=false`, `invoice=null`. No independent approval/admin lock was present;
the test does not prove that deleting an invoice removes such locks.

Private snapshots: `expense-report-invoice-saved-20261001.md`,
`expense-report-draft-delete-dialog-20261001.md`,
`expense-report-draft-deleted-20261001.md`, and
`expense-after-draft-deletion-20261001.md`. Only the disposable second draft was
permanently deleted; pre-existing business invoices were preserved.

## Invoice line correction and removal follow-up

A third disposable draft included quantity 3 at 0.50 from the USD 1.50 expense,
using its client's EUR default. Updating the invoice price to 0.75 saved EUR
2.25; the source remained quantity 3 / USD 1.50 and its editor still linked to
that invoice with an invoiced warning. No partial release or source repricing
occurred. The subsequent project USD→GBP change relabeled the source to GBP
1.50 without changing the invoice's EUR 2.25; see [currency evidence](currency-evidence.md).

After restoring project USD, deleting the sole tracked line in the draft editor
and saving succeeded. The invoice remained Draft, now EUR 0.00 with no item,
and the success message explicitly reported that associated expenses became
uninvoiced. This is a saved empty draft, not an invoice deletion. No recipient
artifact or sent-state behavior was tested.

The source list again exposed ordinary Edit. A subsequent successful baseline
save returned quantity 3 / USD 1.50, billable, `is_billed=false`,
`is_locked=false`, `invoice=null`; the original receipt downloaded with 200 and
431 bytes. No independent approval/archive/admin lock was present. Private
snapshot: `expense-after-line-removal-20261001.md`. The empty test draft remains
identified for cleanup; its removed test line can be recreated, but its original
line identity is not restored by doing so.

## Report validation limitation in the current account

The new source had a synthetic PDF receipt. Invoice preparation selected the
expense report and the review form retained its true include-report flag and
the exact source expense identity. The saved draft did not expose an attachment
link or attachment controls in the observed page. This does not establish that
no report was generated or that the feature was removed.

Trying the draft's Preview action opened an explicit restriction: the current
free/trial account needs a paid plan or a connected Stripe account to preview.
Neither action is authorized or necessary for the independent tests. No bypass,
upgrade, new integration, sending or payment was attempted. The documented
[draft report generation/regeneration](https://support.getharvest.com/hc/en-us/articles/9864825272589-Attaching-files-and-reports-to-invoices)
contract remains documented rather than browser-certified. The missing attachment
controls and the explicit preview restriction are separate observations; their
causal relationship is not proven.

Evidence: `expense-report-invoice-review-20261001.md`,
`expense-report-invoice-actions-20261001.md` and
`expense-report-preview-opened-20261001.md`. Report content, regeneration and
recipient access remain open until suitable reference access is available.
