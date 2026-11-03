# Research: Harvest OAuth Client Settings

## 1. Runtime resolution instead of a startup snapshot

- **Decision**: Keep the environment part (`HarvestDeployment`: optional client pair, optional redirect URL, optional key) in `AppState`. Every use calls `client_settings::resolve(db, &deployment, org_id)` to get `Option<HarvestConfig>`.
- **Rationale**: One indexed read per connect, status, enqueue or job keeps "change without restart" trivially correct with several server processes. Consumers (`oauth.rs`, the importer engine) keep taking `&HarvestConfig`, so they don't change.
- **Alternatives**: an in-memory cache with invalidation. Rejected: it breaks with more than one process, and the read is not hot.

## 2. Precedence

- **Decision**: The client ID and secret come from the environment as a pair when both are set; otherwise from the stored row. The redirect URL comes from the environment if set, otherwise from the stored row, but only while the stored client is in effect: an environment client ignores the stored row entirely. The key is environment only. The effective client exists only when all four parts are present and the stored secret decrypts.
- **Rationale**: Matches the requester's rule and today's "partial environment means unconfigured" behaviour for the pair. A deployment that sets only `HORAE_HARVEST_REDIRECT_URL` and the key gets a fixed callback with a page-managed client.

## 3. Sealing

- **Decision**: `credentials::encrypt`/`decrypt` (XChaCha20-Poly1305, random 24-byte nonce prefix) with `HORAE_HARVEST_ENC_KEY`. If decryption fails, `resolve` returns `None` and logs a warning without the value. The view reports `secret_readable = false`.
- **Rationale**: Reuse the existing scheme. Key rotation already has "reconnect" as its recovery; for the client, the recovery is "enter the secret again".

## 4. Effect on connections, jobs and pending authorizations

- **Decision**: Save and remove run in the same shape as `account_switch::change`: `lock_import` (a running import makes it `Busy`), transaction, `authorize_change`, `gate`, refusal when `status().active_imports > 0`, then the write and a `connection_revision` bump. A client ID change or a removal also deletes `harvest_credentials`. `harvest_account_bindings` and `account_generation` stay untouched.
- **Rationale**: Tokens are issued to a client, so a new client cannot refresh them; leaving them would only fail later. Disconnect semantics keep the binding, so only the original account can reconnect (the account-switch invariant). The revision bump makes `complete_connect` and `store_for_attempt` reject a callback started under the old client, the same way they reject one started before a disconnect. Same-account job retries keep working because the generation is unchanged.
- **Alternatives**: keeping the tokens on a client change. Rejected: refresh fails against Harvest and the error surfaces mid-import. Blocking only API jobs: the existing blocker counts both kinds; reused as is.

## 5. Redirect URL source

- **Decision**: The page shows `HORAE_HARVEST_REDIRECT_URL` when it is set. Otherwise it shows `window.location.origin + "/auth/harvest/callback"`, read with `document::eval`, which the page already uses for focus. It submits that value with the save. The server validates it as absolute `https` (plain `http` only for `localhost`, `127.0.0.1` and `[::1]`), without credentials, query or fragment, with an optional numeric port and the exact path `/auth/harvest/callback`.
- **Rationale**: The design shows the URI read-only with Copy. The admin's browser origin is the public address Harvest will redirect that browser to. Reading it server-side from `Host` would trust a header behind proxies.

## 6. Audit

- **Decision**: Insert into the existing `audit_log` (`action` `harvest.oauth_client.save` / `harvest.oauth_client.remove`, `entity_type` `harvest_oauth_client`, `data` = `{client_id, disconnected}`) in the same transaction, and emit `tracing::info!` with org, actor and action.
- **Rationale**: The table exists for exactly this; no secret goes in.

## 7. Environment-managed client

- **Decision**: While the environment pair is set, save and remove are refused with `CONFLICT`, and the view reports `source = Environment` plus `stored_ignored` when a row exists.
- **Rationale**: The page reads as locked (design footer). Editing a client that cannot take effect is misleading.
