# Current authorization inventory

Baseline: `a7727f1`, reviewed 2026-09-30; approval and legacy assignment repairs are recorded below, with user-mutation reauthorization added on 2026-10-02. This describes current code, not the target permission policy. New scope evaluation is not yet consumed by the application.

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
| `server_fns/users.rs` | Active directory for all; Member rates redacted; inactive directory/profile/activation/role writes Admin-only; create/role/activation recheck active same-org actor after organization lock and retain actor lock through commit | Split people management from permission administration; preserve last-admin locking; profile revisions/audit still pending |
| `get_me`, compatibility `/users/me` | Own full model including rates/costs | Explicit payload redaction; avoid returning identity-provider subject in directory DTOs |
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

1. `approve_ids` originally updated approvals by ID/state without organization; `reject_submission` originally loaded by ID alone. Tests must invoke actual transaction helpers using two tenants, not duplicate SQL in a test body. The independent tenant repair is tracked separately from the future flexible-approval cutover.
1. Legacy assignment endpoints previously accepted foreign project/person IDs. The independent repair now validates both sides and current administrator authority transactionally. Database assignment relationships still lack organization provenance; schema-level constraints and treatment of pre-existing malformed links remain part of the full migration. The repair does not delete or rewrite those links.
1. Entry-time role checks do not prevent every stale-authority commit. User creation/role/activation now reload and lock the actor after acquiring the organization lock, retaining existing last-admin serialization. Profile revisions, durable access-change audit and other entry-point revocation remain separate cutover work.
1. Jobs cannot reauthorize an initiating actor they never recorded. A service/system job is a different trust boundary, not an implicit administrator.
1. Progress and directory DTOs can expose monetary values or identity-provider metadata; hiding controls alone is insufficient.
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
