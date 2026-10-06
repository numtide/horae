# Task catalog and workflow authority

Owner: OP14/15/16, FR-006/007/008/010/017/018/021. Baseline `1b81680`.

## Reference and boundaries

Checked on 2026-10-06: Harvest's [permission reference](https://support.getharvest.com/hc/en-us/articles/44171549176077-Permissions)
separates task read/write from billable-rate read/write. The [rate guide](https://support.getharvest.com/hc/en-us/articles/360048181492-Setting-billable-rates)
distinguishes global task defaults from project overrides; editing a default
does not rewrite saved project task rates. The [task dropdown explanation](https://support.getharvest.com/hc/en-us/articles/360048181852-My-new-task-isn-t-appearing-on-my-timesheet-s-task-dropdown-Why-not)
requires tasks to be enabled on the selected project before time can be logged.
These sources support the existing operation matrix and FR-021; they do not
settle every composite linking/lifecycle effect or custom-grant combination.
No restricted-user Harvest browser observation is claimed in this pass.

Follow-up documentation checked on 2026-10-06:

- [Task management](https://support.getharvest.com/hc/en-us/articles/360048181332-Creating-editing-archiving-and-deleting-tasks)
  distinguishes global defaults from saved project configuration. Renaming
  affects existing projects; default billing/rate changes apply to future ones.
  Global archive preserves time history but disables the task on its projects.
  Global restore does not restore those project associations: each requires an
  explicit project restore. Common tasks and add-to-all-projects are separate
  operations, not incidental consequences of an ordinary edit.
- [Running timers](https://support.getharvest.com/hc/en-us/articles/360048687651-Why-can-t-I-invoice-approve-or-archive-items-when-a-timer-is-running)
  prevent task archival. Historical recovery fixtures may contain an archived
  running timer, but they do not authorize creating that state through archive.
- The [public task API](https://help.getharvest.com/api-v2/tasks-api/tasks/tasks/)
  documents a create/edit permission prerequisite for its endpoint, whereas the
  web permission reference distinguishes task read and write. Horae's existing
  read-only session compatibility surface follows OP14's read contract; this
  is not evidence of exact public-API authorization parity. Do not silently
  replace that contract with a write requirement.

T230 must reconcile archive/restore association effects and timer exclusion,
not merely put a new grant check around the existing boolean activity update.

The existing pure rate evaluator identifies a catalog default as `GlobalTask`:
managed-project financial authority does not expose or change it. A task name
needed for an authorized project or time workflow is not whole-catalog access.
Tracking identity is not permission to log a new entry; the existing time-context
validator remains authoritative.

## Baseline paths and required integration

- At `1b81680`, `projects::tasks_for_viewer` joins the legacy `task_read_access`
  view for catalog, project and tracking callers. It therefore both rejects a
  canonical task reader with legacy Member role and inherits catalog/rates from
  a legacy Administrator without current grants.
- `list_tasks` is used by the legacy administrative task section.
  `list_tracking_tasks` supplies timer identities. The main project editor has
  its own context-bound catalog; the detail page now uses minimal bound labels.
  `list_project_tasks` remains an authenticated endpoint even without a current
  production UI caller and must enforce current project/workflow authority.
- At that baseline, compatibility `list_tasks`/`get_task` independently use the legacy view;
  count and rows are separate statements. Their filters/direct IDs, financial
  projection and current authority need equivalent enforcement.
- Direct create/update/activity/link endpoints retain legacy guards. Update
  lacks actor identity inside the transaction, and its nullable rate input
  cannot represent protected unchanged intent. Do not wire a canonical form to
  this transport until omission, reset, zero, response redaction, replay and
  indirect project effects have been reconciled with the existing editor.

## Read contract

In canonical policy, ordinary global catalog reads require `TaskReadAll`.
Global default rates additionally require `BillableRateReadAll`, independent of
legacy roles, project management and financial-report grants. Omit withheld
rates; do not manufacture zero. Preserve policy-zero behavior until reviewed
activation, including its existing historical identity semantics.

Tracking reads remain rate-free. A current task catalog reader may obtain its
task identities; an ordinary member retains identities from their own
tenant-consistent time history or project membership. Only own historical tasks
may bypass the active-task filter. This does not enable archived tasks for new
time. Project-specific identities require their linked project/workflow scope;
global task read alone must not enumerate unrelated private project contents.
For the retained project-task endpoint, explicit project read (all, or managed
with a current designation) or the actor's own tracking membership supplies that
workflow scope. Membership does not require shared progress reports. Global
default rates still require all-rate read, even on a managed project; this DTO
does not return project override rates. An unlinked task is never included.

Use strict stored-policy loading and current active actor checks under the
existing organization/actor transaction fences, held through materialization.
Missing/malformed canonical state and unknown policy must not use legacy roles.
Tests must include foreign parents, revocation before/after a lock wait, inactive
actors and preservation of existing tracking behavior, plus real session/API
delivery before claiming this boundary complete.

## Remaining complete delivery

Implement the direct task writes and their independently protected field
intents, catalog/compatibility readers, consumer affordances and applicable
project association effects. Source tests alone do not close the feature.
No new dependencies, permissions, database activation or real-data changes are
needed for the initial read integration. Full transition and Nix gates remain.

The management-compatible API must preserve its active filter, stable ordering,
page count and direct-ID not-found behavior while requiring canonical global
task authority. Count and page must come from one visible snapshot, including
empty pages. Do not reuse the tracking exception to widen the management API.
Its policy-zero projection stays compatible; release/source races and malformed
state need route-level tests before T229 can close.

## Read implementation acceptance

The working read implementation reuses the organization/active-actor transaction
from project reads; it does not introduce a second permission loader. The ordinary
task reader and compatibility count/page query now branch explicitly on stored
policy. The compatibility route never uses historical tracking identity as catalog
authority. Rate omission is performed in SQL before constructing either payload.

The extended verification has passed 29 compatibility tests, 123 project tests
(including twelve task-reader cases), and the registered Dioxus session matrix.
All-target SQLx preparation, offline test compilation and strict native/WASM
lint also passed on that final source snapshot (`93182`, exit 0). Together with
the browser checks and scoped review below, this completes T229, not the full
task-management or permissions feature. Full Nix and activation gates remain.

The added `task-read-permissions` browser check covers the actual shell timer:
archived own-history labels, absence of unrelated catalog identities, rate-free
tracking even with all-rate grants, and the distinction between catalog visibility
and enabled project/task choices. It exercises start/stop only in the browser
runner's disposable database. It passed alongside `mobile-navigation`,
`menu-popovers`, `project-editor-permissions`, `project-task-rates` and
`timesheet-permissions`. The real-pointer regression also exposed a sidebar
resize handle intercepting Cancel. Its scoped fix suspends pointer interception
while either rail popover is open; browser checks verify both menus, restored
dragging and existing mobile behavior. No tokens or shared menu defaults changed.

The actual canonical task-management page and direct writes remain T230. In
particular, the shell label resources are not a claim of completed cross-session
consumer binding, and the editor's separately bound minimal catalog must not be
replaced with whole-catalog permission checks.

Adversarial self-review checked the actual catalog/tracking/project predicates,
tenant parent joins, current actor and strict policy loading, independent global
rate projection, empty/exhausted compatibility pages, route/session tests and
actual consumer requests. No high/critical defect remains identified in this
read increment. This is not an independent reviewer sign-off or acceptance of
the direct mutations and other still-unintegrated permission surfaces.

## Direct-write integration sequence

Start with the existing `create_task` path, not a second task-creation service.
Its canonical global operation requires current `TaskWriteAll`, irrespective of
the legacy role, and creates no default financial rate. Organization and actor
locks precede insertion; unknown policy, malformed permissions, inactive actors
and revoked grants fail without leaving a task. Keep the legacy policy branch
explicit. The authenticated wrapper must not reject a canonical Member through
`require_manager` before the transactional check can run.

The optional project argument is a composite operation, not an alternative
route around project editing. Its authorization must cover both task creation
and the destination project, with the access-changing organization lock acquired
first. Reconcile its association defaults and billing effects with the existing
editor before enabling this canonical branch. Tests must include rollback when
the project is unavailable and prove that global task authority alone cannot
link a task into an unauthorized project.

For direct edits, replace the ambiguous nullable rate transport with explicit
preserve, clear and set intents. Preserve leaves stored amount and denomination
unchanged; clear and set (including zero) require global billable-rate write
authority even if the requested value happens to match storage. Ordinary task
write authority remains necessary. A protected value must not be returned by
the mutation merely because the user may edit the task name. Project-specific
overrides and recorded entries are not rewritten by default edits. Reauthorize
the original intent on retry; current equality is not proof of permission.

Activity commands require the separately documented timer and project-link
effects above. Do not mark this integration complete with global creation alone:
edits, lifecycle, association effects, real-session tests, catalog controls and
the final transition gates remain required by T230.

### Creation implementation boundary

The direct creation helper now checks policy and current task authority after
taking the organization gate and active actor lock. Optional project creation
uses the access-changing gate from the outset and the editor's current
`ProjectWriteAll` or `ProjectWriteManaged` plus designation predicate. It then
uses the existing link validator, so archived/missing/foreign projects or clients
roll back both the task and link. This does not authorize the separately retained
existing-task link endpoint, which remains part of T230.

New tasks have no default financial rate, and their new project links have no
rate override. In canonical nonbillable projects the link is nonbillable while
the explicitly supplied global catalog default remains unchanged. No existing
project association, rate or historical entry is rewritten. Policy zero retains
its prior billing/default behavior. No migration or activation is introduced.

Eleven production-helper creation tests passed in `4964`, covering combined
all/managed grants, unavailable actor state, destination rollback, nonbillable
links, unknown policy and grant/designation revocation across a real lock wait.
The same run exited 0 with all 134 project tests, 29 compatibility tests and the
registered-session matrix, including creation, foreign destinations, revocation
and inactive cookies. All-target SQLx preparation, offline test compilation and
strict native/WASM lint also passed on this snapshot. Scoped adversarial
self-review found no additional defect in this creation increment; it is not an
independent sign-off or full-feature acceptance. Browser task management,
requester-bound canonical controls and remaining writes stay open. No UI/CSS
changed in this increment, and no new browser run or full Nix gate is claimed.

### Edit gaps at the creation baseline

At `8dd61d4`, `update_task_record` had no actor argument, locked the task before any
organization gate, and treated `None` as a rate reset. Its no-op comparison could not
distinguish preserving a hidden rate from explicitly writing an equal value.
`update_task` has no current UI caller; the legacy task section only creates and
lists tasks. Replace that endpoint's ambiguous rate transport before wiring the
canonical editor, and adapt its existing mutation tests rather than maintaining
a second unchecked mutation helper.

Reuse the pure `RateEdit` authorization rules through a serializable explicit
transport. Preserve must retain both amount and stored currency, including an
unknown legacy denomination. Set must validate nonnegative integer minor units
and the current denomination under the organization lock; an explicitly supplied
stale currency cannot silently relabel an amount. Clear removes amount and
denomination. Clear/set require global rate-write permission even for absent,
zero or equal stored values. Task authority and active-actor/policy checks remain
independent, and the response must omit a rate without current all-rate read.

Required regressions include a name-only editor with hidden rates, managed-only
rate grants, report-only grants, global rate grants without task write, revoked
grants after lock waits, unknown denomination preservation, explicit equal/zero
writes, foreign tasks, and unchanged project overrides/time history. Preserve
the existing no-op/row-version and serialized activation/edit tests. A passing
creation test is not evidence for any of these still-unimplemented edit cases.

### Direct-edit working implementation

`TaskRateEdit` uses required tagged preserve/clear/set input; set carries integer
minor units and an explicit currency. `update_task` requires expected-requester
identity and session authentication before the transactional actor/grant checks.
The existing helper now reuses `authorize_task_write`, which selects strict stored
policy and current actor authority under the organization fence before taking
the task lock. No second unchecked mutation helper remains.

The existing pure `RateEdit` authorizer rejects explicit edits without global
rate write, including financial no-ops. Set checks the fresh organization
denomination under the same gate; a concurrent currency change rejects the old
input. SQL preserve retains both stored financial fields without reinterpreting
unknown legacy denomination. The no-op predicate includes currency, so an
explicit authorized denomination change is not swallowed by equal amounts.
Project overrides and recorded entries are not updated.

The helper separates its authorized session projection from the existing service
event payload before commit. Withheld rates never become fake zeroes or cleared
service events; the wrapper dispatches only a committed actual change. No-op
requests are reauthorized before comparing storage and emit no event.

The nine registered-session regressions failed as expected on the retained
implementation (`78533`, exit 101). A separate transport test then caught tagged
unit variants accepting ambiguous extra fields. Empty struct variants retain the
same wire format and reject those fields; the assertion was not weakened.
The corrected snapshot (`28245`) passed the two transport tests, 29 API tests,
142 project tests including eight new edit cases, the registered-session matrix,
SQLx preparation, offline all-target compilation and strict native lint. WASM
flagged its unconsumed browser transport; the type now follows the repository's
narrow explained expectation, inactive in server/test builds. The subsequent
WASM check (`36639`) exited 0 without weakening global warnings or changing
runtime behavior. Scoped adversarial self-review found no additional high/critical
issue in this increment; it is not independent review or acceptance of the
canonical catalog UI, lifecycle or full permission policy. This completes T232,
not T230. No new browser or full Nix gate is claimed. The mutation transport is a
deliberate replacement of the ambiguous internal server-function input, not a
new public Harvest-compatible write API. No existing UI called this edit endpoint.

### Lifecycle inventory for the next integration

Rechecked the [task guide](https://support.getharvest.com/hc/en-us/articles/360048181332-Creating-editing-archiving-and-deleting-tasks)
on 2026-10-06. Global archive also archives project assignments while retaining
recorded time. Global restore does not restore those project assignments.
Single-project archive/restore is a project-editor operation. Bulk archive/delete
has a distinct rule: tasks without recorded time are deleted rather than merely
archived. Common tasks and adding to all existing projects are separate commands,
not implicit ordinary edit effects. These details must not be replaced with a
global boolean shortcut. No restricted-account browser verification is claimed.

The current [permissions reference](https://support.getharvest.com/hc/en-us/articles/44171549176077-Permissions)
still distinguishes global task management from project editing and rate editing.
The [timer restriction](https://support.getharvest.com/hc/en-us/articles/360048687651-Why-can-t-I-invoice-approve-or-archive-items-when-a-timer-is-running)
also applies to task archival. Scope any error detail to the viewer's time-read
authority; a warning must not disclose another person's otherwise hidden entry.

Horae currently stores only `tasks.active`; `project_tasks` retains billable/rate
overrides but no independent lifecycle state. `set_task_active_record` therefore
cannot preserve the documented global-versus-project restoration distinction.
Deleting those links to simulate archive would lose configuration and is not an
acceptable substitute. The required retained-link state must be reconciled with
`time_entry_contexts`, canonical `time_entries::commands::choices`, historical
time handling, editor projection/association saves and project revision triggers.
Global archive changes tracking eligibility, so it needs the access-changing
organization gate from the outset and a reviewed project/task lock order before
the link-trigger writes. Current schema/UI do not implement this yet; adding a
grant guard alone does not complete lifecycle acceptance.

The first lifecycle correction reuses the current task-write loader and requires
the authenticated requester on the registered activity endpoint. Acquire the
access-changing organization gate before actor or task locks, including restores
and repeated requests. Canonical archival must reject any same-organization
running entry for this task before mutation; do not filter the guard to entries
the actor can read. Return a generic conflict without entry, person or project
identifiers. Interactive timer starts already hold the conflicting shared gate,
so a committed start is observed before archival and a completed archive is
observed before a subsequent start. This must be tested across real lock waits.

Activity responses follow the same independent global-rate read rule as detail
edits, including no-ops. Existing bounded plugin events retain their internal
payload, are produced only for an actual transition and are dispatched only
after commit. Current policy and actor checks precede no-op detection. Preserve
the policy-zero business behavior until cutover; accepting this boundary does
not accept legacy lifecycle semantics as the canonical end state.

This activity-authority correction is now implemented in the existing endpoint
and transaction helper. Five new helper tests cover strict state and no-op
authorization, event/session projection, history preservation, revocation after
a real wait and both sides of the tracking-writer gate. Registered-session tests
cover allowed/denied roles, protected rates, foreign and changed requester IDs,
anonymous access, real timer start/stop/archive/start denial and repeated restore
responses. Existing task no-op, transition and deletion-race tests remain intact.

The original session regressions failed as expected (`40406`). The first corrected
run passed its source suites but found a missing required column in the new HTTP
fixture (`33965`); the fixture was corrected without relaxing assertions or rules.
Final verification `16237` exited 0: 29 compatibility tests, two rate-transport
tests, 147 project tests, 56 time-entry tests, the full registered-session matrix,
SQLx preparation, offline all-target test compilation and strict native/WASM
lint. Scoped adversarial self-review found no further high/critical issue in
this correction; no independent sign-off, new browser run, full Nix gate or
complete lifecycle acceptance is claimed. No schema or policy activation changed.

### Retained-link integration checklist

Source inspection identifies these coupled changes for the remainder of T233:

- `ProjectTaskInput` and persisted project drafts currently have no link activity
  field. Add an explicit representation with compatible decoding of older saved
  drafts; do not interpret omission as an instruction to restore an archived link.
- `editing::save_tasks` currently deletes an omitted link without time and rejects
  one with time. Keep archive distinct from destructive removal. Retain billable,
  rate, restriction, member and budget settings across archive/restore, including
  values withheld from the editor. Do not let a settings save restore activity as
  an incidental upsert default.
- The project edit revision triggers already cover `project_tasks` updates. Global
  archive must lock linked parent projects in stable order before updating links
  so open editors become stale without a child-to-parent lock inversion. Keep the
  organization access gate first; review imports and existing link helpers too.
- Both tracking sources must require active global task and active project link;
  historical reads and the confirmed owner-only terminal timer recovery must not
  inherit that new-entry predicate.
- Global restore must leave link state unchanged. A project restore must be an
  explicit project-authorized command, never an implied global task mutation or
  a side effect of merely re-selecting a catalog identity. Reconcile its exact
  behavior while the global task remains archived before exposing that control.

The task guide still documents the two restoration levels, but does not settle
that last order-of-restoration edge case. No browser observation is available
from the loaded tools in this pass. These are remaining acceptance requirements,
not a claim that the schema/editor migration is implemented or reviewed.
