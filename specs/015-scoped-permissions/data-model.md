# Permission data model

The internal migration preflight returns only transient integer diagnostic counts
under `contracts/migration-preflight.md`. No identifiers, readiness flag,
confirmation token, persisted preview or new table is introduced. These counts
cannot authorize activation or substitute for its complete reviewed input set.

Budget email preparation adds only an optional private transient record under
`contracts/budget-email-authority.md`: current recipient/message with stored
attempt count. Skips return no record; terminal rejection stores the existing
sanitized reason within the preparation transaction. Notification/outbox schema
is unchanged; no browser DTO or claim-derived user authority is introduced.

CSV delivery uses transient native cursor rows and per-output-block project IDs
under `contracts/csv-exports.md`. Migration 0046 adds only an invoker transport
function: no new table, persisted job, public DTO or business-data conversion.
The private payload weight includes every projected variable-width field.
Invoice metadata and nullable line-ID sentinel share one source projection;
authorization facts are refreshed separately and are never loaded from it.

Materialized project exports use a private captured UUID set under
`contracts/project-exports.md`. IDs accompany the existing server-only export
rows for current-scope revalidation after rendering; they are neither persisted
nor added to spreadsheet columns. No new table, grant or public DTO is required.

Status: foundation finalized; persistence proposal made concrete on 2026-10-02.
The reviewed non-activating portion now has migration 0042, server-only typed
read models and strict loaders under T035/T036, exercised in disposable databases.
The broader proposal below is not a completed T008 gate or active policy.
Reference-dependent transitions and approval coverage remain open.

Migration 0044 now separates canonical project/person management relationships
from tracking membership for internal profile changes. The reviewed
`contracts/person-profile-commands.md` specifies their tenant references, revision
fencing and confirmed deletion snapshots. No legacy data is copied, no addition
endpoint is exposed and legacy runtime authorization remains unchanged.

Internal project replacement follows `contracts/project-management-commands.md`:
retain existing edge identities/revisions, audit additions/removals and advance the
organization access revision once per real set change. Parent project KEY SHARE
NOWAIT prevents a lock cycle with the legacy editor; it does not synchronize that
editor's membership-based manager flags or replace its form revision contract.

The audit reader follows `contracts/audit-lookup.md`. It reads existing
0043 receipts by tenant and receipt UUID under organization SHARE, using current
explicit Administrator identity. Its separate historical DTOs retain recorded
grants, provenance and removed relationships without joining current subjects.
Required nullable fields distinguish an explicit no-op from an incomplete
document. No intent, request ID or private replay result is projected; these
historical values cannot be loaded as current authorization state.
The session-authenticated `get_permission_audit` projects the same typed history
through `models/permission_audit.rs`; transport deserialization does not apply to
trusted stored permission models. No persisted shape or migration is changed.

Original import requester storage is specified in
`contracts/permission-state.md` under T086–T088: nullable
`horae_jobs.original_requester_id`, tenant-bound to an existing user, recorded
only on new authorized insertion. Historical NULL is preserved. It is provenance,
not effective worker authority, and is absent from existing external job DTOs.

## Own-access display

`models::own_permissions::OwnPermissions` is a separate shared display DTO under
`contracts/own-permissions.md`: exact grants, catalog version, independent
administrator identity, organization/person revisions and sorted own management
IDs. `auth::get_my_permissions` accepts no selectors and derives identity from
the session. Trusted storage models remain server-only and non-deserializable.
Legacy policy yields None, never staged grants or a synthesized profile. The DTO
does not carry authority for subsequent requests or imply resource access.

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

