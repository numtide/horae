# Contract: personal access token management (step 2)

Token issue and revocation are mutations, so they are Dioxus `#[server]` functions
(Principle IV, and the amendment's last bullet). They live in
`crates/horae/src/server_fns/api_tokens.rs`.

| Server function | Who | Input | Output | Errors |
| --- | --- | --- | --- | --- |
| `list_api_tokens()` | any signed-in person | | own tokens: id, name, display prefix, created, expires, last used, revoked | |
| `create_api_token(name, expires_at)` | any signed-in person | name 1–100 chars; optional future expiry | the token string, **returned once**, plus the row | `BAD_REQUEST` on name or expiry |
| `revoke_api_token(id)` | the owner, or an administrator | token id | the row | `NOT_FOUND` for another person's token unless administrator |
| `list_org_api_tokens()` | administrator | | every person's tokens, without secrets | `FORBIDDEN` |
| `api_account_id()` | any signed-in person | | the organization's account id | |

## UI

- Personal settings, section "API tokens": the account id, the token list, a create form, and a
  one-time reveal of a new token with a copy button. Revoking asks for confirmation.
- Organization settings, administrators only: every token with owner and last use, and revoke.
- Wording is Horae's own (AGENTS.md *Harvest parity*); the layout follows `DESIGN.md` tokens.
