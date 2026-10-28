# Quickstart: verifying approval period navigation

Only run builds, databases and browsers in a slot root has granted. The test
IDs refer to the requirement → test map in [plan.md](plan.md). Expected
behaviour is in [contracts/approval-week.md](contracts/approval-week.md).

## No database

```sh
cargo test -p horae-core week::                               # week_bounds: Monday/Sunday alignment, MIN/MAX → None
SQLX_OFFLINE=true cargo test -p horae --features server --test approval_labels_ui   # T-U1..T-U9 and the three existing tests
```

## Disposable database (heavy slot)

```sh
DATABASE_URL=… cargo test -p horae --features server server_fns::approvals   # T-S1..T-S4 and the existing submission/isolation tests
DATABASE_URL=… cargo test -p horae --features server --test integration approval   # existing integration approval tests, unmodified
touch crates/horae/src/main.rs crates/horae/tests/*.rs
cargo sqlx prepare --workspace -- --features server --all-targets             # replaces the list_approvals descriptor only
SQLX_OFFLINE=true cargo clippy -p horae --features server --all-targets -- -D warnings
cargo check -p horae --features web --target wasm32-unknown-unknown
nix fmt
```

Expected results:

- `git diff --stat .sqlx` shows one descriptor removed and one added, both for
  `list_approvals`.
- The browser suite `responsive-layout` (which visits `/approvals`) passes
  unmodified.

## Native smoke (root's Chrome, disposable local Horae DB only)

Use a seeded manager and a submitted week W plus a submitted W−1:

1. Open bare `/approvals`. It shows the current week, labelled "This week",
   with the dates of the current week.
1. Click Prev. The address becomes `?week=<W−1 start>`, and only W−1's rows show,
   with their dates.
1. Refresh. The same week shows.
1. Click This week. The current week shows again.
1. On W, use "Approve visible timesheets". W−1 stays pending; check it via
   Prev.
1. Check that the header and pager fit at 320px with no horizontal page
   scroll.
