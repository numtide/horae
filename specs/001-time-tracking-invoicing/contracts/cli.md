# CLI Contract

The `horae` binary is the server-and-tooling entry point. It is built from the
`server` feature of the single Horae crate; the CLI is defined in `crates/horae/src/cli.rs`
(clap) and dispatched in `crates/horae/src/main.rs`. Configuration is read from the
environment in `crates/horae/src/config.rs`.

Invocation forms:

1. `horae <subcommand> [args]` when running the compiled server binary.
1. `cargo run -p horae --features server -- <subcommand> [args]` during development.
1. Running the binary with **no subcommand** resolves the same defaults and
   environment variables as `serve`. This lets `dx serve` launch the binary bare.

## Subcommands

### `serve`

Starts the HTTP server: the Dioxus fullstack application plus the layered Axum
routes (`/health`, CSV/XLSX export, auth router, and the read-only Harvest v2
API), all under a Postgres-backed session layer.

Arguments:

1. `--host <HOST>` — bind address. Defaults to `127.0.0.1`. Also settable via
   the `HORAE_HOST` environment variable.
1. `--port <PORT>` — listen port. Defaults to `3000`. Also settable via the
   `HORAE_PORT` environment variable.

Behavior notes:

1. On startup, `serve` creates the Postgres pool and **runs migrations eagerly**
   before serving, then initializes shared application state. So a fresh `serve`
   applies pending migrations without a separate `migrate run`.
1. When running under `dx serve`, the `IP` and `PORT` environment variables (set
   by the Dioxus dev tooling for hot-reload proxying) **override** the resolved
   `--host` / `--port` values.

### `migrate run`

Applies all pending database migrations, then exits. The `run` subcommand is
required; a bare `migrate` does not select it automatically.

### `migrate reset --confirm`

Drops the `public` and `tower_sessions` schemas and re-applies the application
migrations (development only). This destroys application and session data; it
does not drop the database itself. The session layer recreates its schema on
the next server start.

1. `--confirm` is **required**. Without it the command prints
   `Pass --confirm to reset the database.` and exits with a non-zero status
   (exit code `1`).

### `init --org-name <NAME> --admin-email <EMAIL> --admin-name <NAME>`

Applies pending migrations, then creates one organization and its first admin.
Refuses if any organization already exists. Use the email the OIDC provider
will assert; the command does not provision an account at the provider.

### `seed`

Initializes an empty, migrated database with demo data and then verifies it.
Creates the admin used by `DEV_LOGIN` and sample time for the current ISO week
in one transaction. Repeating it leaves an existing demo unchanged, including
older, edited, or partially populated demos. Refuses any non-demo organization.

### `user create --email <EMAIL> --name <NAME> [--role <ROLE>]`

Creates a user in the existing organization. Fails if the organization is absent,
the role is invalid, or the email violates the database uniqueness constraint.

1. `--email` — required.
1. `--name` — required.
1. `--role` — optional; defaults to `member`; accepts `admin`, `manager`, or `member`.

### `user list`

Lists stored users by name with their email, organization role, and active status.
Prints `No users found.` only when the query returns no users.

## Environment Variables

Horae reads the process environment, not `.env` files. Export variables or use a
service manager's environment file. `.env.example` is a local-development example,
not a production configuration: it enables the admin bypass.

### Read by Clap (`crates/horae/src/cli.rs`)

1. `HORAE_HOST` — server bind address. Default `127.0.0.1`. Backs `serve --host`.
1. `HORAE_PORT` — server listen port. Default `3000`. Backs `serve --port`.

Explicit `serve` flags override these variables. Under `dx serve`, the Dioxus
`IP`/`PORT` variables override the result for hot-reload proxying.

### Read by `AppConfig` (`crates/horae/src/config.rs`)

1. `DATABASE_URL` — Postgres connection string. Default in code:
   `postgres://localhost/horae`. Used by every subcommand that touches the database.
1. `HORAE_LOG` — log level: `trace`, `debug`, `info`, `warn`, or `error`.
   Default `info`. (The standard `RUST_LOG` env filter, when set, takes
   precedence over this value.)
1. `DEV_LOGIN` — when `1` or `true`, skip OIDC and enable one-click login as the
   seeded admin user. Default: disabled (any other value, or unset). For
   development only.
1. `HORAE_SECURE_COOKIES` — when `1` or `true`, mark session cookies `Secure`.
   Default: disabled. Set it for production HTTPS; it does not configure TLS.
1. `HORAE_PLUGINS_DIR` — plugin directory. Default `plugins`.
1. `HORAE_PLUGIN_DATABASE_URL` — separate restricted login for plugin SQL.
   Unset or empty disables plugin SQL; see the [plugin contract](plugin-interface.md).

### OIDC (`AppConfig.oidc`)

The four required values must all be non-empty; otherwise OIDC remains
unconfigured. `DEV_LOGIN=1` bypasses OIDC even if these values are present.

1. `HORAE_OIDC_ISSUER` — provider issuer URL for metadata discovery.
1. `HORAE_OIDC_CLIENT_ID` — client ID.
1. `HORAE_OIDC_CLIENT_SECRET` — client secret.
1. `HORAE_OIDC_REDIRECT_URL` — exact public callback URL registered with the
   provider, ending in `/auth/callback`.
1. `HORAE_OIDC_ADDITIONAL_AUDIENCES` — optional comma-separated additional
   trusted audiences. Whitespace and empty entries are discarded.
1. `HORAE_OIDC_BUTTON_LABEL` — optional button text; default `Continue with SSO`.

The old unprefixed `OIDC_*` variables are not read.

### Harvest (`AppConfig.harvest`)

`HORAE_HARVEST_CLIENT_ID`, `HORAE_HARVEST_CLIENT_SECRET`,
`HORAE_HARVEST_REDIRECT_URL`, and `HORAE_HARVEST_ENC_KEY` must all be non-empty
to configure the API importer. The key is 32 bytes encoded as 64 hex digits;
the separate Harvest callback ends in `/auth/harvest/callback`. See the
[importer setup](../../004-harvest-importer/quickstart.md).

### Session storage

Sessions live in PostgreSQL; the cookie carries an opaque session identifier.
`SESSION_SECRET` is not used to sign cookies and can be removed from environment
files. Removing or changing it does not revoke existing sessions. Cookie transport
is controlled by `HORAE_SECURE_COOKIES`, not the unprefixed `SECURE_COOKIES`.

## Exit Behavior Summary

1. `serve` runs until the process is terminated; it applies migrations before
   listening.
1. `migrate reset` without `--confirm` exits with code `1` and a message.
1. Other subcommands run to completion and exit; database errors surface as
   process errors (non-zero exit) via the `anyhow` result in `main`.
