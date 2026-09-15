# Quickstart: Harvest Jobs CLI

Acceptance guide for implementation; commands become runnable as tasks complete. Use disposable data.

## Setup

1. Enter `nix develop`; start the normal `process-compose up` stack. Connect Harvest through the existing admin UI for API scenarios.
1. Build `cargo build -p horae --features server`.
1. Sign in as administrator. Copy only the existing `id` cookie value through browser developer tools into a private JSON file outside the repository, following [data-model.md](./data-model.md). Set mode 0600. Never paste the secret into shell commands or tracked files.
1. Set `HORAE_SESSION_FILE` to its path (not the secret). Renew through normal login after expiry. No CLI login/permanent token is added.

## Scenarios

- `horae import harvest-csv sample.csv --dry-run --wait --timeout 120 --json`: compare status/history with web; zero imported writes. Commit unchanged input and reimport; zero duplicates. Repeat API preview/commit/full/incremental.
- Submit detached; after acknowledgement close the submitting process and remove the local CSV. Job completes independently.
- Resubmit identical input with the saved `--request-id`: same job. Change mode or bytes: conflict. Simulate lost acknowledgement. Expired key must fail.
- Page `horae jobs list --limit 1 --json` with `--before`; verify order and completeness on unchanged history.
- Run `horae jobs report UUID --json` and `horae jobs errors UUID --output errors.jsonl`; compare complete errors with web, including archived overflow. Existing destination without `--force` is unchanged on failure.
- Cancel while running, observe acknowledgement, retry after terminal cancellation and verify confirmed progress survives.
- Wait with `--timeout 1` on a queued job: exit 5 and retained job ID. Interrupt a longer waiter: exit 130, no cancellation request.
- Stop/restart server with pending work; CLI never starts workers or opens PostgreSQL. Resume observation and verify recovery without duplicates.
- Exercise every operation with absent/member/manager/inactive/demoted/foreign sessions: no unauthorized disclosure/mutation.
- Reject insecure session-file permissions, non-loopback HTTP and redirects before disclosing credentials.

## Verification

Through Nix, run core/server tests, all-target server Clippy, WASM compilation, formatting, actual CLI subprocess acceptance and full `nix flake check`. Regenerate changed query metadata with `cargo sqlx prepare --workspace -- --features server --all-targets` against disposable migrated PostgreSQL and validate a fresh offline build. Record actual results in `acceptance.md` before closing tasks.

To run the real-session acceptance matrix with a freshly built executable rather
than the in-process CLI adapter, set `HORAE_CLI_TEST_BINARY` to that executable's
absolute path and run:

```sh
cargo build -p horae --features server
HORAE_CLI_TEST_BINARY=/absolute/path/to/target/debug/horae cargo test -p horae --features server --bin horae job_endpoints_enforce_session_role_and_organization
```

This test-only switch uses disposable PostgreSQL sessions and loopback HTTP.
The production CLI has no test-login route or Harvest URL override.
