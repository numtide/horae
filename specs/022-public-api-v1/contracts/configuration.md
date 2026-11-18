# Contract: configuration and deployment

## Environment

| Variable | PR | Meaning | Default |
| --- | --- | --- | --- |
| `HORAE_HARVEST_API` | D | `1`/`true` mounts the Harvest-compatible API (`/harvest/v2`, `/v2`); anything else leaves it off and those paths answer 404 with an empty body | off |
| `HORAE_API_CURSOR_KEY` | F | 64 hex characters, the HMAC key for list cursors. Put it in the secret file | a random key per process, logged once at start; cursors then do not survive a restart |
| `HORAE_API_RATE_LIMIT` | F | `<requests>/<seconds>` per token, shared by both APIs | `100/15` |

An invalid value refuses to start, naming the variable, as the other `HORAE_*` settings do.

## NixOS module (`nix/modules/nixos/horae.nix`)

| Option | PR | Type | Default | Sets |
| --- | --- | --- | --- | --- |
| `services.horae.harvestApi.enable` | D | bool | `false` | `HORAE_HARVEST_API=1` when true |
| `services.horae.api.rateLimit` | F | string `<n>/<s>` | `"100/15"` | `HORAE_API_RATE_LIMIT` |

`HORAE_API_CURSOR_KEY` goes in `secretKeyFile`; the option description of `secretKeyFile` names
it beside the existing secrets.

## Upgrade note (release notes, PR D)

The Harvest-compatible API is now off by default. A deployment that uses `/harvest/v2` sets
`services.horae.harvestApi.enable = true` (or `HORAE_HARVEST_API=1`). The `/api/v2` alias is gone
(PR A); a Harvest client configured with a base URL ending in `/api` switches to the host root or
`/harvest`.
