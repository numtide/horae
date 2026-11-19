# Design implementation inventory

Updated on 2026-11-19 from the [design audit of 2026-11-18](../docs/design/design-audit.md)
(application baseline `3b08dd25`) and the decisions taken on it. [DESIGN.md](../DESIGN.md) holds those
decisions and is the source of truth for every rule below; this file only tracks
what the application still has to change to follow it, and which PR does it.

The audit compared the 26 application screens, at 360, 768 and 1280px in both
themes, with `DESIGN.md`, the kit and the prototypes. Its findings are grouped by
pattern, and each pattern is fixed across every screen in one PR, numbered PR-0
to PR-10. This file is PR-0.

The audit cites 156 screenshots by name (`audit/screenshots/<screen>-<width>-<theme>.png`,
about 10 MB). Because of their size they are not in the repository; each PR takes
its own before-and-after captures.

## Pending work by PR

| PR | Work | `DESIGN.md` section | Status |
| --- | --- | --- | --- |
| PR-0 | Bring `DESIGN.md` and this inventory up to date: the decisions, the glossary, the Pages table, the stale nav-dot and `dark.dc.html` references; the audit itself in `docs/design/`. | all | This change |
| PR-1 | Raise `--color-label` (dark) and `--color-text-muted` (light) to 4.5:1 and use `--color-text-muted` for placeholders; one `--ring` focus rule for links and `[tabindex]`; label the theme select in Settings; calendar event colours as tokens in both themes; move the 10.5, 11, 11.5, 12.5, 13 and 14.5px sizes and the 6, 9 and 11px radii onto the scale; drop the second `.alert-warning`, the static `style:` in `pages/invoices.rs` and `invoices/settings.rs`; decide whether the sign-in page loads `horae.css` or is documented as dark only. | Accessibility, Tokens & utilities | Not started |
| PR-2 | `PageHeader` with the 34/600 title, one back link and the actions on all 26 screens; one `.page-title` definition; replace the Harvest `BackLinks` breadcrumb with a back link; remove the `AdminShell` "Back to Timesheet" link; header actions at normal size. | Type scale, Page header | Not started |
| PR-3 | `ConfirmDialog` and fixed `Modal` slots; add the Danger solid button; move irreversible confirmations to it; drop the dialog when archiving a single record (the toast's "Undo" replaces it) and keep a neutral-tone dialog only for archiving that carries other records or acts in bulk; delete `.ts-modal-*`; give the four unpadded dialogs (disconnect Harvest, change account, both over-invoicing dialogs) the shared anatomy. Footer order waits for its decision. | Dialogs, Destructive actions, Buttons | Not started; footer order pending |
| PR-4 | `EmptyState`, `ErrorState` and `Loading` on about 20 screens; fold the Projects card into `.empty-state`; `ErrorState` with the same 52px icon tile; optional `Skeleton` for listings; "Try again" on every retry. | Empty, loading and error states, Screen States | Not started |
| PR-5 | `FilterBar` and `SearchField` on every filterable list; move the native `<select>` uses (`reports.rs` ×3, `tasks.rs`, `tasks/editor.rs`, `admin/people.rs`, `admin/add_person.rs`, `admin/permission_editor.rs`, `components/project_task_picker.rs`, the Settings theme select) to `SelectField`; retire `form::Select`. | Selects, filters and search | Not started |
| PR-6 | Organization default and personal override for the duration format; one formatter in `horae-core`, used by the UI, the server and the reports, and the `pages/projects.rs` formatter removed; exports stay decimal; replace `type=date` inputs (Reports, invoice preparation, New Project milestones) with `DatePicker`; extract `PeriodNav` from the Timesheet and use it in Approvals. | Formats, Selects, filters and search | Not started |
| PR-7 | Timesheet week fits at 1280px with the sidebar (Total and delete visible); name columns get a minimum width; Projects' last column is not cut off. The mobile layout waits for its design. | Layout | Desktop done; mobile pending |
| PR-8 | One "Actions" menu per table row, except Approvals' inline "Approve" and "Reopen", which become Secondary `btn-sm`; status always a `Badge`; Primary for main actions and form submits, at most one per screen or form; retire Solid: Approvals' "Approve all" and the Invoice settings submits become Primary, "Submit week" becomes a `.btn-split` with a Primary body and `.ts-submit` goes, then delete `.btn-solid` and `--glow-pine`; retire the timesheet's other `ts-*` classes that copy `.btn`. | Buttons, Data Tables, Destructive actions | Not started |
| PR-9 | One toast container in `AppLayout`, styled after `Toast.dc.html`; every success confirmed with a toast; inline "Saved." text and `.alert-success` confirmations removed; long messages wrap instead of truncating. | Toast | Not started |
| PR-10 | Apply the glossary: "People" in the sidebar and subnavigation, "Team" only for a project's team, "Person" for the Reports grouping, "Restore", "Settings" for the workspace page and "My settings" for `/settings`, "Try again", "Discard changes", "Cancel" and "Close", and the conventions (sentence case, ellipses, empty-state titles, toast wording). | Glossary | Not started |

PR-0 and PR-1 depend on no open question. PR-4 follows PR-2, PR-7 follows PR-5,
and PR-10 is best last so its copy changes do not conflict with the others.
The browser suites in `crates/horae/tests/browser/` (`shared-style-audit.cjs`,
`responsive-layout.cjs`, `modals.cjs`) check classes and measurements, so each PR
records the `shared-style-audit` baseline at its starting commit and compares
against it.

## Open questions

Two decisions are under review and are marked *Pending* in `DESIGN.md`:

- **Dialog footer order and alignment** (blocks part of PR-3). The kit puts the
  primary action on the right; `ConfirmDialog`, most prototype screens and the
  app put it first, on the left.
- **Tables on mobile** (blocks the mobile part of PR-7). The prototypes stop at
  900px. The direction under review is stacked cards for list tables and
  horizontal scroll with an indicator for wide data tables; the stacked card and
  the Timesheet at 390px need designing first.

## Superseded entries

These parts of the 2026-09-16 inventory below no longer apply:

- **Work already in progress.** PRs #201 and #202 (Harvest import experience) and
  the Projects improvements in #204 and #205 have merged.
- **Client detail** is implemented in [ClientDetail](../crates/horae/src/pages/clients/detail.rs),
  with the client's projects, hours and invoices. It no longer renders a placeholder.
- **Project detail** is no longer a placeholder: it shows the project's identity,
  details, tasks and team, and offers its record actions in an "Actions" menu. The
  dashboard of feature 004 is still not built (see the table below).
- **`EmptyState` is optional.** It is not: every empty, loading and error state now
  uses the shared components (PR-4).
- **`PeriodNav` waits for a second consumer.** Approvals is one (PR-6).
- **`format.js`: agree on H:MM vs decimal-hour display.** Decided: configurable per
  organization with a personal override, one formatter in `horae-core`, decimal
  exports (PR-6).
- **The root `DESIGN.md` has stale route and path descriptions.** Fixed in PR-0.

## Historical inventory (2026-09-16)

> The sections below are the original source-based comparison against the
> `Horae (1).zip` handoff and application baseline `d32d56f`, kept for the
> product-scope boundaries they record. Their visual and component notes are
> superseded by `DESIGN.md` and the table above; check scope entries against
> current code before planning from them.

This is a source-based inventory, not visual acceptance, a new feature
specification, or an assertion that existing screens are pixel-perfect. No
application code or real account data was changed for this audit.

The export adds client-list and client-detail designs, 13 reusable component
files and `format.js`; all 17 previously versioned prototype/support files change.
New prototype components do not necessarily mean missing production components.

### Screen coverage

“Partial” means useful functionality exists but the handoff contains additional
UI or behavior. Items requiring new product decisions are separated below.

| Prototype | Existing implementation | Remaining work / boundary |
| --- | --- | --- |
| [01 Sign In](project/app/01_Sign%20In.dc.html) | [Auth page](../crates/api/src/auth/page.rs) uses the editorial sign-in layout and one configured OIDC provider (development bypass separately). | Review visual alignment only within supported login. Passwords, recovery, provider picker and direct SAML are not existing Horae flows. |
| [02 Create Workspace](project/app/02_Create%20Workspace.dc.html) | First organization/admin setup is through `horae init`, dispatched in [main.rs](../crates/horae/src/main.rs). | No web onboarding wizard, workspace slug or logo-upload flow. Self-service signup is a product expansion, not a missing button. |
| [03 Invite Team](project/app/03_Invite%20Team.dc.html) | [AdminUsers](../crates/horae/src/pages/admin.rs) creates users and manages their access and active status. | No multi-recipient invitation wizard, invitation lifecycle, delivery or shareable join link. Distinguish user creation from sending an invitation. |
| [04 Timesheet](project/app/04_Timesheet.dc.html) | [Timesheet](../crates/horae/src/pages/timesheet.rs) has day/week/calendar, timer, entry editing, calendar drawing/moving/resizing and weekly submission. | Copy previous week, day/custom-range approval submission and external calendar-event import are not implemented in this page. Reconcile their scope before adding them; retain the working time-entry/calendar behavior. |
| [05 Projects](project/app/05_Projects.dc.html) | [ProjectList](../crates/horae/src/pages/projects.rs) has search, client/status filters, budgets/spend, create/edit/archive and CSV/XLSX exports. | Scheduled/Delta cells are placeholders, not a scheduling engine. Do not rebuild existing exports or CRUD. |
| [06 Approvals](project/app/06_Approvals.dc.html) | [Approvals](../crates/horae/src/pages/approvals.rs) has pending/approved/all views, hour totals, per-row approve/reopen and bulk approval. | Department filtering, not-submitted roster and reminders. The prototype's expense wording is not evidence of an expense feature. |
| [07 Reports](project/app/07_Reports.dc.html) | [Reports](../crates/horae/src/pages/reports.rs) and [report functions](../crates/horae/src/server_fns/reports.rs) provide grouped/detailed time, dates/client/project/person filters, hours/amount/cost and CSV/XLSX. | Configurable multi-field/multi-metric builder, saved report templates, advanced filter composition and tutorial flow. This is more than a visual refresh. |
| [08 Settings](project/app/08_Settings.dc.html) | [Settings](../crates/horae/src/pages/settings.rs) provides profile, theme, own permissions, API tokens and installed-plugin information. | Rates UI, assigned-people overview and notification preferences. The prototype's six permission profiles conflict with Horae's permission model; do not copy them. |
| [09 Workspace](project/app/09_Workspace.dc.html) | [AdminShell](../crates/horae/src/components/admin_shell.rs) exposes People, Settings, Importers, the audit log and API tokens. | Invitation status/last activity, workspace ZIP export and backup management. Automatic backups, retention and workspace deletion need explicit backend and policy work. |
| [10 Importers](project/app/10_Importers.dc.html) | [HarvestImport](../crates/horae/src/pages/importers.rs) has Harvest/CSV, durable preview/import jobs, reports/errors, incremental import, protected account changes and OAuth app settings. | Other importers and refresh promises. Preserve the actual server limits and account-switch guarantees. |
| [11 Project Detail](project/app/11_Project%20Detail.dc.html) | [ProjectDetail](../crates/horae/src/pages/projects.rs) shows the project's identity, details, tasks and team, with edit and archive in an "Actions" menu. | Budget/progress, hours split, task/person breakdowns, invoiced/uninvoiced totals and recent entries: resume feature 004. Charts, costs/margins and advanced editing remain separate deferred scope. |
| [12 Clients](project/app/12_Clients.dc.html) | [ClientList](../crates/horae/src/pages/clients.rs) has search, filters, create/edit/archive and links to the detail page. | Check the prototype's remaining panels (summary cards, bulk actions, export) against current code before planning them. |
| [12 Client Detail](project/app/12_Client%20Detail.dc.html) | [ClientDetail](../crates/horae/src/pages/clients/detail.rs) shows the client's identity, projects, hours and invoices. | Contact/default-rate/payment-term fields need a specified data model, not fabricated values. |

Invoice list/detail screens already exist in [invoices.rs](../crates/horae/src/pages/invoices.rs)
and [Route](../crates/horae/src/route.rs), but the handoff includes no dedicated invoice
screen. Their absence from the export must not be counted as missing application
functionality or used to remove them. D-013 in `specs/harvest-parity.md` lets screens
without a prototype be composed from current components and tokens; they follow
`DESIGN.md` like any other screen.

### Shared UI: reuse before adding

| Export component(s) | Existing production counterpart | Follow-up |
| --- | --- | --- |
| `Rail`, `UserMenu`, `AdminSubnav` | [Sidebar](../crates/horae/src/components/sidebar.rs), [AppLayout](../crates/horae/src/components/layout.rs), [AdminShell](../crates/horae/src/components/admin_shell.rs) | Keep only working destinations and retain accessible mobile navigation. Labels follow the glossary (PR-10). |
| `Dropdown`, `ConfirmDialog` | [Menu](../crates/horae/src/components/menu.rs), [Combobox](../crates/horae/src/components/combobox.rs), [Modal](../crates/horae/src/components/modal.rs) | Reuse native popover/dialog behavior, focus handling and pending-action protection; do not port mock DOM handlers. `ConfirmDialog` is PR-3. |
| `DatePicker`, `PeriodNav` | [DatePicker](../crates/horae/src/components/date_picker.rs) and date navigation in Timesheet | `PeriodNav` is extracted for Approvals in PR-6. |
| `SegmentedControl`, `Toggle`, `Toast`, `Icon` | [Controls](../crates/horae/src/components/controls.rs), [Toast](../crates/horae/src/components/toast.rs), [Icons](../crates/horae/src/components/icons.rs) | The toast moves to the design's position and style in PR-9. |
| `EmptyState`, `ErrorState`, `Skeleton` | Page-specific states | Shared components in PR-4. |
| `format.js` | [Core duration](../crates/core/src/duration.rs), [money](../crates/core/src/money.rs) and existing display helpers | Duration display is PR-6. Do not replace exact Rust calculations with the prototype's JavaScript number arithmetic. |
| `DevBar` | None required | Design-only state picker; never ship it as product UI. |

### Product decisions recorded in 2026-09

These are scope boundaries, not design rules, and still apply unless a
specification changes them.

1. **Currency and financial totals.** The client list combines CHF/EUR/USD/GBP
   examples beneath one EUR headline and shows one client as `EUR / USD`.
   [Feature 001](../specs/001-time-tracking-invoicing/spec.md) assumes one currency
   per client and no conversion. Group monetary totals by currency or require an
   explicit currency filter; never silently add unlike currencies. Define which
   invoice states and date basis count toward billed/open/unbilled totals.
1. **Client metadata and archival.** The [Client type](../crates/types/src/client.rs)
   has name, currency, address, tax ID and active state, but no contact record,
   default rate or payment terms. The mock archive dialog promises to archive all
   projects; [set_client_active](../crates/horae/src/server_fns/clients.rs) updates
   the client, not every project's active flag. Specify cascade/restore,
   duplicate/delete semantics and permissions before promising them in UI.
1. **Roles and ownership.** The prototypes list six profiles in Settings and three
   fixed roles in Workspace. Horae uses scoped permissions (spec 015) and has no
   six-profile matrix. User identity edits also need an OIDC ownership policy. Do
   not extend privileges to match a mockup.
1. **Authentication and tenancy.** Keep single-org, administrator-created accounts
   and configured OIDC unless a separate onboarding/authentication feature is
   approved. A workspace switcher or `horae.app/<slug>` is not an existing tenant
   model; adding provider buttons alone cannot implement it.
1. **Harvest truthfulness.** The prototype says preview writes no rows, every
   record is written, a token refresh resolves itself, and suggests reconnecting
   an original account for mismatches. Features 005–008 distinguish operational
   job records from business records, skipped results, recovery and explicit
   account switching. Preserve those contracts, not misleading export copy.
   The mocked 25 MB upload limit and credential-storage promises also need to
   follow the actual implementation, not the illustration.
1. **Project dashboard scope.** Feature 004 defers charts, forecasting,
   costs/profit, rate editing, recurring/milestone schedules and dashboard
   exports. The prototype additionally shows tags, notes, report access, invoice
   defaults and budget emails. Keep these outside the dashboard unless its spec
   is deliberately amended.
1. **Background notifications and administration.** Reminder delivery, scheduled
   backups, retention and full exports are separate capabilities. The durable
   queue can support future work but does not by itself supply their scheduling,
   transport, authorization or storage policy.
1. **Responsive/accessibility behavior.** The exported CSS hides the rail below
   900 px; it is not permission to remove the application's mobile navigation.
   How tables behave on phones is a pending design decision (see Open questions).

### Import validation (2026-09)

- ZIP integrity check passed for all 34 entries; imported `project/` matches the
  extracted archive byte-for-byte, with no stale extra files.
- 218 static local URL/component-name references were checked. One upstream
  target is missing: `foundations/Components.dc.html` links to `07_Reports.dc.html`
  relative to `foundations/`, instead of the screen in `app/`. Preserved unchanged.
- No prototype scripts were executed and no prototype screenshots were taken.
