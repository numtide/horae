# Persisted permission state and access-change protocol

Status: proposed Horae implementation mechanics for FR-010/011/013/014/017,
reviewed against `b80f8ab` on 2026-10-02. This is not a claim about Harvest's
internal storage or an authorization to create schema before T006–T009 pass.
Read with [data-model](../data-model.md), [migration](migration.md) and the
[current entry-point inventory](current-access.md).

## One authorization serialization boundary

Use the existing organization row as the first lock for protected operations:

| Operation | First lock | Required subsequent checks |
| --- | --- | --- |
| Business mutation under existing permissions, not updating the gate row | Organization `FOR SHARE` | Reload active actor and effective policy; lock selected resources; authorize the entire set and independent business state before writing |
| Access-affecting mutation, policy activation or any update to the organization gate row | Organization `FOR UPDATE` | Reload current actor; check command authority and expected revisions; lock affected people/templates/projects deterministically; protect last active administrator |
| Read/list/aggregate or bounded export page | Organization `FOR SHARE` in its authorization/data transaction | Load current trusted policy and assignments; scope rows, counts and sensitive fields under that boundary before serialization |

Every mutation that can change effective authorization must use the exclusive
path, not only permission-editor saves: active status, management assignments,
template application with verified person effects, relevant access settings and
tracking/task membership restrictions. Access-changing project edits must choose
the exclusive path from the outset; do not acquire a shared gate and upgrade it
after taking resource locks. Ordinary business writes can run concurrently under
shared gates. Authority-changing writes serialize per organization, not globally.
Even non-permission branding/preferences writes on the organization row must
start exclusive to avoid implicitly upgrading that gate. This lock choice does
not itself require Administrator permission; the operation matrix decides that.

The current `users::begin_user_access_change` establishes organization-before-actor
locking for user changes. It is not yet a shared protocol: project editing and
finalization have actor-before-organization paths. All participating consumers
must adopt one order before activation; adding the gate to just the new editor
would retain a deadlock/revocation gap. A single cross-command resource/table
hierarchy, then sorted IDs within each resource class, must be validated against
all participating paths, including locks acquired by foreign keys and triggers.
The existing assignment triggers also touch project revisions. T042 must record
that concrete hierarchy before implementation; per-command orders are not enough.
Do not pre-lock an actor and then acquire lower-ordered subject rows. The shared
organization gate already fences access changes while trusted authority is read.
Bulk commands must not acquire the same set in browser-provided order.

After a wait, reload authority under the lock; a session snapshot, earlier preview
or previously loaded grant set is insufficient. Access changes and sensitive
reads must not use an older transaction snapshot to bypass a committed revision.
On deadlock/serialization failure, roll back; any retry starts a fresh transaction
and repeats authorization and stale-edit checks. Never retry only the final SQL.

Do not hold this gate across external HTTP/OAuth, worker sleeps, mail/plugin
callbacks or client-paced streaming. Perform external preparation outside it,
then reauthorize before each durable commit or bounded authorized output page.
Revocation affects the next authorization check; already delivered data cannot
be recalled. Artifact download authorization checks the current caller and the
artifact's complete recorded scope, not just its original generation permission.

## Commands and revisions

Future session-authenticated server functions accept typed intent, not arbitrary
policy rows. Actor and organization come from the session, never the request body.
Keep distinct commands for applying a built-in/custom profile, editing a person's
grant selection, changing management assignments and changing a template through
its verified lifecycle. Do not add those public mutation endpoints until their
operation contracts are settled and enforced on all consumers.

A command carries an organization-local subject, expected organization/person/
template revisions as applicable and a request identity for outcome discovery.
The server computes canonical prerequisites and the full affected set, then
requires confirmation of the resulting change; it must not silently save a wider
selection than the administrator confirmed. Unknown permissions fail rather than
being dropped. A revision mismatch returns conflict without partial changes.

Within one transaction:

1. Acquire the required organization gate and reload the active session actor/
   current mode. Non-session operator commands use the separate attributed
   operator boundary below, not a synthetic active-user check.
1. Authorize the command before returning subject, template, revision or receipt
   details. Ordinary people management does not grant permission administration.
1. Resolve a matching request receipt before treating an old expected revision
   as a new edit. Exact committed intent returns its recorded outcome only if
   currently authorized to see its recorded scope; changed intent with a reused
   key conflicts. A historical outcome is not the person's current state, and
   replay must not require a deleted source template/assignment to exist again.
