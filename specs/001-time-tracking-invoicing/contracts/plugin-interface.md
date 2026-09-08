# Plugin Interface Contract

**Status: Planned (User Story 5).** The plugin subsystem is **not yet implemented**
in the codebase. This document is a forward-looking interface contract derived from
PLAN.md's "Plugin System" section and functional requirements **FR-018..FR-022**. It
defines the plugin manifest, the event catalog, the host functions, the
dashboard-widget return shape, and the sandbox / failure-isolation guarantees so that
implementation and plugin authors share one contract.

## Overview

Plugins are **WASM modules loaded at runtime via
[extism](https://github.com/extism/extism)**. Any language that compiles to WASM
(Rust, Go, TypeScript, C, Zig, …) can author a Horae plugin. Plugins are
operator-trusted but **sandboxed**: they may only call the host functions Horae
explicitly exposes and can never write to the datastore or render arbitrary UI code.

Planned module layout (`crates/horae/src/plugin/`):

1. `registry.rs` — `PluginRegistry`: scans the `plugins/` data directory, loads each
   `*.wasm` at startup, holds a handle per plugin.
1. `host.rs` — extism host functions (logging, read-only DB queries, HTTP POST,
   config lookup).
1. `event.rs` — the `AppEvent` enum, serialized to JSON and passed to plugins.
1. `manifest.rs` — the `plugin.toml` schema.

At startup the registry loads every plugin and registers it for the hooks it
declares (FR-018). `AppState` gains `plugins: Arc<PluginRegistry>`; on each business
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

Horae exposes these host functions through Extism. Only install trusted plugins:
the current SQL host function uses the application database pool, and its syntax
guard does not isolate the privileges of SQL functions. Separate database
credentials and grants are still required to meet FR-020's read-only guarantee.

1. `horae_log(level, message)` — structured logging. `level` is one of
   `"error" | "warn" | "info" | "debug"`; `message` is a string. Returns nothing.
   Entries are written to the host log annotated with the plugin name.
1. `horae_db_query(sql, params_json) -> rows_json` — **read-only** SQL lookup. `sql`
   is a query string; `params_json` is a JSON array of bind parameters; the result is
   a JSON array of row objects. Direct write statements are rejected, but this is
   not yet a database privilege boundary (see the warning above).
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
  query is executed wrapped as `SELECT json_agg(_t) FROM (<sql>) _t`, a subquery form
  Postgres accepts only for a `SELECT`. This is a syntax check, not a privilege
  boundary. Queries have a 5-second deadline and transaction-local statement
  timeout; the transaction is rolled back even on success. Postgres performs
  row→JSON serialization; the response limit is checked after that aggregation.
- `horae_http_post` — in `{"url": string, "body": <json>}`; out `{"status": u16, "body": string}`, or `{"error": string}`. Bounded by a 10-second timeout and a 1 MiB response body. Invalid UTF-8 and oversized responses are reported as errors.
- `horae_config_get` — in `{"key": string}`; out the JSON string value or JSON `null`.

Per-plugin configuration lives in an optional top-level `[config]` table in the
plugin's `plugin.toml` (string keys and values), read only by that plugin.

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
   execution has a separate 5-second timeout. Each linear memory is limited to
   1,024 memory pages (64 MiB), including its initial allocation. The engine caps
   memories, instances, and tables at four each, allowing for Extism's kernel and
   auxiliary guest instances. On timeout, the host requests engine cancellation.
   Already-running synchronous host I/O cannot be preempted: its own timeout still
   applies, and its instance lock and capacity permits remain held until it exits.
   Dropping an async waiter likewise does not release a still-running call's
   capacity. These limits keep slow plugins off the async runtime's workers; they
   are not a guarantee that a synchronous host call ends at exactly 5 seconds.
1. **Failure isolation.** A plugin that errors, panics, times out, or attempts a
   disallowed action does **not** block, delay, or corrupt the core action that
   triggered the event. The core mutation has already been committed before dispatch;
   plugin outcomes cannot roll it back (FR-021). Failures are caught, isolated to the
   offending plugin, and logged with the plugin name.
1. **Datastore access.** `horae_db_query` accepts SELECT-shaped statements and rolls
   back its transaction, but still uses the application role. This does not yet
   satisfy FR-020 against untrusted SQL functions or unrestricted data reads.

______________________________________________________________________

## Installation & lifecycle (planned)

1. Plugins are dropped into `{dataDir}/plugins/`, each with its `*.wasm` module and a
   `plugin.toml`. The registry scans this directory at startup.
1. A future admin UI page will list loaded plugins and allow enable/disable without a
   restart (hot-reload via `extism::Plugin` re-instantiation).
1. Hook call sites live in the server functions: dispatch `time_entry_created` /
   `time_entry_stopped` after time-entry writes, `invoice_created` / `invoice_sent`
   after invoice mutations, and `user_logged_in` after authentication.
