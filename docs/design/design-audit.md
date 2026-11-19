# Interface design audit

A read-only study of `master` `3b08dd25` (branch `study/design-audit`), dated 18 November 2026.

## Summary

The interface is not broken: no screen overflows horizontally at 360, 768 or 1280px, no button lacks
an accessible name, and almost all of the CSS uses tokens. The inconsistencies come from each spec
solving the same patterns on its own. The screens rebuilt from the new handoff (Clients, Client
detail, Projects, Project detail, New/Edit project) sit next to the older ones (Timesheet, Approvals,
Reports, Invoices, Tasks, People, Settings, Importers), and the difference shows at first glance:

- **Four page title sizes** (24/500, 24/600, 34/600 and 32/700).
- **Four modal anatomies**, and four modals with no padding.
- **Three `.empty-state` variants** and about ten empty states made of loose text.
- **About 45 loading messages** in five different styles. There is no skeleton and no error state with a retry.
- **Five ways to pick a value.** A custom menu, `Combobox`, `SelectField`, `Select` and the native
  `<select>`, sometimes for the same kind of filter.
- **Three duration formats** (`13:00`, `7.00` and `13h`) and two date formats (`16 Nov 2026` and `11/01/2026`).
- **Wide tables with no mobile alternative.** Even at 1280px the timesheet's *Total* column and
  Projects' *Budget remaining* fall out of view.
- **Too little contrast in the faintest text.** Table headers at 2.98:1, *muted* in the light theme at
  2.6:1 and placeholders at 2.55:1 (1.62:1 in the light theme).

Most of it can be fixed one pattern at a time, with a shared component or class and a pass over the
screens. The proposed PRs are at the end. Ten decisions are yours to make (section *Questions*).
Where two styles coexist I have not picked either.

### Method

- **References, read in full:** `DESIGN.md`, `horae.css`, `horae-utils.css` (skimmed only),
  `design/README.md`, `design/IMPLEMENTATION-GAPS.md`, the 32 `*.dc.html` files (with the lines of
  `Components.dc.html` longer than 2,000 characters split so that none was cut off) and the
  `design-implement` skill.
- **Code:** pattern searches with `rg` across `crates/horae/src/{pages,components}` (about 25,000
  lines), plus reading the files involved.
- **Screenshots:** 156 PNGs in `audit/screenshots/`, named `<screen>-<width>-<theme>.png`, covering 26
  screens at 360/768/1280px in the dark and light themes, plus the login page. They come from an
  isolated server, migrated and loaded with the demo seed, started the same way as
  `run-design-checks.sh`. The capture script also measures overflow, titles, controls without a name
  or label, radii and font sizes. The results are in `.data/audit/survey.json` (not versioned).
- **Contrast:** computed with the WCAG formula from the token values.

## Patterns by impact

Impact is the number of screens affected, weighed by how visible the problem is (H = seen as soon as
you open the screen, M = while using it, L = in specific cases or in the code).

| # | Pattern | Screens | Visibility | PR |
|---|---|---|---|---|
| 1 | Page header: title, back and actions | 26 | H | PR-2 |
| 2 | Empty, loading and error states | ~20 | H | PR-4 |
| 3 | Wide tables and mobile | 9 | H | PR-7 |
| 4 | Filters and value pickers | 8 | H | PR-5 |
| 5 | Duration and date format | 7 | H | PR-6 |
| 6 | Contrast and focus | all | M | PR-1 |
| 7 | Modals and confirmations | 12 dialogs | M | PR-3 |
| 8 | Buttons: variants, sizes and row actions | ~15 | M | PR-8 |
| 9 | Success notices | 8 | M | PR-9 |
| 10 | Copy and tone | ~12 | M | PR-10 |
| 11 | Values outside the tokens | 5 | L (half of it shows in the light theme) | PR-1 |
| 12 | Period picker | 4 | M | PR-6 |
| 13 | Screens without a prototype (D-013) | 9 | M | follows from PR-2 to PR-5 |
| 14 | Outdated documentation | — | L | PR-0 |

______________________________________________________________________

