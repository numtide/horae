#!/usr/bin/env bash
# /api/v1 end-to-end check (spec 022, R-14): start a seeded Horae with its own
# PostgreSQL, generate a Python client from the /api/v1/openapi.json it serves,
# and run the quickstart scenarios with that client, unmodified.
set -euo pipefail

: "${HORAE_TEST_SERVER:?Set the absolute path to the built server}"
scenarios=$(realpath "$1")
test_dir=$(mktemp -d "${TMPDIR:-/tmp}/horae-api-v1.XXXXXX")
export PGDATA="$test_dir/postgres"
export DATABASE_URL="postgres://postgres@localhost/horae?host=$test_dir"
url=http://127.0.0.1:8095
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
"$HORAE_TEST_SERVER" seed --api-token-file "$test_dir/token" >/dev/null

if curl -s --max-time 1 -o /dev/null "$url/health"; then
  echo 'Port 8095 is occupied; refusing to test an existing instance' >&2
  exit 1
fi
cd "$(dirname "$HORAE_TEST_SERVER")"
IP=127.0.0.1 PORT=8095 RUST_LOG=warn "$HORAE_TEST_SERVER" serve >"$test_dir/server.log" 2>&1 &
app_pid=$!
for _attempt in {1..100}; do
  kill -0 "$app_pid"
  if curl -fsS "$url/health" >/dev/null 2>&1; then break; fi
  sleep 0.1
done
curl -fsS "$url/health" >/dev/null

curl -fsS "$url/api/v1/openapi.json" >"$test_dir/openapi.json"
openapi-generator-cli generate --input-spec "$test_dir/openapi.json" --generator-name python \
  --output "$test_dir/client" --package-name horae_client \
  --global-property apiTests=false,modelTests=false,apiDocs=false,modelDocs=false >/dev/null

HORAE_API_BASE="$url/api/v1" HORAE_ACCESS_TOKEN=$(<"$test_dir/token") \
PYTHONPATH="$test_dir/client" python3 "$scenarios"
