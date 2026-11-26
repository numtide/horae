# Harvest

Horae works with Harvest in two directions. The importer copies an organization's Harvest data
into Horae, and the Harvest-compatible API lets tools written for Harvest read from Horae. They
are configured separately: the importer works whether the API is on or off.

## Importer

Administrators import from **Administration → Importers** (`/admin/importers`), over Harvest's
OAuth API or from a Harvest CSV export. The terminal has the same imports through the remote
administrator commands ([CLI](cli.md#remote-administrator-commands)).

The OAuth import has its own optional credentials:

| Variable | Description |
|---|---|
| `HORAE_HARVEST_CLIENT_ID` | Harvest OAuth app client ID |
| `HORAE_HARVEST_CLIENT_SECRET` | Harvest OAuth app client secret |
| `HORAE_HARVEST_REDIRECT_URL` | Callback URL registered with Harvest, ending in `/auth/harvest/callback`. Without it, the importer uses the address you open it from |
| `HORAE_HARVEST_ENC_KEY` | 32-byte key as 64 hex digits that encrypts the stored Harvest tokens; required |

See the [importer setup](../specs/004-harvest-importer/quickstart.md) for its callback and
token-encryption requirements; OIDC settings do not configure Harvest access. With only
`HORAE_HARVEST_ENC_KEY` set, an administrator can save the OAuth client from
Import → Harvest → OAuth app instead; its secret is sealed with that key. A client ID and secret
set in the environment take precedence and show there as locked.

## Harvest-compatible API

Endpoints under `/harvest/v2` expose a subset of the
[Harvest API v2](https://help.getharvest.com/api-v2/) response shape:

```
GET    /harvest/v2/company
GET    /harvest/v2/users/me
GET    /harvest/v2/users/me/project_assignments
GET    /harvest/v2/time_entries[?user_id=&client_id=&project_id=&task_id=&is_billed=&is_running=
                                &approval_status=&from=&to=&updated_since=&page=&per_page=&cursor=]
GET    /harvest/v2/time_entries/{id}
POST   /harvest/v2/time_entries
PATCH  /harvest/v2/time_entries/{id}
DELETE /harvest/v2/time_entries/{id}
PATCH  /harvest/v2/time_entries/{id}/stop
PATCH  /harvest/v2/time_entries/{id}/restart
GET    /harvest/v2/projects[/{id}]
GET    /harvest/v2/clients[/{id}]
GET    /harvest/v2/tasks[/{id}]
GET    /harvest/v2/users
GET    /harvest/v2/{expenses,expense_categories,estimates,estimate_item_categories,
                    contacts,invoice_item_categories,roles}   (always empty)
```

Time-entry writes take their parameters as JSON, as a form or in the query string, and run the
same operation as the Timesheet, with its rules and refusals. `POST` with `hours` records that
duration; with `started_time` and `ended_time` it records the time between them; with neither it
starts a timer on the person's today and stops any timer they have running. Hours are rounded
half up to whole minutes.

The same endpoints answer under `/v2`.

The subset is off by default: `/harvest/v2/*` and `/v2/*` answer `404` with an empty body. Turn it
on with `services.horae.harvestApi.enable = true` in the NixOS module, or `HORAE_HARVEST_API=1`
elsewhere; the setting is read at start. There is no `/api/v2` alias: a Harvest client configured
with a base URL ending in `/api` switches to the host root or `/harvest`.

Clients authenticate with a personal access token, the same one `/api/v1` takes, sent as
`Authorization: Bearer` or the `access_token` query parameter, with the account ID shown on the
token page as `Harvest-Account-Id`. A signed-in browser session also works for reads; a write with
only the session cookie answers `401`. Requests count against
the same per-token rate limit as `/api/v1` ([Limits](api.md#limits)).

An app's OAuth access token ([Apps](api.md#apps)) works too, in the `Authorization` header only:
in the `access_token` query parameter it answers `401`. It is limited to its scopes:

| Scope | Endpoints |
| --- | --- |
| any | `/company`, `/users/me` |
| `time:read` | `GET /time_entries`, `GET /time_entries/{id}` |
| `time:write` | every time entry write; includes `time:read` |
| `catalog:read` | my project assignments, projects, clients, tasks, users, and the empty lists of kinds Horae lacks |

A request outside the token's scopes answers `403` with `"error": "insufficient_scope"` and a
`WWW-Authenticate: Bearer error="insufficient_scope", scope="…"` header. An app's requests count
against its authorization, so a refreshed access token keeps the same limit.

IDs are integers, as on Harvest; the older UUID form is still accepted in paths and filters.
Lists take `per_page` from 1 to 2000 (default 2000) and `page`; a page size or page number out
of range answers `422`. Links in each list are absolute URLs on the host the request arrived on
(with the scheme from `X-Forwarded-Proto`) and keep the request's filters. `links.next` and
`links.previous` carry a `cursor`: following them walks a list without skipping or repeating a
record, whatever is edited or deleted meanwhile, while counting up `page` can skip one when a
record is deleted. Clients, projects, tasks and users run newest first; time entries by
`spent_date`, latest first. `updated_since` works on every list except my project assignments.
A list holds back records written after the oldest still-running write began until that write
ends, so the latest `updated_at` a client has seen is a safe `updated_since` for its next run.
Lists of kinds Horae lacks, such as expenses and estimates, are empty.

This is not a drop-in replacement for arbitrary Harvest clients. Check the
[API contract and compatibility matrix](../specs/001-time-tracking-invoicing/contracts/harvest-api.md)
before connecting a tool; several filters, timestamp semantics, and response fields differ.
