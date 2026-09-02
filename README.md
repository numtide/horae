<p align="center">
  <img src="assets/banner.png" alt="Horae — self-hostable time tracking" width="820">
</p>

Self-hostable time tracking — a Harvest / Kimai alternative that stays fully yours.

## About

Horae is a time tracker you run on your own infrastructure. Log hours against clients and
projects, submit and approve timesheets, and export billable reports — without handing your
data to a SaaS vendor. It speaks a read-only, Harvest-compatible API so existing Harvest
integrations and tooling keep working.

It is built as a single Rust + [Dioxus](https://dioxuslabs.com/) fullstack application
(server-rendered plus a WebAssembly SPA) backed by PostgreSQL and Axum. Correctness-critical
domain logic — duration parsing, rounding, money, timesheet totals — lives in a dependency-free
core crate and is unit-tested in isolation.

> Horae is in active Phase-1 development. Expect the schema and API to keep moving.

## Features

- **Timesheets** — Day, Week, and Calendar views with weekly totals and a running timer.
- **Clients & projects** — manage clients, projects, per-project assignments, tasks, and budgets.
- **Approvals** — submit, approve, and reject time entries (manager and admin roles).
- **Reports** — grouped time reports with CSV and XLSX export.
- **Invoices** — draft invoices with CSV, XLSX, and PDF export.
- **Harvest-compatible API** — a read-only `/harvest/v2/*` surface matching the Harvest v2 shape.
- **Auth** — OIDC single sign-on in production; a one-click dev login for local work.

The authenticated SPA is organized by route:

| Route | Description |
|---|---|
| `/timesheet/:view` | Day / Week / Calendar timesheet with weekly totals (`/` redirects here) |
| `/clients`, `/clients/:id` | Client list and detail |
| `/projects`, `/projects/:id` | Project list and detail with assignments |
| `/approvals` | Submit / approve / reject time (manager and admin) |
| `/reports` | Grouped time reports with CSV / XLSX export |
| `/invoices`, `/invoices/:id` | Invoice list and detail with CSV / XLSX / PDF export |
| `/admin/users` | User and task management (admin) |
| `/settings` | Organization and application settings |

## Quick start

Everything runs inside the Nix dev shell, which provides the Rust toolchain, `dx`
(the Dioxus CLI), `sqlx-cli`, PostgreSQL, and `wasm-pack`. A running PostgreSQL is required
for anything that touches the database.

### Try the demo

The fastest way to see Horae is the bundled demo VM — it boots PostgreSQL, applies migrations,
seeds sample data, and enables the one-click admin login:

```sh
nix run .#demo
```

Then open http://localhost:3000/auth/login and choose **Sign in as Admin**.

### Develop locally

```sh
nix develop            # enter the dev shell
nix run .#postgres     # boot a NixOS VM running PostgreSQL (forwards host :5432 and :2222)
```

`DATABASE_URL` defaults to `postgres://localhost/horae`, which matches the forwarded VM port.
On first run, apply migrations and seed demo data, then start the hot-reloading dev server:

```sh
cargo run -p horae --features server -- migrate run   # apply pending migrations
cargo run -p horae --features server -- seed          # insert demo data (idempotent)

cd crates/horae && DEV_LOGIN=1 dx serve               # dev server on :8080, hot reload
```

Open http://localhost:8080/auth/login and choose **Sign in as Admin**. The admin bypass is
only available when `DEV_LOGIN=1` is set.

## Configuration

Horae is configured through environment variables.

| Variable | Default | Description |
|---|---|---|
| `DATABASE_URL` | `postgres://localhost/horae` | PostgreSQL connection URL |
| `HORAE_HOST` | `127.0.0.1` | Bind address |
| `HORAE_PORT` | `3000` | Listen port |
| `HORAE_LOG` | `info` | Log level (`trace`, `debug`, `info`, `warn`, `error`) |
| `SESSION_SECRET` | `dev-secret-…` | Cookie signing secret — **always set in production** |
| `HORAE_SECURE_COOKIES` | `0` | Mark session cookies `Secure` (HTTPS only); set to `1` in production |
| `DEV_LOGIN` | `0` | `1` enables the one-click admin login and bypasses OIDC (dev only) |
| `HORAE_PLUGINS_DIR` | `plugins` | Directory scanned for plugins at startup |

Production authentication uses OIDC. It is enabled only when all four OIDC variables are set
(and `DEV_LOGIN` is off):

| Variable | Description |
|---|---|
| `HORAE_OIDC_ISSUER` | OIDC provider issuer URL |
| `HORAE_OIDC_CLIENT_ID` | OIDC client ID |
| `HORAE_OIDC_CLIENT_SECRET` | OIDC client secret |
| `HORAE_OIDC_REDIRECT_URL` | Callback URL registered with the provider |
| `HORAE_OIDC_ADDITIONAL_AUDIENCES` | Extra `aud` values to trust (comma-separated); optional |
| `HORAE_OIDC_BUTTON_LABEL` | Sign-in button text; defaults to `Continue with SSO` |

## Self-hosting

Horae ships as a NixOS module:

```nix
{
  imports = [ horae.nixosModules.horae ];

  services.horae = {
    enable = true;
    host = "127.0.0.1";
    port = 3000;
    database.createLocally = true;   # provisions a local PostgreSQL and database
    # secretKeyFile = "/run/secrets/horae-env";
    # openFirewall = true;
  };
}
```

The module runs the server as a systemd service; you still apply migrations and (optionally)
seed with the `horae` CLI. Do not set `DEV_LOGIN` in a production deployment — configure OIDC
instead.

## Command-line interface

The `server`-feature binary doubles as an admin CLI (`serve` is the default when no subcommand
is given):

```sh
horae serve --host 0.0.0.0 --port 3000
horae migrate run                   # apply pending migrations
horae migrate reset --confirm       # drop and re-create (dev only)
horae seed                          # insert demo data (idempotent)
horae user list
horae user create --email admin@example.com --name "Admin" --role admin
```

## API

### Harvest-compatible (read-only)

Endpoints under `/harvest/v2` mirror the [Harvest API v2](https://help.getharvest.com/api-v2/)
response shape:

```
GET /harvest/v2/users/me
GET /harvest/v2/time_entries[?from=&to=&user_id=&project_id=&is_running=&page=&per_page=]
GET /harvest/v2/time_entries/{id}
GET /harvest/v2/projects[/{id}]
GET /harvest/v2/clients[/{id}]
GET /harvest/v2/tasks[/{id}]
GET /harvest/v2/users
```

Authentication is session-cookie based. Bearer-token auth is planned but not yet implemented.

### Export

```
GET /api/reports/export/{csv,xlsx}?from=YYYY-MM-DD&to=YYYY-MM-DD
GET /api/projects/export/{csv,xlsx}
GET /api/invoices/{id}/export/{csv,xlsx,pdf}
```

## Architecture

- **One feature-gated app crate (`crates/horae/`), two build targets.** `main.rs` defines three
  `cfg`-selected entry points: `server` (Axum + Tokio + the CLI), `web` (compiled to WASM), and a
  stub. Default features are empty, so builds and tests must select `--features server` (or use
  `dx`). Server-only modules (`auth`, `cli`, `config`, `db`, `harvest`, `reports`, `seed`,
  `state`, …) are gated; the shared UI modules compile for both targets.
- **A pure domain crate (`horae-core`).** Duration parsing, rounding, money, totals, and the entry
  state machine live here with no I/O dependencies, and are unit-tested in isolation.
- **Two API surfaces.** The server layers custom Axum routes — health, exports, auth, and the
  read-only Harvest API — on top of the Dioxus fullstack router, all under a Postgres-backed
  session layer. The SPA performs mutations through session-authenticated Dioxus `#[server]`
  functions.
- **PostgreSQL only.** Migrations live in `crates/horae/migrations/` and apply via `sqlx`.

Domain invariants worth knowing: durations are stored as integer minutes, money as integer minor
units (cents) plus an ISO currency code (never floats), and primary keys are UUID v7.

See [SPEC.md](SPEC.md) for the Phase-1 build spec and [DESIGN.md](DESIGN.md) for the design
system and component conventions.

## Development & testing

```sh
cargo test -p horae-core                              # pure domain unit tests (no DB, no features)
DATABASE_URL=… cargo test -p horae --features server  # integration tests (need Postgres w/ CREATEDB)
cargo clippy -p horae --features server               # lint
nix fmt                                               # treefmt: rustfmt, taplo, nixpkgs-fmt, mdformat
```

Integration tests use `#[sqlx::test]` (each spins up a throwaway database, so the DB role needs
`CREATEDB`). `nix build` builds the package and `nix flake check` runs the formatting check plus a
full NixOS end-to-end test.

All SQL uses sqlx's compile-time-checked macros; after changing a query or migration, regenerate
the offline cache with `cargo sqlx prepare --workspace -- --features server --all-targets` and
commit `.sqlx/`.

The repository also ships agent skills under `.agents/skills/` (Rust best practices, testing,
async patterns, and more) that capture the conventions this project follows.

## Contributing

Contributions are welcome. See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup and
guidelines, and [SPEC.md](SPEC.md) for the current build plan.

## License

Horae is licensed under the MIT License. See [LICENSE](LICENSE) for the full text.