### 1. Page header (title, back and actions)

**What is wrong.** The page title has four looks, depending on which spec last touched the screen
(measured in the browser at 1280px):

| Look | Screens |
|---|---|
| `.page-title`, 24px / 500, `--color-text` | Timesheet, Approvals, Reports, Invoices, Invoice settings, People, Workspace settings, Importers, Harvest, CSV, Runs, Settings, Components |
| `page-title text-4xl font-semibold text-strong tracking-tight`, 34px / 600 | Clients, Client detail, Projects, Project detail |
| `text-4xl font-semibold text-strong`, without `.page-title` or `tracking-tight` | New project, Edit project |
| `text-2xl font-semibold`, 24px / 600 | Tasks, Audit log |
| 404 `h1`, 32px / 700 | 404 |

All 14 prototypes use a single one: Newsreader 34/600, −0.015em, `#F6F2E9`. The "back" control varies too:

- `btn btn-ghost` aligned with the text in Project detail and New project, and indented in Client detail.
- `btn btn-secondary` at the top right in Invoice settings.
- A "← Back to Timesheet" text link across the whole admin area.
- In Harvest that link and a breadcrumb (`Importers / Harvest`) appear together.

Header actions are normal size on almost every screen and `btn-sm` in Tasks. In Reports, at 360px,
they align right; on the other screens, left.

**Rules it breaks.**

- DESIGN.md, *Type scale*: `4xl` is "refreshed handoff page heading".
- Kit, *Scale & tokens*: "4xl — page titles · display only".
- Kit, *Back link*: one link on its own row, with a 16px arrow and 14px text, that "replaces a
  breadcrumb; never both".

**Files.**

- `pages/{timesheet,approvals,reports,reports/scoped,invoices,invoices/settings,admin/people,admin/workspace,importers,importers/harvest/*,settings,gallery}.rs`, with `class: "page-title"`.
- `pages/{tasks,permission_audit}.rs`, with `text-2xl`.
- `pages/new_project.rs:149,232,257,648`.
- `pages/projects.rs:483,486`.
- `pages/invoices.rs:140`.
- `pages/invoices/settings.rs:179`.
- `components/admin_shell.rs`.
- `.page-title` in `horae.css:1056`.

**Screenshots.** `approvals-1280-dark.png` against `clients-1280-dark.png`, `tasks-1280-dark.png`,
`invoice-settings-1280-dark.png`, `importer-harvest-1280-dark.png`, and `client-detail-1280-dark.png`
against `project-detail-1280-dark.png` (the indented back control).

**Proposal.** A `PageHeader { title, eyebrow?, back?, actions }` component and a single `.page-title`
definition, at the size you choose (question 1). The back control becomes a `.back-link` class after
the kit, and either the Harvest breadcrumb or the shell's link is removed. Header actions always use
the normal size and wrap the same way on every screen.

### 2. Empty, loading and error states

**What is wrong.**

- **Empty.** There are three `.empty-state` variants:

  - Projects: `bg-secondary rounded-xl` with an icon tile.
  - Clients: `rounded-xl`, a tile and no background of its own.
  - Timesheet and gallery: the base one, with a 🗓 emoji as the icon.

  Everywhere else, the empty state is loose text:

  - Approvals: `card p-8 text-center` + "No approvals found.".
  - Invoices: `text-muted text-sm p-5`.
  - Reports and Detailed time: `p role=status`.
  - The Timesheet day: `ts-day-empty`.
  - Client detail (projects and invoices) and Project detail (an unstyled `<ul>`).
  - Harvest history.

  Almost none of them says what to do next.

- **Loading.** There are about 45 `p { "Loading…" }` scattered around, with five class combinations:
  `text-muted text-sm`, `text-sm text-secondary`, `text-secondary`, no class, or inside `.modal-body`.
  Some have `aria-busy` and others do not. The handoff's skeleton (`Skeleton.dc.html`) does not exist.

