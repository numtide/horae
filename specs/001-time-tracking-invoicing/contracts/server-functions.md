# Server Functions Contract

This document is the interface contract for Horae's **internal Dioxus `#[server]`
functions** — the primary UI↔server API. The single-page app (SPA) calls these
for all data reads and mutations; Dioxus registers each one as an HTTP endpoint on
the Axum router automatically.

This is a **contract, not an implementation** — no function bodies are given.

## Conventions

1. **Transport & auth**: Every function is session-authenticated. The caller's
   identity is taken from the Postgres-backed cookie session (`tower_sessions`);
   IDs are never trusted from the client for the "acting user". Functions that
   read the session but find no user return `401`.
1. **Error type**: All functions return `Result<T, ServerFnError>`. Failures are
   modeled as `ServerFnError::ServerError { message, code, details }`, where
   `code` mirrors an HTTP status: `401` unauthenticated, `403` forbidden (role or
   assignment), `404` not found, `409` conflict (locked entry, timer already
   running), `500` internal/database error.
1. **ID encoding**: UUIDs cross the wire as `String` and are parsed server-side to
   `Uuid` (invalid strings yield a `500` "Invalid …" error). Dates cross as
   `String` in `YYYY-MM-DD` form and parse to `chrono::NaiveDate`. The "typed
   conceptually" column below shows the logical type.
1. **Roles**: The role hierarchy is `member` < `manager` < `admin` (stored as the
   `org_role` text `"member" | "manager" | "admin"`). "member" means any
   authenticated user. `require_manager()` accepts `manager` or `admin`;
   `require_admin()` accepts only `admin`.
1. **Units**: Durations are integer minutes; money is integer minor units (cents) +
   ISO currency code — never floats (per the domain invariants).
1. **(planned)** marks functions required by the spec (FR-001..FR-023) that are
   **not yet defined** in `crates/horae/src/server_fns.rs`.

______________________________________________________________________

## Auth

| Function | Inputs | Output | Errors | Required role |
|---|---|---|---|---|
| `login` | `email: String`, `password: String` | `()` | `ServerFnError` — always `401`; this is a stub. Real login is the Axum route `POST /auth/dev-login` (dev) / OIDC (prod). | member (public) |
| `logout` | — | `()` | `ServerFnError` (`500` on session error) | member |
| `get_me` | — | `User` | `ServerFnError` (`401` no session, `404` user not found/inactive) | member |

Notes:

1. Interactive sign-in does **not** go through `login`; it is served by plain Axum
   routes (`GET /auth/login`, `POST /auth/dev-login`, `POST /auth/logout`) outside
   the Dioxus `#[server]` surface. Production uses OIDC; `DEV_LOGIN=1` enables a
   one-click admin login.
1. **Dev login** is the Axum `POST /auth/dev-login` handler, not a `#[server]`
   function. It is listed here because it is part of the UI↔server auth contract,
   but it satisfies FR-001 via the Axum surface rather than a server function.

______________________________________________________________________

## Time entries

| Function | Inputs | Output | Errors | Required role |
|---|---|---|---|---|
| `list_time_entries` | `user_id: Option<Uuid>` (reserved; results scoped to session user), `project_id: Option<Uuid>`, `date_from: Option<Date>`, `date_to: Option<Date>`, `limit: Option<i64>` (default 50) | `Vec<TimeEntry>` | `ServerFnError` (`401`, `500`, invalid filter) | member (own entries) |
| `create_time_entry` | `project_id: Uuid`, `task_id: Uuid`, `spent_date: Date`, `minutes: i32`, `notes: Option<String>`, `billable: bool` | `TimeEntry` | `ServerFnError` (`401`, `409` unavailable context, `500`) | member (must be assigned; admins bypass only assignment) |
| `list_time_entry_contexts` | — | `Vec<TimeEntryContext>` (`project_id`, `task_id`, effective `billable`; no rates) | `ServerFnError` (`401`, `500`) | member (own eligible contexts) |
| `update_time_entry` | `entry_id: Uuid`, `minutes: i32`, `notes: Option<String>`, `billable: bool` | `TimeEntry` | `ServerFnError` (`409` entry not found or not in `open` state) | member (own, open entries) |
| `delete_time_entry` | `entry_id: Uuid` | `()` | `ServerFnError` (`409` entry not found or not in `open` state) | member (own, open entries) |
| `start_timer` | `project_id: Uuid`, `task_id: Uuid`, `notes: Option<String>` | `TimeEntry` | `ServerFnError` (`409` unavailable context or a timer already running) | member |
| `stop_timer` | `entry_id: Uuid` | `TimeEntry` | `ServerFnError` (`404` no running timer for this entry) | member (own) |
| `get_current_timer` | — | `Option<TimeEntry>` | `ServerFnError` (`401`, `500`) | member |

