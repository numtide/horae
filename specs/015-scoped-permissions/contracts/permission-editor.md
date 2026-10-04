# Authenticated permission editor

T126–T129 refine US1/US4 using the confirmed profile/template command contracts.
FR-002 permits this integration, not activation or replacement of legacy guards.
Only disposable fixtures enable policy 1; UI and full acceptance remain required.

## Authority and transactions

Derive organization/actor from the session. Require exactly policy 1, an active
tenant-local user and strict explicit Administrator identity, not equivalent
grants. Reject client authority fields. Legacy/future policies deny these endpoints.
Use READ COMMITTED, READ WRITE and transaction-local statement/idle limits of at
most 5/10 seconds, preserving stricter inherited limits. Organization SHARE for
reads or UPDATE for commands precedes requester SHARE held through completion.

Profile commands retain target activity with SHARE. Before demoting an active
Administrator, lock other active Administrator user rows in ID order and strictly
reload their states. This protects the command against direct deactivation; it
does not make legacy lifecycle writers safe for activated canonical policy. No
user UPDATE, project-parent lock or business-row mutation is introduced. Existing
actor SHARE locks remain compatible. Release transactions before user interaction.

The shared effect calculation reads templates, canonical person state and outgoing
relationships without leaf UPDATE locks: the held organization gate already
excludes every canonical writer. Saves retain organization UPDATE until all DML
and the receipt commit; previews use SHARE and issue no DML. Direct SQL modifying
canonical state without this gate is not an authorized writer protocol.

## Editor flow

- Load the local target (including inactive people), exact grants, independent
  identity, provenance, person/access revisions and current template IDs, names,
  grants and revisions. Display DTOs are not trusted deserializable authority.
- Preview explicit action/final grants against expected person/access/template
  revisions using save's transition and relationship-effect calculation. Return
  before/after snapshots, changed status and exact outgoing removal IDs, subject
  IDs and revisions. Last-active-Administrator failure is a conflict, not a
  successful executable proposal. Preview writes no state, revision or receipt.
- Save recomputes effects and requires exact confirmed removal sets. Keeping
  project access is an explicit grant edit, never a server flag. Unchanged or
  unrelated edits cannot clean up pre-existing incompatible relationships.
- Template create/delete reuse existing commands. Deletion preview reports
  affected people and grant/identity preservation; access revision fences the set.
  Preview reserves no name/slot. Cancellation sends no mutation.
- Authorize before replay; exact historical replay precedes now-stale subject or
  template checks. Preserve serialized canonical intent and no-op receipts.

## Errors and verification

Missing session: 401; authority/policy denial: 403; local missing/foreign subject:
404\. Stale revisions, changed request intent, confirmation mismatch and last-admin
protection are conflicts. Invalid input differs from malformed storage. Database,
stored-state and receipt decoding errors are logged and sanitized on delivery.

Production PostgreSQL and registered HTTP tests cover preview/save equality, no
preview writes, replay/no-op, forged authority, policy denial, stale effects,
actor/target/survivor deactivation, inherited settings, timeout/cancellation rollback
and sanitized authentication errors. Full enforcement/migration/browser gates stay
open; no new product decision or schema is needed.