- **Error.** `alert alert-danger` is used consistently (about 70 times), but there is no full-page
  error state with "Try again" (`ErrorState.dc.html`), and `.banner-danger`, which DESIGN.md reserves
  for errors with an action, only appears in the gallery. The retry button says "Retry" in Projects,
  Timesheet and Reports, and "Try again" in Harvest.

**Rules it breaks.**

- DESIGN.md, *Screen States*: the empty state is "`.empty-state` with copy that says what to do next".
- Loading states must be "inline, per region… keep the layout stable".
- Errors that need an action go in "`.banner-danger`".
- Kit, *Empty state* and *Error state*: the same silhouette, a 52px disc, a serif title and one action.

**Files.** `pages/approvals.rs:242`, `invoices.rs:149`, `timesheet.rs:1877,2188`,
`clients/detail.rs:240,308`, `projects.rs:910-935,1463,1468`, `reports.rs:387,463`,
`reports/scoped*.rs` and `importers/harvest/overview.rs:227`. The loading messages are in the output
of `rg '"Loading'`. Styles in `horae.css:1082-1104`.

**Screenshots.** `approvals-1280-dark.png`, `invoices-1280-dark.png`, `importer-harvest-1280-dark.png`
("No runs yet…") and `project-detail-1280-dark.png` (bulleted lists).

**Proposal.** Three components: `EmptyState { icon, title, body, action?, link? }`,
`ErrorState { kind, title?, body?, on_retry }` and `Loading { label }` (always with `role=status` and
`aria-busy`). A table `Skeleton` stays optional, for listings. One visual variant of `.empty-state`:
either the Projects one becomes the base or it is removed (question 2, together with the title size).
"Try again" or "Retry": one verb (question 6).

### 3. Wide tables and responsive layout

**What is wrong.**

- **Timesheet, week.** The grid is at least 1,060px wide (`minmax(280px,1fr) repeat(7,92px) 92px 44px`). At 1280px, with the 264px sidebar, the usable space is about 950px, so the *Total* and delete
  columns fall outside it with nothing to show it. At 360px only the project names are visible and no
  day cell is.
- **Projects.** At 1280px the "BUDGET REMAINI…" header is cut off by its container.
- **At 360px.** Clients, Projects, People, Tasks, Approvals, Reports and Invoices scroll horizontally
  inside the card, with no stacked alternative and no sign that there are more columns. Status and
  actions fall out of view.
