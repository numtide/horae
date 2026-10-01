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
| Project Detail, `004-project-dashboard` | [fa15ff4](https://github.com/numtide/horae/tree/fa15ff4e45955f7f0c0512d3a0f7f4ea3c124283/specs/004-project-dashboard), [#208](https://github.com/numtide/horae/pull/208) | Confirmed permission model propagated; historical plan and partial implementation remain unaccepted | Resolve imported billing/actions and finish the dependent permission matrix before full planning/analysis |
| Clients, `012-clients-design` | [bc5d063](https://github.com/numtide/horae/tree/bc5d06315738f028665386fb405c54232227cf73/specs/012-clients-design), [#209](https://github.com/numtide/horae/pull/209) | Confirmed permission model propagated; draft spec/research/checklist, no plan/tasks | Settle contacts and lifecycle conflict, then plan against feature 015's completed contract |
| Workspace, `013-workspace-design` | [9a7ec5a](https://github.com/numtide/horae/tree/9a7ec5ac01a1670c009767a6b0d9354a365d61f5/specs/013-workspace-design), [#210](https://github.com/numtide/horae/pull/210) | Draft spec, research and requirements checklist; no plan/tasks | Invitations, backup and deletion contracts |
| Personal Settings, `014-personal-settings-design` | [6aafd4e](https://github.com/numtide/horae/tree/6aafd4eb3d24f74ba659ae5c6766f610f8604fb4/specs/014-personal-settings-design), [#211](https://github.com/numtide/horae/pull/211) | Draft spec, research and requirements checklist; no plan/tasks | Profile ownership and notification delivery |
| Scoped permissions/approvals, `015-scoped-permissions` | [f5cf02d](https://github.com/numtide/horae/tree/f5cf02db992d50638c9d89a3f7f661443cc7a5fa/specs/015-scoped-permissions), [#212](https://github.com/numtide/horae/pull/212) | Planning artifacts and constitution 1.1.0 proposal retained; expense read/write defaults now observed; full lifecycle policy unresolved | Complete reference matrix, review mappings and propagate permission contracts to dependent specs |
| Expenses, `016-expense-parity` | [3df72c9](https://github.com/numtide/horae/tree/3df72c92486f4b430e79542475d83e46daa9d976/specs/016-expense-parity), [#214](https://github.com/numtide/horae/pull/214) | Persisted USD precision/signs, rate repricing, project override, receipt byte limit and removal verified; 11/16 checks; no plan/tasks | Currency history/exponents/bounds, source correction, receipt content/access/retention and operation-level permissions |

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
| Expenses | Pending 016 / #214; official sources, persisted isolated numeric/receipt fixtures and client/configuration evidence | Finish numeric bounds/currency history, receipt retention/access, lifecycle permissions and invoice-correction contracts; separate endpoint acceptance from UI behavior |
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
| D-014 | Evidence and contracts / 016 + billing + 015 | Isolated-fixture mutations authorized; numeric USD precision, project override, rate resave and receipt byte/removal rules verified. Currency history/bounds, source correction/deletion and receipt/permission lifecycles remain open. Six basic read/write defaults are observed, not a full enforcement matrix. Non-owner checks require suitable access; no paid seats or owner changes authorized. |

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
