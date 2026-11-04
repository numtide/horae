# Applying the rules to Horae

The places in `crates/horae/src` that break a rule in `SKILL.md`, ranked by impact.
They come from a component audit of master at `b2e7c832` (2026-11-04). Each entry was
re-read in the code. Symbols are named rather than line numbers, so check that a
symbol still exists before acting on it. Fix one entry per PR, and measure before
claiming a gain (see "Before claiming a performance fix" in `SKILL.md`).

## Usage today

`DESIGN.md` makes `use_signal` and `use_resource` the defaults, and `pages.rs`
`run_action` already covers mutations (error signal, `on_ok`, resource restart). The
zero counts below are not violations. They mark options to try only after measuring a
problem they fix.

| API | Uses | Note |
|---|---|---|
| `use_resource` | 58 | every page loads through it, so nothing is in the SSR HTML |
| `use_loader`, `use_server_future`, `SuspenseBoundary`, `ErrorBoundary` | 0 | hand-written loading/error blocks stand in for them |
| `use_action` | 0 | `run_action` and `spawn(async …)` blocks in handlers |
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
  Fix: rows that take their value by prop plus a `Memo<RateContext>`. Reach for a
  Store (section 4) only if that measures short.

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
  run. The gate is only half of it: `ProjectListContent` is keyed on `generation`,
  which the resource sets on **every** successful load, so the content remounts
  anyway. The key is deliberate (#270): it drops state built under an earlier
  requester or permission policy.
  Fix: gate on `projects.read().is_none()`, keep the content mounted with
  `aria-busy`, **and** bump `generation` only when `binding` changes, not on every
  load.
- **Unmount on restart: `pages/clients/detail.rs` `ClientDetailContent`.**
  `ClientProjects` and `ClientInvoices` render only when `ready`, so each
  `details.restart()` remounts them and refetches their data. `ClientProjects` also
  loads `list_project_spend(None)`, the spend for the whole organisation.
- **Stale value under a new key: `pages/approvals.rs` `Approvals`.**
  - The closure already reads its keys (status, refresh, week), but the response is
    tagged only with the week `start`, so after a filter change the previous filter's
    rows stay visible, with live Approve/Reopen buttons.
  - The rows are not keyed.

  Fix: compare the tag against the full key, treat Pending as busy, and key the rows.
- **Waterfalls:**
  - `pages/projects.rs` `ClientProjectList` awaits `get_client_details` before
    `ProjectList` mounts.
  - `pages/invoices/preparation.rs` awaits `list_projects` before `prepare_invoice`.
  - `components/timer_widget.rs` mounts `ProjectTaskPicker` (which owns
    `list_time_entry_contexts`) only after the catalogs resolve.

  Fix: separate resources, or `futures_util::join!`. `pages/new_project.rs`
  `NewProject` already joins its main loads; only its optional client and selection
  lookups run in sequence, which is minor.
- **SSR has no data.** No page suspends, so first paint is "Loading…" everywhere, and
  every request starts after hydration.
  Pilot `use_loader(..)?` on one read-only page (for example `PermissionAudit` or
  `ClientDetail`) inside a local `SuspenseBoundary`. Verify all of these before
  rolling it out:
  - the server fn's `FullstackContext::extract::<Session, _>()` (`server_fns.rs`
    `session_user_id`, which `require_user` calls) sees the page request's session
    during SSR;
  - a **failed** reload raises to the `ErrorBoundary` and unmounts the section, so
    the page needs a boundary whose handler offers a retry (`clear_errors()` or a new
    `key`), and must not hold state the user would lose;
  - the hydrated client makes **no** duplicate request (hash the request body as in
    the Playwright fetch hook);
  - `T` is `Serialize + DeserializeOwned + PartialEq`.

  Don't use `use_server_future` for data that reloads (section "Resources and loading"
  in `SKILL.md`).
- **Mutations.** Keep `pages.rs` `run_action` for them. `use_action` cancels only the
  client task on a second call, so a non-idempotent server fn can commit with no
  result shown, and its `pending()` is false until the task first polls. Consider it
  only for idempotent or query-like actions.

## 4. Lists and Stores

The project editor is the candidate if measurement shows typing lag.
`ProjectForm.tasks`, `.team` and `.milestones` can each hold up to 500 rows. A Store
following `todomvc_store.rs` (`#[derive(Store)]`, a `use_memo` of the ids,
`Row { key: "{id}", form, id }` reading `form.tasks().get(..)`) makes a keystroke
re-render one row. Adding or removing a row still re-renders all of them, with a map or
a `Vec`. Try the cheaper fix in section 1 first: rows that take their value by prop.

## 5. Events and links

- **`pages/settings/own_permissions.rs` `permission_content`.** `a { href:
  "/admin/audit" }` reloads the whole app.
  Fix: `Link { to: Route::PermissionAudit {} }`.
- **`components/sidebar.rs` `SideLink`, `components/admin_shell.rs` `AdmLink`.** They
  look active by route variant, but `Link` sets `aria-current` only on an exact href
  match.
  Fix: pass `aria_current: active.then_some("page")`. `Link` still emits its own,
  so on an exact match the SSR HTML carries `aria-current` twice. Check the page in
  a browser, or render an `a` with `onclick` + `navigator().push` instead.
- **Checked and correct:** all 8 `prevent_default()` sites run before any `.await`.