- **Broken names.** Names break one word per line ("Admin / User" in People, "Acme / Website /
  Redesign" in Reports at 360px).
- **"Timesheet for".** It is a full-width `<select>` stuck to the top edge of the grid, with no gap.

**Rules it breaks.**

- DESIGN.md, *Narrow*: "usable at 390px and at the 768px mobile-navigation breakpoint".
- DESIGN.md, *Layout → Narrow content*: "Dense project/time grids… scroll within their own containers",
  so scrolling is allowed, but losing columns at 1280px is not.
- DESIGN.md, *Long content*.

**Files.** `horae.css:1564-1568` (`.ts-row`) and `1803-1817` (`.proj-grid`), `pages/timesheet.rs`,
`pages/projects.rs`, `pages/clients.rs`, `pages/admin/people.rs`, `pages/tasks.rs` and
`components/table.rs`.

**Screenshots.** `timesheet-week-1280-dark.png`, `timesheet-week-360-dark.png`, `projects-1280-dark.png`,
`projects-360-dark.png`, `clients-360-light.png`, `reports-360-dark.png` and `admin-users-1280-dark.png`.

**Proposal.**

- Timesheet: a project column with `minmax(200px,1fr)` and the total column pinned to the right, so
  that it fits at 1280px.
- Below 768px: a per-day list mode for the Timesheet, or one row per project with the days stacked.
- List tables: either a shared "stacked card" rule below 640px (name, status, main action) or a fade
  at the edge that shows the table scrolls (question 8).
- A name column with a minimum width, as `.client-table` already has (`--width-client-name`).

### 4. Filters and value pickers

**What is wrong.** The same gesture, "filter the list", is solved in four ways:

- Projects and Clients: a custom `Menu` with a `btn-secondary` trigger, aligned right.
- Tasks and People: a full-width native `<select class="form-select">` labelled "Show". In People the
  arrow is the browser's native one, unlike the one `.form-select` draws.
- Reports: a card of labelled selects in a grid.
- Approvals: status with `Segmented`.

To pick a value in forms, `Menu`, `Combobox`, `SelectField`, `form::Select` and a `<select>` without a
component coexist (`reports.rs` ×3, `tasks.rs`, `admin/people.rs`, `admin/add_person.rs`,
`admin/permission_editor.rs`, `components/project_task_picker.rs`). The search field has a magnifier
in Projects and none in Clients.

**Rules it breaks.**

- Kit, *Selects & menus*: "Custom dropdowns (never native `<select>`)".
- Kit, *Filter bar*: "a row of them reads as one instrument".
- Kit, *Search field*: always a magnifier.

**Screenshots.** `projects-1280-dark.png` against `tasks-1280-dark.png`, `admin-users-1280-dark.png`,
`reports-1280-dark.png`, `approvals-1280-dark.png` and `clients-1280-dark.png` (search field without a
magnifier).

**Proposal.**

- A `FilterBar` with `Menu` triggers for every list.
- A shared `SearchField` with a magnifier.
- Decide whether the native `<select>` is allowed in simple forms (question 7). If not, move the eight
  uses to `SelectField`.
- Merge `form::Select` and `SelectField`, or document when each one is used.

### 5. Duration and date format

**What is wrong.**

- **Duration, three formats.** `format_hhmm` gives `13:00` in Timesheet, Approvals and Client detail.
  `format_hours2` gives `7.00` in Reports and Invoices. In Projects, a formatter of its own gives `13h`
  or `0.98h` (`pages/projects.rs:32`). The same project shows "13h" in Projects and "13:00" in Client
  detail.
- **Date, two formats.** The app writes `16 Nov 2026`, but the native `<input type=date>` fields
  (Reports, invoice preparation, New project milestones) show the browser's format: `11/01/2026` in US
  English.

**Rules it breaks.** `IMPLEMENTATION-GAPS.md`, *format.js*: "Agree on H:MM vs decimal-hour
display". It has been pending since September.

**Screenshots.** `projects-1280-dark.png` ("13h"), `client-detail-1280-dark.png` ("13:00") and
`reports-1280-dark.png` ("7.00" and "11/01/2026").

**Proposal.** A single duration formatter in `horae-core`, with the convention you choose
(question 3), and remove the one in `projects.rs`. Replace the `type=date` fields with the shared
`DatePicker` or, at least, with a text field in the app's format.

### 6. Contrast and focus

**What is wrong.**

| Pair | Contrast | AA minimum |
|---|---|---|
| `--color-label` (table headers, 12px) on `--color-bg-secondary`, dark | **2.98** | 4.5 |
| Placeholder (`--color-border-strong`) on `--color-bg`, dark | **2.55** | 4.5 |
| Placeholder on `--color-bg`, light | **1.62** | 4.5 |
| `--color-text-muted` on `--color-bg`, light (loading messages, hints) | **2.60** | 4.5 |
| `--color-text-muted` on `--color-bg-secondary`, light | **2.86** | 4.5 |
| `--color-label` on `--color-bg-secondary`, light | 3.50 | 4.5 |
| Light accent (today) on `--color-bg` | 2.69 | 3 (large text) |
| `--color-text-muted` on `--color-bg-secondary`, dark | 3.88 | 4.5 |

Focus is drawn three ways: `box-shadow: var(--ring)` on buttons, an inset ring on the menu, the pager
and the collapse header, and `outline: 2px` on `.proj-scroll` and the file picker. Links (names in
tables, "View in Projects", "Back to…") have no rule and show the browser's default outline. The
Settings theme `<select>` has no associated label (the survey flags it).

**Rules it breaks.**

- DESIGN.md, *Accessibility*: "Color contrast meets WCAG AA".
- *Screen States → Keyboard*: "visible `--ring` focus".
- Kit, *Focus*: "Ring never replaced by an outline".
- The kit itself warns: "Anything fainter than [text-disabled] fails contrast".

**Files.** Tokens in `horae.css:36-37` (dark) and `205-206` (light), placeholders in `horae.css:942`,
`a {}` in `horae.css:286` and `pages/settings.rs` (the theme select).

**Proposal.** Raise `--color-label` (dark) and `--color-text-muted` (light) to 4.5:1 and use
`--color-text-muted` for placeholders. Add a rule `a:focus-visible, [tabindex]:focus-visible { box-shadow: var(--ring) }`
and the label on the theme select. It is a token change with little code, so it goes first (PR-1).

### 7. Modals and confirmations

**What is wrong.** Four anatomies on top of the same `Modal`:

- **Standard.** A centred `.modal-title` + `.modal-body` + `.modal-actions`, with full-width buttons
  and the main action first. Used by Projects (archive, export), the client archive, the New project
  discard dialogs and tasks.
- **Timesheet duplicate.** `.ts-modal-title`, `.ts-modal-body` and `.ts-modal-actions`, which copy
  `.modal-*` with literal values (`font-size: 18px`).
- **Left-aligned title.** The client editor, with `modal-title text-left px-6 py-4`, and the
  permissions editor, with a left-aligned `text-2xl`.
- **No anatomy.** An unstyled `h2` and paragraphs with no padding, flush against the panel's edge:
  disconnect Harvest (`connection_card.rs:165`), change account (`change_account.rs:60`) and
  over-invoicing, both when editing and when preparing (`invoices/editing.rs:253`,
  `invoices/preparation.rs:386`). In these four, the buttons also sit on the right (`justify-end`)
  with "Cancel" first, or on the left with the dangerous action first.

Confirming something destructive uses `btn-danger` (red border) or `btn-primary` (archiving a client
or project), with no rule. The handoff's solid danger variant (`hb-danger-solid` in
`ConfirmDialog.dc.html`) does not exist.

