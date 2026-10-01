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
| [Categories](https://support.getharvest.com/hc/en-us/articles/360048686731-Managing-expense-categories) | FR-004 lifecycle and repricing | Need a fractional-rate/quantity fixture |
| [Editing](https://support.getharvest.com/hc/en-us/articles/4406054281101-How-to-edit-expenses) | FR-006 privileged locked corrections | Does not explain consequences for an issued invoice |
| [Locks](https://support.getharvest.com/hc/en-us/articles/360048687491-Unlocking-time-and-expenses) | FR-007 independent protection and date coverage | Reconcile with feature 015 before planning |
| [Preferences](https://support.getharvest.com/hc/en-us/articles/360048179912-Customizing-account-preferences) | FR-008 module/reimbursement and currency context | Existing configuration scope is owned by 013 |
| [Detailed reports](https://support.getharvest.com/hc/en-us/articles/360048687171-Detailed-time-and-detailed-expense-reports) | FR-009/010 filters, downloads and manual billing status | Legacy role wording does not settle the six-profile matrix |
| [Expense object](https://help.getharvest.com/api-v2/expenses-api/expenses/expenses/) | FR-005 identity and distinct state fields | An API description is not proof of browser behavior or authority to expand Horae's compatibility API |
| [Invoice editing](https://support.getharvest.com/hc/en-us/articles/360048181012-Editing-and-deleting-invoices-and-estimates) | FR-010 invoice-to-source behavior | Does not settle the reverse direction: changing/deleting a source expense |
| [Currencies](https://support.getharvest.com/hc/en-us/articles/360055384512-Does-Harvest-support-multiple-currencies) | FR-011 manual conversion | Client asset resolves current display priority; persistence/history still unresolved |
| [Current permissions](https://support.getharvest.com/hc/en-us/articles/44171549176077-Permissions) | FR-014 read/write scopes | Corroborated by delivered editor configuration, not lifecycle enforcement |
| [Invoice attachments](https://support.getharvest.com/hc/en-us/articles/9864825272589-Attaching-files-and-reports-to-invoices) | FR-015 recipient report and draft regeneration | Source correction/deletion and sent artifacts remain unverified |
| [Receipt announcement](https://www.getharvest.com/blog/2010/05/upload-expense-receipts-in-harvest) | Historical 10 MB limit | Published 2010; not proof of current exact byte boundary |

The expense reference describes quantity as integer but examples serialize it
as `1.0`. Neither proves the accepted fractional precision or rounding rule.
Do not choose a lossy representation from that mismatch. Existing exact-money
invariants remain mandatory.

Further research separates two directions that must not be conflated: invoice
line edits do not rewrite the original tracked expense, but the cited invoice
article does not establish what a privileged source-expense edit does to an
existing invoice. FR-010 now covers the documented direction; EXP-E02 remains
open for the reverse direction. No persisted browser test was performed.

The fetched currency page contains an internal conflict: its project-specific
currency section permits an override, while its expense FAQ describes client-only
currency. Subsequent [client inspection](reference-validation.md) resolves the
current form/row/weekly-total priority in favor of project overrides. Historical
currency changes and persistence remain open. Neither incomplete search excerpts
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
were inconsistent and do not establish acceptance or rounding. No expense or
category was saved, edited or deleted; reimbursement and non-owner enforcement
remain untested. Console errors were reported; no cause has been established.

The Categories tab also timed out on click. Navigating directly to its observed
link succeeded: the category list exposes New category and per-category Edit,
Archive and Delete controls, including a mileage rate display. Visibility is not
proof that these mutations, rate validation or restrictions work. No such
mutation was invoked. Current navigation places Categories under Expenses,
unlike the older Manage menu wording in the help article.

## Clarification scan

`speckit-clarify` resolved this feature using the checked-in prerequisite helper.
One product question was presented and answered: the user authorized composing
missing screens from the current Horae design system while preserving Harvest
behavior. Recorded in the spec's clarification, FR-013 and assumptions.

| Category | Status | Next action |
| --- | --- | --- |
| Functional scope | Clear | Native expense workflow included by confirmed web scope |
| Domain and data | Partial | Resolve FR-011 precision, currency and invoice-correction effects from evidence |
| Interaction and UX | Partial | Form opened; missing-handoff policy and report selection specified; numeric interaction and detailed composition remain |
| Non-functional quality | Partial | Select explicit upload/scale/failure limits during planning; final receipt privacy contract required |
| Dependencies | Partial | Complete 015 matrix and assign invoice/report owners |
| Edge cases | Partial | Turn remaining numeric, attachment and correction cases into exact expectations |
| Constraints | Clear | Self-hosted architecture, exactness, no new integration, no application/data changes in this phase |
| Terminology | Clear | Billing, approval, reimbursement and invoice association remain distinct |
| Completion signals | Partial | SC-001–005 defined; unresolved contracts prevent full acceptance coverage |
| Placeholders | Partial | Two clarification markers remain, both reference/dependency work rather than questions for the user to guess |

### Open work

- **EXP-D01 — Resolved product/design**: On 2026-10-01 the user authorized
  composing missing surfaces from current Horae components/tokens, preserving
  Harvest behavior. No new handoff is required. FR-013 requires a planning-time
  composition/control-state mapping and shared-screen regression checks.
- **EXP-E01 — Numeric evidence**: Read-only form validation where possible;
  determine supported precision, range, signs, rate application and currency
  transitions. Current client currency precedence is resolved; unsaved numeric
  experiments are inconclusive. Any test requiring a save needs disposable
  fixtures and appropriate authority; no real-account writes are part of this
  planning turn.
- **EXP-E02 — Billing contract**: Define privileged correction/deletion effects
  on invoice lines, source links and already issued totals, with the billing
  owner. Keep manual billed status separate from a real invoice relationship.
- **EXP-E03 — Permission dependency**: Extend 015 to expense operations and
  receipt downloads. Six read/write grants and profile defaults are now observed
  and recorded with the owner; lifecycle mapping and non-owner enforcement remain
  open. Do not translate legacy Manager wording into an unverified grant.
- **EXP-E04 — Receipt contract**: Establish size, replacement, error, download
  revocation and data-archive behavior. Never leave attachments outside the
  organization/expense access checks.

These gates block final planning/tasks/analysis, not independent research.
The concrete distinguishing checks and evidence required are recorded in
[reference-validation.md](reference-validation.md); they are not implementation
tasks or permission to mutate the reference account.

## Preliminary adversarial self-review

This is a draft self-review, not independent review or `speckit-analyze`.

| Finding | Severity | Disposition |
| --- | --- | --- |
| Financial correction/deletion could diverge from issued invoice totals | High | Open EXP-E02; no invented cascade or silent invoice rewrite |
| New expense grants could bypass approved scoped permissions | High | Open EXP-E03; matrix completion is a prerequisite |
| Undefined fractional arithmetic could create inconsistent totals | High | Open EXP-E01; FR-011 blocks readiness |
| A receipt URL could leak confidential material after revocation | High | FR-003/012 require current authorization; EXP-E04 contract still incomplete |
| A blanket internal receipt rule could either expose source records to clients or omit supported invoice attachments | High | FR-015 separates recipient reports; sent/source-change retention remains open with EXP-E02/04 |
| Client formatting could be mistaken for accepted precision or negative support | High | Inconsistent unsaved results recorded; E-NUM persisted evidence required |
| Empty selection could silently apply a bulk action or regress other tables | Medium | FR-016 specifies report targets and confirmation while preserving Projects; future regression checks required |
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
- Next action: obtain authority for isolated disposable reference records and
  suitable non-owner access for E-AUTH; execute the discriminating checks before
  closing the numeric, attachment and source-billing contracts. No application
  change, migration, reference save or merge occurred in this iteration.
