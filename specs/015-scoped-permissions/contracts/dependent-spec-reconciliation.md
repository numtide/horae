# Dependent permission contracts

T008 partial research, 2026-10-02. This register propagates accepted decisions and
identifies remaining integration gates; it does not activate policy, merge other
branches or certify their implementation. No new product answer is inferred.

## Source revisions

Use each feature's latest inspected branch, not the older copy of its spec in
the permissions worktree. Source paths below are repository-relative at these
revisions; revalidate them when integrating:

| Surface | Inspected source |
| --- | --- |
| Project list / bulk / shared editor | `9c07d60`: `specs/009-project-list-design/spec.md`, `specs/010-project-bulk-actions/spec.md`, `specs/011-new-project-screen/spec.md` |
| Project Detail | `feat/project-dashboard` at `48a4156`: `specs/004-project-dashboard/spec.md` |
| Clients | `feat/clients-mvp` at `5a459c4`: `specs/012-clients-design/spec.md` and `increments/mvp/spec.md` |
| Workspace | `feat/workspace-design` at `02d876f`: `specs/013-workspace-design/spec.md` |
| My Settings | `feat/personal-settings-design` at `c1f8856`: `specs/014-personal-settings-design/spec.md` |

The other worktrees were inspected read-only. Their pending files are not copied
over older tracked specs, and their unrelated product/visual decisions remain
with those features.

## Reconciliation and acceptance ownership

| Surface / clause | Accepted target or preserved boundary | Remaining work and tests |
| --- | --- | --- |
| 011 US7 save safeguards and cost-override assumption | FR-022 replaces Administrator-name cost access with effective cost read/write at cutover, without granting private-note or unrelated project editing. This branch adds a conditional transition note and removes the conflation of notes and costs | T014/T015: custom cost reader/writer, read-only denial, redacted saves preserving hidden fields, direct editor requests and revocation; legacy behavior remains until activation |
| 011 FR-015 team and project rates | FR-021 follows the owning project for its person/task overrides; personal defaults remain person-scoped. FR-025 retains existing designations with read access without restoring editing; FR-026 permits project-editor delegation to compatible targets without global grant changes | T012/T013/T014: person/project cross-product, retention versus delegation, compatible managed-read targets without existing designation, no implicit promotion, removal without target grants, revocation and atomic multi-person saves; project creation remains independently authorized |
| 011 FR-013 budget alerts and 014 FR-012/013 notifications | Creator/manager recipient selection is not permission to disclose amounts. FR-025 allows retained managers without financial access; 014 already requires delivery-time authorization | T014/T015 and owning notification tests: check each recipient and payload at delivery; omit unauthorized fee/cost data without inventing financial prerequisites for non-sensitive hours/progress notices |
| 011 privileged approval/import assumption | Legacy authority remains only until reviewed cutover. FR-024 requires current approval authority and full-selection visibility, including expenses; imports retain their separate Administrator boundary | T012/T014/T015: no stale-privilege approval, hidden expenses or partial mutation of a denied selected set; legitimate scoped approval remains supported; no import authority expansion |
| Latest 004 FR-009/017 and SC-005 | Already defer to 015 effective financial/resource scopes. FR-008 report-only projection must not become ordinary dashboard/source access; FR-021/022 distinguish project rates and organization cost grants | T014/T015 plus dashboard acceptance: negative payloads, authorized aggregate contributors, charts/tables/exports, inherited rates without unrelated history, currencies and independent notes/invoice access |
| 009 manager-only controls and 010 FR-001 bulk guard | Their delivered legacy behavior is not the final permission model. Current capability and actual selected project coverage will replace role names at cutover; atomic batch and no-history-change rules remain | OP13 lifecycle predicate is not resolved by ordinary editing. T014/T015 must cover managed/all scope, unauthorized IDs and revocation before any role check is replaced |
| 012 full feature FR-003 / SC-005 | Already defers to 015. Client management alone does not grant source rate, project, invoice or import authority | OP17/18 rate-default/contact/lifecycle predicates remain open. T014/T015 plus 012 verify fields, counts, search and contextual destination checks |
| 012 MVP-003 and its explicit no-cutover clarification | Deliberately preserves legacy authorization for the delivered increment; this is not a contradiction requiring early policy activation | Re-run increment regressions after the actual cutover. Its existing test evidence does not prove six-profile acceptance |
| 013 FR-002/003 and 014 FR-007/008/009 | Both already use 015 for effective permission explanations and assignment distinctions. FR-021/022 apply to Rates; being the subject is not authority. FR-025 read-only retention must not display project-edit powers; FR-027 limits person-management assignment controls to Administrators without blocking authorized ordinary people operations | T016/T017 plus 013/014: consistent descriptions, reachable non-admin own explanations and permitted people destinations, no privilege controls from descriptive labels |
| 013 FR-002 Workspace settings restriction | Administrator-only settings conflicts with OP27's candidate custom CompanyRead/CompanyWrite mapping. This is unresolved, not an accepted blanket boundary or authority expansion | Resolve per-operation settings predicates before cutover. FR-023's Administrator-plus-company-write company-lock rule does not decide all preferences; permission administration/audit remain separate |
| 013 administrative Audit and operational approval history | FR-013 protects permission-change audit. OP46 is a separate scoped operational-history projection; do not expose privilege snapshots there or make all approval history admin-only | T012/T014/T015/T041: own/managed/all history and exports versus denied permission audit, current scope and mixed-scope event filtering; custom activity-history predicate remains open |

## Integration safeguards

- Keep the three-role runtime and its current fixtures until a reviewed atomic
  activation covers all delivery paths. Do not modify tests merely to assert
  that a target profile already works.
- Keep preserved private fields distinct from cleared values on composite saves.
  A redacted read is not permission to overwrite unseen data with defaults.
- Keep feature-local requirement numbers qualified: 015 FR-024 is combined
  approval visibility; older features use the same number for unrelated rules.
- Person-management target eligibility/retention, creation/keep-access behavior, Workspace settings,
  invoice/contact/lifecycle scope, activity-history
  grants and reviewed migration still prevent closing T008/T009 or full Analyze.
  This register does not choose them or reopen accepted FR-021/022/024/025/026/027.
