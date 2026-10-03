# Persisted permission state and access-change protocol

Status: proposed Horae implementation mechanics for FR-010/011/013/014/017,
reviewed against `b7e730c` on 2026-10-02. This is not a claim about Harvest's
internal storage. T035/T036 completed the reviewed non-activating storage boundary.
T053–T055 completed the closed local create/delete subset in `template-commands.md`;
full T037/T038 additionally need resolved command predicates and T042.
T006–T009 remain mandatory before replacing legacy
authorization and full-feature acceptance, not before unrelated pure increments.
Read with [data-model](../data-model.md), [migration](migration.md) and the
[current entry-point inventory](current-access.md) and
[operation matrix](operation-matrix.md).

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

## Concrete lock inventory (T042, partial)

Source inspection at `b7e730c`, not a successful concurrency test. Paths below
are under `crates/horae/src/`; `S` means `FOR SHARE`, `U` means `FOR UPDATE`,
and `W` includes implicit row locks from DML. `TS` is the existing per-person
timesheet advisory barrier; `INV` is the per-organization invoice advisory lock.
The right column classifies the proposed organization gate, not the caller's
required permission. Existing transactions do not yet follow this protocol.

| Current path | Observed lock order / important hidden edge | Required integration |
| --- | --- | --- |
| `server_fns/users::begin_user_access_change` | Organization U → actor S → subject W; last-admin count under organization lock | Exclusive; already supplies the common prefix, but has no new-policy revisions/audit |
| `server_fns/projects::{insert_assignment,remove_assignment}` | Actor S → project/user S or assignment DELETE → child cascades → project W through revision triggers | Exclusive from entry; lock parent before child mutation and avoid upgrading a shared parent lock |
| `server_fns/project_creation::save_draft_record` | Actor S → draft U/W | Shared for draft-only persistence; finalization is a separate authorization check |
| `server_fns/project_creation/finalize::finalize_draft_record` | Actor S → draft U → client S → organization S → new project/settings → user/task S and child writes | Exclusive for membership/access effects; move organization first and pre-resolve referenced parents |
| `server_fns/project_creation/editing/save::save` and `editing/associations::save` | Serializable; actor S → project U → client S → child settings → organization S → users/assignments/tasks/members/milestones | Exclusive from entry; changing isolation alone does not reconcile this inverse resource order |
| Project/client/task activation and editing in `server_fns/{projects,clients}` | Resource U/W; project bulk IDs sorted; task links also write parent project through triggers | Exclusive when availability or tracking restrictions change; ordinary field-only writes follow their actual affected set |
| `server_fns/organization::update_org_branding` | Organization U/W | Exclusive even though branding is not a permission edit; no shared-to-exclusive gate upgrade |
| `server_fns/time_entries` | TS shared → task settings/membership S for insert; updates/reorders lock entries U before task-access rows; stop uses conditional entry W | Shared gate before TS; reconcile entry-before-access ordering; preserve stopping one's timer after tracking access is removed |
| `server_fns/approvals::submit_user_week` | TS exclusive → approval U → entries W → approval upsert | Shared organization gate before TS; new coverage contract must preserve empty-cell serialization |
| `server_fns/approvals::{approve_periods,reopen_period}` | Approval U/W → entries W; current approval paths do not take TS; bulk UPDATE order is not an explicit sorted lock order | Reconcile with submission/entry barriers under the final coverage contract, not a second independent lock order |
| `server_fns/invoices::generate_invoice_with_request` | INV → optional actor S → projects S → entries U → fee parents/settings/milestones → occurrence insert/project W trigger → invoice/line writes | Shared gate before INV; prelock existing project parents in their strongest required mode before sources/children |
| `server_fns/invoices/editing::save`, `invoices::transition_invoice` | INV → actor S → invoice U → line writes/invoice trigger or source-entry updates | Shared gate before INV; reconcile source, invoice and occurrence order with generation |
| Budget notification enqueue | Project/settings S → sorted recipient S → notification/outbox inserts and FKs | Bounded service authority under shared gate; reconcile project-before-user ordering and recipient disclosure |
| Local `main::Commands::User(Create)` | Organization lookup → pool INSERT, without the common transaction | Exclusive attributed operator transaction; do not fabricate a session actor |
| Import apply and durable batches | Nonblocking session reservation → transaction → resolved parents/project-task inserts → TS shared → entries/provenance → job report/checkpoint | Exclusive when creating tracking relationships; resolve parent sets in deterministic order; gate each authorized durable batch before domain locks |
| `importers/harvest/streaming::apply` | Opens one transaction after catalog receipt, then waits on `pages.recv()` inside the entry loop | Do not add a gate around this network-paced transaction; reconcile atomic import behavior with bounded preparation/commit first |
| `importers/harvest/account_switch::{gate,change}` | Session reservation → generation U → credential/binding writes → generation W | Nonblocking reservation outside authorization transaction; organization gate before generation lock inside it |
| Initialization/demo seed | Organization table `SHARE ROW EXCLUSIVE` → initial rows | Explicit empty-installation bootstrap exception only; not a runtime permission bypass |

