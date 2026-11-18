# Quickstart: validating `/api/v1`

Runnable checks that prove the feature end to end, one section per user story. Shapes are in
[contracts/api-v1.md](contracts/api-v1.md); configuration in
[contracts/configuration.md](contracts/configuration.md).

## Prerequisites

```sh
nix develop
process-compose up            # PostgreSQL, migrations, seed, the app on :8080 with DEV_LOGIN=1
```

Sign in at <http://localhost:8080/auth/login> ("Sign in as Admin").

## US1: a script reads with a token

1. Personal settings → API tokens → create "quickstart". Copy the token (shown once).

1. ```sh
   export H=http://localhost:8080 T=horae_pat_…
   curl -s -H "Authorization: Bearer $T" $H/api/v1/me | jq
   curl -s -H "Authorization: Bearer $T" "$H/api/v1/time_entries?from=2026-11-16&to=2026-11-22" | jq '.data | length'
   ```

   Expected: `me` names the admin and the token; the count equals the timesheet's entries for the
   week.

1. Without the header, with only the browser cookie, or with a revoked token:
   `curl -s -o /dev/null -w '%{http_code}' $H/api/v1/me` → `401`, body `code: unauthenticated`.

1. In the database, `SELECT token_sha256, display_prefix FROM api_tokens` shows a digest and a
   prefix, never the token.

## US2: a client from the description

```sh
curl -s $H/api/v1/openapi.json | jq -S . | diff - <(jq -S . crates/api/openapi/v1.json)   # no output
nix build .#checks.x86_64-linux.api-compat           # oasdiff: no breaking change vs the baseline
nix build .#checks.x86_64-linux.api-v1               # generated client runs the US1/US3 scenarios
```

Contract drift: remove a response field in `horae-api::v1`, run
`cargo test -p horae-api openapi`; it fails until the snapshot is refreshed, and the
`api-compat` check then fails on the removed field.

## US3: writes go through the web app's rules

```sh
P=<project id> K=<task id>   # from GET /api/v1/me/trackable
curl -s -H "Authorization: Bearer $T" -H 'Content-Type: application/json' \
  -d "{\"project_id\":\"$P\",\"task_id\":\"$K\",\"spent_date\":\"$(date +%F)\",\"minutes\":90}" \
  $H/api/v1/time_entries | jq
curl -s -X POST -H "Authorization: Bearer $T" -H 'Content-Type: application/json' \
  -d "{\"project_id\":\"$P\",\"task_id\":\"$K\"}" $H/api/v1/timers | jq '.is_running'   # true
```

Expected: the timesheet shows both entries; starting a second timer stops the first (D-6) in the
web app too. Submit the week for approval in the web app, then `PATCH` an entry of that week:
409 with the same reason the timesheet shows. An unknown body field gives 422 `unknown_field`.

## US4: the Harvest-compatible API switch

```sh
curl -s -o /dev/null -w '%{http_code}\n' $H/harvest/v2/users/me   # 404 (default off)
HORAE_HARVEST_API=1 process-compose up                             # or set it in process-compose.yaml
curl -s -H "Authorization: Bearer $T" $H/harvest/v2/users/me | jq '.id'   # an integer (after D2)
curl -s -o /dev/null -w '%{http_code}\n' $H/api/v2/users/me       # 404 (alias removed)
```

NixOS: `nix build .#checks.x86_64-linux.e2e` covers both settings of
`services.horae.harvestApi.enable`.

## US5: administrators oversee tokens

Administration → API tokens lists every token with owner and last use; revoke one, then use it:
401\. As a member, `list_org_api_tokens` answers `FORBIDDEN`.

## US6: a sync walks a large collection

```sh
cargo run -p horae --features server -- seed     # or a larger fixture
next=""; n=0
while :; do
  page=$(curl -s -H "Authorization: Bearer $T" "$H/api/v1/time_entries?limit=1000${next:+&cursor=$next}")
  n=$((n + $(jq '.data | length' <<<"$page"))); next=$(jq -r '.next_cursor // empty' <<<"$page")
  [ -z "$next" ] && break
done; echo "$n"     # equals SELECT count(*) FROM time_entries visible to the admin
```

Alter one byte of a cursor: 400 `invalid_cursor`. Edit one entry, then list with
`updated_since=<the walk's start time>`: exactly that entry.

## Automated

```sh
cargo test -p horae-core
DATABASE_URL=… cargo test -p horae-server            # ops: tokens, reads, restart, D-6
DATABASE_URL=… cargo test -p horae-api               # v1 router tests, schema validation, cursor, limiter
DATABASE_URL=… cargo test -p horae-harvest-api       # the moved Harvest suite
DATABASE_URL=… cargo test -p horae --features server # server functions and pages
nix flake check                                      # adds api-compat and api-v1
```
