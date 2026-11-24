# Contract: OAuth endpoints (PR C)

All under the instance's public base URL (`HORAE_PUBLIC_URL`, the issuer). Without it the
authorization server is off: these paths answer 404 and the administration pages say why
([configuration.md](configuration.md)). Each handler parses the request, calls `ops::oauth` and
renders the result; no rule lives here (Principle IV).

## `GET /oauth/authorize`

Query parameters (RFC 6749 §4.1.1, RFC 7636 §4.3):

| Parameter | Required | Rule |
| --- | --- | --- |
| `response_type` | yes | `code` |
| `client_id` | yes | an enabled client of the organization |
| `redirect_uri` | yes | matches a registered URI (exact, or loopback with any port) |
| `code_challenge` | yes, for public and confidential clients (D-7) | 43 characters base64url, the length of every S256 challenge |
| `code_challenge_method` | yes | `S256` |
| `state` | recommended | echoed unchanged, at most 1,024 characters |
| `scope` | no | space-separated; defaults to the client's scopes |

Answers, in order of checks:

1. Unknown or disabled `client_id`, missing or unmatched `redirect_uri`: **400 HTML page** from
   the catalog ("This app can't connect to Horae") with the reason; never a redirect (FR-022).
1. Any other invalid parameter: **302** to `redirect_uri` with `error` (`invalid_request`,
   `unsupported_response_type`, `invalid_scope`), `error_description`, `state`, `iss`.
1. Valid: the request is stored in the session (data-model *Session data*) and the answer is **302**
   to `/oauth/consent?request=<id>`. If the person is signed out, the guard then sends them to
   `/auth/login` and back (research R-3).

Every answer has `Cache-Control: no-store`, `X-Frame-Options: DENY`,
`Content-Security-Policy: frame-ancestors 'none'` and `Referrer-Policy: no-referrer`.

## `GET /oauth/consent?request=<id>` (Dioxus page)

Described in [server-functions.md](server-functions.md). Allow ends in a full navigation to:

```text
<redirect_uri>?code=horae_oac_…&state=<state>&iss=<issuer>
```

Deny ends in `<redirect_uri>?error=access_denied&state=<state>&iss=<issuer>`. A request id that is
unknown or expired shows the "request expired, start again from the app" state and offers no
action.

## `POST /oauth/token`

`Content-Type: application/x-www-form-urlencoded`. Client authentication: HTTP Basic or
`client_id` + `client_secret` in the body for confidential clients; `client_id` alone for public
clients (research R-12).

### `grant_type=authorization_code`

| Parameter | Rule |
| --- | --- |
| `code` | unused, unexpired, issued to this client |
| `redirect_uri` | identical to the authorization request's |
| `code_verifier` | 43–128 unreserved characters; `S256(verifier) == code_challenge` |
| `client_id` | required for public clients |

### `grant_type=refresh_token`

| Parameter | Rule |
| --- | --- |
| `refresh_token` | unused, within its idle expiry, its grant active |
| `scope` | optional; a subset of the refresh token's scopes |

### Success (200)

```json
{
  "access_token": "horae_oat_…",
  "token_type": "Bearer",
  "expires_in": 3600,
  "refresh_token": "horae_ort_…",
  "scope": "time:read time:write catalog:read"
}
```

Headers: `Cache-Control: no-store`, `Pragma: no-cache`.

### Errors (RFC 6749 §5.2)

| Case | Status | `error` |
| --- | --- | --- |
| Malformed body, missing parameter, verifier of the wrong shape | 400 | `invalid_request` |
| Unknown client, bad secret, public client sending a secret, confidential client without one | 401 (with `WWW-Authenticate: Basic` when Basic was tried) | `invalid_client` |
| Code or refresh token unknown, expired, used, of another client; wrong `redirect_uri`; PKCE mismatch | 400 | `invalid_grant` |
| Grant type other than the two | 400 | `unsupported_grant_type` |
| Refresh asks for scopes outside the token's | 400 | `invalid_scope` |
| Over the per-client limit | 429 + `Retry-After` | `slow_down` is not used; body `{"error":"invalid_request","error_description":"Too many requests"}` |

A reused code or refresh token answers `invalid_grant` and revokes the grant (FR-031, FR-043).
`error_description` never contains a credential.

## `POST /oauth/revoke` (RFC 7009)

Form body: `token`, optional `token_type_hint` (`access_token`, `refresh_token`), plus client
authentication as for the token endpoint. A token of this client ends its grant. Answers **200**
with an empty body for a revoked, unknown, expired or already revoked token (RFC 7009 §2.2); 401
`invalid_client` when the client fails to authenticate.

## `GET /.well-known/oauth-authorization-server` (RFC 8414)

Public, `Cache-Control: max-age=3600`:

```json
{
  "issuer": "https://horae.example.com",
  "authorization_endpoint": "https://horae.example.com/oauth/authorize",
  "token_endpoint": "https://horae.example.com/oauth/token",
  "revocation_endpoint": "https://horae.example.com/oauth/revoke",
  "response_types_supported": ["code"],
  "grant_types_supported": ["authorization_code", "refresh_token"],
  "code_challenge_methods_supported": ["S256"],
  "token_endpoint_auth_methods_supported": ["none", "client_secret_basic", "client_secret_post"],
  "revocation_endpoint_auth_methods_supported": ["none", "client_secret_basic", "client_secret_post"],
  "scopes_supported": ["time:read", "time:write", "catalog:read"],
  "authorization_response_iss_parameter_supported": true,
  "service_documentation": "https://horae.example.com/api/v1/openapi.json"
}
```

## Operations behind them (`crates/server/src/ops/oauth/`)

| Operation | Called by | Rule |
| --- | --- | --- |
| `authorize::validate_request(pool, org, params) -> Result<PendingRequest, AuthorizeError>` | `GET /oauth/authorize` | read-only; splits errors into "show on page" and "return to client" |
| `authorize::approve(tx, actor, pending) -> Done<Redirect>` | server function | active actor; client enabled; creates grant and code; audit `grant_approved` |
| `authorize::deny(pending) -> Redirect` | server function | no write |
| `tokens::exchange_code(tx, client_auth, code, redirect_uri, verifier) -> Done<TokenSet>` | `/oauth/token` | R-9; reuse revokes |
| `tokens::refresh(tx, client_auth, refresh_token, scope) -> Done<TokenSet>` | `/oauth/token` | R-8; reuse revokes |
| `tokens::revoke_token(tx, client_auth, token) -> Done<()>` | `/oauth/revoke` | ends the grant, reason `client` |
| `tokens::authenticate(pool, token) -> Option<Caller>` | both APIs | digest lookup joined with grant, client and person; stamps `last_used_at` once a minute; returns `Channel::OAuth` |

The token endpoint's operations have no `Actor`: their authority is the presented code with its
verifier, the refresh token, or the client secret (constitution amendment, approved in D-3).