### Trigger and foreign-key closure

Migration `0039_project_editing.sql` adds `invalidate_project_editor` after
INSERT/UPDATE/DELETE on eleven relations: `project_settings`,
`project_private_settings`, `project_tasks`, `assignments`,
`project_member_costs`, `project_member_budgets`, `project_task_settings`,
`project_task_members`, `project_tag_links`, `project_fee_milestones` and
`project_fee_occurrences`. Each changed child row writes its parent project;
unchanged UPDATE rows are skipped. Reparenting writes the
old then new project, not sorted IDs. Migration `0041_invoice_editing.sql`
similarly makes changed line rows write old/new invoices. The BEFORE UPDATE revision
triggers modify the already-targeted parent, not another resource class.

Prelock every affected existing old/new parent, sorted, in the strongest mode
the child operation will require; acquiring S and later relying on a trigger's
W can deadlock competing shared holders. Include cascades: assignment deletion
removes member costs/budgets/task memberships; project-task deletion removes
task settings/memberships. Those child triggers also write projects. Source
entries, fee occurrences, invoice lines and edit receipts carry additional
parent/actor FKs (`0001`, `0002`, `0030`, `0031`, `0039`–`0041`). A manually
sorted explicit SELECT is insufficient if subsequent FK/trigger work adds an
earlier resource lock. New transaction-owned rows are uncontended roots, but
their references to existing parents still participate.

### Authentication and credential writers

Follow-up source inspection at `412035d` distinguishes credential maintenance
from changing a person's effective grants. None of these paths may acquire an
organization gate after already locking a later resource.

| Path | Observed boundary | Integration requirement |
| --- | --- | --- |
| `auth/oidc::resolve_user` | Verified identity lookup, then conditional pool UPDATE of `users.oidc_subject`; no grant/role write. Competing links are resolved by rereading the subject. | Preserve admission and one-time identity binding. If transactional authorization is added, acquire organization before user; keep provider HTTP and session rotation outside the gate. No role mapping or privilege refresh from provider claims. |
| `auth/session`, `auth::make_session_layer` | Session stores user UUID, not effective grants; session-store schema initialization is separate. | Session storage/expiry cannot grant access; every protected operation still reloads active identity/current policy. Do not hold the gate across session middleware persistence. |
| `importers/harvest::complete_connect` → `credentials::store_for_attempt` | Callback checks Administrator again after HTTP, but outside storage transaction. Storage takes import reservation → generation U → binding/credential writes → generation revision. | Pass trusted actor identity into storage and check it under organization gate before generation/credential locks. A post-HTTP pool check alone leaves a check-to-write revocation gap. |
| `credentials::disconnect`, `account_switch::change` | Server wrapper checks Admin; helpers receive organization, not actor. Reservation → generation U → credential/binding writes. | Same transaction-level actor check as connection; preserve generation checks and historical data. Never hold the organization gate while acquiring a blocking external reservation. |
| `credentials::update_tokens` from API import | Import reservation held across refresh HTTP; credential UPDATE follows on the reserved connection, outside domain transaction. | Keep transport refresh distinct from local permission administration. Revalidate bounded execution authority before using refreshed credentials; no gate across HTTP, no token/identity data in permission audit. |
| `credentials::advance_watermark` | Credential row W near the end of inline or durable import's data/report completion transaction. | Include this credential lock before job report/checkpoint finalization in the common hierarchy; preserve atomic watermark/data outcome and never treat it as a user grant. |

