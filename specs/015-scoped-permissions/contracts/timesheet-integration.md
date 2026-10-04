# Timesheet person context and operation integration

Implementation map at `5ec183a`, checked 2026-10-04. This is the required
integration of OP02–OP05, FR-006/007/008/010/018/019, not permission to activate
canonical policy. The existing scoped reader and activity fence are prerequisites,
not substitutes for the screen and delegated commands.

## Reference findings

- The [new permission catalog](https://support.getharvest.com/hc/en-us/articles/44171549176077-Permissions)
  separates own, managed and organization time reads from writes. A visible
  teammate's entries are not all editable merely because some are writable.
- The [teammate Timesheet guide](https://support.getharvest.com/hc/en-us/articles/360048687591-How-to-view-edit-and-submit-another-person-s-timesheet)
  documents choosing another person, returning to one's own sheet, delegated
  timer controls and submission that can include expenses. This does not settle
  every custom-grant candidate or submission predicate.
- The [person archive guide](https://support.getharvest.com/hc/en-us/articles/360048687311-Archiving-deleting-and-restoring-people)
  explicitly keeps archived people's history in reports/invoices but not
  Timesheets. Do not remove archived history from the general scoped reader to
  implement this surface rule. Exclude archived people from Timesheet navigation;
  restoration does not automatically reinstate project assignments.
- The [running-timer guide](https://support.getharvest.com/hc/en-us/articles/360048687651-Why-can-t-I-invoice-approve-or-archive-items-when-a-timer-is-running)
  explicitly denies stopping another person's timer without time-edit authority.
  Delegated stop is not restricted to the timer owner, nor granted by read or
  approval access alone. Its legacy bulk-approval skip behavior does not replace
  Horae's accepted atomic-selection contract.
- The [Calendar guide](https://support.getharvest.com/hc/en-us/articles/39977971409549-Track-and-edit-time-in-the-Calendar-view)
  documents Administrator locked-entry notes/start/end correction in the
  Calendar form, but prohibits locked drag/resize and project/task changes.
  This surface-specific exception refines the general editing article's Day-only
  wording. Locked Calendar deletion and custom-grant equivalents are not inferred.

These are official documentation findings, not new browser observations. No
browser tools are loaded in this session, and the earlier client's connection
timeout was not retried without changed evidence.

## Confirmed zero-entry candidate decision

On 2026-10-04 the user selected A: managed-project access includes active project
participants who have not recorded time. Keep per-record/project read scope;
discovering that person never grants their unrelated time, directory details,
financial data or editing. Direct managed-person and organization-wide time
read scope remain independent. This is an approved Horae rule, not a claim of
observed Harvest custom-grant enforcement. Do not ask this decision again.

The final candidate contract must also preserve people with authorized historical
entries after membership removal, without restoring tracking eligibility, and
must state whether the candidate set depends on the viewed dates. Candidate
discovery must not fall back to the general people directory: time-read authority
does not confer directory, email, financial or permission-configuration access.

## Actual consumers to integrate together

| Consumer | Required change and acceptance |
| --- | --- |
| `TimesheetContent` resource and navigation | Bind selected person, date range and requester to the load. Exhaust scoped pages before totals; discard stale responses after person/session changes. Preserve own-sheet navigation and report historical access. |
| Project/task/client name maps | Use authorized entry labels for historical rows, and separately authorized selected-person tracking contexts for new entries. Do not require unrelated client/directory grants or fabricate full internal `TimeEntry` values. |
| `persist_entry`, week cells and modal | Carry explicit target identity on creation; immutable owner on edit. Support documented unlocked project/task changes. Reauthorize the complete affected set, including source/destination contexts and dates. |
| Calendar move/resize/reorder and delete | Evaluate each actual action and lock independently; read-only rows cannot trigger mutations. Reject an unauthorized mixed reorder without partial changes. Do not treat submitted state as blanket protection. |
| Page timer start/stop | Target the selected person under current write scope. Reading a timer is insufficient. Keep terminal own-timer recovery after lost task assignment distinct from delegated authority. |
| `RunningTimer` provider and `TimerWidget` | The shell continues to represent the session person's timer. Viewing or starting a teammate's timer must not switch the shell's owner. Refresh affected page and own-timer state without mixing identities. |
| Submission and status affordances | Integrate reviewed coverage and real time/expense transactions; do not relabel the current time-only weekly command as combined Harvest parity. |

The UI must express operation-specific states, not one cached `can_edit` boolean.
Server commands independently reload actor, target, assignments and lock facts;
UI flags are never an authority token. A project-limited reader cannot obtain a
person-wide total or mutation through grouping. Refused or revoked reads must
not fall back to legacy responses.

## Required delivery sequence

1. Bind the confirmed candidate choice to explicit discovery/date-history cases
   and close remaining ordinary/delegated tracking predicates;
   retain explicit gaps for coverage and privileged corrections.
1. Add real transaction/session tests and implement the selected-person read and
   write contract with consistent lock ordering. Reuse the catalog, stored grants
   and existing scope evaluator; no parallel policy engine or role-name fallback.
1. Integrate Day, Week, Calendar, dialogs and shell consumers under one selected
   context; read the full Timesheet handoff and its imports before UI edits.
1. Verify own/managed/all and read-only combinations, empty sheets, archived
   navigation, retained history, multiple pages, stale responses, source/destination
   denial, timer identity, revocation and keyboard/theme behavior. Run SQLx,
   native/WASM and final Nix gates; record browser limitations honestly.

Full activation still requires the operation matrix, migration review, real
expense/approval integration and cross-surface enforcement. This sequence does
not authorize a partially enforced rollout or omit any of those requirements.
