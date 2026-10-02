# Operation-level permission matrix

Baseline: `b7e730c`, inspected 2026-10-02. This maps the existing public server
functions and additional delivery paths to the target grant dimensions and
unresolved predicates. It is the working T006 matrix, **not** a completed parity
matrix or permission to replace runtime guards. No allow/deny row below has newly
passed six-profile backend acceptance.

Evidence: [observed profile defaults and editor prerequisites](reference-profiles.md),
[current-account conflicts C01–C07](current-account-investigation.md),
[expense evidence](expense-permissions-evidence.md), and the
[current access inventory](current-access.md). The recorded Harvest evidence dates
remain those of the linked artifacts; no account was accessed for this mapping.

## Reading the matrix

Grant names are `horae_core::permissions::catalog::Permission` variants. A family
such as `TimeRead{Own,Managed,All}` denotes those three explicit grants, not a new
permission or profile rank. The six built-in/custom selections supply grants;
they do not themselves answer record scope, sensitive-field access or business
state. A displayed grant is not proof of backend enforcement.

- **D/O**: documented capability with observed editor configuration; saved
  non-owner enforcement is not yet observed.
- **H**: existing Horae-specific boundary to preserve/reconcile, not Harvest parity
  evidence (imports, plugin/operator access and authentication).
- **C/U**: conflicting or unverified operation detail; the row cannot be accepted
  by choosing a convenient scope or retaining a broad legacy Manager guard.

Every eventual allowance requires current active same-organization identity,
applicable capability and record coverage, plus independent membership/task/state
constraints. No matching grant/scope denies. Explicit Administrator identity is
separate from possessing all known grants and never bypasses tenant or integrity
rules. Picker/identity payloads are separate from detailed management reads.
Bulk selection and filtered execution authorize the complete actual affected set;
scope union must not duplicate rows, counts, money or minutes.

## Existing server-function entry points

Symbols below are relative to `crates/horae/src/server_fns/`. All public async
functions in those modules, excluding test fixtures, are assigned a row. A row
groups only functions sharing the same policy concern; lifecycle differences are
called out instead of treating a family name as blanket authorization.

