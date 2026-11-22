# Contract: changes to the two APIs (PRs E and F)

Both APIs accept OAuth access tokens beside personal access tokens. Every change to `/api/v1` is
an addition (022 FR-050).

## Authentication

| | Personal access token | OAuth access token |
| --- | --- | --- |
| Prefix | `horae_pat_` | `horae_oat_` |
| `/api/v1`, `Authorization: Bearer` | yes | yes |
| Harvest API, `Authorization: Bearer` | yes | yes |
| Harvest API, `access_token` query parameter | yes | **no**, 401 (D-5) |
| Scopes apply | no (full authority of the person) | yes |
| Rate limit key | token | grant |
| `Harvest-Account-Id` check | yes | yes |

`horae_api::token::ApiCaller` picks the authenticator by prefix; any other prefix is 401.

## Scopes per operation

### `/api/v1`

| Operation | Scope |
| --- | --- |
| `GET /me` | any |
| `GET /me/trackable`, `GET /me/timer` | `catalog:read` for trackable, `time:read` for timer |
| `GET /time_entries`, `GET /time_entries/{id}` | `time:read` |
| `POST /time_entries`, `PATCH`/`DELETE /time_entries/{id}`, `POST /time_entries/{id}/stop`, `POST /time_entries/{id}/restart`, `POST /timers` | `time:write` |
| `GET /projects`, `/projects/{id}`, `/tasks`, `/tasks/{id}`, `/clients`, `/clients/{id}`, `/people`, `/people/{id}` | `catalog:read` |
| `GET /openapi.json` | public |

`time:write` includes `time:read` (`ScopeSet::normalize`).

### Harvest API

| Route | Scope |
| --- | --- |
| `/company`, `/users/me` | any |
| `/users/me/project_assignments`, `/projects`, `/projects/{id}`, `/clients`, `/clients/{id}`, `/tasks`, `/tasks/{id}`, `/users` | `catalog:read` |
| `GET /time_entries`, `GET /time_entries/{id}` | `time:read` |
| `POST /time_entries`, `PATCH`/`DELETE /time_entries/{id}`, `PATCH /time_entries/{id}/stop`, `PATCH /time_entries/{id}/restart` | `time:write` |

A new Harvest route must name its scope in the route table; a test fails otherwise.

## Errors

| Case | `/api/v1` | Harvest API |
| --- | --- | --- |
| Unknown, expired or revoked token; grant revoked; app disabled; person inactive | 401 problem `unauthenticated`, `WWW-Authenticate: Bearer error="invalid_token"` | 401 in its shape, same header |
| Scope missing | 403 problem `insufficient_scope`, `WWW-Authenticate: Bearer error="insufficient_scope", scope="<needed>"` | 403 in its shape, same header |
| Permission missing (scope present) | unchanged (the operation's `forbidden` / `not_found`) | unchanged |

## `GET /api/v1/me`: additions

`token` gains two fields; existing fields keep their meaning.

```json
"token": {
  "id": "…",            // the personal token, or the OAuth access token
  "name": "…",          // the personal token's name, or the app's name
  "expires_at": "…",
  "kind": "personal",   // new: "personal" | "oauth"
  "oauth": null         // new: {"client_id": "…", "app_name": "…", "scopes": ["time:read"]} for "oauth"
}
```

## OpenAPI document

- New security scheme `oauth2`, type `oauth2`, flow `authorizationCode` with `authorizationUrl`
  `/oauth/authorize`, `tokenUrl` `/oauth/token`, `refreshUrl` `/oauth/token` and the three scopes
  with their descriptions.
- Each token operation's `security` becomes `[{"token": []}, {"oauth2": ["<scope>"]}]` (either
  one). `/me` lists `{"oauth2": []}`.
- New problem code `insufficient_scope` in the documented codes.
- The snapshot `crates/api/openapi/v1.json` is refreshed; `v1.baseline.json` changes only if
  oasdiff flags the security addition (research R-18).
