# Harvest parity register

Checked: 2026-10-01. Stage: initial inventory and scope clarification.

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
| Project Detail, `004-project-dashboard` | [cc04bd6](https://github.com/numtide/horae/tree/cc04bd695ec3e7c05ce41f66c0d6db7851cfb02b/specs/004-project-dashboard), [#208](https://github.com/numtide/horae/pull/208) | Spec/plan/tasks and supporting artifacts exist; partial implementation, whole-feature acceptance open | Reconcile permissions, imported billing and remaining actions before final analysis |
| Clients, `012-clients-design` | [fb5c4ea](https://github.com/numtide/horae/tree/fb5c4ea01095b6fe4b7f3f65a62a90f99a1d57cb/specs/012-clients-design), [#209](https://github.com/numtide/horae/pull/209) | Draft spec, research and requirements checklist; no plan/tasks | Settle contacts and lifecycle conflict, then plan |
| Workspace, `013-workspace-design` | [9a7ec5a](https://github.com/numtide/horae/tree/9a7ec5ac01a1670c009767a6b0d9354a365d61f5/specs/013-workspace-design), [#210](https://github.com/numtide/horae/pull/210) | Draft spec, research and requirements checklist; no plan/tasks | Invitations, backup and deletion contracts |
| Personal Settings, `014-personal-settings-design` | [6aafd4e](https://github.com/numtide/horae/tree/6aafd4eb3d24f74ba659ae5c6766f610f8604fb4/specs/014-personal-settings-design), [#211](https://github.com/numtide/horae/pull/211) | Draft spec, research and requirements checklist; no plan/tasks | Profile ownership and notification delivery |
| Scoped permissions/approvals, `015-scoped-permissions` | [d3a4ff3](https://github.com/numtide/horae/tree/d3a4ff3d38669f1ddca50ca6d5a23d5245188283/specs/015-scoped-permissions), [#212](https://github.com/numtide/horae/pull/212) | Spec/plan/tasks and supporting artifacts exist; foundational code, unresolved parity/cutover gates | Complete matrix and evidence, migration mapping and constitution amendment |

All five PRs were drafts at the inventory snapshot. Status is not a promise about
later GitHub state. No merge is part of this specification delivery.

## Candidate coverage map

The following is an initial discovery map, not an exhaustive or approved backlog.
Split future features by independently testable workflow after D-001; do not
allocate all feature numbers up front. Reuse existing specifications and record
supersession rather than duplicating their requirements.

| Area | Existing owner / evidence | Required investigation before a ready backlog |
| --- | --- | --- |
| Sign-in and onboarding | 001; design 01/02/03; existing OIDC/CLI bootstrap | Separate self-hosted admission and invitations from Harvest ID, hosted signup and subscriptions |
| Time tracking | 001/003; design 04; Timesheet routes | Day/week/calendar parity, copy/reuse flows, validation, timers, historical locks and date boundaries |
| Projects and task catalog | 009/010/011 and pending dashboard; design 05/11/13 | Reconcile delivered list/editor behavior; remaining actions, reporting and global task lifecycle |
| Clients and contacts | Pending 012; design 12 list/detail | Multi-contact billing identity, archive/reactivation, bulk actions and currency-safe totals |
| People, capacity and rates | 001 and pending 013/014/015 | Directory versus admin UI, assignments, contractors/capacity, rate history and effective dates |
| Permissions and approvals | Pending 015; design 06/08/09 | All six profiles, custom grants, scoped approval/withdrawal and every entry point; extend matrix for newly approved domains |
| Reports | 001 and `004-invoice-timesheet-exports`; design 07 | Time/project/team and financial reports, saved/shared/scheduled behavior, permissions and exact export reconciliation |
| Invoice lifecycle and payments | 001/011 and existing invoice modules; dashboard consumers | Draft/send/view, numbering/settings, dates, reminders, recurrence, partial payments/write-offs and project attribution |
| Expenses | No dedicated expense spec or route found in this inventory | Categories, receipts, reimbursable/billable amounts, approval, reporting and invoice integration |
| Estimates | No dedicated estimate spec or route found | Creation, client delivery/response and downstream project/invoice relationships |
| Retainers | Historical 001 project-kind mention; no dedicated ledger spec found | Distinguish advance-payment balance/draws from fixed or recurring project fees |
| Personal settings | Pending 014; design 08 | Profile, timezone, rates, assignments, notifications and truthful security/integration destinations |
| Workspace administration | Pending 013; design 09 | People/invitations, preferences, export/backup guarantees, audit and deletion semantics |
| Import and migration | `004-harvest-importer`, 005–008; design 10 | Preserve delivered job/CLI/account-switch behavior; inventory additional entity import and unknown billing history |
| Integrations and API | 001/002, compatibility API and plugin infrastructure | Name required connectors and read/write contracts; don't equate plugins with supported Harvest integrations |
| Separate/plan-dependent capabilities | Harvest indexes; no scope decision yet | Explicit disposition for native apps/extensions, Forecast, AI, e-invoicing, payment gateways and commercial account features |

Harvest's navigation documents time, expenses, people, clients, projects, tasks,
invoices, estimates, approvals, reporting and account/profile configuration.
Its billing index separately exposes retainers, recurrence and online payments.
Those are candidates even without a dedicated Horae mockup; membership in an
index does not settle integration scope or justify a new external service.
[Navigation](https://support.getharvest.com/hc/en-us/articles/44165229587469-Navigating-Harvest),
[billing index](https://support.getharvest.com/hc/en-us/categories/360004023632-Invoices-estimates).

## Decisions and evidence gaps

Statuses distinguish an unanswered product decision from unfinished research.
Only the former should be presented as a choice to the user.

| ID | Kind / owner | Current state and next action |
| --- | --- | --- |
| D-001 | Product scope / delivery | Asked, unanswered: complete web application with integrations selected separately, only existing/design surfaces, or the broader ecosystem including native apps/Forecast? Recommendation is the complete web application, preserving self-hosting, with named integrations agreed separately. No option is assumed approved. |
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
| D-012 | Governance conflict / 015 + consumers | Constitution still names three roles. Dashboard also retains a three-role assumption, while 012 contains old role-based financial boundaries. Reconcile through explicit governance and shared capability contracts before runtime cutover. |

## Proposed dependency order

This orders planning, not implementation authorization:

1. Confirm D-001 and complete the surface inventory, including commercial and
   integration boundaries. Create explicit accepted exclusions only with the
   user's decision.
1. Complete shared permission/approval evidence and migration governance in 015;
   define billing/date/identity contract ownership alongside it.
1. Reconcile and finish the existing dashboard, Clients, Workspace and Settings
   packages. Their independent research can proceed while specific decisions wait.
1. Specify remaining approved domains using shared contracts. Invoice/payment,
   expense, estimate and retainer boundaries must avoid competing money models.
1. Complete report/integration/import coverage against those domain contracts;
   review all cross-screen journeys and close analysis findings.

## Iteration log

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
