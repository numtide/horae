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

### Next edit checks against the retained implementation

The current `update_task_record` has no actor argument, locks the task before any
organization gate, and treats `None` as a rate reset. Its no-op comparison cannot
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
