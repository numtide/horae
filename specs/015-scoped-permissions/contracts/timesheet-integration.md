# Timesheet person context and operation integration

### Modal focus during permission-bound refresh

Closing the entry modal returns focus to its opener when that control remains
available. A successful create/update/delete or timer start refreshes the scoped
page and tracking options, which can disable or remove the opener. In that case,
use the existing shared Modal fallback to `app-main`; do not retain stale tracking
authority or delay-steal focus after the refresh. Cancel/Escape still return to
the available opener. Browser acceptance must verify persisted deletion after
the new response, not the transient empty grid while authority is reloaded.

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

### Candidate discovery implementation contract

The selector is navigation, not a report filter over a particular week's rows.
Its candidate set therefore does not depend on the selected dates: choosing an
empty week must not remove the person currently being viewed. This follows the
confirmed zero-entry rule and preserves the already-authorized historical read
scope; it is an integration decision, not newly observed Harvest behavior.

- Require the active session actor, canonical policy 1 and strictly restored time
  grants, using the same organization/actor read fences as scoped entry reads.
  Do not use directory access, legacy role or Administrator identity as a bypass.
- `TimeReadOwn` discovers the actor; `TimeReadAll` discovers active local people.
  `TimeReadManaged` discovers active directly managed people plus active people
  participating in a currently managed project, even without time entries.
  Also retain active people with authorized historical entries in a currently
  managed project after their tracking membership is removed. Union these sets
  without duplicates; do not restore membership or tracking authority.
- Archived people are excluded from this Timesheet-only projection. Archived
  projects/tasks do not erase history. Qualify projects, clients, tasks and
  historical entries by tenant just as the existing scoped reader does; malformed
  cross-tenant parent links must not establish candidate visibility.
- Return only candidate ID/name and the session-derived requester. A single-ID
  narrowing filter supports restoring a selected person beyond the first page;
  unknown, foreign, archived and out-of-scope IDs all yield an empty result.
  Search and the exclusive `(name,id)` cursor only narrow authorized rows.
  Bound each page to 50 with an explicit continuation, using the existing people
  cursor; trim search, reject NUL and search over 100 characters. These are
  transport bounds, not claimed Harvest limits or a cap on organization size.
- Reauthorize every page and selected-ID lookup. After grants, assignments or
  activity change, the next request must reflect current scope. The returned
  identity is not authority for time reads or mutations. No candidate payload
  contains email, activity flags, rates, grants, project lists or hour totals.

T179–T181 exercise zero-entry discovery, removed membership/history, each grant
family and all built-in profiles, duplicate paths, archived/foreign data,
pagination/search/narrowing, registered HTTP payloads, both revocation lock orders
and cancellation. This is the first backend step of the joint integration below;
it does not satisfy selected-person UI or delegated-write acceptance by itself.

## Actual consumers to integrate together

### Existing own-week submission identity boundary

The current `submit_week` consumer must capture the same `TimesheetWriteContext`
as ordinary commands. Require its requester to match the authenticated session,
its subject to be that requester, and its policy to be `LegacyOwn`. Inside the
existing submission transaction, take the organization shared gate, verify
policy 0 and lock/recheck the active local owner before taking the exclusive
time-write barrier. Hold these facts through commit. A stale tab, session change,
deactivation or cutover must not submit another person's sheet or invoke legacy
submission under canonical policy. Preserve the existing weekly submission and
rounding behavior; this is not the final flexible/delegated submission contract.

T014/T015 tests must use registered session calls for stale requester, subject,
organization and policy; test actual revocation commit/rollback and the reverse
lock order with PostgreSQL. The UI retains the pending guard and refreshes its
sheet after either result. This closes an identity gap without deciding any of
the open approval-coverage, expense or submitted-editing rules.

### Tracking eligibility and terminal recovery — 2026-10-04

