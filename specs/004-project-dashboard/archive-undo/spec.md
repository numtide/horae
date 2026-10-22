# Feature specification: undo project archive

**Branch**: `feat/project-archive-undo` | **Created**: 2026-10-22

This bounded extension implements existing H102 / #208 FR-015 from
`../header-increment.md`. It does not regenerate the dashboard specification.
Input: deliver the approved Project Detail archive toast's functioning Undo.

## User scenarios & testing

### US1 — Undo an accidental archive (P1)

An authorized project manager can reactivate the project immediately after
archiving it, without leaving its detail page or confirming twice.

Independent test: archive a disposable project, activate Undo with the keyboard,
then reload; the project is active and its existing data is unchanged.

1. Given a successful archive, the success notice offers Undo and Dismiss.
1. Undo directly starts reactivation; pending feedback prevents duplicate actions.
1. Success shows the persisted active state, removes Undo and leaves focus usable.
1. Dismissing the notice changes no data. A later normal Reactivate still works.

### US2 — Recover without bypassing access (P1)

Independent test: interrupt the Undo response, retry, and verify the resulting
state; separately revoke authority before Undo and confirm no unauthorized write.

1. A transport failure displays actionable feedback and permits a safe retry.
1. A response lost after success can be retried without reversing the status or
   duplicating a status transition.
1. Changed account, organization, role or policy cannot reuse old authority;
   denial suppresses the stale project and success notice, with bound recovery.
1. Navigation to another project cannot carry over its Undo action or feedback.

### Edge cases

- An already-active project remains active when the same Undo request is retried.
- A refresh that loses access removes stale feedback and provides recovery focus.
- A normal Reactivate success does not offer archive as an inverse Undo action.
- Dismissing while Undo is pending does not cancel that request or suppress its
  result: a retryable failure must show the error and Undo again.
- Short/mobile screens and enlarged text keep the action and dismissal reachable.

## Requirements

- UFR-001: Only a successful archive from this detail view offers Undo for that
  project. Dismissal, subsequent status actions, loss of access or leaving the
  project removes the opportunity, except that a pending request's retryable
  failure restores its error and retry; it is not a persistent history feature.
- UFR-002: Undo directly requests the active state under current authority. It
  changes no project content, tracked time, invoices, budget or relationships.
- UFR-003: While Undo is pending, duplicate status submissions are prevented and
  feedback is explicit. A retryable failure keeps a clear, keyboard-usable retry;
  an authorization failure uses the existing bound access recovery.
- UFR-004: Undo, dismissal and resulting focus work at 320/390/768/1440px, both
  themes and 200% text. Existing actionless notices and status confirmation
  dialogs keep their behavior and appearance.
- UFR-005: No new permission is granted or activated; no integration, mail,
  payment, deletion or archived-editor restriction is added.

### Key entities

The existing project status and acting account remain authoritative. The notice
is transient view feedback, not an archive receipt or a new stored entity.

## Success criteria

- USC-001: A completed archive can be undone with one activation of Undo, without
  another confirmation, and remains active after reload.
- USC-002: Success, retry, lost response and denial cases preserve non-status
  project data; denial performs no unauthorized write.
- USC-003: All listed screen/theme/text and keyboard checks pass; both independent
  reviews and required delivery checks pass before merge.

## References, assumptions and clarification

The approved visual source is `design/project/app/11_Project Detail.dc.html`:
its archived toast has `action-label="Undo"` and a direct action. Harvest's
[project archive/restore documentation](https://support.getharvest.com/hc/en-us/articles/360048686911-Archiving-restoring-and-deleting-projects)
(read 2026-10-22) confirms restoring makes a project active and archiving retains
its data. A Harvest Undo toast was not exercised: reference-account writes are
out of scope. The direct Undo affordance comes from the approved design, not a
claim of an observed Harvest mutation.

Clarification scan: scope, identities, state transitions, errors, accessibility,
authority and completion are clear from H102 and the existing status contract.
No new product question is needed. H103 remains separate: Harvest's documented
restore-before-edit rule conflicts with an observed enabled archived editor;
its write behavior is unverified. This increment makes no decision about it.
