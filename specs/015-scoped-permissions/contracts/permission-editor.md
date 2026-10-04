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

## Person editor consumer (T018, in progress)

Extend the existing AdminUsers surface without changing its legacy shell or
directory authorization. Show the entry action only after the own-permission
projection returns the supported catalog and explicit Administrator identity.
The dialog independently loads its target through the canonical endpoint; UI
visibility is not authority. Legacy mode exposes no staged editing controls.
Canonical Administrators with a non-admin legacy role still need the separately
gated shell/directory integration before full acceptance.

Load exact grants with strict restoration, not normalization or profile matching.
The initial proposal is an ordinary edit; applying/resetting a built-in or
template is explicit. Custom edits use the existing prerequisite graph and
immutable Member floor. Administrator customization requires choosing another
profile. Any edit invalidates the preview and removal confirmation.

Review sends no mutation. Show added/removed grants, identity changes and both
exact management-loss sets. Confirm losses explicitly. Keep-project-access is
initially unchecked, adds managed project read/write to the draft and requires a
new preview before saving. Back to editing preserves the draft; cancelling the
editor sends no save. Unavailable/failed previews cannot authorize a save.

Disable controls and dismissal while a request is pending. After an uncertain
save response, retain the identical command and request ID and offer retry,
without permitting edits or reporting success. A known rejection requires reload;
authentication/authority/not-found denial also hides the person's form. Distinguish
an unchanged successful receipt from a real change. No request is persisted in
browser storage by this consumer; navigation/reload recovery remains an acceptance
case to resolve before declaring the full editor complete.

### Direction contract

- Thesis: review an actual person's access before changing it, without presenting
  the prototype's obsolete three-role matrix as the six-profile policy.
- Own-world: inherit DESIGN.md and the Workspace handoff; reuse Modal, Checkbox,
  labelled fields, shared permission copy and token utilities. No global CSS.
- Story: choose a profile, inspect individual grants, review exact effects, then
  confirm or return to editing. No fake people, seats, timestamps or audit rows.
- First viewport: protected-focus editor with the person's name, saved source,
  profile selector and permission controls; review/save actions follow the form.
  The shared dialog owns scrolling, inertness and focus restoration.
- Form: local extension of AdminUsers in Operate mode, not a new visual identity
  or a concept-selection exercise. Template create/delete controls remain open.
- Finish: browser, keyboard, viewport/theme and independent visual review remain
  mandatory. Current loss DTOs contain subject IDs, not names; improve this
  presentation before full acceptance instead of inventing labels from unrelated
  or stale directory data. No full T018 completion follows from source/SSR tests.