Notes:

1. **FR-004** (one running timer per user) is enforced both by `start_timer`
   (returns `409`) and by a DB partial unique index.
1. New entries and all three project/task pickers use the `time_entry_contexts` view. The user, client, project, and task must be active and in the same organization, the task must be enabled on the project, and non-admin users (including managers) must be assigned. The insert resolves eligibility and billability in one SQL statement, without a separated pre-check. Admins cannot bypass activity, organization, or task enablement.
1. Effective billability is the requested entry flag AND the project's billing type is not `non_billable` AND the project-task override allows billing. Timers request billable time but obey both restrictions. Editing existing open time remains possible after archiving, but cannot override non-billability. An old entry without a project-task link falls back to the task's catalog default when edited or reported; new entries require a link.
1. Unbilled invoice selection, reports, project spend, and Harvest reads apply these billing restrictions. Already-invoiced entries retain their recorded billability; changing a project does not rewrite historical invoices. This does not change the separate rate-resolution policy.
1. **FR-015 / edit-lock**: `update_time_entry` and `delete_time_entry` succeed only
   while the entry is in the `open` state; once an entry is `submitted`, `approved`,
   or attached to an invoice it is locked (returns `409`).
1. `update_time_entry` locks the owned, open entry before comparing normalized
   values and effective billability. An unchanged edit returns the current entry
   without rewriting it, changing `updated_at`, dispatching an update event, or
   scheduling a budget check. Concurrent edits compare against the preceding
   committed edit; a concurrent lock or deletion still returns `409`.
1. `stop_timer` computes elapsed minutes exactly from `started_at` (minimum 1
   minute), satisfying FR-003 / FR-023.

______________________________________________________________________

## Clients

| Function | Inputs | Output | Errors | Required role |
|---|---|---|---|---|
| `list_clients` | `include_inactive: bool` | `Vec<Client>` | `ServerFnError` (`500`) | member |
| `create_client` | `name: String`, `currency: String`, `address: Option<String>`, `tax_id: Option<String>` | `Client` | `ServerFnError` (`403` non-manager) | manager |
| `update_client` | `client_id: Uuid`, `name: String`, `currency: String`, `address: Option<String>`, `tax_id: Option<String>` | `Client` | `ServerFnError` (`403`, `404`) | manager |
| `set_client_active` | `client_id: Uuid`, `active: bool` | `Client` | `ServerFnError` (`403`, `404`) | manager |

Notes:

1. `list_clients` returns only `active = true` rows by default so inactive clients
   are not selectable for new work (FR-011); the management view passes
   `include_inactive = true` to also list deactivated clients for reactivation.
1. Per FR-008 client create/edit/deactivate are gated at **manager** (managers or
   admins).
1. Client edits and activation changes lock the organization's client before
   comparing values. An unchanged request returns the current client without
   rewriting the row or emitting an event. Concurrent requests report only the
   changes they commit; missing or foreign clients return `404`, including when
   the requested values would otherwise be unchanged. Editing details preserves
   the active flag, and activation changes preserve the client's details.

______________________________________________________________________

## Projects

| Function | Inputs | Output | Errors | Required role |
|---|---|---|---|---|
| `list_projects` | `client_id: Option<Uuid>` (reserved; not yet filtered), `include_inactive: bool` | `Vec<Project>` | `ServerFnError` (`500`) | member |
| `create_project` | `client_id: Uuid`, `name: String`, `project_type: String`, `currency: String`, `budget_kind: String`, `budget_value: String`, `rate_value: String` | `Project` | `ServerFnError` (`403` non-manager, `409` invalid rate) | manager |
| `update_project` | `project_id: Uuid`, `name: String`, `project_type: String`, `currency: String`, `budget_kind: String`, `budget_value: String`, `rate_value: String` | `Project` | `ServerFnError` (`403`, `404`, `409` invalid rate) | manager |
| `set_project_active` | `project_id: Uuid`, `active: bool` | `Project` | `ServerFnError` (`403`, `404`) | manager |

Notes:

1. `project_type` and `budget_kind` are Postgres enums bound as text; the billing
   method / budget rate fields of FR-009 map onto these plus `budget_amount_cents`
   / `budget_minutes` on the row. `rate_value` is an exact hourly amount with at
   most two decimal places; blank clears the optional `rate_cents`, and zero is
   an explicit free rate. Negative, malformed, and overflowing rates are rejected.
