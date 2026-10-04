# Own permission explanation reader

T095–T097 refine FR-006/007/010/012/017/018 and US4. This is a read-only delivery
slice, not policy activation, another person's editor or complete Settings UI.
FR-012 already requires a reachable own-access explanation for non-administrators;
no unresolved Harvest lifecycle predicate is selected here.

## Session identity payload repair (T162–T164)

At `4c00660`, `get_me` serializes the complete database `User`, including populated
cost/billable rates, OIDC subject, creation time and activity. Its callers need
only ID/organization ID for recovery ownership, name/email for the account menu,
and the legacy role for unchanged pre-cutover display gates. None consumes the
financial/provider fields. This is the independent OP01/FR-008 payload boundary,
not a replacement for the canonical explanation reader below.

Return an explicit `CurrentUser` with exactly `id`, `org_id`, `name`, `email` and
`org_role`. Keep `require_user`'s current session/activity lookup and its internal
database model; construct the response without serializing or flattening that
model. Keep current legacy presentation helpers, not additional authority flags.
No caller-supplied ID selects the returned person. Logout stays independent of
active-account authorization. This changes neither grants nor other endpoint
responses, and does not activate or satisfy canonical shell authorization.

Acceptance uses the registered route and real session cookies: exact fields with
non-null private values for every legacy role, foreign/forged target parameters,
same-cookie role change and deactivation, anonymous/missing identity denial and
logout revocation. Run existing navigation, approval, shell and permission-editor
consumer tests with the production response type where those tests use full user
fixtures; compile server and WASM. No new SQL, schema, CSS or product predicate
is needed for the production projection.

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

## Settings consumer (T120–T122)

Use the existing no-argument server function in Settings, independently of the
plugin resource. Preserve General/Plugins and the app shell. The read-only section
uses the handoff's heading, explanatory copy and administrator-only callout with
existing utility/banner classes; no global CSS changes. Do not reproduce the
prototype's fake profile selection, retired-Manager warning or update button.

Show every returned grant using shared, exhaustive presentation descriptions,
without normalization, profile classification or inferring Administrator from
the grant set. Explain own/managed/all scope in the descriptions. Show management
relationship counts separately, not names or UUIDs; responsibility does not grant
unconditional access. Configured grants do not enable absent product features;
saved-report grants refer to inactive owners, not inactive report features.
No profile/source label is available
in this DTO, so do not fabricate one. Workspace will reuse these descriptions
when its independently authorized editor is integrated.

Loading/reloading hides the previous snapshot. Legacy None explains that detailed
permissions are not enabled, not that the user has no access. An empty grant list
has an explicit empty state, not an inferred Member floor. Authentication and
forbidden failures give distinct recovery guidance; other errors are sanitized.
A real refresh/retry control reloads from the server and is disabled while pending.
No edit controls, administrator-only help destination, raw error, revisions or
assignment identifiers are rendered. An unsupported catalog is an unavailable
state, never a guessed explanation.

SSR tests cover these states, exact selected grants, independent administrative
identity, private-field omission and pending stale-content suppression. A focused
resource test must exercise the production component's read and refresh wiring.
Server/WASM lint and formatting are local gates. Browser viewport/theme/keyboard
acceptance and the remaining Workspace surface still belong to T018; unit or SSR
tests must not mark those complete.
