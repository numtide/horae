# Permissions discovery

## Timesheet integration evidence (2026-10-04)

- Revalidated clean published `5ec183a` and traced `TimesheetContent`,
  `persist_entry`, the Calendar callbacks, modal and shell `RunningTimer` owner.
  `contracts/timesheet-integration.md` maps their joint integration and tests;
  wiring only the existing scoped list would leave session-owned mutations.
- Fresh official archive documentation closes one earlier unknown: archived
  people retain reporting/invoice history but are absent from Timesheets. Keep
  historical scoped reads intact; this is a separate navigation constraint.
- Fresh running-timer documentation confirms delegated stopping requires edit
  authority, not merely visibility or approval. Fresh Calendar documentation
  specifically permits locked Administrator notes/start/end correction via its
  form while forbidding drag/resize and project/task changes; the general
  Day-only wording must not erase that surface-specific exception.
- Asked one new product question about active managed-project participants with
  no hours in the teammate selector. The user subsequently confirmed A on
  2026-10-04: include active managed-project participants without widening
  visible records or editing rights. The catalog and current docs do not prove
  Harvest's exact custom-grant candidate set. No browser
  tools are loaded; no new reference-account observation or write is claimed.
- No UI, runtime grants, migration or data changed. This is integration research,
  not implemented Timesheet parity or execution of unavailable Spec Kit skills.

## Session identity payload (2026-10-04)

- Source `4c00660`: `auth::get_me` returns `require_user()`'s entire database
  model. A real registered-session regression with populated rates and provider
  subject fails on five extra keys: costs, billable rates, OIDC subject, activity
  and creation time. The initial short-name `--exact` command selected zero tests;
  the corrected filter runs and reproduces the payload failure in 0.53s.
- Traced Sidebar, AdminShell, Projects, Clients, Approvals, Reports, invoice
  recovery and permission-editor recovery. Their used identity/legacy display
  fields close a five-field response contract independently of rate-read policy.
  Returning private fields merely because the requester is the same person is
  unnecessary for every current consumer, including an Administrator.
- Keep the internal authentication lookup and its activity check unchanged.
  The wire identity is not a permission token; existing server mutations still
  reauthorize. Other financial APIs, mutation responses, legacy display gates
  and canonical shell activation remain distinct work, not covered by this fix.

## Project form field and effect inventory (2026-10-04)

- Traced the real full-form load/save, selected catalogs, draft finalization,
  association writes and idempotent receipt payloads at `6b5dbae`. The financial
  evaluator already exists; the missing integration is explicit preservation
  intent and per-field projection, including indirect mode/removal effects.
  See `contracts/project-form-permissions.md` and T158–T161.
- Reopened the current official permission and creation references. Existing
  FR-021/022 settle billable and cost dimensions, not creator self-designation.
  The outstanding question concerns initial explicit selection of already-
  eligible managers under creation authority; no response is recorded here.
