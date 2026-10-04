# Current authority during CSV delivery

T110–T113 implement the US3 / FR-006/007/010/017/018 and SC-006 export
subsets. They preserve legacy operation predicates until the reviewed full
policy cutover; this is not canonical policy activation or full US3 acceptance.

## Source and authorization

Retain the trusted session actor in timesheet, project and invoice CSV handlers.
Use one reserved, close-on-drop connection with explicit READ COMMITTED, READ
WRITE. Keep the existing five-second statement, ten-second idle-transaction
and sixty-second total download deadlines and four-download admission limit.
The statement deadline now bounds each cursor fetch or authority statement,
not an unbounded source query; the total deadline still bounds the whole job.

Before declaring the source cursor, authorize inside a savepoint: organization
SHARE, then active same-organization actor SHARE. Timesheet and invoice exports
require the existing Manager/Admin predicate. Projects require an active actor
and the existing `project_read_access` predicate, not an invented Manager gate.
Rollback and RELEASE the named authorization savepoint successfully before
source declaration; successful checks must not accumulate savepoint frames.
Declare the NO SCROLL cursor outside rollback scopes. Its source snapshot must
include scope changes committed while the initial organization gate waited.

Subsequent source fetches retain that snapshot. Invoice metadata, lines and
stored totals come from one LEFT JOIN cursor; an empty invoice has a nullable
line-ID sentinel, a missing/foreign invoice has no row. Preserve exact integers,
nullable fee-only line quantities, dates, CSV quoting, headers, order and names.
Reuse existing adjustment labels and integer formatting, not new money logic.

Before each nonempty output block (including first, empty-source headers and
invoice total tail), reserve channel capacity without authorization locks.
Then authorize in a fresh savepoint using the same connection. Project blocks
also lock their distinct captured parents in UUID order and separately query
current access after those waits, sharing the XLSX release predicate. Deny the
entire pending block if any captured project is missing or inaccessible.
Rollback the savepoint and synchronously send through the reserved permit,
with no intervening await. Failure to release locks suppresses delivery.

Already authorized queued/delivered blocks cannot be recalled. A later denial
interrupts the body, never a successful truncated EOF. No rollback/retry may
resume a failed cursor. Cancellation closes its connection and releases the
existing admission permits; no second connection or new worker is required.

## Bounded native transport

Migration 0046 adds a SECURITY INVOKER, PUBLIC-revoked fixed-cursor helper that
returns native records. Each invocation accepts 1–128 rows and stops after
the row that reaches 64 KiB of logical projected payload. Each source computes
private `export_bytes` using bigint sums of `octet_length` for every variable
field plus fixed overhead. Reject invalid limits or NULL/missing/negative weights.
Do not use physical/compressed tuple size or JSON conversion. Return the
threshold-crossing row; do not drop it or scroll backwards. Fetch only one row
initially, then at most 128. Process a batch before fetching another.

Output blocks independently flush after their first record, 128 records or
64 KiB serialized bytes. These thresholds permit one oversized record: they
are not a hard whole-process memory ceiling. Decoded input, output buffering,
CSV escaping and the one queued block coexist. Project-ID retention is bounded
by the output record limit, independently of input-batch boundaries.

SQLx checks DECLARE and typed helper SELECT statements independently; their
cross-statement anonymous record shape is verified at runtime. Test each real
projection, including NULLs, Unicode and extreme integers. No dependency or
business-data migration is needed; apply the helper only to disposable test
databases during development.

The application normally applies migrations using its runtime database role,
which owns and can execute the helper. A deployment using a distinct migration
owner must explicitly grant EXECUTE to its runtime role; do not grant PUBLIC
execution or switch to SECURITY DEFINER to bypass deployment permissions.

## Acceptance

- Reproduce inactive project-actor acceptance before changing production code.
- Verify all three routes for allowed, missing/inactive/foreign actor and
  current-role denial, including empty sources and missing invoices.
- Race organization/actor and project-parent waits against revocation and scope
  gains. A reader-first authorization delays writers only through its check.
- Revoke while output capacity is exhausted; already queued bytes may drain,
  but the next block fails and no locks prevent revocation.
- Change source rows after capture and between batches: original data persists
  while fresh authorization observes new privileges. Invoice assertions retain
  original filename, metadata, lines and totals, not either coherent version.
  Pause the invoice fixture during FETCH, not before DECLARE captures its source.
- Verify row/byte boundaries, compressible oversized Unicode, no skipped rows,
  more than 10,000 records, first-record delivery, one-connection operation,
  timeout, query failure, cancellation and connection/admission cleanup.
- Exercise session-authenticated CSV routes in the existing HTTP harness; run
  full server/export regressions, complete SQLx regeneration, offline lint/WASM,
  formatting, adversarial review and scoped analysis before publication.

Research and native-cursor feasibility evidence are in `../research.md`.
