# Project read integration

Owner: OP08/09/32/34; FR-006/007/008/010/018/021/022/034.
Baseline inspected at `2497dbe`, 2026-10-06. The baseline and investigation
sections below retain the gaps found before integration; they are not the
current completion checklist. See **Delivery acceptance** for the verified
increment. Invoice, lifecycle and activation contracts remain separate gates.

## Delivery acceptance

T223–T226 cover the implemented ordinary project readers, bound list/detail
consumers, CSV/XLSX and read-only compatibility delivery. Verification includes
current row and financial scope, strict stored-policy validation, revocation
while waiting for authority or transport, registered sessions and actual browser
flows. The reproducible suites and snapshot limits are in `quickstart.md` and
`progress.md`.

The final scoped self-review traced source authorization through payload
materialization and download release. It checked tenant-consistent parents,
tracking versus progress scope, summary versus private breakdown, optional
money versus zero, minimal workflow labels, requester continuity and stale UI
state. No additional high/critical defect was identified in this increment.
This is not an independent reviewer sign-off or a full-feature activation audit.

Still required: canonical creation/lifecycle and remaining membership/task
mutations, fee/invoice authority, cross-command lock inventory, reviewed data
transition and complete feature/Nix acceptance. The legacy policy remains
unchanged in real databases. Passing these read-delivery tests does not make
the draft PR ready to merge or complete the permissions feature.

## Evidence and settled rules

