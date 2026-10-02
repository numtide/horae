# Scoped permissions investigation progress

## 2026-10-02 — Resume PR #212 with the current Harvest account

- Objective: extract all available permission evidence from the existing account;
  retain full web parity, without substituting the legacy three-role model.
- Starting branch: `feat/scoped-permissions`, commit `6ce9071`, clean worktree.
  Reuse this worktree and PR; no runtime cutover, migration, purchase, invitation,
  permission change or merge is part of this research iteration.
- Read repository guidance, the proposed 1.1.0 constitution and feature artifacts.
  Executed Spec Kit clarification prerequisite discovery and plan setup with
  `SPECIFY_FEATURE=015-scoped-permissions`; the existing plan was preserved.
  No extension hooks are configured. Clarification questions are reference
  questions first, not requests to repeat the confirmed product scope.
- Continued Phase 0 research. The full-task artifact explicitly remains incomplete,
  so a complete `speckit-analyze` pass is not yet eligible. Do not describe a
  focused evidence review as full-feature analysis or acceptance.
- Reopened current official permissions, flexible approval, user API, person
  profiles, teammate-assignment and retainer documentation. New-model and legacy
  descriptions coexist; an administrator-only legacy endpoint is not proof of
  the new web capability boundary.
- The historically observed permission-editor asset now returns HTTP 404.
  Preserve the historical observation, but reacquire the current asset through
  an approved live browser before treating its source as fresh evidence.
- Started the existing Horae Playwright MCP client. Browser access is awaiting
  extension approval; no fresh account snapshot has been obtained yet. Existing
  snapshots remain dated evidence, not a revalidation of the current account.
- Next action: finish the read-only account probe register, inspect the current
  editor/catalog and owner-visible assignment/approval/billing controls after
  connection approval, then reconcile findings and remaining evidence limits.

The full-feature checklist remains 12/16. T006–T009 and runtime acceptance remain
open. This entry records progress, not completion of the investigation or PR.

### Evidence checkpoint and review

- Added `contracts/current-account-investigation.md`: 15 safe read-only probes,
  six documentation findings, source links and explicit limits. Linked it from
  the existing research and evidence register, preserving historical observations.
- Independently reviewed the approval reference, then directly checked the new
  company-cutoff guides. Access through links in the general approval article
  succeeded where direct opening failed. Company locking, submission, scoped
  approval and invoicing must not be conflated.
- The old public editor asset returned 404. A current asset cannot be selected
  from that historical name; capture the asset actually loaded by today's page.
- Adversarial review of this documentation increment found no actionable issues.
  It did not validate browser connectivity, pending probes or full-feature parity.
- `nix fmt --` on the four changed Markdown files and `git diff --check` passed.
  No Rust, CSS, SQLx, migration or runtime change; no new application test or
  full-flake result is claimed for this documentation checkpoint.
- Product clarification questions asked/answered: 0. The confirmed scope is
  unchanged. Evidence questions remain; no new spec requirements were invented.
  Checklist re-evaluation remains 12/16, with no newly passing items or regressions.
- Connection checkpoint: both browser-list calls timed out after 240 seconds
  each on the same MCP client. No account page was read in this session. The
  client remains available, but no browser operation is reported as a successful
  or still-running probe. Await connection approval/evidence before another retry.

| Clarification coverage | Status and next evidence |
| --- | --- |
| Functional scope and personas | Scope clear; detailed operation mapping still partial |
| Domain model and lifecycle | Partial: profile persistence, approval transitions and independent locks |
| Interaction and UX | Partial: 15 read-only probes pending browser access |
| Quality attributes | Security boundaries clear; full-feature validation not performed |
| External dependencies | Current documentation reviewed; live account access not yet established |
| Edge cases | Partial: rate conflict, assignment promotion and overlapping approval coverage |
| Constraints and tradeoffs | Clear: existing account, no purchases/invites/writes or weakened parity |
| Terminology | Distinguish profile, descriptive role, assignment, approval and company cutoff |
| Completion signals | Clear: research evidence is not implementation or merge acceptance |
| Outstanding reference questions | Retained explicitly; not converted to product defaults |

Next action remains the same live-account investigation, not a new feature:
approve Playwright's connection in Chrome, then execute R01–R15 and attach dated,
redacted outcomes. Do not repeatedly retry the browser without new evidence.

## 2026-10-02 — Approved browser connection and fresh account inspection

The earlier connection limitation is superseded. The same Playwright MCP client
connected after user approval; a dedicated Harvest tab was used and the unrelated
tab preserved. Continued Spec Kit plan Phase 0; setup preserved the existing plan,
and no extension hooks are configured. This is not a completed full plan/analyze.

- Revalidated one immutable owner, no archived people and a paid second-seat gate.
  Owner profile selectors and save remain disabled. No bypass or invitation.
- Retained a fresh 50-grant/11-category catalog and six profile defaults, plus the
  current public editor asset with SHA-256 provenance. Unknown IDs stay unknown.
- Inspected assigned projects/people, rates, project editing, approval status views,
  preferences/modules, expenses/categories, detailed expense report, invoice draft
  actions, retainer form, reports and saved reports. Account activity is plan-gated;
  estimates are disabled. Captures stay in ignored scratch, not Git.
- The project editor raised a leave-page dialog despite no field changes. Accepted
  leaving without saving; the next Reports page loaded successfully. This is not
  evidence of a business mutation. Expense editing was explicitly cancelled.
- Added C01–C07 conflicts/discrimination cases. Most importantly, template deletion
  help promises grant preservation while source warns of possible Member downgrade.
  Marked the expectation provisional consistently in spec, model, tasks, quickstart
  and evidence; neither alternative is approved for implementation from this alone.
- Report-specific data access, managed rates, legacy cost help, deadline gating,
  custom approval dependencies and project-manager loss preview remain distinct
  questions. No owner success is promoted into non-owner enforcement evidence.
- No saved profiles, assignment changes, lock/submission/approval actions, invoices,
  payments, retainer funding, settings changes or migrations. No runtime/CSS edits.

Full-feature checklist remains 12/16; T006–T009 and runtime acceptance remain open.
Next: finish focused evidence review/formatting, publish this research increment
to #212, then use the recorded discriminating cases when separately authorized
editable non-owner reference access and disposable mutation fixtures are available.
Do not repeat owner-only probes as substitutes or request a parity simplification.

### Research increment validation

- R01–R15 now have inspected outcomes or explicit current-account access limits.
  Settled current-week/all-time approved views are empty; an actual detailed
  expense report was run read-only. No empty view is treated as transition proof.
- Independent adversarial review found one low-severity provenance omission:
  owner Archive/Delete controls required the expanded-menu capture. Added the
  existing capture path; no critical/high findings in the reviewed increment.
  The final R09/R11 additions and progress entry were subsequently checked by the
  primary reviewer against their raw captures, not claimed as part of that review.
- The owner DOM contains hidden generic reset copy, not an observed deletion;
  it was read without changing visibility. This strengthens C01's conflict record,
  not either proposed persistence outcome.
- Targeted `nix fmt --` and `git diff --check` passed for the Markdown increment.
  No application tests/full flake run claimed for documentation-only changes.
- Current-account read-only research is recorded; full-feature checklist remains
  12/16. The next implementation gate is still T006's verified operation matrix,
  followed by migration/dependency review and detailed tasks. PR remains draft;
  no merge or runtime activation is authorized by these observations.
