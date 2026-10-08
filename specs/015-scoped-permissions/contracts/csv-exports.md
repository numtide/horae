# Current authority during CSV delivery

T110–T113 implement the US3 / FR-006/007/010/017/018 and SC-006 export
subsets. The ordinary time refinement below applies canonical scope only to
policy-1 time exports. Invoice and project predicates remain unchanged; this is
not canonical policy activation or full US3 acceptance.

## Source and authorization

Retain the trusted session actor in timesheet, project and invoice CSV handlers.
Use one reserved, close-on-drop connection with explicit READ COMMITTED, READ
WRITE. Keep the existing five-second statement, ten-second idle-transaction
and sixty-second total download deadlines and four-download admission limit.
The statement deadline now bounds each cursor fetch or authority statement,
not an unbounded source query; the total deadline still bounds the whole job.

Before declaring the source cursor, authorize inside a savepoint: organization
SHARE, then active same-organization actor SHARE. Policy-0 timesheet and invoice
exports require the existing Manager/Admin predicate. Projects require an active actor
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

## Canonical ordinary time refinement

Reuse the strict state restoration and current time authority used by XLSX.
Acquire initial organization/actor gates inside the authorization savepoint and
pin the selected policy version. Release the savepoint before `DECLARE`.
The cursor independently reads policy, actor activity/legacy role, complete stored
permission state and management relationships from its own source snapshot. Do
not use earlier grant flags to choose its rows: gains/losses committed before
`DECLARE` must change its result. Source rows use the same tenant-qualified report
facts and narrowing filters as the ordinary reader and XLSX.

Root source authority in a constant identity row with LEFT JOINs, not a filtered
active actor. Carry native private authority fields beside each result; retain a
nullable entry sentinel even for missing authority or zero visible entries.
Validate captured policy/actor and strict canonical state before omitting the
sentinel or writing any output. An invalid state captured during `DECLARE` must
fail even if it is restored before delivery. Policy 0 must not require irrelevant
canonical state. No source metadata or owner/project scope IDs enter the CSV.

Retain the captured owner/project pairs for each pending output block. After
reserving capacity, reload active identity, pinned policy and current strict
grants, and apply own OR managed-person/project OR all to every pair. Never
replace captured context with a current source-entry lookup. Rollback/release
must succeed before synchronous send; clear scope together with its block.
Count every private variable field, including grant array elements and provenance
labels, in native `export_bytes`. The sentinel is not a CSV data record. Preserve
all existing transport, timeout, cancellation and error-body guarantees.

## Bounded native transport

### Canonical grouped time

The grouped CSV source uses the same four dimensions, scope, rounding and five
filter dimensions as grouped XLSX, but does not inherit workbook row or text
limits. Preserve the native cursor and the download admission/deadlines above.
Aggregate first by entity ID and original `(person, project)` pair; carry group
totals and an explicit last-context marker on the bounded native fragments.
Never collect all contexts of a group into an array or a Rust vector. Distinct
people and projects cannot be validated independently: their correlation matters.

Reserve output capacity before acquiring the current organization/actor gates.
Keep those same gates through validation of every context fragment of the group,
checking at most 128 pairs at once. Only after its last fragment may that group's
CSV row be sent. Release the authorization savepoint successfully and send
synchronously, without another await. A group is an output block; include the
headers in the first block. An empty source still validates its captured state
and current authority before releasing a header-only file. Releasing gates between
fragments would allow earlier context checks to become stale before the total is
disclosed. Waiting for output capacity with gates held would block revocation.

The source cursor must outlive these savepoint rollbacks and retain its original
scope even if entries are later changed or deleted. Labels and private authority
metadata count toward the existing native byte threshold. As with detailed CSV,
one oversized record is permitted; this is not a hard 64 KiB process-memory limit.
No monetary columns, authority fields or private scope identifiers enter the file.

Verify complete multi-context groups, partial revocation, authority stability
across fragment batches, release before backpressure, frozen source changes,
empty/invalid source authority, all dimensions/filters, exact rounding, oversized
Unicode labels and more than 10,000 groups as well as contributing entries.

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