**Rules it breaks.**

- DESIGN.md: "Build each state from the existing classes".
- Kit, *Modal*: "One anatomy for every dialog… a centred serif title bar… a footer whose primary action
  sits right".
- `ConfirmDialog.dc.html`: a single component for destructive actions, with an icon, a serif title and
  the confirm button first. The handoff contradicts itself: the kit puts the main action on the right
  and almost every screen puts it on the left (question 4).

**Screenshots.** None: these dialogs cannot be opened with the seed, because they need a connected
Harvest account or over-invoicing. The evidence is in the code cited.

**Proposal.** A `ConfirmDialog { title, body, confirm_label, tone, busy, on_confirm, on_dismiss }` on
top of `Modal`, and fixed slots (`ModalHeader`, `ModalBody`, `ModalFooter`) for the rest. Delete
`.ts-modal-*`. Fix the four unpadded modals. Decide the order and alignment of the footer, and whether
danger is a border or a fill (questions 4 and 9).

### 8. Buttons: variants, sizes and row actions

**What is wrong.**

- **The component is hardly used.** `components/button.rs` appears 12 times, against 267 hand-written
  `class: "btn …"`.

- **Form submits do not follow the rule.** DESIGN.md wants `btn-solid` for "form submits", but only
  Invoice settings and approval in Approvals use it. Settings, clients, tasks, people and New project
  submit with `btn-primary`.

- **The timesheet's own buttons.** `ts-add`, `ts-addrow`, `ts-day-action`, `ts-del` and
  `ts-project-icon` repeat `.btn-icon` or `.btn-secondary btn-sm` with their own measurements
  (`border-radius: 9px`, `padding: 10px`).

- **Row actions.**

  - Clients: separate "Edit" and "Archive" buttons.
  - People: "Rates", "Permissions" and "Archive", which wrap onto two lines at 1280px.
  - Tasks: "Edit".
  - Project detail: an "Actions" menu.

  The handoff always uses a compact "Actions" menu per row.

- **Status is not always a badge.** In Tasks the status is plain text ("Active"); in Clients and
  People it is a badge (`badge`).

