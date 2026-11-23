# Quickstart: OAuth 2.0 with PKCE

Runnable checks that the feature works end to end. The `oauth-flow` Nix check (PR G) automates
scenarios 1 to 5 with a scripted client; the commands below are the manual version against the dev
stack.

## Prerequisites

- `process-compose up` with `DEV_LOGIN=1` and `HORAE_PUBLIC_URL=http://127.0.0.1:8080` (both set by
  the stack, [contracts/configuration.md](contracts/configuration.md)).
- `curl`, `jq`, `openssl`.
- Sign in at http://127.0.0.1:8080/auth/login as the demo administrator.

## 1. Register a public app (US3)

In Administration → Apps, register "Quickstart app", type "Installed on people's devices", redirect
URI `http://127.0.0.1/callback`, all three scopes. Copy its client id.

```sh
CLIENT_ID=horae_app_…
```

Expected: the app is listed as enabled; no secret is shown. Registering `http://localhost/callback`
or `https://example.com/*` is refused with a reason.

## 2. Authorize with PKCE (US1)

```sh
VERIFIER=$(openssl rand -base64 48 | tr '+/' '-_' | tr -d '=\n' | cut -c1-64)
CHALLENGE=$(printf %s "$VERIFIER" | openssl dgst -sha256 -binary | openssl base64 | tr '+/' '-_' | tr -d '=\n')
STATE=$(openssl rand -hex 8)
echo "http://127.0.0.1:8080/oauth/authorize?response_type=code&client_id=$CLIENT_ID&redirect_uri=http://127.0.0.1:53682/callback&code_challenge=$CHALLENGE&code_challenge_method=S256&state=$STATE&scope=time:read%20catalog:read"
```

Open the printed URL in the signed-in browser. Expected: the approval page names "Quickstart app",
the demo administrator and the organization, and lists two scope sentences. Click Allow. The
browser goes to `http://127.0.0.1:53682/callback?code=horae_oac_…&state=<STATE>&iss=http://127.0.0.1:8080`
(the page fails to load, since nothing listens; copy the `code` from the address bar). The port
differs from the registration and is still accepted (loopback).

Signed out, the same URL goes through `/auth/login` and lands back on the approval page.

## 3. Exchange the code (US1)

```sh
CODE=horae_oac_…
curl -s http://127.0.0.1:8080/oauth/token \
  -d grant_type=authorization_code -d client_id=$CLIENT_ID -d code=$CODE \
  -d redirect_uri=http://127.0.0.1:53682/callback -d code_verifier=$VERIFIER | tee /tmp/tokens.json | jq
```

Expected: `token_type` `Bearer`, `expires_in` 3600, `scope` `"time:read catalog:read"`, an access
and a refresh token. Running the same command again answers `invalid_grant`, and the tokens from the
first run stop working (step 4 then answers 401).

## 4. Call the APIs (US1, US5)

```sh
AT=$(jq -r .access_token /tmp/tokens.json)
curl -s -H "Authorization: Bearer $AT" http://127.0.0.1:8080/api/v1/me | jq .token
curl -s -H "Authorization: Bearer $AT" "http://127.0.0.1:8080/api/v1/time_entries?limit=5" | jq '.data | length'
curl -si -H "Authorization: Bearer $AT" -H 'Content-Type: application/json' \
  -d '{"project_id":"…","task_id":"…","spent_date":"2026-11-22","minutes":30}' \
  http://127.0.0.1:8080/api/v1/time_entries | head -1
```

Expected: `token.kind` is `oauth` with the app's name and scopes; the list answers; the create
answers 403 with `insufficient_scope` and a `WWW-Authenticate` header naming `time:write`.

With `HORAE_HARVEST_API=1`, the same token reads `/harvest/v2/time_entries` with
`Harvest-Account-Id`, and is refused as `?access_token=$AT`.

## 5. Refresh and reuse (US2)

```sh
RT=$(jq -r .refresh_token /tmp/tokens.json)
curl -s http://127.0.0.1:8080/oauth/token -d grant_type=refresh_token -d client_id=$CLIENT_ID -d refresh_token=$RT | jq .refresh_token
curl -s http://127.0.0.1:8080/oauth/token -d grant_type=refresh_token -d client_id=$CLIENT_ID -d refresh_token=$RT | jq
```

Expected: the first call returns a new refresh token; the second answers `invalid_grant`, and every
token of the authorization now answers 401. Administration → Audit log, kind "Apps", shows the
reuse (D-4).

## 6. Revoke (US4)

Authorize again (steps 2 and 3). In My settings → Connected apps, revoke "Quickstart app". The
access token answers 401 at once. As administrator, the app's page lists the authorizations of
every person with Revoke. `POST /oauth/revoke` with `token=$AT&client_id=$CLIENT_ID` answers 200
and ends the authorization too.

## 7. Discovery

```sh
curl -s http://127.0.0.1:8080/.well-known/oauth-authorization-server | jq
```

Expected: the document of [contracts/oauth-endpoints.md](contracts/oauth-endpoints.md) with
`code_challenge_methods_supported: ["S256"]`.

## Test suites

```sh
cargo test -p horae-core oauth
DATABASE_URL=… cargo test -p horae-server oauth
DATABASE_URL=… cargo test -p horae-api oauth
DATABASE_URL=… cargo test -p horae-harvest-api oauth
DATABASE_URL=… cargo test -p horae --features server oauth
nix build .#checks.x86_64-linux.oauth-flow
```
