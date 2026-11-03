# Contract: Harvest OAuth client server functions

All three are Dioxus server functions under the session layer. Callers who are not signed in get `UNAUTHORIZED`; non-administrators get `FORBIDDEN` ("Admin access required"). The organization always comes from the session, never from the body.

## `harvest_client_settings() -> HarvestClientSettings`

`POST /api/import/harvest/client`. Returns the view from [data-model.md](../data-model.md). It never includes a secret, ciphertext or key.

## `save_harvest_client(client_id, client_secret: Option<String>, redirect_url: String, confirm_disconnect: bool) -> HarvestClientSettings`

`POST /api/import/harvest/client/save`. A blank `client_secret` (`None` or empty) keeps the stored secret, which is allowed only for an unchanged, readable client ID.

| Condition | Code |
|---|---|
| Invalid input (data-model rules), or new client ID / unreadable secret without a new secret | `BAD_REQUEST` |
| Encryption key not configured | `CONFLICT`: "Set HORAE_HARVEST_ENC_KEY …" |
| Environment client in effect | `CONFLICT`: "configured by the environment" |
| Harvest import queued, running, or holding the import lock | `CONFLICT`: "Finish or cancel the active Harvest import …" |
| Administrator lost authority after the wait | `FORBIDDEN` |
| The change would delete an existing connection's tokens and `confirm_disconnect` is false | `CONFLICT`: `DISCONNECT_UNCONFIRMED`; nothing changes, and the page shows the warning and retries with the flag |

On success, the secret is sealed and the row upserted. Tokens are deleted when the client ID changed. `connection_revision` is bumped, an audit row is written, and the new view is returned.

## `remove_harvest_client(confirm_disconnect: bool) -> HarvestClientSettings`

`POST /api/import/harvest/client/remove`. Has the same refusals as save, except input validation and the missing encryption key: a client whose secret no longer opens can still be removed. Deletes the row and any tokens (the binding is kept), bumps the revision and writes an audit row. Removing when no row exists returns the view unchanged and writes nothing.

## Effects on existing endpoints

- `harvest_connection_status().configured` = `resolve(org).is_some()`.
- `harvest_connect_start`, the OAuth callback, `start_harvest_api_import` and the job worker use `resolve(org)` and answer "not configured" (`NOT_FOUND`) as before when it is `None`.
- An authorization started before any save or remove is rejected at the callback (revision mismatch); no credentials are stored.
