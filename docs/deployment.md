# Deployment

Horae runs as a NixOS service or as an OCI container. Either way it needs PostgreSQL 15 or newer
with ICU ([Configuration](configuration.md#database)) and, in production, an OIDC provider
([Authentication](authentication.md)) and an HTTPS reverse proxy in front of it.

## NixOS module

```nix
{
  imports = [ horae.nixosModules.horae ];

  services.horae = {
    enable = true;
    host = "127.0.0.1";
    port = 3000;
    database.createLocally = true;   # provisions a local PostgreSQL and database
    # secretKeyFile = "/run/secrets/horae-env";
    # api.cursorKeyFile = "/run/secrets/horae-api-cursor-key";
    # openFirewall = true;
  };
}
```

The module runs the server as a systemd service and applies pending migrations on every start.

| Option | Default | Description |
|---|---|---|
| `host`, `port` | `127.0.0.1`, `3000` | Bind address and port |
| `database.url` | `null` | PostgreSQL URL. When null with `database.createLocally`, the service connects over the local socket (`postgres:///horae`) |
| `database.createLocally` | `true` | Provision a local PostgreSQL with a `horae` database owned by the service user |
| `database.backup.*` | on with `createLocally` | Nightly dumps; see [Backups and restore](#backups-and-restore) |
| `secretKeyFile` | `null` | systemd environment file with secrets; see [Secrets](#secrets) |
| `api.cursorKeyFile` | `null` | Key that signs `/api/v1` cursors; see [API cursor key](#api-cursor-key) |
| `api.rateLimit` | `"100/15"` | Requests per token per window, passed as `HORAE_API_RATE_LIMIT` |
| `harvestApi.enable` | `false` | Serve the Harvest-compatible API (`HORAE_HARVEST_API=1`); see [Harvest](harvest.md#harvest-compatible-api) |
| `logLevel` | `"info"` | Passed as `HORAE_LOG` |
| `openFirewall` | `false` | Open the firewall for `port` |

Other variables from [Configuration](configuration.md) go in `systemd.services.horae.environment`
when they are not secret, or in the `secretKeyFile` when they are.

### Secrets

For production, set `secretKeyFile` to a root-readable runtime file (for example,
`"/run/secrets/horae-env"`) provisioned outside the Nix store. It is a systemd environment file,
not a cookie-signing key. For an HTTPS deployment it can contain:

```dotenv
DEV_LOGIN=0
HORAE_SECURE_COOKIES=1
HORAE_OIDC_ISSUER=https://identity.example.com
HORAE_OIDC_CLIENT_ID=horae
HORAE_OIDC_CLIENT_SECRET=replace-with-provider-secret
HORAE_OIDC_REDIRECT_URL=https://time.example.com/auth/callback
```

Use your provider's issuer and credentials, register the exact redirect URL, and configure the
HTTPS reverse proxy separately. Do not put real secrets in Nix string literals or copy the
development admin bypass into a production environment.

### API cursor key

`/api/v1` list cursors are signed, so a client cannot forge a position in a list. Give each
deployment its own key:

1. Generate a key once: `openssl rand -hex 32` (64 hex characters).
1. Store it with the deployment's other secrets, so it lands at a runtime path such as
   `/run/secrets/horae-api-cursor-key`.
1. Set `services.horae.api.cursorKeyFile` to that path and deploy. The module passes it to the
   service through systemd `LoadCredential`.
1. Check the start log: it has no cursor-key warning.

While `api.cursorKeyFile` is unset, the module adds an evaluation warning and the server signs with
a random key drawn at every start. Rotating the key invalidates every cursor in flight; clients
start their list again after a `400 invalid_cursor`.

Outside NixOS, set `HORAE_API_CURSOR_KEY_FILE` to the key file.

## First run

A fresh database has no organization, so create one, along with the first admin user, before
anyone signs in. `horae seed` inserts demo clients, projects and time entries; it is for trying
the app out, not for standing one up.

`seed` initializes an empty, migrated database in one transaction, with sample time for the
current ISO week. Repeating it leaves an existing demo completely unchanged, even in a later week:
it does not add time, overwrite edits, or restore deleted rows. Older or partially populated demos
are also left untouched. It refuses a database containing any non-demo organization; use a
separate empty database to try the demo.

The CLI is the same binary as the service. Put it on PATH with
`environment.systemPackages = [ config.services.horae.package ];`. The service runs under a
systemd `DynamicUser`, so run the CLI as `horae` while the unit is up and the socket credentials
match the database owner:

```sh
sudo -u horae DATABASE_URL=postgres:///horae \
  horae init --org-name "Contoso" --admin-email ops@contoso.example --admin-name "Ops"
```

`init` creates exactly one organization and one admin and refuses if an organization already
exists. Add the rest of the team with `horae user create` ([CLI](cli.md)), or let them create
their own account at first sign-in
([Authentication](authentication.md#creating-accounts-at-first-sign-in)). Do not set `DEV_LOGIN`
in a production deployment. Configure OIDC instead, and give the admin the email address the
provider will assert.

## Container image

The flake also builds the server as an OCI image, for x86_64-linux and aarch64-linux:

```sh
nix build .#oci-image                                     # host architecture
nix build .#packages.aarch64-linux.oci-image              # needs an aarch64 builder
```

`result` is an `oci-archive`: a tar of an OCI image layout (`oci-layout`, `index.json`, OCI
manifest and config, gzip layers), tagged `horae:<version>`. It is not a `docker save` archive.
Podman and skopeo read it directly; for Docker, convert it to a Docker archive first:

```sh
podman load -i result

skopeo copy oci-archive:result docker-archive:horae.tar:horae:0.1.0
docker load -i horae.tar
```

PostgreSQL runs outside the container; pass its URL and the rest of the configuration as
environment variables. CLI subcommands run from the same image:

```sh
podman run --rm -e DATABASE_URL=postgres://horae@db.example/horae horae:0.1.0 migrate run
podman run --rm -e DATABASE_URL=… horae:0.1.0 \
  init --org-name "Contoso" --admin-email ops@contoso.example --admin-name "Ops"
podman run -d -p 127.0.0.1:3000:3000 --read-only --tmpfs /tmp \
  -e DATABASE_URL=… --env-file /run/secrets/horae-env horae:0.1.0
```

The image runs `horae serve` as uid/gid 65534 (`nobody`), bound to `0.0.0.0:3000` inside the
container, from the directory holding the bundled WASM, JS and CSS. That bind is only the
container's own network namespace: which host addresses reach it depends on how you publish the
port. The example above publishes it on host loopback only; terminate TLS in a reverse proxy or
private ingress in front of it, as for the NixOS module. Behind HTTPS, set
`HORAE_SECURE_COOKIES=1` in the container's environment, since the server marks its own cookies.

The image holds the server's runtime closure (the binary and the libraries it links) and minimal
NSS files for the unprivileged user. It has no shell, database server, data or secrets, and no
mail transport: a `HORAE_SENDMAIL_PATH` executable or a `HORAE_PLUGINS_DIR` has to be provided to
the container yourself, for example from a mounted volume. Outbound HTTPS to an OIDC provider or
Harvest trusts the Mozilla root certificates compiled into the binary (webpki-roots); a custom CA,
through `SSL_CERT_FILE` or otherwise, is not supported. `DEV_LOGIN` is unset, and with the default
bind, which is not loopback, the server refuses to start if you set it. `serve` applies pending
migrations on start.

No image is published to any registry; build it yourself from the flake. To build one that shows a
release version, see [Releases](releases.md).

## Backups and restore

With `database.createLocally = true` the module enables `services.postgresqlBackup` for the
`horae` database. A nightly `pg_dump` lands in `/var/backup/postgresql/horae.sql.gz`, with the
previous run kept beside it as `horae.sql.prev.gz`. That is two days of history on the same disk
as the database, so copy it somewhere else if it matters.

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

When the database lives elsewhere, back it up there.

## Upgrading

Migrations are forward-only (there are no down-migrations), so restoring a dump taken before an
upgrade is the only way back from one. Take a dump before upgrading.

Administrators see the running server and browser versions above the account menu; see
[Releases](releases.md).

### The migration baseline

The 65 original migrations were squashed into one baseline, `0001_baseline.sql`. A database
migrated before that refuses to start (sqlx reports migration 2 as applied but missing). On a
deployment, run `horae migrate reset --confirm` and then `horae init`. The reset drops every
table, so take a dump first if the data matters.

### Upgrading to the release with `/api/v1`

Check these before deploying it:

- The Harvest-compatible API is off by default. If anything uses `/harvest/v2` (or `/v2`), set
  `services.horae.harvestApi.enable = true`, or `HORAE_HARVEST_API=1` outside NixOS. Its clients
  now authenticate with a personal access token.
- `/api/v2` is gone. A Harvest client configured with a base URL ending in `/api` switches to the
  host root or to `/harvest`.
- Set the cursor key. Put a key in `services.horae.api.cursorKeyFile`, as in
  [API cursor key](#api-cursor-key). Without it, cursors stop working at every restart.
- Timers changed for everyone, in the web app too. Start on an entry of today resumes it, starting
  a timer stops the one already running, and stopping adds the elapsed time to the minutes the
  entry already had.

### Accounts linked before issuers were recorded

Accounts linked before Horae recorded the OIDC issuer cannot sign in until they are linked again;
see [Authentication](authentication.md#upgrading-accounts-linked-without-an-issuer).
