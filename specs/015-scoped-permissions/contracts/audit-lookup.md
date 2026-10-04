# Permission-audit lookup

FR-010/011/013, T062–T064 refine the internal read portion of T041; T123–T125
add authenticated single-record delivery. This is not audit browsing or policy activation. Reuse receipt storage
0043; no schema, pagination product rule or operator mutation is introduced.

## Read boundary

Start fresh READ COMMITTED, READ WRITE and acquire organization FOR SHARE, then
the same-organization requester FOR SHARE. Use local five-second statement and
ten-second idle-transaction limits; retain any stricter inherited statement limit.
Require mode 1,
an active same-organization requester and strictly loaded explicit Administrator
identity. All ordinary grants, legacy Admin role or authorship of a receipt do
not substitute. Authenticate before looking up the requested receipt UUID.
Query by organization and receipt ID, not recorded actor: an Administrator may
read other organizational actors' history. Foreign and missing IDs both return
no record. Materialize the projection under the gate, then commit and return.

Plain state/receipt reads follow the organization and requester gates. No project
row locks, FK inserts, writes or external work occur. A revocation winning UPDATE
makes a waiting reader reload and deny; an already-authorized SHARE reader may
finish before the revocation. This does not promise recall of delivered data.
The requester lock also fences direct deactivation that does not acquire the
organization gate. No locks survive response materialization.

## Authenticated delivery

`get_permission_audit(receipt_id: Uuid)` derives organization and requester only
from the current session, never request claims. Return `Option<AuditEntry>` using
shared historical DTOs in `models/permission_audit.rs`. Missing and foreign
receipts return the same successful null response after authorization. Missing,
expired and already-inactive sessions return 401; denied current authority or
non-active policy returns sanitized 403. Invalid stored authority/history,
unsupported formats and database failures return one sanitized 500 message.
Do not fall back to the legacy Admin role or return raw SQL/parser errors.
The DTO is display data, never input for server authorization or a mutation.

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

Extend the existing registered-route test harness (one global AppState) with real
cookies, forged identity/tenant payloads, independent administrator identity,
revocation, exact historical projection and non-disclosing errors. Reader tests
cover direct deactivation in both lock orders, inherited transaction settings,
timeout/cancellation cleanup and all existing decoder cases.

Full T042, history browsing, browser integration and T041's
complete acceptance remain open. Unresolved person-management lifecycle does not
affect this already-confirmed Administrator-only read rule.
