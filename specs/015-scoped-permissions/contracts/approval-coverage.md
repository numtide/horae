# Approval lifecycle and coverage

Status: implementation-boundary review, 2026-10-04, against `3f45b7c`.
Owns the remaining T006/T009 approval work before T012/T013. This is not a
closed schema contract, browser observation or permission to activate policy.
Read with [combined visibility](approval-visibility.md) and
[serialization](permission-state.md).

## Reference boundary

The [flexible approval guide](https://support.getharvest.com/hc/en-us/articles/39974542812429-Flexible-timesheet-approval)
documents selected submitted dates, functional project filters, empty-cell
protection and whole-timesheet coverage when authority and filters qualify.
Submission alone does not prevent editing. Approval-page withdrawal is scoped;
Day/Week withdrawal covers the week. Self-approval has a separate setting.
This newer flow is the FR-019 target, not the weekly-only behavior still described
in the [FAQ](https://support.getharvest.com/hc/en-us/articles/360048181912-Timesheet-approval-FAQ).
The FAQ also confirms that time and expenses are approved together.

The [submission guide](https://support.getharvest.com/hc/en-us/articles/360048181832-Submitting-and-approving-timesheets)
documents custom submission ranges and a confirmation step when expenses are
reviewed. It explicitly provides requesting changes rather than rejection.
The legacy `reject_submission` endpoint is therefore not the target workflow.

[Unlocking approved work](https://support.getharvest.com/hc/en-us/articles/4408205049869-Unlocking-approved-time-and-expenses)
does not specify the resulting submission status. Its legacy Administrator-only
description cannot override the flexible guide's scoped withdrawal permissions.
[Independent locks](https://support.getharvest.com/hc/en-us/articles/360048687491-Unlocking-time-and-expenses)
must be evaluated separately: withdrawing approval does not remove invoice,
archive or company protection.

All sources were read on 2026-10-04. No interactive browser/MCP tool was loaded.
An empty owner approval list from an earlier session cannot resolve lifecycle or
restricted-actor behavior.

## Facts that must not share one state field

The following are Horae design requirements derived from FR-009/019/024, not a
claim about Harvest's internal database:

- Submitted date coverage exists independently of the presence of source rows.
- Approval coverage identifies subject, dates and applicable project coverage.
- Time and expense source records retain their own identities and exact values.
- Invoice, archive and company locks remain independently effective.
- Historical actions retain actual actor and affected scope, even when current
  coverage changes. A permission-change audit is not approval history.

Do not choose between per-date rows, ranges or whole-timesheet markers yet.
In particular, freezing only existing entry IDs cannot protect empty cells.
Conversely, treating every filtered approval as a person-wide week can widen it.
FR-009's preservation of other projects' approval **state** must not silently
become an assertion that empty-project **coverage** can never widen under the
documented whole-timesheet rule. Pin that distinction before schema selection.

## Current implementation replacement map

| Current symbol or surface | Required replacement boundary |
| --- | --- |
| `approvals::submit_user_week` | Fixed seven-day range, nonempty open-time requirement and row-state freezing are not a complete custom-range submission contract. Preserve integer rounding; explicitly reconcile frozen values with later permitted edits. |
| `approvals::approve_periods` | Updates selected weekly rows, silently skips unavailable IDs and transitions only submitted time. Replace with explicit selection, complete time/expense acquisition, coverage and atomic FR-024 authorization. |
| `approvals::reopen_period` / `reject_submission` | Deletes the submission and clears rounding across a week. Do not reuse as scoped withdrawal or infer the target post-withdrawal state from it. |
| `db::lock_time_entry_write` / `begin_time_entry_write` | Existing shared user barrier coordinates time writers with submission. It is not a complete approval/expense/revocation boundary. |
| `time_entries` mutation functions | State-based submitted locks must become the reviewed independent lock checks; moves must check source and destination. Preserve terminal timer behavior and exact totals. |
| `importers::harvest::apply` | Joins the time barrier on insertion. Joining a barrier alone does not check newly approved coverage. Imports must participate in the final lock contract. |
| `approvals::approve_ids`, models, UI and plugin payloads | Weekly post-commit totals and one approver field cannot represent a filtered partial action. Capture truthful committed scope without leaking unrelated work. |

The design handoff `design/project/app/06_Approvals.dc.html` already has client,
project, teammate and date controls and a combined approval action. Its sample
rows are not a lifecycle or authorization oracle. No CSS/UI changes are part of
this contract review.

## Transaction requirements to reconcile in T042

These are implementation constraints, not verified Harvest storage mechanics:

1. Establish bounded READ COMMITTED operation and the organization gate before
   participant/resource locks; reload current actor, policy and assignments.
1. Resolve explicit selection without silently removing unreadable records.
   Reject unauthorized identifiers without disclosing their existence. Do not
   substitute list pagination or grouping for mutation scope.
1. Fence creation, movement and editing of both domains before loading the
   final affected set. A row lock on existing entries cannot fence an absent
   expense or empty date. Reuse the user barrier only after every participating
   writer and foreign-key/trigger order is reconciled. Bulk subjects need a
   deterministic order, not browser order; no shared-to-exclusive upgrade.
1. Under those fences, evaluate submission coverage, operation-specific
   authority, self-approval and FR-024 on the complete current records. Check
   coverage authorization separately; a successful empty-slice guard proves none.
1. Persist selected effects, historical scope and command outcome atomically.
   Rollback, cancellation or denial must leave neither a partial domain change
   nor a successful history event. Do not hold locks over mail/plugin delivery.
1. After a wait/retry, recompute current authority and affected records. Do not
   retry only the final UPDATE or treat a stale preview as authorization.

The existing approval transactions do not implement this protocol. A proposed
organization-first order is not proof that all import, invoice, archive, expense
and timesheet paths can safely participate. T042 remains open.

## Acceptance fixtures and unresolved discriminators

These refine T012/T013; they are not passing tests:

| ID | Fixture / required check | Basis |
| --- | --- | --- |
| AC01 | Submitted Monday–Wednesday; select Tuesday–Friday: Thursday/Friday stay outside approval coverage. | Flexible guide, FR-019 |
| AC02 | A and B both submitted; approve only A for Tuesday: B and other dates unchanged; A's empty cells protected. | Flexible guide, FR-009/019 |
| AC03 | Exercise Approval withdrawal and Day/Week withdrawal independently; retain invoice/archive/company locks. | Flexible and unlocking guides, FR-019 |
| AC04 | Edit submitted, unapproved work; approver sees current values and totals, including changed rounding inputs. | Submission guide, FR-019 and Constitution I |
| AC05 | Hidden selected expense, new expense racing approval, and revoked read/approval/assignment in both race orders. All effects or none; no private denial detail. | FR-024; real feature 016 records required |
| AC06 | Approve A then independently B; partial/complete display and historical actors remain truthful; retry/cancel cannot duplicate or erase history. | FR-009/010, SC-008 |

Open discriminators, to resolve with suitable reference fixtures or an explicit
Horae product decision rather than guessed expected outputs:

- A is the only submitted project; filters capture A. Verify resulting coverage
  for an existing empty B and a project created **after** approval. Separately
  test people-management and project-management authority; no universal wildcard
  or creation-time project snapshot is chosen by this document.
- Withdraw a strict interior range from overlapping approvals: inspect remaining
  coverage, pending/unsubmitted status, resubmission and historical attribution.
  The ability to unlock does not decide which submission records survive.
- Verify own approval and withdrawal for arbitrary custom grants, plus the
  withdrawal visibility predicate. FR-024 answers approval visibility only.
- Submit an entirely empty interval and edit/move/add within a submitted one;
  verify submission eligibility and any review confirmation before finalizing
  the submission command. Preserve invoice/rounding facts independently.

## Expense dependency and execution gate

Read-only inspection of `feat/expense-parity` at `ba1b8e9` found reference/spec
artifacts under `specs/016-expense-parity/`, but no expense model, migration or
server module. Its `independent-work-audit.md` explicitly leaves implementation
and plan/tasks readiness open. Therefore “use feature 016 fixtures” is a real
implementation dependency, not an available test helper or permission to make
a mock-only approval implementation look complete.

Before T013 coverage storage: resolve the schema-affecting discriminators above,
review its concrete tenant/lifecycle schema and reconcile T042 writer ordering.
Before combined acceptance: integrate actual 016 records and transactional tests.
Before activation: complete operation matrix, preserved-data migration and
cross-surface enforcement. These are separate gates; do not wait on the global
cutover gate to do otherwise closed work, or waive a local unknown because a
pure guard already passes.

## Adversarial self-review

- Rejected existing-entry-only coverage: it cannot protect an empty cell.
- Rejected treating a filtered project as always isolated: whole-timesheet
  promotion needs a distinct fixture and must not be inferred from grouping.
- Rejected adopting legacy row deletion as withdrawal semantics or renaming
  “reject” without changing its destructive whole-week behavior.
- Rejected time-only acquisition, read-filtered acquisition and expense mocks
  as combined-flow acceptance; T136–T138 prove only their supplied record set.
- Rejected declaring the lock order complete from one command: current import
  and time writers must be reviewed with expense and coverage writers together.

The open discriminators remain material findings. This review does not close
T006, T009, T012, T013 or T042 and is not an independent review.
