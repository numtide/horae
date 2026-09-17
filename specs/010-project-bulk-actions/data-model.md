# Data model

No schema changes.

- Project: existing organization-owned record. Only active changes: true → false (archive), false → true (reactivate); same state is a no-op.
- Selection: page-local set of UUIDs, cleared on filters and successful changes. Only currently visible IDs enter confirmation.
- Confirmation: action and snapshot of IDs/display names, pending flag, optional error. Open → cancelled or pending → succeeded/failed; failed remains retryable.
- Batch: 1–100 submitted IDs, deduplicated/sorted before locking; one commit, rollback on any invalid or missing/foreign row.

Project details, assignments, entries, invoice links and amounts remain unchanged. Existing event bus semantics remain best effort.
