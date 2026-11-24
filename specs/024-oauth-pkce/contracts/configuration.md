# Contract: configuration (PRs B, C, G)

Read at start-up in `crates/server/src/config.rs`, like the other `HORAE_*` settings.

| Setting | Variable | NixOS option | Default |
| --- | --- | --- | --- |
| Public base URL, the issuer | `HORAE_PUBLIC_URL` (absolute `https` URL of the host's root, with no path and no trailing slash, written in canonical form: lowercase scheme and host, no default port, no spaces; the error names that form; `http` allowed only for a loopback host) | `services.horae.publicUrl` | unset: the authorization server is off, `/oauth/*` and the metadata answer 404, and the admin apps page says so |
| Access token lifetime | `HORAE_OAUTH_ACCESS_TTL` (seconds) | `services.horae.oauth.accessTokenTtl` | `3600` |
| Refresh token idle expiry | `HORAE_OAUTH_REFRESH_IDLE` (seconds) | `services.horae.oauth.refreshIdle` | `2592000` (30 days, D-2) |
| Refresh absolute limit | `HORAE_OAUTH_REFRESH_MAX` (seconds) | `services.horae.oauth.refreshMax` | `31536000` (1 year, D-2) |
| Token endpoint limit | `HORAE_OAUTH_TOKEN_RATE_LIMIT` (`<requests>/<seconds>`) | `services.horae.oauth.tokenRateLimit` | `30/60` |

A malformed value refuses to start, as for `HORAE_API_RATE_LIMIT`. Lifetimes outside sane bounds
(access 60 s to 1 day; refresh idle 1 hour to 1 year; maximum at least the idle value and at
most 10 years, so adding it to a timestamp cannot overflow) refuse to start too.

Under `DEV_LOGIN=1`, the dev stack's `process-compose.yaml` sets `HORAE_PUBLIC_URL` to
`http://127.0.0.1:8080`, so the flow can be tried locally.

The built-in Android client's identifiers are constants in `ops::oauth::clients`:

| Field | Value |
| --- | --- |
| `client_id` | `horae-android` |
| `kind` | `public` |
| `redirect_uris` | `dev.horae.android:/oauth/callback` |
| `scopes` | `time:read time:write catalog:read` |

The scheme is a placeholder in reverse-domain form until the Android app's package name is fixed;
changing it later is a migration that updates the built-in rows.
