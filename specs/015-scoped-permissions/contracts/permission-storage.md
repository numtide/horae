# Non-activating permission storage

T035/T036 store the confirmed state without replacing any current authorization
guard. T037/T038 own authenticated mutations, replay and audit; T013 owns the new
management relations and approval coverage. This is Horae's storage design, not
a claim about Harvest's internal database.

## Reference and product decisions

- [Harvest permissions](https://support.getharvest.com/hc/en-us/articles/44171549176077-Permissions)
  documents 50 custom profiles per account and unique names up to 100 characters.
  The current public editor's `vi` handler trims surrounding whitespace and
  rejects an empty name before requesting creation. Its server comparison is
  unverified. User-approved FR-032 supplies case-insensitive creation uniqueness.
- The [Users API](https://help.getharvest.com/api-v2/users-api/users/users/#custom-profiles)
  describes a current matching profile label, not an immutable assignment ID.
  Its custom/access-role name collision error does not establish creation rules.
- `contracts/profile-application.md` already separates canonical grants, explicit
  application provenance and computed presentation. Presentation needs no column
  in the authoritative state. Unknown backend classification therefore does not
  block storing grants or known source IDs. Rendering and save/reload parity
  remain independent acceptance obligations; loading never infers provenance.

## Additive schema and state ownership

Recheck the migration sequence before creating
`crates/horae/migrations/0042_scoped_permission_state.sql`.

| Relation | Required fields and constraints |
| --- | --- |
| `organizations` additions | `permission_policy_version` integer defaults to 0 (legacy); nonnegative `access_revision` bigint defaults to 0. Installing the migration does not activate a policy |
| `permission_templates` | UUID v7 `id`; `org_id` FK; display `name`; catalog version; canonical `text[]` grant IDs; nonnegative revision; creation timestamp. Unique `(org_id, id)` for tenant references and unique `(org_id, lower(name COLLATE permission_template_names))` for FR-032 |
| `person_permission_states` | UUID v7 `id`; `org_id` FK; user ID with composite `(org_id, user_id)` FK and one row per organization/user; catalog version; canonical `text[]` grants; explicit administrative identity; source discriminator and source metadata; nonnegative revision; creation/update timestamps |

Add the required unique `(org_id, id)` key on `users`; never rewrite user IDs,
roles, project membership or children to create it. Migration creates no template
or person-state rows and copies no legacy roles. Canonical grants are independent
of profile names and never reconstructed from audit or current template defaults.

### Name boundary

The server trims surrounding whitespace, rejects blank names and counts Unicode
scalar characters for the 100-character limit. Preserve display casing and do not
remove internal whitespace or accents. PostgreSQL's `lower(name COLLATE permission_template_names)` is the single
comparison authority for lookup/conflict checks and the unique index; do not
introduce a differently normalized Rust key. Migration 0047 pins a deterministic
ICU root (`und`) collation instead of inheriting the database locale: a C-locale
cluster otherwise admits both `Ágil` and `ágil`, violating FR-032. This requires
PostgreSQL built with ICU support (included in the pinned Nix package). Accents
and different Unicode normalization forms remain distinct; no accent stripping,
normalization library or separate Rust comparison is introduced. SQL also checks
nonempty/length bounds.

The index replacement is transactional and does not rewrite names, IDs, grants
or assignments. Existing equivalent names cause migration failure with the old
index and rows preserved; never rename/delete profiles automatically. Diagnose
and resolve collisions explicitly before retrying on an existing installation.
No canonical policy is activated. See PostgreSQL's
[collation documentation](https://www.postgresql.org/docs/15/collation.html)
for ICU support and case-conversion semantics.

Names do not identify a template in mutation requests: use its tenant-scoped ID
and revision. A custom label equal to a built-in name cannot confer Administrator
identity. Do not add a reserved-name prohibition from the API's lookup warning.
Creating the 51st template and concurrent creation at the limit must be denied
by T037/T038 under the organization lock; a uniqueness index alone does not
enforce the count. T035/T036 must not claim that command behavior is implemented.

### Provenance versus authority

Use a closed source discriminator: `built_in`, `template`, `individual`.

- `built_in`: exactly one known built-in key; template fields absent. Grants may
  include per-person adjustments. The source key alone never sets administrative
  identity.
- `template`: no built-in key; same-organization template ID and nonnegative
  applied revision are required. A grant-equivalent template never confers
  administrative identity. Canonical person grants may differ from the template.
- `individual`: no active built-in/template reference. Used when a custom template
  is deleted under the approved C01 rule; canonical grants and administrative
  identity remain unchanged. Historical source information belongs in durable audit.

Administrative identity is an independently stored explicit fact. Source-shape
checks do not couple it to a source key or grant equality. Authorized transition
contracts determine which commands may change that fact; storing the independent
facts does not authorize arbitrary transitions or template-based promotion.

Use SQL checks for valid source shapes and composite FKs for template/user
tenancy. Template deletion is restricted until the authorized transaction detaches
affected person states and records provenance changes; never `CASCADE` grants or
silently `SET NULL` an incomplete source shape. A same-name replacement has a new
ID and cannot inherit old applications. Do not infer provenance for migrated
legacy records: T007 still owns that mapping.

## Trusted loading

The internal loader accepts a server-owned organization and user ID and uses
compile-time checked SQL in `server_fns/permissions.rs`. It is not a public server
function and does not authorize the caller. Keep active user/session checks,
locks, revisions and policy-mode enforcement with the eventual command/consumer.

Decode every stored grant through the closed catalog; use
`PermissionSelection::from_stored` to reject unknown values, duplicates, missing
floor/prerequisites and unsupported catalog versions. Never normalize stored
grants. Decode source shapes strictly and preserve explicit administrative identity
without inferring it from source or selection. An equivalent non-admin selection
stays non-admin. Missing rows are distinct from invalid rows and must
not cause legacy fallback after activation. No loader is wired into live guards
by this increment, and no client DTO becomes trusted mutation input.

## Required storage tests

T035 must exercise the installed schema and real loader in isolated PostgreSQL:

1. New and populated organizations stay in legacy mode; no automatic state rows,
   role conversion or business-data changes.
1. Six built-in selections and adjusted/custom/individual selections round-trip
   exactly, with explicit identity and provenance distinct from displayed names.
1. Duplicate person rows and foreign user/template references fail at the database
   boundary; foreign-org lookups disclose no state.
1. Invalid source shapes, negative revisions, unknown
   grants, duplicate grants, incomplete selections and future versions fail closed.
1. Equivalent names in one organization conflict; different organizations succeed.
   Test ASCII and non-ASCII case pairs against the same database comparison,
   plus canonical trimming/empty and 100/101-character boundaries at the server.
1. Referenced templates cannot be deleted directly; the later authorized deletion
   transaction must preserve grants and audit detachment. No implicit rebinding.

T036 is complete only with schema, model/loader, regenerated `.sqlx/`, real storage
tests, offline server compilation, Clippy and formatting. Transactional commands,
50-template races, migration activation, UI/browser and full parity remain open.