Separating management must preserve existing tracking memberships, not rebuild
them by delete/insert: `0030_project_creation.sql` cascades membership deletion
into cost overrides, budgets and restricted-task allowances. Preserve assignment
IDs, rates and child identities/content; verify M01–M08 in the
[migration contract](contracts/migration.md#concrete-preservation-and-preflight-cases).
These fixtures refine preservation, not the unapproved legacy role mapping.

| Proposed entity | Stored facts | Constraints and consumers |
| --- | --- | --- |
| Person permission state | User ID, canonical normalized grant set, explicit administrative identity, selected profile/template provenance, catalog version, person revision | One row per organization/user; active status and sign-in identity remain on `users`. Custom grants/display classification cannot create administrative identity. Trusted loaders, both permission screens and audit use this same state. |
| Custom permission profile | Organization-local ID, name, normalized grant set, template revision and timestamps; creator attribution belongs to the audited creation command | No cross-org lookup; Member floor and prerequisites enforced; unsupported/unknown grants rejected. Deletion preserves assignees' current grants/scope as person-specific configurations. FR-032 defines trimmed case-insensitive names; in-place updates are not assumed. |
| Project management assignment | Manager user ID, project ID, revision/provenance | Unique organization/manager/project relationship; distinct from ordinary tracking membership. Migration of legacy Lead/Admin labels is reviewed, not inferred at request time. |
| Person management assignment | Manager user ID, managed user ID, revision/provenance | Unique organization/manager/person relationship; no inference from shared projects or transitive management. FR-031 forbids equal manager/managed IDs. FR-027/028 govern writes and additions; FR-029 governs confirmed removal on grant loss. |
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
The display classifier can select a different matching template after the
available template set changes, even with identical person grants. Therefore it
must not overwrite explicit application provenance or manufacture access-change
revisions/audit on read; see `contracts/profile-application.md`.
Applying a profile is an explicit command. Application and deletion must preserve
their specified canonical-state, provenance, revision and audit invariants in one
authorized transaction. Saving a new template does not propagate privileges to
existing assignees. No in-place template editing or bulk propagation is assumed.

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

This approved Horae behavior does not establish Harvest's actual deletion result.
FR-032 separately settles trimmed, case-insensitive creation-name equivalence.
In-place update/rename is an evidence watch,
not a prerequisite for the documented create/apply/adjust/delete lifecycle.
[Profile application](contracts/profile-application.md) now distinguishes unchanged
saves, explicit baseline selection/reset and final individual grant edits.
Saved Harvest classification remains unverified; computed presentation is not an
authoritative storage field. `contracts/permission-storage.md` defines the reviewed
non-activating storage boundary without requiring Harvest's private schema.

Administrative identity is persisted separately from ordinary grant membership:
only an explicit authorized Administrator assignment can set it. A custom profile
containing every known grant is not an Administrator. The storage contract uses
closed built-in/template/individual source shapes independently of that identity;
do not infer either from display classification, an ordered enum or legacy role.
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
| Saved template application (C01 and FR-032 resolved) | Saved reference presentation and full command lifecycle acceptance | `contracts/permission-storage.md` separates grants, explicit identity and provenance; backend classification is not a prerequisite for additive storage. Explicit selection/reset versus unchanged saves follows `contracts/profile-application.md` |
| C02–C04 resolved; enforcement pending | Approved report projections, resource-specific billable scope and explicit organization-wide cost read/write | Keep report and ordinary rate authorization separate; general person billable rates use person management, project billable rates use project management, costs use independent read/write grants |
| C05 schedule modes documented; C06 approval visibility resolved | Company lock execution/calendar details and remaining custom approval/withdrawal predicates | Keep cutoff/configuration separate from coverage and invoice protection. FR-024 requires authority and visibility across the actual approval set, not a new grant prerequisite or withdrawal rule; see `contracts/company-locks.md` and `contracts/approval-visibility.md` |
| C07 retention/delegation resolved by FR-025/026 | Project creation | Project editors can change manager designations for compatible people, never global grants. Evaluate target managed-read eligibility with the proposed relationship; retention requires read, not editing. Confirmed read loss removes designations atomically, not membership/history. FR-027 reserves person-management writes to explicit Administrators; FR-028 requires compatible existing grants for new assignments without adding privileges. FR-029 retains relationships while any compatible grant remains and requires preview/confirmation for atomic removal on last-grant loss, preserving incoming relationships/history with no automatic restoration. FR-030 explicitly adds managed-project read/write only on confirmed keep-access, then recalculates both relationship rules. FR-031 forbids equal responsible/managed person identities, independently of actor identity and own/all grants. Other transitions still need predicates |
| T007 / US5 | Approved existing-data mapping and historical job/approval transition | Needed before any policy activation, including imported/development data |

Each schema increment needs a reviewed contract for its own entities, tenant
constraints, trusted loading and lifecycle before implementation. The reviewed
non-activating storage contract now supplies those local T035/T036 prerequisites;
implementation uses isolated test databases, not real data. Do not guess classification, approval prerequisites
or coverage splitting. The complete matrix and migration/activation review still
gate replacing legacy authorization, not the pure relationship-validation
increment in `contracts/person-management-validation.md`. No new persisted entity
is needed for that increment.
