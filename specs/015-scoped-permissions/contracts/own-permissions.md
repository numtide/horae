# Own permission explanation reader

T095–T097 refine FR-006/007/010/012/017/018 and US4. This is a read-only delivery
slice, not policy activation, another person's editor or complete Settings UI.
FR-012 already requires a reachable own-access explanation for non-administrators;
no unresolved Harvest lifecycle predicate is selected here.

## Request and response

Add Dioxus `get_my_permissions()` with no target/tenant/grant input. The wrapper
derives actor and organization from the session through `require_user`, then
calls a pool-injected reader with those trusted IDs. Unauthenticated/expired
sessions remain 401. A subsequently inactive, missing or foreign actor is denied.

Return `Option<OwnPermissions>`: policy version 0 yields None, never a synthesized
legacy profile or staged canonical data; version 1 requires strict canonical
state. Unsupported policy versions, missing state and malformed stored grants or
provenance fail with a fixed sanitized internal-error message, not fallback or
normalization. A separate shared serde DTO contains exact own grants, explicit
Administrator identity, catalog version, organization/person revisions and sorted
own managed-person/project IDs. No names, emails, rates, profile/template source,
other users' grants, receipts or audit are included. Existing trusted storage
models stay server-only and non-deserializable.

Management IDs describe responsibility relationships, not unconditional resource
access or a people-directory grant. Current action, field and state checks remain
mandatory. Neither DTO values nor revisions are reusable authorization tokens;
legacy activity changes need not change canonical revisions. UI and administrator
editing projections remain separate tasks; a future UI must not advertise absent
domains as functioning merely because their grant identifiers exist.

## Consistency and isolation

Use explicit READ COMMITTED, then organization SHARE, then reload and SHARE-lock
the active same-organization actor. This check precedes even the version-0 return.
Only then load canonical state and assignment IDs scoped by tenant and manager.
Materialize before commit, release locks before HTTP delivery, and never lock or
join managed resource rows. Canonical mutations serialize on organization UPDATE;
actor SHARE also fences direct deactivation. No transaction spans client waits.
Snapshot reads never change revisions, grants, assignments or receipts.

## Acceptance

| Test | Expected outcome |
| --- | --- |
| Legacy mode with absent, valid or malformed staged rows | None, with current activity still required |
| Six profiles/custom selections, independent admin bit and assignment ownership | Exact own projection; grants are not recomputed from role/source |
| Other manager, ordinary membership and foreign-tenant data | Not included in own management sets |
| Missing/inactive/foreign actor | Non-disclosing denial |
| Missing canonical state, unsupported policy/catalog or malformed grants | Sanitized error; no repair or legacy fallback |
| Winning organization update, including inherited REPEATABLE READ default | Whole new snapshot or revoked-actor denial, never stale/mixed state |
| Direct actor deactivation wait | Denial after commit; reader-first holds activity until read completes |
| Registered route with real cookies, expired/absent session and forged target fields | Own session is the only selector; safe HTTP status/payload |
| Successful or denied reads | No revisions, permission rows, assignments or receipts changed |

Extend the existing registered HTTP matrix: its process-global AppState owns one
test pool. Do not initialize that singleton in a second independently seeded test.
Only disposable databases may enable version 1. No real migration is authorized.
