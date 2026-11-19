# Horae documentation

Running Horae:

- [Configuration](configuration.md): the environment variables, the database requirements,
  sessions and cookies, and budget alert email.
- [Deployment](deployment.md): the NixOS module, secrets, the API cursor key, the first
  organization and administrator, the container image, backups, and upgrading.
- [Authentication](authentication.md): OIDC, linking identities, accounts created at first
  sign-in, the development login, and personal access tokens.
- [Harvest](harvest.md): the importer and the Harvest-compatible API.
- [Command-line interface](cli.md): local and remote administration commands.

Using the API:

- [API v1](api.md): tokens, requests, cursors, sync, errors, limits and compatibility.

Working on Horae:

- [Development](development.md): the dev stack, migrations, tests and the sqlx query cache.
- [Architecture](architecture.md): the crates, the web app's routes and the other HTTP routes.
- [Versions and releases](releases.md): what the version panel shows and how to cut a release.
- [Design audit](design/AUDITORIA-DISENO.md) (in Spanish): the 2026-11-18 audit of the interface
  that [DESIGN.md](../DESIGN.md) and the [implementation inventory](../design/IMPLEMENTATION-GAPS.md)
  build on.
- [CONTRIBUTING.md](../CONTRIBUTING.md) and [AGENTS.md](../AGENTS.md) at the repository root.
