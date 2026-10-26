# Implementation plan: remove the Project Detail fee balances panel

Branch `fix/project-detail-fee-panel`, based on `8975b881` (#305). Scope:
[spec.md](spec.md) only. #208 (`48a41560`) is a read-only reference; its
`projects.rs` is not copied because master's detail page has diverged.

## Technical context and approach

Rust/Dioxus SSR+WASM; `ProjectDetailContent` in `crates/horae/src/pages/projects.rs`.
No PostgreSQL, schema, query, dependency, CSS or shared component change.

1. Delete `crates/horae/src/pages/projects/fee_balances.rs` and its `#[path]`
   module declaration.
1. In `ProjectDetailContent`, remove the panel mount and the state used only for
   it: the `generation` signal, which only keyed the panel so a rebinding
   reload remounted it. `binding`, `access_lost`, `details`, the status write
   (409 refusal, Undo, focus recovery) and the tasks/team sections stay. Reword
   the status-denial comment that pointed to the fee panel.
1. Keep `server_fns::get_project_fee_balances`, `fetch_project_fee_balances` and
   `ProjectFeeBalance`: they are authorized API with server tests in
   `server_fns/invoices/fee_tests.rs`, `snapshot_tests.rs` and the importer
   authorization tests. After the removal, no page constructs
   `ProjectFeeBalance` on web, and strict WASM Clippy fails on `dead_code`.
   The coordinator extended scope to this one model treatment. The web client
   stub of the preserved server function still returns the DTO, so feature-gating
   it would change the API. `crates/horae/src/models/project.rs` gets a web-only
   `cfg_attr(not(feature = "server"), expect(dead_code, reason = ...))`. It follows
   the `allow(dead_code)` convention in `models/organization.rs`, but uses
   `expect` so it fails once a page uses the type again. Global lints and server
   APIs are unchanged.

Constitution check: I–V unaffected. Exact amounts stay integer cents in the
untouched preparation flow, and mutations still go through server functions.
No deviation to track.

## Callers and test relocation

| Removed coverage | Where | Replacement |
|---|---|---|
| SSR `fee_response` probe and `get_project_fee_balances` mock | `tests/detail_navigation.rs` | Deleted with the caller. Absence assertions for "Fee balances" stay in `detail.rs`. |
| SSR `late_fee_access_denial_hides_loaded_detail_and_retry_keeps_original_identity` (401/403 from a late fee read hides the project and its diagnostic; Retry keeps the bound requester) | `tests/detail_navigation/detail.rs` | The fee read no longer exists. The remaining late lost-access path is the status write. The SSR harness can't open the Actions menu because `MenuItem` has no id, and changing it is out of scope. `project-detail-header.cjs` already covers 403 from account, policy and role changes with the bound Retry. Add a real-server 401 (expired session) case that also checks the response text isn't shown. Detail-read rebinding and denial stay in `detail_refresh_keeps_initial_identity_and_rejects_wrong_project_or_policy` and `pending_and_denied_detail_never_mount_sensitive_children_or_render_raw_errors`. |
| Browser: panel shows Invoiced 127.00 and Over-invoiced −2.00 for September | `tests/browser/invoice-preparation.cjs` | Same figures in "Fees to invoice" for the same project and period, including the Over-invoiced marker. |
| Browser: no horizontal scroll at 390/768/1440 | same | Same widths with the preparation balances visible, plus a Project Detail check that the region is absent. |
| Browser: aborted read shows an error and hides stale amounts; retry recovers | same | Not identical. Hiding stale amounts was specific to the removed panel and goes with it. Preparation is unchanged: after an aborted `prepare_invoice` review it keeps the "Last review" values and local edits, shows an error and disables Generate. A retried review clears the error and shows the expected balance. The test does not claim stale values are hidden or restored. |
| Browser: demoted manager's replayed request is 403 with no amount | same | Replay of the captured `prepare_invoice` request; Project Detail still has no panel. |
| Browser: monthly fee has two occurrences, Remaining 900.00, nothing materialized | same | Preparation for Sept–Oct lists two fee groups, each Remaining 900.00, with distinct occurrence labels; fee-occurrence count stays 0. |
| Browser: panel absent for a reader | `tests/browser/project-read-permissions.cjs` | Unchanged; the absence still holds. |

Server-side fee and authorization tests (`fee_tests.rs`, `snapshot_tests.rs`,
`authorization_tests/{project_reads,financial_snapshots}.rs`) are unchanged.

## Validation

Disposable PostgreSQL and target directories only (under `/tmp` or this
worktree). Confirm that no other Horae build is running before heavy builds.
Run focused tests, then `cargo test -p horae --features server --test detail_navigation`,
strict server Clippy (all targets), WASM check/Clippy, `nix fmt`, and the browser
suites `invoice-preparation`, `project-detail-header` and `project-read-permissions`.
The full flake runs only when the coordinator asks.

Progress and the next action are recorded in [progress.md](progress.md).
