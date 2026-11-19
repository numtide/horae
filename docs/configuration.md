# Configuration

Horae is configured through environment variables. [.env.example](../.env.example) lists the
supported settings for local development, but the binary does not load `.env` files
automatically: export the variables in your shell or set them in your service manager. On NixOS,
the module sets some of them from its options; see [Deployment](deployment.md).

## Database

PostgreSQL 15 or newer with ICU support is required. The bundled Nix PostgreSQL includes ICU;
externally managed databases must provide it too. Permission-profile names use a fixed Unicode
collation, so case-insensitive uniqueness does not change with the database locale.

## Environment variables

| Variable | Default | Description |
|---|---|---|
| `DATABASE_URL` | `postgres://localhost/horae` | PostgreSQL connection URL |
| `HORAE_HOST` | `127.0.0.1` | Bind address |
| `HORAE_PORT` | `3000` | Listen port |
| `HORAE_LOG` | `info` | Log level (`trace`, `debug`, `info`, `warn`, `error`) |
| `HORAE_SECURE_COOKIES` | `0` | `1` marks session cookies `Secure` (HTTPS only); set it in production |
| `DEV_LOGIN` | `0` | `1` enables the one-click admin login and bypasses OIDC (development only; see [Authentication](authentication.md#development-login)) |
| `HORAE_PLUGINS_DIR` | `plugins` | Directory scanned for plugins at startup |
| `HORAE_PLUGIN_DATABASE_URL` | unset | Separate unprivileged login for [plugin SQL](../specs/001-time-tracking-invoicing/contracts/plugin-interface.md); unset or empty disables it |
| `HORAE_JOB_MAX_ATTEMPTS` | `5` | Total attempts of a durable job, including the first run (1 to 100). Applies only to new jobs; queued jobs and manual retries keep their original limit |
| `HORAE_SENDMAIL_PATH` | unset | Absolute path to the optional budget-alert mail executable; no command arguments. See [Budget alert email](#budget-alert-email) |
| `HORAE_MAIL_FROM` | unset | Sender mailbox for budget alerts; required together with `HORAE_SENDMAIL_PATH` |
| `HORAE_API_CURSOR_KEY_FILE` | unset | Path to a secret file with the key that signs `/api/v1` list cursors (64 hex characters); see [API cursor key](deployment.md#api-cursor-key) |
| `HORAE_API_CURSOR_KEY` | unset | The same key as a value, for development and tests; ignored when `HORAE_API_CURSOR_KEY_FILE` is set |
| `HORAE_API_RATE_LIMIT` | `100/15` | Requests per personal access token per window, as `<requests>/<seconds>`. The Harvest-compatible API shares the limit |
| `HORAE_HARVEST_API` | `0` | `1` serves the read-only Harvest-compatible API; see [Harvest](harvest.md#harvest-compatible-api) |

`DEV_LOGIN`, `HORAE_SECURE_COOKIES` and `HORAE_HARVEST_API` also accept `true`.

Two more groups have their own pages: the `HORAE_OIDC_*` sign-in settings in
[Authentication](authentication.md#oidc) and the `HORAE_HARVEST_*` importer credentials in
[Harvest](harvest.md#importer). `HORAE_SESSION_FILE` belongs to the remote administrator commands
([CLI](cli.md#remote-administrator-commands)), and `HORAE_RELEASE_TAG` to release builds
([Releases](releases.md)).

### Bind address

`horae` and `horae serve` use the same bind defaults and the `HORAE_HOST`/`HORAE_PORT` settings.
Explicit `serve --host`/`--port` flags take precedence over those variables. The `IP` and `PORT`
variables, when set, override both; `dx serve` sets them for its hot-reload proxy.

### Validation at startup

An invalid API setting refuses to start, as does a cursor key file that is missing, unreadable or
not 64 hex characters. With neither cursor variable set, the server draws a random key at start
and logs a warning: cursors stop working after a restart. That fallback is for development only.

## Sessions and cookies

Sessions are stored in PostgreSQL; the cookie contains an opaque session identifier, not signed
user data. `SESSION_SECRET` is not used and can be removed from existing environment files;
changing it does not revoke sessions. Serve production traffic over HTTPS and set
`HORAE_SECURE_COOKIES=1`. The `Secure` flag restricts cookie transport; it does not configure TLS
or a reverse proxy.

## Budget alert email

Budget alerts are optional and use a sendmail-compatible program that you configure separately.
Leave both mail variables unset or empty to disable delivery; New Project then disables its email
checkbox and explains why. Setting only one variable, an invalid mailbox, or a path that is not an
absolute executable file rejects startup.

Set both values in the service environment and restart Horae after configuring the transport:

```dotenv
HORAE_SENDMAIL_PATH=/absolute/path/to/sendmail
HORAE_MAIL_FROM=alerts@example.com
```

Replace the example path with your executable. Horae invokes it directly with `-i -f FROM -- TO`
and writes the message to stdin; it does not run a shell or configure SMTP credentials. Sender and
recipient addresses must be plain ASCII mailboxes, without display names or multiple recipients.
The supported local-part characters are letters, digits, `.`, `_`, `+` and `-`; leading `-`/`.`,
trailing `.` and consecutive dots are rejected. Domains use nonempty DNS-style labels. See
[.env.example](../.env.example) for the environment template.

On NixOS, use `services.horae.secretKeyFile` or `systemd.services.horae.environment`. The
executable and its configuration must work as the service's dynamic user under `NoNewPrivileges`,
`ProtectSystem=strict` and `ProtectHome=true`. A transport that needs privilege elevation or
configuration in a home directory is not compatible with this sandbox; configure an unprivileged
transport instead of weakening the service protections. The [container image](deployment.md#container-image)
has no mail transport, so provide the executable yourself, for example from a mounted volume.

Each attempt has a 20-second deadline. Budget mail allows at most five attempts, independently of
`HORAE_JOB_MAX_ATTEMPTS`; permanent failures and revoked recipient/project eligibility stop
delivery earlier. Transport output is discarded, and stored errors are sanitized. An exit-zero
acknowledgement means transport acceptance, not final receipt. A crash or lost acknowledgement can
cause duplicate delivery with the same Message-ID. Acknowledged events are not retried; failed
events remain available for inspection.

An operator can inspect delivery state with a read-only query in the administrative database:

```sql
SELECT id, attempts, delivered_at, failed_at, last_error
FROM horae_outbox
WHERE event_kind = 'budget_email'
ORDER BY created_at DESC;
```
