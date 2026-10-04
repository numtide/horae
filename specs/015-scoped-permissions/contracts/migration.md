# Permission migration contract

Status: planning draft; migration mappings and runtime cutover are not approved.
Source revision: `d3a4ff3d38669f1ddca50ca6d5a23d5245188283`, checked 2026-10-01.
Owner: US5, FR-014/017, with FR-005/007/010/011/019 dependencies.

This contract defines the evidence and safeguards needed to review a migration.
It does not select replacement profiles, authorize a database reset, or treat a
profile's display name as an access-preserving mapping. No migration was run.

## Verified source facts

T086–T088 add original-requester provenance for newly inserted jobs through the
updated commands, under `permission-state.md`. Migration 0045 leaves historical
NULL unchanged and is tested only on disposable databases. It does not resolve
M08 execution/retry policy, select an author for legacy work or authorize runtime
cutover. The source facts below retain their pre-cutover context.

Read with the broader [access inventory](current-access.md) and the distinct
[Harvest profile evidence](reference-profiles.md). These facts concern current
Horae, not an assertion that its access should be preserved unchanged:

| Source | Migration-relevant fact |
| --- | --- |
| `migrations/0001_init.sql`, `server_fns::{require_user,require_manager,require_admin}` | A person's active flag and Admin/Manager/Member organization role drive current guards; authentication identity must not be rebound during migration. |
| `server_fns/time_entries.rs::list_time_entries` and entry mutations | The timesheet uses the session person, including for managers. A target profile allowing edits to other people's work adds a capability, even if its name sounds narrower. |
| `server_fns/users.rs::list_users`, `server_fns/auth.rs::get_me` | Manager sees directory rates; Member directory rates are redacted. The own-user response is a separate path and cannot be inferred from directory redaction. |
| `server_fns/organization.rs::update_org_branding` | Manager can change invoice branding. The target workspace/account boundary must explicitly classify this existing operation. |
| `server_fns/project_creation.rs::lock_creation_actor` | Project editing accepts a current Manager/Admin under an actor lock. Existing project/billing writes do not require a six-profile capability. |
| `server_fns/project_creation/finalize.rs`, `server_fns/project_creation/editing.rs` | The project manager checkbox stores `ProjectRole::Lead`; the editor also reads legacy `ProjectRole::Admin` as manager. Neither field is an organization profile or a person-management assignment. |
| `migrations/0001_init.sql::assignments` | Assignment rows carry project/person references without their own organization column. Repaired endpoints prevent new cross-organization links; they do not establish that all historical links are valid. |
| `server_fns/approvals.rs::approve_periods` | Current approval records have person/date bounds and an actor/time; their submitted entries transition together. They do not store the new explicit project/date coverage contract. |
| `migrations/0023_durable_jobs.sql`, `jobs.rs`, `jobs/lease.rs` | Existing jobs carry organization/lease provenance, not a persisted initiating person. An unknown requester cannot be reconstructed by choosing a current administrator. |

Paths in this table are relative to `crates/horae/`, with Rust modules under
`src/`. Static inspection establishes these shapes and guards, not a database
audit of the user's installation or executed migration acceptance.

### Schema dependency follow-up

Checked migration SQL at `80e2e42` on 2026-10-02, independently of any live
database. These additional facts refine preservation/preflight obligations;
they do not establish that the installation contains anomalous rows.

- `0001_init.sql` gives `assignments` independent project/user foreign keys and
  `UNIQUE(project_id, user_id)`, but no organization column. A cross-organization
  membership can satisfy those constraints. Ordinary duplicate membership pairs
  cannot coexist under the intact unique constraint.
- `0030_project_creation.sql` makes `project_member_costs`,
  `project_member_budgets` and `project_task_members` reference that membership
  pair with `ON DELETE CASCADE`. Their composite tenant checks do not prove that
  a parent assignment without children is tenant-safe. No migration FK references
  `assignments.id`; preserving the pair by deleting/reinserting it would still
  delete the children. Preserve membership identity, creation time and child
  identities/content while introducing separate management relationships.
- `0031_project_billing.sql::resolve_project_rate` uses the assignment override
  for legacy/person billing. NULL inherits, while zero is a real override;
  permission migration must not replace either with a person's default rate.
- `0034_task_tracking_access.sql::time_entry_contexts`,
  `0035_project_read_access.sql::project_read_access` and
  `0036_task_read_access.sql::task_read_access` embed legacy role or membership
  predicates. The latter two also expose a legacy role-based rate flag. These
  SQL consumers must participate in activation; changing only Rust guards leaves
  old financial access. Preserve narrow own-history identity reads independently
  of new tracking eligibility, management and financial fields.
