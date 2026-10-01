# Expense cross-contract self-review

Checked 2026-10-01 against expense working artifacts, permissions snapshot
`f5cf02d`, dashboard snapshot `fa15ff4`, the existing project editor specification
and current `design/`. This is adversarial self-review, not independent review
or a completed `speckit-analyze`; final plan/tasks do not yet exist.

| Contract | Finding and evidence | Disposition / acceptance gate |
| --- | --- | --- |
| Lock reason × correction | High: FR-006 formerly prohibited billability changes under every lock. Project-archive owner correction succeeds, unlike invoice lock. | Corrected FR-006 and added archive scenario. Combined manual-billing/archive test proves billing clearance leaves archive lock intact. Feature 015 still owns actor authority and approval/admin combinations. |
| Category lifecycle | High: a disabled Delete control alone did not prove deletion safety. | Resolved for owner fixture: stale enabled control submitted DELETE and server refused an in-use category. Non-owner matrix remains open. |
| Exact monetary bounds | High: reference boundary messages are inconsistent with persisted responses and a large accepted amount loses cents. | Quantity/rate limits now recorded; do not copy lossy behavior. Ordinary amount boundary and overflow policy need an explicit exact-money contract before planning. |
| Currency and rate representation | High: two-decimal JPY amounts and three-decimal rates cannot be inferred from list formatting or casually represented as currency-native minor units. | Constitution exactness remains mandatory; resolve scaled representation explicitly without float arithmetic or silent truncation. |
| Draft versus source value | High: invoice-line amount is independent from tracked cost and may use a different currency. | Do not assert invoice + uninvoiced = current tracked total. Current dashboard US3 already recognizes independent values; expense attribution and currency integration tests remain necessary. |
| Receipt acceptance versus validity | High: a header-only PDF is accepted; accepted MIME/content recognition is not proof of a renderable document. | FR-003 now distinguishes acceptance from structure. Rendering failures must not lose source files or silently certify report completeness; final artifact contract remains open. |
| Privilege and revocation | High: ordinary read/write defaults cannot establish category administration, locked correction, invoice actions or receipt access. | Keep 015 as policy owner and do not translate owner success into custom grants. Suitable non-owner access is still required. |
| Report action target set | High: documented page scope disagrees with the empty-selection form shape; assuming shared all-pages code proves expense-report pagination was also too strong. | 101-fixture check verifies filter containment and all displayed records marked; report rendered one page while the expense list actually paginated at 50. Higher-volume report pagination remains unproven; Projects keeps disabled-empty Actions. Export/email authority is separate. |
| Budget integration | High: editor/dashboard coverage previously omitted expense budget consumption. | Resolved reference cases: enable/disable, both billability states, monthly work-date boundaries, total-hours exclusion and owner USD list/detail/report reconciliation. Shared contract propagated to editor FR-026 and dashboard FR-023 in their owning worktrees. Complete permission/currency matrix remains a dependency, not implemented behavior. |
| Cost denomination | High: editor FR-004's blanket organization-currency wording would relabel project-denominated expense costs. | Corrected to distinguish organization-denominated time costs and effective project/client expense costs. Reference fixture shows EUR zero time cost beside USD 1.50 expenses; nonzero unlike-cost aggregation still needs time/rate fixture authority. |
| Mixed-currency group presentation | Medium: assuming every subtotal is a currency breakdown would invent a client-group display. | USD/GBP fixture verifies separate weekly/report totals, N/A mixed-client subtotal and separate project subtotals; active-only filtering excludes the archived USD source. Added explicit acceptance case. |
| Input path equivalence | High: ordinary endpoint scientific-notation acceptance could become an incorrect UI guarantee. | UI quantity and ordinary fields normalize 1e3 to visibly displayed 13.00 and save 13, while the endpoint returns 1000. FR-018 now preserves the distinction; no second Horae mutation API implied. |
| Design and shared defaults | Medium: no dedicated expense prototype exists; this is not permission to omit the flow or change global control behavior. | User authorized existing component/token composition. Preserve shared project/time/settings defaults and require cross-screen regression in planning. No UI changes made in this phase. |

Budget source: [including expenses in project budgets](https://support.getharvest.com/hc/en-us/articles/360052763131-How-do-I-include-expenses-in-my-project-budget),
read 2026-10-01. Subsequent [live budget evidence](budget-evidence.md) verifies
the named cases; [budget-contract.md](budget-contract.md) separates them from
untested variants and authorization. No account-wide configuration or
notifications were changed.

Invoice-line correction/removal, draft-linked currency changes, combined
manual/archive locks, bounded report action scope, mixed-currency list/report
reconciliation and budget contribution now have persisted reference evidence.
The three consumer specs name the budget dependency; their historical plans/tasks
are not thereby reconciled. No final plan/tasks/analyze is claimed.

The independent-work goal remains active at this publication checkpoint. Next:
audit remaining receipt rendering and budget-variant evidence, then classify
every remaining gate by evidence, product conflict or unavailable authority.
Detailed-report pagination is still not exposed at tested volumes; do not call
that a passed multi-page case or turn it into endless equivalent probes.
Export email, non-owner permissions, alternate account locale, nonzero time/rate
fixtures and recipient/sent artifacts have separate authority/access boundaries.
Resolving findings in research is not implementation or runtime acceptance.