- **Archiving is offered in two ways.** A header button in Client detail ("Archive client") and a menu
  item in Project detail.

**Rules it breaks.**

- DESIGN.md, *Buttons*: Solid for "form submits".
- Kit, *Control heights*: "Interactive height never falls off these four".
- Kit, *Dropdown*: "compact for table rows".

**Screenshots.** `clients-1280-dark.png`, `admin-users-1280-dark.png`, `tasks-1280-dark.png`,
`invoice-settings-1280-dark.png` and `client-detail-1280-dark.png` against `project-detail-1280-dark.png`.

**Proposal.** Decide the primary/solid rule (question 10) and apply it. Use `Button` or, at the very
least, remove the timesheet's own classes that copy `.btn`. A `RowActions` pattern (a compact menu) for
every table, and status always as a `Badge`.

### 9. Success notices

**What is wrong.** Saving or archiving is confirmed in four ways:

- `Toast`, bottom right: Projects, Clients and Client detail.
- `alert alert-success` inside the page: Projects too (`projects.rs:545,559,849`), Approvals and
  Missing people.
- Inline text: "Saved." in Invoice settings, "Profile saved." in green (`text-success`) in Settings,
  and a grey paragraph with `role=status` in Tasks ("Task archived.") and People ("Permissions saved.",
  rates).

The app's `Toast` sits at the bottom right with a coloured left border. The handoff's sits at the
bottom centre, in black, with an icon and an optional action ("Undo").

**Rules it breaks.** Kit, *Form dirty state*: "Saving confirms with a toast, never silence".
`Toast.dc.html`: "Bottom-centre notice".

**Proposal.** A mutation's success always goes in a `Toast`, through a page notice context
(`ToastContainer` in `AppLayout`), and `.alert-success` stays for persistent states. Decide the
position and style (question 5).

### 10. Copy and tone

**What is wrong.**

- **The opposite of archive.** "Reactivate" for projects and "Restore" for clients, people and tasks.
- **People.** In the sidebar and the subnavigation they are called "Team", and the page is titled
  "People". The subnavigation says "Settings" and the page "Workspace settings".
- **Capitalisation.** "New Invoice" against "New project", "New client" and "New task".
- **Retry.** "Retry", "Retry report", "Retry tags", "Retry people", etc., against "Try again".
- **Ellipses.** "Archive task…" and "Restore task…" have them, and the other actions that open a
  dialog do not.
- **Back.** From the admin area you go back to "Timesheet", not to the screen you came from.
- **Actions that say the same thing in different words.** "Cancel changes" and "Cancel over-invoicing"
  against "Cancel". "Close" against "Dismiss".
- **Empty states with different endings.** "No approvals found." and "No invoices yet. Generate one…"
  end with a period, and the `.empty-state` titles do not.

**Rules it breaks.** DESIGN.md, *Empty*: "copy that says what to do next". The `humanizer` skill
recommends a consistent vocabulary.

**Proposal.** A short glossary in DESIGN.md (verbs, section names and sentence case) and a PR that
applies it (questions 6 and 11).

### 11. Values outside the tokens

**What is wrong.**

- **Literal colours in `horae.css`.**

  - Calendar: `#2b7a66`, `#e6b45f`, `#f6f2e9`, `#4a3208`, `#8fc4b3` and `#6b4a10` in
    `.ts-cal-event` and `.ts-cal-ev-*`.
  - `.ts-submit-caret`: `#164034` and `#0f3d33`.
  - Shadows with `rgba` in `.auth-card`, `.ts-submit` and `.btn-split`.

  In the light theme, the text on today's card loses contrast. The comment on line 1714 ("Design-exact
  darks, not tokens") contradicts the one on the next line, which says the opposite.

- **Font sizes off the scale.** 10.5px (sidebar labels, 102 nodes), 11, 11.5, 12.5, 13 and 14.5px,
  mostly in `.ts-*`. The kit: "no 13.5px type. The scale skips 7, 9, 11 and 13–15".

- **Literal radii.** `11px` (collapsed sidebar), `9px` (`.ts-addrow`) and `6px` (`.ts-cal-event`),
  instead of `--radius-*`.

