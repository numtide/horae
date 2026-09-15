# Validation Guide: Harvest Import UX

These are future acceptance instructions, not claims of implemented or tested behavior.

## Isolated prerequisites

Use the feature worktree and `nix develop`. Point `DATABASE_URL` at an isolated, migrated PostgreSQL database owned by a role with CREATEDB. Follow the existing process-compose/dev-shell setup with a separate app port and matching server/web bundle. Do not migrate/reset the operator's live database or restart its app to run fixtures.

The new browser harness follows `tests/browser/modals.cjs`, using an explicit isolated URL and existing Playwright/Chromium tooling. Use `PLAYWRIGHT_MODULE` / `CHROMIUM_PATH` overrides if necessary; missing tooling is a reported prerequisite, not a new application dependency.

## Automated gates

Inside the Nix shell with the isolated database:

```sh
cargo test -p horae-core
cargo test -p horae --features server
cargo clippy -p horae --features server --all-targets -- -D warnings
cargo build -p horae --features web --target wasm32-unknown-unknown
cargo sqlx prepare --workspace -- --features server --all-targets
nix fmt
nix flake check
```

Iterate with `cargo test -p horae --features server --test import_jobs_ui` and focused projection/CLI tests. Never omit `--features server` from SQLx preparation; review cache changes.

After implementing the browser harness and starting its isolated fixture app:

```sh
HORAE_TEST_URL=http://127.0.0.1:8092 node crates/horae/tests/browser/importers.cjs
```

The harness must verify fixture prerequisites and intercept provider/destructive account operations where appropriate; never use real credentials. A script that was not run is not acceptance evidence.

## Fixture matrix

1. All seven connection categories; imported-data/active-job blockers; cancel/confirm; stale account change; failed authorization after release. Compare business rows and retained reports for preservation.
1. API and CSV preview/results: current/historical, source replacement, mixed outcomes, all skipped, zero records, partial and no-report failure. Assert labels and primary-action count.
1. Queued/running with known/unknown counts, cancellation requested/completed, completion racing cancellation, polling failure/resume and delayed old-selection responses. Resume uses the same ID without start/retry calls.
1. History selection, source/mode/date labels, pagination, loading/unavailable/empty/expired history; old API generation, missing CSV upload, unknown metadata and stale retry availability. Old-account partial reports stay partial.
1. Compatibility: old/future availability JSON, unknown kind/phase, unchanged CLI behavior, foreign/inactive/non-admin sessions and consistent status/list projections.
1. Browser: keyboard/disclosure/modal focus, pending actions, announcements, long identifiers and local table scrolling at 360/768/1440 widths and 640 height. Test actual 200% browser zoom; pixel density is not equivalent. Record manual zoom evidence if not reliably automated.
1. Read the complete handoff and required support resources using the design skill before implementation. Compare actual main-panel states; document unsupported prototype behaviors and justified deviations.

## Real Harvest dry-run: separate operator gate

Only after explicit go-ahead, confirm the local URL, matching server/web version and currently authorized account. Any deployment required is a separate coordinated operation, not implicit permission to upgrade a running instance.

Capture business-table counts/digests privately, submit Preview only and follow its job ID. Verify business rows are unchanged; operational job/report rows are expected. Never confirm an import, disconnect/change the account or broaden provider permissions automatically.

Record either a readable preview or the precise actionable provider limitation. A permission error validates error UX, not a successful import. Keep secrets and real data out of committed evidence.

## Completion evidence

Update `acceptance.md` with tested commit, environment, actual commands/results, fixture/browser outcomes, design deviations and the separately authorized dry-run. Mark unavailable gates blocked with their prerequisite. Documentation delivery is not implementation acceptance.
