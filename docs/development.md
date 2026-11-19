# Development

Everything runs inside the Nix dev shell, which provides the Rust toolchain, `dx` (the Dioxus
CLI), `sqlx-cli`, PostgreSQL and process-compose. A running PostgreSQL is required for anything
that touches the database. [CONTRIBUTING.md](../CONTRIBUTING.md) covers system setup and the
project's conventions; [AGENTS.md](../AGENTS.md) has the full command reference.

## The dev stack

```sh
nix develop            # enter the dev shell
process-compose up     # postgres + the app, on http://localhost:8080
```

Or `nix run .#dev` to do both at once.

That is the whole loop. `process-compose up` starts PostgreSQL as a plain process, waits for it,
applies migrations, then runs the hot-reloading dev server. Demo data is seeded the first time, so
there is nothing to run by hand.

Open http://localhost:8080/auth/login and choose **Sign in as Admin**. The admin bypass is only
available when `DEV_LOGIN=1` is set, which the dev stack does for you.

Database state lives in `.data/postgres` and survives restarts; `rm -rf .data` starts over.
`process-compose down` stops everything.

[pgweb](https://github.com/sosedoff/pgweb) runs alongside the app at http://localhost:8081 for
browsing and querying the database. To start only part of the stack, name the processes you want:
`process-compose up postgres migrate` gives you a database on its own, which is what the
integration tests need.

## Migrations

Migrations live in `crates/server/migrations/`. They run on every `process-compose up`, and only
the pending ones are executed. To apply a new one without restarting the stack, run
`horae-migrate`: it migrates and rebuilds the app, which is needed because sqlx checks its queries
at compile time.

The 65 original migrations were squashed into one baseline, `0001_baseline.sql`. A database
migrated before that refuses to start (sqlx reports migration 2 as applied but missing): locally,
run `rm -rf .data`. Deployments have their own steps in
[Deployment](deployment.md#the-migration-baseline).

## PostgreSQL in a VM

`nix run .#postgres` boots a NixOS VM running PostgreSQL (forwarding host `:5432` and `:2222`). It
exercises the same NixOS module used for self-hosting, which makes it useful for checking packaging
changes, but it is slower to start and needs KVM. With it running, drive the app by hand:

```sh
cargo run -p horae --features server -- migrate run   # apply pending migrations
cargo run -p horae --features server -- seed          # insert demo data (idempotent)

cd crates/horae && DEV_LOGIN=1 dx serve               # dev server on :8080, hot reload
```

## Tests and lints

```sh
cargo test -p horae-core                              # pure domain unit tests (no DB, no features)
DATABASE_URL=… cargo test -p horae --features server  # integration tests (need Postgres w/ CREATEDB)
cargo clippy -p horae --features server               # lint
nix fmt                                               # treefmt: rustfmt, taplo, nixpkgs-fmt, mdformat
```

The other crates have their own suites (`horae-types`, `horae-server`, `horae-api`,
`horae-harvest-api`); [AGENTS.md](../AGENTS.md#test--lint) lists them all.

Integration tests use `#[sqlx::test]` (each spins up a throwaway database, so the DB role needs
`CREATEDB`). `nix build` builds the package and `nix flake check` runs the whole suite against a
PostgreSQL started inside the build sandbox, plus clippy, formatting, and the NixOS end-to-end
tests.

## The sqlx query cache

All SQL uses sqlx's compile-time-checked macros. After changing a query or migration, regenerate
the offline cache with `cargo sqlx prepare --workspace -- --features server --all-targets` and
commit `.sqlx/`. [AGENTS.md](../AGENTS.md#sqlx-query-cache) has the full procedure.

## Agent skills

The repository ships agent skills under `.agents/skills/` (Rust best practices, testing, async
patterns, and more) that capture the conventions this project follows.
