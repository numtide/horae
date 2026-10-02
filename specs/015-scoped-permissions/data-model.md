# Permission data model

Status: foundation finalized; persistence proposal made concrete on 2026-10-02.
The proposal below is reviewable design, not an installed schema or a completed
T008 gate. Reference-dependent transitions and approval coverage remain open.

## Pure record-scope foundation

- `AccessScope`: private flags for `NONE`, `OWN`, `MANAGED_PEOPLE`, `MANAGED_PROJECTS`, `ORGANIZATION`. Union is explicit and never infers a capability or a broader role.
- `Actor`: authenticated person's UUID, organization UUID and current active status.
- `ScopedResource`: organization UUID and optional person/project UUIDs. Missing IDs never match a person/project scope.
- `ManagementAssignments<'a>`: actor UUID, organization UUID, borrowed managed-person and managed-project UUID slices. Ordinary project membership must not be supplied as management.

The server supplies current trusted facts for one capability evaluation. These types do not authenticate caller-provided claims and are not a browser authorization payload. The result answers only record coverage; capability resolution, action-specific filters and business locks remain separate mandatory checks.

## Persisted model proposal

The independent [grant catalog](contracts/grant-catalog.md) now defines typed
permissions, six profile defaults and normalized editable selections. These are
pure data/algorithms, not authenticated actor facts or persisted policy activation.

The existing organization row is the proposed serialization point for authorization.
It gains a policy version/mode and a monotone access revision; installing support
must leave legacy policy active. A reviewed atomic activation changes the mode.
Mode is organization-wide, never an endpoint-specific fallback or a browser flag.
Missing/incompatible policy state after activation denies access; it does not
restore old Manager checks or automatically repair grants.

All new tables have a UUID v7 primary key and a non-null organization foreign key.
References to users, projects, templates and permission records use composite
`(org_id, id)` foreign keys backed by corresponding unique constraints. A separate
organization column plus a globally unique resource ID does not prevent linking
resources across organizations. Existing malformed assignments block their
reviewed migration; do not erase or relink them to make constraints pass.

| Proposed entity | Stored facts | Constraints and consumers |
| --- | --- | --- |
| Person permission state | User ID, canonical normalized grant set, explicit administrative identity, selected profile/template provenance, catalog version, person revision | One row per organization/user; active status and sign-in identity remain on `users`. Custom grants/display classification cannot create administrative identity. Trusted loaders, both permission screens and audit use this same state. |
| Custom permission profile | Organization-local ID, name, normalized grant set, template revision, creator and timestamps | No cross-org lookup; Member floor and prerequisites enforced; unsupported/unknown grants rejected. Deletion preserves assignees' current grants/scope as person-specific configurations. Exact name-equivalence/update rules remain gated below. |
| Project management assignment | Manager user ID, project ID, revision/provenance | Unique organization/manager/project relationship; distinct from ordinary tracking membership. Migration of legacy Lead/Admin labels is reviewed, not inferred at request time. |
| Person management assignment | Manager user ID, managed user ID, revision/provenance | Unique organization/manager/person relationship; no inference from shared projects or transitive management. Self-assignment validity remains part of the operation matrix. |
| Access change and request receipt | Explicit user/operator actor variant, typed action, organization-local subject IDs, request identity, canonical intent, before/after revisions and permission/scope state, timestamp and outcome | One committed receipt per organization/principal/request identity; changed state and durable audit commit together. Administrator-only user-facing audit reads; operator attribution is not a fabricated user FK. No session credentials, OIDC subjects or unrelated financial data in snapshots. |

These are logical entities, not a demand for one table per row in this document.
The concrete migration must choose typed columns/relations appropriate to its
validated queries, retain the composite tenant constraints, and avoid generic
unbounded JSON authorization documents. Historical audit snapshots may use a
versioned typed serialization; runtime authority must not be reconstructed from
event history.

### Stored grants versus profile provenance