- The official [budget guide](https://support.getharvest.com/hc/en-us/articles/360048686811-How-to-set-project-budgets)
  distinguishes limited hour-based progress from financially gated fee budgets,
  and missing budgets from zero limits. It describes historical effects different
  from effective-dated rates. Its legacy-role language does not close the new
  custom-grant write matrix; do not extrapolate all monetary fields from rates.
- The [invoice context guide](https://support.getharvest.com/hc/en-us/articles/360048686671-Getting-project-context-when-invoicing-Fixed-Fee-projects)
  keeps note visibility behind an independent account preference. Horae's accepted
  Administrator-only `admin_notes` is not that shared-note preference. Preserve
  the existing field boundary and track the parity gap rather than coupling notes
  to cost or invoice grants.
- Existing editor tests already cover legacy hidden-cost preservation, forbidden
  cascade removal, post-demotion redaction and nullable budget recovery. Those
  tests permit legacy billable edits and cannot prove canonical read-only grants.
  Reuse their fixtures and add the actual canonical cross-product at integration,
  instead of introducing an unused generic patch abstraction or duplicate tests.
- No new browser observation, permission activation, schema change or Harvest
  write. Spec Kit prerequisites pass; the named skills are absent from this
  worktree, so this is artifact maintenance, not a claimed execution of its suite.

## Workflow identity evidence (2026-10-04)

- Successful fresh Chrome/MCP connection using the existing stdio client; opened
  a dedicated Harvest tab without disturbing the unrelated existing tab. Current
  server reports `1.64.0-alpha-1789764292000`; its click tool uses `target`, not
  the older `ref` argument. The rejected initial click performed no action.
- An empty current-week report still has a teammate available in its detailed
  filter. Archived choices have a separate toggle. This rules out deriving
  every candidate from result rows; it does not prove custom-grant scope.
- Reopened current project/report help and inspected an existing project editor
  without changing fields or saving. The only person is already assigned; the
  account cannot demonstrate additional candidate or non-owner access behavior.
- `contracts/project-people-picker.md` separates confirmed active-person
  assignment eligibility from exact-project authority, rates, general directory
  access and manager designation. The canonical capability mapping is a stated
  inference from the sources and approved catalog, not a Harvest server probe.
- `contracts/people-directory.md` records report-specific open discriminators
  rather than silently substituting current entries or the people directory.
  No new product decision, migration, policy activation or account write occurred.

## Relationship transaction follow-up (2026-10-04)

- Before extending person-management commands, inspected the existing project
  delegation path. Its plain activity reads permit direct deactivation before
  receipt commit, unlike the updated profile/template commands. Three new
  regressions reproduce actor activity loss, added-manager activity loss and
  inherited READ ONLY failure. Reuse the existing administration setup and SHARE
  activity fences; preserve retained/removed-manager eligibility and project
  NOWAIT behavior. This is FR-010/026 enforcement, not a new product predicate.
- Rechecked the official [people-assignment guide](https://support.getharvest.com/hc/en-us/articles/4422314817677-Making-people-assignments-for-Managers),
  [teammates API](https://help.getharvest.com/api-v2/users-api/users/teammates/)
  and [archive guide](https://support.getharvest.com/hc/en-us/articles/360048687311-Archiving-deleting-and-restoring-people).
  They establish Administrator assignment authority and retained historical
  reporting after archival, but do not settle adding archived people to a
  responsible person's scope. Asked one product question: restrict new edges to
  active endpoints while retaining/removing existing edges, or allow new edges
  involving archived people. No answer has been accepted; FR-027/028/029/031
  remain unchanged. This is not a reason to repeat their confirmed decisions.
- No current Harvest browser connection or account mutation occurred. The source
  guides do not establish custom-profile enforcement. The person-management
  command and its inactive-endpoint predicate remain unfinished; no temporary
  default or new archive/restore side effect is introduced.

## Permission editor integration (2026-10-04)

- Decision: connect existing commands through the session-derived editor contract,
  without enabling canonical policy. FR-002 permits this confirmed integration.
- Independent source review found plain actor/target/survivor activity reads and
  inherited READ ONLY/unbounded waits. Retain SHARE activity locks under the
  organization gate with explicit bounded settings. Existing SHARE stays compatible.
- Share preview/save effects; preserve canonical intent serialization and authorize
  before replay. Display DTOs never become trusted authority. Reject duplicated
  preview policy, implicit keep-access grants, new preview storage and activation.

## Authenticated audit delivery (2026-10-04)

- Decision: expose the existing single-receipt historical projection through a
  session-authenticated server function, with separate shared wire DTOs. FR-013
  already resolves Administrator-only visibility; this requires no new product
  choice. Keep history browsing and active policy cutover separate.
- Independent source review found plain actor activity could race direct user
  deactivation, and inherited READ ONLY defaults reject the current row locks.
  Use organization SHARE then requester SHARE, explicit READ COMMITTED/READ WRITE
  and transaction-local bounded waits. Historical subjects remain plain reads.
- Alternatives rejected: legacy `require_admin`, inferred identity from all
  grants, untyped JSON, deserializable trusted permission state, new pagination
  policy, schema changes and a new transaction abstraction. Existing receipt
  decoder and one registered-route harness cover the relevant boundary.

## Invoice evidence follow-up and source preflight (2026-10-04)

- Fresh independent primary-source review did not close new-model mixed-project
  or unlinked-invoice authorization. The current permission guide establishes
  distinct read/draft/manage grants, not the document-level scope predicate.
  [Single-invoice creation](https://support.getharvest.com/hc/en-us/articles/360048686371-How-to-create-a-single-invoice)
  supports optional project links on manual documents. The
  [overview's project filter](https://support.getharvest.com/hc/en-us/articles/12389698137869-Invoices-overview)
  is not evidence of the authorization predicate. Do not infer either from
  older Manager descriptions. One mixed-project coverage question is pending;
  its answer will not implicitly settle unlinked invoices or source disclosure.
- Browser evidence limitation: the existing MCP client initialized, but the
  tab-list request timed out. No current account observation or account write
  occurred. Existing snapshots remain historical evidence only.
- Decision: advance the already specified read-only M01/M07/M08 diagnostics,
  not another legacy behavior repair or an unreviewed invoice predicate.
  `contracts/migration-preflight.md` defines a fixed numeric projection,
  organization/actor fencing and one statement snapshot. Independent review
  found inbound approval references can occupy another tenant's subject/period
  uniqueness slot; include every cross-tenant approval touching the inspected
  organization, without exposing any foreign identity.
- Schema/source review: the shared entry-state enum permits open/invoiced
  approvals, but current approval writers use submitted/approved or deletion.
  Count unexpected states, non-approved attribution and missing approved
  attribution separately. An inactive or demoted historical approver with
  complete local attribution is not missing provenance. Job NULL requesters
  remain unknown; group by actual pending/terminal state without choosing their
  execution/retry policy. No diagnostic repairs data or proves full readiness.
- Alternatives rejected: per-row payloads, a ready-to-activate flag, guessed
  historical actors, role mappings, automatic cleanup and interpreting all job
  kinds as user imports. No new dependency, schema or public endpoint is needed.

## Next canonical integration target: invoice permissions (2026-10-04)

- Decision: next refine OP21–OP24 into an executable canonical invoice contract,
  rather than treating another isolated legacy guard as six-profile delivery.
  Full T006 still blocks replacing active authorization; T007 separately gates
  approved migration/activation. Constitution 1.1.0 already includes the target,
  so T008's remaining work is reconciliation and transition design, not another
  amendment adopting six profiles.
- Source inspection confirmed `generate_invoice_with_request`,
  `transition_invoice` and `invoices/editing::save` acquire the invoice advisory
  root before an organization gate. `fees::scheduled_fees` takes project/settings
  SHARE for generation; `prepare_fees` then inserts occurrences whose migration
  0039 trigger writes the project. Their common hierarchy needs design and race
  tests; changing the initial lock alone is not a complete fix.
- Fresh [Harvest permission documentation](https://support.getharvest.com/hc/en-us/articles/44171549176077-Permissions)
  confirms distinct managed invoice read, managed draft creation/editing, managed
  invoice management and organization-wide read/manage grants. This does not by
  itself establish the complete mixed-project/manual-line authorization rule.
- [Invoice/project linking](https://support.getharvest.com/hc/en-us/articles/360048686631-Linking-invoices-to-projects)
  distinguishes linked line projects from underlying tracked sources: relinking
  does not move the original entries. A project's displayed invoice amount is
  only its linked portion. Preserve that distinction when researching authority;
  one visible portion is not proof of access to the entire invoice.
- [Fixed-fee context](https://support.getharvest.com/hc/en-us/articles/360048686671-Getting-project-context-when-invoicing-Fixed-Fee-projects)
  separately conditions opening related invoices on invoice permission and keeps
  notes visibility independent. Its role terminology is older; do not infer the
  new custom-profile predicate from it. Restricted-user enforcement and manual
  invoices without project links remain unverified.
- Next: verify the full read/draft/status/source matrix and its transaction
  contract before implementing the internal canonical consumer. Do not activate
  policy or choose mixed-scope behavior by guess. If reference investigation
  leaves an indispensable product choice, ask that one choice; meanwhile a
  read-only disposable migration preflight can advance T007 without selecting
  role mappings or repairing records. Previously asked inactive-manager and
  historical-unknown-requester questions remain open and must not be repeated.

## Budget email preparation design (2026-10-04)

- Decision: organization SHARE, current recipient SHARE, project SHARE, then
  outbox UPDATE; use separate fresh checks after waits. Preserve the existing
  recipient predicate while preparing one message, then commit before transport.
- Independent review confirms migration 0039 child-to-parent triggers make
  settings/assignment locks after the project unsafe. Parent fencing with a
  later predicate avoids that cycle. Enqueue inserts new notification/outbox
  rows rather than updating the claimed row; no additional inversion was found.
- Alternatives rejected: a pre-wait lease check can expire while waiting;
  outbox SHARE followed by an UPDATE upgrades competing readers; terminalizing
  outside the transaction can fail a same-token replacement payload. Late
  outbox UPDATE with in-transaction terminal rejection avoids those races.
- Notification identity columns are not immutable by schema. Require exact
  discovered/locked identities in the final read; retargeting or a newly present
  notification skips this attempt instead of using unlocked resources. Missing
  settings with an unchanged notification is real ineligibility, not drift.
- PostgreSQL's [READ COMMITTED snapshots](https://www.postgresql.org/docs/17/transaction-iso.html)
  and [row lock compatibility](https://www.postgresql.org/docs/17/explicit-locking.html)
  support this ordering. Its [current-time functions](https://www.postgresql.org/docs/17/functions-datetime.html#FUNCTIONS-DATETIME-CURRENT)
  distinguish transaction-start `now()` from actual `clock_timestamp()`; the
  final lease margin must use the latter. Local tests will verify the exact path.
- No new product decision is required. This closes the local preparation design
  in `contracts/budget-email-authority.md`, not OP37 or full canonical activation.

## Next service-reader integration candidate (2026-10-04)

- Evidence: `notifications::deliver` first reads an outbox claim and then reads
  recipient/payload through separate pool queries. The current recipient rule
  requires active recipient/project, enabled alerts and Manager/Admin or an
  eligible lead/admin/creator assignment. Existing `notifications/tests.rs`
  covers stale claims and recipients revoked before delivery, not concurrent
  revocation during preparation. The main review confirmed both production
  queries and their lack of a shared authorization boundary.
- Existing requirement: feature 011's data model requires eligibility at enqueue
  and delivery; `contracts/permission-state.md` requires bounded current reads
  and release before external transport. No new Harvest predicate is needed to
  integrate that existing rule. OP37's canonical/custom recipient mapping is
  still separate and must not be inferred from legacy roles.
- Next design task: refine a preparation transaction that gates organization,
  recipient and project/settings in the reviewed hierarchy, refreshes eligibility
  after waits and rechecks claim ownership/remaining lease using current time.
  Return only authorized message/recipient after releasing locks; keep sendmail,
  acknowledgements, five-attempt policy and message identity outside that gate.
  Test concurrent revocation, claim expiry, one-connection reuse and no locks
  across a blocked stub transport. This is research, not an implemented fix.
- Do not substitute this for import-worker activation. Historical NULL
  requesters, retry/restoration and final service authority remain open;
  `original_requester_id` is still provenance, not executable permission.
  Budget enqueue's own lock integration also remains separate.

## CSV cursor transport feasibility (2026-10-04)

- Decision: continue the single-connection READ COMMITTED cursor design rather
  than holding authority locks across browser backpressure or borrowing a second
  connection. This is a validated transport candidate, not CSV authorization
  acceptance or permission to activate canonical policy.
- Evidence: SQLx 0.8.6's `Executor::describe` prepares/describes but does not
  execute `DECLARE`. A temporary checked `FETCH NEXT` probe produced `PgRow`,
  not the requested struct (E0609 on its field). A checked `DECLARE` plus a
  SECURITY INVOKER `SETOF record` fetch helper and static caller column list
  passed the runtime proof: exact i64 extremes, UUID/date/enum/bool/null/text,
  cursor survival after savepoint rollback, released row locks, frozen source
  rows across a concurrent edit and a fresh subsequent READ COMMITTED read.
  The helper needs a reviewed migration before production use; none was added.
- Caveat: both SQL statements are checked, but the anonymous cursor row layout
  is matched against the helper's caller column list at runtime. Each actual
  export needs projection/decoding tests; this is not compile-time proof that
  arbitrary cursor projections match that list.
- Rationale: PostgreSQL documents [cursor survival and position](https://www.postgresql.org/docs/17/sql-rollback-to.html)
  and [release of post-savepoint locks](https://www.postgresql.org/docs/17/explicit-locking.html).
  Declare the source outside later rolled-back authorization savepoints. Reserve
  channel capacity before each current-authority check, release its locks and
  enqueue synchronously. Capture invoice metadata/lines in one cursor query.
- Alternatives: two connections break size-one pools and risk pool deadlock;
  [WITH HOLD materializes at commit](https://www.postgresql.org/docs/17/sql-declare.html).
  A fetch helper buffers its complete invocation ([PL/pgSQL returns](https://www.postgresql.org/docs/17/plpgsql-control-structures.html)),
  so its first call must return at most one row. `FETCH 1` is the simplest
  baseline; batching must have a decoded-byte bound as well as a row cap.
- Independent review rejects `pg_column_size(record)` as that bound: storage
  [reflects compression](https://www.postgresql.org/docs/17/functions-admin.html),
  and [expanded records can retain compressed fields](https://github.com/postgres/postgres/blob/REL_17_STABLE/src/backend/utils/adt/expandedrecord.c).
  A candidate private `export_bytes` projection sums logical `octet_length`
  for every variable-width returned field plus fixed overhead, using bigint
  operands before addition. Reject invalid weights; return the threshold-crossing
  row before stopping, retaining the existing one-oversized-record allowance.
  No JSON conversion or added library is necessary. This batching contract and
  its compressed/oversized/threshold/first-row tests remain to implement.
- Next: refine the streaming contract/tasks, including both revocation orders,
  empty/header/totals-tail authorization, exact existing CSV bytes, cancellation,
  one-connection reuse and real-cookie checks. The five-second statement timeout
  would apply per fetch, not to the complete source query; retain the outer
  sixty-second download bound and test/document that distinction.

## Fresh materialized project exports (2026-10-04)

- Decision: READ COMMITTED organization/actor gates followed by one bounded
  statement for project size and payload, then captured-parent locks and a
  separate current-scope statement after rendering. See `contracts/project-exports.md`.
- Rationale: source inspection and independent review confirm legacy assignment,
  editor and finalization writes lock the organization without changing its
  revision. Snapshot-visible parent locks miss winning scope expansion and new
  projects. Migration 0039 supplies the final parent fence for direct child writes.
- Alternatives rejected: RR with only existing/candidate parent locks misses
  new scope; separate RC size/payload queries mix snapshots; an organization
  revision trigger would broaden writer changes and risk late lock upgrades;
  combining final parent locks and access joins can use pre-wait relationships.
- Sources: PostgreSQL [READ COMMITTED](https://www.postgresql.org/docs/17/transaction-iso.html#XACT-READ-COMMITTED)
  supplies a fresh snapshot per statement; [CTE materialization](https://www.postgresql.org/docs/17/queries-with.html#QUERIES-WITH-CTE-MATERIALIZATION)
  supports reusing one bounded result. Inspected migrations 0035/0039,
  `db::lock_organization`, project writers and report loaders. No reference
  account writes or new Harvest grant assumptions are needed.

## Materialized export authority (2026-10-04)

- Decision: reuse the manager prelude for entries/invoice/PDF materialization,
  retain session actor and organization IDs, and check again after bounded
  rendering before response release. Preserve all size/query/render limits.
- Rationale: current `require_manager` discards actor identity, while
  `limits::{entries,invoice,pdf}` accept organization alone. Independent code
  research confirms these are the three manager-only materialized consumers.
  The renderer may run for 30 seconds after reading; a fresh final check prevents
  a revocation during that work from releasing an already-generated file.
- Alternatives rejected: authorization only before rendering misses that window;
  holding locks through rendering/client delivery delays revocation; configuring
  READ ONLY after the locking helper or changing isolation after its queries is
  incompatible with the prelude. Keep the streaming configurator's behavior and
  Member project exports separate. Share only existing deadline setup.
- Evidence: inspected `reports.rs`, `reports/{limits,bounded,streaming}.rs` and
  `server_fns/snapshot.rs`; reuse the verified PostgreSQL snapshot/retry sources
  below. No new Harvest operation or product policy is assumed. The final check
  defines release authorization, not a guarantee after bytes have been released.

## Invoice editor snapshot boundary (2026-10-03)

- Decision: reuse `server_fns::snapshot::manager` for editor load/review, retaining
  their session-derived identities, queries, exact revision check and evaluation.
- Rationale: independent code research confirms only plain invoice/revision/line
  reads follow authorization. No invoice advisory or row lock is requested, so
  the new organization→actor order adds no cycle with mutation actor SHARE or
  organization FK KEY SHARE. Existing `editing::lock_actor` also serves save,
  generation and status mutations and must not be replaced globally.
- Alternatives rejected: retaining inherited READ ONLY can fail at actor SHARE;
  retrying only the actor query cannot refresh an old snapshot; changing save or
  financial logic is unnecessary. Reuse the PostgreSQL sources and tested retry
  reasoning below; no new Harvest behavior or product exception is inferred.
- Evidence/limits: inspected `invoices.rs`, `invoices/editing.rs`,
  `invoices/balances.rs` and existing draft-edit tests. New editor-specific
  fixtures must not consume the fees used by preparation/balance fixtures.
  Full canonical policy, mutation integration and report streaming remain open.

Status: incremental planning; independent scope foundation specified, full policy research incomplete. Baseline `9301112`; inspected 2026-09-30.

Current decision update, 2026-10-02: the user explicitly resolved C01 for Horae.
Deleting a template retains every assignee's current grants/scope as a
person-specific configuration, removes future template availability and explains
that effect before confirmation. Revocation remains a separate explicit change.
Rationale: deleting a reusable configuration should not silently revoke existing
access. The rejected alternative is an automatic Member downgrade. Historical
investigation entries below retain the evidence available then; Harvest's actual
deletion result remains unverified, while Horae's behavior is now approved.

## Current Horae boundaries

### Materialized snapshot reauthorization, 2026-10-03

- Decision: start REPEATABLE READ READ WRITE, organization SHARE then actor
  SHARE, and retry the whole prelude on 40001 before any business read. Preserve
  current Manager/Admin boundaries for fee balances and invoice preparation.
- Rationale: legacy user role/activity writers lock but do not update the
  organization tuple. The actor lock detects that committed change even when
  its snapshot predates the wait. Canonical revision updates instead conflict
  at the organization row. One connection suffices; no generic replay callback
  or new dependency is needed. See `contracts/manager-snapshots.md`.
- Sources: PostgreSQL [Repeatable Read](https://www.postgresql.org/docs/17/transaction-iso.html#XACT-REPEATABLE-READ)
  requires a transaction restart when a locked row changed after the snapshot;
  [SET TRANSACTION](https://www.postgresql.org/docs/17/sql-set-transaction.html)
  disallows these locks in READ ONLY and changing isolation after a query.
  Source inspection traced current readers, user writers and export consumers.
- Independent research identified a separate CSV hazard: the shared transaction
  configurator serves browser-paced streams. Do not insert authority locks into
  it. Member project exports also need a scope fence because legacy membership
  changes do not update the actor/organization tuple. Editor load/review already
  lock the actor but still need organization-first integration separately.
- Alternatives rejected: dropping snapshot isolation, acquiring READ COMMITTED
  locks then switching isolation, a second authorization connection, retrying
  within an aborted transaction, or broad changes to the shared export setup.

### Approved financial field gates, 2026-10-03

- Decision: implement FR-021/022 with typed Person/Project/GlobalTask ownership,
  action-specific billable scope and independent organization-wide cost checks
  in the pure core. Reuse canonical selections and `AccessScope::covers`.
- Rationale: the user already resolved C03/C04. No further reference observation
  is needed to implement those approved local rules. An all-read grant must not
  widen a managed-write grant; project authority must not leak person defaults.
- Alternatives rejected: a generic person-or-project scope union, profile-name
  checks, Administrator-only costs, report grants opening ordinary financial
  fields, a new policy dependency, or premature runtime activation.
- Independent adversarial design/code review found no blocker and recommended
  explicit read-prerequisite revocation coverage; the new test covers denial of
  both actions after removing billable/cost read prerequisites. Producer-side
  field classification and concurrent consumer enforcement remain T014/T015.
- Fresh official documentation still does not establish the exact custom-grant
  mapping for project archive/restore. The [archived-resource guide](https://support.getharvest.com/hc/en-us/articles/4408222060301-Unlocking-time-and-expenses-if-the-project-task-or-person-is-archived)
  does establish that restoring a project requires restoring its archived client
  first; client restoration is not an implicit side effect. Independent locks
  can remain after restoration. Record this OP13 state prerequisite without
  inventing lifecycle authority or claiming a restricted-user browser test.

### Import disposal after nested transaction cancellation, 2026-10-03

- Decision: drain existing responses in shared `release_import`, tolerating only
  invalid-savepoint errors during disposal, then full ROLLBACK, advisory unlock
  and close. Do not continue import work on that session.
- Evidence: pinned `sqlx-postgres-0.8.6/src/transaction.rs` decrements depth only
  after awaiting RELEASE; its drop rollback can therefore target a savepoint
  the server already released. `connection/mod.rs::wait_until_ready` consumes
  one ErrorResponse before returning; another flush continues draining without
  submitting another command. A dropped BEGIN at depth zero is another reason
  to explicitly roll back server state before releasing the reservation.
- Rationale: CSV row savepoints, API parent/row savepoints and API preview
  transactions share the same cleanup boundary. The recorded cancellation
  failure is not evidence that all driver failures can be ignored.
- Alternatives rejected: suppress all cleanup errors, unlock before rollback,
  rely on eventual socket closure for immediate retry, or stop cancelling SQL
  consumers while they wait on locks. A library upgrade or protocol proxy is
  unnecessary for this disposal fix. The deterministic fixture constructs the
  driver divergence; it does not claim a timed reproduction of the driver race.
- Independent read-only review confirms the finite response-drain approach and
  recommends full ROLLBACK even without a missing-savepoint response. Execution
  authority, legacy requesters and retry delegation remain separate contracts.

### Original import requester, 2026-10-03

- Decision: retain `original_requester_id` only on insertion by the updated
  session commands. Its nullable tenant FK preserves unknown historical authors
  without selecting execution authority. Use ordinary NO ACTION references.
- Rationale: `start_api`/`start_csv` are the only production enqueue callers and
  already hold current actor authority in the insertion transaction. Storing
  that fact does not depend on deciding who may reauthorize old work. Production
  enqueue functions require a UUID; private helpers preserve test-only legacy
  insertion with NULL. No DTO or payload changes are necessary.
- Alternatives rejected: infer an administrator for historical rows, add the
  requester to idempotency identity, replace original authors on retry, cascade
  job deletion with a person, or erase known attribution with SET NULL. These
  would change history or lifecycle semantics rather than preserve provenance.
- Evidence: `jobs.rs` enqueue/conflict/retry/claim/retention paths;
  `server_fns/importers/commands.rs`; current user lifecycle; tenant key in 0042
  and comparable audit references in 0043. The RED command test returns NULL
  instead of its authorized actor. No claim is made about Harvest's internal
  schema. Mixed old/new binaries may continue inserting NULL until all writers
  are upgraded; worker activation still needs its reviewed deployment fence.
- Review: production inserts derive the actor from the existing session-bound
  transaction; conflict updates never assign it. The new FK checks the already
  locked current actor, so adds no reversed acquisition. Historical report
  conversion must compare its old fields separately from new NULL provenance.
  Both the direct-command and registered-HTTP tests check that client-supplied
  authors cannot override the session. This review does not establish worker
  execution authorization or resolve legacy-job policy.

### Durable CSV preparation, 2026-10-03

Source inspection for T042 finds no retained initiating actor in the current job
row, claimed job, lease or import checkpoint. Enqueue/retry receive trusted actor
identity but do not persist it. A lease token or Harvest connection generation
is not user authority. Historical-job policy remains an explicit pending choice;
this prerequisite does not choose it or infer an actor.

API input pages are prepared before their write transactions; durable CSV used
to await its parser inside each 500-record transaction. T083–T085 move only CSV
preparation outside SQL while preserving absolute checkpoint offsets and its
reserved import connection. The real-parser/backend-PID regression reproduced
that open transaction before the change. Additional buffering is bounded by
500 normalized records, not by a measured RSS ceiling; existing source admission
limits remain unchanged. The independent review found no correctness issue in
the production diff; runtime regression verification remains required.

### Permission storage

Current storage clarification, 2026-10-03: the user selected trimmed,
case-insensitive organization-local template-name uniqueness (FR-032). Harvest's
creation comparison remains unverified; its Users API describes name lookup and
current matching labels, not immutable application provenance. The retained
editor's `vi` handler supplies observed trimming/blank rejection. The bounded
storage design in `contracts/permission-storage.md` therefore stores canonical
grants, explicit administrative identity and source separately, without a saved
computed-label column. Unknown Harvest internal storage does not block it.
The adversarial review rejected coupling administrative identity to source shape
or full-grant equality. Corrected: identity survives C01 detachment and only
authorized transitions may change it. No remaining high/critical finding in that
bounded design review; this is not full feature analysis or runtime verification.

The expanded [entry-point inventory](contracts/current-access.md) now records current checks, redaction differences, jobs/plugin trust boundaries and migration deltas. The [reference evidence register](contracts/harvest-evidence.md) separates confirmed browser observations from documentation and unverified custom behavior.

- `crates/core/src/types.rs`: organization roles are Admin/Manager/Member. `OrgRole::is_manager_or_above` describes organization-wide billing and approvals. ProjectRole is a separate legacy concept.
- `crates/core/src/state.rs`: approval and invoice transitions currently accept organization roles. The new model must preserve valid state transitions while changing authorization deliberately.
- `crates/horae/src/server_fns.rs`: `require_user` reloads active identity; `require_admin` and `require_manager` enforce broad role gates.
- `crates/horae/src/server_fns/users.rs`: active-user listing redacts rates below manager, while user creation and role/activation changes require administrator authority. The organization lock protects concurrent last-administrator changes.
- `crates/horae/src/server_fns/approvals.rs`: submissions cover a person's configured week. A project-scoped permission cannot safely stand in for authority over that entire submission.
- `crates/horae/src/harvest/auth.rs`: the compatibility surface resolves active identity and organization role separately from fullstack server functions. Updating only the UI or shared helpers would not establish complete parity.
- Search also identifies role-dependent consumers in reports/downloads, imports, jobs, notifications, navigation and plugins. This is an entry-point discovery list, not a completed security audit.
- Feature 011 explicitly separates project-manager designation from organization promotion and protects cost/private-note data. Any changed boundary requires an explicit replacement contract and regression tests.

## Reference and design conflicts

The official [Harvest permissions reference](https://support.getharvest.com/hc/en-us/articles/44171549176077-Permissions), checked 2026-09-30, documents a progressively deployed six-profile model and optional custom profiles/per-person adjustments. Only administrators administer role assignments. Its permission categories distinguish self, managed work and account-wide access; profile names alone are insufficient to copy those semantics.

This is documentation evidence, not a fresh interactive test of the user's account. Accounts may still have the outgoing model. The user's 2026-09-30 parity instruction resolves FR-004 and FR-009 in favor of custom permissions and project-scoped approval. They are no longer unanswered product choices.

The current [approval reference](https://support.getharvest.com/hc/en-us/articles/360048181832-Submitting-and-approving-timesheets), checked 2026-09-30, explicitly describes approval limited by project/client filters, even when grouping by person. It also describes weekly locks and editing submitted work before approval. The exact interaction of filtered approval with empty days and new work needs direct verification; do not infer the entire lock scope from the selected approval rows.

The older general approval article restricts withdrawal to administrators, whereas the new permissions article includes scoped withdrawal for People Admin and Executive Manager. Project Managers cannot withdraw approvals; that is not a prohibition on approving. Use the current permissions experience as the target rather than copying the old three-role rule. Assignment editing/promotion and custom-template update behavior still need evidence. None of these investigations authorizes replacing parity with simpler existing Horae behavior.

### Flexible approval evidence

The current [flexible timesheet approval reference](https://support.getharvest.com/hc/en-us/articles/39974542812429-Flexible-timesheet-approval), read 2026-09-30, refines the older weekly article:

- Submissions and approvals can cover dates shorter than a week; submitted work remains editable before approval.
- Managed-project or managed-person authority can cover work; one eligible approver suffices. Manager self-approval is disabled by default and configurable.
- Approval locks selected dates and project coverage, including cells without entries. Whole-submission coverage is possible with sufficient authority and qualifying filters.
- Withdrawal from Approval uses its date/filter scope. Withdrawal from Day/Week unlocks the whole week. There is no rejection transition; changes can be requested and work edited/resubmitted.

Independent locks must still apply after withdrawal; see [unlocking time and expenses](https://support.getharvest.com/hc/en-us/articles/360048687491-Unlocking-time-and-expenses). These are documentation findings, not fresh browser observations. The precise arbitrary-custom-profile approval predicate, newly created projects after whole-submission approval, overlapping interval splitting and post-withdrawal submission state remain unverified.

### Custom-profile evidence

The new permissions reference documents administrator-managed templates, an immutable Member floor, automatically included prerequisites/dependent removal, unique names of at most 100 characters, at most 50 templates, per-person difference indicators and deletion that preserves existing permissions. Applying a profile requires saving the person. Read-only inspection of the live editor has now established its profile defaults, dependency traversal and classification rules; see [reference profiles](contracts/reference-profiles.md). Saved enforcement, name case sensitivity and template update/propagation still require direct verification. The live catalog also identifies approval-specific grants and report/rate-scope differences from the article; those findings supersede guesses based on its role summaries.

### Import and profile-write boundaries

The [API application evidence](contracts/reference-profiles.md#api-application-evidence)
adds documented reapplication and assignment-removal cases to T006/T007. It does
not establish template-update propagation or resolve the UI persistence gate.

Current code inspection on `757f43d` confirms separate integration boundaries:

- `importers::harvest::api_source::ApiUser` consumes only identity fields.
- `importers::harvest::resolve::resolve_user` matches existing same-organization
  users; it does not provision users or write their permissions. Missing and
  ambiguous identities fail instead of selecting an arbitrary account.
- `harvest::router` exposes only GET routes; `HarvestUser` currently serializes
  `is_admin`, not a six-profile/custom-grant representation.

FR-014/017 require preserving that import boundary through migration. External
metadata must not become a local grant merely because the importer can parse it.
The compatibility serializer needs an explicit reviewed projection when the new
model is implemented; adding write endpoints is not required to enforce reads.
T019 must exercise repeat imports and identity linking against existing custom
grants without changing them. This is a required future regression, not a test
already run or permission-policy activation.

### Independent foundation decisions

- Decision: model record coverage as an explicit union of own, managed-person, managed-project and organization scopes. Rationale: FR-006 and the reference distinguish these dimensions. Rejected: inferring authority from role ordering or ordinary project membership.
- Decision: evaluate trusted facts in the existing pure core with borrowed assignment slices. Rationale: correctness is independently testable without storage or new dependencies. Rejected: introducing a policy-engine dependency or a new crate.
- Decision: keep the foundation disconnected from runtime authorization until the full matrix and migration are reviewed. Rationale: a scope predicate does not define capability grants, locks or transactional revocation. Rejected: swapping existing guards piecemeal and claiming parity.

`design/project/app/08_Settings.dc.html` displays the six profiles. `09_Workspace.dc.html` explicitly describes three fixed roles with no per-person permissions. The new approved matrix must become authoritative for both screens; neither inconsistent mockup can silently settle the policy.

The baseline constitution named three organization roles. The 2026-10-01 amendment
to 1.1.0 in this branch records the approved extension and its migration gates;
it grants no runtime access. The exact operation matrix and dependent feature
acceptance remain to be reconciled before cutover.

## Workflow

### Import job command authority review — 2026-10-03

Traced all production enqueue/cancel/retry/status/history callers. Authenticated
wrappers supply the actor from the session; no public actor parameter is added.
Queue SQL now participates in the caller-owned authorization transaction, and
the returned status is read on that same connection before commit. Pool-level
enqueue/retry adapters are test-only and share the production SQL.

Adversarial self-review covers replay and no-op authorization, actor-only and
organization wait races, foreign IDs, malformed CSV, partial insert failure and
single-connection exhaustion. The real HTTP fixture revokes the Administrator
when the upload body is first read: admission alone cannot authorize acceptance,
and holding an actor lock across body reading would prevent this fixture from
completing. No test-only hook is present in production code.

Organization SHARE precedes actor SHARE and the existing generation/job/upload
order. Foreign-key KEY SHARE remains compatible; none of these operations
upgrades the organization lock or holds it across external HTTP or client-paced
upload consumption. A dedicated test retains the import reservation while both
authorized submissions execute, guarding against accidentally serializing queue
acceptance behind worker execution. The worker does not inherit the caller's
identity. Worker effect authorization and bounded report downloads remain open;
this review is not an independent full-feature review or proof of T042 closure.
No new Harvest product behavior is inferred by this repair.

### Harvest connection authority review — 2026-10-03

Reviewed the complete human connection family, not only the disconnect endpoint:
`complete_connect`/`store_for_attempt`, `harvest_disconnect`/`disconnect`, and
`harvest_change_account`/`account_switch::change`. Wrappers keep admission checks;
trusted actor identity now reaches the final transaction. A shared guard checks
active same-tenant Administrator authority after organization SHARE, then holds
user SHARE through the unchanged generation/binding/credential writes. Explicit
READ COMMITTED prevents a pool's older snapshot from hiding committed revocation.

Lock review: import reservation is nonblocking and precedes the transaction;
token exchange and account lookup finish before that reservation. No gate spans
HTTP or another pool acquisition. Generation insertion's organization KEY SHARE
is compatible with SHARE; existing queue/retry generation holders need no
conflicting organization write to release it. Import token/watermark writers
remain excluded by the same reservation, with their separate service-authority
work still open. No credential-management helper bypass was added for fixtures:
each fixture supplies a concrete Administrator, and production writers run in tests.

The tests cover completed revocation, both wait locations, writer-first retention,
first-connect/no-op denial and late-write rollback. Typed forbidden mappings
strip private error context. This is a focused self-review, not an independent
full-feature adversarial review or proof of complete runtime parity. No public
Harvest behavior was inferred or changed by this local invariant repair.

### Locked-entry corrections and branding revocation — 2026-10-03

- Fresh dedicated time/expense editing documentation distinguishes privileged
  correction/deletion from unlocking. Corrected the overly broad denial in
  `contracts/company-locks.md`; source links, field/surface limits and remaining
  new-model/company-lock/financial questions are recorded there. No account
  mutation or runtime correction policy is inferred from this discovery.
- Source tracing of `organization::update_org_branding_record` found a separate
  concrete FR-010 gap: the helper receives no actor, so the wrapper's Manager
  check can become stale while waiting for organization UPDATE. Retain existing
  roles and post-commit events; pass identity and recheck active same-tenant
  authority inside the transaction after its first lock.
- The bounded path touches only organization and actor rows, in that order;
  branding columns have no child-write trigger. Existing organization UPDATE is
  retained, not upgraded after actor/resource locks. Explicit READ COMMITTED
  prevents a connection's older snapshot default from preserving stale authority.
  T071–T073 require production-helper denial, actual lock waits, no-op/privacy,
  rollback and existing field/idempotency regressions before publication. This
  self-review is not independent full T042 approval or six-profile activation.

### Operation mapping and transaction inventory — 2026-10-02

- Follow-up at `412035d`: fresh reading of the public permissions guide explicitly
  establishes creation as well as editing under the managed-project draft-invoice
  permission. OP22 now records this documented allowance instead of retaining an
  unnecessary creation unknown. Mixed-source financial scope and saved non-owner
  enforcement remain open; no browser mutation was performed.

- Completed the named source-tracing gaps for OIDC linking, session persistence,
  credentials, queue maintenance, report publication and outbox delivery. The
  [state contract](contracts/permission-state.md) distinguishes short queue-only
  maintenance from user-authorized execution and protected output. Session or
  lease identity does not carry grants; external work stays outside row gates.

- Independent tracing found a high-priority integration risk: legacy report
  conversion locks a job before inserting chunks with an organization FK, the
  reverse of gated imports. Corrected the proposed ordering and specified
  nonlocking candidate discovery followed by organization-first locked recheck.
  Startup invocation is not an enforced offline boundary. This is a design
  correction for future integration, not a repair/test of running code.

- Credential helpers currently lack an actor parameter and the OAuth callback's
  post-HTTP role check occurs outside their transaction. Download chunk reads
  likewise lose caller identity after initial authorization. Both require current
  authorization at their durable write/output boundaries under T039/T040/T015;
  no new legacy-only implementation detour is selected.

- Decision: map actual public symbols and non-server-function delivery paths in
  [operation-matrix.md](contracts/operation-matrix.md), with unresolved predicates
  explicit. Rationale: profile names and UI categories omit compound effects,
  picker projections, mixed invoices and delegated execution. Rejected: treating
  a catalog key or legacy Manager check as the complete operation policy.

- The map covers 80 public async symbols at `b7e730c`. Independent review found
  no high/critical draft contradiction, but identified omitted authentication
  routes and one-sided wording about invoice-draft creation. Added those routes
  and kept creation eligibility unresolved in both directions. This review is
  not full T006, runtime acceptance or an evidence upgrade for C01–C07.

- Source tracing now records concrete writer order and eleven project-child
  revision triggers plus invoice-line parent writes in
  [permission-state.md](contracts/permission-state.md). Reparenting and cascades
  require existing parents prelocked in the strongest mode; a shared parent lock
  can later upgrade implicitly. Remaining maintenance/identity/job edges and
  the finalized policy still gate T042.

- A local PostgreSQL 17.10 diagnostic rejected the proposed shared row lock in a
  READ ONLY transaction. Three inspected read/preview paths use that mode.
  Decision: retain consistent snapshots while adapting transaction mode and
  testing the organization revision fence and fresh-transaction retry. Rejected:
  adding a lock to a read-only transaction, retaining an old policy snapshot
  after a wait, or weakening export totals/size consistency without analysis.

- Inline import `streaming::apply` awaits entry pages inside its transaction.
  Decision: reconcile bounded preparation/commit with existing import atomicity
  before adding authorization gates; do not hold revocation behind network waits
  or silently change an atomic import into partially committed batches.

### Persistence and concurrency design — 2026-10-02

- Decision: persist canonical person grants separately from template provenance
  and explicit administrative identity. Rationale: upgrades/display classification
  must not silently grant authority; custom all-grants selection must not count as
  Administrator. Alternatives rejected: live template-name lookup, rank ordering
  and inferring administrative identity from a particular grant set. This storage
  proposal does not decide pending template deletion/update/reapplication rules.
- Decision: propose the organization row as one shared/exclusive authorization
  gate, with current-state reload and revision checks. Rationale: it extends the
  proven user-access serialization boundary without an extra locking service.
  Alternatives rejected: admission-only checks and adding a new editor gate while
  leaving existing writers outside it. Every organization-row update starts
  exclusive, even when its permission is not administrative, to avoid upgrades.
- Existing `(id, org_id)` keys in migration `0030_project_creation.sql` support
  tenant-constrained references. The old `audit_log` in `0001_init.sql` lacks the
  required FK/revision/receipt contract and has no inspected insertion path; do
  not mistake its existence or plugin events for durable access-change auditing.
- Decision: one transaction records state, revisions, attributed audit and request
  outcome. User/operator actors are distinct; credentials/provider subjects are
  excluded. Exact replay checks current disclosure authority and recorded scope
  before returning a historical result, without requiring deleted references to
  exist again. Alternatives rejected: replaying old mutations, fabricated admin
  actors and logging denials in the transaction that will be rolled back.
- Independent read-only review identified actor-before-organization ordering in
  project finalization/editing and assignment paths, hidden project-revision
  trigger locks, operator-attribution ambiguity and live-reference-before-replay
  ordering. The proposed contract now addresses these; the exact cross-command
  resource hierarchy remains an explicit T042 design gate, not implemented code.
- The [state protocol](contracts/permission-state.md) and expanded data model are
  partial planning under T008. C01–C07, approved migration mapping, dependent-spec
  reconciliation and the operation matrix remain open. Neither complete Phase 1
  nor the full `speckit-analyze` gate is claimed: full tasks are still incomplete.

### Current-account investigation — 2026-10-02

- Decision: continue reference discovery using the existing account, as requested,
  before asking for additional seats or accepting a product deviation. The
  [probe register](contracts/current-account-investigation.md) tracks safe account
  surfaces, fresh documentation findings and evidence limits; [progress](progress.md)
  records the next action.
- Rationale: current owner-visible configuration can narrow the unknown catalog,
  dependency and operation boundaries even when non-owner enforcement cannot be
  exercised. Browser approval subsequently succeeded: one immutable owner and
  no archived people were confirmed, along with 50 grants, six profiles and the
  current delivered editor. Dated outcomes and limits are in the probe register.
- Alternatives rejected: bypassing owner-disabled controls, treating legacy API
  restrictions as six-profile web policy, or saving an unchanged form as a
  supposedly read-only probe. Assignment controls may autosave, and eligible
  invoice saves can move retainer funds.
- The new evidence identifies separate assignment read/write, own-permission
  visibility and retainer lifecycle questions, plus a documented company-cutoff
  lock distinct from scoped approvals. The independent approval review's lock
  findings were checked directly in the linked official guides. T006 remains
  open; these are reference questions, not invitations to simplify confirmed parity.
- Fresh browser/source evidence adds C01–C07: template-deletion preservation is
  contradicted by a downgrade warning; report access and ordinary rate access
  need distinct checks; managed-rate/cost help conflicts remain; deadline gating,
  approval prerequisites and project-manager assignment-loss preview need
  discrimination. No account writes or non-owner enforcement tests were performed.
  Decision: retain the full-feature gates and label provisional contracts instead
  of choosing whichever source grants more access. No runtime policy changes.

### C05 documentation reconciliation — 2026-10-02

- Decision: use current dedicated company-lock guides and the refreshed general
  approval article for the deadline dependency. The existing parity instruction
  selects newer web behavior; no additional user preference is needed.
- Rationale: dedicated guidance explicitly distinguishes independent schedules,
  and the general guide now agrees while acknowledging staged rollout. Captured
  deadline-disabled controls are therefore not a universal implementation rule.
- Alternative rejected: requiring a submission deadline for all modes, or
  modifying live settings to re-probe an already explained rollout difference.
- Target and remaining execution acceptance are in `contracts/company-locks.md`.
  This resolves the product dependency, not temporal algorithms or runtime tests.

### Constitution reconciliation — 2026-10-01

- Executed the checked-in `speckit-constitution` workflow against the user's
  already recorded six-profile/custom-permission decision. Version 1.1.0 expands
  authorization constraints and verification guidance without replacing the five
  core principles, authentication, financial invariants or datastore rules.
- Checked all three feature templates and runtime guidance. No command-template
  directory or extension hooks exists. Kept generic templates and accurate
  current-runtime documentation unchanged; added the required Sync Impact Report.
- Updated this specification and plan to distinguish the included amendment from
  completed policy design or deployment. T008 remains unchecked: dependent-spec
  reconciliation and persisted authorization/revocation design are not finished.
- The change is proposed in PR #212, not merged or a substitute for reviewing
  migration differences. No schema, application behavior or data changed.

### Migration research — 2026-10-01

- Decision: compare effective operation/scope/field access, not profile labels,
  before activating a reviewed mapping. Rationale: the current Manager has
  financial/project writes but the timesheet still operates on the session
  person's entries; none of the new profile labels proves equivalent access.
  Rejected: automatic Manager-to-Project-Manager or Executive-Manager mapping.
- Decision: retain explicit unknowns for historical job requesters and approval
  coverage. Rationale: current storage does not contain those new facts.
  Rejected: attributing jobs to the current owner or inventing project approval
  history during migration.
- The new [migration contract](contracts/migration.md) supplies the previously
  missing T007 artifact, preview/activation acceptance and concrete fixture
  categories. Actual mappings, compatibility strategy and historical job/approval
  transition remain review gates; T007 and T019 are not marked complete.
- Reran the checked-in plan setup helper; it preserved the existing plan and
  resolved feature 015. Continued Phase 0 research only. Full Phase 1 design,
  post-design constitution approval and full-feature analysis remain incomplete.

Additional Phase 0 investigation of managed rates could not resolve the conflict
from current official documentation. [Rate-scope evidence](contracts/rate-scope-evidence.md)
records the sources, limits and independent person/project probes required to
settle it. No rate scope or prerequisite has been invented to close the gate.

### Dependent-spec reconciliation — 2026-10-02

- Decision: propagate accepted financial and designation contracts without
  activating runtime policy or rewriting delivered legacy acceptance as
  six-profile evidence. The source-revision and test-ownership register is
  `contracts/dependent-spec-reconciliation.md`.
- Rationale: the shared editor still grouped administrator-only costs with
  private notes; approved FR-022 deliberately separates them. Its conditional
  transition is now explicit. Latest dashboard/Clients/Settings/Workspace specs
  already depend on feature 015, whereas the authorized Clients MVP intentionally
  retains legacy authorization until the reviewed cutover.
- Alternatives rejected: using the stale dashboard spec from this branch as
  final acceptance, modifying other worktrees, inferring a client-default-rate
  permission, silently promoting project managers or enabling grants piecemeal.
- This is bounded Phase 0 integration research and partial T008 reconciliation.
  FR-026 now settles project-editor delegation to compatible people. Creation,
  person-management eligibility/retention, remaining lifecycle predicates and migration
  remain open, so full planning/Analyze gates are not satisfied.

### Migration schema dependencies — 2026-10-02

- Decision: refine T007/T019 with M01–M08 in `contracts/migration.md` from the
  checked-in SQL, without selecting role mappings or running a migration.
- Rationale: assignment pairs own cascading cost/budget/task-access children;
  replacing membership rows would destroy business settings. Rate NULL/zero
  semantics, SQL authorization views and revision triggers also participate in
  a data-preserving cutover, beyond copying role labels.
- Alternatives rejected: delete/reinsert membership, checking counts only,
  assuming safe child rows prove all parent tenancy, deriving requester identity
  from job tenant/lease metadata or repairing historical attribution by guess.
- This is bounded Phase 0 evidence and fixture refinement. Person-management
  writer authority is now settled by FR-027; remaining eligibility/retention,
  operation predicates and migration mappings still gate full planning, Analyze
  and runtime activation.

### Increment readiness dependency repair — 2026-10-03

- Decision: apply the user's explicit authorization to separate confirmed local
  implementation prerequisites from full-policy activation and acceptance gates.
  Next is T050–T052, the pure FR-028/029/031 relationship prerequisites; no new
  product decision, dependency, schema or runtime policy is introduced.
- Rationale: Constitution 1.1.0 requires matrix/migration/concurrent activation
  review before replacing legacy guards, not before every pure unit-tested
  contract. A spec-readiness checklist asks for defined acceptance coverage,
  while achieved runtime outcomes belong to T020. Full readiness remains 12/16.
- Rechecked the current [Harvest permission guide](https://support.getharvest.com/hc/en-us/articles/44171549176077-Permissions):
  custom-role creation, application, person-specific adjustment/save-as-new and
  deletion are documented. It does not establish in-place rename/update or bulk
  propagation. This is a limit of the evidence, not proof those features cannot
  exist; retain them as evidence watches rather than invented blocking features.
- Alternatives rejected: wait for hypothetical lifecycle operations; infer their
  absence as a permanent parity exclusion; treat canonical grants as explicit
  Administrator identity; expose a partial policy; or equate pure tests with
  data-preserving migration and cross-surface acceptance.
- Remaining gates are attached to their consumers: saved classification and
  name equivalence to affected persistence; withdrawal/calendar predicates to
  approval/lock work; migration and complete enforcement to activation. Existing
  unresolved questions remain recorded, without blocking unrelated pure checks.

### Person-profile transaction review — 2026-10-03

- Decision: implement the bounded command in `contracts/person-profile-commands.md`.
  Explicit Administrator selection requires its full set; other selection and
  changed individual grants produce non-admin identity. Unchanged saves preserve
  independent stored facts. This derives draft intent from the current guide and
  retained `Rt`/`gi`/`pe`/`Ze`/`Ve` handlers, not tested Harvest persistence.
- Rationale: custom grant equivalence cannot promote a person; restricting an
  Administrator must not leave unrestricted identity behind. Existing Horae role
  changes support inactive targets without activation; preserve that behavior.
- Read-only adversarial research closed local FK/trigger ordering. Reuse existing
  project `(id, org_id)` uniqueness; do not add a redundant index. Keep independent
  management tables free of legacy assignment cascades and project-write triggers.
- Alternatives rejected: deriving identity from saved source, preserving admin
  identity after a confirmed restricted proposal, refusing all inactive target
  edits without evidence, or claiming legacy role-based deactivation is compatible
  with the new canonical last-admin count. Full activation remains gated.

### Project designation insert lock review — 2026-10-03

- Decision: use the reviewed `contracts/project-management-commands.md` for
  T059–T061, retaining full integration gates. Project delegation has confirmed
  predicates independently of person-management inactive-target questions.
- Rationale: migration 0044 child INSERT still acquires a project FK KEY SHARE.
  `editing/save.rs` takes project UPDATE before `editing/associations.rs` asks
  organization SHARE. Ordinary parent waiting under organization UPDATE would
  create a cycle. Acquire parent KEY SHARE NOWAIT and roll back on contention.
- Alternatives rejected: deferred FK checks, a parent revision-writing trigger,
  treating separate tables as proof of no FK lock, automatically promoting
  managers, rechecking addition eligibility on removals, or bypassing grants by
  administrative identity. Preserve current archived-project edit availability.
- Bounded independent read-only review found no unresolved high contract finding.
  Verify the actual command with database contention tests; source review alone
  neither completes T042 nor reconciles the legacy editor's manager representation.

### Person-management lifecycle clarification — 2026-10-03

- Rechecked Harvest's [people-assignment guide](https://support.getharvest.com/hc/en-us/articles/4422314817677-Making-people-assignments-for-Managers),
  [archiving guide](https://support.getharvest.com/hc/en-us/articles/360048687311-Archiving-deleting-and-restoring-people)
  and [teammates API](https://help.getharvest.com/api-v2/users-api/users/teammates/).
  Archived people cannot sign in and historical reporting remains available;
  these sources do not establish whether an inactive responsible person may
  receive new managed-person relationships. Login denial is not a write predicate.
- Executed Spec Kit Clarify prerequisites and asked one question: require an active
  receiving responsible person for new relationships, or allow preconfiguration
  while inactive. No answer has been accepted at this checkpoint. This does not
  decide retention on deactivation, restoration or inactive managed subjects.
- Continue independent project-delegation verification. Do not copy FR-026's
  active-project-manager rule into FR-028 without resolving this distinction.

### Administrator audit lookup review — 2026-10-03

- Decision: implement `contracts/audit-lookup.md` with a single-receipt lookup and
  closed version-1 historical DTOs. Do not expose raw intent or replay results.
- Rationale: 0043's JSON-object constraint does not prove a valid audit shape.
  Ordinary optional-field decoding can mistake a missing change for an explicit
  no-op. Validate required nullable fields, supported catalogs/provenance and
  revision transitions without weakening trusted runtime models.
- Independent read-only review closes the local order: organization SHARE under
  READ COMMITTED, then plain active-actor/state/receipt reads. No project/user row
  locks, writes, FK acquisition or external waits are added. Test both revocation
  orders; this does not prove the full cross-surface hierarchy.
- Alternatives rejected: returning arbitrary stored JSON, normalizing historical
  grants, requiring deleted sources to exist, fabricating user attribution for
  operators, or inventing an operator command registry. The recorded command is
  an audit label, never execution authority. UI/listing remains separate.

### Legacy converter lock repair — 2026-10-03

Independent source review traced the converter, chunk foreign keys, worker lease
publication and queue maintenance. Organization SHARE before job UPDATE avoids
the reverse organization-FK edge while allowing an existing worker's KEY SHARE.
One transaction connection retains size-one-pool support; commit before choosing
another candidate avoids retaining locks across organizations. Discovery is not
authority to mutate: recheck the exact tenant/job and oversized predicate.

PostgreSQL documents compatible SHARE/KEY SHARE modes and READ COMMITTED's fresh
statement snapshots and predicate re-evaluation after concurrent updates. These
support the selected mechanism, not proof that every application writer follows
the full hierarchy. Sources checked 2026-10-03:
[row locks](https://www.postgresql.org/docs/17/explicit-locking.html#LOCKING-ROWS),
[READ COMMITTED](https://www.postgresql.org/docs/17/transaction-iso.html#XACT-READ-COMMITTED).

The project-editor investigation is separate: UPDATE-first can deadlock with
ungated invoice/budget FK writers. NO KEY UPDATE is a candidate staging mode,
not an implemented or globally verified solution. Preserve editor isolation and
review both task-linking callers and revision triggers before changing it.

### Project-family gate integration — 2026-10-03

The subsequent source review closed a bounded prefix covering every caller of
`lock_creation_actor` and both production callers of `enable_project_task`.
No product policy is inferred: existing role checks remain, and the task creation/
linking wrappers now pass trusted identity for transaction-level revalidation.
The local contract and test mapping are in `permission-state.md` and T068–T070.

Organization NO KEY UPDATE is required for access-changing project operations:
an in-flight invoice can hold project SHARE before acquiring organization FK
KEY SHARE. Organization UPDATE would add a reverse edge. Similarly, newly added
project prelocks for assignment/task-link child revision writes use NO KEY UPDATE
so entry insertion can finish its project FK while retaining task-member SHARE.
The existing editor project UPDATE is different: preserve its historical-data
exclusion, SERIALIZABLE isolation and reload-on-conflict behavior. Editor loading
retains REPEATABLE READ and actor SHARE after the gate for stale-snapshot denial.

Independent read-only research supplied these two counterexamples and reviewed
the caller closure. Implementation self-review also checked tenant-safe assignment
discovery/recheck, post-commit events, task-rate/idempotency behavior and absence
of late organization locks. Production-path tests, not duplicate lock SQL, cover
the actual opposing operations. Full T042, revised access fencing, credentials/
imports and other historical writers remain required before activation.

### Bounded report-download review — 2026-10-03

Scoped implementation self-review, not an independent or full-feature review:

- Traced every production body/chunk caller. The registered route is the only
  production body constructor; archive-fragment reads now use the same
  transaction as current actor authorization. Other pool-based fragment callers
  are fixtures. No client-supplied actor/organization is trusted.
- Reuse the existing import guard rather than duplicate its query: explicit READ
  COMMITTED, organization SHARE, active same-tenant Administrator SHARE. Both
  importer commands and result readers retain their existing policies/errors.
- Metadata serialization and each bounded page commit before output. Only the
  already authorized 16-fragment buffer can drain after revocation; the next
  page and separately captured tail recheck. Empty reports do not bypass that
  check. No lock spans a client-paced yield or an external request.
- Captured archive end/tail remain immutable when workers append. Missing chunks,
  failed authorization or transaction failure abort the stream with safe errors;
  none is converted to successful EOF or a private database error message.
- Fixed the malformed-report fixture, not production parsing: status chooses
  checkpoint report before final report. The test must remove that checkpoint
  to exercise the final-report error and rollback with a size-one pool.
- Registered HTTP testing revokes after preparation but before body polling;
  it requires a non-timeout transfer failure, subsequent session denial and
  healthy exact-byte output for another Administrator. Existing CLI checks remain.

The requirement-to-test map is in `quickstart.md`. All 912 non-excluded
server-binary tests, complete cache regeneration and fresh offline all-targets
Clippy pass. No schema, historical data, profile mapping or worker authority
changes are included; full T042 and runtime-policy gates remain open.

### Earlier workflow record

- Followed the checked-in `speckit-specify` skill, local template and constitution. No extension hooks or template preset overrides were found.
- Feature 015 follows the independent 012/013/014 design drafts. The branch/worktree starts from fetched `origin/master`, not an unmerged application branch.
- The two scope questions are answered by the user's explicit parity instruction. The clarification is recorded in the spec and its scenarios, requirements and success criteria.
- Incremental plan and foundation contract now exist. Full research/design remains open; complete reference verification, operation matrix, governance and migration before runtime policy implementation.
- After clarification, 12/16 checklist markers pass. The remaining gaps are detailed requirements/acceptance coverage and outcome readiness, not the two answered scope questions.
