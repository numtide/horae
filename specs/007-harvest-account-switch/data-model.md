# Data Model: Safe Harvest Account Switching

## Connection state

`harvest_connection_generations`: `org_id` UUID primary key and organization FK, `account_generation` bigint ≥0, `connection_revision` bigint ≥0. Initialize existing organizations at zero; create lazily for future organizations. Never delete this record on disconnect or change.

Change account increments both counters and deletes only the organization's credential and binding rows atomically. Disconnect preserves binding/account generation but advances revision even when already disconnected, invalidating pending OAuth. Successful credential store advances revision. Token refresh leaves both counters unchanged.

## Jobs

`horae_jobs.account_generation` bigint ≥0 defaults to zero for legacy jobs. API enqueue captures the expected current generation under the connection gate. A duplicate identity must match both payload and generation. API retry and execution reject a noncurrent generation. CSV source identity is unaffected, although active CSV work blocks changing accounts.

## Authorization attempt and confirmation

Session authorization state captures nonce, actor identity/organization, account generation and connection revision. Store the structured value as JSON text inside the session: the MessagePack session backend otherwise cannot round-trip arbitrary-precision JSON integer values. Old nonce-only state fails closed. Confirm Change account submits inspected account identity plus generation/revision, so another administrator's connection update invalidates stale confirmation.

## Public connection status

Expose configuration/connection flags, bound account identity even while disconnected, token expiry, generation/revision and eligibility inputs (imported identities and active Harvest work). No token, encryption key or arbitrary SQL error is returned.
