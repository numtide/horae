# Research: Project bulk actions

## Mutation boundary

**Decision**: One `set_projects_active(project_ids, active)` function calls `require_manager`, validates 1–100 submitted IDs, parses/deduplicates/sorts UUIDs, and changes them in one transaction. Reuse `lock_project` and the existing UPDATE through an in-transaction helper; dispatch existing events after commit only for actual transitions.

**Rationale**: Existing setters already distinguish no-ops and lock before comparison. Ordered UUID locks avoid reverse-order batch deadlocks; org-scoped lookup treats missing and foreign rows identically.

**Alternatives**: Client loops permit partial completion; queues add unrelated recovery infrastructure; separate bulk SQL duplicates lifecycle behavior.

The existing plugin bus is best effort and may drop saturated calls. This feature guarantees one dispatch attempt per committed transition, not durable delivery or an outbox.

## Selection and accessibility

**Decision**: Page-local ID set; use the same filtered list for rendering and selection. Clear on filter events, intersect with available rows before confirmation, snapshot names/IDs. Keep Modal mounted. Add optional mixed/compact/disabled Checkbox props with defaults preserved.

**Rationale**: Handoff checkboxes are 18px pine with a 34px footprint; Actions sits between New project and Import. Shared menus already support keyboard and top-layer positioning; native dialogs provide inertness and focus restoration.

**Alternatives**: Hidden cross-filter baskets are surprising; new widgets duplicate existing primitives. Bulk task/tag/edit entries are omitted by approved scope.

## Verification

**Decision**: SQLx helper tests plus real-session HTTP tests in the isolated browser server, controlled transport-failure fixtures, and before/after style snapshots of eight other pages.

**Rationale**: The global server state is a OnceCell, so adding another independently initialized HTTP SQLx suite in the same test binary can target the wrong pool. The isolated browser runner avoids that trap. Reuse existing deterministic blocked-transaction test helpers.

**Alternatives**: UI stubs cannot prove authorization/rollback; real imported data is not a test fixture.

## Tooling

Installed Spec Kit skills and setup scripts are used; no standalone specify executable or extension hooks are present. No agent-context update script exists in this checkout, and no new technology requires an AGENTS.md change.
