# Independent reference-work audit

Checked 2026-10-01, including the final changed-result-set experiment and cleanup;
dashboard dependency checkpoint remains `48a4156`.
This audits the requested independent expense investigations and shared-contract
consolidation, not the much larger complete-web-parity delivery. A reference
test, a clarified requirement and a working Horae feature are different outcomes.

## Evidence against the named work

| Investigation | Current evidence | Disposition |
| --- | --- | --- |
| Category archive, rate/mode changes and stale deletion | category-evidence.md | Owner fixtures verified; non-owner authority is a separate gate |
| Project archive and independent lock clearance | archive-evidence.md | Manual billing clearance preserves archive lock; restore separately releases it |
| Numeric signs, precision, current locale and bounds | numeric-evidence.md | Quantity/rate bounds and UI normalization verified; ordinary bounds remain a reference conflict, not an exact accepted interval |
| Receipt limits, content, removal and correction failure | receipt-evidence.md | Bounded fixtures verified; internal PDF download distinguished from generated-report rendering |
| Draft line/source independence and billing release | billing-evidence.md, currency-evidence.md | Verified for draft correction, deletion and line removal, not sent artifacts |
| Report action target set | report-scope-evidence.md | Resolved: 1,000 loaded rows plus a later-matching entry are all marked; two selected IDs clear only two; out-of-filter control preserved. No selection uses current filter matches, not loaded rows |
| Mixed-currency reconciliation | currency-evidence.md | USD/GBP weekly/report totals, mixed-client N/A, project subtotals and active-only filtering verified; no export-byte claim |
| Expense budget contribution | budget-evidence.md, budget-contract.md | Fee inclusion, billability, month boundary, total-hours exclusion and list/detail/report reconciliation verified; other modes have documented/UI evidence |
| Consumer contract propagation | editor FR-026/FR-004 in #214, dashboard FR-023 in #208 | Budget contribution and cost denomination reconciled; historical implementation tasks do not cover new requirements |
| Adversarial review | cross-contract-review.md | Self-review corrected lock, money, group-presentation and budget omissions; high unresolved contracts remain explicitly open |
| Fixture preservation | Private exact-ID ledger and fresh browser read-backs | Original source/431-byte receipt preserved. Earlier 101-row and GBP expenses removed; latest 1,001-row lot plus one separately verified synthetic duplicate deleted. October 3/4 reports empty; original active/no-budget project unchanged |

## Residual gates, not equivalent repeat tests

- **Resolved — detailed-report target set:** the later-matching-record
  experiment distinguishes live filters from loaded rows without relying on a
  paginator. The 1,001-row report itself has no pagination controls; neither an
  unlimited size nor a multi-page UI was certified. No further size escalation
  is needed to resolve this behavioral distinction.
- **Product/exactness conflict — ordinary bounds:** observed acceptance conflicts
  with error limits and loses cents at a large value. Quantity/rate contracts
  are settled; ordinary limits need an explicit exact-money resolution. Do not
  copy the precision loss or silently choose one inconsistent reference signal.
- **Export delivery authority:** generation may send email. Control inspection
  is complete; bytes and receipt-rendering failure in generated exports remain
  untested until that delivery is authorized.
- **Reference access:** non-owner/custom grants, archived people and approval
  combinations require suitable actors. The current owner cannot stand in for
  them. Invoice recipient/Preview and sent-state artifacts have their own gates;
  no upgrades, integrations or sent invoices are authorized.
- **Fixture authority:** changing account locale or creating time/rate records
  for nonzero unlike-cost aggregation is outside the authorized expense fixture
  scope. Current-locale and expense-only currency checks do not prove those cases.
- **Self-hosted policy:** receipt/storage retention and backup erasure need a
  Horae contract; the Harvest download route cannot prove provider backup policy.

The named independent investigations and cross-contract propagation are now
complete at the evidence level stated in the table. The previously outstanding
target-set distinction has a discriminating result, not merely another larger
one-page test. Entry versus project-period protection was also verified and
propagated to feature 015's evidence without inventing non-owner permissions.
Final delivery requires these documents and the coordination record to be
published and their PR revisions verified; no merge is authorized.

This does **not** complete the expense specification, its implementation or the
full Harvest-parity delivery. The remaining items above need product policy,
delivery authority or suitable reference access. Reusable fixture client,
category, original project/expense and unsent draft invoices remain identified
in the private ledger for those gated tests; the disposable high-volume lot has
no outstanding cleanup. Do not recreate or erase fixtures to manufacture a
cleaner readiness status. Next clarification should resolve the exact-money
conflict, then the other explicit policy/access gates; no final plan/tasks/analyze
success is claimed while those prerequisites remain unresolved.

## Clarification coverage and completion report

One product question was asked at handoff, with no answer accepted: use the
reference's advertised ordinary-amount limits with exact arithmetic, or agree
different limits. It is not recorded as an accepted clarification. Expense spec sections changed:
User Scenarios & Testing, Edge Cases and Functional Requirements. Requirements
checklist: 11/16 before and after, no newly passing or regressing markers.
Remaining unchecked: no clarification markers, unambiguous requirements,
complete acceptance scenarios, complete requirement acceptance and readiness.
No extension hooks exist. Do not run a final plan/tasks/analyze chain yet.

| Clarification category | Status |
| --- | --- |
| Functional scope and behavior | Clear scope; permission-dependent operations remain under Dependencies |
| Domain and data | Partially resolved; ordinary-money and retention contracts remain gated |
| Interaction and UX | Named budget/currency/input flows and live-filter versus explicit-ID target sets resolved; detailed composition belongs in planning |
| Non-functional quality | Receipt boundary established; authorization/retention and final scale contract open |
| Integration and external dependencies | Export delivery, permissions and invoice artifact gates open |
| Edge cases and failure handling | Named independent fixtures resolved; permission/artifact cases remain gated |
| Constraints and tradeoffs | Clear; exactness and mutation authority unchanged |
| Terminology and consistency | Clear; budget consumption, cost, invoice value and billing locks kept distinct |
| Completion signals | Criteria exist; full coverage/readiness not achieved |
| Placeholders | Two clarification markers remain intentionally open: numeric/billing/locale and lifecycle permissions |

Publication verification and the final independent-work disposition belong to
the coordination register in PR #213, which records the exact published feature
revisions. The completion audit preserves the broader delivery's unchecked gates.

Suggested continuation: `speckit-clarify` for the named unresolved contracts,
then `speckit-plan` only after its blocking prerequisites are actually resolved.
