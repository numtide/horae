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

Delivered for review in [PR #205](https://github.com/numtide/horae/pull/205). CI remains a required pre-merge gate; this delivery does not merge the PR.

### Review follow-up

The initial alignment checks missed checkbox overflow inside the first subgrid track. A failing geometry test reproduced the reported overlap: 18px checkboxes occupied a zero-width cell and overlapped labels by 4px. Including the row's start padding in that track restores an 18px cell and 16px label gap, without changing the read-only grid.

Actions now uses a disabled native trigger with no selection or while submitting. Tests cover disabled clicks and arrow-key handling, enabling after selection, and disabling again after clearing it; other menus retain their defaults. The 13 controls/navigation tests, Clippy, WASM/fullstack build, all five browser suites and the eight-page style comparison pass after these changes. Geometry is checked in both themes at 320/768/1440px.

### Bulk recovery follow-up

Successful confirmation now falls back to the project-status filter when the disabled Actions trigger cannot receive focus. Cancellation retains native focus restoration. Pending project reads hide stale rows and disable bulk selection; failed reads offer Retry without resubmitting the mutation.

The new `project-bulk-recovery` browser suite first failed against the previous bundle on focus restoration, then passed for successful, delayed and failed refreshes against the updated bundle. All six isolated browser suites, 13 controls/navigation tests, server/all-target Clippy and WASM/fullstack compilation pass. No database queries, migrations or CSS rules changed.

### Merge-queue regression — 2026-09-18

The merge-queue browser run failed with `RuntimeError: unreachable`. Repeated release-client runs with the pinned Playwright 1.60.0 browser and 4× CPU throttling reproduced it when moving from an empty manager list to the member fixture. Temporary panic diagnostics identified `dioxus-fullstack` 0.7.9's response-body `unwrap()` in `magic.rs`: navigation cancelled an unfinished fetch. The diagnostics were removed; application code and dependencies are unchanged.

The empty fixture now waits for the rendered **No projects yet** heading. A zero-row assertion alone also passed while the resource was loading, allowing premature navigation. Browser errors still fail the suite, now with a stack trace; the selection-only path checks them too. No retries or fixed sleeps were added.

Validation: 100 oversized/empty/member cycles and ten complete fixture-suite repetitions pass under 4× CPU throttling with the diagnostic release client. All six isolated suites pass with the original release client and release server, including real archive/reactivate and authorization checks against disposable PostgreSQL.

To run all simulated cases against a non-production local preview without reaching the real-mutation section, set `HORAE_TEST_URL` and `PLAYWRIGHT_MODULE`, then run:

```sh
node crates/horae/tests/browser/project-bulk-actions.cjs --fixtures-only
```

The default full suite still requires the disposable runner on port 8093 and its Unix-socket database. Port 8080 remains forbidden in every mode.
