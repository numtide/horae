# Validation guide

Use the Nix shell. Never mutate the imported workspace on port 8080.

## Rust

```sh
SQLX_OFFLINE=true cargo test -p horae --features server --test trigger_utilities --test detail_navigation --locked
SQLX_OFFLINE=true cargo test -p horae --features server server_fns::projects::bulk_tests --locked
SQLX_OFFLINE=true cargo clippy -p horae --features server --all-targets --locked -- -D warnings
SQLX_OFFLINE=true cargo check -p horae --features web --target wasm32-unknown-unknown --locked
nix fmt
```

SQLx tests require DATABASE_URL on a dedicated cluster with CREATEDB; each test gets its own database. Changed macros require `cargo sqlx prepare --workspace -- --features server --all-targets` against migrated disposable data.

## Browser

Build from crates/horae with `dx build --platform web --fullstack true --force-sequential true --locked`. Set HORAE_TEST_SERVER to the bundle server and PLAYWRIGHT_MODULE to pinned playwright/test, then:

```sh
bash crates/horae/tests/browser/run-design-checks.sh
```

The runner creates/seeds a temporary PostgreSQL cluster, refuses occupied port 8093, and stops its own processes. Expected: selection/mixed/select-all, filters, cancel, actual archive/reactivate, server permissions, transport failure/retry, duplicate prevention, batch limit, themes and widths. Existing design/menu/responsive/mobile suites remain required.

Compare shared-style-audit.cjs snapshots from the exact base commit (`b389fb9`) and the new preview, with the same demo database, week and browser: eight other pages, three widths. Do not reuse an older running preview as the baseline; it may predate other merged changes. Open one PR; do not merge.

## Verification evidence — 2026-09-17

- All server unit and integration targets pass with `SQLX_OFFLINE=true cargo test -p horae --features server --all-targets --locked`; this includes 36 project tests and 12 controls/navigation tests.

- All 88 horae-core tests pass. Server/all-target Clippy with `-D warnings`, WASM compilation, fullstack build, `nix fmt` and `git diff --check` pass.

- SQLx preparation and `prepare --check` pass. Reused build artifacts initially omitted existing integration-test queries; a package-scoped `cargo clean -p horae` followed by preparation and a full offline test build recovered them. The final cache adds only ten new test queries and removes none.

- The isolated browser runner passes all five suites: Projects design, responsive layout, menu popovers, mobile navigation and bulk actions. Bulk checks cover keyboard selection/confirmation in both themes at 320/768/1440px; filter reset, mixed state, cancellation/focus return, network failure/retry, pending guards, empty/oversized selection and member visibility.

- Real-server checks verify anonymous/member/inactive denial, manager/admin access, malformed/empty/oversized and foreign-organization batches, duplicate IDs and safe retries. Archiving preserves full project details, assignments, tasks, entries and invoices. SQLx tests additionally prove rollback, overlapping-batch concurrency, 100-project acceptance, no-op row versions and invoiced spend preservation.

- Shared-style snapshots are identical on Clients, Invoices, Reports, Users, Settings, Importers, Components and Timesheet at 320/768/1440px, comparing a fresh build of base commit `b389fb9` against this implementation with the same demo data. The older running preview was not a valid Importers baseline; its different copy/layout predated already-merged changes.

All mutations used temporary PostgreSQL clusters. The imported workspace on port 8080 was not used for tests.
