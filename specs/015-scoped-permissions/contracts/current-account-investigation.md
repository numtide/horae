# Current-account permission investigation

Started 2026-10-02 for PR #212. Scope: gather the maximum evidence available with
the existing Harvest account. This is a research register, not the completed
operation-level authorization contract required by FR-002/020.

## Evidence rules and safety boundary

- Separate current browser observations, historical observations, delivered
  client configuration, public documentation and unverified server behavior.
  A visible control or a permission label is not a passing enforcement test.
- Use normal owner-visible navigation and read-only inspection of delivered
  configuration. Do not enable disabled controls, impersonate another person,
  probe guessed private endpoints, or treat owner success as non-owner evidence.
- Preserve other tabs. Keep raw account snapshots in the main checkout's ignored
  `.scratch/playwright-windows/`; commit only redacted findings and evidence paths.
- Do not buy seats, invite people, save profiles, modify assignments, submit or
  approve work, send email, alter subscriptions/modules, or change business data.
  A form labelled Save is not a read-only probe, even if its values look unchanged.
- A prerequisite dependency visible in a client asset establishes an editor rule,
  not the backend rule. Do not ship third-party implementation code in Horae.

## Read-only probe inventory

The initial connection checkpoint was superseded by successful read-only browser
inspection on 2026-10-02. Outcomes below distinguish inspected surfaces from
access limitations; none constitutes non-owner authorization acceptance.

| ID | Surface and evidence to capture | What the current owner can establish | What remains outside that observation |
| --- | --- | --- | --- |
| R01 | Team, active/archived people and available admission navigation | Current number/type of available test actors and any displayed seat gate | Signing in as another profile; purchasing capacity |
| R02 | Owner Permissions page, current public asset URL and configuration shape | Owner restriction, six profile choices, actual catalog/defaults and unknown IDs | Saved profile enforcement or permission revocation |
| R03 | Current permission catalog, including labels and keys | Exact time/expense/project/person/rate/invoice/estimate/report/withdrawal grants | Which lifecycle operations each grant authorizes on the server |
| R04 | Delivered editor prerequisite and classification rules | Required dependencies, floor, tie order and presentation of differences | Persisted prerequisite validation and backend classification |
| R05 | Owner-visible template controls and public editor lifecycle | Available create/apply/delete/rename controls and request shapes in delivered source | Save/delete persistence, uniqueness collisions and assignee propagation |
| R06 | Assigned projects and project editor, without toggling controls | Tracking membership, manager designation and rate controls are distinct | Non-owner assignment authority, promotion confirmation and concurrent revocation |
| R07 | Assigned people page, without changing relationships | Owner behavior and available controls/help | Who may manage another person's assignments under custom grants |
| R08 | Person Rates and project billing settings, read-only | Exact labels/fields for default, historical and project-specific rates | Managed-person versus managed-project rate access and financial redaction |
| R09 | Approval page, filters, unsubmitted/pending/approved views and lock banner | Enabled approval experience, displayed selection/date/status controls, company cutoff and empty states | Approval, overlap splitting, empty-cell/new-project locks, withdrawal persistence |
| R10 | Preferences/Modules, read-only | Availability, self-approval, deadline, company timezone, auto-lock and auto-submit configuration | Effects of changing modules/settings; custom-profile approval eligibility |
| R11 | Expenses and detailed expense report | Owner-visible lifecycle/download/billing actions and current lock explanations | Non-owner receipt/category/billing/correction authority |
| R12 | Invoice, estimate and retainer navigation/forms, without saving | Owner-visible lifecycle/payment actions, module restrictions and distinct entry points | Managed invoice draft versus finalization, mixed-scope invoices, non-owner retainer authority |
| R13 | Reports and saved reports navigation | Available report families and any owner-visible inactive-owner distinctions | Restricted rates, derived totals, download scope and inactive-owner authorization |
| R14 | Settings and own profile help/navigation | Owner versus ordinary-profile explanations and account-only controls | Non-owner profile/permission visibility, identity-provider policy and ownership transfer |
| R15 | Existing activity/history, if already available | Historical actor/date/scope and any explicit before/after states | Missing events, inferred withdrawal states or newly manufactured history |

