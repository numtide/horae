# Contract: personal access tokens (PR C)

021's [token contract](../../021-harvest-api-compat/contracts/api-tokens.md), implemented once
for both APIs. Issue and revocation are mutations, so they are Dioxus `#[server]` functions over
`ops::api_tokens` (constitution IV). The functions live in
`crates/horae/src/server_fns/api_tokens.rs`.

| Server function | Who | Input | Output | Errors |
| --- | --- | --- | --- | --- |
| `list_api_tokens()` | any signed-in person | | own tokens: id, name, display prefix, created, expires, last used, revoked | |
| `create_api_token(name, expires_at)` | any signed-in person | name 1–100 characters; optional future expiry | the token string, **returned once**, plus the row | `BAD_REQUEST` on name or expiry |
| `revoke_api_token(id)` | the owner, or an administrator | token id | the row | `NOT_FOUND` for another person's token unless administrator |
| `list_org_api_tokens()` | administrator | | every person's tokens with owner name, without secrets | `FORBIDDEN` |

Differences from 021's contract: `api_account_id()` is not part of this feature (it is the Harvest
API's account id and is shown by PR D2's page section only when the Harvest API is on).

## Operations (`crates/server/src/ops/api_tokens.rs`)

| Operation | Rule |
| --- | --- |
| `list_own(tx, actor)` | the actor's tokens, newest first |
| `create(tx, actor, name, expires_at) -> (Secret, ApiToken)` | active actor; name trimmed, 1–100; expiry in the future |
| `revoke(tx, actor, id)` | owner or `actor.administrator()`; repeating it is a no-op that returns the row |
| `list_org(tx, actor)` | `actor.require_administrator()` |
| `authenticate(pool, secret) -> Option<Caller>` | digest lookup; unrevoked, unexpired, owner active; stamps `last_used_at` at most once a minute; returns `Caller::token(org_id, user_id, token_id)` |

`authenticate` runs before any transaction, like `active_caller` for sessions; `Actor::load` then
checks the person again inside the operation's transaction.

## UI

- **Personal settings** (`crates/horae/src/pages/settings/api_tokens.rs`), section "API tokens": the
  token list (name, prefix, created, expires, last used), a create form (name, optional expiry),
  a one-time reveal with a copy button and a note that it will not be shown again, and revoke
  with confirmation. A link to the API description (`/api/v1/openapi.json`).
- **Administration** (`crates/horae/src/pages/admin/api_tokens.rs`), administrators only: every
  token with owner, name, prefix, created, expires and last used, and revoke.
- Wording is Horae's own; layout follows `DESIGN.md` tokens.