Existing wrappers and credential helpers therefore do not yet establish FR-010
for connect/disconnect/account change. This inventory identifies the required
integration; it is not a regression test or a claim those races were repaired.

`scheduler::sweep` also writes `time_entries.notified_long_running_at` and returns
entry notes for a plugin event. It is bounded service work, not a delegated user
edit: keep its marker semantics and explicit plugin authority distinct from time
approval/permission changes. Any organization gate must precede entry locks, with
per-organization bounded selection rather than an unsorted cross-org lock loop.
`notifications::deliver` reads recipient eligibility before sending externally;
reevaluate current recipient/payload access under the bounded read protocol, then
release the gate before mail transport. A queued event is not perpetual recipient
authority, and already handed-off mail cannot be recalled.

`db::run_migrations` runs schema changes, legacy report conversion and constraint
validation; `init`/`seed` use an empty-installation table lock. These are explicitly
coordinated operator/startup operations, not ordinary permission transactions.
The existing destructive reset command is outside the data-preserving transition
and is not authorized by this feature. Deployment fencing must prevent old
writers during activation; adding a row gate cannot substitute for that fence.

### Queue maintenance and report publication

| Path | Observed lock/write set | Required boundary |
| --- | --- | --- |
| `jobs::{enqueue_api,enqueue_csv,retry}` | Generation U → job insert/update → optional upload insert, including organization/job FKs | User-authorized organization gate before generation; requester provenance and current authority are separate from idempotency/connection generation |
| `jobs::cancel` | Conditional job UPDATE in its own statement | User cancellation must authorize in a transaction before job W; never retain a job lock then wait for the organization gate |
| `jobs::{claim,fail_claimed}`, `JobLease::renew` | Separate queue-only autocommit UPDATEs; claim's candidate uses `SKIP LOCKED`, but its preceding bulk recovery UPDATE does not; no domain transaction retained across claim/execution | Bounded maintenance may remain outside the user gate only while it neither changes policy/domain data nor publishes privileged user results. A lease is execution ownership, not user authorization |
| `JobLease::{archive_report,save_checkpoint,complete}` | Archive locks job U → chunk INSERTs → conditional checkpoint/completion on the same job, inside caller's domain transaction | Organization/domain locks first, then job root, chunks and final same-root update. Both current permission and lease fences must pass before commit |
| `jobs::cleanup` | Upload DELETE, then terminal job DELETE in separate statements; job deletion cascades uploads/chunks | Explicit retention maintenance, not authority to execute/retry work. Preserve retention/retry rules and treat missing download chunks as failure, not a complete successful export |
| `jobs::report::download` / `download_body` | Checks Admin before returning a stream; later chunk reads receive org/job IDs, not actor ID | Carry caller provenance and reauthorize bounded reads/tail release under the read protocol; do not hold an organization transaction while waiting for client consumption |
| `jobs::{claim_outbox,mark_outbox_delivered,mark_outbox_failed,stop_outbox_delivery}` | Short outbox-only writes; delivery happens after claim returns | Keep claim/ack outside callback duration. Service capabilities and recipient/current disclosure checks remain required at delivery, not inferred from claim possession |

