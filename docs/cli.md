# Command-line interface

The server binary doubles as an administration CLI. `serve` is the default when no subcommand is
given.

## Local commands

These connect to PostgreSQL through `DATABASE_URL`:

```sh
horae serve --host 0.0.0.0 --port 3000
horae init --org-name "Contoso" --admin-email ops@contoso.example --admin-name "Ops"
horae migrate run                   # apply pending migrations
horae migrate reset --confirm       # drop every table and re-migrate (dev only)
horae seed                          # insert demo data (idempotent)
horae user list
horae user create --email admin@example.com --name "Admin" --profile administrator
horae user link-oidc --user-id <ID> --issuer '<iss>' --subject '<sub>'
```

- `serve` takes its bind address as described in [Bind address](configuration.md#bind-address).
- `init` creates the first organization and administrator and refuses if an organization exists;
  see [First run](deployment.md#first-run).
- `seed` fills an empty database with demo data. `--api-token-file FILE` also writes an API token
  of the demo administrator, for test harnesses.
- `user create` takes a built-in profile: `administrator`, `executive_manager`,
  `project_manager`, `accounting`, `people_admin` or `member` (the default).
- `user link-oidc` binds an OIDC identity to an existing active account and never replaces a
  binding; see [Linking OIDC identities](authentication.md#linking-oidc-identities).

On a NixOS host, run them as the service user; [First run](deployment.md#first-run) shows how. In
the container image, pass them as the container's command
([Container image](deployment.md#container-image)). In a checkout, use
`cargo run -p horae --features server -- <subcommand>`.

## Remote administrator commands

`import harvest-api`, `import harvest-csv FILE` and `jobs status|list|report|errors|cancel|retry|wait`
talk to a running server. They need a private session file selected by `--session-file` or
`HORAE_SESSION_FILE`, and they do not load deployment configuration, connect to PostgreSQL or
start workers. The [jobs CLI quickstart](../specs/006-harvest-jobs-cli/quickstart.md) covers
authentication, preview and commit, waiting, JSON output and exit codes.