| ID | Module and public functions | Target capability / independent boundary | Evidence and acceptance still needed |
| --- | --- | --- | --- |
| OP01 | `auth`: `get_me`, `logout` | Current identity/session; no profile grant needed to end one's session | H. Split safe identity from own rates/costs/provider subject in `get_me`; financial read policy still applies. Logout must not require Administrator or a surviving active account. |
| OP02 | `time_entries`: `list_time_entry_contexts`; `projects`: `list_tracking_projects`, `list_tracking_tasks` | Minimal identities for authorized own tracking; project/task membership and availability, not general project/task management grants | D/O + H. Preserve historical/stopped-timer context behavior; no rates, private notes or unrelated directory payloads. |
| OP03 | `time_entries`: `list_time_entries`, `get_current_timer` | `TimeRead{Own,Managed,All}` for entry rows; current timer is session-owned | D/O. Current list ignores another requested user; managed/all paths must become real scoped operations. Define timer ownership separately, and keep sensitive amounts out without their field permission. |
| OP04 | `time_entries`: `start_timer`, `stop_timer`, `create_time_entry`, `update_time_entry`, `delete_time_entry`, `reschedule_time_entry`, `reorder_untimed_entries` | `TimeWrite{Own,Managed,All}` is the ordinary time-write dimension; timer ownership, tracking eligibility and locks remain separate | D/O for ordinary logging/editing; C/U for complete custom lifecycle boundaries. Verify move source and destination, deletes, terminal own-timer stop after assignment removal, and multi-entry reorder atomically. Submitted-but-unapproved editing must follow FR-019, not today's blanket submitted lock. |
| OP05 | `approvals`: `submit_week` | Own submission workflow, distinct from approval capability | D/O flexible approval; C/U final transition mapping. Replace week-only assumptions with verified selected dates; submitting cannot approve or change independent locks. |
| OP06 | `approvals`: `list_approvals`, `approve_submission`, `approve_submissions` | `TimeApproveManaged` / `TimeApproveAll`; eligible person/project scope, explicit date/client/project filters and self-approval policy | C06 approval visibility is user-resolved by FR-024: current read visibility over every selected time/expense record plus approval authority, otherwise atomic denial without hidden expense disclosure or implicit time-only filtering. No ordinary expense-write prerequisite is inferred. List eligibility, self-approval and coverage algorithms still require explicit predicates. Approve A must leave B unchanged; counts/partial state reflect actual coverage, including empty cells. Current skip/count bulk semantics are not acceptance. |
| OP07 | `approvals`: `reject_submission` | Verified withdrawal workflow: `ApprovalWithdrawManaged` or explicit Administrator authority, applicable filters and independent locks | D/O + C06. Existing endpoint name/reopen-week behavior must not introduce a Harvest rejection transition. Approval-page scoped withdrawal and Day/Week whole-week withdrawal need distinct semantics and authorization. |
| OP08 | `projects`: `list_projects`, `get_project_details`, `list_project_tags` | `ProjectReadManaged` / `ProjectReadAll` for management detail; separate permitted tracking/member progress path | D/O + H. Match tag/client identities to visible projects. Ordinary management does not expose private admin notes, personal time notes, billable rates or costs. |
| OP09 | `projects`: `list_project_spend`, `list_project_budget_progress`, `get_project_fee_balances` | Project visibility plus applicable amount/progress/invoice visibility; no blanket money access from `ProjectRead*` | Apply approved C03/FR-021 billable scope; C04 follows FR-022's explicit cost grants; invoice scope remains open. C02's report-only authorization does not automatically open ordinary project financial payloads. Split hour budgets, permitted aggregate progress, billable amounts, costs and fee/invoice metadata; totals must exclude invisible contributors where required. |
| OP10 | `project_creation`: `load_project_draft`, `save_project_draft`, `discard_project_draft`, `finalize_project_draft` | `ProjectCreateAll`, creator-owned draft and separately authorized form effects | D/O creation; C/U composite form effects. Finalization must independently authorize client/task creation, team/manager assignment, rates/costs/private notes; draft ownership is not authority to commit those effects later. FR-026 resolves delegation with project-edit authority, not automatic creator grants or designations. |
| OP11 | `project_creation`: `load_project_editor`, `save_project_editor` | `ProjectWriteManaged` / `ProjectWriteAll`, plus each independently protected form field/relationship | D/O ordinary edits; C03 uses FR-021's approved project-owned rate scope, C04 follows FR-022's cost grants. FR-025/026 distinguish retention from project-editor delegation to compatible targets. Authorize loaded payload as well as save. Project editing alone cannot modify rates, promote people globally or change participants' general rate defaults. |
| OP12 | `project_creation`: `project_creation_selection`, `project_creation_client`, `project_creation_options` | Bounded identities needed for an authorized creation/edit workflow; separate rate/cost visibility | D/O + C/U exact picker payload. No requirement to grant whole-directory management merely to select eligible team members; no unrestricted catalogs/financial fields through search or selected-ID resolution. |
| OP13 | `projects`: `set_project_active`, `set_projects_active` | Project lifecycle authority in applicable managed/all scope | C/U exact lifecycle predicate within `ProjectWrite*`. Authorization and state checks cover all selected IDs, reactivation and lost assignment consequences; no unreported partial action. |
| OP14 | `projects`: `list_tasks`, `list_project_tasks` | `TaskReadAll` for the management catalog; project-specific detail additionally follows project/resource access | D/O + C/U management versus picker boundary. Current Member historical reads must be reconciled, not conflated with task management or rate visibility. |
| OP15 | `projects`: `create_task`, `update_task`, `set_task_active`, `link_project_task` | `TaskWriteAll` for global task mutations; project task linking and rate/availability changes need their own project/field checks | D/O ordinary task writes; C/U linking, lifecycle and financial changes. Linking a task must not grant unrelated project management or change tracking eligibility outside the authorized set. |
| OP16 | `projects`: `list_assignments`, `create_assignment`, `delete_assignment` | Tracking membership, project-manager designation and person-management are distinct relationships | FR-025 retains existing designations with project read, not editing; confirmed read loss removes designations, not membership/history. FR-026 permits current project editors to add/remove designations: adding needs active same-org targets with compatible existing read grants; removing does not require those grants. Recheck full-set authority and revisions, audit scope changes, never promote globally or disclose target configurations. FR-027 separately reserves person-management relationship writes to Administrators. Membership/target predicates and legacy endpoint/editor reconciliation remain separate gates. Hide rates/costs separately. |
| OP17 | `clients`: `list_clients` | `ClientReadAll` for management detail; minimal client names may be required by an independently authorized workflow | D/O. Catalog identity does not imply contact/address/tax/default-rate exposure; inactive visibility and picker projection need explicit checks. |
| OP18 | `clients`: `create_client`, `create_project_client`, `update_client`, `set_client_active` | `ClientWriteAll` ordinary management; billable defaults and lifecycle retain independent predicates | D/O + C/U lifecycle/default-rate interaction. The inline project-client creator must enforce the same policy as standalone creation. |
| OP19 | `users`: `list_users`, `create_user`, `set_user_active` | `PeopleRead{Managed,All}` / `PeopleWrite{Managed,All}` for verified directory/ordinary person management; activation/invitation/admission and rates are separate | D/O + C/U exact lifecycle. Do not let PeopleWrite select the initial privileged role through today's `create_user(role)` argument. Current Admin-only activation is not evidence for all future person actions. |
| OP20 | `users`: `set_user_role`; future profile/custom-grant/template writes | Explicit Administrator identity, revisions, atomic audit and last-active-administrator protection | D documented + FR-011; C01 deletion is user-resolved: retain grants/scope as person-specific configurations, explain preservation, and remove only future template availability. Permission changes must apply FR-025's retained-read/confirmed-read-loss designation rule in the same transaction, with stale-preview rejection. Other saved template lifecycle rules remain pending. Grant-equivalent custom profiles never satisfy Administrator identity. Person/profile source and template application are explicit commands, not lossy role writes. |
| OP21 | `invoices`: `list_invoices`, `get_invoice`, `get_invoice_editor` | `InvoiceReadManaged` / `InvoiceReadAll` for visible invoices; editor-only backing data requires independently allowed fields | D/O + C/U mixed-project/manual-line scope. Define authorization of the entire invoice, not first project or client alone; no hidden out-of-scope line, customer metadata or total. |
| OP22 | `invoices`: `prepare_invoice`, `generate_invoice` | `InvoiceDraftWriteManaged` permits creation/editing of managed-project drafts in the current public reference; `InvoiceWriteManaged` / `InvoiceWriteAll` cover broader management, subject to actual source scope | D for draft creation, checked 2026-10-02 in the [Invoices category](https://support.getharvest.com/hc/en-us/articles/44171549176077-Permissions); C/U for mixed-project/source/rate/fee scope and saved non-owner enforcement. Read-only preview also discloses billable data; generation rechecks after preview and preserves source locks/exact amounts. Draft creation does not itself grant sending, payment or other status transitions. |
| OP23 | `invoices`: `review_invoice_edit`, `save_invoice_draft` | `InvoiceDraftWriteManaged`, `InvoiceWriteManaged` or `InvoiceWriteAll` as applicable; draft state, whole invoice and field/source scope | D/O + C/U compound preview/save effects. Rate/default changes, adjusted source ranges and fee balance metadata need explicit checks; recorded snapshots are not recomputed by permission changes. |
| OP24 | `invoices`: `update_invoice_status` | Invoice management plus allowed business transition; not draft editing alone | D/O + C/U each status transition/payment mapping. Do not treat marking paid as proof of the full payment-recording feature; no implicit unlock or rebilling. |
| OP25 | `reports`: `report_time`, `report_detailed` | Authorized time rows plus report-specific/financial-field contract; `ReportProfitabilityRead`, `ReportContractorRead`, `ReportInvoicingRead` are distinct product grants | C02 resolved by user (A): the applicable financial report grant authorizes its defined financial projection and matching export without ordinary rate/cost grants. It does not open source/rate APIs, rate history, edits or unrelated report families; ordinary time/rate visibility does not grant financial reports. C03 follows approved FR-021; C04 follows approved FR-022. Counts, groups, filters, notes, totals and exports need equivalent report scope; a client-selected report label cannot widen a generic endpoint's projection. |
| OP26 | `organization`: `get_week_start`, `get_org_name` | Safe same-org application metadata needed by ordinary signed-in people | H. Do not gate the timesheet's week start or workspace display name behind company administration; use narrow DTOs rather than full settings. |
| OP27 | `organization`: `get_org_branding`, `update_org_branding` | `CompanyRead` / `CompanyWrite` is the candidate workspace-setting dimension, with invoice-rendering projection separate | D/O + C/U exact branding mapping. Current Manager write is a migration delta. Do not expose bank details through generic org metadata; all organization-row writers start with the exclusive gate. |
| OP28 | `importers`: `harvest_connect_start`, `harvest_connection_status`, `harvest_change_account`, `harvest_disconnect` | Existing explicit Administrator integration boundary; connection generation and identity migration remain separate from permission grants | H. Preserve current integration, reauthorize after external OAuth work and never treat `BillingWrite`, company settings or imported roles as local admission. |
| OP29 | `importers`: `start_harvest_api_import`, `start_harvest_csv_import`, `get_harvest_import_job`, `list_harvest_import_jobs`, `cancel_harvest_import_job`, `retry_harvest_import_job` | Existing Administrator import/job boundary plus persisted requester and execution/download reauthorization | H. Queue acceptance does not grant future execution. Historical unknown requesters require migration review, not assignment to the current owner. |
| OP30 | `plugins`: `list_plugins`; `reports`: `get_plugin_widgets` | Safe registry metadata versus explicitly bounded service-generated content | H. Existing active-user access to global widgets is not per-viewer authorization. No new plugin integration or generic access to generated sensitive widgets is implied. |

## Other delivery paths

| ID | Surface | Same-policy obligation / remaining evidence |
| --- | --- | --- |
| OP31 | `reports::{export_csv,export_xlsx}` | OP25's full row/field/filter policy at execution and bounded output checks; filenames/counts/error metadata must not reveal excluded records. |
| OP32 | `reports::{export_projects_csv,export_projects_xlsx}` | OP08/OP09 project/progress and financial-field policy, including current member visibility. |
| OP33 | `reports::{export_invoice_csv,export_invoice_xlsx,export_invoice_pdf}` | OP21 whole-invoice visibility plus deliberate branding/line projection; direct IDs and regenerated downloads reauthorize current scope. |
| OP34 | `harvest::router`: `/users/me`, `/users`, `/time_entries`, `/time_entries/{id}`, `/projects`, `/projects/{id}`, `/clients`, `/clients/{id}`, `/tasks`, `/tasks/{id}` | Match OP01/03/08/14/17/19 for equivalent reads. Pagination totals/detail responses share scope. Preserve read-only API; existing `is_admin` serialization needs a reviewed projection, never a custom-profile round trip. |
| OP35 | Harvest OAuth callback, CSV upload, `jobs::report::download`, remote `cli::imports` | OP28/OP29 authority at the actual HTTP boundary; token/session/upload provenance must not substitute for active authorization. Remote CLI uses server policy, not a second DB identity. |
| OP36 | Local bootstrap/migrate/seed/user CLI | Existing privileged operator boundary, distinct attributed principal; access-changing commands participate in revision/lock/last-admin protocol. No new user-facing privilege bypass. |
| OP37 | Import workers, budgets/notifications/outbox, plugin host/database | Requester-based work and bounded system/service work remain separate. Recheck recipients/current payload authority where relevant; no external-role-driven local grants or blanket Administrator service fallback. |
| OP44 | `auth::router`: GET `/auth/login`, `/auth/oidc/start`, `/auth/callback`; POST `/auth/dev-login`, `/auth/logout` | H. Keep sign-in/session termination distinct from privileged operations. Preserve existing-user identity binding, inactive-user denial and session rotation; login must not overwrite local grants. The bypass is registered only in explicit development mode. OIDC callback is distinct from OP35's Harvest connection callback. |
| OP45 | GET `/health`, login redirect middleware and static assets | H. Public liveness/static resources must not include private state. Redirecting page navigation is not authorization; protected SSR/server-function data still follows the same current policy. |

## Catalog domains not implemented by this branch

| ID | Grant family / domain | Required target mapping; not a functioning endpoint |
| --- | --- | --- |
| OP38 | `ExpenseRead{Own,Managed,All}`, `ExpenseWrite{Own,Managed,All}` | Feature 016 ordinary expense rows follow own/person/project/all scope. Categories, receipts/downloads, deletion, marking billed/unbilled, locked corrections and approval interactions remain separate open predicates in the expense evidence. |
| OP39 | `BillableRateRead{Managed,All}`, `BillableRateWrite{Managed,All}`, `CostRateReadAll`, `CostRateWriteAll` | Cross-cutting fields in OP01/09/10–12/15–19/21–25. C03 resolved for Horae by FR-021: managed person for general person rates/history, managed project for project-owned rates/history including person/task overrides, always with matching rate action and resource authority; organization-wide grants cover the corresponding rate action, including global task defaults. No generic union, management-only rate access or cross-project override authority. Project projection may show effective inherited rates, not unrelated personal history. C04 resolved for Horae by FR-022: explicit organization-wide cost read/write, not Administrator identity; retain independent resource constraints and report-only separation. |
| OP40 | `EstimateReadAll`, `EstimateWriteAll` | Approved estimate feature must map list/detail/create/edit/send/accept/decline/delete/download and conversion effects. Grant configuration alone does not settle lifecycle authority or implement estimates. |
| OP41 | Retainers and payment recording | Approved invoice/retainer/payment features must distinguish balances, funding, application/refund, payment records and downloads. Do not map Harvest subscription billing grants to client retainers/payments. |
| OP42 | `SavedReportReadInactive`, `SavedReportWriteInactive` | Deactivated owners' saved reports only, not a blanket permission for all personal reports; feature lifecycle and underlying report data checks still required. |
| OP43 | `BillingRead`, `BillingWrite`; unknown reference IDs 59/60 | Harvest subscription billing is outside self-hosted client invoicing; do not expose these as working Horae grants or invent meanings for unknown IDs. Native apps, Forecast and new integrations remain excluded. |
| OP46 | Approval activity history and CSV/XLSX export | [Documented](https://support.getharvest.com/hc/en-us/articles/34910294705037-Activity-log-Approvals) own / managed project-person / all history for Member / Manager / Administrator. Custom-grant mapping remains open. Distinct from FR-013 permission-change audit: preserve FR-009 attribution without exposing permission snapshots or unrelated event portions, actors, filter values or counts. T012/T014/T015 must cover scoped history, equivalent exports, revocation and mixed-scope events; viewing history does not grant approval/withdrawal. |

## Additional target operations

OP47 and OP48 below are newly inventoried target surfaces, not current endpoints:

| ID | Domain | Required target mapping; not a functioning endpoint |
| --- | --- | --- |
| OP47 | Person-management relationship administration | User-approved FR-027 restricts add/remove/replace to active same-org Administrators, not PeopleWriteAll, People Admin, Executive Manager or custom ordinary grants. This includes changes to one's own managed-person set; restricted-user Harvest enforcement remains unverified. FR-028 requires at least one existing compatible managed-person grant for the receiving manager before new relationships are added; no dormant additions, inferred grants or people-directory prerequisite. Evaluate proposed scope and current eligibility at commit, including new edges in a replacement. Separate reading one's effective scope, relationship writes and global grants. Legacy API full-set replacement and `people_manager` side effects cannot silently redefine Horae grants. Self-assignment and later retention remain open. T012/T013 cover complete-set authorization, organization isolation, revision/revocation, audit and preservation of history; existing project delegation FR-026 does not settle this operation. |
| OP48 | Project duplication and permanent deletion, including bulk delete | Separate domain obligations under full web parity, not feature 010's delivered archive/reactivate increment. The lifecycle guide distinguishes retained archived records from destructive removal; invoices survive deletion while losing project links. Duplication requires source projection and separately authorized creation/fields, not project read/write alone. Exact custom grants, dependent financial/expense/retainer effects and domain acceptance remain open. T006/T009/T014/T015 must bind owning-domain requirements and transactional tests before implementation; no data deletion is authorized by this inventory. |

## Completion rule

T006 remains open until the C/U cells have verified operation predicates or an
explicitly approved product deviation, custom dependencies and approval contracts
are complete, and each resulting allowed/denied case is mapped to a runnable
test. A covered symbol is not an implemented policy. Profile defaults, source
inspection and owner-only success are not non-owner enforcement tests.

When adding/changing a public server function or delivery route, update this map
and the tests for its actual returned/mutated set. Do not merge implementation
with a runtime fallback for an unmapped operation. The full SC-001–009 gates and
reviewed migration remain necessary even if the symbol coverage check is complete.
