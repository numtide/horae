# Shared expense budget contribution

Status: behavioral contract grounded in the documented rule and isolated
fee-budget fixture. Not a completed technical plan or implemented capability.

## Ownership and dependencies

- Feature 016 owns expense amount, work date, billability and lifecycle inputs.
- Feature 011's shared project editor owns budget kind, amount, period and the
  expense-inclusion setting. This is a project option, not a category property.
- Project list/detail and reporting consume the same applicable contribution;
  do not compute a different number independently on each screen.
- Feature 015 owns disclosure and mutation authority. A budget total must not
  provide access to expense notes or receipt bytes outside that authority.
- Notification delivery stays with existing budget-alert contracts; these
  reference tests kept alerts off and do not validate email delivery.

## Rules established

1. Expense inclusion applies to Total project fees budgets when enabled. It
   includes both billable and non-billable expenses. Budget inclusion must not
   make a non-billable expense invoice-eligible.
1. Disabling inclusion removes expense amounts from budget consumption, not
   from expense records, internal costs or their ordinary report scope.
1. Monthly budget consumption follows the expense work date within the relevant
   month. A prior-month expense still appears in All time costs without consuming
   the current month's allowance.
1. Evaluate amounts exactly in the applicable currency. Do not mix currencies,
   convert money into hours or infer current tracked expense cost from a saved
   invoice line amount. Expense costs follow effective project/client currency;
   time costs retain their own organization-denominated contract. A shared cost
   label must not relabel unlike amounts into one currency. Mixed-currency list
   and report presentation follows [currency-evidence.md](currency-evidence.md);
   nonzero mixed time-cost/expense-cost aggregation remains separately gated.
1. A source correction affects current expense-based budget consumption. It
   must not silently rewrite a saved invoice. Archive preserves history; it is
   not deletion or an instruction to remove historical budget consumption.

## Traceable integration acceptance

| Case | Requirement | Expected result / evidence status |
| --- | --- | --- |
| USD 100 fee budget, inclusion on, USD 1.50 billable expense, no time | FR-019, SC-003 | Remaining USD 98.50; persisted reference verified |
| Same expense becomes non-billable | FR-010/019 | Remaining USD 98.50; no new invoice eligibility; persisted reference verified for budget |
| Inclusion off | FR-019 | Remaining USD 100; expense costs remain USD 1.50; persisted reference verified |
| Monthly inclusion on, source October 1 → September 30 | FR-019 | October remaining USD 100; All time costs USD 1.50; persisted reference verified |
| Restore source October 1 | FR-019 | October remaining USD 98.50; persisted reference verified |
| Total project hours budget 100, no time | FR-019 | List/detail remaining 100, spent 0, expense costs USD 1.50; persisted reference verified |
| Task/person-scoped budgets | FR-019 | No automatic expense consumption; documented rule, variants not exercised |
| Fee-budget list/detail and detailed expense report | FR-019, SC-003 | USD 1.50 contribution, USD 98.50 remainder on budget surfaces and USD 1.50 report total; owner-only USD fixture verified |
| Permitted and forbidden actors; multiple currencies | FR-012/014/019, SC-002/003 | Same authorized contribution and currency semantics; complete permission and cross-currency matrix remains open |

Planning must trace these cases into the editor, dashboard and reporting tasks,
preserving time-only budgets and existing alert behavior. A package must not
claim full expense parity merely because capture and invoicing are specified.
Evidence: [budget-evidence.md](budget-evidence.md). No budget feature or migration
was implemented by writing this contract.
