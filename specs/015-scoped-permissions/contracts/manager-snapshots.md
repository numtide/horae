# Materialized manager snapshots

T098–T100 refine FR-006/007/010/017/018 and T039/T040's snapshot prerequisite.
This increment integrates current-authority revalidation in project fee balances
and invoice preparation, not canonical capability activation. Preserve their
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

- Both production readers allow current Manager/Admin and deny invalid actors.
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

## Explicitly separate consumers

Do not change `reports::limits::configure_transaction`: CSV workers reuse it and
await browser-paced channel sends inside the transaction. Holding authority locks
there would delay revocation through client backpressure. Materialized exports,
Member project export scope, invoice editor load/review and full canonical grants
remain separate integration tasks. In particular, legacy membership writers can
change scope without updating the organization row or actor; this prelude alone
does not fence those changes. It is safe for these two existing manager-only
readers, not proof of all-surface permission enforcement.