- `0039_project_editing.sql` invalidates project revisions after assignment or
  dependent-child changes. Rewriting a relationship can lock/update project
  parents and fail on exhausted revisions. The migration must follow the common
  hierarchy and stale-editor contract, not disable triggers to hide changes.
- `0001_init.sql::approvals` stores organization, person and optional approver
  through independent foreign keys, not composite tenant references. Attribution
  and person tenancy require preflight; a valid actor UUID is not proof of the
  same organization. Historical audit rows have still weaker references and
  must not be used to invent missing authority or approval coverage.
- Job migrations `0023`/`0025`/`0029` record leases, claim tokens and account
  generations, not a typed initiating-user field. `0026` and `0027` already bind
  uploads/error chunks to `(job_id, org_id)`. Artifact tenancy is not requester
  provenance, and a worker ID, generation or uploader filename cannot supply it.

## Concrete preservation and preflight cases

The independent count-only reader is refined in
[migration-preflight.md](migration-preflight.md), T117–T119. It inspects source
anomalies/provenance only. No complete preview, mapping, conversion or activation
is authorized by this implementation; clean counts cannot satisfy those gates.

These cases extend the fixtures below and are owned by T007/T019. They are
required test inputs/outcomes, not executed tests or approved role mappings.

| Case | Fixture and required result |
| --- | --- |
| M01 — Parent tenancy | A cross-org assignment without child settings blocks affected activation. Preflight derives tenancy from both resources, never only the project or existence of tenant-safe children. Tenant-facing diagnostics expose no foreign identity or financial payload. |
| M02 — Cascaded settings | A membership with cost override, per-person budget and restricted-task allowance retains membership ID/pair/creation time and every child ID/value when management is separated or removed. A delete/reinsert implementation fails this fixture even if membership counts match. |
| M03 — Financial preservation | Use NULL, zero and positive assignment rates with nonzero fallback defaults under legacy/person billing; verify identical resolved rates, exact amounts and snapshots before/after migration and retry. No repricing follows from designation changes. |
| M04 — Role namespaces | Cross each legacy organization role with lead/admin/freelancer project roles and active/inactive state. Only the explicitly reviewed mapping sets management/grants; project `admin` never supplies organization Administrator identity or inferred managed people. |
| M05 — SQL consumers | Compare tracking contexts, project/task identities, progress and financial payloads across UI/API/export consumers after activation. A legacy Manager label cannot restore denied rates; own-history identity access cannot become fresh tracking or management authority. |
| M06 — Revision/rollback | Race activation with a project editor and membership/child changes; reject stale confirmations. Exercise an exhausted revision and an interrupted transaction: preserve all source rows and business values on rollback, with no partial policy or silent revision reset. |
| M07 — Approval provenance | Foreign person/approver references and missing/ambiguous historical facts prevent unreviewed conversion. Preserve existing attribution as evidence; never assign the current admin or derive historical project coverage from today's assignments. No repair is implicit in preflight. |
| M08 — Jobs and artifacts | Cover queued/running/terminal jobs with unknown requesters and retained tenant-bound uploads/chunks. Require the reviewed provenance policy before execution/retry/publication after cutover; preserve lease/generation fences and artifacts, never infer authority from their tenancy. |

Preflight must be read-only and distinguish a source invariant violation from a
candidate mapping difference or missing reference evidence. Reread/fence its
inputs at activation; a report from an earlier snapshot is not permission to
commit. Anomalies block the affected organization transition without silently
deleting, reparenting, resetting revisions or normalizing financial values.

## Reviewable preview

Before any policy activation, produce a same-organization, administrator-only
preview containing:

- The source policy/schema revision, candidate target policy revision and the
  identities/revisions of all access-affecting records used by the preview.
- Each person's stable identity and active state; old effective capabilities;
  proposed profile, individual adjustments and management assignments; and
  explicit added/removed capabilities, scope and financial-field visibility.
- Separate project membership, project-manager designation and person-management
  relationships. Do not derive managed people from colleagues sharing a project.
- Changes to direct requests, lists/aggregates, exports, compatibility reads and
  queued work. A hidden menu is not an access difference report.
- Historical assignment anomalies, missing requester provenance and approval
  records that cannot yet be translated from established facts. Count and explain
  them without exposing another organization's identities or records.
- The active-administrator count before/after, unreviewed mappings and the exact
  conditions preventing activation. Inactive people remain inactive and need a
  defined policy for any later reactivation.

