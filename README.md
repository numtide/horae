<p align="center">
  <img src="assets/banner.png" alt="Horae, self-hostable time tracking" width="820">
</p>

Horae is a time tracker you run on your own infrastructure, an alternative to Harvest and
Kimai. Log hours against clients and projects, submit and approve timesheets, and export
billable reports, with the data in your own PostgreSQL database.

It is a Rust + [Dioxus](https://dioxuslabs.com/) fullstack application (server-rendered, with a
WebAssembly SPA) on Axum and PostgreSQL.

## What it does

- Timesheets in Day, Week and Calendar views, with weekly totals and a running timer.
- Clients, projects, per-project assignments, tasks and budgets, with optional budget alert email.
- Approvals: people with approval permission submit, approve and reject time. Only an
  Administrator may approve or reject their own.
- Permissions from built-in profiles, templates or individual grants, scoped to own, managed or
  all, plus an explicit Administrator identity.
- Grouped time reports with CSV and XLSX export.
- Draft invoices with CSV, XLSX and PDF export.
- An importer for Harvest data, over OAuth or from a CSV export.
- A public API, `/api/v1`, with personal access tokens, and an optional read-only subset of the
  Harvest API v2 at `/harvest/v2`, off unless the operator turns it on.
- OIDC single sign-on in production, and a one-click login for local development.

## Try it

The demo boots a NixOS VM with PostgreSQL, applies the migrations, seeds sample data and turns on
the one-click admin login:

```sh
nix run .#demo
```

Open http://localhost:3000/auth/login and choose **Sign in as Admin**.

## Run it locally

You need Nix with flakes enabled.

```sh
nix develop            # dev shell: Rust toolchain, dx, sqlx-cli, PostgreSQL
process-compose up     # PostgreSQL + the app on http://localhost:8080, migrated and seeded
```

`nix run .#dev` does both at once. Open http://localhost:8080/auth/login and choose
**Sign in as Admin**. [Development](docs/development.md) covers the rest of the loop: resetting
the database, new migrations, tests and the sqlx query cache.

## Deploy it

Horae ships as a NixOS module and as an OCI image you build from the flake. A minimal NixOS
configuration:

```nix
{
  imports = [ horae.nixosModules.horae ];

  services.horae = {
    enable = true;
    database.createLocally = true;
  };
}
```

[Deployment](docs/deployment.md) covers secrets, the first organization and administrator, the
container image, backups and upgrades. Production sign-in uses OIDC; see
[Authentication](docs/authentication.md).

## Status

Horae is in active Phase-1 development. Expect the schema and the API to keep moving. `/api/v1`
is the exception: within v1 it only gains additions (see the
[compatibility promise](docs/api.md#compatibility)).

## Documentation

- [Configuration](docs/configuration.md): environment variables, sessions, budget alert email.
- [Deployment](docs/deployment.md): NixOS module, container image, first run, backups, upgrading.
- [Authentication](docs/authentication.md): OIDC, linking identities, accounts created at sign-in,
  the development login.
- [API v1](docs/api.md): personal access tokens, cursors, errors, limits.
- [Harvest](docs/harvest.md): the importer and the Harvest-compatible API.
- [Command-line interface](docs/cli.md)
- [Development](docs/development.md) and [Architecture](docs/architecture.md)
- [Releases](docs/releases.md)

Requirements and plans for each feature live in [specs/](specs/), the cross-cutting rules in the
[Horae Constitution](.specify/memory/constitution.md), and the design system in
[DESIGN.md](DESIGN.md).

## Contributing

Contributions are welcome. [CONTRIBUTING.md](CONTRIBUTING.md) explains the setup, the
conventions and how to submit changes.

## License

Horae is licensed under the MIT License. See [LICENSE](LICENSE) for the full text.
