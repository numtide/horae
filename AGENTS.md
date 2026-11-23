# AGENTS.md

This file provides guidance to coding agents (Claude Code and others) when working with code in this repository. `CLAUDE.md` is a symlink to this file.

Horae is a self-hostable time tracker (a Harvest/Kimai alternative) built as a Rust + [Dioxus](https://dioxuslabs.com/) fullstack app (SSR + WASM) on PostgreSQL/Axum.

## Precedence

- Follow repository documentation and checked-in project conventions first.
- Use the guidance in this file when the repository does not specify an alternative.
- Treat `Prefer` as a default choice, not an absolute rule.
- Treat `Use` and `Do not` as stronger instructions.

## Commands

Work inside the Nix dev shell; a running PostgreSQL is required for anything touching the database.

```sh
nix develop            # dev shell: rust toolchain, dx (dioxus-cli), sqlx-cli, postgres, process-compose
process-compose up     # the dev stack: postgres + the app on :8080, migrated and seeded
nix run .#dev          # the same thing without entering the shell first
```

`process-compose up` is the normal way to run things locally. It starts PostgreSQL as a plain
process, waits for it to accept connections, then runs `dx serve`. Database state lives in
`.data/postgres` (gitignored) and persists across restarts — `rm -rf .data` is the reset button.
Migrations are applied before the app is built — sqlx checks its query macros against the
live database at compile time, so an unmigrated database fails to compile, not to run — and
the demo seed runs once on an empty database. The TUI shows per-process logs; `process-compose down` stops everything.

pgweb runs alongside it at http://localhost:8081 for browsing and querying the database.
To start only part of the stack, name the processes: `process-compose up postgres migrate`
gives you a database on its own, for `cargo test` or for running `dx serve` yourself.

Migrations are applied on every `process-compose up`, and `sqlx migrate run` only executes the
pending ones — so the first run and every later one are both covered. To apply one without
restarting the stack, run `horae-migrate`: it migrates and then rebuilds the app, which a new
migration requires because the query macros are checked at compile time.

The original 65 migrations were squashed into `0001_baseline.sql`, a `pg_dump` of their result.
A local database migrated before the squash fails to start (sqlx reports migration 2 as
applied but missing); `rm -rf .data` resets it. New schema changes go in `0002_*.sql` onwards; never edit
the baseline.

Connection details live in the `environment` block of `process-compose.yaml` and nowhere
else — change the port or user there and every process follows.

`nix run .#postgres` still boots a NixOS VM running PostgreSQL (forwards host :5432, :2222).
Use it to exercise the NixOS module — not for day-to-day work.

`DATABASE_URL` defaults to `postgres://localhost/horae`; the dev shell exports
`postgres://horae@127.0.0.1:5432/horae`, which both options above serve.

### Build & run

The app crate lives at `crates/horae/` (the repo root is a virtual workspace). It is **feature-gated** — `crates/horae/src/main.rs` has three `cfg`-selected `main()`s, and default features are empty. Always pick a feature or use `dx`, and select the crate with `-p horae`:

```sh
cargo build -p horae --features server          # server binary + CLI
cd crates/horae && DEV_LOGIN=1 DATABASE_URL=… dx serve   # dev server (dx runs where Dioxus.toml is), hot reload on :8080
cargo run -p horae --features server -- <subcommand>     # run the server binary directly
```

CLI subcommands: `serve`, `init --org-name … --admin-email … --admin-name …` (bootstrap: one org + one admin, refuses if an org exists), `migrate run`, `migrate reset --confirm`, `seed` (demo data; `--api-token-file FILE` also writes an API token of the demo administrator, for test harnesses), `user list`, `user create --email … --name … --profile …`, `user link-oidc --user-id … --issuer … --subject …` (binds an OIDC identity to an existing active account; never replaces a binding).

Remote administrator commands: `import harvest-api`, `import harvest-csv FILE`,
and `jobs status|list|report|errors|cancel|retry|wait`. They require a running
server and a private session file selected by `--session-file` or
`HORAE_SESSION_FILE`; they do not load deployment configuration, connect to
PostgreSQL or start workers. See `specs/006-harvest-jobs-cli/quickstart.md` for
authentication, preview/commit, waiting, JSON output and exit codes.

These are for one-off tasks; `process-compose up` covers the normal run loop. Open
http://localhost:8080/auth/login and "Sign in as Admin" (needs `DEV_LOGIN=1`, which the stack sets).

### Test & lint

```sh
cargo test -p horae-core                        # pure domain unit tests (no DB, no features)
cargo test -p horae-types                       # serde contracts of the UI↔server types (no DB)
cargo test -p horae-i18n                        # the catalog's build checks and the unused-message test (no DB)
nix develop .#kani -c cargo kani -p horae-core  # bounded proofs behind cfg(kani); Linux only, see below
DATABASE_URL=… cargo test -p horae-server               # backend tests: ops, jobs, importers, … (need Postgres with CREATEDB)
DATABASE_URL=… cargo test -p horae-api                  # HTTP routes: /api/v1, exports, import error downloads, /auth/*
DATABASE_URL=… cargo test -p horae-harvest-api          # the Harvest-compatible API and its switch
DATABASE_URL=… cargo test -p horae --features server     # server functions, CLI, UI and integration tests
DATABASE_URL=… cargo test -p horae --features server <name>   # a single test
UPDATE_OPENAPI=1 cargo test -p horae-api openapi        # rewrite crates/api/openapi/v1.json after changing /api/v1
cargo clippy -p horae-server --all-targets
cargo clippy -p horae-api --all-targets
cargo clippy -p horae-harvest-api --all-targets
cargo clippy -p horae --features server
nix fmt                                         # treefmt: rustfmt, taplo, nixpkgs-fmt, mdformat
cargo run -p horae-dioxus-lint                  # Dioxus rules ratchet (or: nix build .#checks.x86_64-linux.dioxus-lint)
```

`cargo test -p horae-core` also runs the proptest properties in `crates/core/tests/properties.rs`, `crates/core/src/permissions/properties.rs` and `crates/core/src/permissions/oauth/properties.rs`. The Kani harnesses live in `crates/core/src/proofs.rs`, `crates/core/src/permissions/proofs.rs` and `crates/core/src/permissions/oauth/proofs.rs`. Kani (`nix/packages/kani.nix`, with the nightly toolchain it was built against) lives in its own Linux-only dev shell, `nix develop .#kani`, so the default shell doesn't carry its 1.4 GiB. No `cargo kani setup` is needed. All fifteen harnesses prove: a bare `nix develop .#kani -c cargo kani -p horae-core` takes about two and a half minutes. Two take most of that: `proofs::invoice_discount_stays_within_the_subtotal` (about 60 s, bounded to subtotals up to 2^28 cents) and `proofs::line_amount_is_half_up_for_every_input` (about 45 s). The rest of `proofs` (rounding, invoice amounts and taxes, budget percentage, snap) and all of `permissions::proofs` (coverage, rate edits, catalog versions) and `permissions::oauth::proofs` (the scope ceiling: normalizing, the scope lists and their prerequisites) take under 20 s each. Kani is weak at strings, `BTreeSet` and bounding 128-bit quotients, so some properties have no proof. The parsers (`duration::parse`, `time_of_day::parse`), the half-up rounding of each invoice component, `discount ≤ subtotal` above 2^28 cents and the discount allocation over lines are sampled by proptest. The narrowing of an OAuth request's grants is a std set intersection that the `ops::actor` tests in `horae-server` check. No proof covers FR-023's line sum, which the server computes, not core. The module docs say why. Cap memory when running Kani, for example `systemd-run --user --scope -p MemoryMax=8G`, and name harnesses with `--exact --harness <path>` (repeatable) to run a subset. Follow-up: no CI job runs them yet.

`crates/dioxus-lint` enforces the `dioxus-best-practices` skill: it parses every `rsx!` with `dioxus-rsx` and fails when a file's count for a rule differs from `crates/dioxus-lint/baseline.toml`, either up (a new violation: fix it) or down (lower the count). `clippy.toml` bans `Callback::new`/`EventHandler::new` in favour of `use_callback`. `cargo test -p horae-dioxus-lint` runs its fixtures and checks every rule against a SKILL.md heading.

Integration tests (`crates/horae/tests/integration.rs`) use `#[sqlx::test]` — each spins up a throwaway database, so the DB role needs `CREATEDB` — and are marked `#[serial]`. `nix build` builds the package; `nix flake check` runs the whole suite against a PostgreSQL started inside the build sandbox (`nix/checks/tests.nix`), plus clippy, formatting, the `.sqlx` cache check, the Dioxus rules ratchet, the NixOS e2e tests, `api-compat` (oasdiff: the `/api/v1` snapshot against the published `v1.baseline.json`; within v1 only additions pass), `api-v1` (a Python client generated from the served OpenAPI document runs the quickstart scenarios) and `harvest-compat` (real Harvest clients against `/v2`). Both API checks take their token from `horae seed --api-token-file`.

### Build speed

- Keep Cargo's incremental compilation on (the default). With `CARGO_INCREMENTAL=0`, rebuilding the server after a one-file change takes about 60 s instead of 18 s, and `cargo test --no-run` about 210 s instead of 60 s. When disk runs low, delete the worktree's `target/` instead.
- `[profile.dev]` in the root `Cargo.toml` builds proc macros optimised (most of a rebuild of the `horae` crate is macro expansion: sqlx, `rsx!`, serde) and keeps line tables only, so backtraces still show file:line. The `test` profile inherits it.
- In-crate tests take `#[sqlx::test(migrator = "crate::db::MIGRATOR")]` (in `horae` that path is the re-exported `horae_server::db`). A bare `#[sqlx::test]` or `migrations = "../server/migrations"` pastes every migration's SQL into the test, which across the suite doubled macro expansion of the unit-test binary.
- The bottleneck is rustc's per-crate frontend, not linking: rustc already links with its bundled `rust-lld`, and mold saves about 1 s. The backend is its own crate (`horae-server`) so a UI change does not recompile it and its tests build in their own binary; run `cargo test -p horae-server` for backend work. In `horae`, filter tests with `--bin horae` or `--test <file>` to build only what you run.
- `cargo build` and `cargo test` resolve different features (the dev-dependencies enable some), so running both compiles dioxus, sqlx and typst twice in one `target/`.
- Don't use `dx serve --hot-patch`. The server runs its own `axum::serve` rather than `dioxus::serve`, so dx answers server changes with `Missing server jump table` and never rebuilds; after a failed first build it ignores every later change. Wired through `dioxus::serve`, a warm patch took as long as the normal rebuild and relaunch (about 25 s).
- No shared compiler cache is configured: sccache can't cache the incremental `horae` crate, which dominates a cold build, so a fresh worktree saved nothing measurable.

### sqlx query cache

All SQL queries use compile-time checked macros (`sqlx::query!`, `sqlx::query_as!`, `sqlx::query_scalar!`). These validate SQL against the database schema at compile time, catching typos, type mismatches, and schema drift before the code runs.

At compile time, macros need either `DATABASE_URL` pointing to a live DB with migrations applied, or `SQLX_OFFLINE=true` with the `.sqlx/` cache committed to the repo. Nix builds use `SQLX_OFFLINE=true`.

After changing any `query!`/`query_as!`/`query_scalar!` macro or migration, regenerate the cache:

```sh
touch crates/server/src/lib.rs crates/api/src/lib.rs crates/harvest-api/src/lib.rs crates/horae/src/main.rs crates/horae/tests/*.rs
cargo sqlx prepare --workspace -- --features server --all-targets   # requires live DB with migrations applied
git add .sqlx/                                        # commit the updated cache
```

The timestamp refresh is necessary for repeated preparations: SQLx's selective
recompile discovery uses default-feature metadata, which omits this app's optional
SQLx dependency. Otherwise cached test targets can be skipped while their query
descriptors are removed. `touch` changes no source contents. Verify the resulting
cache with the offline full test build, not the prepare exit code alone.

**Important:** the `--features server` flag is required because the `horae` crate's sqlx query macros live behind `#[cfg(feature = "server")]`. Without it, `cargo sqlx prepare` finds only `horae-server`'s queries and **deletes** the rest of the cache. `horae-server` has no `server` feature (all of it is server code); `--workspace --features server` accepts that. Both crates share the one `.sqlx/` at the workspace root.

For custom PostgreSQL enum types, use type overrides in the SQL:

- **Columns**: `state as "state: EntryState"` in SELECT
- **Parameters**: `EntryState::Open as EntryState` in macro arguments
- **Optional filters**: `($N::type IS NULL OR column = $N)` pattern with `Option<T>` parameters

## Architecture

**One app crate (`crates/horae/`), two build targets, feature-gated.** `crates/horae/src/main.rs` defines three `main()`s behind `cfg`: `server` (Axum + Tokio + the CLI), `web` (`dioxus::launch`, compiled to WASM), and a stub that errors if neither feature is set. The server-only module `cli` is `#[cfg(feature = "server")]`; the shared UI modules (`app`, `route`, `pages`, `components`, `server_fns`) compile for both targets. This is why a bare `cargo build`/`test` (empty default features) won't do what you expect.

**The backend is the `server` crate (`horae-server`, `crates/server/`)**: `ops`, `db`, `state`, `config`, `plugin`, `jobs`, `importers`, `notifications`, `timezone`, `render`, `init`, `seed`, `scheduler`, the OIDC backend (`auth::oidc`) and the migrations (`crates/server/migrations/`). It is a lib with no Dioxus or Axum dependency. `crates/horae/src/main.rs` re-exports its modules (`pub(crate) use horae_server::{ops, db, …}`), so the app keeps naming them `crate::ops::…`; what the app uses is `pub` there. Its test fixtures (`testing`) are behind `cfg(any(test, feature = "test-support"))`, and `horae` enables `test-support` as a dev-dependency.

**The HTTP surfaces are the `api` crate (`horae-api`, `crates/api/`)**: the public API `/api/v1` (`v1`: thin utoipa handlers over the shared operations, authenticated by personal access tokens, errors as `application/problem+json`, signed list cursors; its OpenAPI document is generated from the handlers and snapshotted in `crates/api/openapi/v1.json`, and `v1.baseline.json` is the published contract), `/health`, the CSV/XLSX/PDF exports (`exports`), the import error download (`import_errors`), the Harvest OAuth callback (`harvest_oauth`), the session and the `/auth/*` routes (`auth`). It is a lib with Axum and no Dioxus; it sits between `horae-server` and `horae`. `horae_api::router(RouterOptions)` returns all its routes, and it exports `make_session_layer`, `login_redirect_guard` and `session` for the binary and the server functions. `LoginAssets` is how `horae` passes the icon URLs its `asset!` macros resolve. Like `horae`, it re-exports the `horae-server` modules it uses, so its code names them `crate::ops::…`. Its `test-support` feature exposes the workbook readers and the CSV helper that `horae`'s tests use.

**The `core` crate (`horae-core`) is pure domain logic** — duration parsing, rounding, money, totals, the entry state machine — with no I/O dependencies (only serde/uuid/chrono/thiserror). Correctness-critical code belongs here and is unit-tested in isolation; the Horae Constitution forbids sqlx/axum/dioxus deps in `core`.

**The `types` crate (`horae-types`) holds the types that travel between the UI and the server**: server-function arguments and results, and the DTOs the pages render. It is a lib that builds for the server and for WASM, depends only on serde, serde_json, uuid, chrono and `horae-core`, and has no `server` feature or `cfg`. Rules belong in `core`; the wire shape of what they produce belongs in `types`. `crates/horae/src/main.rs` imports it as `models` (`use horae_types as models;`), so the app names these types `crate::models::…`. The UI tests in `crates/horae/tests/` reach them through `horae_types` instead of including the source with `#[path]`. Server-only types stay in `ops/`, and the server's logic over shared types goes in free functions there, because Rust forbids inherent `impl` blocks on another crate's types.

**The `i18n` crate (`horae-i18n`, `crates/i18n/`) is the interface's text catalog**: Fluent files under `locales/<lang>/horae.ftl`, English the source. Its `build.rs` checks the catalogs (`build/checks.rs`, shared with `tests/build_checks.rs`) and generates `Lang` and one method per English message on `Tr`, with the message's variables as parameters and its English text as the doc comment, so a missing message or argument does not compile. It builds for the server and for WASM, depends on `fluent-bundle` and `unic-langid` only, and `horae-core` and `horae-types` must not depend on it. The app reaches it through `crates/horae/src/i18n.rs`: `App` calls `use_tr_provider()` and components call `use_tr()`. The UI tests that include sources with `#[path]` include that module too. `tests/unused_messages.rs` fails on a message no code calls.

**The server layers custom Axum routes on top of the Dioxus fullstack router** (`Commands::Serve` in `main.rs`): it calls `.serve_dioxus_application()`, then `.merge`s `horae_api::router()` (`/api/v1`, `/health`, the exports, `/auth/*`) and the Harvest-compatible API (`horae_harvest_api::mount(cfg.harvest_api, …)`, `/harvest/v2/*` and `/v2/*`; the `horae-harvest-api` crate, `crates/harvest-api/`, mounted only with `HORAE_HARVEST_API=1` and an empty 404 otherwise), all under a Postgres-backed session layer. Both APIs share one per-token rate limiter. The surfaces:

- Dioxus `#[server]` functions in `server_fns.rs` — session-authenticated; the SPA uses these for all mutations.
- `/api/v1` — personal access tokens; reads and writes through the same operations as the server functions.
- `/harvest/v2` (optional) — reads with tokens or the session; time-entry writes through the same operations, with tokens only.
- Plain Axum routes — health, exports and auth, session-authenticated.

**Server operations (`ops/`)**: new or changed server behaviour goes in `crates/server/src/ops/`, one function per action with the shape `fn(tx: &mut Tx, actor: &Actor, input) -> Result<Output, OpError>`. The operation authorizes, validates, runs its SQL and returns its plugin events in `Done<T>`; it never commits and never builds a `ServerFnError`. This is the constitution's Principle IV, *Mutations Through Shared Operations*: the server functions and the token-authenticated APIs mutate only through these operations. Adapters (server functions, the `/api/v1` and Harvest handlers) only translate: they open `ops::begin` (or `ops::begin_read`), which loads the `Actor` inside the transaction, call the operation, and finish with `WriteTx::commit` (events are dispatched after the commit) or `ReadTx::finish`; `OpError` maps to named status codes. Authorize only through `Actor` (`has(permission)`, `administrator()`), which answers from the person's stored permission state. `clients` is the reference domain.

**Shared state**: `state.rs` holds a global `AppState` in a `OnceCell`, initialized once at startup with the `PgPool`; server fns and auth read from it. The pool is created and migrations applied eagerly on `serve`.

**Auth**: production uses OIDC (`openidconnect`); `DEV_LOGIN=1` enables a one-click admin login that bypasses OIDC (see `auth/`). Sessions are cookie-based, persisted in Postgres. A refused sign-in leaves a one-time notice in the session for `/auth/login`. `HORAE_OIDC_AUTO_PROVISION_DOMAINS` lets a first sign-in create an active member account (the Principle IV sign-in exception); `HORAE_OIDC_TRUST_UNVERIFIED_EMAIL` accepts a missing `email_verified` claim for that only.

## Domain invariants (from the Horae Constitution — do not violate)

- Durations are stored as **integer minutes**; money as **integer minor units (cents) + ISO currency code** — never floats. The one exception: an XLSX (Excel) export may convert final integer minutes to a numeric hours cell at write time (Constitution I).
- Primary keys are **UUID v7** (time-ordered).
- **PostgreSQL only** (no SQLite). Migrations live in `crates/server/migrations/` and apply via `sqlx` / `migrate run`.
- Single organization for now, but every table keeps an `org_id` FK so multi-org is a later flip.

Feature requirements and implementation plans live under `specs/<NNN-feature>/`; `.specify/memory/constitution.md` is authoritative for cross-cutting invariants. `DESIGN.md` is the design system (Invoicer aesthetic; tokens in `crates/horae/assets/css/horae.css`; components are `#[component]` functions, one family per file, using `use_signal`/`use_resource`, with no global mutable UI state).

## Skills

The repo ships agent skills in `.agents/skills/` (surfaced to Claude Code through `.claude/skills/` symlinks). Invoke the relevant one before the matching task — they carry conventions this project relies on:

- **`rust-best-practices`** — idiomatic Rust from Apollo's handbook: borrowing vs cloning, `Result`/error handling, performance, and the sqlx compile-time-macro and module-layout rules this repo follows. Use when writing, reviewing, or refactoring Rust.
- **`rust-testing`** — unit, integration, async, and property-based testing patterns (TDD). Use when adding or changing tests; pairs with the `#[sqlx::test]` / `#[serial]` setup under *Commands → Test & lint*.
- **`rust-async-patterns`** — Tokio, async traits, and concurrency patterns. Use for async server code or when debugging async behaviour.
- **`dioxus-best-practices`** — Dioxus 0.7.9 rules read from the locked source: prop memoization (inline closures and `children` re-render whenever the parent does), subscriptions, `use_resource` restarts and SSR, and where Horae breaks them. Use when writing, reviewing or optimising components, pages or hooks.
- **`ponytail`** — enforces the smallest solution that works (YAGNI, stdlib before deps, one line before fifty). Use on any coding task, especially before adding a dependency or abstraction.
- **`humanizer`** — rewrites prose that reads as machine-written (staged contrasts, one-line closers, forced triads, inflated claims, stock words) without changing what it says. Use when writing or reviewing commit messages, PR bodies, docs, specs, or UI copy.

A `speckit-*` suite (`specify`, `plan`, `tasks`, `implement`, `analyze`, `checklist`, `clarify`, `constitution`, `converge`, `taskstoissues`) supports spec-driven development against the feature artifacts under `specs/` and the constitution.

## Conventions

### General

- Write for readers of the final code, not for the diff.
- Prefer simple, direct code over clever or overly compact code.
- Keep changes scoped to the task. Avoid incidental refactors unless they are necessary to make the change safe or understandable.
- Prefer minimal diffs that solve the problem clearly.
- Preserve existing naming, formatting, and architectural patterns unless there is a clear reason to change them.
- Comments should explain why, constraints, or non-obvious tradeoffs, not restate what the code already says.
- Do not leave comments that describe the editing process or previous instructions from the conversation.
- When referring to code, reference stable symbols or files rather than line numbers. Line-number references go stale quickly.

### Project-specific (from code review)

Rules on top of idiomatic Rust (see the `rust-best-practices` skill). These come from code review — follow them so the same notes don't recur:

- **Interface text goes in the catalog.** New or moved text of the web app goes in `crates/i18n/locales/en/horae.ftl`, under its area's section and following the conventions in the file's header, and the screen calls `tr.<id>()`. `dioxus/literal-text` counts loose text in `rsx!` per file and fails when a count rises; the `dioxus-best-practices` skill's "Interface text" section has the rules.
- **Named status codes, not integer literals.** When building `ServerFnError::ServerError { code, .. }` in `server_fns.rs`, use named constants (e.g. `NOT_FOUND`, `FORBIDDEN`) rather than bare `404`/`403`, so error paths read at a glance.
- **Avoid `Option<bool>` parameters.** `Some(false)` is ambiguous at the call site. For a two-state flag on a server function, prefer a plainly named `bool` with an obvious default, or a small purpose-named enum.
- **New modules use the `foo.rs` + `foo/` layout, not `foo/mod.rs`.** A multi-file module lives in `foo.rs` (the module root) beside a `foo/` directory of submodules — as `pages`, `components`, and `server_fns` already do. `mod.rs` is *not* deprecated (the Rust Reference keeps both forms working), but the Reference encourages the sibling-file form and it keeps the tree free of many identically-named `mod.rs` files. The remaining `plugin/mod.rs` predates this rule; convert it opportunistically, not in an unrelated PR.

### Harvest parity

Horae reproduces Harvest's behaviour and workflows, not Harvest itself. [`specs/harvest-parity.md`](specs/harvest-parity.md) tracks coverage and evidence.

- Rebuild what a screen does from its observable behaviour and the public docs. Never copy Harvest's source, JS bundles, private endpoints, assets, logos, copy text or branding (the one exception, a logo that identifies an integration, is below). Matching what a button does is parity; matching its sentence is copying. Short standard labels (Archive, Duplicate, Timesheet, Approvals) are generic, so reusing them is not copying.
- Work from the public help center and the public API docs, plus read-only browsing in a session the user has already signed in to (open a new tab; never reuse or alter the user's tabs). Never sign in on the user's behalf.
- Never create, edit or delete data in the production Harvest account. Disposable fixtures are allowed only in the test (evidence) account the user designates, with the user's authorization for the task recorded in `specs/harvest-parity.md`. Never touch pre-existing business records, delivery, payments or account settings. Track fixtures by exact ID and clean them up, or record that they are deliberately retained.
- Naming Harvest where Horae interoperates with it (the importer, the `/harvest/v2` compatibility API) is fine. Outside those places, the product's look, name and wording stay Horae's (`DESIGN.md`).
- An external service's own logo may be shown only to identify Horae's integration with that service (for example, the Harvest importer). Serve it as a local asset, never alter it, and never use it as Horae's branding. Copying anything else stays forbidden.

## Repository Hygiene

- Write commit messages, branch names, PR titles, PR bodies, and issue comments as a human developer would in any repository.
- Do not mention AI assistance, model names, model versions, or references that are not appropriate for the repository.
- Do not use `Co-Authored-By` lines or other attribution that reveals AI involvement.
- Describe only the user-visible change, bug fix, refactor, or implementation detail relevant to the repository.
- Good examples:
  - `Handle missing config file during startup`
  - `Add validation for empty API responses`
  - `Simplify cache invalidation logic`
- Bad examples:
  - `Generated with an AI assistant`
  - `Tested with model-x experimental build`
  - `Sync changes from internal-tooling branch`
  - `Co-Authored-By: AI Assistant <assistant@example.com>`
