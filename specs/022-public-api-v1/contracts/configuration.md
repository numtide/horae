# Contract: configuration and deployment

## Environment

| Variable | PR | Meaning | Default |
| --- | --- | --- | --- |
| `HORAE_HARVEST_API` | D | `1`/`true` mounts the Harvest-compatible API (`/harvest/v2`, `/v2`); anything else leaves it off and those paths answer 404 with an empty body | off |
| `HORAE_API_CURSOR_KEY_FILE` | F | path to a secret file holding the HMAC key for list cursors (64 hex characters). Production sets it (the NixOS module does, below) | unset |
| `HORAE_API_CURSOR_KEY` | F | the same key as a value, for development and tests; ignored when `HORAE_API_CURSOR_KEY_FILE` is set | unset |
| `HORAE_API_RATE_LIMIT` | F | `<requests>/<seconds>` per token, shared by both APIs | `100/15` |

An invalid value refuses to start, naming the variable, as the other `HORAE_*` settings do; so
does a cursor key file that is missing, unreadable or not 64 hex characters.

**Development fallback**: with neither cursor variable set, the server draws a random key at
start and logs a warning at start: cursors will not survive a restart, and production must set
`HORAE_API_CURSOR_KEY_FILE`. It is meant for `process-compose up` and tests only.

## NixOS module (`nix/modules/nixos/horae.nix`)

| Option | PR | Type | Default | Sets |
| --- | --- | --- | --- | --- |
| `services.horae.harvestApi.enable` | D | bool | `false` | `HORAE_HARVEST_API=1` when true |
| `services.horae.api.rateLimit` | F | string `<n>/<s>` | `"100/15"` | `HORAE_API_RATE_LIMIT` |
| `services.horae.api.cursorKeyFile` | F | null or path (a runtime secret path, outside the Nix store) | `null` | `LoadCredential=api-cursor-key:<path>` and `HORAE_API_CURSOR_KEY_FILE=%d/api-cursor-key` |

While `api.cursorKeyFile` is `null` the module adds an evaluation warning that the server will use
a random cursor key per start.

## Deployment runbook: cursor key (PR F)

1. Generate a key once per deployment: `openssl rand -hex 32` (64 hex characters).
1. Store it with the deployment's secrets (the same mechanism that delivers `secretKeyFile`), so
   it lands at a runtime path such as `/run/secrets/horae-api-cursor-key`.
1. Set `services.horae.api.cursorKeyFile` to that path and deploy.
1. Check the start log: no cursor-key warning.

Rotating the key invalidates every cursor in flight; clients restart their walk (400
`invalid_cursor`). io and shipyard get the key in the deployment that ships PR F.

## Upgrade note (release notes, PR D)

The Harvest-compatible API is now off by default. A deployment that uses `/harvest/v2` sets
`services.horae.harvestApi.enable = true` (or `HORAE_HARVEST_API=1`). The `/api/v2` alias is gone
(PR A); a Harvest client configured with a base URL ending in `/api` switches to the host root or
`/harvest`.
