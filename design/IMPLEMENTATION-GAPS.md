# Design implementation inventory

Updated on 2026-11-22 from the [design audit of 2026-11-18](../docs/design/design-audit.md)
(application baseline `3b08dd25`), the decisions taken on it, the decisions of
2026-11-20 on two studies of master `d9791da4` (screen coherence and loading), and
the PRs merged up to 2026-11-22 (#443 to #453, and #455).
[DESIGN.md](../DESIGN.md) holds those decisions and is the source of truth for every
rule below; this file only tracks what the application still has to change to follow
it, and which PR does it.

The audit compared the 26 application screens, at 360, 768 and 1280px in both
themes, with `DESIGN.md`, the kit and the prototypes. Its findings are grouped by
pattern, and each pattern is fixed across every screen in one PR, numbered PR-0
to PR-10. This file is PR-0. The decisions of 2026-11-20 add PRs named by letter
(PR-P, PR-C, PR-S, PR-T, PR-A, PR-People, the Tasks PR and the session PR). Rows
cite the coherence study's findings by id (P-n.m); the study is not in the
repository, so [Findings cited](#findings-cited) lists each one.

The audit cites 156 screenshots by name (`audit/screenshots/<screen>-<width>-<theme>.png`,
about 10 MB). Because of their size they are not in the repository; each PR takes
its own before-and-after captures.

## Pending work by PR

| PR | Work | `DESIGN.md` section | Status |
| --- | --- | --- | --- |
| PR-0 | Bring `DESIGN.md` and this inventory up to date: the decisions, the glossary, the Pages table, the stale nav-dot and `dark.dc.html` references; the audit itself in `docs/design/`. | all | Done (#434) |
| PR-1 | Raise `--color-label` (dark) and `--color-text-muted` (light) to 4.5:1 and use `--color-text-muted` for placeholders; one `--ring` focus rule for links and `[tabindex]`; label the theme select in Settings; calendar event colours as tokens in both themes; move the 10.5, 11, 11.5, 12.5, 13 and 14.5px sizes and the 6, 9 and 11px radii onto the scale; drop the second `.alert-warning`, the static `style:` in `pages/invoices.rs` and `invoices/settings.rs`; decide whether the sign-in page loads `horae.css` or is documented as dark only. | Accessibility, Tokens & utilities | Done. The sign-in page is decided (it loads `horae.css` and follows the theme) and goes in its own PR |
| PR-2 | `PageHeader` with the 34/600 title, one back link and the actions on all 26 screens; one `.page-title` definition; replace the Harvest `BackLinks` breadcrumb with a back link; remove the `AdminShell` "Back to Timesheet" link; header actions at normal size. | Type scale, Page header | Done (#437). Left: the Timesheet's `.ts-header` copy of `.page-header` and its 26px `Segmented` (P-1.10) |
| PR-3 | `ConfirmDialog` and fixed `Modal` slots; move irreversible confirmations to it; drop the dialog when archiving a single record (the toast's "Undo" replaces it) and keep a neutral-tone dialog only for archiving that carries other records or acts in bulk; delete `.ts-modal-*`; give the four unpadded dialogs (disconnect Harvest, change account, both over-invoicing dialogs) the shared anatomy; one footer order for every dialog, drawn only by `Modal`. | Dialogs, Destructive actions, Buttons | Done (#445). Footer order resolved: primary first on the left, Cancel last, stretched. Every confirmation, including deleting a time entry, the Harvest OAuth app, discarding permission changes and deleting a custom profile, is a `ConfirmDialog`. Left for PR-9: archiving one client or one task still asks first (neutral `ConfirmDialog`), because their toasts have no Undo yet. Left for its own PR: the editor navigation guard (below) |
| PR-4 | `EmptyState`, `ErrorState` and `Loading` on about 20 screens; fold the Projects card into `.empty-state`; `ErrorState` with the same 52px icon tile; optional `Skeleton` for listings; "Try again" on every retry; the `AdminShell`'s "Access unavailable" card, which still has its own `h1` and a `btn-secondary` "Back to Timesheet" (a no-permission state: name who can grant access instead). | Empty, loading and error states, Screen States | Done (#443). Retries after an uncertain save ("Retry same save", "Retry generation" and the like) keep their labels. Each kind of empty state now has its own icon (P-7.1): `plugin` and `key` from PR-P (#447), `history` from PR-A (#450) |
| PR-5 | `FilterBar` and `SearchField` on every filterable list; move the native `<select>` uses (`reports.rs` ×3, `tasks.rs`, `tasks/editor.rs`, `admin/people.rs`, `admin/add_person.rs`, `admin/permission_editor.rs`, `components/project_task_picker.rs`, the Settings theme select) to `SelectField`; retire `form::Select`. | Selects, filters and search | Not started. `FilterBar` and `SearchField` exist (`components/filter_bar.rs`, from PR-People, #448), and People and the audit log use them; PR-5 brings them to the other lists. Also left from PR-P: the theme as a `Segmented` instead of a native select |
| PR-6 | Organization default and personal override for the duration format; one formatter in `horae-core`, used by the UI, the server and the reports, and the `pages/projects.rs` formatter removed; exports stay decimal; one money formatter in `horae-core` (currency code, grouped thousands, the currency's own decimals) used by the UI, the server, PDFs and emails, with no amount built by hand in a page and `format_cents_plain` kept for editable fields and interchange formats; replace `type=date` inputs (Reports, invoice preparation, New Project milestones) with `DatePicker`; use `PeriodNav` in Approvals (PR-People extracted it from the Timesheet's week pager, #448). | Formats, Selects, filters and search | Not started |
| PR-7 | Timesheet week fits at 1280px with the sidebar (Total and delete visible); name columns get a minimum width; Projects' last column is not cut off. The mobile layout waits for its design. | Layout | Desktop done (#436); mobile pending |
| PR-8 | One "Actions" menu per table row, except Approvals' inline "Approve" and "Reopen", which become Secondary `btn-sm`; status always a `Badge`; Primary for main actions and form submits, at most one per screen or form; retire Solid: Approvals' "Approve all" and the Invoice settings submits become Primary, "Submit week" becomes a `.btn-split` with a Primary body and `.ts-submit` goes, then delete `.btn-solid` and `--glow-pine`; retire the timesheet's other `ts-*` classes that copy `.btn`. Its labels go into the text catalog (`crates/i18n/locales/en/horae.ftl`) from its first commit, as any new interface text does (`specs/023-i18n`, C-9). | Buttons, Data Tables, Destructive actions | Mostly done (#449). People's row actions and "Add person" (P-2.3, P-1.1) came with PR-People (#448). "Submit week" is a plain Primary: the `.btn-split` waits for a second way to submit. Left: the Timesheet's `.ts-addrow`, `.ts-del` and `.ts-pager-btn` (the week grid's "Add row" and "×" have no `.btn` look at their size, and `.ts-pager-btn` is now `PeriodNav`'s own chrome, which Approvals' pager still copies until PR-6); Client detail's project table as `DataTable` (P-2.8, C5); Projects' glyph buttons (P-10.6, C11b) |
| PR-9 | One toast container in `AppLayout`, styled after `Toast.dc.html`; every success confirmed with a toast; inline "Saved." text and `.alert-success` confirmations removed; long messages wrap instead of truncating. | Toast | Not started |
| i18n I-1 | The interface text catalog: the `horae-i18n` crate with `locales/en/horae.ftl`, the `Tr` methods generated from it, `use_tr()` in the app and the `dioxus/literal-text` lint ratchet (`specs/023-i18n`, T001–T022). No screen changes. | Glossary | Done (#444). People, My settings and the audit log have since taken their text from the catalog |
| i18n I-2 onwards | Move the text of each area into the catalog (I-2 to I-12 in `specs/023-i18n`), lowering the `literal-text` baseline as files empty out, then I-13, which closes the ratchet. | Glossary | Not started |
| PR-10 | Apply the glossary: "People" in the sidebar and subnavigation, "Team" only for a project's team, "Person" for the Reports grouping, "Restore", "Settings" for the workspace page and "My settings" for `/settings`, "Try again", "Discard changes", "Cancel" and "Close", and the conventions (sentence case, ellipses, empty-state titles, toast wording). It waits for I-13 of `specs/023-i18n` (the ratchet closed), and then edits only `crates/i18n/locales/en/horae.ftl` and tests (C-6). | Glossary | Not started |
| Session PR | Load the session (`get_me`, `get_my_permissions`) with `use_loader` in `use_session_provider`, so the server renders the sidebar, its sections and permission-dependent actions in the first HTML; move the week start into the session; drop `AdminShell`'s own "Loading…" and load `get_org_name` the same way; add the `ErrorBoundary` that `use_loader` failures need. First check that server functions can read the session during the server render. It changes the first HTML, the sidebar and the waves that wait on the session; it does not change how screens reload. | Screen States (Loading) | Done (#451). The person, their permissions, the week start and the workspace name load in one `use_loader`, because a second loader beside it hangs the server render in Dioxus 0.7.9 (`tests/loader_suspense.rs`). `AdminShell` lost its "Loading…", and HTML responses carry `Cache-Control: private, no-store`. Screens still load their own data on the client |
| PR-P | Permissions in My settings and one "no access" form. My settings becomes subpages in a side menu (SETTINGS: Profile, Preferences, API tokens; ACCOUNT: Your access); Your access: for a member, an area × View / Edit / Approve table and the line "To change your access, ask an administrator of this workspace."; for an administrator, one sentence linking to People > Permissions and no table; for both, People you manage, Projects you manage and a `Link` to "See changes to your access"; no amber notice and no "Refresh permissions"; the profile shown once. Plugins move to Workspace settings, with an empty state written for users. Own icons for the empty plugins and token lists. One "Access unavailable" state on every screen, with nothing around it; the sidebar filters Invoices and Approvals by permission. Covers P-4.6, P-5.1, P-6.4, P-7.1 (plugins, both token lists), P-7.2–P-7.4, P-8.1, P-8.4, P-8.6 and P-9.1–P-9.3, P-9.5, P-9.6 (P-9.4 is superseded by the Your access decision). | My settings, Workspace settings, Sidebar, Screen States (No permission), Glossary (copy) | Done (#447). The administrator's sentence links to People. Left: the token form layout, the 720px width and the radius (PR-S); the theme as a `Segmented` (PR-5); `LoadError`'s "Could not load {what}" title, which stays literal until each caller has its own message. Time zone stays in Profile with its Save button instead of moving to Preferences |
| PR-People | People as a main screen, phase 1: rename the sidebar's Organize entry from "Team" to People, remove the "Team" link from the admin subnavigation and take the screen out of `AdminShell` (the route path is decided here); "Add person" in the header, opening a `Modal` with `Radio` profile cards; a week selector, extracting `PeriodNav` from the Timesheet's week pager, and the hours summary; `FilterBar` (search, a profile `Menu`, a status `Menu` with Active, Archived and All, Active by default, counts in each option); table with hours, billable hours and billable %; one "Actions" `Menu` per row with only what the viewer may do; whole list, no "Refresh"; the screen keeps its data on reload, so rows dim while a new week loads. Needs the profile and the week's hours in the people listing (a small server change). Covers P-1.1, P-2.3, P-3.1 (People), P-4.2 and P-6.3 (People). | People, Sidebar, Data Tables | Done (#448). The route is `/people`, and `/admin/users` redirects to it. The name is not a link, since View timesheet covers it. Left: "Edit details…" (row below) |
| PR-People 2 | Weekly capacity per person (a new column and its editor), then utilization and available capacity. Later, an assignments tab and Export. | People | Not started |
| Edit details | "Edit details…" in People's row menu. No operation edits a person's name or email yet, and changing the sign-in email needs its own decision. | People | Not started. Its own PR |
| PR-S | Sections, radius and width: every section a card with `card-title`; `--radius-xl` for tables, cards, `Skeleton`, `EmptyState` and `ErrorState`; `--width-form` (720px) for forms and form cards, lists at full width; remove `.adm-main { max-width: 760px }`; lists of data without bullets. With C4 (`Card`). Covers P-5.2–P-5.8, P-6.1, P-6.2, P-6.5 and what is left of P-6.3 (the API tokens table; People left the admin shell in #448). Visual: take before-and-after captures. | Typography (section titles), Border Radius, Layout (form width), Catalog (lists) | Done. Workspace settings is one form, and Plugins has its own page in the Workspace subnavigation (`/admin/plugins`). Left: folding Project detail's team past about ten people; Invoice preparation's Projects and Preview tables keep their titles above the table instead of a card; the Tasks groups wait for the Tasks PR; New Project's uppercase fieldset legends |
| PR-C | What is left of refreshing and paging. First, reloads keep their data on the screens whose own PR has not done it (Clients, Reports and the scoped reports): load only the first time (`read().is_none()`), or `use_loader` per list, instead of checking `state() != Ready`. Then one `visibilitychange` listener in `AppLayout` that the screens' resources follow; Project detail without "Refresh details"; "Load more" for Harvest runs and the scoped detailed report, whose "Refresh report" and "Refresh access" go; the copy that points to a refresh button. Covers P-1.7, P-4.4, P-4.5, P-4.7 and P-4.8. | Interaction Principles, Data Tables, Screen States (Loading) | Not started. After PR-5 and the Tasks PR |
| PR-A | Audit log redesign: no "Refresh history"; one "Load more"; `FilterBar` (change type, person, period) kept in the URL; times in the viewer's time zone; events grouped by day as readable sentences with a "+2 −1" summary and a kind badge; the expanded detail as the Your access table limited to the changed areas, changed cells highlighted, plus the receipt id with Copy; names link to the person, the project, or the log filtered to the actor; its own history icon, the filtered empty state and a skeleton of grouped rows; the screen keeps its data on reload. Covers P-4.3 and P-7.1 (audit log). | Audit log | Done (#450). A person's name links to the log filtered to them, as people have no page; the sentence names the kind of change and leaves the moved permissions to the detail, which a translatable sentence cannot list |
| PR-A 2 | Server work, its own PR: an optional reason on a permission change, CSV export, "Restore this version" for custom profiles. | Audit log | Not started |
| PR-I1 | Invoices, step 1: redesign the screens with what exists and add five small features. List with a summary (outstanding, overdue, clients ready to invoice), `FilterBar` (status, client, period, search), one table whose row links to the invoice, status `Badge`s, "New invoice" and a "More" menu in the header; New invoice in two steps (client, then period, projects, values and a preview that follows the form); the draft editor as its own screen with sticky totals; the detail with its Primary by status and "More" (Edit draft, Duplicate, "Void…"). Additions: the unapproved-time warning in step 2, Overdue computed from the due date, the Uninvoiced view, an optional Subject (migration 0005, editor, detail and PDF), Duplicate. | Invoices | Done (#453). "Paid this month" waits for PR-I3, which brings the paid date |
| PR-I2 | Invoices, step 2: group tracked time into lines (one per entry as today, or by task, person or project) when creating a draft. | Invoices | Not started. After PR-I1 |
| PR-I3 | Invoices, step 3: payments, partial or full, with Paid derived from them (and the "Paid this month" summary); manual lines and editing time lines in a draft; the invoice's history. | Invoices | Not started. After PR-I2 |
| Tasks PR | What is left after PR-8 (the row's "Actions" menu and the status `Badge`, P-2.2 and P-10.2) and PR-5 (the `FilterBar` and search, P-3.1): Edit opens a dialog; `BulkBar` at the bottom for bulk selection; an "Assigned by default" column; the whole list, with no Previous/Next or "Refresh tasks"; `PeoplePicker` for default assignees; `ExportDialog` for exports; 16px radius; the screen keeps its data on reload. Covers P-4.1. | Tasks, Catalog | Not started. After PR-5 |
| PR-T | Copy for users: no environment variables, directories, file extensions or test cases in the interface; the `HORAE_HARVEST_*` variables on the Harvest OAuth screen fold under a collapsed "For the operator"; the Harvest empty history says how to start; "Not set" instead of "Unknown currency 0.00"; plain estimate copy in invoice preparation; the Tasks section title; optionally, the build labels moved into the account menu. Covers P-7.5, P-8.2, P-8.3, P-8.5, P-8.7 and P-8.9. | Glossary (copy) | Not started. With or before PR-10. The Unknown currency fix (see [Other work](#other-work-merged-queued-or-in-progress)) takes P-8.3 first: "currency not set" after the amount, and a zero rate as no rate |

C3 of the component study (`Button` takes native attributes and a text label) is
done (#441).

A follow-up of PR-3, in its own PR and not numbered yet:

- **Editor navigation guard asks in `ConfirmDialog`.** `assets/js/project-edit-navigation.js`
  still uses the browser's `confirm()` and `alert()` when history navigation would
  leave an editor (project, client, invoice, timesheet, permissions) with unsaved
  changes or a pending request, because it decides synchronously. Block the
  navigation, open a `ConfirmDialog` and replay the navigation on confirm, then
  rewrite the browser suites that answer the native dialog. It is the one
  exception `tests/dialog_anatomy.rs` allows.

Two follow-ups of PR-1, each in its own PR and not numbered yet:

- **Sign-in page follows the theme.** `crates/api/src/auth/page.rs` loads `horae.css` (its hashed URL passed through `LoginAssets`) and the theme script, instead of copying the dark palette by hand.
- **Merge `--color-text-muted` and `--color-label`.** At 4.5:1 they are almost the same tone (1.05:1 between them in the dark theme, 1.03:1 in the light), so one token replaces both. Pending.

Recommended order for what remains:

1. PR-S, before PR-5, because the radius and width change every capture
1. PR-5 (`FilterBar` on the remaining lists), then the Tasks PR
1. PR-C, once PR-5 has given Tasks a search
1. PR-6
1. PR-T and PR-10, last, so their copy changes do not conflict with the others

PR-People 2, PR-A 2, Edit details, the navigation guard, the i18n increments and
PR-8's leftovers (the Timesheet's `.ts-addrow` and `.ts-del`, Client detail's
project table as `DataTable`, Projects' glyph buttons) depend on none of these
and can go in between. PR-8's `.ts-pager-btn` goes with PR-6.

Each finding belongs to the first PR in this order that does it; a later row says
what is left. Each screen PR (PR-People, the Tasks PR, PR-A) makes its own screen
keep its data on reload, and PR-C does it for the remaining screens before it adds
the automatic refresh.

The mobile part of PR-7 waits for its design.
The browser suites in `crates/horae/tests/browser/` (`shared-style-audit.cjs`,
`responsive-layout.cjs`, `modals.cjs`) check classes and measurements, so each PR
records the `shared-style-audit` baseline at its starting commit and compares
against it.

## Screens still to build from the canvas

The canvas has designs for screens that the rows above do not cover yet: Clients,
Projects, Timesheet, Approvals, Reports, and Invoices beyond step 1 (PR-I2 and
PR-I3 add the features, and the screens they need follow the canvas). Each gets
its own PR, after the pattern PRs that touch it (PR-S, PR-5, PR-6).

## Other work merged, queued or in progress

Not design work, but it changes the screens above or has to land before them.

| Work | PR or branch | Status |
| --- | --- | --- |
| Spec 025, importing Harvest invoices | #452 | Specified |
| Harvest invoice import (spec 025) | `feat/harvest-invoice-import` | In progress. Lands after PR-I1 |
| Spec 024, OAuth 2.0 with PKCE for `/api/v1` and the Harvest-compatible API | #455 | Specified |
| OAuth PR A and PR B (spec 024) | `feat/oauth-foundation` | In progress |
| Spec 021 step 4: page and sync the Harvest API's lists as Harvest does | #454 (`feat/harvest-api-pagination`) | In the merge queue |
| Spec 021 step 5, the spec update | `spec/harvest-api-step5` | In progress |
| Unknown currency fix, PR 1: a zero rate counts as no rate, and a missing currency reads "currency not set" instead of "Unknown currency 0.00" (P-8.3) | `fix/unknown-currency` | In progress |
| Spec 026, task rate templates: templates have no currency and two decimals, and take the project's currency; each line rounds half up, and totals are the sum of the lines | `spec/task-rate-templates` | Being specified. Implementation after the Harvest invoice import and the deploy, near the Tasks PR |
| Spec 027, Harvest mirror mode: Horae reads Harvest every few minutes and keeps a read-only copy, nightly parity checks compare records, report totals and invoices, and the durable queue moves into its own crate (`horae-jobs`) | `spec/harvest-mirror` | Specified. Ten PRs, the jobs crate first; the mirror is turned on only after spec 026's second delivery. Adds a "Synced from Harvest" mark and a mirror status page in the Harvest section |

## Findings cited

The coherence study's findings that the rows above cite, as measured on master `d9791da4`:

| Id | Screen | Finding |
| --- | --- | --- |
| P-1.1 | People | "Add person" (Primary) sits in the body, under the header |
| P-1.7 | Project detail | "Refresh details" as a Ghost header action |
| P-1.10 | Timesheet | `.ts-header` copies `.page-header`; its `Segmented` is 26px in a header of 44px buttons |
| P-2.2 | Tasks | A loose "Edit" button per row; archiving lives in the editor |
| P-2.3 | People | "Rates", "Permissions" and "Archive" stacked per row, overflowing the table at 1280px |
| P-3.1 | Tasks, People | A full-width native "Show" `<select>` and no search |
| P-4.1 | Tasks | "Refresh tasks" and Previous/Next |
| P-4.2 | People | "Refresh" and Previous/Next |
| P-4.3 | Audit log | "Refresh history", and "Newest events"/"Older events" even when empty |
| P-4.4 | Harvest runs | "Older runs" as a link instead of "Load more" |
| P-4.5 | Project detail | "Refresh details" |
| P-4.6 | My settings | "Refresh permissions" |
| P-4.7 | Reports (scoped) | "Refresh report", "Refresh access" and Previous/Next |
| P-4.8 | Tasks editor, Harvest | Copy that tells the user to press a refresh button |
| P-5.1 | My settings | "Your permissions" outside a card, with a 24/600 heading |
| P-5.2 | Project detail | "Tags" as a bare 14/700 heading; tasks and team as bulleted lists |
| P-5.3 | Client detail | Section titles at 18/600 instead of `card-title` |
| P-5.4 | Harvest | `.himp-title` headings and "History" outside a card |
| P-5.5 | Harvest OAuth | A 14/600 heading and uppercase field labels |
| P-5.6 | New project | Section titles at 20/600 |
| P-5.7 | Invoice preparation | `FormCard` title in 14/700 capitals |
| P-5.8 | All | Panel radii of 11, 16 and 20px on the same screen |
| P-6.1 | My settings | Forms 952px wide (Theme at 902px, the token name at 820px) |
| P-6.2 | Workspace, Invoice settings | Each form card sets its own width (about 510 and 540px) |
| P-6.3 | Admin shell | `.adm-main { max-width: 760px }` cuts the People and API tokens tables |
| P-6.4 | My settings | The management `dl` spreads its two figures 470px apart |
| P-6.5 | My settings | The token form: full-width name, misaligned "Expires", the button alone below |
| P-7.1 | Plugins, token lists, audit log | Five empty states share the `settings` icon |
| P-7.2 | 20 routes | Seven different titles for a missing permission |
| P-7.3 | Tasks (member) | Filter, "Refresh" and pager around the access error |
| P-7.4 | New/Edit project (member) | A load failure shown for something the member may not do |
| P-7.5 | Harvest | "Start a preview" in the empty history before a preview is possible |
| P-8.1 | My settings | "Drop .wasm files into the plugins/ directory." |
| P-8.2 | Harvest OAuth | `HORAE_HARVEST_*` variables in the interface |
| P-8.3 | Tasks | "Unknown currency 0.00" |
| P-8.4 | Tasks, audit log (member) | "If you changed accounts, reopen this page." |
| P-8.5 | Invoice preparation | Specification wording about estimates and charges |
| P-8.6 | My settings | Specification notes under the permissions |
| P-8.7 | Tasks | An "Other tasks" section with no other section visible |
| P-8.9 | Sidebar | Server and front-end build labels shown to administrators |
| P-9.1 | My settings | The amber "Only an Administrator can edit your permissions" notice, shown to administrators too |
| P-9.2 | My settings | Permissions shown twice: a disabled field in Profile and the section |
| P-9.3 | My settings | "Administrator access" as a loose bold paragraph |
| P-9.4 | My settings | About 50 permissions in an ungrouped bulleted list (superseded) |
| P-9.5 | My settings | "View permission audit log" as `a { href }` |
| P-9.6 | Sidebar | Invoices and Approvals shown to members who cannot open them |
| P-10.2 | Tasks | Status "Active" as plain text |

## Open questions

One decision is under review and is marked *Pending* in `DESIGN.md`:

- **Tables on mobile** (blocks the mobile part of PR-7). The prototypes stop at
  900px. The direction under review is stacked cards for list tables and
  horizontal scroll with an indicator for wide data tables; the stacked card and
  the Timesheet at 390px need designing first.

Resolved:

- **Dialog footer order and alignment** (question 4). The primary action comes
  first, on the left, then Cancel; the buttons stretch to the footer's width.
  `Modal` draws every footer from its props, so no page can order it by hand
  (PR-3).

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
- **`PeriodNav` waits for a second consumer.** It was extracted in #448 (Timesheet, People); Approvals adopts it in PR-6.
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

The invoice plan agreed on 2026-11-22 has three steps: PR-I1 (the screens and
small additions), PR-I2 (grouping time into lines) and PR-I3 (payments with
Paid derived from them, manual lines and editing time lines, history). Left out
for now: expenses, recurring invoices, retainers, online payment, UBL and
sending invoices by email.

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
