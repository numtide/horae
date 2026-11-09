#!/usr/bin/env bash
# Harvest API compatibility suite (specs/021-harvest-api-compat, FR-070): start
# a seeded Horae with its own PostgreSQL, then run each client's workflow
# against it. Every argument is a client executable; each reads
# HORAE_API_BASE and HORAE_SESSION_COOKIE and exits non-zero on a failure.
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

if curl -s --max-time 1 -o /dev/null "$url/health"; then
  echo 'Port 8094 is occupied; refusing to test an existing instance' >&2
  exit 1
fi
cd "$(dirname "$HORAE_TEST_SERVER")"
DEV_LOGIN=1 IP=127.0.0.1 PORT=8094 RUST_LOG=warn "$HORAE_TEST_SERVER" serve >"$test_dir/server.log" 2>&1 &
app_pid=$!
for _attempt in {1..100}; do
  kill -0 "$app_pid"
  if curl -fsS "$url/health" >/dev/null 2>&1; then break; fi
  sleep 0.1
done
curl -fsS "$url/health" >/dev/null

# Until personal access tokens exist, clients read with the dev login's session.
curl -fsS -c "$test_dir/cookies" -o /dev/null -X POST "$url/auth/dev-login"
HORAE_SESSION_COOKIE=$(awk -F '\t' 'NF == 7 { printf "%s%s=%s", sep, $6, $7; sep = "; " }' "$test_dir/cookies")
export HORAE_SESSION_COOKIE
export HORAE_API_BASE="$url/v2/"

for client in "$@"; do
  echo "== $(basename "$client")"
  "$client"
done
