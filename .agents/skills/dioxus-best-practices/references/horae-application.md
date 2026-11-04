# Applying the rules to Horae

The places in `crates/horae/src` that break a rule in `SKILL.md`, ranked by impact.
They come from a component audit of master at `b2e7c832` (2026-11-04). Each entry was
re-read in the code. Symbols are named rather than line numbers, so check that a
symbol still exists before acting on it. Fix one entry per PR, and measure before
claiming a gain (see "Before claiming a performance fix" in `SKILL.md`).

## Usage today

| API | Uses | Note |
|---|---|---|
| `use_resource` | 58 | every page loads through it, so nothing is in the SSR HTML |
| `use_loader`, `use_server_future`, `SuspenseBoundary`, `ErrorBoundary` | 0 | about 40 hand-written loading/error blocks stand in for them |
| `use_action` | 0 | about 48 `spawn(async …)` blocks in handlers, each with its own busy flag |
| `use_store` / `#[derive(Store)]` | 0 | the editable lists use `Signal<Vec<_>>` |
| `use_callback` | 33 | it is the pattern, but inline closure props remain (see below) |

## 1. Subscriptions in the wrong scope

- **`pages/timesheet.rs` `render_calendar_view`.** It is a plain `fn` called from
  `TimesheetContent`, and it reads `add_hint` and `cal_drag`. Every mouse move or drag
  step therefore re-renders the whole page, including both `Modal`s, which re-render
  anyway because they take `children`.
  Fix: make it `#[component] CalendarView` with `add_hint` inside it, and
  `use_memo(move || cal_drag.read().is_some())` for the page's `dirty`.
- **`pages/importers.rs` `HarvestImport` poll loop.** It calls
  `job_progress.set(Some(job))` every second, unconditionally, and the history rows
  read `job_progress`.
  Fix: `if job_progress.peek().as_ref() != Some(&job) { job_progress.set(Some(job)) }`,
  and move the history into a child that takes `selected: Option<Uuid>`.
- **`pages/new_project/{tasks,team,billing}.rs` `TaskRow`, `MemberRow`, `Milestone`.**
  Each row does `form.read().tasks.iter().find(..)` plus several `form.read().…`, so
  every keystroke anywhere in the form re-renders every row, with an O(n) `find` each.
  Fix: a Store (section 4), or rows that take their value by prop plus a
  `Memo<RateContext>`.

## 2. Unstable props

- **Inline closures passed to memoized children:**
  - `pages/reports/scoped.rs` `ScopedReports` passes `on_detail` to
    `GroupedTimeReport`;
  - `pages/clients.rs` `ClientList` passes `on_dismiss`/`on_saved` to `ClientEditor`
    and `on_archived` to `ArchiveDialog`;
  - `pages/reports.rs` `LegacyReports` passes `onselect` to `FilterSelect`.

  Fix: `use_callback` for each.
- **`EventHandler::new` in a component body:** `pages/invoices/preparation.rs`
  `PrepareInvoice` (`generate`) and `pages/invoices/editing.rs` `DraftEditForm`
  (`save`). Both re-render on every keystroke and keep a new box each time.
  Fix: `use_callback`.
- **Row components that take `children`.** `pages/projects.rs` `ProjectListContent`
  renders a `Menu` with `MenuItem`s inline in each row, so every click or selection
  rebuilds every row.
  Fix: `ProjectRow` with plain props, so its `Menu` only re-renders when that row's
  props change.

## 3. Resources and loading

- **Unmount on restart: `pages/projects.rs` `ProjectList`.**
  `if projects.state()() != UseResourceState::Ready { return … }` unmounts
  `ProjectListContent`, and with it the search, scope, client, tag and selection
  signals, after every row or bulk action that calls `projects.restart()`. The
  `projects_loading` and `Some(Err(_))` branches inside `ProjectListContent` can never
  run.
  Fix: gate on `projects.read().is_none()` and keep the content mounted with
  `aria-busy`.
- **Unmount on restart: `pages/clients/detail.rs` `ClientDetailContent`.**
  `ClientProjects` and `ClientInvoices` render only when `ready`, so each
  `details.restart()` remounts them and refetches their data. `ClientProjects` also
  loads `list_project_spend(None)`, the spend for the whole organisation.
- **Stale value under a new key: `pages/approvals.rs` `Approvals`.**
  - The response is tagged only with the week `start`, so after a filter change the
    previous filter's rows stay visible, with live Approve/Reopen buttons.
  - The rows are not keyed.

  Fix: read every key reactively inside the closure (the old task is then cancelled)
  and compare the tag against the full key. Treat Pending as busy.
- **Waterfalls:**
  - `pages/projects.rs` `ClientProjectList` awaits `get_client_details` before
    `ProjectList` mounts.
  - `pages/new_project.rs` `NewProject` awaits `project_creation_client`, then
    `project_creation_selection`.
  - `pages/invoices/preparation.rs` awaits `list_projects` before `prepare_invoice`.
  - `components/timer_widget.rs` mounts `ProjectTaskPicker` (which owns
    `list_time_entry_contexts`) only after the catalogs resolve.

  Fix: separate resources, or `futures_util::join!`.
- **SSR has no data.** No page suspends, so first paint is "Loading…" everywhere, and
  every request starts after hydration.
  Pilot `use_loader(..)?` on one read-only page (for example `PermissionAudit` or
  `ClientDetail`) inside a local `SuspenseBoundary`. Verify all of these before
  rolling it out:
  - the server fn's `FullstackContext::extract::<Session, _>()` (`server_fns.rs`
    `require_user`) sees the page request's session during SSR;
  - the hydrated client makes **no** duplicate request (hash the request body as in
    the Playwright fetch hook);
  - `T` is `Serialize + DeserializeOwned + PartialEq`.

  Don't use `use_server_future` for data that reloads (section "Resources and loading"
  in `SKILL.md`).
- **Mutations.** `pages.rs` `run_action`, `pages/approvals.rs` `spawn_action` and the
  timesheet handlers re-implement pending/error tracking with `spawn`.
  Prefer `use_action` for new code: `pending()` replaces the busy flag, and a second
  call cancels the first. Migrate existing code only alongside other work in that
  file.

## 4. Lists and Stores

The project editor is the first candidate. `ProjectForm.tasks`, `.team` and
`.milestones` can each hold up to 500 rows. Follow `todomvc_store.rs`:

- `#[derive(Store)]` on the form and on the row types;
- rows stored by UUID, or kept as a `Vec` if order matters and rows are only
  appended (removing from a `Vec` store dirties every row after the removed index);
- a `use_memo` of the ids;
- `Row { key: "{id}", form, id }`, reading `form.tasks().get(..)`.

## 5. Events and links

- **`pages/settings/own_permissions.rs` `permission_content`.** `a { href:
  "/admin/audit" }` reloads the whole app.
  Fix: `Link { to: Route::PermissionAudit {} }`.
- **`components/sidebar.rs` `SideLink`, `components/admin_shell.rs` `AdmLink`.** They
  look active by route variant, but `Link` sets `aria-current` only on an exact href
  match.
  Fix: pass `aria_current: active.then_some("page")`.
- **Checked and correct:** all 8 `prevent_default()` sites run before any `.await`.
