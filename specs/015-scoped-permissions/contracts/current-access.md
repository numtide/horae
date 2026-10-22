# Current authorization inventory

Historical baseline: `a7727f1`, reviewed 2026-09-30; approval and legacy assignment repairs are recorded below, with user-mutation reauthorization added on 2026-10-02. The original rows are not a description of every later delivery. Read them with the dated current-source reconciliation below and the owning contracts; some canonical consumers now exist behind the policy gate. No policy activation or full-feature acceptance follows from their presence.

## Delivery paths

The [operation matrix](operation-matrix.md) expands this current-state inventory
into public-function and delivery-path mappings. It keeps unresolved target
predicates explicit; symbol coverage is not six-profile enforcement evidence.

| Surface / stable symbols | Current boundary | Required cutover concern |
| --- | --- | --- |
| `server_fns::{require_user,require_manager,require_admin}` | Active session user reloaded; Manager means Manager or Admin | Resolve current effective capabilities/scopes; revalidate mutations transactionally |
| `server_fns/time_entries.rs` | Own entries only, including for managers; requested other user is ignored | Explicitly permit managed/all-person operations; keep tracking/task/state constraints |
| `time_entry_contexts`, migration `0034_task_tracking_access.sql` | Active same-org resources and project/task membership; Admin bypasses project membership, not restricted-task grants | Tracking membership is not management authority |
| `project_read_access`, `task_read_access`, migrations 0035/0036 | Manager/Admin organization-wide read and rates; Member membership/history-dependent identity and progress | Split scope, progress, rates and detailed notes; preserve historical identity reads |
| `server_fns/projects.rs` reads | Resource-access views; private notes Admin-only; Member task/assignment rates redacted | Prevent financial values leaking through spend/progress and exports |
| Project/task lifecycle and linking | Manager/Admin organization-wide | Capability plus project/task scope, not role rank |
| `project_creation` create/edit/finalize | Manager can edit all org projects, billing and team; Admin-only private notes/project costs; actor locked/reloaded | Preserve working actor-lock pattern and separate rates/costs/team capabilities |
| `projects::{create_assignment,delete_assignment}` | Legacy Admin-only APIs; repaired to validate both resources' organization and lock/reload the active administrator until commit | Reconcile with editor allowing Manager team changes; assignments still lack org FK |
| `server_fns/clients.rs` | Any active org user reads catalog/address/tax data; Manager mutates and can set defaults | Separate catalog identity from contacts/financial values and client management |
| `server_fns/users.rs` | Active directory for all; explicit ID/name/email/legacy-role/activity payload, no rates/provider subject/org ID/creation time; inactive directory/profile/activation/role writes Admin-only; create/role/activation recheck active same-org actor after organization lock and retain actor lock through commit | Split people management from permission administration and workflow choices per `people-directory.md`; preserve last-admin locking; full policy cutover still pending |
| `get_me`, compatibility `/users/me` | Own full model including rates/costs | Explicit payload redaction; avoid returning identity-provider subject in directory DTOs |
| `auth::get_my_permissions` | Session-owned read-only snapshot; active same-org actor rechecked under organization SHARE then actor SHARE. Legacy mode returns None without exposing staged data; policy 1 requires strict stored state | `contracts/own-permissions.md`; display DTO never authorizes operations. Does not activate policy or replace `get_me` payload handling |
| `server_fns/approvals.rs` | Self submits whole week; repaired Manager/Admin approve/reopen mutations constrain selected IDs to the same organization | Preserve repaired tenant boundary; replace weekly storage and role-only authority through the verified flexible-approval contract |
| `server_fns/reports::{report_time,report_detailed}` | Manager/Admin org reports; aggregate contains financial values, detailed contains notes | Scoped rows, counts/totals and field-level redaction before serialization |
| `server_fns/invoices.rs`, `invoices/editing.rs` | Manager/Admin org-wide read/write/lifecycle; editing locks current actor | Distinguish invoice read/draft/manage and authorize all affected projects |
| `server_fns/organization.rs` | Org name/week-start for active users; branding read/write for Manager | Workspace setting capabilities must not inherit old Manager allowance |
| `reports.rs` download routes | Own Manager/Admin guard for time/invoice exports; project export uses progress view | Cover all formats, download-time revocation and protected derived amounts |
| `harvest/auth.rs`, compatibility handlers | Active cookie user; own time for Member, all-org time for Manager; projects/tasks reuse views | Same effective policy as UI, including count queries and list/detail parity |
| Compatibility `/users` | Manager/Admin, including inactive people | Reconcile UI/API inactive-directory discrepancy |
| `server_fns/importers.rs`, `importers/harvest.rs` | Admin; OAuth callback rechecks after external exchange | Retain admission boundaries and current authority at enqueue/execution/download |
| `jobs::{ClaimedJob,JobLease,execute}` | Org, lease and connection generation; no initiating actor | Persist actor for user jobs; distinguish trusted scheduled service work |
| `jobs/report.rs::download` | Current active Admin and matching job organization | Retain download-time authorization; no stale permission snapshot |
| `cli/imports/transport.rs` | Remote session uses same server endpoints | No separate CLI privilege path |
| Local bootstrap/migrate/seed/user CLI | Operator with DB credentials, not an end-user session | Document operator boundary; do not treat as delegated user jobs |
| Plugin registry/host/database | Read-only constrained service DB and explicit host capabilities; no user scope | Service access is not a user grant; gate widget content by consumer authority |
| `get_plugin_widgets` | Active-user access to globally generated widgets | No per-viewer authorization context currently supplied |
| Budgets/notification delivery/outbox | Role/assignment recipients rechecked at delivery; committed service events | Reconcile recipient capability/scope without changing historical event facts |
| `horae-core::{types,state}` | Legacy roles authorize state transitions | Separate action authorization from valid business-state transitions |