1. Billing resolves task → assignment → project → user default (FR-024), with
   zero when every level is unset. Current rates apply to un-invoiced time,
   including time logged before a rate change. Generating an invoice freezes
   its line rates and amounts; grouped reports, project spend, and Harvest entry
   rates use the attached invoice line while it remains attached. An old void
   invoice's lines cannot override the rate of a replacement invoice.
1. Per FR-009 project create/edit/deactivate are gated at **manager**. The
   management view passes `include_inactive = true` to `list_projects` to include
   inactive projects for reactivation. The `client_id` filter argument on
   `list_projects` is accepted but not yet applied.

______________________________________________________________________

## Tasks

| Function | Inputs | Output | Errors | Required role |
|---|---|---|---|---|
| `list_tasks` | — | `Vec<Task>` (active, session organization only) | `ServerFnError` (`500`) | member |
| `list_project_tasks` | `project_id: Uuid` | `Vec<Task>` (linked via `project_tasks`) | `ServerFnError` (`401`, `500`) | member |
| `create_task` | `name: String`, `billable_default: bool`, `project_id: Option<Uuid>` | `Task` | `ServerFnError` (`403` non-manager, `404` unavailable project, `409` empty name) | manager |
| `update_task` | `task_id: Uuid`, `name: String`, `billable_default: bool`, `default_rate_cents: Option<i64>` | `Task` | `ServerFnError` (`403`, `404`) | manager |
| `set_task_active` | `task_id: Uuid`, `active: bool` | `Task` | `ServerFnError` (`403`, `404`) | manager |
| `link_project_task` | `project_id: Uuid`, `task_id: Uuid` | `()` | `ServerFnError` (`403`, `404`) | manager |

Notes:

1. Tasks are **org-level** in the current schema; the per-project relationship is
   the `project_tasks` join table surfaced by `list_project_tasks`.
1. Per FR-010 task create/edit/deactivate and project linking are gated at
   **manager**. `link_project_task` inherits the task's `billable_default` /
   `default_rate_cents` onto the new `project_tasks` row and is idempotent
   (`ON CONFLICT DO NOTHING`).
1. Passing `project_id` to `create_task` creates and enables the task in one transaction. A linking failure rolls back the task, and the creation event is emitted only after commit. The project detail page offers both this action and enabling an existing catalog task. Linking checks the active client/project/task and organization even when a link already exists; repeating it preserves existing billability and rate overrides.

______________________________________________________________________

## Assignments

Supporting surface for FR-005/FR-006 (a user may only log time on projects they are
assigned to). Not called out in the prompt's grouping but part of the real contract.

| Function | Inputs | Output | Errors | Required role |
|---|---|---|---|---|
| `list_assignments` | `project_id: Uuid` | `Vec<Assignment>` | `ServerFnError` (`401`, `500`) | member |
| `create_assignment` | `project_id: Uuid`, `user_id: Uuid`, `role: String` | `Assignment` | `ServerFnError` (`403` non-admin) | admin |
| `delete_assignment` | `assignment_id: Uuid` | `()` | `ServerFnError` (`403` non-admin) | admin |

______________________________________________________________________

## Invoices

| Function | Inputs | Output | Errors | Required role |
|---|---|---|---|---|
| `list_invoices` | `status: Option<String>` | `Vec<Invoice>` | `ServerFnError` | manager (planned; currently unauthenticated stub) |
| `get_invoice` **(planned)** | `invoice_id: Uuid` | `Invoice` (with line items) | `ServerFnError` (`403`, `404`) | manager |
| `generate_invoice_from_time` **(planned)** | `client_id: Uuid`, `period_from: Date`, `period_to: Date` | `Invoice` | `ServerFnError` (`403`, `404` nothing to invoice, `409` time already billed) | manager |
| `update_invoice_status` **(planned)** | `invoice_id: Uuid`, `status: "draft" \| "sent" \| "paid" \| "void"` | `Invoice` | `ServerFnError` (`403`, `404`, `409` illegal transition) | manager |

Notes:

1. **Invoicing is Phase 4 and largely (planned)**. The current `list_invoices` is a
   stub: it takes `status`, ignores it, does **not** check the session, and always
   returns an empty `Vec` because the `invoices` table does not exist yet.
1. **(planned)** functions above implement FR-012..FR-015: generate a draft invoice
   from a client's billable, un-invoiced time so totals reconcile exactly (FR-012,
   FR-023); mark covered entries invoiced so they cannot be double-billed (FR-013);
   carry number/issue date/due date/total across the draft→sent→paid / void
   lifecycle (FR-014); and lock invoiced time against edit/delete (FR-015).

______________________________________________________________________

## Reports

