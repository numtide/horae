#!/usr/bin/env bash
# Harvest API compatibility suite (specs/021-harvest-api-compat, FR-070): start
# a seeded Horae with its own PostgreSQL, then run each client's workflow
# against it. Every argument is a client executable; each reads
# HORAE_API_BASE, HORAE_ACCESS_TOKEN and HORAE_ACCOUNT_ID and exits non-zero on
# a failure.
set -euo pipefail

: "${HORAE_TEST_SERVER:?Set the absolute path to the built server}"
test_dir=$(mktemp -d /tmp/horae-harvest-compat.XXXXXX)
export PGDATA="$test_dir/postgres"
export DATABASE_URL="postgres://postgres@localhost/horae?host=$test_dir"
url=http://127.0.0.1:8094
app_pid=
cleanup() {
  result=$?
  if [[ -n $app_pid ]]; then
    kill -TERM "$app_pid" 2>/dev/null || true
    wait "$app_pid" || true
  fi
  pg_ctl -D "$PGDATA" -m fast stop >/dev/null 2>&1 || true
  if [[ $result != 0 && -f "$test_dir/server.log" ]]; then
    tail -100 "$test_dir/server.log"
  elif [[ $result == 0 ]]; then
    rm -rf "$test_dir"
  fi
  exit "$result"
}
trap cleanup EXIT

initdb -D "$PGDATA" --no-locale --encoding=UTF8 -U postgres >/dev/null
pg_ctl -D "$PGDATA" -l "$test_dir/postgres.log" \
  -o "--unix_socket_directories=$test_dir --listen_addresses= -c fsync=off" start >/dev/null
createdb -h "$test_dir" -U postgres horae
"$HORAE_TEST_SERVER" migrate run
"$HORAE_TEST_SERVER" seed >/dev/null

# A personal access token of the seeded administrator, stored as Horae stores
# one (its SHA-256 only); issuing one otherwise takes the web app. The id is a
# UUID v7: the time in milliseconds, then random bits.
HORAE_ACCESS_TOKEN="horae_pat_$(head -c 32 /dev/urandom | basenc --base64url | tr -d '=\n')"
digest=$(printf '%s' "$HORAE_ACCESS_TOKEN" | sha256sum | cut -d ' ' -f 1)
random=$(od -An -N9 -tx1 /dev/urandom | tr -d ' \n')
millis=$(printf '%012x' "$(date +%s%3N)")
token_id="${millis:0:8}-${millis:8:4}-7${random:0:3}-8${random:3:3}-${random:6:12}"
HORAE_ACCOUNT_ID=$(psql -h "$test_dir" -U postgres -d horae -v ON_ERROR_STOP=1 -qtA -c "
  INSERT INTO api_tokens (id, org_id, user_id, name, token_sha256, display_prefix)
  SELECT '$token_id', u.org_id, u.id, 'harvest-compat', decode('$digest', 'hex'),
         substr('$HORAE_ACCESS_TOKEN', 11, 8)
  FROM users u
  JOIN person_permission_states p ON p.org_id = u.org_id AND p.user_id = u.id
  WHERE p.is_administrator AND u.active
  ORDER BY u.id LIMIT 1
  RETURNING (SELECT api_account_id FROM organizations o WHERE o.id = api_tokens.org_id)")
if [[ -z $HORAE_ACCOUNT_ID ]]; then
  echo 'The seed has no active administrator to issue a token to' >&2
  exit 1
fi
export HORAE_ACCESS_TOKEN HORAE_ACCOUNT_ID

if curl -s --max-time 1 -o /dev/null "$url/health"; then
  echo 'Port 8094 is occupied; refusing to test an existing instance' >&2
  exit 1
fi
cd "$(dirname "$HORAE_TEST_SERVER")"
HORAE_HARVEST_API=1 IP=127.0.0.1 PORT=8094 RUST_LOG=warn "$HORAE_TEST_SERVER" serve >"$test_dir/server.log" 2>&1 &
app_pid=$!
for _attempt in {1..100}; do
  kill -0 "$app_pid"
  if curl -fsS "$url/health" >/dev/null 2>&1; then break; fi
  sleep 0.1
done
curl -fsS "$url/health" >/dev/null

export HORAE_API_BASE="$url/v2/"

for client in "$@"; do
  echo "== $(basename "$client")"
  "$client"
done
