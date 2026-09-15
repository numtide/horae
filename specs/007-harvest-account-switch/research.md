# Research: Safe Harvest Account Switching

## Separate account generation from connection revision

**Decision**: Keep persistent per-organization `account_generation` and `connection_revision`. Increment both on Change account; increment only revision on successful credential store and disconnect. OAuth starts capture revision, generation and actor organization; completion revalidates after external exchange.

**Rationale**: Generation alone either allows old OAuth to undo Disconnect or unnecessarily invalidates same-account retries. Persisting counters independently of credentials/binding also fences A→B→A and callbacks when no binding currently exists.

**Rejected**: Deleting the binding alone; session-only invalidation; comparing only account IDs; holding a database lock during network authorization.

## Serialize acceptance without blocking the queue behind a worker

**Decision**: Short organization-row gate for enqueue/retry and connection changes, plus the existing import session exclusion when switching/storing/disconnecting. Recheck provenance and queued/running work in the committing transaction. API execution checks captured generation after acquiring the import exclusion and before loading credentials.

**Rationale**: Claim only transitions between two active states, so it cannot turn a safe switch into unsafe work. Enqueue and retry can. Reusing the long-lived worker lock for ordinary enqueue would prevent queueing behind a running import.

## Preserve history and idempotency identity

**Decision**: Keep old job rows/reports and their immutable captured generation. Existing `(org, kind, key)` uniqueness remains cross-generation; a conflicting-generation replay fails. API submissions include an expected generation captured before sending; legacy missing generation means zero and fails after a switch.

**Evidence**: `jobs::cleanup` retains terminal jobs for 30 days, not a configurable duration. `submission_key` accepts keys for at most 24 hours with five minutes future skew. Thus every accepted key remains retained until well after it becomes invalid, including future-dated keys. No deletion/tombstone subsystem is needed.

**Boundary**: An unaccepted request is not a durable job. A deliberate new CLI invocation captures the current generation, but reusing an accepted old identity can never target the replacement account. No generation prefix is added to the unique key, which would defeat this protection.

## Existing UI and scope

**Decision**: Reuse `Modal`, existing importer resources and typed server functions. Show retained binding while disconnected, separate Change account from Disconnect, and preserve reports/CSV history. Block provenance and active API/CSV work; do not offer a destructive imported-data reset.

**Evidence**: The live incident was a failed preview with zero imported identities, disconnected credentials and a surviving account binding. A manual operator reset was necessary; the product flow must make this eligible case self-service without reproducing manual history deletion.
