# Internal permission-audit lookup

FR-010/011/013, T062–T064 refine the internal read portion of T041. This is not
audit browsing, a public endpoint or policy activation. Reuse receipt storage
0043; no schema, pagination product rule or operator mutation is introduced.

## Read boundary

Start fresh READ COMMITTED and acquire organization FOR SHARE. Require mode 1,
an active same-organization requester and strictly loaded explicit Administrator
identity. All ordinary grants, legacy Admin role or authorship of a receipt do
not substitute. Authenticate before looking up the requested receipt UUID.
Query by organization and receipt ID, not recorded actor: an Administrator may
read other organizational actors' history. Foreign and missing IDs both return
no record. Materialize the projection under the gate, then commit and return.

Only plain actor/state/receipt reads follow the organization gate. No user/project
row locks, FK inserts, writes or external work occur. A revocation winning UPDATE
makes a waiting reader reload and deny; an already-authorized SHARE reader may
finish before the revocation. This does not promise recall of delivered data.

## Projection

Return receipt ID, recorded timestamp, distinct `User { user_id }` or
`Operator { invocation_id, command }` attribution, and a typed historical audit.
Omit request ID, raw intent and replay result. Operator command is a recorded
code, never a claimed human identity, dispatch instruction or source of authority.
There is no operator command registry to invent an allowlist from. Auditing an
operator change does not expose its separately protected replay outcome.

Version 1 recognizes the three actual writer shapes: template before/after and
detached people; person-profile before/after and removed relationships; project
manager added/removed relationships. Decode to separate historical wire DTOs;
do not add unchecked Deserialize to trusted `PersonPermissions` or normalize
grants. Reject unknown/mixed fields, unsupported formats/catalogs, invalid
provenance, negative revisions and inconsistent change/no-op revisions.
Nullable `change`, `before` and `after` fields must be present: missing is not
equivalent to explicit null. Explicit unchanged outcomes remain unchanged, not
fabricated access-change events. Template changes require create/delete shape.

Do not join snapshots to live subjects, template defaults or relationships.
Historical actor activity is irrelevant to the requesting Administrator's current
authority. Deleted sources and later permission changes must not rewrite history.

## Acceptance

Use actual command-created receipts for all three forms and no-op cases; check
full historical grants/scope and metadata. Test grant-equivalent non-admin,
inactive, revoked, missing/foreign actor, legacy/future policy, malformed current
authority and foreign receipt denial. Exercise a real organization-lock wait
across revocation, operator attribution without intent/result disclosure, strict
wire errors and failed commands producing no successful record. Reads change no
state, revisions or receipt count.

Independent read-only design review closes this local SHARE/plain-read order.
Full T042, the authenticated history surface, browser integration and T041's
complete acceptance remain open. Unresolved person-management lifecycle does not
affect this already-confirmed Administrator-only read rule.
