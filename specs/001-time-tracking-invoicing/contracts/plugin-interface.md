# Plugin Interface Contract

**Status: Runtime implemented (User Story 5); hot reload remains planned.**
This interface contract is derived from PLAN.md's "Plugin System" section and
functional requirements **FR-018..FR-022**. It
defines the plugin manifest, the event catalog, the host functions, the
dashboard-widget return shape, and the sandbox / failure-isolation guarantees so that
implementation and plugin authors share one contract.

## Overview

Plugins are **WASM modules loaded at runtime via
[extism](https://github.com/extism/extism)**. Any language that compiles to WASM
(Rust, Go, TypeScript, C, Zig, …) can author a Horae plugin. Plugins are
operator-trusted but **sandboxed**: they may only call the host functions Horae
explicitly exposes and can never write to the datastore or render arbitrary UI code.

Module layout (`crates/horae/src/plugin/`):

1. `registry.rs` — `PluginRegistry`: scans the `plugins/` data directory, loads each
   `*.wasm` at startup, holds a handle per plugin.
1. `host.rs` — extism host functions (logging, read-only DB queries, HTTP POST,
   config lookup).
1. `event.rs` — the `AppEvent` enum, serialized to JSON and passed to plugins.
1. `manifest.rs` — the `plugin.toml` schema.

At startup the registry loads plugins sequentially on a blocking worker and
registers each valid plugin for the hooks it declares (FR-018). Directory access,
compilation, and instantiation do not run on an async runtime worker.
`AppState` holds `plugins: Arc<PluginRegistry>`; on each business
event, `registry.dispatch(event)` invokes all subscribed plugins concurrently
(FR-019).

______________________________________________________________________

## Plugin manifest (`plugin.toml`)

Each plugin ships a `plugin.toml` sidecar (or embedded section) in its directory
under `{dataDir}/plugins/`, next to the `*.wasm` module.

```toml
[plugin]
name = "slack-notifier"
version = "1.0.0"
hooks = ["time_entry_created", "invoice_sent"]
```

Schema:

1. `name` (string, required) — unique plugin identifier; also the key under which the
   operator stores this plugin's configuration.
1. `version` (string, required) — semantic version of the plugin.
1. `hooks` (array of strings, required) — the event names this plugin subscribes to.
   Each name MUST be one of the events in the catalog below. For every declared hook,
   the plugin MUST export a WASM function whose name matches the hook (e.g.
   `time_entry_created`). Horae calls that export with the event's JSON payload.

A manifest that is malformed, declares an unknown hook, or references a missing export
is **rejected at load time** and the plugin is not registered (spec edge case:
malformed/unsupported plugins are rejected and gain no capabilities).

______________________________________________________________________

## Event catalog

Horae dispatches these business events to subscribed plugins (FR-019). Each event is
delivered as a single JSON object argument to the matching exported function. All
timestamps are RFC 3339 UTC; all IDs are UUID v7 strings; durations are integer
minutes and money is integer minor units (cents) + ISO currency code.

Every payload carries a common envelope:

1. `event` — the hook name (string).
1. `occurred_at` — RFC 3339 UTC timestamp of dispatch.
1. `org_id` — the organization UUID.

### `time_entry_created`

Fired after a manual entry is created or a timer is started and its row is written.

```json
{
  "event": "time_entry_created",
  "occurred_at": "2026-07-10T14:03:21Z",
  "org_id": "018f9c2e-0000-7000-8000-000000000001",
  "time_entry": {
    "id": "018f9c2e-1111-7000-8000-000000000002",
    "user_id": "018f9c2e-2222-7000-8000-000000000003",
    "project_id": "018f9c2e-3333-7000-8000-000000000004",
    "task_id": "018f9c2e-4444-7000-8000-000000000005",
    "spent_date": "2026-07-10",
    "minutes": 0,
    "billable": true,
    "is_running": true,
    "notes": "Kickoff call",
    "started_at": "2026-07-10T14:03:21Z"
  }
}
```

### `time_entry_stopped`

Fired after a running timer is stopped and its final duration is recorded.

```json
{
  "event": "time_entry_stopped",
  "occurred_at": "2026-07-10T15:12:47Z",
  "org_id": "018f9c2e-0000-7000-8000-000000000001",
  "time_entry": {
    "id": "018f9c2e-1111-7000-8000-000000000002",
    "user_id": "018f9c2e-2222-7000-8000-000000000003",
    "project_id": "018f9c2e-3333-7000-8000-000000000004",
    "task_id": "018f9c2e-4444-7000-8000-000000000005",
    "spent_date": "2026-07-10",
    "minutes": 69,
    "billable": true,
    "is_running": false,
    "notes": "Kickoff call"
  }
}
```

### `invoice_created`

Fired after a draft invoice is generated from tracked time.

```json
{
  "event": "invoice_created",
  "occurred_at": "2026-07-10T16:00:00Z",
  "org_id": "018f9c2e-0000-7000-8000-000000000001",
  "invoice": {
    "id": "018f9c2e-5555-7000-8000-000000000006",
    "client_id": "018f9c2e-6666-7000-8000-000000000007",
    "invoice_number": "2026-0007",
    "status": "draft",
    "issue_date": "2026-07-10",
    "due_date": "2026-08-09",
    "currency": "EUR",
    "total_cents": 420000,
    "line_item_count": 12
  }
}
```

### `invoice_sent`

Fired after an invoice transitions to `sent`.

```json
{
  "event": "invoice_sent",
  "occurred_at": "2026-07-10T16:05:33Z",
  "org_id": "018f9c2e-0000-7000-8000-000000000001",
  "invoice": {
    "id": "018f9c2e-5555-7000-8000-000000000006",
    "client_id": "018f9c2e-6666-7000-8000-000000000007",
    "invoice_number": "2026-0007",
    "status": "sent",
    "issue_date": "2026-07-10",
    "due_date": "2026-08-09",
    "currency": "EUR",
    "total_cents": 420000
  }
}
```

### `user_logged_in`

Fired after a user successfully authenticates (for audit-log plugins).

```json
{
  "event": "user_logged_in",
  "occurred_at": "2026-07-10T09:01:12Z",
  "org_id": "018f9c2e-0000-7000-8000-000000000001",
  "user": {
    "id": "018f9c2e-2222-7000-8000-000000000003",
    "email": "casey@example.com",
    "name": "Casey Rivera",
    "org_role": "member",
    "method": "oidc"
  }
}
```

______________________________________________________________________

## Host functions

Horae exposes these host functions through Extism. SQL access requires a separate,
restricted PostgreSQL login configured through `HORAE_PLUGIN_DATABASE_URL`.
Without it, `horae_db_query` returns a configuration error; it never falls back to
the application's writer pool. Other plugin capabilities remain available.

1. `horae_log(level, message)` — structured logging. `level` is one of
   `"error" | "warn" | "info" | "debug"`; `message` is a string. Returns nothing.
   Entries are written to the host log annotated with the plugin name.
1. `horae_db_query(sql, params_json) -> rows_json` — **read-only** SQL lookup. `sql`
   is a query string; `params_json` is a JSON array of bind parameters; the result is
   a JSON array of row objects. PostgreSQL grants and a read-only transaction
   enforce the data boundary, including SELECTs that call functions.
1. `horae_http_post(url, body_json) -> response_json` — outbound HTTP POST for
   webhooks and integrations. `url` is the target; `body_json` is the request body;
   the return is a JSON object with the response status and body. Subject to the
   host's timeout and any operator-configured network policy.
1. `horae_config_get(key) -> value` — reads a value from **this plugin's own**
   configuration (keyed by the plugin `name` from the manifest). Returns the string
   value or null if unset. A plugin cannot read another plugin's or the host's config.

### Wire ABI

Each host function takes a single JSON-string argument and (except `horae_log`)
returns a single JSON string, consistent across all four:

- `horae_db_query` — in `{"sql": string, "params": [ ... ]}`; out a JSON array of row
  objects, or `{"error": string}`. A `SELECT`/`WITH`
  prefix guard rejects a leading write or a second `;`-separated statement, and the
  query is wrapped as a bounded row-to-JSON SELECT. The syntax check supplements,
  but does not replace, database permissions. Queries have a 5-second deadline
  and statement timeout, at most 1,000 rows, and at most 1 MiB of serialized JSON.
  Oversized results return an error, never silently truncated data. Rows are
  streamed rather than aggregated into an unbounded JSON array; oversized rows
  are rejected before transfer to the host. Every transaction is read-only and
  rolled back; its connection is closed even on cancellation to discard session
  settings and advisory locks. At most four database connections are admitted.
- `horae_http_post` — in `{"url": string, "body": <json>}`; out `{"status": u16, "body": string}`, or `{"error": string}`. Bounded by a 10-second timeout and a 1 MiB response body. Invalid UTF-8 and oversized responses are reported as errors.
- `horae_config_get` — in `{"key": string}`; out the JSON string value or JSON `null`.

Per-plugin configuration lives in an optional top-level `[config]` table in the
plugin's `plugin.toml` (string keys and values), read only by that plugin.

### Provisioning SQL access

Provision the login as a database administrator, after Horae's migrations. Grant
only the business data your installed plugins need; this example allows time
lookups without exposing authentication or import credentials:

```sql
CREATE ROLE horae_plugin LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE
  NOREPLICATION NOBYPASSRLS;
GRANT USAGE ON SCHEMA public TO horae_plugin;
GRANT SELECT ON public.time_entries, public.projects, public.clients,
  public.tasks, public.project_tasks TO horae_plugin;
GRANT SELECT (id, org_id, email, name, org_role, active)
  ON public.users TO horae_plugin;
ALTER ROLE horae_plugin SET default_transaction_read_only = on;
```

Set its password using `\password horae_plugin` in psql or provision equivalent
certificate/peer authentication. Put `HORAE_PLUGIN_DATABASE_URL` in the service's
secret environment file (NixOS: `services.horae.secretKeyFile`), using that login
and the Horae database. Do not put passwords in checked-in Nix expressions.

Startup and each query validate the effective permissions. The login must have
no elevated role attributes, role memberships, database/schema creation rights,
relation ownership, or data-write grants. Read grants are limited to the `public`
business tables: organizations, users, clients, projects, tasks, project_tasks,
assignments, time_entries, approvals, invoices, and invoice_line_items. The
`users.oidc_subject` column is excluded: do not grant table-wide SELECT on users.
Session tables, Harvest credentials, audit logs, import provenance, and other
non-business relations are not readable. Publicly updatable `pg_settings` is a
session-configuration exception, not an application-data write capability.

Executable SECURITY DEFINER functions are rejected. Outside PostgreSQL's system
schemas, only Horae's `harvest_norm(text)`, `line_amount_cents(bigint,integer)`, and
`set_updated_at()` functions are accepted. Additional functions/extensions may
require revoking their default PUBLIC execution grants and restoring grants for
their intended application roles. Likewise, older databases granting PUBLIC
creation on the public schema need those grants reviewed. These are shared
database permissions: review other consumers before changing them. Horae does
not create roles or revoke operator permissions automatically.

These controls limit data access and returned results, not every possible cost
of arbitrary SQL inside PostgreSQL. Use database resource controls or a separate
read replica when stronger CPU/memory isolation is required. Plugin HTTP access
also remains subject to the deployment's network policy.

Database security tests create and remove disposable login roles, so the test
administrator needs CREATEROLE as well as CREATEDB (CI uses an isolated superuser).

Host requests and serialized responses are limited to 1 MiB. Event payloads and
plugin return values have the same limit. An oversized host request/response
fails the invocation before copying it into another WASM/host buffer.

______________________________________________________________________

## Dashboard-widget return spec

A plugin may contribute a dashboard widget by returning a **structured JSON spec**
(FR-022). WASM plugins cannot render Dioxus components directly and **MUST NOT inject
arbitrary interface code** — they return content, and Horae renders it in designated
plugin slots on the dashboard (and settings) pages.

```json
{
  "widget": {
    "title": "Slack notifications",
    "body_format": "markdown",
    "body": "**3** invoices sent this week.\n\n- 2026-0007 → Acme\n- 2026-0008 → Globex"
  }
}
```

Schema:

1. `title` (string, required) — the widget heading.
1. `body_format` (string, required) — `"markdown"` or `"html"`.
1. `body` (string, required) — the widget content. Markdown is rendered by the host;
   HTML is **sanitized** by the host before rendering so no scripts, event handlers,
   or arbitrary UI code can execute. Returning any other structure, or omitting
   `title`/`body`, means no widget is rendered for that plugin.

______________________________________________________________________

## Sandbox & failure-isolation guarantees

These guarantees implement FR-020 and FR-021 and the spec's plugin edge cases.

1. **Sandboxed WASM (extism).** Each plugin runs as an isolated extism WASM instance
   with access limited to the four host functions above. No direct datastore writes,
   no filesystem, no ambient capabilities (FR-020). A malformed, unsupported, or
   malicious module is rejected at load time and never gains capabilities beyond those
   explicitly granted.
1. **Concurrent dispatch.** On each business event, `registry.dispatch(event)` schedules
   subscribed plugins on blocking workers, never on the async runtime's workers.
   Each instance executes one invocation at a time. The registry admits at most
   64 pending/running calls and runs at most 8 concurrently, shared by events and
   widgets. Delivery is best-effort: exhausted capacity or an oversized payload is
   logged and skipped, without delaying or undoing the core action.
1. **Timeouts and memory.** Waiting for an instance and worker is limited to 5 seconds;
   execution has a separate 5-second wait timeout. The engine additionally supplies
   100,000,000 fuel units during construction and replenishes that budget for each
   call, including guest initialization. This covers WASM start functions and
   WASI/Haskell initializers that can run before Extism starts its call timer.
   Fuel measures metered guest work, not elapsed time or native host instructions.
   A plugin that exhausts fuel cannot execute subsequent calls until the server
   restarts and loads a fresh instance; this avoids running a partially initialized
   guest. Ordinary successful calls receive a fresh budget rather than consuming
   one lifetime allowance.
   Each linear memory is limited to
   1,024 memory pages (64 MiB), including its initial allocation. The engine caps
   memories, instances, and tables at four each, allowing for Extism's kernel and
   auxiliary guest instances. On timeout, the host requests engine cancellation.
   Already-running synchronous host I/O cannot be preempted: its own timeout still
   applies, and its instance lock and capacity permits remain held until it exits.
   Dropping an async waiter likewise does not release a still-running call's
   capacity. These limits keep slow plugins off the async runtime's workers; they
   are not a guarantee that a synchronous host call ends at exactly 5 seconds.
   Compilation and local filesystem operations are not bounded by guest fuel;
   operators must trust the installed plugin files. These are not hard native
   process RSS/CPU limits.
1. **Failure isolation.** A plugin that errors, panics, times out, or attempts a
   disallowed action does **not** block, delay, or corrupt the core action that
   triggered the event. The core mutation has already been committed before dispatch;
   plugin outcomes cannot roll it back (FR-021). Failures are caught, isolated to the
   offending plugin, and logged with the plugin name.
1. **Datastore access.** `horae_db_query` uses only the validated restricted login,
   with read-only transactions, bounded results, and a fresh session per query.
   Missing or unsafe credentials never grant access to the application's writer.

______________________________________________________________________

## Installation & lifecycle

1. Plugins are dropped into the configured plugins directory, each in its own
   subdirectory with a `*.wasm` module and `plugin.toml`. The registry scans this
   directory at startup. Invalid plugins are logged and skipped; a failure of the
   loading worker itself is returned to startup rather than silently ignored.
1. A future admin UI page will list loaded plugins and allow enable/disable without a
   restart (hot-reload via `extism::Plugin` re-instantiation).
1. Hook call sites live in the server functions: dispatch `time_entry_created` /
   `time_entry_stopped` after time-entry writes, `invoice_created` / `invoice_sent`
   after invoice mutations, and `user_logged_in` after authentication.
