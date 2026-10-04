# Permission-audit lookup

FR-010/011/013, T062–T064 refine the internal read portion of T041; T123–T125
add authenticated single-record delivery. T182–T185 extend it with browsing under
the boundary below, without policy activation. Reuse receipt storage 0043; no
schema, retention policy or operator mutation is introduced.

## Read boundary

### Browsable permission history

FR-013 and T018/T041 require an administrator to discover changes without knowing
receipt IDs. Extend the existing authenticated lookup with a paged history reader
and a Workspace Audit log consumer. This is permission-change history only:
do not fabricate the prototype's invitation/invoice/export events or its 90-day
retention limit. No new retention/deletion policy or database migration is needed.

`list_permission_audit(after, expected_requester)` returns the session-derived
requester, up to 25 `AuditEntry` values and an optional exclusive `(created_at,id)`
cursor in descending order. The page size is a transport bound, not a product
retention rule. An optional captured requester must match the current session;
the UI retains the first admitted requester for subsequent page/refresh requests.
Authorize using the same current policy, administrator and activity transaction
as single-record lookup. Scope before pagination, decode every returned record
strictly and never silently skip malformed history. No raw intent/replay payloads
or current-person/template joins are added. Actor and subject UUIDs remain the
recorded identities, including deleted/inactive sources and operator attribution.

Use one SQL page query, not repeated separately authorized lookups. Read a bounded
lookahead to determine continuation; timestamps with equal values are ordered by
receipt ID. Cursors are ordering bounds, not record lookups or authority. Every
page reauthorizes; cursor values must not select another organization or requester.
No total count or frozen multi-request snapshot is promised.

The consumer uses existing Workspace navigation, table/form utilities and native
expandable details for actual historical grants, provenance, administrator identity
and relationship additions/removals. Preserve explicit unchanged outcomes rather
than naming them access changes. Do not show hidden admin data while loading or
after a denied refresh. Paging replaces the displayed page and supports returning
to newest entries; it must not combine pages from different requester contexts.
Provide loading, empty, denied, retry and end-of-history states without mock data.

Route `/admin/audit` uses canonical Administrator identity, independently of the
legacy role. Keep the existing People/Importers shell gates unchanged until their
operation contracts are implemented. The current canonical Administrator can
reach history from Settings' own-permission section as well as Workspace Data.
Shell and page errors must not disclose internal authentication/storage details.

Acceptance: production-created profile/template/project-manager receipts, exact
stable paging including timestamp ties, foreign/cursor isolation, malformed data,
revocation between pages and both gate orders, real registered sessions, UI paging
and stale-response/error handling, all three detail renderers and unchanged cases.
Preserve existing single-receipt behavior and run the shared native/WASM/SQLx and
format gates. Browser verification remains separate; no canonical policy activation
is authorized by this history flow.

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

History browsing and bounded Chromium acceptance are implemented; see T182–T185
and `quickstart.md` for the distinct server, controlled-DOM and browser evidence.
Full T042, cross-browser/UI acceptance and T041's complete acceptance remain open.
Unresolved person-management lifecycle does not
affect this already-confirmed Administrator-only read rule.
