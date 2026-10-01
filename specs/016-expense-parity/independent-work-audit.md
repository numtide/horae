# Independent reference-work audit

Checked 2026-10-01 after expense checkpoint `7f193af` and dashboard `48a4156`.
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
| Report action filter containment | report-scope-evidence.md | 101-row action and out-of-filter preservation verified; report had one page, so larger-volume scope remains unresolved |
| Mixed-currency reconciliation | currency-evidence.md | USD/GBP weekly/report totals, mixed-client N/A, project subtotals and active-only filtering verified; no export-byte claim |
| Expense budget contribution | budget-evidence.md, budget-contract.md | Fee inclusion, billability, month boundary, total-hours exclusion and list/detail/report reconciliation verified; other modes have documented/UI evidence |
| Consumer contract propagation | editor FR-026/FR-004 in #214, dashboard FR-023 in #208 | Budget contribution and cost denomination reconciled; historical implementation tasks do not cover new requirements |
| Adversarial review | cross-contract-review.md | Self-review corrected lock, money, group-presentation and budget omissions; high unresolved contracts remain explicitly open |
| Fixture preservation | Private exact-ID ledger and fresh browser read-backs | Original source/431-byte receipt restored; 101 bulk-test expenses and GBP expense deleted; GBP project archived; original project active/no budget/alerts off |

## Residual gates, not equivalent repeat tests

- **Reference uncertainty — detailed-report target set:** no report pagination
  was exposed at 2, 51 or 101 records, and page/per_page probes did not create
  it. Public help and delivered shared selection code do not establish a
  higher-volume threshold. Do not call the multi-page requirement complete.
  A next discriminating check needs a justified larger-volume fixture or
  authoritative web-report evidence; API v2 pagination is not that evidence.
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

This audit does not mark the independent-work goal or expense package complete:
the larger-volume target-set requirement remains unproven and the contract
gates must retain their ownership. Next investigate that remaining reference
uncertainty without repeating already-closed 101-row, currency or budget tests.

## Clarification coverage and completion report

No new product questions were asked or answered. Expense spec sections changed:
User Scenarios & Testing, Edge Cases and Functional Requirements. Requirements
checklist: 11/16 before and after, no newly passing or regressing markers.
Remaining unchecked: no clarification markers, unambiguous requirements,
complete acceptance scenarios, complete requirement acceptance and readiness.
No extension hooks exist. Do not run a final plan/tasks/analyze chain yet.

| Clarification category | Status |
| --- | --- |
| Functional scope and behavior | Clear scope; permission-dependent operations remain under Dependencies |
| Domain and data | Partially resolved; ordinary-money and retention contracts remain gated |
| Interaction and UX | Named budget/currency/input flows resolved; larger-volume report target set open |
| Non-functional quality | Receipt boundary established; authorization/retention and final scale contract open |
| Integration and external dependencies | Export delivery, permissions and invoice artifact gates open |
| Edge cases and failure handling | Named independent fixtures resolved; permission/artifact cases remain gated |
| Constraints and tradeoffs | Clear; exactness and mutation authority unchanged |
| Terminology and consistency | Clear; budget consumption, cost, invoice value and billing locks kept distinct |
| Completion signals | Criteria exist; full coverage/readiness not achieved |
| Placeholders | Three clarification markers remain intentionally open |

Suggested continuation: `speckit-clarify` for the named unresolved contracts,
then `speckit-plan` only after its blocking prerequisites are actually resolved.
