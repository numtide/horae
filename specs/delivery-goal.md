# Harvest parity: specification delivery

Status: web scope confirmed; feature specification and clarification in progress; not implementation-ready.
Started: 2026-10-01. Baseline: `9301112c6a02ae3c92716273534f38889db241d1`.

## Objective

Complete the feature specifications, decisions, implementation plans and
verification tasks needed for the agreed Harvest parity scope and Horae's
`design/` handoff. Produce an implementation backlog that does not require an
implementer to invent product behavior or silently omit a designed control.

This is a specification-only delivery. It does not implement features, change
the database, replace the CSS system or authorize merging existing PRs. A ready
backlog is not a finished application. Implementation and release acceptance are
a subsequent delivery.

[The parity register](harvest-parity.md) records the inventory, source revisions,
feature ownership, dependencies and unanswered decisions. It is a coordination
document, not a replacement for `specs/<feature>/spec.md`.

## Authority and scope

- Apply `AGENTS.md` and `.specify/memory/constitution.md`. Preserve exact integer
  minutes/money, organization isolation and imported/development data.
- Use the user's confirmed decisions for product intent; use current Harvest
  evidence for behavior and `design/` for presentation. Record contradictions
  explicitly. A prototype handler, old implementation or unavailable test
  account does not establish a functional exception.
- Keep OIDC and the existing self-hosted, single-organization architecture unless
  the user explicitly approves a change through the relevant specification and
  governance process. Do not infer SaaS provisioning, subscriptions or new login
  mechanisms from Harvest parity.
- The six profiles, custom permissions and scoped approvals in feature 015 are
  already requested. Do not ask again whether three profiles or whole-person
  approval would be sufficient.
- D-001 is confirmed: cover the complete Harvest web application, including
  expenses, estimates, retainers and invoicing. Native applications, Forecast and
  new integrations are excluded from this delivery, not merely awaiting selection.
  Preserve existing Harvest import, OIDC, compatibility API and plugin behavior;
  the exclusion does not authorize removing working functionality.
- Third-party payment gateways and other new connectors are excluded. Invoice
  payment recording, outstanding balances and other native billing workflows
  remain in scope. Existing operational infrastructure is not a request to add
  new integration products.
- Historical v1 exclusions are evidence of the original scope, not blanket
  exemptions from this delivery. Supersede them explicitly where appropriate.

## Per-feature loop

1. Refresh the source revision, worktree status and owning PR. Preserve unrelated
   changes. Reuse an existing feature rather than creating a competing spec.
1. Read the applicable Spec Kit skill and its required references. Follow the
   checked-in helpers, template resolution, feature pointer and extension hooks;
   writing similarly named Markdown files is not sufficient execution evidence.
1. Investigate the current implementation, the entire relevant handoff and
   Harvest's documented/observed behavior. Record dates, source links, evidence
   limitations and the exact behavior being compared.
1. Run `speckit-specify` for a new feature or reconcile its existing specification.
   Define independently testable stories, requirements, failure cases and
   measurable acceptance. Allocate new feature numbers only after checking
   existing local/remote branches and directories; use full directory names to
   distinguish the existing features numbered 004.
1. Run `speckit-clarify`. Reuse recorded answers and ask one material product
   question at a time. Investigate reference facts rather than asking the user
   to guess how Harvest works. Save confirmed answers in the owning spec and
   update affected requirements, not only the central register.
1. Once blocking choices are resolved, run `speckit-plan`: complete research,
   constitution gates, data/lifecycle rules, contracts, migration implications
   and the verification guide. Explain why an artifact is inapplicable instead
   of leaving an unexplained gap.
1. Run `speckit-tasks`: trace every requirement to concrete, ordered work and
   verification tasks. Distinguish tests from implementation and feature-local
   work from shared prerequisites. Future implementation tasks remain unchecked.
1. Run the read-only `speckit-analyze` only after its required artifacts are
   complete. Report coverage and findings. Do not silently edit during analysis;
   obtain the required approval for remediation and then return to the owning
   specification/planning step. Reanalyze the changed artifacts.
1. Review the completed package adversarially: look for privilege escalation,
   financial inconsistencies, lifecycle races, omitted controls, inaccessible
   flows, false success, migration loss and regressions in shared styling.
   Distinguish self-review from any genuinely independent review.
1. Publish the scoped documentation changes in a PR; record its revision,
   remaining findings and dependencies. Repeat with the next eligible feature.

Do not run `speckit-implement` or an implementation-convergence loop in this
phase. Do not manufacture passing browser results or mark implementation tasks
done because their specification is complete.

## Cross-feature contracts

Resolve each shared behavior once with a named owner. Consumers must reference
the same contract and include integration acceptance cases:

- Effective capabilities, project/person scope, revocation and imported-role
  mapping: feature 015; amend the constitution explicitly before role cutover.
- Time submission, partial approval, empty-date coverage, withdrawal and
  independent invoice locks: feature 015, shared with time and expenses,
  invoice eligibility and Workspace preferences.
- Currency, rate inheritance, tracked versus invoice value, invoice contribution,
  unknown external billing and payments: existing billing/editor/import specs
  plus the billing-parity owner to be assigned during decomposition. Do not conflate a project
  fee schedule with a client's retainer ledger.
- Work dates, reporting intervals, workspace/personal timezones and week starts:
  time/reporting owners with features 013/014; no implicit historical rewrites.
- Identity admission, invitations, profile ownership and privileges: features
  013/014/015; an email match or import must not silently grant access.
- Notification delivery, retries, permission checks, generated artifacts and
  downloads: consuming feature contracts using existing jobs where appropriate;
  infrastructure availability is not proof of a delivered user workflow.
- Shared components/tokens, navigation and responsive states: feature-specific
  handoff acceptance plus a cross-screen regression matrix. Missing mockups need
  an explicit design decision, not omission of an approved feature.

## Completion gates

- [x] D-001 is answered: complete web application; no native apps, Forecast or new integrations in this phase.
- [ ] Each discovered surface maps to the confirmed scope, an explicitly
  approved exclusion, or a separately agreed follow-up. No silent omissions.
- [ ] Every in-scope behavior has a feature owner and dated evidence; reference
  uncertainty and design conflicts that affect behavior are resolved.
- [ ] Each feature has a reconciled spec, clarification record, research, plan,
  applicable data model/contracts, verification guide and executable task list.
- [ ] Each feature has requirements-to-tasks/test coverage and completed
  requirements-quality checks; no blocking clarification remains.
- [ ] Read-only analysis and adversarial review have no unresolved critical/high
  findings. Lower-severity findings have an explicit disposition.
- [ ] Cross-feature contracts, dependency order, constitution changes and
  existing-data migrations are agreed without contradictory definitions.
- [ ] Documentation validation passes; PRs contain only intended changes and
  their status is reported separately from specification readiness.
- [ ] The implementation handoff names the first executable work package, its
  prerequisites and its test/browser/CI acceptance gates.

## Pause and resume

Persist progress after each bounded iteration: source revision, completed stage,
actual evidence, open question, next action and owning PR. Prefer one coherent
feature per iteration over incomplete plans for every screen at once.

Continue independent, in-scope research when another feature needs clarification.
If further progress requires a product decision, paid account capability, browser
connection or unavailable credentials, state exactly what is needed. Never treat
silence as approval, buy seats, send mail, record payments or delete real data to
get through a planning gate. Resume from the register rather than repeating
completed research. Do not mark this delivery complete merely because the
coordination documents or draft PR exist.