Never present an unverified operation as unchanged. Distinguish a deliberate
functional permission change from removal of a tenant leak or unauthorized
payload field; unsafe access is not a grandfathered privilege.

## Candidate mappings, not defaults

The [current inventory](current-access.md#migration-differences-requiring-review)
lists Manager candidate deltas. Expand them against the final operation matrix
before asking for approval. In particular:

- `Manager → Project Manager` removes broad finance and unassigned project access
  while adding genuinely delegated time editing where configured.
- `Manager → Executive Manager` adds people administration and time editing; it
  does not preserve every current rate or branding write.
- Accounting and People Admin have distinct responsibilities; neither is a
  rank-equivalent fallback for an old Manager.
- `Admin → Administrator` and `Member → Member` also need comparison. Expanded
  approval behavior, financial redaction and reference-specific read boundaries
  make matching names insufficient evidence of exact equivalence.
- A custom profile may express a reviewed combination; it must not bypass
  administrator status, prerequisite rules or last-administrator protection.

The proposed simplest cutover is one reviewed policy activation per organization,
with no implicit Manager default and no indefinite legacy-profile mode. This is
a planning proposal, not an approved mapping. If a compatibility period is
selected, document its entry/exit conditions, allowed operations and retirement
gate before implementation; dual authorization cannot be an accidental fallback.

## Activation and recovery acceptance

1. Installing schema support alone must not activate new grants. Current access
   remains authoritative until the reviewed cutover's prerequisites pass.
1. Bind confirmation to the actual preview revision. A changed role, active flag,
   assignment, profile or other preview input invalidates that preview rather than
   silently applying a different migration.
1. Reauthorize the initiating administrator and preserve at least one active
   administrator under the same concurrency protocol as access changes. Apply
   the mapping and its attributed audit record atomically; interruption before
   commit must not leave half the organization on a new policy.
1. Retry of a committed activation must not duplicate grants, assignments or the
   audit fact. After an unknown acknowledgement, inspect the recorded outcome
   instead of replaying an unbound privilege update.
1. Enforce the activated policy on subsequent authorization checks, active
   sessions, exports and worker execution. No reader or worker may fall back to
   legacy role rank because its consumer has not been migrated yet.
1. Preserve identity/organization, work dates/minutes, money/currencies, rates,
   invoice snapshots and business history. Permission migration does not reprice
   work, unlock invoices, replay notifications or rebind Harvest/OIDC identity.
1. Approval storage conversion requires its own verified coverage mapping.
   Preserve recorded actor/time and state; do not fabricate historical project
   approval decisions. New empty-cell lock semantics and submitted-entry editing
   need explicit compatibility acceptance under FR-019.
1. Jobs with unknown historical requesters require a reviewed transition policy
   before activation. Do not label them trusted system work, attribute them to the
   owner, or discard their state just to satisfy a new non-null requester field.
1. Reversal after activation is another reviewed access change. Restoring old
   flags unconditionally could regrant privileges revoked after cutover; do not
   advertise that as safe rollback. Failed pre-commit activation retains the old
   policy without resetting business data.

These are contract-level obligations. Transaction locks/revisions, deployment
coordination, job handling and exact persistence belong in the completed data
model once the remaining reference gates are resolved.

## Required verification fixtures

| Case | Required outcome |
| --- | --- |
| All old roles, active/inactive, imported/local identities | Preview enumerates every reviewed access difference; no automatic promotion, reactivation or identity change |
| Overlapping project membership/management and unrelated colleagues | Membership stays distinct from management; no inferred people assignment or duplicate grants |
| Foreign/malformed historical assignment | Non-disclosing diagnostic and blocked affected cutover; no silent deletion, relinking or permission grant |
| Two administrators changing access while a preview is open | Stale confirmation fails; a valid concurrent sequence cannot remove the last administrator |
| Pre-commit failure and lost post-commit acknowledgement | No partial activation, safe outcome discovery and no duplicate audit/grants on retry |
| Pending/running user jobs and retained export artifacts | The chosen provenance transition and current execution/download permissions are enforced |
| Submitted/approved mixed-project periods and invoiced time | Reviewed coverage migration preserves recorded history and independent locks; totals remain exact |
| Repeated import and OIDC linking after migration | Existing custom permissions survive; external role metadata does not grant local authority |
| Reversal after a later permission revocation | No stale snapshot silently restores the revoked access |

No fixture in this document is claimed executed. T007 remains open until the
complete operation matrix, concrete mappings and transition policies are reviewed;
T019 owns implementation and verification after those gates.
