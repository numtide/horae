# Detailed expense target-set investigation

Observed 2026-10-01 using only individually recorded disposable expenses in
the fixture project. Existing source expense is dated October 1; additional
test records use October 2 so report-filter leakage can be distinguished.

## Pagination precondition

The original multi-page hypothesis came from shared delivered selection code,
not from a visible expense-report pagination control. The authenticated report
with 2, 51 and 101 matching expenses rendered every row on one page. Appending
`per_page=1` with two expenses did not reduce the result. A read-only request
with `page=2` at 101 expenses returned the same first/last IDs and all 101 rows;
no pagination controls or page metadata were present in that response.

The shared client defines all-pages selection, but that alone does not establish
that this expense report exposes it. API v2 report pagination is a different
surface and cannot establish web-page semantics. These results disprove neither
a higher-volume threshold nor plan-dependent pagination. Do not label this a
passed multi-page test or manufacture pages by hiding rows in the browser.

The 101 records are exact cleanup targets in the private pagination ledgers.
All were created through the observed expense form endpoint with the fixture
project/category, unique notes and 0.50 USD each. They are not business expenses.

## Empty selection and filter containment

With all 101 October 2 records visible and no row selected, the action form
contained only the fixture project/date filters and no IDs. Confirming invoiced
reported 101 updated expenses. The ordinary expense list, unlike this report,
actually paginates at 50 entries: its delivered response advertised three pages
for 102 entries, with explicit next links. Reading all three returned all 101
test records billed/locked; the October 1 baseline remained unbilled/unlocked.
Thus filter containment and multi-record marking are verified; these are not
three pages of the detailed report and cannot prove its hypothetical page scope.

The delivered expense-list objects represent manual marking with `invoice.id=1`.
That is a client representation, not proof of a real invoice association or a
Horae identifier to copy. The manual-marked editor says marked as invoiced,
whereas a source on a real draft links to that draft. Preserve the separate
domain facts required by FR-005/010 rather than interpreting a truthy reference
object as an actual invoice relationship.

## Cleanup

All 101 temporary expenses were deleted using their individually recorded and
revalidated IDs, each returning 200. A fresh October 2 fixture-project report
returned zero rows and its explicit empty state. The October 1 baseline and its
receipt were excluded from deletion. Deleted test identities cannot be restored;
no pre-existing business record was targeted.

## Larger-volume and changed-result-set discriminator

A later bounded experiment created 1,001 individually checkpointed disposable
expenses on October 3, leaving the original October 1 expense outside the filter.
The final detailed report displayed all 1,001 exact IDs, no missing/foreign IDs,
no pagination controls and USD 500.50. This establishes the observed volume,
not a claim that Harvest can never paginate or limit a larger report.

The target-set ambiguity was then resolved without requiring a page boundary:

1. Move the last disposable expense to October 4 and load the October 3 report.
   It contains exactly 1,000 rows; the moved expense is absent.
1. Open the no-selection invoiced confirmation. It names all expenses within
   the displayed project/date filters. The form action carries those filters
   and no selected expense IDs.
1. Move that same expense back to October 3 through the guarded expense update,
   without reloading the report or confirmation. Read-back verifies it is
   unbilled; the DOM still contains 1,000 rows and excludes that ID.
1. Confirm the existing action. Harvest reports 1,001 marked expenses. Fresh
   reads across all 21 ordinary expense-list pages verify all 1,001 fixture IDs
   billed, including the entry absent from the loaded report. The out-of-filter
   original remains unbilled/unlocked with its original amount and receipt.

Thus no selection targets the **current matching filter set at execution**, not
a snapshot of the loaded rows. The result discriminates those behaviors even
when the report exposes no pagination. It closes the previous target-set gate;
it does not establish an unlimited report size, non-owner authority or exports.
Do not continue increasing fixture size merely to search for a hypothetical
page boundary now that the actual behavioral distinction has been tested.

Selecting exactly the first and last fixtures afterwards sent only those two
IDs. Confirmation cleared two entries; a fresh 21-page read found two unbilled
and 999 still billed, with the original outside record unchanged. Explicit
selection therefore does not become filter-wide merely because filters remain
in the action URL.

### Entry billing and project-period protection

After clearing the two selected entries, the late expense was unbilled with no
invoice association but still locked. A fresh response identified the reason
as a project lock for that time period. The other 999 expenses remained billed.
Running the no-selection uninvoiced action for the same exact project/date
filter reported 999 changed entries; a fresh complete list read then found all
1,001 unbilled and unlocked, with the outside control unchanged.

Do not infer editability solely from `is_billed=false`, or promise that clearing
one row removes a project-period lock. This demonstrates the named state sequence,
not every interval derivation or overlap rule. The [unlocking guide](https://support.getharvest.com/hc/en-us/articles/4408204890381-Unlocking-invoiced-time-and-expenses)
separately describes entry and project-timeframe protection. No actual invoice,
approval or company-wide lock was created or changed by this experiment.

One additional synthetic duplicate with the first test note appeared outside
the creation checkpoint during setup. Its exact project/date/note/amount were
verified, separately ledgered and deleted before the final 1,001-row test.
Its cause was not established; it is not evidence of a Harvest idempotency defect.
All 1,001 checkpointed fixtures were subsequently deleted, each returning 200.
Created/deleted ID sets match exactly, with no duplicate cleanup IDs. Fresh
October 3 and October 4 project-filtered reports both returned empty states and
zero rows. The ordinary list contained only the original October 1 expense:
three units / USD 1.50, billable, unbilled, unlocked, no invoice, with its original
431-byte receipt. Its project remained active, no budget, inclusion/monthly/alerts
off. Exact checkpoints and before/after results remain in private scratch.
