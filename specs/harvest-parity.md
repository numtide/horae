# Harvest parity register

Checked: 2026-10-01. Stage: web scope confirmed; feature decomposition and clarification.

This register supports [the specification delivery](delivery-goal.md). It does
not certify functional parity or replace feature artifacts. An existing route,
checked task or merged PR is not evidence that every Harvest workflow is covered.

## Evidence baseline

- Horae `master`: `9301112c6a02ae3c92716273534f38889db241d1`. Inventory inspected
  `crates/horae/src/route.rs`, page/server-function file locations, checked-in
  specs, constitution and the pending feature packages below. This is not a
  complete code audit or a fresh browser regression run.
- Design: the current `design/README.md` identifies the `Horae (3).zip` handoff.
  `design/IMPLEMENTATION-GAPS.md` is dated 2026-09-16 and predates subsequent
  project work; use it as a discovery aid, not current acceptance evidence.
- Harvest: [navigation inventory](https://support.getharvest.com/hc/en-us/articles/44165229587469-Navigating-Harvest)
  and [invoicing/estimates index](https://support.getharvest.com/hc/en-us/categories/360004023632-Invoices-estimates)
  consulted on 2026-10-01. These establish candidate product areas, not detailed
  lifecycle contracts. Deep-link the behavior-specific source in each spec.
- Existing authenticated-browser evidence is retained in the pending feature
  research files. It was not rerun during this inventory. Preserve its stated
  limitations; do not relabel documented behavior as interactively verified.

## Existing feature ownership

Pending packages are linked to immutable revisions because their directories may
not exist on `master` yet. Do not copy them into this coordination branch or imply
their implementation has merged. Recheck heads before modifying an owning branch.

| Package | Snapshot / PR | Actual planning state | Next gate |
| --- | --- | --- | --- |
| Project Detail, `004-project-dashboard` | [48a4156](https://github.com/numtide/horae/tree/48a415608f22fac8141a5f7609febcca1bef5d6d/specs/004-project-dashboard), [#208](https://github.com/numtide/horae/pull/208) | Permission model and expense-budget FR-023 propagated; historical plan/implementation remain unaccepted | Resolve imported billing/actions and finish the dependent permission matrix before full planning/analysis |
| Clients, `012-clients-design` | [bc5d063](https://github.com/numtide/horae/tree/bc5d06315738f028665386fb405c54232227cf73/specs/012-clients-design), [#209](https://github.com/numtide/horae/pull/209) | Confirmed permission model propagated; draft spec/research/checklist, no plan/tasks | Settle contacts and lifecycle conflict, then plan against feature 015's completed contract |
| Workspace, `013-workspace-design` | [9a7ec5a](https://github.com/numtide/horae/tree/9a7ec5ac01a1670c009767a6b0d9354a365d61f5/specs/013-workspace-design), [#210](https://github.com/numtide/horae/pull/210) | Draft spec, research and requirements checklist; no plan/tasks | Invitations, backup and deletion contracts |
| Personal Settings, `014-personal-settings-design` | [6aafd4e](https://github.com/numtide/horae/tree/6aafd4eb3d24f74ba659ae5c6766f610f8604fb4/specs/014-personal-settings-design), [#211](https://github.com/numtide/horae/pull/211) | Draft spec, research and requirements checklist; no plan/tasks | Profile ownership and notification delivery |
| Scoped permissions/approvals, `015-scoped-permissions` | [f5cf02d](https://github.com/numtide/horae/tree/f5cf02db992d50638c9d89a3f7f661443cc7a5fa/specs/015-scoped-permissions), [#212](https://github.com/numtide/horae/pull/212) | Planning artifacts and constitution 1.1.0 proposal retained; expense read/write defaults now observed; full lifecycle policy unresolved | Complete reference matrix, review mappings and propagate permission contracts to dependent specs |
| Expenses, `016-expense-parity` | [49f8038](https://github.com/numtide/horae/tree/49f8038364108b51fb3eb5ef815f659fa35f0221/specs/016-expense-parity), [#214](https://github.com/numtide/horae/pull/214) | Combined locks, 101-row filter containment, fee/hour budgets, USD/GBP list/report reconciliation and UI exponent normalization verified; shared contract propagated to editor/dashboard; 11/16 checks, no final plan/tasks | Receipt-download/budget-control audit published; resolve the remaining detailed-report target-set evidence. Larger-volume report scope remains uncertain; numeric conflict, export email, non-owner and recipient access remain distinct gates |

All six feature PRs were drafts at the latest inventory snapshot. Status is not a promise about
later GitHub state. No merge is part of this specification delivery.

## Coverage map

The web application scope is approved by D-001 below. The following is an initial
discovery map, not an exhaustive implementation-ready backlog. Split features by
independently testable workflow; do not allocate all feature numbers up front.
Reuse existing specifications and record supersession rather than duplicating
their requirements. Missing mockups do not exclude approved web workflows.

| Area | Existing owner / evidence | Required investigation before a ready backlog |
| --- | --- | --- |
| Sign-in and onboarding | 001; design 01/02/03; existing OIDC/CLI bootstrap | Separate self-hosted admission and invitations from Harvest ID, hosted signup and subscriptions |
| Time tracking | 001/003; design 04; Timesheet routes | Day/week/calendar parity, copy/reuse flows, validation, timers, historical locks and date boundaries |
| Time off (Beta) | Current authenticated Harvest navigation and `/pto/activation`, observed 2026-10-01; no owner yet | Native web surface: activation overview advertises allowances, requests, balances and team calendar; setup names holiday calendars, people assignment and vacation policy. Investigate without activating or assuming those flows have been tested. Not Forecast. |
| Projects and task catalog | 009/010/011 and pending dashboard; design 05/11/13 | Reconcile delivered list/editor behavior; remaining actions, reporting and global task lifecycle |
| Clients and contacts | Pending 012; design 12 list/detail | Multi-contact billing identity, archive/reactivation, bulk actions and currency-safe totals |
| People, capacity and rates | 001 and pending 013/014/015 | Directory versus admin UI, assignments, contractors/capacity, rate history and effective dates |
| Permissions and approvals | Pending 015; design 06/08/09 | All six profiles, custom grants, scoped approval/withdrawal and every entry point; extend matrix for newly approved domains |
| Reports | 001 and `004-invoice-timesheet-exports`; design 07 | Time/project/team and financial reports, saved/shared/scheduled behavior, permissions and exact export reconciliation |
| Invoice lifecycle and payments | 001/011 and existing invoice modules; dashboard consumers | Draft/send/view, numbering/settings, dates, reminders, recurrence, partial payments/write-offs and project attribution |
| Expenses | Pending 016 / #214; persisted capture/category/receipt/draft/manual-billing and budget/currency reconciliation evidence | Finish independent evidence audit and shared contract review; keep larger-volume scope uncertainty, export delivery authority and non-owner/recipient access distinct |
| Estimates | No dedicated estimate spec or route found | Creation, client delivery/response and downstream project/invoice relationships |
| Retainers | Historical 001 project-kind mention; no dedicated ledger spec found | Distinguish advance-payment balance/draws from fixed or recurring project fees |
| Personal settings | Pending 014; design 08 | Profile, timezone, rates, assignments, notifications and truthful security/integration destinations |
| Workspace administration | Pending 013; design 09 | People/invitations, preferences, export/backup guarantees, audit and deletion semantics |
| Import and migration | `004-harvest-importer`, 005–008; design 10 | Preserve delivered job/CLI/account-switch behavior and resolve unknown billing semantics; new connector/entity-import expansion is not authorized by web parity |
| Existing integrations and API | 001/002, compatibility API and plugin infrastructure | Regression/authorization coverage only; do not remove working features or add new connectors/write endpoints |
| Native applications, Forecast and new integrations | Excluded by D-001 | No implementation/specification work for these products in this phase, including new payment gateways, calendar/accounting connectors and third-party extensions |
| Other plan-dependent web capabilities | Harvest indexes; investigate within confirmed web scope | Classify AI, e-invoicing and commercial account features against the self-hosted architecture and no-new-integration boundary; do not silently invent external services |

Harvest's navigation documents time, expenses, people, clients, projects, tasks,
invoices, estimates, approvals, reporting and account/profile configuration.
Its billing index separately exposes retainers, recurrence and online payments.
The native web workflows are required even without a dedicated Horae mockup;
online-payment connectors are excluded by the user's no-integration decision.
Membership in an index does not justify a new external service.
[Navigation](https://support.getharvest.com/hc/en-us/articles/44165229587469-Navigating-Harvest),
[billing index](https://support.getharvest.com/hc/en-us/categories/360004023632-Invoices-estimates).

## Decisions and evidence gaps

Statuses distinguish an unanswered product decision from unfinished research.
Only the former should be presented as a choice to the user.

| ID | Kind / owner | Current state and next action |
| --- | --- | --- |
| D-001 | Confirmed / delivery | On 2026-10-01 the user confirmed the complete Harvest web application and explicitly excluded native apps, Forecast and integrations for now. Expenses, estimates, retainers and invoicing are included. No new connectors are planned; preserve existing working integrations without expanding them. |
| D-002 | Confirmed / 015 | Six built-in profiles, custom profiles, per-person changes and scoped approvals were requested on 2026-09-30. Complete exact contracts; do not reopen scope to simplify them. |
| D-003 | Evidence and source conflict / 012 | Contact cardinality and archive policy remain open. Existing Harvest research documents separate contacts and archive only after all projects are archived; the handoff cascades. Reconcile under the confirmed parity mandate and explicitly resolve the visual/behavioral conflict. |
| D-004 | Product policy / 013 | FR-006: named-recipient invitation versus reusable workspace join link; admission, expiry and role rules depend on it. |
| D-005 | Self-hosted design contract / 013 | FR-012/014/015: real deployment URL, portable data archive versus recoverable backup, destination, schedule and retention. Proposed values are not user decisions. |
| D-006 | Destructive scope / 013 | FR-019: in-browser deletion versus operator reset; affected data/artifacts, later bootstrap and retention promises need an explicit contract. No real deletion is authorized by this register. |
| D-007 | Identity ownership / 014 | FR-003: locally editable profile versus provider-owned values. Preserve verified identity binding either way. |
| D-008 | Product and delivery contract / 014 | FR-010: operational notifications and actual delivery conditions; don't copy unsupported home, security or promotional controls from placeholders. |
| D-009 | Cross-spec conflict / dashboard + importer | Dashboard US3/FR-011 assumes known external billed state; importer FR-016 deliberately leaves entries locally open without persisting it. Choose an explicit unknown-state/import policy before claiming uninvoiced certainty. |
| D-010 | Reference contract / dashboard | PD-001 still asks whether to omit Pin/Duplicate/Delete/Link/Unlink. Reconcile with the new parity request; investigate their semantics and destructive safeguards, not just whether a button is visible. |
| D-011 | Reference access / 015 | Current evidence account has only its immutable owner; invite path requests another paid seat. Custom-profile persistence and several approval edge cases remain unverified. Use available documentation first; request suitable test access if needed, never purchase or bypass restrictions. |
| D-012 | Governance and contract dependency / 015 + consumers | PR #212 proposes constitution 1.1.0, not merged. PRs #208/#209 now use the confirmed six-profile/custom target and distinguish legacy evidence from acceptance. Full operation-matrix integration, persistence and migration review remain required; the broad three-role contradiction is resolved in the pending specs. |
| D-013 | Confirmed / 016 and other absent mockups | On 2026-10-01 the user authorized composing screens without a dedicated mockup from current Horae components and tokens, preserving Harvest behavior. A new handoff is not required. Plans must record composition/control states and protect shared defaults through cross-screen regression. This does not authorize implementation in the specification phase. |
| D-014 | Evidence and contracts / 016 + billing + 015 | Isolated fixtures verify USD/JPY/BHD precision, historical current-currency relabeling without FX, tiny-rate rounding, category archive, receipt boundaries and draft source independence/release. Bounds/exports, issued reports/attribution and permissions remain open. Invoice Preview requires paid access or Stripe on the current account; no upgrade/integration authorized. Non-owner checks need suitable access; six defaults do not prove enforcement. |
| D-015 | Reference discrepancy / 016 + reporting | Empty-selection marking of 101 fixture records respects date/project filters and leaves an out-of-filter source unchanged. Detailed report renders all 101 on one page and ignores page/per_page probes; ordinary expense list separately paginates at 50. Larger-volume report scope remains uncertain; shared all-pages code does not prove an exposed report control. Projects' disabled-empty Actions stay unchanged. |
| D-016 | Delivery authority / 016 exports | CSV/XLSX/PDF controls and fixture-filtered generation URLs observed, but generation can return an emailed result. No generation submitted under the no-messages authorization. This is not proof of a paid-plan export gate or a content-validation pass. |
| D-017 | Reference conflict / 016 money | Bounded tests establish quantity/rate limits, but ordinary-amount boundary messages disagree with accepted values and a large accepted response loses cents. Preserve constitutional exactness; agree an explicit bound/overflow contract rather than copying that loss or treating an error message as a verified bound. |
| D-018 | Shared dependency / 016 + project editor/dashboard | Live fee inclusion on/off, both billability states, monthly boundaries, total-hours exclusion and owner USD list/detail/report reconciliation verified. Shared contract in #214 feeds expense FR-019, editor FR-026 and dashboard FR-023 (#208). USD/GBP expense totals remain separate; mixed-client subtotal is N/A. Nonzero unlike time-cost/expense-cost testing requires time/rate fixtures outside current authorization; complete permission and budget-variant evidence remain separate. |

## Proposed dependency order

This orders planning, not implementation authorization:

1. Complete the surface inventory against confirmed D-001: native web workflows
   in scope; native applications, Forecast and new integrations excluded. Review
   ambiguous commercial/web capabilities without reopening the answered boundary.
1. Complete shared permission/approval evidence and migration governance in 015;
   define billing/date/identity contract ownership alongside it.
1. Reconcile and finish the existing dashboard, Clients, Workspace and Settings
   packages. Their independent research can proceed while specific decisions wait.
1. Specify remaining approved domains using shared contracts. Invoice/payment,
   expense, estimate and retainer boundaries must avoid competing money models.
1. Complete reports and existing integration/import regression coverage against those domain contracts;
   review all cross-screen journeys and close analysis findings.

## Iteration log

### 2026-10-01 — Independent evidence audit

- Expense checkpoint `49f8038` adds `independent-work-audit.md` with named
  investigations, evidence strength, unresolved gates and clarification coverage.
  Budget variants expose no expense-inclusion control, corroborating official
  guidance without claiming per-variant persisted consumption tests.
- Internal receipt PDF is served as an attachment; the expense editor has no
  receipt PDF viewer. Generated-report rendering remains separate and cannot be
  certified without its export/recipient artifact path. No new mutations were
  needed; cancelled both editor inspections without saving.
- Checklist remains 11/16, no new questions or marker changes. Scoped formatting
  and diff checks pass; PRs remain open drafts, no merge or implementation.
- Next independent action: investigate the remaining detailed-report target-set
  gap with a discriminating reference, not another equivalent 101-row test.
  Larger-volume behavior remains unproven. Money-bound conflict, retention,
  export email and reference-access/fixture-authority gates stay explicit; the
  active goal is not declared complete by this audit.

### 2026-10-01 — Expense budgets and cross-currency reconciliation

- Expense checkpoint `7f193af` and dashboard checkpoint `48a4156` publish the
  shared budget contract and its owning editor/dashboard requirements. No
  application implementation, migration or merge is included.
- Combined manual/archive locks: clearing billing preserves the archive lock;
  restoring the project separately removes it. A bounded 101-record report
  action respects filters; all rows appear on one report page. Ordinary-list
  pagination is not report pagination. Deleted all 101 exact test identities.
- Fee budgets include both billable and non-billable expenses when enabled;
  disabling inclusion removes consumption, not costs. Monthly consumption uses
  expense work dates. A 100-hour budget with only expenses retains 100 hours.
  List/detail/report reconcile the same USD 1.50 expense contribution.
- A disposable GBP project verifies separate USD/GBP list/report totals,
  mixed-client N/A, denominated project subtotals and active-only filtering.
  UI 1e3 normalizes and saves 13 in both amount and quantity fields, unlike the
  endpoint's 1000. Removed that temporary expense, archived its project and
  restored the original active/no-budget USD fixture and its unchanged receipt.
- Cross-contract self-review corrected the editor's blanket organization-cost
  currency statement. Exactness, expense authority, time-cost currency, budget
  inclusion and invoice eligibility remain separate. New requirements are not
  covered by historical checked implementation tasks.
- Clarify asked/answered zero new product questions; checklist 11/16 unchanged.
  Scoped Nix formatting and diff checks passed. No mail, payment, sent invoice,
  pre-existing business-record change, account-wide setting or new integration.
- Next: audit remaining receipt-rendering and budget-variant evidence, then
  classify every residual gate before the independent-work completion audit.
  Larger-volume report scope is an unresolved reference uncertainty, not a
  passed multi-page test. Export email authority, alternate account locale,
  nonzero time/rate fixtures, non-owner and recipient/sent-state access remain
  separate from work that can still proceed. Goal remains active.

### 2026-10-01 — Archive locks and draft line lifecycle

- Expense checkpoint `7dc6064` integrates category endpoint rate changes while
  archived, change-away/return rejection and server refusal of stale Delete.
  Project archive preserves expense/receipt and permits owner amount/billability
  correction, unlike the invoice lock's disabled billability. Corrected FR-006;
  default reports include archived expenses, Active projects only excludes them.
- Quantity/rate boundaries now have bounded evidence. Ordinary amount error
  limits disagree with acceptance and one accepted response loses cents; exact
  money remains mandatory, not a request to copy that defect. Header-only PDF
  acceptance versus signature-only PNG rejection distinguishes upload recognition
  from structural validity/renderability.
- A third fixture-only draft verifies invoice price correction leaves source
  values unchanged; changing a draft-linked source's project USD→GBP relabels
  the expense without changing the invoice's EUR amount. Saved removal of its
  sole line releases source billing and keeps the receipt, leaving an empty
  unsent draft. Project/expense baseline restored; exact fixtures tracked locally.
- Cross-contract self-review found missing expense inclusion in project budgets;
  FR-019 now records the documented Total project fees inclusion option. This
  must feed editor/dashboard/report planning; it is not a passed live budget test.
- Clarify prerequisites/coverage/checklist and targeted Nix formatting passed;
  11/16 unchanged, zero new product questions. No app implementation, migration,
  existing-business-record change, message, payment, integration or merge.
- Next independent checks: multi-page report targeting, combined locks,
  mixed-currency reconciliation and budget inclusion, then reconcile remaining
  cross-contract findings. Export generation remains unsubmitted pending email
  authority; non-owner enforcement and recipient artifacts need suitable access.
  Goal `termina lo independiente` remains active, not complete at this checkpoint.

### 2026-10-01 — Independent input, receipt and report-action checks

- Expense checkpoint `8452653` records current-locale parsing (`1,25` saves as
  125), larger accepted amounts without claiming a maximum, content-based
  receipt rejection and valid PNG/GIF/JPEG acceptance with MIME normalization.
- Confirmed manual expense mark/clear and refusal of non-billable marking using
  only the fixture project. No invoice was created or sent. Category mode
  changes retain historical amounts until expense resave; ordinary resave sets
  quantity1, later unit resave applies the current rate. Restored all fixtures.
- Self-review found the page-only guarantee was stronger than available live
  evidence. FR-016 now carries the discrepancy rather than an invented rule.
  Export controls/columns are inspected; bytes remain unverified because of the
  possible email delivery side effect. No export job was launched.
- Clarify workflow/prerequisites/checklist and targeted Nix formatting passed;
  11/16 remains, no readiness claim. No new product questions, app changes,
  migrations, messages, payments, deletions or merges. PRs/worktrees reused.
- Goal `termina lo independiente` remains active. Next: archived rate editing,
  change-away/return, project archive, bounded numeric/malformed-file checks,
  invoice-line/currency effects and multi-page selection, then cross-contract
  review. Keep export authority, non-owner enforcement and recipient artifacts
  as distinct remaining gates; do not confuse this checkpoint with completion.

### 2026-10-01 — Independent currency, category and draft-release tests

- Reused #214/#213 and their worktrees. Feature checkpoint `fc352ae` adds
  currency-evidence and category-evidence, extends billing evidence, narrows
  FR-011/017 and updates acceptance cases. No feature implementation or merge.
- New isolated expense verified two-decimal JPY/BHD endpoint amounts versus JPY
  zero-decimal list display. Changing project currency and client inheritance
  relabeled old unbilled expenses without conversion; original client EUR and
  project USD override were restored. Plan must reconcile exact fractional
  representation with the constitution, not truncate from display formatting.
- Saved a second test draft as EUR1.24 from USD1.24 source, then permanently
  deleted only that draft through exact-ID/number confirmation. Source and PDF
  survived with invoice/billed/lock flags cleared. Unrelated locks were absent;
  no claim about their release. Original business invoices untouched.
- Invoice report preparation retained its include flag, but saved draft exposed
  no attachment link. Preview explicitly requires paid plan or Stripe; did not
  upgrade, connect, send or pay. Report generation/content remain unverified;
  missing controls are not proof the feature is removed or caused by that gate.
- Category0.0004 rejected,0.0005→0.001; restored0.5. Archive preserved existing
  entry correction and receipt; new capture returned422. Restored category to
  active and verified read-back. Delete-in-use is UI-disabled; server deletion
  enforcement was not claimed.
- Clarify prerequisites, absent hooks, coverage and self-review completed for
  this evidence iteration; zero new product questions. Checklist11/16→11/16,
  no marker changes; Nix format/whitespace checks passed. No final plan/tasks/
  analyze or complete parity claim. Private ledger retains all cleanup targets.
- Next independent checks: locale/bounds, export reconciliation and receipt
  content validation. Non-owner and recipient/issued-artifact validation need
  suitable reference access; no new paid seats or integrations are authorized.

### 2026-10-01 — Draft invoice and source independence

- Expense checkpoint `d393d53` adds owner-only browser evidence: saved one
  fixture-only draft invoice for USD 1.00, corrected its source expense to USD
  2.00 and changed the note. Reloaded invoice retained its original quantity,
  description and USD 1.00 total. Permanently deleting the test source expense
  also left that draft unchanged; no business expense was touched.
- Invoice review defaulted to client EUR despite the project's USD override.
  USD was explicitly selected before saving; cross-currency persistence is not
  claimed. The source editor warns about invoice independence and disables date,
  project, category and billability. Non-owner authorization is still untested.
- FR-006/011 and acceptance coverage updated; Nix formatting and whitespace
  checks passed. Checklist still 11/16 with no marker changes; no final analysis
  or readiness claim. No invoice sent/marked sent, payment, migration or merge.
- Remaining fixture client/project/category and unsent draft are identified in
  the private scratch ledger. The original test expense and synthetic receipt
  were deleted during their lifecycle checks and must not be reused by ID.
- Next: currency/history and receipt/report lifecycle using only isolated
  fixtures; resolve source attribution and safe sent-state evidence with billing.
  E-AUTH continues to need suitable non-owner reference access, without seat
  purchase or owner changes. Do not repeat the completed numeric/boundary tests.

### 2026-10-01 — Persisted expense and receipt fixtures

- Reused the authorized client/project and one expense/category in the same MCP
  connection. Hidden visibility did not prevent focused keyboard/guarded DOM
  saves; no further browser-restoration request is needed.
- Expense checkpoint `3df72c9` in #214 records project USD over client EUR,
  ordinary amount and quantity rounding/signs, three-decimal rates, rounding
  quantity before multiplication, and repricing only on resave. Zero is accepted
  by the endpoint but cleared/blocked by UI validation; evidence levels stay
  separate. Upper bounds, currency exponents/history and locale remain open.
- Synthetic PDFs establish a strict receipt limit below 10,485,760 bytes.
  Oversized/text replacements preserve previous receipt and expense fields;
  cancelled removal retains the receipt; saved removal makes its route return
  404 while keeping the expense. Removed only the reproducible synthetic PDF.
- Ran clarify evidence integration, self-review, checklist revalidation (11/16,
  unchanged), Nix formatting and whitespace checks. Final plan/tasks/analyze
  remain gated; no implementation or completed parity is claimed.
- Existing business records, user tabs, owner and paid seats remain untouched.
  No invoice was created or sent, no payment, migration or merge. Test
  client/project/category/expense remain for follow-up, with exact cleanup IDs in
  the main checkout's private scratch ledger.
- Next: inspect fixture-only draft invoicing and source-edit effects, then
  currency history and remaining receipt checks. E-AUTH still requires non-owner
  reference access; do not invent permissions from owner-only observations.

### 2026-10-01 — Expense rules and discriminating reference checks

- Reused PRs #214/#212/#213 and their isolated branches; all three were checked
  open and draft. Expense commit `9c87682`; permission evidence commit `f5cf02d`.
- Ran the clarify prerequisite/coverage workflow. Opened the actual expense form
  without saving. Hidden duplicate buttons explain the initial failed selector,
  but later interaction failures remain. Unsaved numeric experiments were
  inconsistent; no server precision/range/rounding rule is certified from them.
- Inspected the delivered expense asset and current permission configuration.
  Recorded project-over-client currency priority and six read/write scope
  defaults with evidence levels, without treating configuration as enforcement.
- Added the distinct invoice-recipient expense-report contract, draft report
  regeneration and billable-only manual marking. Expense-report no-selection
  actions follow the documented current-page target and confirmation; Projects'
  separately confirmed disabled-empty behavior and shared defaults stay intact.
- Recorded eight discriminating reference checks for precision/rate, currency,
  invoice source correction, receipts, permissions, archive and reports. The
  historical 10 MB announcement does not establish today's exact byte boundary.
- Revalidated the expense checklist: 11/16, unchanged. High financial/permission/
  receipt findings remain open; no final analyze or implementation readiness is
  claimed. Nix formatting and whitespace checks passed on the feature documents.
- No reference records were saved, no messages sent, no migrations or application
  changes made and no merges performed. Discarded the unsaved form and ended the
  MCP client connection, preserving the user's tabs.
- Next: obtain explicit authority for disposable reference fixtures (no existing
  business records, delivery, payments or account changes), then execute E-NUM,
  E-RATE, E-CUR, E-BILL and E-REC. E-AUTH additionally needs editable non-owner
  access; do not buy seats or change the owner. Continue evidence work rather
  than inventing the missing rules or marking clarification complete.

### 2026-10-01 — Missing-surface design authorization

- D-013 is confirmed: compose missing mockups from Horae's current components
  and tokens while preserving Harvest behavior. Updated the delivery contract
  and feature 016's clarification, FR-013 and assumptions; no UI implementation
  or global design-system change is authorized in this phase.
- Executed `speckit-clarify` path resolution, absent-hook checks, answer
  integration and checklist revalidation. One product question answered;
  checklist remains 11/16 with no marker changes. Remaining gaps are numeric,
  currency, receipt, permission and billing contracts, not missing-handoff policy.
- Extended expense research with current invoice-editing and currency sources.
  Invoice-line edits and source-expense edits are distinct directions; the latter
  remains unverified. Manual currency conversion is documented, but the fetched
  currency page contradicts itself about project overrides versus client-only
  expense currency. Recorded this conflict instead of choosing an unverified rule.
- No browser mutations, application changes, migrations or merges. Next:
  resolve the expense currency/precision and privileged source-correction
  evidence, then finish the shared billing and permission contracts before plan.

### 2026-10-01 — Expense specification and live inventory

- Rechecked remote branches/PRs and existing worktrees; retained their owners.
  Created `feat/expense-parity` in `.worktrees/expense-parity` from the unchanged
  master baseline; feature number 016 was free locally and remotely.
- Executed the installed specify workflow with template resolution, helper
  dry-run, feature pointer, quality checklist and absent-hook checks. Executed
  clarify path resolution and coverage scan; D-013 was asked, not answered.
- Published draft PR #214, commit `c5bf7d5`: expense stories/requirements,
  seven official reference links, browser limitations and preliminary adversarial
  self-review. Checklist 11/16; high findings remain open. No full plan/tasks or
  `speckit-analyze` readiness is claimed.
- Windows Chrome/MCP opened a dedicated reference tab without disturbing the
  existing tab. Expenses empty state and Categories list were observed; clicks
  timed out, so form validation and mutations are not verified. A direct visit
  to the Categories link succeeded. Local snapshots are under the main worktree's
  `.scratch/playwright-windows/` and are not committed.
- Found Time off (Beta) in live navigation. Its read-only activation overview
  is recorded in `time-off-inventory-20261001.md`; no module was activated and no
  policy or account record was changed. Added the surface to the coverage map
  rather than silently omitting it or conflating it with Forecast.
- Propagated confirmed web scope into feature 015 through clarify: PR #212
  `b569c75` expands target matrix coverage and receipt-denial acceptance while
  retaining implementation boundaries. Checklist remains 12/16; grants are not
  inferred and no runtime authorization changes were made.
- Nix format/fail-on-change and Git whitespace checks passed for the changed
  feature documents. No application code, migrations, real-data mutations or
  merges occurred. Existing operational integrations are unchanged.
- Next: record D-013 when answered, continue expense numeric/billing evidence,
  assign invoice/report contracts and investigate Time off using read-only
  evidence. Other feature decisions remain open; do not repeat finished scope
  research or call the backlog implementation-ready.

### 2026-10-01 — Inventory bootstrap

- Created isolated `docs/harvest-parity-specs` worktree from the baseline above.
- Read Spec Kit specify/clarify/plan/tasks/analyze instructions, constitution,
  existing feature packages and design inventory. Resolved the dashboard's
  clarification context using the checked-in prerequisite helper.
- Recorded five pending feature owners and the difference between artifact
  existence, implementation progress and acceptance.
- Consulted the current official Harvest navigation and billing indexes.
- Asked D-001; no answer recorded. No feature is newly marked ready.
- Planning/tasks generation, full per-feature analysis and adversarial acceptance
  have not been completed by this bootstrap. No application code, database or
  existing feature branch was modified.
- Next action: record D-001, expand the approved inventory, then reconcile 015's
  evidence/governance and the existing feature clarifications. Use this register
  as a checkpoint, not as a substitute for executing the feature workflows.

### 2026-10-01 — Permission planning research

- Continued the already confirmed feature 015 scope without assuming a D-001
  answer. Reran Spec Kit's plan setup helper and continued Phase 0 research;
  full planning/analysis readiness is not claimed.
- Published `dcf21ef` on PR #212, documentation only: added the missing migration
  contract, defined reviewable access differences and stale-preview/activation
  safeguards, and documented historical assignment, job and approval gaps.
- Compared current guards with target profiles: old Manager is not an equivalent
  replacement for Project Manager or Executive Manager. Mappings remain subject
  to explicit review, not automatic renaming.
- Investigated managed-rate scope through official permission and API references.
  Their people/project and old/new-profile discrepancies remain unresolved.
  Added a discriminating test protocol instead of choosing an unverified rule.
- Nix formatting with fail-on-change and Git whitespace checks passed for the six
  changed Markdown files. No application tests, browser mutation, data migration
  or runtime policy change took place. T006/T007 remain open.
- Next: obtain suitable new-model non-owner reference access to resolve the
  recorded rate/profile/approval evidence gaps, or continue independent research
  in the already requested screens. At this checkpoint D-001 was unanswered;
  no new product area had been treated as approved.

### 2026-10-01 — Authorization governance reconciliation

- Followed `speckit-constitution` using the already confirmed profile/custom
  permission scope. PR #212 commit `b3ee8da` proposes constitution 1.1.0 and updates
  feature 015's specification, plan and research; no new product choice inferred.
- Compared the five core principles byte-for-byte with the previous revision:
  unchanged. Checked feature templates, runtime guidance, version/date consistency
  and absence of unresolved constitution placeholders. No extension hooks or
  command-template directory exists in the owning worktree.
- Nix format validation passed for the three feature documents; constitution
  whitespace and content checks passed separately because `.specify/` is excluded
  from the repository formatter. No runtime or database changes were made.
- T008 remains open for dependent-spec reconciliation and policy persistence;
  the amendment alone does not approve mappings, deployment or full-feature
  acceptance. Next independent work is propagating the confirmed capability
  boundary to pending consumer specs without inventing the unresolved matrix.
- At this checkpoint D-001 was unanswered; the governance update did not
  authorize new product-area specifications.

### 2026-10-01 — Consumer clarification reconciliation

- Ran `speckit-clarify` path resolution for dashboard and Clients, read their
  specs/constitution/checklists and propagated the already confirmed permission
  answer. No new user answer was inferred and no new scope question was settled.
- PR #208 `fa15ff4`: updated dashboard permissions/acceptance/dependencies and
  annotated legacy plan/research boundaries. Checklist 13/16 → 11/16: corrected
  two optimistic markers for ambiguity and complete scenarios; existing action,
  imported-billing and permission-contract gaps prevent those checks passing.
- PR #209 `bc5d063`: updated client-management personas, capability boundaries,
  six-profile/custom acceptance and the feature 015 dependency. Checklist remains
  11/16. Contact/lifecycle choices remain unanswered.
- Nix formatting and Git whitespace checks passed; only documentation changed.
  Full clarification/planning/analysis is still incomplete for both features.

### Historical clarification checkpoint — before D-001 answer

The independent propagation of confirmed decisions is complete for the five
pending surface/permission packages. The remaining readiness gates cannot be
closed by another status update or by selecting an unanswered option:

- Delivery: D-001 must bound the complete feature inventory.
- Dashboard/Clients: action/import-billing and contact/lifecycle choices remain.
- Workspace/Settings: invitation, backup/deletion, identity and delivery policy
  choices remain.
- Permissions: current-reference conflicts need suitable non-owner test access;
  documented research has not resolved them. Migration mappings remain unapproved.

Resume by recording the user's D-001 answer, then handle the next consequential
feature clarification one at a time. Do not create speculative implementation
plans, claim full analysis, purchase reference access or silently reduce parity
to bypass these gates. All published work remains reviewable; no PR was merged.

## Clarifications

### Session 2026-10-01

- Q: Complete Harvest web application, with native applications, Forecast and integrations treated separately? → A: The user confirmed the complete web scope and explicitly does not want native applications, Forecast or integrations for now. Integrations are excluded from this phase, not pending selection.
- Q: May screens without a dedicated mockup be composed from existing Horae components and tokens while preserving Harvest behavior? → A: Yes, authorized on 2026-10-01. D-013 is closed; no new handoff is required for those surfaces.

The completed answer supersedes the earlier D-001 waiting checkpoints. Existing
product-specific decisions remain open; this answer does not choose contact
lifecycle, invitation admission, backup/deletion policy or unverified permissions.
The delivery checklist records scope confirmation only, not feature readiness.
Next: decompose the newly confirmed native web domains into feature specifications
and continue feature clarifications without reopening D-001.
