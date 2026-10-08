# Atomic person-profile commands

Local command contract for T056–T058, refining FR-004/010/011/013/015/025/029/030.
T126–T129's session consumers and shared previews follow `permission-editor.md`.
No legacy backfill, policy activation or real-data migration.

## Resolved transitions and evidence

The [permission guide](https://support.getharvest.com/hc/en-us/articles/44171549176077-Permissions),
rechecked 2026-10-03, describes unrestricted Administrators and changing role to
limit access. The retained editor asset in `current-account-investigation.md`
loads all non-Forecast grants on explicit Administrator selection (`Rt`). Custom
selection (`gi`) and ordinary customization (`pe` → `Ze`) use `Ve`, excluding
Administrator. The Administrator's ordinary panel is hidden (`N`/`Ns`); this is
source evidence of draft intent, not an observed Administrator toggle or verified
Harvest backend persistence. The following command rules implement that intent:

| Final explicit intent | Grants | Administrative identity / provenance |
| --- | --- | --- |
| Apply Administrator | Must equal the full catalog Administrator selection; reject a reduced payload | True / selected built-in |
| Apply another built-in or custom template, including a reset | Exact confirmed final set, including individual adjustments | False, even for all grants / explicitly selected source and current template revision |
| Edit current person's grants | Exact confirmed final set | Preserve identity if grants are unchanged; otherwise false / preserve explicit source |

An Administrator selection followed by customization must become a visibly
non-admin final proposal, not a reduced Administrator payload. Preview must show
identity loss and last-admin protection. Never infer identity from source labels
on load. Unchanged saves and template deletion preserve independent stored facts.
Grant-only edits do not reset source provenance or reapply a template baseline.

Existing `users::change_user_role` permits inactive targets. Preserve that ability
without changing activation or identity bindings; inactive targets cannot act or
count as the remaining active Administrator. This preserves Horae behavior, not
verified Harvest archived-user parity. New relationship additions have separate
eligibility contracts; this command adds no relationship.

## State and transaction

Add distinct `project_management_assignments` and `person_management_assignments`
with UUID v7 IDs, organization FKs, same-tenant composite references, unique
manager/resource pairs, nonnegative revisions and person self-link rejection.
Reuse existing `(id, org_id)` parent keys. No cascades, legacy-membership mutation,
project revision triggers or automatic backfill. Deletion snapshots preserve IDs.

The command carries request, organization/person revisions, target ID, explicit
intent, final canonical grants and exact confirmed removed relationship IDs.
Reject duplicate/unknown grants, missing prerequisites and duplicate removal IDs;
sort sets for replay comparison, without adding privileges or dropping effects.
Template application also carries the expected template revision. Actor and
organization identities come from the server, not a payload authority field.

1. Begin fresh READ COMMITTED, READ WRITE with bounded transaction-local waits
   under `permission-editor.md`; lock organization UPDATE before any other lock.
   Require policy version 1, active same-tenant actor and strict canonical explicit
   Administrator identity. Retain actor/target user SHARE locks to protect activity;
   no user UPDATE, project row locks or user/project DML.
1. Compare canonical intent with the shared user/request receipt before checking
   current subject/template revisions. Same-key different command kinds conflict;
   exact authenticated replay returns the historical result even after template
   deletion. Revoked actors cannot read a prior result.
1. Check organization revision, then strictly load a selected template before
   the target state under the retained organization gate. The editor's shared
   calculation uses plain canonical-state reads, not leaf UPDATE locks.
   Check target existence/tenancy, person/template revisions and
   resolve the transition above. A changed template is never silently refreshed.
1. On loss of project-read compatibility or the last person-compatible grant,
   recompute outgoing relationship removals. Require exact confirmation, including
   both sets when both are lost. Read-only project grants retain designations.
   Keep-project-access is an explicit final-grant edit before this command, never
   a server flag silently adding privileges. Incoming links, tracking membership
   and business history are untouched. Restoring grants does not restore links.
   Pre-existing incompatible links are not a new loss: unchanged/unrelated edits
   cannot become historical cleanup, even if cleanup IDs are submitted. Their
   migration/preflight handling remains a separate reviewed operation.
1. Demoting an active explicit Administrator requires another active, strictly
   valid explicit Administrator in this organization. Malformed state fails closed;
   grant-equivalent non-admins and inactive admins cannot satisfy this protection.
   Lock remaining active Administrator user rows in ID order with SHARE and recheck
   activity after waits; retain these locks through commit.
1. A real change atomically updates person state, removes only confirmed links,
   advances person/org revisions and inserts typed before/after audit plus receipt.
   Check overflow before writes. Any failure rolls everything back.
1. An exact no-op preserves timestamps/revisions and creates only an unchanged
   outcome receipt (`change: null`), not a fabricated access-change event.

All future relationship writers must take this gate and advance `access_revision`;
it fences relationship previews, including remove/recreate races. No local result
completes T042: legacy activation counts old roles and does not advance the new
revision, so mixed-mode operation remains forbidden. The independent tables have
no parent-writing triggers. Receipt FK user KEY SHARE is compatible with existing
user SHARE locks; the target-state update trigger only updates its own timestamp.

## Executable acceptance

Production-command PostgreSQL tests cover adjusted template application; explicit
same-source reset; equal-grant source/identity changes; all-grant custom non-admin;
unchanged saves; inactive targets; current authority; foreign/missing targets;
strict malformed state; stale person/template/org revisions; exact removal sets;
read-only retention; keep-project-access with person losses; no restoration;
last-admin/concurrent-demotion protection; historical replay across template
deletion; cross-command request conflicts; audit rollback and existing user SHARE
lock compatibility. Schema tests cover tenancy and self-links. These tests do not
replace authenticated endpoint, UI, migration or full-policy acceptance.
