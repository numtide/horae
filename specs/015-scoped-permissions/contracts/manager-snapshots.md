# Materialized manager snapshots

T098–T103 refine FR-006/007/010/017/018 and T039/T040's snapshot prerequisite.
These increments integrate current-authority revalidation in project fee balances,
invoice preparation and invoice editor load/review, not canonical capability activation. Preserve their
existing active same-organization Manager/Admin requirement and every financial
query, validation and exact result. No schema, UI or real-data changes are needed.

## Transaction contract

`server_fns/snapshot.rs` supplies one shared manager-snapshot prelude. Session
wrappers provide organization and actor IDs; invoice preparation must retain the
actor rather than passing only its organization. No browser-supplied role or grant
is trusted. The existing test-only preview adapter also supplies a fixture actor.

Begin one transaction with explicit REPEATABLE READ, READ WRITE before queries.
Acquire organization SHARE, then same-tenant actor SHARE; require current active
Manager/Admin before any business payload read. Do not lock project or client
targets, upgrade the organization lock, or acquire another pool connection.
READ WRITE permits locking only: these consumers still execute no business writes.

A legacy writer can hold the organization gate and update the actor without
changing `access_revision`. Locking only the organization would leave a stale
snapshot. The actor lock detects its changed row through PostgreSQL serialization
failure. On SQLSTATE 40001, explicitly roll back and retry the entire prelude in
a fresh transaction, up to three attempts. No business query has run yet. Other
database errors are not retried; return fixed messages and log internal detail.
Exhausted conflicts report a retryable conflict, not permission success. Missing,
foreign, inactive or demoted actors receive the same non-disclosing denial.

Retain existing resource checks and snapshot-consistent business reads after the
prelude; materialize and commit before delivery. Use a five-second lock timeout
for the prelude only, resetting it before business queries; do not add a new
business-query timeout or change accepted invoice sizes. Cancellation/denial
must release the transaction and pool connection. No external work or client
wait occurs while holding these gates. Preserve a stricter inherited lock timeout
and restore the prior value after the prelude; five seconds is a per-lock ceiling,
not a total operation deadline.

## Required verification

- All four production readers allow current Manager/Admin and deny invalid actors.
- Organization-first legacy demotion/deactivation with unchanged access revision
  wins before reading: a fresh retry denies, with no payload.
- Direct actor update wins without the organization gate: same denial.
- Organization revision change wins: retry sees one fresh business snapshot.
- Reader-first retains actor authority until materialization; the next read denies.
- Concurrent business changes cannot mix project fee values/defaults or totals.
- Inherited READ ONLY/READ COMMITTED/REPEATABLE READ defaults do not alter the
  contract. Single-connection pools, failed reads and cancellation release locks.
- Retry exhaustion is bounded; existing invoice/fee arithmetic and HTTP session
  identity regressions remain green. Compile-time SQL cache is complete.

## Invoice editor integration (T101–T103)

Apply the same prelude to `invoices/editing::{load,review}`. Their existing
session wrappers already supply both organization and actor IDs. Keep the
mutation-only `editing::lock_actor` and its save/generation/status callers
unchanged; never start a second transaction inside a mutation.

Preserve missing/foreign invoice 404, draft-only conflict, exact review revision,
line/subtotal consistency, fee validation and all discount/excess calculations.
Reads create no receipts, change no sources and never replace a stale requested
revision with the current one. The readers acquire no invoice advisory or row
lock and commit before returning their existing DTOs. No new model or schema is
needed. The existing snapshot timeout/retry contract applies unchanged.

Test both real readers with organization-gated and direct actor revocation,
revision-first refresh, reader-first retention, single-connection cancellation,
inherited READ ONLY/isolation settings and complete business-row preservation.
Pause at `invoice_line_items` to prove metadata/revision/line consistency; a fresh
load sees committed changes and a stale review conflicts. Retain existing
financial/invalid-edit tests. Extend the existing real-cookie HTTP matrix for
session identity, foreign resources, stale reviews and non-draft conflicts.

## Materialized exports (T104–T106)

Integrate `reports::limits::{entries,invoice,pdf}` and their XLSX/PDF handlers.
Retain both trusted session actor and organization IDs through loading and final
response release. Use the same manager prelude before size/payload queries;
map its fixed errors to HTTP status only. Apply existing five-second statement
and ten-second idle-transaction deadlines after the prelude, without issuing
another SET TRANSACTION. The READ WRITE mode permits authorization locks only;
business queries remain read-only. Existing size, field, row and output limits,
filters, coherent snapshots and exact rendering remain unchanged.

Commit before rendering. After the existing bounded blocking renderer returns,
recheck current authority in a fresh short transaction before returning the body
or constructing response headers. A revocation that completes during rendering
denies delivery and drops both generated bytes and their admission permit.
This is a response-release authorization boundary, not continuous revocation
after that check or a promise to recall delivered bytes. Never hold a database
transaction through rendering, semaphore/body lifetime or client backpressure.

Verify all three real readers: invalid/foreign/inactive/demoted actors, both
revocation orders, organization-revision retry, same-snapshot size and payload,
inherited settings and cancellation/pool reuse. A channel-controlled renderer
must allow revocation to commit and then deny its result. Real-cookie Axum routes
must preserve authorized files/headers, deny missing/member/inactive sessions,
ignore forged identity selectors and hide foreign invoice identifiers/filenames.
Retain existing limit, deadline, rendering, admission and streaming regressions.

## Remaining consumers

Do not change `reports::limits::configure_transaction`: CSV workers reuse it and
await browser-paced channel sends inside the transaction. Holding authority locks
there would delay revocation through client backpressure. Materialized project
scope instead follows `project-exports.md` (T107–T109). Legacy membership writers
can change scope without updating the organization row or actor; this manager
prelude alone does not fence those changes. CSV and full canonical grants remain
open, not inferred from these materialized-reader integrations.