## Verified risks to address

These findings belong to the historical inventory. Later delivered fixes and
remaining obligations must be distinguished using the reconciliation below;
do not reopen completed payload/provenance work solely from this list.

1. `approve_ids` originally updated approvals by ID/state without organization; `reject_submission` originally loaded by ID alone. Tests must invoke actual transaction helpers using two tenants, not duplicate SQL in a test body. The independent tenant repair is tracked separately from the future flexible-approval cutover.
1. Legacy assignment endpoints previously accepted foreign project/person IDs. The independent repair now validates both sides and current administrator authority transactionally. Database assignment relationships still lack organization provenance; schema-level constraints and treatment of pre-existing malformed links remain part of the full migration. The repair does not delete or rewrite those links.
1. Entry-time role checks do not prevent every stale-authority commit. User creation/role/activation now reload and lock the actor after acquiring the organization lock, retaining existing last-admin serialization. Profile revisions, durable access-change audit and other entry-point revocation remain separate cutover work.
1. Jobs cannot reauthorize an initiating actor they never recorded. A service/system job is a different trust boundary, not an implicit administrator.
1. The directory's unused money/provider metadata is removed by T145–T147's typed payload repair. Email/role visibility in the legacy list and progress, own-user and mutation responses still require their independent full-policy projections; hiding controls alone is insufficient.
1. Plugin events after commit are not a durable, attributed permission audit. Global widget output also needs an explicit consumer-access contract.

## Migration differences requiring review

There is no verified drop-in replacement for the old Manager. Identity linking/imports must continue to match existing users without importing privileges.

| Candidate mapping | Differences that the preview must enumerate |
| --- | --- |
| Manager → Project Manager | Remove unassigned/global project and time access, finance/invoices/branding; add actual managed-person time editing absent from current timesheet API |
| Manager → Executive Manager | Add people administration and other-person time editing; remove existing rate editing and branding authority; resolve private project costs explicitly |
| Manager → Accounting | Remove project/task mutation and approvals; preserve only verified financial/client/invoice operations |
| Manager → People Admin | Add people/time management; remove finance/invoices and project/client/task management unless explicitly granted |

These are review inputs, not approved mappings. Preserve current business data and expose grant/revocation differences before activation. Do not infer equivalence from a profile label.

