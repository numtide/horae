# Bulk project status contract

## Server function

`set_projects_active(project_ids: Vec<String>, active: bool) -> Result<Vec<Project>, ServerFnError>`

- Authenticated manager/admin; organization comes from session, never client input.
- Reject zero or more than 100 submitted IDs and malformed IDs before writes.
- Deduplicate/sort UUIDs, lock/update with org predicates in the same order, commit once.
- Missing/foreign project: NOT_FOUND and full rollback. Validation/auth failures use existing named error conventions.
- Return each project once. Existing activation events are dispatched after commit, one attempt per actual transition.
- Retry requests a state, never toggles. Lost response may mean commit already happened.
- No durable exactly-once event-delivery promise; preserve current plugin bus semantics.

## UI

Named keyboard-operable checkboxes; header unchecked/mixed/checked. Manager/admin-only Actions shows selected count or guidance, and Archive or Reactivate according to scope. Filters clear selection. Over 100 disables action with explicit narrowing guidance.

Confirmation lists names/count, explains history preservation, and offers cancel/confirm. Pending blocks duplicates/dismissal. Failure remains visible and retryable; success clears selection and refreshes resources. Shared defaults, both themes and responsive scroller remain intact. No deferred prototype features.