The official [tracking troubleshooting guide](https://support.getharvest.com/hc/en-us/articles/27133087832205-Why-can-t-I-track-time)
and [project setup guide](https://support.getharvest.com/hc/en-us/articles/360048686831-Create-and-duplicate-projects)
require project assignment and a task added to that project. Treat time-write
grants as independent from the selected owner's tracking context; they do not
themselves assign a person or remove FR-006 task restrictions. This is an
implementation interpretation of documented prerequisites, not an observed
custom-grant bypass test. Do not copy the legacy SQL view's role-name bypass
into the canonical predicate or silently change policy-0 behavior.

The user selected **B** for the undocumented recovery edge: if the owner loses
tracking eligibility while their timer runs, the terminal stop exception remains
**owner-only** (FR-033). A delegate must first have an eligible owner/context again
and still hold current scoped write authority. This never waives other locks or
permits creation, restart, reassignment or general editing. Record it as Horae's
decision, not verified Harvest enforcement; do not ask the same question again.

The next delegated transaction tests must contrast actor and owner, eligible and
removed/restored membership or task access, read-only versus write scopes, and
both revocation orders. Keep own terminal recovery separate from delegated stop.
Approval coverage and privileged correction questions remain independent.

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

1. Implement and verify the candidate discovery/date-history contract above
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

## Selected-person integration boundary at `60f60f9`

Source review confirms the handoff's Day/Week/Calendar layout, but its complete
`04_Timesheet.dc.html` contains no teammate selector or delegated warning state.
Compose these from the existing controls under the confirmed missing-mockup
authorization; do not invent an alternate Timesheet layout. The current
`ProjectTaskPicker` loads the session person's contexts internally and therefore
cannot safely be reused unchanged for a selected teammate. The shell's use of
that component must remain session-owned.

The next implementation is a connected consumer change, not another independently
delivered permission reader:

1. **Resolve the page context.** Distinguish authenticated requester from selected
   subject and include both in every load/mutation context. Select the legacy or
   canonical path from an explicit server policy result, never from a failed
   canonical request. Policy 0 keeps the existing own sheet; canonical failures,
   unknown versions and inaccessible subjects do not fall back to it. Do not
   activate policy as part of the UI change.
1. **Preserve navigation identity.** Carry the selected subject through Day, Week,
   Calendar, date navigation, span changes and browser history. Own-sheet links
   remain valid. Invalid or inaccessible subject IDs must not silently become a
   different person's sheet. Resolve selected IDs outside the first candidate
   page using the verified narrowing query.
1. **Bind asynchronous work.** A loaded page belongs to requester, subject and
   date window; only the matching current context may display its entries,
   totals, errors or mutation result. Discard obsolete responses and clear stale
   dialogs, pending rows and drag state on accepted context changes. Pending
   mutations retain their original context; a later selection cannot retarget
   them. Reuse existing dirty/pending navigation conventions instead of silently
   dropping unsaved inputs. Server mutations independently check the expected
   requester and selected subject against current authenticated authority.
1. **Replace the data dependency, not the facts.** Day/week/calendar currently
   consume `TimeEntry`, own tracking project/task lists and `list_clients(true)`.
   The canonical path must consume the safe scoped entry projection and its
   labels; do not fabricate a full internal entry or fetch the general client
   directory merely to name historical work. Collect all authorized pages before
   displaying period totals, keeping each total equal to displayed source rows.
1. **Separate tracking choices from historical labels.** Creation and editing
   need target-specific eligible project/task combinations and current operation
   scope. Session-person tracking contexts do not describe a teammate. Historical
   labels remain readable after tracking membership changes without restoring
   tracking eligibility or granting project management.
1. **Connect every action.** `persist_entry`, week cells, row removal, dialog
   save/delete, Day start/stop and Calendar move/resize/reorder must carry the
   same captured subject. The current edit dispatch omits project/task changes;
   the current row-removal loop issues independent deletions and reports partial
   results. Neither may be mistaken for a delegated atomic command. Pin and test
   the complete affected set before enabling a corresponding bulk affordance.
   Keep source/destination scope and independent locks; no one `can_edit` flag.
1. **Retain shell ownership.** `RunningTimer` refreshes the session timer and
   invalidates page data. A teammate action may invalidate these resources but
   must never replace the shell timer owner or operate on it accidentally.

Required discriminating tests include an A→B selection while A's load/save is
pending, a requester change between pages, a stale dialog after selection,
unreadable selected IDs, a selected person beyond candidate page one, mixed
project read/write scope, archived historical labels and a concurrent own timer
while viewing a teammate. Test Day/Week/Calendar and route history together.
Submission/withdrawal and locked corrections retain their explicit coverage and
company-lock dependencies; do not hide those requirements behind this boundary.

### Atomic page context

`load_timesheet_page(TimesheetQuery)` now resolves the authenticated requester,
selected active subject, explicit policy and one authorized entry page in the
same bounded transaction. This is the read boundary for the connected consumer,
not completed Day/Week/Calendar integration or delegated commands.

- Policy 0 admits only the session person's sheet, without requiring canonical
  permission state. Policy 1 uses the existing strict grants and candidate/entry
  predicates. Missing or unknown policy and invalid stored state do not fall
  back to legacy mode. No activation or role remapping occurs.
- An absent subject means the requester; an explicit inaccessible, foreign,
  archived or unknown subject is denied, never replaced by the requester. Empty
  managed-project participants retain the user-confirmed navigation rule without
  exposing their unrelated time.
- Hold both requester and selected-subject activity stable until labels and rows
  are materialized. This covers independent target archiving as well as the
  existing organization policy/relationship fence. No database session defaults
  are changed; rollback/cancellation release locks.
- Return `requester`, minimal `subject`, `policy`, safe `entries` and `next_after`.
  Reuse the existing 500-entry descending cursor query and labels; no invoice,
  currency, rates, directory or authentication fields are added.
- The consumer must capture returned requester/policy/subject on the initial
  load, pass `expected_requester` and `expected_policy` on continuation and
  retain the selected subject. Mismatches are denied; these expected values are
  equality checks, never sources of authority. Exhaust pages before totals;
  there is no claim of a repeatable snapshot across different page requests.

Six transaction tests and the real registered-session matrix cover legacy own
compatibility, canonical subject resolution, empty membership, mixed-project
scope, identity/policy changes, inaccessible subjects, 501-entry continuation,
invalid query/state, target-archive ordering and cancellation. Existing candidate
and scoped-reader suites remain unchanged and pass. Native/WASM lint and full
SQLx verification are recorded separately in the progress log.

### Connected own-sheet consumer

Day, Week and Calendar now consume `VisibleTimeEntry` through the complete-page
loader, not `list_time_entries` or fabricated internal entry records. The loader
pins requester, selected subject and policy across continuations and rejects
identity changes, out-of-window/other-owner rows, duplicates and non-decreasing
cursors. A failed continuation returns no partial period totals.

The resource tags successes and errors with the requested week. Only a Ready
result for the current week supplies rows, totals or enabled entry creation;
pending/refused loads have explicit status and retry. Historical project, task
and client labels come from authorized entries, without `list_clients(true)` or
currency metadata. Existing own tracking choices remain separate.

Week-cell drafts live outside the refreshable grid, keyed by project/task/date;
refreshing after another cell's save must not destroy typed input. Restored
drafts save on blur, including retries without retyping. Row discard clears only
the captured week's drafts, and pending cell writes cannot race row removal.

This connects the current own-sheet route only. Selected-person navigation,
captured-requester mutation checks, target tracking contexts, delegated actions,
dirty/pending navigation, atomic row deletion and full submission remain open.
The existing own action endpoints are not evidence of canonical delegated write
enforcement. No policy is activated by this UI integration.