The [migration contract](migration.md), checked against `d3a4ff3` on 2026-10-01,
specifies preview evidence, stale-confirmation and activation safeguards,
historical assignment/job/approval gaps and required fixtures. It does not choose
the mappings or complete the runtime cutover gate.

## Current-source reconciliation (2026-10-13)

Read-only source inspection at `b675d960`; no runtime tests or user-data queries
were executed. The operation matrix now records explicit HTTP registrations,
including grouped downloads and the seven Dioxus POST import routes. This
section updates concrete stale statements, not every historical row's complete
authorization audit.

| Surface | Source evidence at this baseline | Remaining target / verification |
| --- | --- | --- |
| Session identity and Clients | `get_me` has the narrow identity projection; Clients list/detail call the canonical page readers. See `own-permissions.md` and `client-permissions.md`. | Do not describe these as the original full-model readers. Legacy workflow readers and compatibility `/users/me` have separate contracts and must not inherit a pass from `get_me`. |
| Detailed/grouped time exports | The registered CSV handlers call streaming sources; XLSX handlers call `limits::time` and reauthorize captured scope during rendering. Grouped exports are distinct registered routes. | Follow `time-reports.md` and `csv-exports.md` for exact policy, filters, payload and release checks. Their presence is not financial-report or full-picker acceptance. |
| Project exports | CSV uses the project streaming source; XLSX binds requester identity, loads bounded rows and passes captured project/monetary IDs to `render_project_export`. | Verify the current project/progress/financial contract and both actual formats under `project-exports.md`; do not infer unrestricted amounts from ordinary visibility. |
| Invoice exports | All three public handlers still enter through `require_manager`; materialized formats also call `render_manager_export`. | Current-authority machinery does not implement `InvoiceReadManaged/All` or settle mixed/manual invoice scope. OP21/33 still need that contract and equivalent content/metadata tests. |
| New import provenance | API/CSV command insertion records `original_requester_id` through `enqueue_api_in`/`enqueue_csv_in`. Historical NULL remains unrecorded provenance. | T086–T088 storage is distinct from worker execution authority and historical transition policy; do not add another provenance field or claim execution is solved. |
| Import worker execution | `ClaimedJob` carries job/org/payload/claim token, not the requester; `execute` passes a lease to API/CSV importers. Production references to `original_requester_id` persist provenance rather than authorize execution. | Reconcile current requester at execution/commit under FR-007/010, lease and account-generation fencing, and the reviewed historical-job policy. Admission authorization and a valid lease are not authority after revocation. |
| Job controls and error-report download | `jobs::access::begin_admin_access` holds organization/active-Administrator authority for bounded operations; `jobs::report` uses it. | Preserve actual-route checks, report streaming/release semantics and current actor authority; this is not original-requester worker authorization or permission to widen import access. |
| Budget mail | `notifications::preparation::prepare` validates stored claim and current recipient/project facts before returning a prepared message; `budget-email-authority.md` preserves the existing recipient predicate. | Canonical/custom recipient and protected-payload mapping remains an OP37 requirement. Transport after authorized release cannot retract an already released message. |
| Timers, outbox and plugins | Startup runs the existing timer/budget scheduler and separate job, outbox and mail workers. Plugin host functions use explicit capabilities and a separately validated read-only database identity. | Service authority, timer-event payload, widget consumer access and future company-lock execution remain distinct. Do not silently convert stored service work into an Administrator user grant. |
| Login and CLI | `auth::router` selects development versus OIDC registration; `login_redirect_guard` handles page navigation only. Remote Import/Jobs use session HTTP transport, while local operator commands use their separate deployment boundary. | Preserve branch-specific auth, direct handler checks, identity-linking invariants and operator coordination at cutover; no real authentication settings or operator commands were exercised. |

This closes neither the complete legacy-to-canonical access-difference review
nor the cross-command lock audit. It prevents source-inventory drift from being
mistaken for missing implementation or proof of full enforcement. T006/T007,
T014/T015, T042 and T020 remain open until their respective contracts and actual
acceptance evidence are complete.