- **Static `style:` in `rsx!`.** In `pages/invoices.rs:233,276,290` (`margin-left: 0.5rem`, a grid)
  and in `PRE_LINE` in `invoices/settings.rs`. The ones for the calendar and the sidebar width are
  dynamic and are correct.

- **Duplicated CSS.** `.alert-warning` is defined twice (`horae.css:1115` and `1901`), and
  `.ts-modal-*` duplicates `.modal-*` (pattern 7).

- **The login page** (`crates/api/src/auth/page.rs`) copies the dark palette by hand and does not
  respond to the light theme (`login-1280-light.png` comes out dark).

**Rules it breaks.** DESIGN.md, *Tokens & utilities*: "never hardcode a colour, spacing, or radius".
`design-implement` skill: "No inline `style=`, no hardcoded colors".

**Proposal.** Tokens for the calendar event colours in both themes, move the sizes onto the scale,
delete the duplicates and the inline styles. The login page is for you to decide: either it loads
`horae.css` or it is documented as dark only.

### 12. Period picker

**What is wrong.**

- **Timesheet.** `ts-pager` with ←/→ arrows as text, a ▦ icon and `DatePicker`.
- **Approvals.** Reuses the `ts-pager-*` classes with links, and has no icon or calendar.
- **Reports and Invoice preparation.** Use two native `type=date` fields.
- **Project detail.** Has no period, although the prototype does.

The handoff has a single `PeriodNav` (with `caret-left/right` and a calendar), "Used in Timesheet,
Approvals, Reports and Project Detail".

**Screenshots.** `timesheet-week-1280-dark.png`, `approvals-1280-dark.png` and `reports-1280-dark.png`.

**Proposal.** Extract `PeriodNav` from the Timesheet into `components/` (`IMPLEMENTATION-GAPS.md`
already made that depend on a second consumer, and Approvals is one) and use it in Approvals. Reports
needs a custom range mode: that is product work, not only visual work.

### 13. Screens without a prototype (D-013)

D-013 (`specs/harvest-parity.md:101`) allows screens without a mockup to be composed "from current
Horae components and tokens". Current state:

| Screen | Current components? | Deviations |
|---|---|---|
| Invoices (list) | yes | old title; empty state as text; "New Invoice" |
| Invoice detail | yes | static `style:` (`invoices.rs:233,276,290`) |
| Invoice settings | yes | back as a button at the top right; inline "Saved."; `btn-solid` submit |
| Tasks | partly | `text-2xl` title; full-width native filter; `btn-sm` buttons; status without a badge |
| Audit log | partly | `text-2xl` title |
| People, rates, permissions | partly | native filter; row actions that wrap; permissions modal with its own title |
| Settings | partly | a prototype exists (`08_Settings`), but the page does not use the profile subnavigation: stacked cards with a 50-item bulleted list |
| Harvest runs and run view | yes | back link and breadcrumb together; empty states as text |
| 404 | no | its own card outside the shell, with no navigation; 32/700 `h1` |

None of them needs a new prototype: PR-2 to PR-5 fix them along the way.

### 14. Outdated documentation

- **DESIGN.md.**
  - The *Pages* table does not match the current routes: it cites `/auth/register`, `Dashboard`,
    `/time` and `AdminUsers`, and is missing Tasks, Approvals, Reports, Audit and Importers.
  - It cites `design/project/dark.dc.html`, which does not exist.
  - It says the active item's pine dot is "leading", when the CSS puts it after the label.
- **`horae.css`.** The `--radius-lg` comment says "cards, tables", but tables use `--radius-panel`.
- **`design/IMPLEMENTATION-GAPS.md`.** It still says that Client detail "only renders `Client detail for {id}`" and that the project panel is a placeholder. Both exist now.

**Proposal.** PR-0, documentation only, to update those tables and link this audit.

______________________________________________________________________

## Proposed PRs

Each PR fixes one pattern across every screen at once and carries before-and-after screenshots from
the same runner. Approximate sizes: S < 150 lines, M 150–500, L > 500.

