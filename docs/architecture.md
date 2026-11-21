# Architecture

## Crates

- One feature-gated app crate (`crates/horae/`), two build targets. `main.rs` defines three
  `cfg`-selected entry points: `server` (Axum + Tokio + the CLI), `web` (compiled to WASM), and a
  stub. Default features are empty, so builds and tests must select `--features server` (or use
  `dx`). The server-only `cli` module is gated; the shared UI modules (`app`, `route`, `pages`,
  `components`, `server_fns`) compile for both targets.
- A pure domain crate (`horae-core`). Duration parsing, rounding, money, totals, and the entry
  state machine live here with no I/O dependencies, and are unit-tested in isolation.
- The types that travel between the UI and the server live in `horae-types`, which builds for the
  server and for WASM.
- The backend and the HTTP surfaces are their own crates. `horae-server` holds the operations,
  jobs, importers, configuration and migrations; `horae-api` serves `/api/v1`, the exports, `/auth`
  and `/health`; `horae-harvest-api` serves the optional Harvest-compatible API. The server layers
  them on top of the Dioxus fullstack router, under a Postgres-backed session layer. The SPA
  performs mutations through session-authenticated Dioxus `#[server]` functions, and `/api/v1`
  through the same operations with a token.
- PostgreSQL only. Migrations live in `crates/server/migrations/` and apply via `sqlx`.

Domain invariants worth knowing: durations are stored as integer minutes, money as integer minor
units (cents) plus an ISO currency code (never floats), and primary keys are UUID v7.

[AGENTS.md](../AGENTS.md#architecture) describes each crate in more detail. See the
[Speckit feature specifications](../specs/) and the
[Horae Constitution](../.specify/memory/constitution.md) for requirements and cross-cutting
invariants, and [DESIGN.md](../DESIGN.md) for the design system and component conventions.

## Web app routes

The authenticated SPA is organized by route:

| Route | Description |
|---|---|
| `/timesheet/:view` | Day / Week / Calendar timesheet with weekly totals (`/` redirects here) |
| `/clients`, `/clients/:id` | Client list and detail |
| `/projects`, `/projects/:id` | Project list and detail with assignments |
| `/projects/new` | Create a project with a private resumable draft (with project creation permission) |
| `/projects/:id/edit` | Edit a project |
| `/tasks` | Task catalogue |
| `/approvals` | Submit / approve / reject time (with approval permission) |
| `/reports` | Grouped time reports with CSV / XLSX export |
| `/invoices`, `/invoices/:id` | Invoice list and detail with CSV / XLSX / PDF export |
| `/invoices/settings` | The organization's issuer details, payment details and invoice defaults |
| `/admin/users` | People and their permissions (with people permissions) |
| `/admin/workspace` | Workspace settings, such as the time zone, and the installed plugins |
| `/admin/importers` | Importers, including the [Harvest importer](harvest.md#importer) and its runs |
| `/admin/audit` | Audit log of permission changes |
| `/admin/api-tokens` | Everyone's API tokens |
| `/settings/profile` | My settings: your name, email, sign-in and time zone (`/settings` opens it) |
| `/settings/preferences` | My settings: the theme on this device |
| `/settings/api-tokens` | My settings: your API tokens |
| `/settings/access` | My settings: what your account can see and do |

`/auth/*` is served by Axum, outside the SPA.

## Other HTTP routes

- `/api/v1`: the public API, with personal access tokens. See the [API guide](api.md).
- `/harvest/v2` and `/v2`: the optional Harvest-compatible API. See [Harvest](harvest.md).
- `/health`.
- Exports, authenticated by the web session:

```
GET /api/reports/export/{csv,xlsx}?from=YYYY-MM-DD&to=YYYY-MM-DD
GET /api/reports/time/grouped/{csv,xlsx}
GET /api/projects/export/{csv,xlsx}
GET /api/invoices/{id}/export/{csv,xlsx,pdf}
```