1. For a new request, resolve all referenced rows within the organization,
   enforce tenant constraints and verified assignment/profile transitions,
   compare all expected revisions and compute the exact before/
   after grants, administrative identity and affected assignments. Validate the
   last active administrator and all operation-specific invariants.
1. Persist the complete new state, increment affected revisions and the
   organization access revision, and insert the durable change record/receipt.
   Commit all of them or none. Audit failure must prevent the access change.
1. Emit existing applicable plugin events only after successful commit. Audit
   storage, not event delivery, is the source of durable access-change history.

An exact no-op may produce a request receipt but no fabricated change/revision or
plugin event. A lost acknowledgement is recovered by request identity; it cannot
reapply a once-authorized edit after revocation. Outcome lookup is authorized and
organization-scoped. A revoked actor must not receive old privileged snapshots
merely because it originally issued the request.

Unauthorized, stale and invalid requests do not alter grants, revisions or
success audit facts. If rejected attempts are recorded for operational security,
use a separate sanitized outcome path; an audit insert in the rolled-back
business transaction is not durable denial logging. Never persist/return a
foreign person's actual configuration in a denial record. Such logging contains
only bounded reason/action/correlation identifiers, not raw requests, grants,
guessed foreign metadata, credentials or provider subjects. Logger failure never
turns denial into success or makes a privileged receipt public.

## Trusted state and service boundaries

Trusted loaders reject missing/invalid/non-canonical persisted state in active
mode; they do not normalize it into more access, infer Administrator from grants,
or fall back to a legacy role. Two permission screens may share a read DTO, but
that DTO is not a trusted input for later authorization.

New user-initiated jobs need persisted requester identity and explicit affected
scope. Reauthorize at execution, publication and download boundaries under current
policy. Trusted system jobs/plugins keep their separately bounded service
authority. Historical jobs with no requester remain a migration review item;
do not silently attribute them to the owner. The local operator CLI is not a
session user, but its access-affecting writes still need the serialization,
revision and integrity protocol so they cannot race application operations.

Audit attribution therefore distinguishes `User(user_id)` from
`Operator(invocation_id, command_kind)`. User actors have a composite tenant FK.
Operator commands are authenticated by their existing privileged deployment/DB
access, not a fabricated user session; their typed command kind and stable
invocation/request identity identify the operation, not a claimed human identity.
Retries preserve that identity and canonical intent. Receipts are unique within
organization/principal/request, and operator outcomes are not exposed through an
end-user outcome lookup. The same last-admin and integrity constraints apply.
Require exactly one actor variant in storage, and never attribute operator work
to a convenient administrator. Services cannot initiate permission changes via
this operator variant; user-initiated worker actions retain the requesting user.

Rolling activation requires every running application/worker and every mutating
operator path to understand the same protocol. An old binary that still writes
legacy roles cannot coexist with active new-policy writers without an explicitly
reviewed deployment fence. The activation preview must report this prerequisite;
a database mode bit alone cannot enforce it.

## Executable acceptance cases to add after the gates

| Case | Required observation | Requirements |
| --- | --- | --- |
| Cross-org user/template/assignment IDs, including direct SQL fixture inserts | No foreign relation or private response; database constraints reject invalid links | FR-005/006/008/011 |
| Two saves with the same old revision | One valid change; the other conflicts; no lost update or mixed audit | FR-011/013 |
| Revocation commits before a waiting business/access mutation resumes | Waiting request reloads and denies; state and successful audit remain unchanged | FR-010/011 |
| Business change owns shared gate before revocation | Revocation waits for that transaction; later requests see the new state | FR-010 |
| Concurrent Administrator demotion/deactivation/custom replacement | At least one active explicit Administrator; grant-equivalent custom state never counts | FR-011/015 |
| Write or audit fails after part of a multi-row change | All person/template/assignment state and revisions roll back | FR-011/013/017 |
| Lost acknowledgement then exact retry, changed intent, or revoked caller | No duplicate/replayed change; changed key use conflicts; revoked caller sees no protected receipt | FR-010/011/013 |
| Template or assignment changes after preview | Confirmation conflicts; no silent dependency addition or assignment removal | FR-005/011/015 |
| Malformed stored selection or future catalog/policy version | No runtime normalization, rank fallback or inferred authority | FR-007/011/015 |
| UI/API/report/job/operator write overlapping a permission change | All relevant paths use compatible ordering and current authority; no unbounded external wait under the gate | FR-007/010/018 |

These cases are planned tests, not results. Existing user-lock tests prove only
the current three-role transaction boundary. Approval scope, template lifecycle,
full operation mapping and reviewed data migration still require their own
acceptance evidence; this protocol cannot substitute for them.