The current [permissions guide](https://support.getharvest.com/hc/en-us/articles/44171549176077-Permissions)
separates managed/all project reads from time, rate and invoice access. A
canonical grant must not be overridden by the user's legacy role. Project
management designations are separate from tracking membership (FR-025/026).

The [Projects overview guide](https://support.getharvest.com/hc/en-us/articles/360048181432-Projects-overview)
documents assigned teammates' access to explicitly shared reports, restricted
to hourly budget information. Losing overview access does not remove the
independent ability to track time to an assigned project. Its legacy role and
cost wording does not override approved FR-021/022.

The [budget guide](https://support.getharvest.com/hc/en-us/articles/360048686811-How-to-set-project-budgets)
limits that shared-member projection to project totals and the member's own
hours; per-person budgets may additionally show their own budget, not another
person's budget. Per-task budgets do not open individual task allowances to
ordinary shared members. Missing allocations and actual zero remain distinct.

The [analysis guide](https://support.getharvest.com/hc/en-us/articles/360048181412-Project-analysis)
distinguishes project summaries, task/team breakdowns and detailed time reports.
Its invoice tab shows the linked project's portion, not an unrestricted invoice.
This does not resolve custom invoice grants or mixed-project invoice authority.
Detailed time notes retain their independent scoped-time contract.

These sources were reopened on 2026-10-06. No interactive Harvest browser tools
are loaded in this session; no new restricted-user browser observation is
claimed. New-model custom combinations absent from these sources remain explicit
gates, not inferred enforcement behavior.

## Delivery baseline at `2497dbe`

Paths are under `crates/horae/src/` unless prefixed with `migrations/`.

| Consumer / symbol | Current dependency and risk | Required integration |
| --- | --- | --- |
| `server_fns/projects::projects_for_viewer` | Both overview and tracking use legacy `project_read_access`; money budgets follow progress rather than financial authority | Separate canonical managed/all and shared-member visibility; tracking identity must not acquire financial/project-report access |
| `fetch_project_details`, `fetch_project_tags` | Legacy view; task-rate currency and private notes use `org_role` | Same project scope as the list; explicit administrative identity for existing private notes; rate-writing affordance is independent |
| `fetch_project_spend`, `models::ProjectSpend` | Every visible project returns non-optional `spent_cents` | Withhold unauthorized derived money, not zero it or merely hide a column; preserve authorized integer-hour totals |
| `server_fns/budgets::{progress_for_viewer,fetch_progress}` | Shared progress grants all configured person/task budget rows and fee consumption; the same query also serves trusted alerts without a viewer | Separate interactive row/field projection from trusted service calculation; no member cross-person budget disclosure |
| `pages/projects::ProjectList` | Separate legacy project, client catalog, tag, spend and budget resources; global `is_manager` actions | Bind requester/policy, source minimal client names from visible projects and use per-project actions; clear stale resources on identity/revocation |
| `pages/projects::ProjectDetailContent` | Actual `/projects/:id` route still uses role-controlled task/team/fee sections and the global user directory | Use scoped details and workflow labels without directory prerequisites; keep task/team writes and fee data independently authorized |
| `reports/limits/project::{projects,authorize_project_rows}` | XLSX materialization and release only recheck legacy project visibility | Capture field authority as well as project IDs; financial revocation must deny release of previously rendered money even if project read remains |
| `reports/streaming/cursor::declare_projects` | CSV cursor captures unrestricted budget columns for visible projects | Equivalent projection and bounded post-capacity reauthorization; retain existing cancellation/backpressure contract |
| `harvest/mod::{list_projects,get_project}` | Legacy visibility, raw budget amounts and separate count/data reads | Equivalent current scope and protected fields for list/detail/count, including direct IDs and revocation |

The current legacy privacy tests intentionally expect `spent_cents` for a
shared member. They are not evidence for the canonical financial contract.
Keep legacy migration differences explicit; do not silently replace those
expectations and call it full cutover acceptance.

## First executable cases

1. A canonical project reader whose legacy role is Member can list and open
   an authorized private project without tracking membership.
1. A canonical own-only member whose legacy role is Manager cannot list/open
   an unrelated private project or obtain its tags.
1. A canonical project reader whose legacy role is Administrator does not get
   rate fields or monetary budgets without the respective current grant.
1. Managed reads require a current project-management designation; legacy
   assignment role, managed-person scope and past time do not substitute for it.
1. Shared-member progress, management detail and historical tracking identity
   remain distinct. Test hours/fees, per-person/per-task allocations, missing
   settings and inactive/foreign resources explicitly.
1. Repeat row/field and financial-only revocation checks through registered
   session routes, CSV/XLSX and compatibility list/detail, not SQL helpers alone.

Use the existing canonical permission loader, rate evaluator and organization
read fence. No new permission catalog, dependency or parallel policy engine.
Unknown policy and malformed canonical state fail closed; no fallback to legacy
roles after canonical authorization fails. Financial-family reports, invoice
balances, client defaults and creation keep their separate gates. Completion
requires the actual consumers and equivalent deliveries, not only these first
reader tests.

## Ordinary project spending

This is OP09's ordinary project projection, not a financial report family.
Reuse the list's current project visibility. Return total tracked integer
minutes without opening detailed personal entries or notes. Project-owned
billable amounts additionally require `BillableRateReadAll`, or
`BillableRateReadManaged` with a current project-management designation, following
FR-021 and OP09. A report-only grant does not substitute for that ordinary-field
authority (FR-008); legacy role labels do not supply it in policy 1.

Represent withheld `spent_cents` as an absent optional field, never a zero.
An authorized project with no tracked entries has zero tracked minutes and,
only with financial authority, zero monetary spending. The project overview
must not manufacture a monetary zero for a missing/withheld response. Preserve
the existing exact rate cascade, rounding and attached-invoice amount precedence;
do not turn this aggregate into permission to open or change an invoice.

Aggregate only tenant-consistent entry/person/task/project/client relationships.
Hold the existing organization/active-actor read fences until materialization,
including empty projects and financial-only revocation. The configured-budget
and trusted alert calculations retain their separate integration obligations.

## Configured-budget follow-through

Reopened the official budget and overview guides on 2026-10-06 while integrating
spending. The shared-hour rules are explicit: project-wide budget and team hours
remain visible; only the viewer's individual person budget may additionally be
disclosed. Individual task budgets and other people's allocations are withheld.
An aggregate must therefore not be computed from only the disclosed detail rows:
doing that would mislabel a member's allowance as the whole project's budget.
Keep summary and permitted breakdown semantics distinct in the next reader/UI
integration, including their period and units.

The budget guide also distinguishes blank allocations from actual zero:
unallocated task/person work does not consume the allocated budget. Current
`configured_row_spend` makes the entire summary unavailable if any row lacks an
allowance and otherwise sums all row consumption. Reconcile this separate
summary gap with the owning budget tests while preserving detailed unallocated
rows and the trusted per-scope alert evaluator; do not hide it by returning only
the current user's rows or by silently replacing missing allowances with zero.

The new-model custom grants continue to follow approved FR-021/022/034, not
legacy role labels in the guides. No interactive restricted-account enforcement
was observed in this research pass.

The interactive DTO now separates `ProjectBudgetOverview` from its permitted
`ProjectBudgetProgress` breakdown. The overview contains the complete summary,
scope, kind, currency and period; its budget/consumption are absent when monetary
authority is withheld. No financial detail rows are then disclosed. The UI must
consume the summary directly, never reconstruct it from the filtered breakdown.
Explicit project readers may see scoped hour allocations; shared members see
only their own person allowance, with no individual task allowance. Legacy
policy keeps its previous visibility until the separately reviewed cutover.

`horae_core::budget::allocated_totals` computes scoped summaries before detail
filtering: missing allowances and their usage are excluded, zero allowances
remain included, and overflow does not wrap or become zero. Project-level
budgets retain their direct total. The internal per-scope alert evaluator keeps
its original allowances/consumption and does not use a recipient-filtered DTO.
The same organization/actor fence spans authorization and materialization of
the complete interactive response. Current implementation still requires its
pending database, delivery and whole-feature acceptance; these are contracts,
not a claim of completed activation.

## Requester-bound consumer integration

The post-export/API inspection still finds these gaps in `pages/projects.rs`:

- `ProjectList` resolves names from `list_clients(true)`, not from the visible
  project projection. Its search/group labels must not require a general client
  directory grant or fetch client rates/addresses merely to obtain a name.
- Project, tag, spend, budget and current-user resources are independent and
  unbound to a requester. Existing `Ready` checks hide retained data during an
  explicit refresh, but do not establish that all responses belong to the same
  session identity. Reset selection, open actions and retained sensitive values
  across requester changes; reject mismatched responses before rendering them.
- `is_manager(&me)` still controls new/edit/bulk actions and detail task/fee
  sections. Canonical action availability must come from the matching current
  action permission and project scope, not legacy labels or mere project read.
  Keep independent fee/invoice authority separate from ordinary project reads.
- `ProjectDetailContent` obtains labels and assignment candidates from
  `list_users(false)`. Reuse the existing scoped project people/selection
  workflow for its own purpose rather than requiring a directory permission.
- A project-ID key remounts detail state between routes, but does not reset it
  when the session changes while staying on the same project.

Reuse existing requester/snapshot conventions in the People, Reports and
project editor flows. The actual overview needs minimal client identity alongside
visible projects and appropriate per-project action availability. Do not expand
the shared tracking DTO or general directory authority to make this page work.
Bind auxiliary reads and mutations to the displayed requester, and verify
account-switch, permission-refresh, loading, error and empty states through the
actual components. Existing tokens, shared control defaults, visual layout and
keyboard behavior remain unchanged unless a specific acceptance failure requires
a scoped correction. This inspection is not browser acceptance or completed UI
implementation.

### Overview identity and edit projection

The overview response binds its authenticated `PermissionRequester` to the
project rows. A refresh supplying a different expected requester is denied,
including an otherwise empty result. This value is a continuity check, not
authority supplied by the client; the server authenticates and reloads current
organization/actor permissions independently. An unavailable actor cannot
establish a successful empty binding.

Each visible project carries only its client's ID, name and active state for
grouping, search and existing active/archived filter labels. No client address,
tax identifier, default rate or unrelated directory row is needed. Canonical
reads retain tenant-consistent client parents. Tracking callers keep their
existing minimal wire shape rather than acquiring this overview projection.

An edit affordance follows `ProjectWriteAll`, or `ProjectWriteManaged` plus the
current management designation, matching the existing editor's authority.
Read-only, shared-member and legacy role labels do not supply canonical edit
authority. Policy zero uses the current stored role, not a captured user object.
These flags never authorize a save or imply archive/restore, deletion, invoice,
creation or rate-writing authority; those operations retain their own checks.
Current authority is held through materialization of rows and affordances.

The list consumer now mounts only after the bound overview resolves. Refreshes
retain the initial requester/policy and remount its dependent resource/selection
state. Tags, spend, budgets and existing lifecycle requests supply that requester;
authorization failures suppress the prior view and offer an access refresh.
Other legacy callers may omit the binding and keep their current authenticated
behavior; this does not prove detail-page integration.

Creation and import affordances are separate from editing. Canonical creation
advertises `ProjectCreateAll`, but the creation workflow still needs its separately
tracked authorization cutover. Canonical import follows explicit Administrator
identity. Legacy lifecycle controls stay policy-zero-only until OP13 is resolved;
their absence is an incomplete canonical integration gate, not accepted final
parity or an inferred denial policy. The existing lifecycle server checks still
require full reconciliation.

Project download links now carry `expected_org_id` and `expected_user_id`, using
the same flat query convention as time-report downloads. Both supplied values
must match the authenticated session before acquiring a download/render slot.
A partial or malformed pair is a bad request; another identity is forbidden,
including empty exports. Links omitting both retain their existing authenticated
behavior. These values never choose the tenant or grant authority: source and
release still perform their existing current row/field checks independently.
CSV and XLSX have the same binding; no buffering, limit, cancellation or
backpressure behavior is changed by it.

Detail consumers, canonical lifecycle/creation enforcement and final component/
browser acceptance remain required. Neither the endpoint nor this in-progress
list integration completes T226 or permits policy activation.

### Project membership read boundary

The detail route's assignment reader must not inherit canonical authority from
the legacy `project_read_access.can_view_team` or `can_view_rates` columns.
Explicit all-project readers, or managed-project readers with a current
designation, can read the retained project's membership rows without a general
people-directory grant. An ordinary member's own membership remains visible;
it does not disclose other members merely because the project shares progress.
Project-assignment roles are retained data, not canonical management authority.
Inactive teammates' existing assignments are not silently removed or hidden.

Assignment override rates are project-owned fields under FR-021: use the
independent all-rate grant or managed-rate grant with current project management.
Do not expose general person defaults, costs, emails or permission configurations.
The organization and active-requester fences must span materialization, including
empty results; unknown policy or missing canonical state must not fall back to
legacy. Filter the project, client and assigned person to the same organization.
The old policy keeps its existing team-read behavior until reviewed cutover.

The official [project analysis guide](https://support.getharvest.com/hc/en-us/articles/360048181412-Project-analysis)
and [permission guide](https://support.getharvest.com/hc/en-us/articles/44171549176077-Permissions)
were reopened on 2026-10-06: project reporting includes team breakdowns;
ordinary Members have own-work access, not other people's data or financial
information. These establish the read boundary alongside FR-005/006/021, not
membership-write, invoice or task-catalog authority. No interactive browser
observation is claimed. The detail consumer still needs workflow-specific
labels and requester binding; this reader change alone does not complete it.

### Bound detail view

Load saved details, current edit affordance and minimal task/team labels in one
requester-bound response under the existing read transaction. A name needed for
this project is not directory access. Team labels come only from the authorized
membership set, including retained inactive people; enabled task labels come
only from tenant-consistent links on the readable project. Neither label set
needs emails, global roles, permission configuration, rates or costs.

Keep the initial requester and policy when refreshing the same project. Do not
mount child resources or show retained content while the response is pending,
denied, for another project, or for another session/policy. A failed refresh
does not erase the initial binding. A successful refresh remounts dependent
state. Use the existing project editor for task/team changes rather than a
second role-based form inside the detail view. An Edit link follows current
project-write scope and the editor reauthorizes independently.

Fee/invoice authority remains separate. Preserve existing policy-zero fee
access with a requester-bound request; canonical fee delivery remains an open
integration gate, not an inferred grant from project read/edit or a claim of
final parity. Test the real routed component for pending/denied responses,
refresh identity/policy/project mismatch, per-project editing and absence of
directory reads. Test the registered endpoint and transaction, not mocks alone.