| Function | Inputs | Output | Errors | Required role |
|---|---|---|---|---|
| `report_time` | `from: Date`, `to: Date`, `group_by: "project" \| "task" \| "client" \| "person"`, optional `client_id`, `project_id`, `user_id` | `Vec<ReportRow>` (`group_id`, `label`, `currency`, `total_minutes`, `rounded_minutes`, `billable_minutes`, `billable_cents`, `cost_cents`) | `ServerFnError` (`401`, `403`, `500`, invalid date/filter) | manager |
| `report_detailed` | `from: Date`, `to: Date`, optional `client_id`, `project_id`, `user_id` | `Vec<DetailedReportRow>` | `ServerFnError` (`401`, `403`, `500`, invalid date/filter) | manager |

`report_time` groups by the selected entity's UUID and the client's currency,
not by its display name. Each row has a non-null currency; one person or task
working across currencies appears in separate rows. Names remain labels, so
distinct entities with the same name are not merged. Unknown dimensions retain
the project fallback. Ordering is bytewise label, UUID, then bytewise currency.
The UI keys rows by UUID and currency and does not sum monetary totals across
currencies. A same-currency total exceeding the supported integer range is
reported explicitly. This does not change currency authority or convert money.

Export links (not `#[server]` functions):

1. Report/invoice **export** is served by plain Axum routes (CSV / XLSX via
   `reports.rs`), not by server functions, so the browser can download a file
   directly. Exported totals reconcile exactly with the on-screen figures
   (FR-016, FR-023, SC-007).
1. Both report functions require manager access and scope every query to the
   manager's organization. The `"client"` dimension uses the actual client ID
   and name.

______________________________________________________________________

## Approvals

Weekly submit/approve workflow (milestone M7). Not a spec FR (the spec defers a
formal approval step), but part of the real contract and the entry-locking model.

| Function | Inputs | Output | Errors | Required role |
|---|---|---|---|---|
| `get_week_start` | — | `Weekday` | `ServerFnError` (`500` invalid stored configuration) | member (own organization) |
| `submit_week` | `week_start: Date` | `Approval` | `ServerFnError` (`400` wrong first weekday or out-of-range week, `404` no open entries, `409` running timer or already-approved week) | member (own week) |
| `list_approvals` | `status: Option<String>` | `Vec<Approval>` | `ServerFnError` (`403` non-manager) | manager |
| `approve_submission` | `approval_id: Uuid` | `Approval` | `ServerFnError` (`403`, `404` not in `submitted` state) | manager |
| `reject_submission` | `approval_id: Uuid` | `()` | `ServerFnError` (`403`, `404`) | manager |

Submission checks the organization's configured first weekday and running timers,
freezes each open entry's current rounded minutes, writes the approval and reads
the event total in one transaction. The event is dispatched only after commit.

Interactive entry writes and imported inserts take a shared transaction advisory
lock for the user before writing entries; submission takes the same lock
exclusively before checking the period. This deliberately covers every week for
that user, so cross-week moves need no racy source-date lookup or two-period lock
ordering. Other users remain independent and ordinary writers share the lock.
An import holds its successful row locks until the outer transaction ends, so a
long import can delay that user's submission. Direct SQL and maintenance seed
operations do not participate in this application-level barrier.

The timesheet and date picker use the configured first weekday. The five-day
calendar still shows Monday–Friday, in date order within that week. Invalid
configuration is displayed as an error, not silently replaced with Monday.

The existing policy on new open time in an already-approved week is unchanged:
it can still be added, but the week cannot be resubmitted until a manager reopens
it. Deciding whether to reject such additions or automatically reopen the week
is a separate policy change.

______________________________________________________________________

## Admin / Users

| Function | Inputs | Output | Errors | Required role |
|---|---|---|---|---|
| `list_users` | — | `Vec<User>` (active only) | `ServerFnError` (`500`) | member (planned: admin) |
| `create_user` **(planned)** | `email: String`, `name: String`, `role: "member" \| "manager" \| "admin"` | `User` | `ServerFnError` (`403`, `409` email exists) | admin |
| `set_user_role` **(planned)** | `user_id: Uuid`, `role: "member" \| "manager" \| "admin"` | `User` | `ServerFnError` (`403`, `404`) | admin |
| `set_user_active` **(planned)** | `user_id: Uuid`, `active: bool` | `User` | `ServerFnError` (`403`, `404`) | admin |

Notes:

1. **(planned)**: FR-002 requires admins to create users, assign roles, and
   deactivate accounts (deactivated users cannot sign in). The current code exposes
   only `list_users`, which returns active users and does **not** yet require the
   admin role. User creation today happens via the CLI (`user create`) rather than a
   server function. Deactivation must preserve historical entries (edge case in the
   spec).
