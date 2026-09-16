# Design implementation inventory

Reviewed on 2026-09-16 against the `Horae (1).zip` handoff and application baseline
`d32d56f` (`origin/master`, safe Harvest account switching). This is a source-based
inventory, not visual acceptance, a new feature specification, or an assertion
that existing screens are pixel-perfect. No application code or real account data
was changed for this audit.

The export adds client-list and client-detail designs, 13 reusable component
files and `format.js`; all 17 previously versioned prototype/support files change.
New prototype components do not necessarily mean missing production components.

## Work already in progress

- [PR #201](https://github.com/numtide/horae/pull/201) specifies the consistent
  Harvest import experience, feature `008-harvest-import-ux`.
- [Draft PR #202](https://github.com/numtide/horae/pull/202), stacked on #201,
  implements connection management, truthful preview/results, durable-job history
  and recovery, and responsive/keyboard behavior. Reviewed at `3eac906`.
  Its full Nix gate and authorized real Harvest preview acceptance remain pending;
  do not count it as merged or implement the same work again.
- The [project dashboard specification](../specs/004-project-dashboard/spec.md),
  plan and tasks already exist. `ProjectDetailContent` still has a placeholder
  header plus task and assignment management. The dashboard is specified, not
  implemented; refresh those artifacts against current code before resuming them.

PR state above is a snapshot, not a live status indicator.

## Screen coverage

“Partial” means useful functionality exists but the handoff contains additional
UI or behavior. Items requiring new product decisions are separated below.

| Prototype | Existing implementation | Remaining work / boundary |
| --- | --- | --- |
| [01 Sign In](project/app/01_Sign%20In.dc.html) | [Auth page](../crates/horae/src/auth/page.rs) uses the editorial sign-in layout and one configured OIDC provider (development bypass separately). | Review visual alignment only within supported login. Passwords, recovery, provider picker and direct SAML are not existing Horae flows. |
| [02 Create Workspace](project/app/02_Create%20Workspace.dc.html) | First organization/admin setup is through `horae init`, dispatched in [main.rs](../crates/horae/src/main.rs). | No web onboarding wizard, workspace slug or logo-upload flow. Self-service signup is a product expansion, not a missing button. |
| [03 Invite Team](project/app/03_Invite%20Team.dc.html) | [AdminUsers](../crates/horae/src/pages/admin.rs) creates users and manages three roles/active status. | No multi-recipient invitation wizard, invitation lifecycle, delivery or shareable join link. Distinguish user creation from sending an invitation. |
| [04 Timesheet](project/app/04_Timesheet.dc.html) | [Timesheet](../crates/horae/src/pages/timesheet.rs) has day/week/calendar, timer, entry editing, calendar drawing/moving/resizing and weekly submission. | Copy previous week, day/custom-range approval submission and external calendar-event import are not implemented in this page. Reconcile their scope before adding them; retain the working time-entry/calendar behavior. |
| [05 Projects](project/app/05_Projects.dc.html) | [ProjectList](../crates/horae/src/pages/projects.rs) has search, client/status filters, budgets/spend, create/edit/archive and CSV/XLSX exports. | Scheduled/Delta cells are placeholders, not a scheduling engine. Check row actions, empty-state guidance and presentation against the refreshed source; do not rebuild existing exports or CRUD. |
| [06 Approvals](project/app/06_Approvals.dc.html) | [Approvals](../crates/horae/src/pages/approvals.rs) has pending/approved/all views, hour totals, per-row approve/reopen and bulk approval. | Department/period filtering, not-submitted roster and reminders. The prototype's expense wording is not evidence of an expense feature. |
| [07 Reports](project/app/07_Reports.dc.html) | [Reports](../crates/horae/src/pages/reports.rs) and [report functions](../crates/horae/src/server_fns/reports.rs) provide grouped/detailed time, dates/client/project/person filters, hours/amount/cost and CSV/XLSX. | Configurable multi-field/multi-metric builder, saved report templates, advanced filter composition and tutorial flow. This is more than a visual refresh. |
| [08 Settings](project/app/08_Settings.dc.html) | [Settings](../crates/horae/src/pages/settings.rs) provides theme selection and installed-plugin information. | Personal profile/photo/timezone, rates UI, assigned-people overview and notification preferences. The prototype's six permission profiles conflict with current roles; do not copy them. |
| [09 Workspace](project/app/09_Workspace.dc.html) | [AdminShell](../crates/horae/src/components/admin_shell.rs) exposes People and Importers; [AdminUsers](../crates/horae/src/pages/admin.rs) provides basic people/task administration. | General-settings UI, role matrix, invitation status/last activity, workspace ZIP export, backup management and audit-log UI. Automatic backups, retention and workspace deletion need explicit backend and policy work. |
| [10 Importers](project/app/10_Importers.dc.html) | [HarvestImport](../crates/horae/src/pages/importers.rs) has Harvest/CSV, durable preview/import jobs, reports/errors, incremental import and protected account changes. | Review the refreshed handoff against #202 after acceptance. OAuth credential editor/handshake, other importers and refresh promises are not covered by #202. Preserve the actual server limits and account-switch guarantees. |
| [11 Project Detail](project/app/11_Project%20Detail.dc.html) | [ProjectDetailContent / ProjectTasks](../crates/horae/src/pages/projects.rs) provides tasks and assignments; dashboard header remains a placeholder. | Identity, budget/progress, hours split, task/person breakdowns, invoiced/uninvoiced totals and recent entries: resume feature 004. Charts, costs/margins and advanced editing are separate deferred scope. |
| [12 Clients](project/app/12_Clients.dc.html) | [ClientList](../crates/horae/src/pages/clients.rs) lists name/currency/address/status and supports create/edit/activate/deactivate. | Search, active/archive/currency filters, real summary cards, project/hour/unbilled columns, bulk actions, export and guided empty state. Client names are currently plain text: add navigation to a functional detail page. |
| [12 Client Detail](project/app/12_Client%20Detail.dc.html) | [ClientDetail](../crates/horae/src/pages/clients.rs) only renders `Client detail for {id}`. | Real identity, projects, hours, unbilled/billed/open-invoice summaries, invoice links and contextual actions. Contact/default-rate/payment-term fields need a specified data model, not fabricated values. |

Invoice list/detail screens already exist in [invoices.rs](../crates/horae/src/pages/invoices.rs)
and [Route](../crates/horae/src/route.rs), but this ZIP includes no dedicated invoice
screen. Their absence from the export must not be counted as missing application
functionality or used to remove them. Invoice-related panels above still need
integration with those existing flows.

## Shared UI: reuse before adding

| Export component(s) | Existing production counterpart | Follow-up |
| --- | --- | --- |
| `Rail`, `UserMenu`, `AdminSubnav` | [Sidebar](../crates/horae/src/components/sidebar.rs), [AppLayout](../crates/horae/src/components/layout.rs), [AdminShell](../crates/horae/src/components/admin_shell.rs) | Align grouping, active/detail-route state and identity/menu content. Keep only working destinations and retain accessible mobile navigation. |
| `Dropdown`, `ConfirmDialog` | [Menu](../crates/horae/src/components/menu.rs), [Combobox](../crates/horae/src/components/combobox.rs), [Modal](../crates/horae/src/components/modal.rs) | Reuse native popover/dialog behavior, focus handling and pending-action protection; do not port mock DOM handlers. |
| `DatePicker`, `PeriodNav` | [DatePicker](../crates/horae/src/components/date_picker.rs) and date navigation in Timesheet | Reuse current controls; extract period navigation only if a real second consumer needs it. |
| `SegmentedControl`, `Toggle`, `Toast`, `Icon` | [Controls](../crates/horae/src/components/controls.rs), [Toast](../crates/horae/src/components/toast.rs), [Icons](../crates/horae/src/components/icons.rs) | Check styling, keyboard semantics and consistent use; these are not missing infrastructure. |
| `EmptyState` | Page-specific empty states | Add actionable, truthful empty states where needed. A shared component is optional, not a prerequisite to shipping a screen. |
| `format.js` | [Core duration](../crates/core/src/duration.rs), [money](../crates/core/src/money.rs) and existing display helpers | Agree on H:MM vs decimal-hour display, date/timezone and currency labels. Do not replace exact Rust calculations with the prototype's JavaScript number arithmetic. |
| `DevBar` | None required | Design-only state picker; never ship it as product UI. |

## Decisions required before implementation

1. **Currency and financial totals.** The client list combines CHF/EUR/USD/GBP
   examples beneath one EUR headline and shows one client as `EUR / USD`.
   [Feature 001](../specs/001-time-tracking-invoicing/spec.md) assumes one currency
   per client and no conversion. Group monetary totals by currency or require an
   explicit currency filter; never silently add unlike currencies. Define which
   invoice states and date basis count toward billed/open/unbilled totals.
1. **Client metadata and archival.** The [Client model](../crates/horae/src/models/client.rs)
   has name, currency, address, tax ID and active state, but no contact record,
   default rate or payment terms. The mock archive dialog promises to archive all
   projects; [set_client_active](../crates/horae/src/server_fns/clients.rs) updates
   the client, not every project's active flag. Specify cascade/reactivation,
   duplicate/delete semantics and permissions before promising them in UI.
1. **Roles and ownership.** Settings lists six profiles; Workspace explicitly
   lists three fixed roles. The application uses admin/manager/member and has no
   equivalent six-profile matrix. User identity edits also need an OIDC ownership
   policy. Do not extend privileges to match a mockup.
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
1. **Project dashboard scope.** Feature 004 explicitly defers charts, forecasting,
   costs/profit, rate editing, period filtering, recurring/milestone schedules and
   dashboard exports. The prototype additionally shows tags, notes, report access,
   invoice defaults and budget emails. Keep these outside the existing MVP unless
   its spec is deliberately amended. Reconcile its older rate/query assumptions
   with today's billing code before implementing its tasks.
1. **Background notifications and administration.** Reminder delivery, scheduled
   backups, retention, full exports and audit trails are separate capabilities.
   The durable queue can support future work but does not by itself supply their
   scheduling, transport, authorization or storage policy.
1. **Responsive/accessibility behavior.** The exported CSS hides the rail below
   900 px; it is not permission to remove the application's mobile navigation.
   Validate real pages at narrow/short viewports and keyboard/zoom acceptance when
   implementing them. The source audit has not performed that visual validation.

## Suggested delivery order

1. Finish acceptance of #201/#202 and review the updated Importers handoff against
   that implementation; do not duplicate it in a new feature.
1. Resume `004-project-dashboard`: start with real identity, budget and hours,
   then reconciled task/person and invoice totals. Update its existing Spec Kit
   artifacts rather than creating a competing dashboard specification.
1. Specify Clients list/detail as one coherent feature: navigation, existing
   fields, projects and currency-safe summaries first. Separate additional
   contact/billing metadata and bulk/destructive actions until clarified.
1. Specify personal settings and workspace administration in small slices, with
   explicit role and identity boundaries; keep notification/backups work separate.
1. Specify the report builder/saved reports and approval reminders independently;
   keep existing reporting and approval operations working throughout.

Shared visual cleanup should accompany each approved screen, not block the above
on a wholesale component rewrite. There is no need for another crate or queue
solely to import this design or align its presentation.

Before claiming any slice complete: update its Spec Kit artifacts, implement
real authorized data/actions and empty/error/loading states, test arithmetic and
permissions where relevant, perform browser accessibility/responsive checks, and
pass the repository's formatting and Nix gates.

## Import validation

- ZIP integrity check passed for all 34 entries; imported `project/` matches the
  extracted archive byte-for-byte, with no stale extra files.
- 218 static local URL/component-name references were checked. One upstream
  target is missing: `foundations/Components.dc.html` links to `07_Reports.dc.html`
  relative to `foundations/`, instead of the screen in `app/`. Preserved unchanged.
- No prototype scripts were executed and no prototype screenshots were taken.
- Rust, SQL, deployment configuration and application assets are unchanged;
  application tests were not rerun for this reference/documentation-only update.

This inventory is not a replacement for `specs/`. The root `DESIGN.md` also has
stale route/path descriptions (for example `project/dark.dc.html`); refresh those
separately as the corresponding screen specifications are reconciled.