Persist the canonical selected grants rather than recomputing saved access from a
mutable display name or whichever defaults happen to ship in a later build. The
selected built-in/custom profile, applied template revision and descriptive
classification explain origin/differences; they are not an alternate grant source.
Applying a profile is an explicit command. A template change, application or
removal that affects existing people must update their canonical states, revisions
and audit in the same authorized operation once that behavior is verified.

The user resolved C01 on 2026-10-02: deletion removes the reusable template and
detaches current assignees without changing their canonical grants, individual
adjustments, administrative identity or management relationships. Their access
becomes a person-specific configuration, not a newly inferred built-in profile.
Record the source/template deletion in the same authorized revision/audit
transaction; preserved grants do not make the provenance change an exact no-op.
No cascading delete may remove permissions or assignments. Historic audit
provenance remains available to authorized readers, without keeping a deleted
template applicable. Confirmation explains preservation; cancellation writes
nothing. Revocation is a separate confirmed command, not a deletion side effect.

This approved Horae behavior does not establish Harvest's actual deletion result
or decide template rename/update propagation, name equivalence or reapplication.
Those remaining contracts still need their own evidence/decision.

Administrative identity is persisted separately from ordinary grant membership:
only an explicit authorized Administrator assignment can set it. A custom profile
containing every known grant is not an Administrator. The exact profile-source
discriminator and consistency checks must be finalized with the editor's saved
classification contract; do not infer them from an ordered enum or legacy role.
The last-administrator count uses active users plus that explicit identity in the
same organization, never a count of users possessing a particular ordinary grant.

### Revisions and atomic changes

Use non-negative checked integer revisions; overflow fails the change rather than
wrapping. Person/template revisions detect stale edits; the organization revision
also fences assignment, activation, policy and other effective-access changes.
Bound mutation/preview confirmations to every relevant revision, including the
source template. Revisions are not supplied as authority by the browser.

An unchanged effective grant set is not necessarily a no-op: changing selected
profile provenance or management scope may still be a real auditable change.
Conversely, an exact no-op must not manufacture a permission-change event.
Unknown catalog values, non-canonical persisted sets and incompatible versions
fail closed and require explicit repair; reading a row must not silently add
newly introduced prerequisites/defaults and grant access during a software upgrade.

The lock order, command/retry contract, denied outcomes and verification cases are
defined in [permission-state.md](contracts/permission-state.md). Existing runtime
locks do not yet implement that complete protocol.

### Deferred approval and migration entities

Approval coverage must represent dates and projects without requiring a time-entry
row, because approved empty cells can be locked. Attribution, submission state and
independent invoice/company locks remain separate. Interval splitting, newly
created projects and historical whole-week conversion require the verified
FR-019 contract before selecting a coverage schema. No approval table is replaced
by a generic permission audit.

The [migration contract](contracts/migration.md) still owns legacy role mappings,
historical requester provenance and atomic cutover/recovery. The proposed mode and
revisions supply mechanics, not approval of any mapping or a permanent legacy mode.

### Remaining design gates

| Gate | Decision/evidence still required | Why storage mechanics do not settle it |
| --- | --- | --- |
| Saved template application (C01 deletion resolved) | Update/reapply outcomes, name equivalence and saved classification outside the approved deletion flow | Determines remaining person/template changes and uniqueness/transition constraints |
| C02 resolved; C03–C04 open | Report grants authorize their financial projection/export without ordinary rate grants; managed billable/cost scopes remain unresolved | Keep report and ordinary rate authorization separate; no extra rate grant is persisted merely to open a report |
| C05–C06 and FR-019 | Independent scheduling locks, approval/withdrawal predicates and coverage transitions | Determines approval locks and migration, not merely grant serialization |
| C07 and FR-005 | Assignment authority, loss preview, induced profile changes | Determines complete affected set, prerequisites and audit scope |
| T007 / US5 | Approved existing-data mapping and historical job/approval transition | Needed before any policy activation, including imported/development data |

Do not create schema or choose template propagation, custom-profile classification, approval prerequisites or coverage splitting until the reference evidence and operation matrix are complete.
