# Feature Increment: Client archive and restore

**Feature Branch**: `fix/client-archive-lifecycle`

**Created**: 2026-10-25

**Status**: Implemented and locally validated under policy 0; exact-head CI pending.
Parent 012 (checklist 11/16) and 015 global readiness remain incomplete.

**Input**: Deliver the approved client lifecycle under the current (policy 0)
permission model. Parent: [feature 012](../../spec.md) FR-016, resolved by
[015 FR-036](../../../015-scoped-permissions/spec.md) and the
[OP13](../../../015-scoped-permissions/contracts/operation-matrix.md) state
prerequisite. Harvest references: [clients guide][clients] ("You can only archive
clients if you archive all of their projects") and [unlocking guide][unlocking]
(restore the client before its project).

## Governance

This increment is governed by this spec, its
[readiness checklist](checklists/requirements.md), and the approved sources it
implements: 015 FR-036 and OP13, and 004 FR-011/014/015/017–021. It does not
satisfy, revalidate or waive the [parent checklist](../../checklists/requirements.md)
(11/16) or 015's global readiness.

The parent's open items do not apply to this scope. FR-015 (contacts) and FR-019
(Pin) are not touched. FR-016 still carries its marker in the parent text, but
its product choice was confirmed in 015 FR-036 and recorded as resolving FR-016
([client-permissions](../../../015-scoped-permissions/contracts/client-permissions.md),
[dependent-spec-reconciliation](../../../015-scoped-permissions/contracts/dependent-spec-reconciliation.md)).
FR-036 supersedes only the cascade in parent US4 scenario 3 and in the
[detail prototype](../../../../design/project/app/12_Client%20Detail.dc.html):
projects are not archived with the client, so there is no affected-project count.
The confirmation itself and the Client Detail entry point still apply and are
delivered here with non-cascading wording. The bulk half of FR-017 stays with the
parent. Parent US4 scenario 4 (history kept, new tracking prohibited) is covered
by LC-008 and the existing archived-client rules.

## Clarifications

### Session 2026-10-25

Settled rules (FR-036, user-confirmed 2026-10-06) are inputs, not questions:
archive needs every linked project archived first; restore changes only the
client; no cascade; history preserved; denials do not disclose projects.

- Q: Which authority applies? → A: Today's active manager/admin role, rechecked
  in the committing transaction with policy version 0 and the organization gate,
  as project status writes already do. Any other policy fails closed. No policy 1
  integration or activation (015 T238 remains open).
- Q: What does an import do with a new, active project whose resolved client is
  archived? → A: Derived from existing contracts, not a new choice: fail that
  record visibly (004 FR-018–FR-021), keep the run going, and report it the same
  way in dry run and commit (FR-014/015). Creating it active breaks FR-036,
  creating it archived breaks the data-model rule that source activity is kept,
  and restoring the client is an implicit client write (OP13). A re-run after the
  client is restored creates it once (FR-011, 005 FR-008). Existing matches stay
  unchanged (FR-017); incoming archived projects import as today.
- Q: Does the import rule apply to a client the same run created archived? → A:
  Yes. The check is on the resolved client's state, whichever way it resolved.
  Harvest cannot normally export this state; when it appears, it is reported like
  the matched case instead of creating an active project under an archived client.
- Q: Do the confirmation and the Client Detail action apply? → A: Yes (coordinator
  review). Archive asks for confirmation on both list and detail, saying that only
  the client changes, never counting or naming projects; Cancel and Escape change
  nothing. Restore is direct: parent US4 scenario 3 and the detail prototype's
  dialog cover archive only, and no approved source asks for a restore
  confirmation.
- Q: Where does focus go after the list refreshes and the row has left the current
  Active/Archived filter? → A: To the same row's control if it is still shown,
  otherwise to the Clients heading. Focus is never moved if the user has already
  moved it elsewhere.

## User Scenarios & Testing *(mandatory)*

### User Story 1 — Archive a finished client (Priority: P1)

A manager archives a client whose projects are all archived. A client that still
has an active project stays active, and the manager is told to archive its
projects first.

**Independent Test**: Clients with zero, only archived, and one or more active
linked projects (including projects the actor cannot see under other policies);
compare every project, time entry, invoice and import record before and after.

**Acceptance Scenarios**:

1. **Given** a client whose linked projects are all archived (or none exist),
   **When** a manager archives it, **Then** only the client becomes archived.
1. **Given** a client with any active linked project, **When** a manager archives
   it, **Then** nothing changes and the message says to archive the client's
   projects first, without naming or counting projects.
1. **Given** an archived client, **When** archive is requested again, **Then** the
   request succeeds as a no-op and reports no new transition.

### User Story 2 — Restore a client, then its projects (Priority: P1)

A manager restores an archived client without changing its projects, and can then
restore those projects individually or in bulk.

**Independent Test**: Restore a client with archived projects; check projects are
untouched. Try single and bulk project restoration before and after.

**Acceptance Scenarios**:

1. **Given** an archived client, **When** a manager restores it, **Then** the
   client is active and every linked project keeps its prior state.
1. **Given** an archived project of an archived client, **When** a manager
   restores the project, **Then** it is refused and asks for the client to be
   restored first. In a bulk request, one such project refuses the whole batch.
1. **Given** the client has been restored, **When** the project is restored,
   **Then** it succeeds as it does today.

### User Story 3 — Lifecycle stays consistent under concurrency and import (Priority: P1)

Archiving can never leave an archived client with an active project, whichever
request commits first.

**Independent Test**: Run archive against project creation, reassignment,
reactivation and import in both orders; run import dry run and commit against an
archived client, restore it, re-run.

**Acceptance Scenarios**:

1. **Given** a concurrent project creation, reassignment or restoration for the
   client, **When** both commit, **Then** either the archive is refused or the
   project write is refused; never both succeed.
1. **Given** an import that would create an active project under an archived
   client, **When** it runs (dry run or commit), **Then** that project record and
   its dependent time entries are reported as failed with a reason, other records
   import, and the client stays archived.
1. **Given** the client is then restored, **When** the import is re-run, **Then**
   the project and its entries are created once.

### User Story 4 — Clear lifecycle controls (Priority: P2)

The Clients list and Client Detail offer Archive or Restore and show the state
Archived. Archive asks for confirmation first; Restore acts directly. For both, a
pending request cannot be repeated, a failure is visible with a way forward, and
focus lands somewhere sensible.

**Independent Test**: Keyboard-only archive/restore from list and detail; Cancel
and Escape; delayed and failed responses; Active and Archived filters; narrow
screens and both themes.

**Acceptance Scenarios**:

1. Active clients offer Archive; archived clients show Archived and offer Restore,
   on the list row and in the detail header.
1. Archive opens a confirmation that says only the client changes, with no project
   count or names. Cancel or Escape closes it and changes nothing. Restore sends
   the request without a dialog.
1. While a request is pending its control (Archive's confirm button, or Restore)
   is disabled and announced busy, and the archive dialog cannot be dismissed.
1. A refused archive keeps the dialog open with the reason and a link to the
   client's projects. A failed restore shows the existing page alert. Retrying is
   safe in both cases.
1. After success the view refreshes. Focus returns to the same control if it is
   still shown, otherwise to the page heading (list) or Back to Clients (detail),
   and is left alone if the user has moved it.

### Edge Cases

- Existing data with an archived client and an active project (allowed before this
  increment): left as is; editing it may keep that client; restoring the client works.
- Already-active project of an archived client: restoring it is an unchanged no-op.
- Actor deactivated, demoted, or policy changed while the request waits; foreign or
  unknown client IDs (not found, no disclosure).
- Client deleted or edited concurrently; repeated and duplicate submissions.
- The database's default transaction isolation is not READ COMMITTED.
- A CSV with several lines for the same refused project: each line is reported with
  its own location and reason under the existing per-row accounting.
- Under the Active or Archived filter, the changed client leaves the list.

## Requirements *(mandatory)*

### Functional Requirements

- **LC-001**: Archiving a client MUST be refused, with no change, while any linked
  project is active, counting all linked projects regardless of who can see them.
  The refusal MUST NOT reveal project identities or counts.
- **LC-002**: Restoring a client MUST change only the client.
- **LC-003**: Both actions MUST recheck, when committing, that the actor is active,
  is a manager or administrator, belongs to the organization and that the
  organization uses permission policy 0. Otherwise they fail closed with no change.
- **LC-004**: Concurrent project creation, reassignment, restoration and import
  MUST NOT let an archived client end up with an active project those writes
  created or restored, whatever the database's default transaction isolation.
- **LC-005**: Restoring a project (single or bulk) MUST require its client to be
  active; a bulk request with any such project MUST change nothing. Unchanged
  projects remain no-ops.
- **LC-006**: An import MUST report a new active project under an archived client,
  and its dependent time entries, as per-record failures with a reason, continue
  the run, agree between dry run and commit, and create them once on a re-run after
  restoration. Incoming archived projects and existing matches are unaffected.
- **LC-007**: Repeated requests MUST be no-ops with no new transition or event.
  Successful transitions keep today's client events.
- **LC-008**: Neither action MUST change projects, time entries, invoices or import
  history (provenance, runs, reports).
- **LC-009**: The Clients list and Client Detail MUST offer Archive/Restore, show
  the state Archived, confirm archive (not restore) with non-cascading wording and
  no project count, keep state on Cancel/Escape, guard pending requests, show
  failures with recovery, and place focus as in US4 for both actions, using
  existing components and tokens.
  No cascade wording, bulk, delete, duplicate, contact, metadata or other detail
  redesign.
- **LC-010**: Verification uses synthetic data on disposable databases only; no
  real Harvest writes, mail or payments.

### Key Entities

- **Client lifecycle**: The existing client `active` flag; no new state, schema or
  audit entity.
- **Linked project**: Any project referencing the client, visible to the actor or not.

## Success Criteria *(mandatory)*

- **LC-SC-001**: In every tested order of archive against creation, reassignment,
  restoration and import, zero archived clients end with a newly active project.
- **LC-SC-002**: Project, time, invoice and import records are identical before and
  after every archive, restore and refused request.
- **LC-SC-003**: Refusals contain no project identity or count; foreign IDs are
  indistinguishable from unknown ones.
- **LC-SC-004**: Import dry run and commit report the same failed records; a re-run
  after restoration creates each once and reports zero duplicates.
- **LC-SC-005**: Archive/Restore work by keyboard alone from list and detail in both
  themes at 320px and 1440px, with no repeated request while pending, zero changes
  after Cancel or Escape, and focus on a live element after every refresh.

## Assumptions

- Policy 0 only; client-write grants under policy 1 stay in 015 T238. Client
  status changes fail closed under any other policy, as project status changes
  already do. After implementation and tests, one sentence in 015
  `client-permissions.md` records that the legacy operation enforces the
  prerequisite (T019); nothing else in 015 changed.
- Single-client controls only; 012's bulk client actions and contacts stay separate.
- Owner-session read-only Harvest observation (2026-10-22, root
  `.scratch/client-lifecycle-next-20261022.md`) corroborates the archive
  restriction; no Harvest record was changed.

[clients]: https://support.getharvest.com/hc/en-us/articles/360048181312-Create-and-edit-clients-and-client-contacts
[unlocking]: https://support.getharvest.com/hc/en-us/articles/4408222060301-Unlocking-time-and-expenses-if-the-project-task-or-person-is-archived