## Fresh documentation findings

### D01 — Assignment read, assignment write and profile administration differ

The [people-assignment article](https://support.getharvest.com/hc/en-us/articles/4422314817677-Making-people-assignments-for-Managers)
reserves editing managed-person relationships to administrators. The
[teammates API](https://help.getharvest.com/api-v2/users-api/users/teammates/)
also documents administrator-only access, a Manager subject requirement and
replacement of the complete assignment set on update. Neither source explains
the six-profile/custom-grant mapping. Keep read versus write and relationship
replacement as separate cases in FR-005; do not infer a web grant from this API.

### D02 — Profile visibility and assignment promotion need explicit resolution

The [person-profile article](https://support.getharvest.com/hc/en-us/articles/360048687291-Person-profiles)
describes non-admin access to their own or managed people's permissions and an
explicit confirmation before promoting a Member through a project-manager
assignment. It also describes immediately persisted manager-checkbox changes.
Therefore R06 must not toggle a checkbox as a supposedly harmless preview.
The [new permissions reference](https://support.getharvest.com/hc/en-us/articles/44171549176077-Permissions)
instead says role visibility/change is administrator-only. Treat this as a
reference conflict, not evidence to remove Horae's required own-access explanation
(FR-012) or to grant non-admin access to another person's configuration.

### D03 — Retainer funding is not an ordinary invoice operation

The [retainer guide](https://support.getharvest.com/hc/en-us/articles/360048180992-How-to-create-and-draw-from-a-retainer)
documents administrator-only creation/editing. It separates adding funds from
drawing funds through eligible work invoices; even saving an unchanged eligible
invoice may apply newly available funds. R12 therefore inspects forms only.
The [legacy Manager exclusions](https://support.getharvest.com/hc/en-us/articles/30704133376269-What-can-people-with-Manager-permissions-never-access-in-Harvest)
also distinguish retainer administration from drawing on one. These descriptions
do not establish the newer Accounting/Executive Manager or custom invoice-grant
boundaries. Keep retainer create/fund/draw/archive/delete and payment recording
as separate operation-matrix rows, not one blanket invoice-write grant.

### D04 — Public API cannot substitute for custom-template interaction

The [Users API](https://help.getharvest.com/api-v2/users-api/users/users/#custom-profiles)
explicitly does not offer custom-profile creation or editing. Its documented
profile application behavior supplements the editor evidence but is not an
alternative way to test template persistence with an immutable owner. A failed
or refused owner mutation would not establish normal non-owner behavior either.

### D05 — Company cutoff is distinct from scoped approval coverage

[Timesheet Lock](https://support.getharvest.com/hc/en-us/articles/46563687732877-Understanding-Timesheet-Lock)
documents a single company cutoff shared by manual and scheduled locking.
Automatic execution cannot move it backward; a manual change can, without
cancelling the schedule. Manual dates cannot exceed today in company timezone.
Scheduled modes follow a deadline, the previous week/month or a 1–90-day window.
Approval-module removal stops scheduling without clearing the existing cutoff.
Auto-submit accompanies auto-lock; affected running timers are stopped.

The [lock Q&A](https://support.getharvest.com/hc/en-us/articles/46563806657421-Timesheet-auto-lock-and-manual-lock-Q-A)
reserves cutoff changes to administrators with company-write authority. Locking
does not itself approve or invoice entries; lock-triggered submission does not
send manager notifications. A submission deadline alone is not an editing lock.
The guide's blanket editing prohibition and the Q&A's administrator exception
leave privileged correction unresolved. Do not infer custom-grant behavior.

FR-019 already requires independent administrative locks. Record this newly
documented mechanism separately from scoped approval, invoice, archive and
project-period restrictions. In particular, clearing the company cutoff must
not be interpreted as withdrawing all other kinds of protection. R09/R10/R15
will check rollout and existing evidence without applying or removing a lock.
This discovery does not authorize implementing a scheduler in this research PR.

Direct opening of these two sources failed during this session; following their
links from the [general approval article](https://support.getharvest.com/hc/en-us/articles/360048181832-Submitting-and-approving-timesheets)
returned their content successfully. The same general article mixes older
whole-week statements with newer features; it cannot override the current
flexible-approval contract.

### D06 — Approval unknowns remain concrete, not discretionary simplifications

A fresh reading of [flexible approval](https://support.getharvest.com/hc/en-us/articles/39974542812429-Flexible-timesheet-approval)
still does not settle post-withdrawal submission state, future project coverage
after whole-submission approval, or overlapping-interval removal. It describes
force submission and combined time/expense workflows without supplying the
arbitrary-custom-grant predicate. Its current text does explicitly describe
configurable Project Manager self-approval, disabled by default. Keep that as
documentation evidence until the account preference and enforcement are tested.

The read-only owner investigation may establish available controls or existing
history, not every transition. A second actor and authorized disposable mutation
fixtures remain necessary where no existing evidence distinguishes the outcomes.

## Fresh account evidence — 2026-10-02

Connection succeeded after the user approved Chrome. All filenames in this
section are relative to the main checkout's ignored
`.scratch/playwright-windows/`; prefix `permissions-20261002-` is abbreviated
below. Captures containing account identity are not committed. Ordinary page
loads can emit telemetry; no business mutation, form save or permission change
was requested. Existing fixtures were inspected, not created in this iteration.

| Probe | Outcome and local evidence | Remaining limit |
| --- | --- | --- |
| R01 | `team.yml`, `archived-people.yml`: one active owner, no archived people; invitation requests a second purchased seat; owner Archive/Delete disabled | No alternative actor in this account |
| R02 | `owner-permissions.yml`, `owner-profile.yml`: six disabled profile choices, immutable owner, save disabled; descriptive roles are distinct | No editable permission panel or persisted profile test |
| R03 | Fresh delivered configuration: 50 catalog grants in 11 categories, six profile defaults; exact keys below | Catalog labels are not lifecycle enforcement |
| R04 | Current delivered public editor inspected without executing downloaded code; transitive prerequisites, Member floor and classification revalidated | Persisted validation, arbitrary custom combinations untested |
| R05 | Source exposes save-as-new, apply, deletion and permission-loss preview; no verified rename/update flow | Owner cannot exercise it; deletion source conflicts with documentation (C01) |
| R06 | `assigned-projects.yml`, `project-editor.yml`: all-project access is distinct from assignment needed to track; project management checkbox is separate from membership and custom rates | No toggles or saves; non-owner assignment/promotion authority unresolved |
| R07 | `assigned-people.yml`: owner explanation grants all-person reporting/approval/editing without explicit assignments; no editable relationship controls shown | No non-owner relationship authority test |
| R08 | `rates-settled.yml`, `project-editor.yml`: no owner rate history; new-rate controls; project/person/task/no billable-rate modes, custom person-project cost control; project billing currency distinct from account-currency costs | No financial values changed; managed scope unresolved (C03/C04) |
| R09 | `approvals-settled.yml`, `approvals-unsubmitted.yml`: pending view empty, unsubmitted owner row available; date, status, grouping and project/client/person filters; company-lock banner and force-submit/reminder controls | No approval, withdrawal, submission, reminder or lock mutation; no existing transition history |
| R10 | `preferences.yml`, `modules.yml`: manager self-approval off; timezone configured; deadline unset, auto-lock/submit disabled; notes visibility admin-only with managed-project alternative | Settings unchanged; deadline gating conflict (C05); estimates off and activity log plan-restricted |
| R11 | `expenses.yml`, `expense-editor.yml`, `expense-categories.yml`: receipt download/edit/delete-receipt, billable/reimbursement and submission controls; category deletion disabled when expenses use it | Editor cancelled unchanged; no category, receipt, billing or approval write |
| R12 | `invoices.yml`, `invoice-detail.yml`, `invoice-actions.yml`, `retainers.yml`, `retainer-form.yml`: draft invoice blocks payment until sent/marked sent; duplication/recurrence/preview/export/delete menu; empty retainers list and client/project creation form | No invoice/retainer created or saved; no funded fixture; estimates module disabled, not enabled for research |
| R13 | `reports.yml`, `saved-reports.yml`: report-family navigation, own/shared saved-report tabs and three existing own reports | No inactive-owner fixture; grants and returned financial fields need separate testing (C02) |
| R14 | `owner-profile.yml`, `preferences.yml`: own email managed through Harvest ID; ownership and private-note configuration controls | No identity, ownership or preference change; non-owner access unresolved |
| R15 | `activity.yml`, `modules.yml`: activity history requires an unavailable plan/module | Existing permission/approval transitions cannot be reconstructed here; no upgrade |

R01's owner action-menu evidence is the automatic snapshot
`output/page-2026-10-02T08-47-53-388Z.yml` (Archive/Delete disabled); the team
overview alone does not contain the expanded menu. R11 additionally has
`expense-report.yml` and `expense-report-results.yml`: read-only report execution
shows billable/invoiced filters, grouping, row selection, receipt links and
Actions/Export controls. No bulk billing action, saved report or export was run.
R09's `approvals-approved-settled.yml` confirms the approved current-week view is
empty and shows a withdrawal control; that control was not activated.
`approval-period-options.yml` exposes Day through Custom/All time periods;
selecting All time produces `approvals-approved-all-time.yml`, also displaying
no matching timesheets. This is an empty owner view, not audit-history evidence.

### Current catalog and editor provenance

`permissions-catalog-20261002.json` retains categories, roles and the disabled
flag only, excluding person-specific requested/direct permissions and tokens.
SHA-256: `419e57fafe966a2580dfab0388f600446b875bff0474b52ba794e918b9722f0b`.

The owner page delivered
[permissions-VNAENDKR.js](https://cache.harvestapp.com/static/people/permissions-VNAENDKR.js),
saved as `permissions-editor-VNAENDKR-20261002.js` (1,009,752 bytes), SHA-256
`0ff000944cad184dfead46ab92e3725d1c1cf4e401e15dd581d7d6535c02e32d`.
The historical `permissions-MQ3TAOLU.js` now returns 404; its earlier observation
remains historical. No third-party implementation is copied into Horae.

Compact catalog notation: each brace expands into independent exact grant keys.
Numbers identify this observed configuration only, not Horae database IDs.

| Category | Exact keys (IDs in corresponding order) |
| --- | --- |
| Time | `timers:read:{own,managed,all}` (44,37,26); `timers:write:{own,managed,all}` (38,47,27); `timers:approve:{managed,all}` (57,58) |
| Expenses | `expenses:read:{own,managed,all}` (24,15,3); `expenses:write:{own,managed,all}` (25,16,4) |
| Projects | `projects:read:{managed,all}` (39,32); `projects:write:{managed,all}` (40,33); `projects:create:all` (19) |
| Clients and tasks | `clients:{read,write}:all` (30,31); `tasks:{read,write}:all` (45,46) |
| People | `users:read:{managed,all}` (17,5); `users:write:{managed,all}` (18,6) |
| Rates | `billable_rates:read:{managed,all}` (20,9); `billable_rates:write:{managed,all}` (41,10); `cost_rates:{read,write}:all` (11,12) |
| Invoices | `invoices:read:{managed,all}` (42,34); `invoices:write:{managed_drafts,managed,all}` (49,43,35) |
| Estimates | `estimates:{read,write}:all` (48,36) |
| Reports | `reports:read:{profitability,contractor,invoicing}` (54,55,56); `saved_reports:{read,write}:inactive` (52,51) |
| Approvals | `approvals:withdraw:managed` (53) |
| Account | `company:{read,write}:own` (7,8); `billing:{read,write}:own` (28,29) |

Source-order profile direct-set sizes: Administrator 34, Executive Manager 26,
Project Manager 16, Accounting 16, People Admin 10, Member 4. These are not
effective-grant counts after dependency closure. Administrator's IDs 59/60 remain
undefined in the catalog. The Member floor is own time/expense read and write
(44,38,24,25). Absence of a separate receipt, retainer, payment, category or manual
lock key does not establish denial or justify granting that operation implicitly.

### Conflicts and discriminating checks

| ID | Fresh evidence, not a selected policy | Contract and next discriminating check |
| --- | --- | --- |
| C01 | `_c` in the delivered editor fetches affected assignees; its failure warning says deletion may downgrade matching users to Member. `Ms` sends DELETE and reloads. Public permissions help instead promises preserved grants. | Resolved for Horae by explicit user decision, 2026-10-02: preserve every assignee's current grants/scope as person-specific configuration, remove future template availability, explain preservation before confirmation and keep revocation separate. Reference conflict remains unverified; no Harvest persistence claim. Test ordinary and individually adjusted assignees locally. |
| C02 | Report-category hint says report access exposes displayed underlying data; client prerequisite function `aJ` does not add cross-resource time/rate grants for report reads. | Resolved for Horae by explicit user decision, 2026-10-02 (A): report permission authorizes its defined financial fields/amounts and corresponding exports within report scope, without ordinary rate/cost grants. Direct source/rate access, rate history, editing and unrelated reports retain separate authorization. Test report-only, rate-only, neither, revocation and tenant boundaries locally. Restricted-user Harvest payload/export enforcement remains unverified. |
| C03 | Exact managed billable keys still label scope as managed people, whereas public help describes projects. | Resolved for Horae by user decision, 2026-10-02 (A), FR-021: person management for general person rates, project management for project-owned rates/person/task overrides, always with the corresponding financial permission. No generic person-or-project union or management-only financial access. Execute the local cross-product in `rate-scope-evidence.md`; Harvest enforcement remains unverified. |
| C04 | Person Rates page still describes cost visibility as administrator-only, but current Accounting/Executive defaults contain cost read. | Resolved for Horae by user decision, 2026-10-02, FR-022: explicit organization-wide cost read/write and new-model defaults; Accounting/Executive read-only, Administrator read/write, other built-ins neither. Custom effective grants control access, not Administrator identity. Validate local payloads/mutations/revocation and preserve FR-008 report separation; restricted-user Harvest enforcement remains unverified. |
| C05 | Current Preferences disables auto-lock/submit until a submission deadline is configured; newer company-lock guidance describes custom schedules independent of deadlines. | Deadline dependency resolved from current dedicated guides and the refreshed general approval guide: use the newer documented modes under the existing parity mandate. FR-019/023 and `company-locks.md` define target and local verification; retained owner controls are rollout evidence, not a universal prerequisite. No account setting was changed. |
| C06 | `aJ` gives approve-managed its managed/own time reads and approve-all its all read plus managed approval; it does not add time-write or expense grants. Withdrawal has no such edge, despite a hint recommending time visibility. | Approval visibility resolved for Horae by user decision, 2026-10-02 (FR-024): approval authority plus visibility of every selected time/expense record; otherwise deny atomically, never silently approve only time. No catalog dependency or ordinary expense-write grant added. Harvest enforcement, withdrawal and other lifecycle predicates remain unverified/open; see `approval-visibility.md`. |
| C07 | Permission-loss preview uses POST; `a3` warns about lost project-manager assignments. Its keep-access option adds managed-project read and write. | User-approved FR-025 retains with project read; confirmed read loss removes designations atomically, preserving membership/history. FR-026 permits project-editor delegation to compatible targets without global grant changes. Person-management delegation, creation and explicit keep-access behavior remain open. Harvest enforcement and preview POST effects remain unverified. |

Source inspection also distinguishes initial template/profile selection (`dt`,
built-in wins an equal-size tie) from non-admin automatic best-fit classification
(`Ve`/`DL`). A chosen template is normalized with the Member floor. No observed
source establishes template rename propagation or successful server persistence.

Read-only owner DOM inspection (`deletion-dom.json`) additionally finds generic
permission-reset copy in `.js-custom-profile-deleted-alert`, with `hidden=true`
and computed `display=none`; the deletion dialog is absent. This supports the
existence of contradictory delivered copy, not a deletion event or actual reset.
No hidden element was displayed or enabled to bypass the immutable-owner view.

Project editor help additionally exposes independent private-note visibility and
project-report visibility choices. These remain feature 004/015 cross-surface
checks, not justification to give all tracking members private notes or rates.

### C06/C07 focused follow-up — 2026-10-02

Independent read-only review rechecked the public approval guides, Users API
and retained editor source. C06 remains an authority question, not a missing
ordinary edit prerequisite: custom approval adds time reads but no expense
grants, although the documented workflow handles time and expenses together.
The user accepted requiring visibility of the entire selected time/expense set
for approval, otherwise denying it atomically (FR-024). The final question and
answer concern approval only: withdrawal remains distinct and unresolved. This
does not add expense grants to the editor dependency graph or prove Harvest's
restricted-user enforcement. See [the approved contract](approval-visibility.md)
for acceptance cases and the remaining boundaries.

For C07, the [Users API](https://help.getharvest.com/api-v2/users-api/users/users/#custom-profiles)
documents preserving individual permissions when repeating the current profile,
replacement when switching, and loss of project-manager designations when
project access is lost. It does not say whether read without write preserves
designation. Horae's threshold is now selected by FR-025; assignment-driven
privilege changes and Harvest enforcement remain open.

The retained editor delegates loss prediction to its server. Its keep-access
choice starts unchecked and adds managed-project read AND write at confirmation;
it is not mere relationship preservation. Closing that dialog retains draft
changes without final submission. Backend effects of the earlier preview POST
remain untested. Horae must preview permission expansion explicitly, revalidate
current authority/assignments at save and keep cancellation free of persisted
changes under FR-010/011/013; those are local integrity obligations, not observed
Harvest persistence. Nothing here authorizes removing tracking memberships,
historical records or managed-person relations along with manager designation.

### C07 assignment threshold follow-up — 2026-10-02

Rechecked current official sources without changing an account:

- The [Users API](https://help.getharvest.com/api-v2/users-api/users/users/#custom-profiles)
  still defines loss of project access as removing manager designations, but
  does not distinguish read-only from read/write access.
- [Person profiles](https://support.getharvest.com/hc/en-us/articles/360048687291-Person-profiles)
  and [project creation](https://support.getharvest.com/hc/en-us/articles/360048686831-Create-and-duplicate-projects)
  describe an explicit confirmation when a Member becomes a manager. Their
  legacy Manager terminology does not specify the new custom-grant transition.
- The [bulk assignment guide](https://support.getharvest.com/hc/en-us/articles/360048687351-How-do-I-assign-a-person-to-multiple-projects)
  describes role-dependent manager defaults and an exception for existing
  projects with Member-tracked time. Do not apply these legacy defaults to
  arbitrary custom profiles or confuse adding membership with granting powers.
- The [assignment API](https://help.getharvest.com/api-v2/projects-api/projects/user-assignments/)
  separates the manager flag, active assignment, rates and budget. Its old role
  defaults do not resolve the read-only threshold either.

Re-reading the retained editor's `a3` and confirmation handler confirms that the
server supplies the lost-assignment count; the unchecked keep-access choice
adds both managed-project read and write. The client does not reveal the server
predicate. The existing owner-only snapshot offers no discriminating case, so
repeating owner navigation would not settle this question.

Accepted Horae choice A, FR-025: retain an existing manager
designation when the resulting effective grants allow reading that project
(managed or organization-wide project read); editing is independently granted.
Tracking membership or shared progress alone does not satisfy this threshold.
Remove the designation when project read access is lost, with affected projects
previewed and explicitly confirmed. Never restore editing implicitly. Requiring
editing too was rejected: a read-only downgrade retains the designation. This
is an approved local rule, not verified Harvest enforcement or resolution of
every C07 predicate.

Acceptance cases for T012/T013 (planned, not executed):

| Permission change / fixture | Required result |
| --- | --- |
| Remove editing, retain managed-project read | Retain existing designation; deny project edits and unrelated-project reads |
| Retain organization-wide project read without editing | Retain existing designations; no new designations or edit grant |
| Retain read and write | No incidental designation or privilege change |
| Remove all project read, including custom/per-person grants | Preview all affected designations; explicit confirmation atomically saves grants, removals and audit |
| Only tracking membership or shared progress remains | Does not count as management project read; preserve membership and historical work while removing confirmed designations |
| Cancel, direct request without required confirmation, stale preview or revoked actor authority | No partial grant/designation changes or stale-authority commit; refresh the affected-set preview when stale |
| Project-derived scope removed with independent person/organization scope still granted | Recompute access without stale project scope, duplicates, implicit new grants or revoking independent scope |

Eligibility to create a new designation, promotion and the explicit keep-access
action remain separate from retaining an existing designation. This decision
does not waive administrator-only privilege changes or migration review.

### Assignment authority and remaining reference checks — 2026-10-02

The [Member FAQ](https://support.getharvest.com/hc/en-us/articles/360048687031-Members-FAQs-and-troubleshooting)
explicitly describes an Administrator or Manager assigning a project manager,
and an Administrator restoring Member permissions afterwards. Together with the
project-creation guide, this supports separating project delegation from global
profile administration. The newer custom-grant predicate remains unspecified;
legacy Manager wording must not become a blanket new-model grant.

The new-model [permissions guide](https://support.getharvest.com/hc/en-us/articles/44171549176077-Permissions)
reserves profile administration to Administrators. Its current Project Manager
exclusions now name withdrawal, not approval; older search snippets still say
approval. The current page therefore no longer contradicts the captured
approve-managed / no-withdraw default on that particular statement. Its Time
catalog still omits the explicit approval grants seen in the delivered editor,
and it does not resolve arbitrary custom eligibility or self-approval.

Accepted Horae choice A, FR-026: permit someone with current project-edit authority to add
or remove manager designations within that project when the target already has
compatible project-read grants. This is project delegation, not a global profile
change. A read-only manager may retain a designation under FR-025 but cannot
delegate merely because it is retained. If compatible grants are absent, an
Administrator must explicitly change permissions; neither project editing nor
ordinary membership may promote the target. Reserving all manager designation
changes to Administrators was not selected. This is a user-approved requirement,
not a verified Harvest custom-profile predicate or implemented endpoint. Existing
managed-project read grants qualify when the designation supplies project scope;
do not require a pre-existing designation to create one. Removing a designation
does not require the target still to have compatible grants.

Reconcile OP10/11/16/20 and feature 011's existing no-silent-profile-
change requirement. Test actor authority on the actual project, target
eligibility, active same-organization identities, direct/bulk requests and
concurrent revocation. Adding scope can activate the target's already-granted
managed capabilities; it must not add grants or reveal their full permission
configuration to a non-administrator. Authority to manage another person's
person-management relationships remains separate.

Independent template check: current documentation describes editing a person's
grants and saving them as a new reusable role, not updating every existing
assignee. The retained editor likewise exposes save-as-new and delete. This
narrows the evidenced UI workflows but does not establish same-template
reapplication persistence, creation-name equivalence or an in-place rename API.
Keep those distinctions rather than infer propagation from mutable provenance.
No new account probe or state-changing request was made.

The independent withdrawal review followed the flexible-approval guide through
its FAQ, submission-report and activity-log links. Neither the FAQ nor the
submission-report page establishes the post-withdrawal state or overlapping
coverage algorithm; their older whole-week statements do not supersede flexible
approval. Do not repeat these sources as if they settle those gaps.

The [approval activity-log guide](https://support.getharvest.com/hc/en-us/articles/34910294705037-Activity-log-Approvals)
does establish a distinct read surface: own history for Members, own/managed
project/person history for Managers, all for Administrators, with actor/owner/
project/event filters and CSV/XLSX exports. This is operational approval history,
not FR-013's administrator-only permission-change audit. Exact custom-grant
mapping remains unverified, but the two projections must not share blanket
administrator-only access or expose each other's private fields. OP46 tracks
this contract; it does not activate a new report or infer withdrawal authority.

### Person-management delegation follow-up — 2026-10-02

Reopened the current [assignment guide](https://support.getharvest.com/hc/en-us/articles/4422314817677-Making-people-assignments-for-Managers)
and [teammates API](https://help.getharvest.com/api-v2/users-api/users/teammates/).
Both restrict relationship administration to Administrators. The API additionally
adds `people_manager` on the first assignment and removes it when clearing the
set; omitted IDs are unassigned. That legacy access-role side effect must not be
copied into Horae's canonical grants. It is distinct from changing only relationship
scope. This API is evidence, not a proposed Horae mutation surface.

The [current permissions guide](https://support.getharvest.com/hc/en-us/articles/44171549176077-Permissions)
allows People Admin/Executive Manager to invite, archive and manage people, but
does not explicitly state who may redefine another manager's managed-person set.
Its role summaries and legacy API subject label do not establish custom-profile
eligibility. FR-026 decides project delegation, not this relation.

Asked one product question: retain Administrator-only person-management
assignment writes (recommended, matching the explicit relationship docs), or
also allow current organization-wide people writers. No answer yet. Neither
choice would authorize implicit global grant changes; subject eligibility,
self-assignment and relationship retention still need distinct predicates.
OP47 makes this non-implemented operation explicit instead of hiding it under
ordinary person editing or project membership.

### Project lifecycle inventory follow-up — 2026-10-02

The [lifecycle guide](https://support.getharvest.com/hc/en-us/articles/360048686911-Archiving-restoring-and-deleting-projects)
documents archive as retaining work and reporting/invoicing availability while
preventing new time and project edits until restore. Analysis and duplication
remain available subject to their own authority. Deletion is different: linked
time, expenses and retainers are removed, while invoices survive with project
links removed. Single/bulk deletion requires typed confirmation. No deletion,
restore or other account mutation was executed. These are documented behaviors,
not verified custom-grant predicates or authorization to delete existing data.

The [creation guide](https://support.getharvest.com/hc/en-us/articles/360048686831-Create-and-duplicate-projects)
separately permits copying active or archived projects and excludes certain
project/person custom rate histories from that copy. It does not establish
automatic creator management or a create-only custom profile's final access.

Feature 010 deliberately delivered only archive/reactivate, and feature 009
excluded deletion from its increment. Those historical delivery limits are not
global exclusions from the confirmed web-parity goal. OP48 records duplication
and deletion as unimplemented domain/permission obligations, without extending
the completed increment or treating project-write as blanket destructive access.
Full lifecycle ownership and source-retention/financial tests must be completed
before those operations can be implemented; OP13 remains archive/restore only.

## Acceptance boundary

This research increment is complete only when every read-only probe has an
evidence-backed outcome or an exhausted access limitation, new findings are
reconciled with the existing contracts, and an adversarial evidence review finds
no unjustified promotion of observations into enforcement claims. That does not
complete T006, the full spec, implementation, or PR acceptance: non-owner and
state-changing discrimination cases may still require separate reference access.
