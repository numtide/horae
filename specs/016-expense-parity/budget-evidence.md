# Expense inclusion in project budgets

Observed 2026-10-01 using only the disposable T&M project and its existing USD
1.50 expense. No time entries or business records were added. Email budget
alerts were explicitly unchecked before saving; no notification was requested.

## Total project fees

Selecting Total project fees exposed a budget amount, monthly-reset option,
expense-inclusion checkbox and email alert controls. The inclusion label covers
both billable and non-billable expenses. A USD 100 budget with inclusion enabled
and monthly reset disabled persisted with `cost_budget=100`,
`cost_budget_include_expenses=true`, `budget_is_monthly=false` and
`notify_when_over_budget=false`.

Because the fixture has no billing method/rates, Harvest asked for an additional
Save project anyway confirmation. Confirming saved the budget; merely clicking
Update project before that confirmation was not a successful save. The dashboard
then showed USD 98.50 remaining, total budget USD 100 and 1.5% used, despite zero
tracked hours. Its uninvoiced valuation separately reported missing billable
rates; that limitation did not suppress the known expense budget contribution.

Changing the sole source expense from billable to non-billable retained USD 1.50
and the dashboard's USD 98.50 remaining. Thus this inclusion rule is independent
of invoice eligibility, as documented. No rate, invoice or reimbursement state
was changed by that correction.

Disabling expense inclusion and saving returned USD 100 remaining / 0% used,
while internal costs still showed the USD 1.50 expense. Excluding an expense
from the budget therefore does not remove it from costs or reporting.

## Monthly boundary

Re-enabling inclusion with monthly reset persisted a USD 100 monthly budget.
The October 1 expense consumed USD 1.50. Moving that same non-billable fixture
to September 30 left the current October budget at USD 100 / 0% used, while
All time expense costs remained USD 1.50. Restoring October 1 and billability
returned the monthly remainder to USD 98.50. No timezone or organization-wide
period setting was changed; this is the reference account's September/October
work-date boundary, not proof of every timezone boundary.

## Hours budget and list reconciliation

Changing the same fixture to Total project hours, budget 100, monthly reset off
and expense inclusion off persisted `budget_by=project`, `budget=100`, with
alerts still disabled. The dashboard showed 100 remaining / 0% used and retained
USD 1.50 expense costs. The loaded project-list row independently showed budget
100, spent 0, remaining 100 (100%) and costs USD 1.50. Thus expense money did not
become tracked hours or disappear from costs. Per-task and per-person variants
were not exercised by this case.

The list initially rendered only the project name; values appeared after its
data loaded. The initial empty cells are not a contradictory financial result.

A subsequent fee-budget check re-enabled USD 100 / expense inclusion, with monthly
reset and email alerts off. The dashboard and fully loaded project list both
showed USD 1.50 spent, USD 98.50 remaining and 1.5% used. The fixture-filtered
detailed expense report for September 28–October 4 showed the same source ID,
three units, USD 1.50 row/client/overall totals. It reports expense value, not a
budget remainder. This reconciles the three surfaces for the single-currency,
owner-only fixture; it does not establish the permission or mixed-currency matrix.

## Restoration

The original fixture was restored to No budget. A fresh project response showed
`budget_by=none`, null hour/fee amounts, inclusion/monthly/alerts false. After
the later mixed-currency experiment it was active again; the source retained
October 1, three units / USD 1.50, billable, unbilled, unlocked, no invoice, with
its original downloadable 431-byte receipt. No time, rates or notifications
were created to establish these results.
