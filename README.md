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
process-compose up     # postgres + the app, on http://localhost:8080
```

Or `nix run .#dev` to do both at once.

That is the whole loop. `process-compose up` starts PostgreSQL as a plain process, waits for
it, applies migrations, then runs the hot-reloading dev server. Demo data is seeded the first time,
so there is nothing to run by hand.

Database state lives in `.data/postgres` and survives restarts; `rm -rf .data` starts over.
`process-compose down` stops everything.

Migrations run on every `process-compose up`, and only the pending ones are executed. To apply
a new one without restarting the stack, run `horae-migrate` — it migrates and rebuilds the app,
which is needed because sqlx checks its queries at compile time.

[pgweb](https://github.com/sosedoff/pgweb) runs alongside the app at http://localhost:8081 for
browsing and querying the database. To start only part of the stack, name the processes you
want — `process-compose up postgres migrate` gives you a database on its own, which is what the
integration tests need.

Open http://localhost:8080/auth/login and choose **Sign in as Admin**. The admin bypass is
only available when `DEV_LOGIN=1` is set, which the dev stack does for you.

<details>
<summary>Running PostgreSQL in a VM instead</summary>

`nix run .#postgres` boots a NixOS VM running PostgreSQL (forwarding host `:5432` and `:2222`).
It exercises the same NixOS module used for self-hosting, which makes it useful for checking
packaging changes, but it is slower to start and needs KVM. With it running, drive the app by
hand:

```sh
cargo run -p horae --features server -- migrate run   # apply pending migrations
cargo run -p horae --features server -- seed          # insert demo data (idempotent)

cd crates/horae && DEV_LOGIN=1 dx serve               # dev server on :8080, hot reload
```

</details>

## Configuration

Horae is configured through environment variables.

`horae` and `horae serve` use the same bind defaults and `HORAE_HOST`/`HORAE_PORT`
environment settings. Explicit `serve --host`/`--port` flags take precedence over
those variables. Under `dx serve`, Dioxus's `IP`/`PORT` variables override the bind
address for hot-reload proxying.

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

The module runs the server as a systemd service and applies pending migrations on every start.

### First run

A fresh database has no organization, so create one — along with the first admin user —
before anyone signs in. `horae seed` inserts demo clients, projects and time entries; it is
for trying the app out, not for standing one up.

`seed` initializes an empty, migrated database in one transaction, with sample time
for the current ISO week. Repeating it leaves an existing demo completely unchanged,
even in a later week: it does not add time, overwrite edits, or restore deleted rows.
Older or partially populated demos are also left untouched. It refuses a database
containing any non-demo organization; use a separate empty database to try the demo.

The CLI is the same binary as the service — put it on PATH with
`environment.systemPackages = [ config.services.horae.package ];`. The service runs under a
systemd `DynamicUser`, so run the CLI as `horae` while the unit is up and the socket
credentials match the database owner:

```sh
sudo -u horae DATABASE_URL=postgres:///horae \
  horae init --org-name "Contoso" --admin-email ops@contoso.example --admin-name "Ops"
```

`init` creates exactly one organization and one admin and refuses if an organization already
exists. Add the rest of the team with `horae user create`. Do not set `DEV_LOGIN` in a
production deployment — configure OIDC instead, and give the admin the email address the
provider will assert.

### Backups and restore

With `database.createLocally = true` the module enables `services.postgresqlBackup` for the
`horae` database. A nightly `pg_dump` lands in `/var/backup/postgresql/horae.sql.gz`, with the
previous run kept beside it as `horae.sql.prev.gz`. That is two days of history on the same
disk as the database — copy it somewhere else if it matters.

```nix
services.horae.database.backup = {
  enable = false;                       # opt out; on by default with createLocally
  location = "/var/backup/postgresql";  # where the dumps land
  startAt = "*-*-* 01:15:00";           # systemd.time(7) calendar format
};
```

The dumps are taken with `pg_dump -C`, so a restore recreates the database:

```sh
systemctl stop horae
sudo -u postgres dropdb horae
gunzip -c /var/backup/postgresql/horae.sql.gz | sudo -u postgres psql -d postgres
systemctl start horae
```

Migrations are forward-only — there are no down-migrations — so restoring a dump taken before
an upgrade is the only way back from one. Take a dump before upgrading.

## Command-line interface

The `server`-feature binary doubles as an admin CLI (`serve` is the default when no subcommand
is given):

```sh
horae serve --host 0.0.0.0 --port 3000
horae init --org-name "Contoso" --admin-email ops@contoso.example --admin-name "Ops"
horae migrate run                   # apply pending migrations
horae migrate reset --confirm       # drop every table and re-migrate (dev only)
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
`CREATEDB`). `nix build` builds the package and `nix flake check` runs the whole suite against a
PostgreSQL started inside the build sandbox, plus clippy, formatting, and the NixOS end-to-end
tests.

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
