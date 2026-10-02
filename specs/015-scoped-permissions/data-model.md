# Permission data model

Status: foundation contract finalized; persisted policy and approval design pending reference verification.

## Pure record-scope foundation

- `AccessScope`: private flags for `NONE`, `OWN`, `MANAGED_PEOPLE`, `MANAGED_PROJECTS`, `ORGANIZATION`. Union is explicit and never infers a capability or a broader role.
- `Actor`: authenticated person's UUID, organization UUID and current active status.
- `ScopedResource`: organization UUID and optional person/project UUIDs. Missing IDs never match a person/project scope.
- `ManagementAssignments<'a>`: actor UUID, organization UUID, borrowed managed-person and managed-project UUID slices. Ordinary project membership must not be supplied as management.

The server supplies current trusted facts for one capability evaluation. These types do not authenticate caller-provided claims and are not a browser authorization payload. The result answers only record coverage; capability resolution, action-specific filters and business locks remain separate mandatory checks.

## Pending persisted model

The independent [grant catalog](contracts/grant-catalog.md) now defines typed
permissions, six profile defaults and normalized editable selections. These are
pure data/algorithms, not authenticated actor facts or persisted policy activation.

The full design must cover capability grants, built-in/custom templates, person-specific grants, project/person assignments, revisioned access-change audit, migration mapping, submissions and approval coverage. Every persisted entity is organization-scoped. Template-deletion grant preservation remains provisional pending resolution of [C01](contracts/current-account-investigation.md); no destructive alternative is approved. Approval coverage must represent dates and projects without requiring a time-entry row, because approved empty cells can be locked.

Do not create schema or choose template propagation, custom-profile classification, approval prerequisites or coverage splitting until the reference evidence and operation matrix are complete.
