# Expense reference and clarification record

Checked: 2026-10-01. Stage: specify/clarify; not a completed technical plan.

## Evidence and limits

Baseline: Horae `9301112c6a02ae3c92716273534f38889db241d1`.
Route/spec inventory found no dedicated expense workflow. This is not a complete
Rust audit. The handoff inventory lists 14 screens and no expense-specific
prototype; expense references in Approvals/Settings do not define that screen.
`DESIGN.md` defines shared controls and the generated utility layer, which must
not be replaced or globally restyled to add this feature.

Official references, retrieved on the date above:

| Reference | Contract coverage | Limit |
| --- | --- | --- |
| [Tracking](https://support.getharvest.com/hc/en-us/articles/360048687611-Tracking-expenses) | FR-002/003 and billability | Does not settle upload limits or numeric precision |
| [Categories](https://support.getharvest.com/hc/en-us/articles/360048686731-Managing-expense-categories) | FR-004 lifecycle and repricing | Fractional, archived-resave, mode-change and stale delete-in-use server rejection verified; non-owner authorization remains open |
| [Editing](https://support.getharvest.com/hc/en-us/articles/4406054281101-How-to-edit-expenses) | FR-006 privileged locked corrections | Does not explain consequences for an issued invoice |
| [Locks](https://support.getharvest.com/hc/en-us/articles/360048687491-Unlocking-time-and-expenses) | FR-007 independent protection and date coverage | Reconcile with feature 015 before planning |
| [Preferences](https://support.getharvest.com/hc/en-us/articles/360048179912-Customizing-account-preferences) | FR-008 module/reimbursement and currency context | Existing configuration scope is owned by 013 |
| [Detailed reports](https://support.getharvest.com/hc/en-us/articles/360048687171-Detailed-time-and-detailed-expense-reports) | FR-009/010 filters, downloads and manual billing status | Legacy role wording does not settle the six-profile matrix |
| [Expense object](https://help.getharvest.com/api-v2/expenses-api/expenses/expenses/) | FR-005 identity and distinct state fields | An API description is not proof of browser behavior or authority to expand Horae's compatibility API |
| [Invoice editing](https://support.getharvest.com/hc/en-us/articles/360048181012-Editing-and-deleting-invoices-and-estimates) | FR-010 invoice-to-source behavior | Does not settle the reverse direction: changing/deleting a source expense |
| [Currencies](https://support.getharvest.com/hc/en-us/articles/360055384512-Does-Harvest-support-multiple-currencies) | FR-011 manual conversion | Unbilled and draft-linked historical relabeling verified; sent-state and mixed-currency reconciliation remain unresolved |
| [Current permissions](https://support.getharvest.com/hc/en-us/articles/44171549176077-Permissions) | FR-014 read/write scopes | Corroborated by delivered editor configuration, not lifecycle enforcement |
| [Invoice attachments](https://support.getharvest.com/hc/en-us/articles/9864825272589-Attaching-files-and-reports-to-invoices) | FR-015 recipient report and draft regeneration | Source correction/deletion and sent artifacts remain unverified |
| [Receipt announcement](https://www.getharvest.com/blog/2010/05/upload-expense-receipts-in-harvest) | Historical 10 MB limit | Published 2010; not proof of current exact byte boundary |

The expense reference describes quantity as integer but examples serialize it
as `1.0`. Those examples alone do not prove precision. Subsequent
[persisted web experiments](numeric-evidence.md) confirm two-decimal quantities,
three-decimal rates and quantity-before-total rounding for the tested USD cases.
Do not choose a lossy representation from the API label. Existing exact-money
invariants remain mandatory; the plan must reconcile fractional rates explicitly.

Further research separates two directions that must not be conflated: invoice
line edits do not rewrite the original tracked expense, but the cited invoice
article does not establish what a privileged source-expense edit does to an
existing invoice. FR-010 now covers the documented direction; EXP-E02 remains
partially open for the reverse direction. A subsequent owner-only draft fixture
confirms source correction/deletion leaves the draft line and total unchanged;
sent-state, attribution and report consequences remain open.

The fetched currency page contains an internal conflict: its project-specific
currency section permits an override, while its expense FAQ describes client-only
currency. Subsequent [client inspection](reference-validation.md) resolves the
current form/row/weekly-total priority in favor of project overrides, corroborated
by the persisted EUR-client/USD-project fixture. Subsequent historical tests
confirm current-currency relabeling without conversion, project override removal
and client inheritance. JPY/BHD accepted precision differs from assumptions based
on display; see currency-evidence. Neither incomplete search excerpts
nor client formatting alone establish those rules.

### Browser observation

Used Horae's existing stdio Playwright MCP client with Windows Chrome. Created a
dedicated Harvest tab; the unrelated existing tab was left intact. Opened the
authenticated Expenses screen, which showed an empty state, Track expenses and
All expenses/Categories tabs. Navigation also exposed Time off (Beta), a new
inventory candidate requiring separate research.

Local, uncommitted evidence in the main worktree:

- `.scratch/playwright-windows/expense-reference-20261001.md`
- `.scratch/playwright-windows/expense-form-reference-20261001.md`
- `.scratch/playwright-windows/expense-categories-loaded-20261001.md`

The second snapshot is **not** evidence of an opened form: the initial click
timed out. A later visible-control selection did open the form, recorded in
[reference-validation.md](reference-validation.md). Unsaved numeric experiments
were inconsistent and do not establish acceptance or rounding. After isolated
fixture authorization, an expense and unit category were created and edited;
the persisted follow-up supersedes matching unsaved experiments. Reimbursement
and non-owner enforcement remain untested. Console errors were reported; no
cause has been established.

The Categories tab also timed out on click. Navigating directly to its observed
link succeeded: the category list exposes New category and per-category Edit,
Archive and Delete controls, including a mileage rate display. Visibility is not
proof that these mutations, rate validation or restrictions work. Later tests
verified creation, rate validation, archive/resave/restore and mode changes;
a subsequent stale enabled Delete request returned 422, establishing server
enforcement for the owner fixture. Current
navigation places Categories under Expenses,
unlike the older Manage menu wording in the help article.

## Clarification scan

`speckit-clarify` resolved this feature using the checked-in prerequisite helper.
One product question was presented and answered: the user authorized composing
missing screens from the current Horae design system while preserving Harvest
behavior. Recorded in the spec's clarification, FR-013 and assumptions.

| Category | Status | Next action |
| --- | --- | --- |
| Functional scope | Clear | Native expense workflow included by confirmed web scope |
| Domain and data | Partial | USD/JPY/BHD response precision and historical relabeling verified; finish bounds, exports, attribution and exact-representation constitution check |
| Interaction and UX | Partial | Persisted capture/correction and single-row selection verified; multi-page report scope conflicts with help wording; detailed composition remains |
| Non-functional quality | Partial | Upload byte boundary/failure preservation verified; scale and final receipt privacy/retention contract required |
| Dependencies | Partial | Complete 015 matrix and assign invoice/report owners |
| Edge cases | Partial | Turn remaining numeric, attachment and correction cases into exact expectations |
| Constraints | Clear | Self-hosted architecture, exactness, no new integration or application/business-data changes; isolated reference fixtures authorized |
| Terminology | Clear | Billing, approval, reimbursement and invoice association remain distinct |
| Completion signals | Partial | SC-001–005 defined; unresolved contracts prevent full acceptance coverage |
| Placeholders | Partial | Three clarification markers remain: numeric/billing, lifecycle permissions and multi-page report scope; investigate before asking the user to guess |

### Open work

- **EXP-D01 — Resolved product/design**: On 2026-10-01 the user authorized
  composing missing surfaces from current Horae components/tokens, preserving
  Harvest behavior. No new handoff is required. FR-013 requires a planning-time
  composition/control-state mapping and shared-screen regression checks.
- **EXP-E01 — Numeric evidence**: USD amounts, fractional quantities/rates,
  rounding order, signs and active-category repricing now have persisted
  evidence, including JPY/BHD responses and historical currency changes. UI zero
  rejection differs from endpoint acceptance. Tiny-rate rounding before positive
  validation, quantity/rate bounds and current-locale separators are verified.
  Ordinary-amount bound messages conflict with persisted acceptance and one
  large response loses cents; resolve an exact-money contract instead of
  copying reference precision loss. Finish other locales, other currencies and
  export/mixed-currency reconciliation using only authorized disposable fixtures.
- **EXP-E02 — Billing contract**: Define privileged correction/deletion effects
  on source links, recipient reports and already issued totals, with the billing
  owner. Draft line/amount independence is now verified after both source
  correction and deletion. Keep manual billed status separate from a real
  invoice relationship, and do not generalize owner-only results to other grants.
  Manual mark/clear and non-billable refusal now have owner browser evidence.
  Deleting a second fixture draft released billing/lock state while retaining the
  source and receipt. A third draft verifies price edits leave source values
  unchanged, source currency relabeling leaves draft currency/value unchanged,
  and saved line removal releases the source while retaining its receipt and
  the now-empty draft. The current account's Preview explicitly requires paid
  access or Stripe; no bypass, upgrade or integration is authorized. Report
  generation was requested, but its content/regeneration could not be certified.
- **EXP-E03 — Permission dependency**: Extend 015 to expense operations and
  receipt downloads. Six read/write grants and profile defaults are now observed
  and recorded with the owner; lifecycle mapping and non-owner enforcement remain
  open. Do not translate legacy Manager wording into an unverified grant.
- **EXP-E04 — Receipt contract**: Size, replacement failure, removal, benign
  content checks, MIME normalization and owner access after category archive are
  verified. Header-only PDF acceptance and signature-only PNG rejection now
  distinguish upload recognition from structural validity. Finish rendering
  failure, download revocation,
  storage/backup retention and recipient report lifecycle. Never leave
  attachments outside organization/expense access checks.
- **EXP-E05 — Report scope and export delivery**: A single-row report proves
  reconciliation, confirmation/cancel and manual billing but cannot prove
  pagination. Help says page scope; the delivered empty-selection form sends
  filters without IDs. Resolve with a multi-page fixture before fixing the
  target-set contract. Export generation can deliver email and was not submitted
  under the no-messages authorization; columns/routes were inspected only.

These gates block final planning/tasks/analysis, not independent research.
The concrete distinguishing checks and evidence required are recorded in
[reference-validation.md](reference-validation.md); they are not implementation
tasks. Only the user's explicit disposable-fixture authorization permits the
listed reference mutations; it does not permit existing-record changes.

## Preliminary adversarial self-review

This is a draft self-review, not independent review or `speckit-analyze`.
The follow-up [cross-contract review](cross-contract-review.md) identifies
lock-specific billability, numeric-bound conflicts, receipt structure and the
missing project-budget inclusion dependency. FR-006 is corrected and FR-019
adds the documented budget requirement; the remaining findings stay open.

| Finding | Severity | Disposition |
| --- | --- | --- |
| Financial correction/deletion could diverge from issued invoice totals | High | Open EXP-E02; no invented cascade or silent invoice rewrite |
| New expense grants could bypass approved scoped permissions | High | Open EXP-E03; matrix completion is a prerequisite |
| Incomplete numeric bounds/currency rules could create inconsistent totals | High | FR-017 resolves tested arithmetic; remaining EXP-E01 cases and exact-rate constitutional reconciliation still gate planning |
| Display precision could discard JPY fractions or invent BHD precision | High | Currency evidence distinguishes two-decimal accepted values from list formatting; exact representation must pass the plan's constitution check |
| Treating current expense currency as a frozen historical snapshot would diverge from reference | High | FR-011 now requires current project/client relabeling without FX; invoice currency is independently persisted |
| A receipt URL could leak confidential material after revocation | High | FR-003/012 require current authorization; EXP-E04 contract still incomplete |
| A blanket internal receipt rule could either expose source records to clients or omit supported invoice attachments | High | FR-015 separates recipient reports; sent/source-change retention remains open with EXP-E02/04 |
| Client formatting could be mistaken for accepted precision or negative support | High | Addressed for recorded cases by separate UI/endpoint evidence, read-back and explicit zero divergence; do not extrapolate to other currencies or inputs |
| Empty selection could affect unseen pages while the specification promises page-only scope | High | Open EXP-E05; removed the unsupported guarantee from FR-016 and acceptance scenarios; a multi-page reference test is required. Projects remains unchanged |
| Missing prototype could lead to omission or unrelated global CSS changes | Medium | EXP-D01 resolved; FR-013/SC-005 require existing design-system composition, shared defaults and cross-screen regression; execution remains future work |

## Spec Kit execution record

- Read `speckit-specify` and `speckit-clarify` completely.
- Inspected local/remote branches and feature directories; 016 was available.
- Created isolated `feat/expense-parity` worktree from master, independent of
  existing feature owners; no existing feature was copied into this branch.
- Resolved `spec-template` through `common.sh`'s template resolver; the core
  template is active. Ran feature creation helper in dry-run mode with number
  16, then wrote the template-based specification and `.specify/feature.json`.
- No `.specify/extensions.yml` exists; before/after hooks are inapplicable.
- Ran `check-prerequisites.sh --json --paths-only` for clarification and reviewed
  the resulting spec with the coverage taxonomy above.
- After the user's design authorization, reran clarification path resolution
  and integrated the answer. Checklist remains 11/16, with no newly passing or
  regressing items: numeric, receipt, permission and billing contracts still
  prevent full readiness. The checklist's original FR-013 note is historical;
  its content is preserved by the clarify workflow's marker-only update rule.
- `speckit-plan`, `speckit-tasks` and `speckit-analyze` are not complete or claimed
  by the existence of this research file. Implementation tasks remain uncreated
  until the blocking contracts are resolved.
- Follow-up clarification iteration: reread the skill and constitution, reran
  prerequisite resolution once, checked absent extension hooks, inspected the
  live form/current permission catalog and integrated evidence in FR-003/009–016,
  story 3 and edge cases. No new product question has been answered. Revalidated
  every checklist item: still 11/16, no newly passing items or regressions. Do not
  mark readiness merely because the evidence register is more complete.
- Historical next action before authorization: obtain authority for isolated disposable reference records and
  suitable non-owner access for E-AUTH; execute the discriminating checks before
  closing the numeric, attachment and source-billing contracts. No application
  change, migration, reference save or merge occurred in this iteration.
- Authorization follow-up before Chrome restoration: the user approved isolated fixture mutations, now
  recorded in Clarifications and the reference-validation scope. Created one
  test client/project and verified the persisted EUR client / USD project pair.
  No expense or invoice has been saved. Fixture IDs and cleanup ledger remain in
  local scratch. Browser visibility is currently hidden despite tab selection;
  requested restoration of Chrome, retaining the same live connection and
  fixtures. Next: once visible, save/reopen the first fixture expense, then run
  E-NUM/E-CUR. E-AUTH still needs suitable non-owner access, not another fixture
  authorization. Checklist remains 11/16; authorization does not prove a rule.
- Persisted follow-up after restoration: reused the same client/project and
  connection; saved one expense and unit category. Verified project USD override
  over client EUR, ordinary and quantity rounding/signs, three-decimal rates,
  quantity-before-total rounding and historical repricing on unchanged-field
  resave. Recorded exact cases in numeric-evidence and FR-017, keeping UI and
  endpoint acceptance distinct. Synthetic receipt boundary checks establish a
  strict limit below 10,485,760 bytes. No invoice, existing-record change,
  notification, payment, application implementation, migration or merge.
  Checklist re-evaluated: still 11/16, no marker changes. Its earlier no-persisted-
  expense note is historical, preserved by clarify's marker-only rule. Remaining
  work includes source billing, receipt lifecycle/access, currency history and
  operation-level permissions. Fixture cleanup remains tracked in local scratch.
- Draft billing follow-up: created one fixture-only draft through the UI (never
  sent/marked sent), then changed source quantity 2 to 4 and its note. Expense
  became USD 2.00; invoice retained its original quantity 2, description and USD
  1.00 total. Permanent source deletion likewise preserved the draft. Updated
  FR-006/011 and acceptance coverage; billing-evidence separates this proof from
  still-unverified attribution, reports and sent-state behavior. Deleted only the
  disposable expense, not the invoice or existing business data; exact remaining
  cleanup targets are in scratch. Invoice review's client-currency default is
  observed, but cross-currency persistence is not. Checklist remains 11/16; no
  new product questions or hook executions. Next: currency/history and remaining
  receipt/report checks; E-AUTH still needs suitable non-owner access.
- Independent-test iteration: reread clarify, repository instructions and
  constitution; resolved prerequisites once and checked absent hooks. No product
  questions asked/answered. Created one new disposable expense (previous source
  was deleted), verified JPY/BHD response precision, historical currency labels,
  project/client inheritance and restored the original EUR/USD fixture settings.
  Saved a second draft with EUR default from a USD source, then deleted that
  exact draft through confirmation; source and receipt survived with billing
  locks released. Preview reported an explicit paid-plan/Stripe requirement;
  no report content/generation success is claimed and no account upgrade or
  integration was attempted. Verified tiny positive rate boundary, category
  archive, existing-entry resave and receipt access; new capture with the archived
  category returned 422. Recorded evidence and narrowed FR-011/017; all fixtures
  remain identified in scratch. No implementation, migration, sent invoice,
  payment, business-record change or merge. Checklist revalidated 11/16→11/16;
  no passing-state changes. Remaining requirements, complete acceptance coverage
  and readiness stay unchecked because unresolved contracts remain. Next:
  locale/bounds, exports and receipt content cases; non-owner/recipient evidence
  requires suitable reference access, not another generic fixture authorization.
- Independent continuation: reread clarify and constitution, ran path resolution
  once and confirmed no extension hooks. Previous status-only turn classified
  as no progress; reused the live browser handle and current fixture read-back
  instead of restarting. No product questions asked or answered. Verified
  current-locale `1,25`→125 through a UI save and separate ordinary endpoint
  cases; larger amount samples through 100000000000000 accepted, no maximum
  claimed. Verified empty/disguised-file rejection, PNG/GIF/JPEG acceptance and
  MIME normalization, then restored the original synthetic PDF and fields.
  Verified manual billing/clearing, non-billable refusal and ordinary↔unit
  category history/resave behavior. All surviving fixtures restored to the
  ledger's baseline; no new identity, message, payment, deletion, migration,
  application change or merge. Export generation was not submitted because its
  delivery path may send email; UI columns/routes are evidence, file bytes are
  not. Self-review found an unsupported page-only bulk guarantee: corrected
  FR-016 and acceptance coverage, recorded the live-form/documentation conflict
  as EXP-E05 rather than asserting a scope from a one-row fixture. Checklist
  re-evaluated 11/16→11/16, no marker changes: unresolved requirements, complete
  acceptance coverage and readiness remain unchecked. Next independent work:
  archived rate editing/change-away-return, project archive, bounded numeric
  and recognizable malformed-file cases, invoice-line removal/currency effects,
  and a multi-page scope test followed by cross-contract review. Export bytes,
  non-owner enforcement and recipient artifacts retain distinct authority/access
  gates. Do not mark the active independent-work goal complete at this checkpoint.
- Lifecycle/bounds iteration: prior recommendation-only turn classified as no
  progress; recovered the live browser handle's pending result before any new
  mutation. Read clarify/constitution, resolved prerequisites once and confirmed
  hooks absent. No product question asked or answered. Verified project archive
  correction/new-capture refusal, receipt survival and active-only reporting;
  restored the project and expense. Integrated quantity/rate bounds, conflicting
  ordinary-amount boundaries, header-only PDF/signature-only PNG results,
  archived category change-away/return and stale Delete rejection. FR-003/004/006/
  011/017 and acceptance scenarios now reflect those distinctions. Created one
  disposable draft; line price correction and project currency change left
  source/invoice values independent. Saved removal of its sole line released
  source billing and preserved the receipt; the draft remains empty and unsent.
  No real record, account configuration, message, payment or integration changed.
  Added a cross-contract self-review; corrected the blanket lock-field rule and
  added FR-019 for documented budget inclusion. Checklist revalidated 11/16→11/16,
  no marker changes: unambiguous requirements, complete acceptance and readiness
  still fail. No final plan/tasks/analyze or implementation claimed. Next safe
  checks: multi-page scope, combined locks, mixed-currency reconciliation and
  project-budget inclusion, then reconcile remaining cross-contract findings.
  Alternate locale/account access, recipient artifacts and export email authority
  remain separate gates. Final independent-work completion is not yet proven.
