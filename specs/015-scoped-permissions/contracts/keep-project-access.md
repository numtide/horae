# Explicit keep-project-access choice

FR-030 resolves the editor action under the already confirmed Harvest parity
mandate. It does not change FR-025's read-only retention threshold.

## Evidence and limits

The retained Harvest editor asset
[permissions-VNAENDKR.js](https://cache.harvestapp.com/static/people/permissions-VNAENDKR.js)
was inspected on 2026-10-03 without execution. Its provenance and SHA-256 are in
[the evidence register](current-account-investigation.md#current-catalog-and-editor-provenance).
Function `a3` resets the option to unchecked and exposes the warning for a
positive lost-project-manager count. The confirmation handler explicitly adds
`projects:read:managed` and `projects:write:managed` when checked, then submits.
Closing the dialog retains the draft without submitting it.

This proves client intent, not Harvest persistence or its server's loss predicate.
The [permissions guide](https://support.getharvest.com/hc/en-us/articles/44171549176077-Permissions)
reserves permission editing to Administrators. No account mutation or new user
decision is needed to specify this observed action. Forecast behavior in the same
asset is excluded from Horae's scope.

## Required command behavior

- For a permission edit whose proposed result would lose existing project-manager
  designations under FR-025, offer an initially unchecked alternative to removal.
  Explain that it adds both managed-project reading and editing, not merely
  preserves a label or supplies read-only access.
- Choosing it changes the proposed permission set. Display the resulting grants,
  project scope and any remaining relationship losses before final confirmation.
  It does not add project creation, organization-wide project access, rates,
  administrative identity or unrelated grants.
- Recompute FR-025/029 effects from that final set. Project-only grants do not
  satisfy FR-028; person-management losses must still be previewed and confirmed.
  The retained grants apply to managed-project scope, not a hidden allowlist of
  projects shown in one earlier warning.
- Commit only with current same-organization Administrator authority, current
  person/assignment revisions and confirmation of the final effects. Use the
  existing atomic permission, relationship, revision and audit protocol. A preview
  is read-only; unavailable/stale previews cannot authorize a save.
- Closing cancels persistence and retains the draft. Reopening requires fresh
  effects and an unchecked choice. An unchecked final confirmation follows normal
  FR-025 removal. Choosing this action cannot restore designations already removed.

## Acceptance and ownership

T012/T013 own relationship effects; T010/T011 own permission persistence;
T016/T018 own shared editor behavior. All cases require production-path tests at
integration time, not copied SQL or a passing pure model alone.

| Case | Required outcome |
| --- | --- |
| Lose project read; leave option unchecked and confirm | FR-025 removes only the current confirmed designations |
| Explicitly check and confirm | Add managed read/write, retain eligible current designations, audit actual changes |
| Remove only editing while retaining read | FR-025 retains designations without adding editing or requiring this action |
| Lose project and person-compatible grants; choose keep-project-access | Retain eligible project designations; independently confirm FR-029 person removals |
| Cancel/reopen or fail to load current effects | No persisted change; no remembered implicit opt-in |
| Stale assignments/grants, revoked actor, foreign target or audit/write failure | No partial grant or relationship changes |
| Removed historical designation or unrelated project | No automatic reassignment or unrelated access |

Full permission activation, migration and browser acceptance remain pending.
