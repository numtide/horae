#!/usr/bin/env bash
set -euo pipefail

# Browser fixtures must never inherit a developer's real outbound mail transport.
unset HORAE_SENDMAIL_PATH HORAE_MAIL_FROM

# CI supplies a built server with its public/ directory beside it. Everything
# else is created here: this runner never connects to an existing database.
: "${HORAE_TEST_SERVER:?Set the absolute path to the built server}"
: "${PLAYWRIGHT_MODULE:?Set the path to playwright/test}"
browser_tests=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
test_dir=$(mktemp -d)
export PGDATA="$test_dir/postgres"
export DATABASE_URL="postgres://postgres@localhost/horae?host=$test_dir"
export HORAE_TEST_URL=http://127.0.0.1:8093
app_pid=
cleanup() {
  result=$?
  if [[ -n $app_pid ]]; then
    kill -TERM "$app_pid" 2>/dev/null || true
    wait "$app_pid" || true
  fi
  pg_ctl -D "$PGDATA" -m fast stop >/dev/null 2>&1 || true
  if [[ $result != 0 ]]; then
    [[ ! -f "$test_dir/server.log" ]] || tail -100 "$test_dir/server.log"
    [[ ! -f "$test_dir/postgres.log" ]] || tail -100 "$test_dir/postgres.log"
  fi
  exit "$result"
}
trap cleanup EXIT

initdb -D "$PGDATA" --no-locale --encoding=UTF8 -U postgres >/dev/null
pg_ctl -D "$PGDATA" -l "$test_dir/postgres.log" \
  -o "--unix_socket_directories=$test_dir --listen_addresses= -c fsync=off" start
createdb -h "$test_dir" -U postgres horae
"$HORAE_TEST_SERVER" migrate run
"$HORAE_TEST_SERVER" seed
HORAE_TEST_WEEK=$(psql "$DATABASE_URL" -At -c 'SELECT min(spent_date) FROM time_entries')
export HORAE_TEST_WEEK

# Do not let an unrelated process on the test port satisfy the health probe.
if curl -s --max-time 1 -o /dev/null "$HORAE_TEST_URL/health"; then
  echo 'Port 8093 is occupied; refusing to test an existing instance' >&2
  exit 1
fi
cd "$(dirname "$HORAE_TEST_SERVER")"
DEV_LOGIN=1 IP=127.0.0.1 PORT=8093 RUST_LOG=warn "$HORAE_TEST_SERVER" serve >"$test_dir/server.log" 2>&1 &
app_pid=$!
for _attempt in {1..100}; do
  kill -0 "$app_pid"
  if curl -fsS "$HORAE_TEST_URL/health" >/dev/null 2>&1; then break; fi
  sleep 0.1
done
kill -0 "$app_pid"
curl -fsS "$HORAE_TEST_URL/health" >/dev/null

if [[ -n ${HORAE_STYLE_RECORD:-} ]]; then
  node "$browser_tests/shared-style-audit.cjs" record "$HORAE_STYLE_RECORD"
fi
if [[ -n ${HORAE_STYLE_BASELINE:-} ]]; then
  node "$browser_tests/shared-style-audit.cjs" compare "$HORAE_STYLE_BASELINE"
fi

# Explicit filenames allow focused iteration without bypassing database isolation.
suites=("$@")
if [[ ${#suites[@]} == 0 ]]; then
  suites=(projects-design responsive-layout menu-popovers mobile-navigation project-bulk-actions project-bulk-recovery action-errors new-project invoice-preparation project-task-rates new-project-permissions new-project-task-errors)
fi
for suite in "${suites[@]}"; do
  if [[ ! $suite =~ ^[a-z][a-z-]*$ || ! -f "$browser_tests/$suite.cjs" ]]; then
    echo "Unknown browser suite: $suite" >&2
    exit 1
  fi
  node "$browser_tests/$suite.cjs"
done