`run_claimed` also invokes `JobLease::complete` after work returns. A standalone
completion is not exempt from result-publication authorization merely because
its SQL touches only a job row. Keep phase/heartbeat/failure bookkeeping separate
from authorizing import effects and publishing a new privileged result.

`jobs::report::upgrade_legacy_reports` is a specific exception to the queue-only
classification: it locks a job first, then inserts error chunks whose organization
FK (`0027_job_report_error_chunks.sql`) can acquire an organization key-share lock.
This is a job → organization edge opposite to a gated import's organization → job
edge. The safe proposed rewrite discovers a candidate without locking it, then
locks its organization before locking and rechecking the job; if eligibility
changed, it retries without trusting the initial discovery. Preserve conversion
and lease invalidation atomically. Alternatively an offline-only conversion needs
an enforced deployment fence, not a comment asserting startup is exclusive.
`db::run_migrations` calls this converter after the SQLx migration call returns;
that order does not establish exclusion from other running application instances.

### Candidate common hierarchy, not yet activation-ready

1. Acquire any nonblocking import reservation outside the authorization
   transaction. Never wait for external work while holding the organization gate.
1. Acquire the organization gate in the final required mode.
1. Acquire coordination barriers: INV before any affected TS keys; sort multiple
   TS keys consistently. Submission/approval versus ordinary-entry barrier modes
   remain part of the coverage contract.
1. Resolve and lock existing authority/user rows, clients, tasks/tags, drafts,
   then projects, with sorted IDs within each class and strongest modes known
   before child writes. Reading trusted actor identity does not require taking
   its row lock ahead of a lower-ID subject while the gate fences access changes.
1. Project settings/assignments, dependent task/member rows, then milestones.
1. Existing invoice/approval roots, then fee occurrences and time entries in
   consistent class/ID order across all commands.
1. Lines, provenance, notifications/outbox, receipts/audit and any import
   credential watermark write; then job root, report chunks and final same-root
   checkpoint, with existing FK targets already resolved in their parent order.

This is a candidate ordering to test, not a proof that all edges fit it. T042
still requires the finalized operation matrix and production-path validation of
the documented job/credential/identity participation and maintenance exceptions.
The follow-up inventory closes those named source-tracing gaps, not their runtime
implementation. T039 must exercise opposing production paths,
including approval versus entry edits, invoice generation versus edits/voids,
project changes versus invoicing, access changes versus imports, and legacy
report conversion versus gated import completion. A serial
happy-path test or a test that duplicates SQL does not prove this hierarchy.

### Snapshot and read-only incompatibilities

`reports/limits::configure_transaction`,
`server_fns/projects::fetch_project_fee_balances` and
`server_fns/invoices/preview::prepare` use REPEATABLE READ, READ ONLY. A local
PostgreSQL 17.10 diagnostic on the isolated port 55415 returned
`cannot execute SELECT FOR SHARE in a read-only transaction`; no business rows
were changed. The proposed row gate therefore cannot simply be inserted into
those functions. Their transaction mode must permit the gate while their
application operation remains read-only. Preserve snapshot-consistent totals,
size checks and payload reads rather than removing their consistency guarantee.

For REPEATABLE READ/SERIALIZABLE consumers, a snapshot taken before waiting for
the gate can predate a committed access change. Every access change must update
the organization access revision under its exclusive gate; locking a changed
gate row against an older snapshot must fail and restart the entire transaction,
not continue with the old policy. Merely locking the unchanged organization row
while changing a user is not a revision fence. READ COMMITTED consumers must
load policy after acquiring the gate. These are proposed integration requirements,
not claims about current revision writes.

PostgreSQL documents the
[older-snapshot hazard](https://www.postgresql.org/docs/17/applevel-consistency.html)
and [row-lock serialization failure](https://www.postgresql.org/docs/17/explicit-locking.html).
Add production-transaction tests for a read-only-mode failure, a waiting
snapshot-based reader, and a fresh retry observing revocation before accepting
T039/T040. No query-cache, isolation-level or runtime change is made by this
inventory.

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
