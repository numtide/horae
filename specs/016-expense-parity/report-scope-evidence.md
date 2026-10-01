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
