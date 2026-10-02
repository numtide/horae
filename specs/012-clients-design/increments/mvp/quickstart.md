# Clients MVP validation guide

Run in this worktree's `nix develop`. Initialize a temporary PostgreSQL cluster
with a dedicated socket/port and CREATEDB test role. Set DATABASE_URL explicitly
for every DB command and apply migrations only to this new disposable database,
never the normal development database. Record exact invocations in progress.md.

```sh
cargo test -p horae-core
cargo test -p horae --features server clients
cargo test -p horae --features server project_creation
cargo test -p horae --features server privacy_tests
cargo clippy -p horae --features server --all-targets -- -D warnings
cargo check -p horae --features web --target wasm32-unknown-unknown
touch crates/horae/src/main.rs crates/horae/tests/integration.rs crates/horae/tests/cli_restart.rs
cargo sqlx prepare --workspace -- --features server --all-targets
nix fmt
nix flake check
```

On a reused build cache, force all three SQL-bearing target roots before each
SQLx prepare or prepare-check invocation. This changes only mtimes and prevents
an already-cached target from silently omitting its queries; inspect the cache
diff as well as the exit status. See progress.md for the observed reproduction.

Fixtures: two orgs; admin/manager/member/inactive users; active/inactive clients,
duplicate/Unicode/long names, unset/zero/positive rates; visible/hidden projects
with mixed currencies/states; draft/open/paid/void invoices. No real Harvest import.

1. Combine name/status/currency filters; reconcile counts, no-match/empty states,
   clear/retry and exact detail navigation.
1. Check every role against fixtures and inspect payloads for hidden projects,
   currencies, counts, rates and invoices. Probe direct/foreign IDs.
1. Create/reload/edit/cancel; invalid name/currency/rate; failed-input preservation,
   duplicate submits, concurrent changes, explicit currency/rate replacement.
   Assert historical project/invoice rows unchanged and unset differs from zero.
1. Preserve activation/deactivation and existing import navigation.
1. Project/invoice context without draft, unrelated draft, pending recovery,
   invalid context; zero business records on navigation; authority recheck on save.
1. Check direct routes, sidebar, back/forward and dirty/pending protection.
1. Capture list/detail/form light/dark at 320/390/768/1440px, short screens, 200%
   text and keyboard focus. Inspect images. Retest Projects/import/Invoices after
   shared component/style changes.

Completion requires adversarial review with no critical/high findings, published
unsigned scoped PR(s) and green CI. No merge is authorized.