| Order | PR | Patterns | Size | Depends on | In parallel with |
|---|---|---|---|---|---|
| 0 | Docs: DESIGN.md (pages, links) and IMPLEMENTATION-GAPS brought up to date | 14 | S | — | everything |
| 1 | Tokens: label, muted and placeholder contrast; link focus; calendar colours; off-scale sizes; duplicated CSS | 6, 11 | M | — | 0, 3, 6 |
| 2 | `PageHeader` + `.back-link`: one title and one back link on all 26 screens | 1, 13 | M | question 1 | 3, 6 |
| 3 | `ConfirmDialog` + `Modal` slots; delete `.ts-modal-*`; fix the 4 unpadded modals | 7 | M | questions 4 and 9 | 1, 2, 6 |
| 4 | `EmptyState`, `ErrorState` and `Loading` on the ~20 screens | 2, 13 | L (mechanical) | 2, for the vertical rhythm | 5 |
| 5 | `FilterBar`, `SearchField` and moving off the native `<select>` | 4 | M | question 7 | 4 |
| 6 | One duration formatter; shared `PeriodNav` in Approvals; `type=date` out wherever there is a `DatePicker` | 5, 12 | M | question 3 | 1, 2, 3 |
| 7 | Tables: a timesheet that fits at 1280px, stacked mode or a scroll indicator below 640px, a minimum name column | 3 | L | 5 (filters above) and question 8 | 8 |
| 8 | `RowActions` in tables, `Badge` for status, the primary/solid rule and the `ts-*` classes that copy `.btn` removed | 8 | M | question 10 | 7 |
| 9 | Success notices unified in `Toast` from `AppLayout` | 9 | S–M | question 5 | 7, 8 |
| 10 | Copy glossary in DESIGN.md and a copy pass | 10 | S | questions 6 and 11 | any; best last to avoid conflicts |

**To start now:** PR-0 and PR-1 depend on no question and remove the accessibility problems. Then
PR-2 and PR-3 in parallel, because they change what is most visible.

**Shared risk.** The suites in `crates/horae/tests/browser/*.cjs` (especially `shared-style-audit.cjs`,
`responsive-layout.cjs` and `modals.cjs`) check classes and measurements. Each PR must record the
`shared-style-audit` baseline at its starting commit and compare against it, as the specs require.

## Questions for you

1. **Page title.** Newsreader 34/600 (every prototype and the new screens) on every screen, or 24/500
   (most of the current screens)? DESIGN.md today leaves it as a per-screen option.
1. **Empty state.** Does the Projects version (a `bg-secondary` card, radius 16 and an icon tile) become
   the base of `.empty-state`, or does the current base stay (sunken, radius 11, a 46px disc)?
1. **Duration.** `H:MM` everywhere, decimal (`7.00`) everywhere, or `H:MM` in time entry and decimal in
   reports and invoices? Or a per-user preference, as Harvest does?
1. **Dialog footer.** Main action on the right with "Cancel" on the left (kit, *Modal*), or the main
   action first, on the left (ConfirmDialog and most of the handoff's screens, and the app today, full
   width)?
1. **Toast.** Bottom centre in black, with an "Undo" action (handoff), or bottom right with a coloured
   border (app)?
1. **Retry.** "Try again" or "Retry"?
1. **Native `<select>`.** Allowed in simple forms (time zone, theme, "Show"), or always a custom
   dropdown, as the kit says?
1. **Tables on mobile.** A row stacked as a card, or horizontal scroll with a visible indicator?
1. **Destructive confirmation.** A solid red button (handoff) or the current red border (`btn-danger`)?
   And does archiving count as destructive, given that today it is sometimes `btn-primary` and
   sometimes `btn-danger`?
1. **Form submits.** Pine `btn-solid` for every submit, as DESIGN.md says, or `btn-primary`, as almost
   every screen does? If it is the primary, DESIGN.md has to change.
1. **Names.** "Team" or "People" for people? "Restore" or "Reactivate" as the opposite of archive?
   "Settings" or "Workspace settings"?
