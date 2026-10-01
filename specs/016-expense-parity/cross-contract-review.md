# Expense cross-contract self-review

Checked 2026-10-01 against expense working artifacts, permissions snapshot
`f5cf02d`, dashboard snapshot `fa15ff4`, the existing project editor specification
and current `design/`. This is adversarial self-review, not independent review
or a completed `speckit-analyze`; final plan/tasks do not yet exist.

| Contract | Finding and evidence | Disposition / acceptance gate |
| --- | --- | --- |
| Lock reason × correction | High: FR-006 formerly prohibited billability changes under every lock. Project-archive owner correction succeeds, unlike invoice lock. | Corrected FR-006 and added archive scenario. Feature 015 must map actor authority separately; combined locks remain a required test. |
| Category lifecycle | High: a disabled Delete control alone did not prove deletion safety. | Resolved for owner fixture: stale enabled control submitted DELETE and server refused an in-use category. Non-owner matrix remains open. |
| Exact monetary bounds | High: reference boundary messages are inconsistent with persisted responses and a large accepted amount loses cents. | Quantity/rate limits now recorded; do not copy lossy behavior. Ordinary amount boundary and overflow policy need an explicit exact-money contract before planning. |
| Currency and rate representation | High: two-decimal JPY amounts and three-decimal rates cannot be inferred from list formatting or casually represented as currency-native minor units. | Constitution exactness remains mandatory; resolve scaled representation explicitly without float arithmetic or silent truncation. |
| Draft versus source value | High: invoice-line amount is independent from tracked cost and may use a different currency. | Do not assert invoice + uninvoiced = current tracked total. Current dashboard US3 already recognizes independent values; expense attribution and currency integration tests remain necessary. |
| Receipt acceptance versus validity | High: a header-only PDF is accepted; accepted MIME/content recognition is not proof of a renderable document. | FR-003 now distinguishes acceptance from structure. Rendering failures must not lose source files or silently certify report completeness; final artifact contract remains open. |
| Privilege and revocation | High: ordinary read/write defaults cannot establish category administration, locked correction, invoice actions or receipt access. | Keep 015 as policy owner and do not translate owner success into custom grants. Suitable non-owner access is still required. |
| Report action target set | High: documented page scope disagrees with the empty-selection form shape. | FR-016 remains gated on a multi-page test; Projects keeps disabled-empty Actions. Export/email authority is a separate issue. |
| Budget integration | High: current editor/dashboard specs do not explicitly cover expenses in project budget consumption. Official reference supports expenses only in Total project fees budgets when the inclusion option is enabled. | Add explicit shared requirement and integration cases, including billable and non-billable expenses, enable/disable and period boundaries. Do not add expenses to hours budgets or equate budget inclusion with invoice eligibility. |
| Design and shared defaults | Medium: no dedicated expense prototype exists; this is not permission to omit the flow or change global control behavior. | User authorized existing component/token composition. Preserve shared project/time/settings defaults and require cross-screen regression in planning. No UI changes made in this phase. |

Budget source: [including expenses in project budgets](https://support.getharvest.com/hc/en-us/articles/360052763131-How-do-I-include-expenses-in-my-project-budget),
read 2026-10-01. This establishes the documented inclusion rule, not a passed
live budget test. No account-wide configuration or notifications were changed.

Invoice-line correction/removal and draft-linked currency changes now have
persisted evidence in billing/currency files. The independent-work goal is not
complete: multi-page and combined-lock checks, remaining reconciliation and
the identified budget dependency must be
accounted for. Access/authority gates must remain distinct from unfinished safe
work. Resolving findings in research is not implementation or runtime acceptance.
