# Budget email preparation authority

This is the bounded service-reader portion of FR-006/007/010/017/018 and
SC-006, refining US3 and the notification boundary in `permission-state.md`.
It preserves feature 011's existing recipient rule, not OP37's unresolved
canonical/custom-profile mapping. No account mutation, real mail, new endpoint,
dependency, migration or policy activation is needed.

## Preparation and release

1. Begin an explicit READ COMMITTED, READ WRITE transaction, overriding inherited
   isolation/read-only settings. Use five-second statement and ten-second idle
   transaction deadlines. Acquire organization SHARE before resource locks.
1. Read the actual tenant-bound `budget_email` claim, not the event object's
   payload or attempt count. Discover the notification's recipient/project IDs
   without treating discovery as authority.
1. Lock the same-tenant recipient SHARE, then project SHARE. Do not lock settings
   or assignments afterwards: their migration 0039 triggers update the parent,
   so a child-first writer could otherwise deadlock with this reader. Parent
   locking followed by a fresh statement orders committed child changes.
1. Lock the tenant-bound outbox row UPDATE last. In a separate fresh statement,
   revalidate claim token, kind, undelivered/nonterminal state, exact stored
   payload and attempts, and more than 25 seconds of remaining lease using
   `clock_timestamp()`. A changed claim/payload/attempt count or short lease
   skips delivery without acknowledging or failing a replacement claim.
1. Obtain message fields in a fresh statement under those locks. Require the
   exact discovered notification/project/recipient IDs and tenant; active project
   and recipient; enabled alerts; and the existing Manager/Admin or eligible
   lead/admin/creator assignment predicate. A notification retargeted while
   waiting must never disclose an unlocked replacement's payload.
1. Return only the prepared address/message and stored attempt count after
   committing. Invalid payload, excessive attempts or ineligible recipients use
   existing sanitized terminal reasons within the already locked transaction,
   avoiding a stale decision failing a changed payload after release. Mapping
   drift is a skip, not terminal ineligibility; recheck an initially absent
   notification after the final wait too. Missing settings are genuine
   ineligibility for an unchanged notification. No mail transport or post-send
   acknowledgement runs while holding domain locks.

Keep the 20-second transport bound, five attempts, stable Message-ID, retry
semantics, disabled-worker behavior and sanitized errors. A revocation winning
the preparation gate prevents disclosure; a writer ordered after authorized
preparation cannot recall a message already released for external delivery.
Claim ownership is not recipient authority and this is not exactly-once mail.
Database failure/cancellation returns no prepared message and releases its
transaction; queue recovery retains its existing semantics.

## Acceptance

| Case | Required observation |
| --- | --- |
| Organization/recipient/project wait followed by committed revocation | Actual `deliver` never invokes the stub sender; existing terminal handling remains |
| Assignment removal or alert disable wins project parent lock | Fresh eligibility denies after the wait, including inherited REPEATABLE READ |
| Winning grant/activation or changed address/project name | Fresh coherent eligible payload, not discovery-time fields |
| Claim replacement, payload change, terminal state or short lease during a wait | No sender, no stale acknowledgement or terminal update of replacement claim |
| Lease margin expires during a lock-only wait | Current clock rejects it even when transaction-start time would pass |
| Foreign/missing IDs and forged in-memory event fields | No cross-tenant payload; stored claim controls attempts and message |
| Reader wins preparation, then transport blocks | Organization/user/project writers and a size-one pool remain available |
| Cancellation/deadline while preparing | No sender; no retained authority lock or pool slot |
| Existing success/retry/failure/disabled worker | Original behavior and stable message bytes/identity remain |

Use disposable PostgreSQL and executable local stubs only. Coordinate races with
observed database blocking or explicit IPC, not guessed sleeps. Run notification
and outbox tests, affected/full server regressions, complete SQLx preparation,
offline all-targets Clippy, WASM and formatting before publishing verified code.
Full OP37, budget enqueue integration, T042 and policy cutover remain open.
