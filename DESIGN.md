# Horae Design Language

Horae follows the Invoicer design system: editorial warm-dark surfaces, serif display type, humanist sans-serif for UI, and monospace numerals — an accountancy aesthetic that suits time-tracking and invoicing.

## Sources and precedence

This file is the single source of truth for Horae's design. When it disagrees with anything else, this file wins.

- **`DESIGN.md`** (this file) holds the rules: tokens, components, screen states, layout and wording. A rule here overrides the kit and the prototypes, and the sections below say so wherever they differ.
- **`crates/horae/assets/css/horae.css`** implements the tokens and component classes. Where the CSS and this file disagree, the CSS is out of date and the PR that touches it brings it in line.
- **`design/project/`** is the claude.ai/design export of 2026-09-21: the kit (`foundations/Components.dc.html`), the shared prototype components (`app/*.dc.html`) and the screen prototypes (`app/01_*` to `13_*`). It is a historical visual reference. Use it for layout, proportions and copy ideas, never as a rule that overrides this file, and never edit it (see [design/README.md](design/README.md)).
- **Decisions on open questions** were taken after the [design audit of 2026-11-18](docs/design/design-audit.md). Each one is written into the section it governs, with a line on why. One is still under review and is marked *Pending* where it applies: tables on mobile ([Layout](#layout)).
- **Decisions of 2026-11-20** came from two screen-by-screen studies of master `d9791da4`, one on coherence and one on loading. Their rules are written into the sections they govern, like the audit's, and the screens they redesign have their own subsections under [Pages](#pages).
- [`design/IMPLEMENTATION-GAPS.md`](design/IMPLEMENTATION-GAPS.md) tracks what the application still has to change to match this file, and which PR does it.

## Color Palette

All colors are CSS custom properties in `assets/css/horae.css`.

### Surface layers (darkest → lightest)

| Token | Value | Usage |
|---|---|---|
| `--color-bg` | `#100F0C` | Page background (void) |
| `--color-bg-secondary` | `#1A1813` | Cards, nav bar, sidebar |
| `--color-bg-tertiary` | `#232019` | Table cells, raised chips |
| `--color-bg-overlay` | `#2E2A22` | Hover-raise over a raised surface |
| `--color-menu` | `#201D17` | Dropdown / popover surface (deeper than overlay) |
| `--color-row-hover` | `#1C1A15` | Table / ledger row hover (sinks) |

### Text

| Token | Value | Usage |
|---|---|---|
| `--color-text-strong` | `#F6F2E9` | Headings, wordmark |
| `--color-text` | `#EFEAE0` | Primary body text |
| `--color-text-secondary` | `#A29C8D` | Labels, captions, sidebar links |
| `--color-text-muted` | `#918B7B` | Placeholders, section headers |
| `--color-label` | `#8D8777` | Uppercase micro-labels: table headers, sidebar section labels, `MetricCard` labels |

The light theme sets muted to `#6E675A` and label to `#70695B`. Both tokens stay at 4.5:1 or more on `--color-bg`, `--color-bg-secondary`, `--color-bg-tertiary` and `--color-menu` in each theme, which leaves little room between them: they now differ mostly in use (case and tracking), not in tone, and a later PR merges them (see the inventory).

### Borders

| Token | Value | Usage |
|---|---|---|
| `--color-border` | `#322E26` | Card borders, nav border, table edges |
| `--color-border-light` | `#262219` | Table row dividers |
| `--color-border-strong` | `#5A5446` | Input hover, checkbox edge, control-lift |

### Accent

| Token | Value | Usage |
|---|---|---|
| `--color-primary` | `#4FB79A` | Pine-300 — logo, active links, timer, focus ring |
| `--color-primary-hover` | `#63C9AC` | Brand button hover — brightens (with a soft glow) |
| `--color-primary-light` | `#6ECAB0` | Active sidebar link text |
| `--color-primary-bg` | `rgba(79,183,154,0.14)` | Active sidebar link background |
| `--color-accent` | `#D99A3C` | Brass — invoicing context, send/bill actions |
| `--color-accent-hover` | `#E5AB52` | Brass button hover — brightens |
| `--color-accent-bg` | `rgba(217,154,60,0.14)` | Brass tint background |
| `--color-pine` | `#1F5C4D` | Deep pine — solid fills, marks, selected states |

### Semantic (warm dark-tuned)

| Token | Value |
|---|---|
| `--color-success` | `#3FB489` |
| `--color-warning` | `#D6A24A` |
| `--color-danger` | `#E06661` |
| `--color-info` | `#4FB79A` (reuses pine-300) |

Semantic backgrounds use solid dark tints (`--color-success-bg: #15291F`, etc.) rather than rgba.

## Typography

Three typefaces loaded from Google Fonts:

```
@import url('https://fonts.googleapis.com/css2?family=Newsreader:ital,opsz,wght@0,6..72,400;0,6..72,500;0,6..72,600;1,6..72,400&family=Instrument+Sans:wght@400;500;600;700&family=IBM+Plex+Mono:wght@400;500;600&display=swap');
```

| Role | Family | Token | Usage |
|---|---|---|---|
| Display / headings | Newsreader (serif) | `--font-family-display` | `h1–h6`, brand wordmark, card titles, page titles, auth logo |
| UI / body | Instrument Sans (humanist sans) | `--font-family` | Body, labels, buttons, nav links, form inputs |
| Numerals | IBM Plex Mono | `--font-family-mono` | Timer, stat values, amounts, hours, rates, dates |

### Type scale

Sizes are `--font-size-*` tokens with matching `text-*` utilities: `xs` 12 · `sm` 14 (body, numerals) · `base` 16 · `lg` 18 (H2) · `xl` 20 (empty- and error-state titles) · `2xl` 24 · `3xl` 30 (H1) · `4xl` 34 (page titles) · `display` 44 (hero totals, e.g. invoice "Total due"). Headings resolve to `--color-text-strong`. The kit's text specimen (27, 15 and 11 px) sits off this scale; the scale here wins.

**Section titles.** Inside a screen, either every section is a card or none is, and a section's title is always `card-title` (Newsreader 18/500). Uppercase micro-labels are only for table headers, the sidebar's section labels and the label of a `MetricCard`, whose figure is the heading. Today section titles come in seven styles (18/500, 18/600, 20/500, 20/600, 24/600, 14/600 and 14/700 in capitals), and My settings, Project detail and Harvest mix sections in cards with sections outside them. New Project keeps its own layout (see [New Project](#new-project)), but its section titles use the same class.

**Page title.** Every screen's title is Newsreader 34/600 (`4xl`) with −0.015em tracking (`tracking-tight`), in `--color-text-strong`, rendered by `PageHeader` (see [Page header](#page-header-srccomponents)). This is the size every prototype uses, so one title reads the same on every screen. The 24/500 `.page-title`, the `text-2xl` titles and the 404's 32/700 heading are retired. The sign-in shell keeps its own layout but uses the same 34/600 heading.

## Border Radius

| Token | Value | Usage |
|---|---|---|
| `--radius-sm` | `4px` | Micro elements |
| `--radius` | `6px` | Menu items, segmented inner, small controls |
| `--radius-btn` | `8px` | Buttons, inputs |
| `--radius-lg` | `11px` | Menus / popovers |
| `--radius-icon` | `12px` | Empty- and error-state icon tiles, dialog icon tiles, toasts |
| `--radius-xl` | `16px` | Panels: tables, cards, grouped lists, `Skeleton`, `EmptyState` and `ErrorState` |
| `--radius-panel` | `20px` | Modals, auth card |
| `--radius-full` | `9999px` | Badge pills, toggles |

**One radius for panels.** Tables, cards, `Skeleton`, `EmptyState` and `ErrorState` all use `--radius-xl` (16px). A 20px corner looks too round on a 300px card, and an 11px card beside a 20px table gives one screen two looks. `--radius-lg` stays for menus, popovers, toasts and notices (`.banner`), and `--radius-panel` for `Modal`.

The kit draws modals and `ConfirmDialog` at 16px; Horae's dialogs use `--radius-panel`, as the kit's own token table says.

## Layout

- **Sidebar** (`--sidebar-width: 264px`): full-height left rail on the warm dark surface (`--color-bg-secondary`), 1px right border, sticky. Holds the brand, a start-timer action, grouped navigation, and the signed-in user; there is no separate top nav. Collapses to a 68px icon strip.
- **Content area**: scrollable, generous padding, `--color-bg` (`#100F0C`) background.
- **Mobile navigation** (up to 768px): a labelled toggle expands the same sidebar above the content. It stays in normal document flow, so short screens can scroll to every link. Selecting a link closes the panel and focuses the content; Escape closes it and returns focus to the toggle. Desktop width and collapse state are preserved independently.
- **Narrow content**: page headers and actions wrap; the admin subnavigation moves above its panel when the two columns no longer fit. Dense project/time grids and the day-total strip scroll within their own containers. Project names and badges wrap inside an aligned name column instead of overlapping budget figures.
- **Wide tables at desktop width**: at 1280px with the expanded sidebar, every column of a table or grid is visible without scrolling. The Timesheet week (name, seven days, Total and the delete column) fits in that space, and so do all of Projects' columns. The name column has a minimum width, so names wrap inside it instead of breaking one word per line: `--width-name-column` for the Timesheet, Projects and any table marked `.name-table` (People, Tasks, Approvals, Reports, Clients and Client detail). In Projects the type badge moves below the name when both do not fit on one line. Below that width the grid or table scrolls inside its own container.
- **Form width**: a form, or a form card, is at most `--width-form` (720px) wide and aligned left. Lists and tables use the full width of the content area, including the tables on a screen that also has forms (My settings). A single-field form at 952px is hard to scan. The utility is `max-w-form`; `FormCard` applies it. The admin shell sets no width of its own, so the content sets it.
- **Tables on mobile: *Pending design*.** Neither this file nor the prototypes (which stop at 900px) cover phones. The direction under review is a mixed rule: list tables stack each row as a card, and wide data tables (the Timesheet week, reports) keep horizontal scrolling with a visible indicator that more columns exist. The stacked card and the Timesheet at 390px still need designing, so do not build either until this note is replaced by a rule.

## Component Inventory

Every shared component lives in `crates/horae/src/components/`, one family per file (see [Component Conventions](#component-conventions)), declared in `components.rs`. File names below are relative to that directory; the paths in the rule headings further down (`src/components/…`) are relative to `crates/horae/`. The catalog says which component to reach for. The subsections after it (from [Sidebar](#sidebar-srccomponentssidebarrs) to [Empty, loading and error states](#empty-loading-and-error-states-srccomponents)) describe how each family must look and behave. Where the code does not match those rules yet, the status column names the PR that closes the gap, and [`design/IMPLEMENTATION-GAPS.md`](design/IMPLEMENTATION-GAPS.md) tracks it.

Status:

- **in use**: screens call it, and any hand-written copies of its markup are fewer than the calls.
- **underused**: screens call it, but write the same markup by hand more often. The figure is calls outside `components/` and the gallery (`pages/gallery.rs`) against hand-written copies, counted on 2026-11-19. Calls are exact; hand-written copies are approximate, since they depend on the pattern counted (a class, an element, a wrapper). In new code, call the component.
- **kept, not yet used**: no screen calls it yet (only the gallery does), and it stays because screens are going to use it. When a screen needs what it draws, call it instead of writing the markup by hand. The figures count the same way.
- **retired**: do not add calls; its callers move in the PR named.
- **planned in PR-N**: the rule exists and the component does not. Build the state by hand, as the rule describes, until that PR lands, and do not invent its API.

### Summary

| Component | File | Family | Status |
|---|---|---|---|
| `AppLayout` | `layout.rs` | Shell and navigation | in use (route layout) |
| `Sidebar` | `sidebar.rs` | Shell and navigation | in use (inside `AppLayout`) |
| `AdminShell` | `admin_shell.rs` | Shell and navigation | in use (route layout) |
| `PageHeader`, `BackLink` | `page_header.rs`, `back_link.rs` | Shell and navigation | in use (every screen) |
| `NavItem` | `nav.rs` | Shell and navigation | kept, not yet used (0 calls; the rail renders its own `nav-item` links) |
| `NavIcon` | `icons.rs` | Shell and navigation | in use (13) |
| `HoraeMark` | `logo.rs` | Shell and navigation | in use (sidebar brand) |
| `TimerWidget` | `timer_widget.rs` | Shell and navigation | in use (sidebar) |
| `VersionInfo` | `version_info.rs` | Shell and navigation | in use (sidebar) |
| `ThemeInit` | `theme.rs` | Shell and navigation | in use (`app.rs`) |
| `Button` | `button.rs` | Actions | underused (7 calls, counted on 2026-11-22; most screens still write the `btn` classes by hand) |
| `IconButton` | `button.rs` | Actions | kept, not yet used (0 calls; the Timesheet day row writes its markup, because a plain render helper cannot hold its handler's hook) |
| `SplitButton` | `button.rs` | Actions | kept, not yet used (0 calls; "Submit week" becomes one when a second way to submit exists) |
| `Menu`, `MenuItem`, `MenuDivider` | `menu.rs` | Menus and pickers | in use (17, 33 and 8, counted on 2026-11-22) |
| `RowActions` | `menu.rs` | Menus and pickers | in use (People, Clients, Tasks, Projects) |
| `SelectField` | `select_field.rs` | Menus and pickers | in use (11) |
| `Combobox` | `combobox.rs` | Menus and pickers | in use (1) |
| `DatePicker` | `date_picker.rs` | Menus and pickers | in use (2; 7 `Input { kind: "date" }` move to it in PR-6) |
| `DateField` | `date_field.rs` | Menus and pickers | in use (New Project, New invoice) |
| `ProjectTaskPicker` | `project_task_picker.rs` | Menus and pickers | in use (`TimerWidget`); renders native `<select>`s until PR-5 |
| `FilterBar`, `SearchField` | `filter_bar.rs` | Menus and pickers | in use (People, the audit log); the other lists move to it in PR-5 |
| `PeriodNav` | `period_nav.rs` | Menus and pickers | in use (Timesheet, People); Approvals moves to it in PR-6 |
| `PeoplePicker` | — | Menus and pickers | planned in the Tasks PR |
| `FormCard` | `form.rs` | Forms | in use (2) |
| `FormGroup` | `form.rs` | Forms | in use (47; 6 hand-written `form-group`) |
| `Input` | `form.rs` | Forms | in use (51; 9 hand-written `form-input`) |
| `Textarea` | `form.rs` | Forms | in use (3) |
| `Select` | `form.rs` | Forms | retired: 7 calls and 8 bare `select {}` move to `SelectField` in PR-5 |
| `Checkbox` | `controls.rs` | Forms | in use (20; 1 native checkbox) |
| `Radio` | `controls.rs` | Forms | kept, not yet used (0 calls, 6 native radios in New Project) |
| `Toggle` | `controls.rs` | Forms | kept, not yet used (0 calls) |
| `Segmented` | `controls.rs` | Forms | underused (2 calls, 5 hand-written groups) |
| `DataTable` | `table.rs` | Data display | in use (21; Clients, Client detail, Tasks and Projects still frame their tables by hand) |
| `TableSkeleton`, `TableMessage` | `table.rs` | Data display | in use (Plugins) |
| `NameCell` | `table.rs` | Data display | in use (Client detail) |
| `Badge` | `badge.rs` | Data display | in use (15; 14 hand-written `badge` spans) |
| `Card` | `card.rs` | Data display | underused (Client detail and the Harvest run sections; most sections still write `card` by hand) |
| `MetricCard` | `card.rs` | Data display | in use (the Invoices summary) |
| `Avatar` | `avatar.rs` | Data display | in use (3, plus the sidebar) |
| `Chip` | `avatar.rs` | Data display | kept, not yet used (0 calls; New Project's tag chips are hand-written) |
| `BulkBar` | `table.rs` | Data display | kept, not yet used (the gallery; Tasks and Projects adopt it) |
| `Toast`, `ToastContainer` | `toast.rs` | Feedback | in use (3 pages, each with its own container); one container in `AppLayout` and the new look in PR-9 |
| `NoTrackingHint` | `no_tracking.rs` | Feedback | in use (Timesheet, `TimerWidget`) |
| `EmptyState`, `ErrorState`, `LoadError`, `Loading`, `Skeleton` | `screen_state.rs` | Feedback | in use (`Skeleton` in the Invoices screens, People and the audit log) |
| `Modal`, `ModalHeader`, `ModalBody`, `ModalActions` | `modal.rs` | Overlays | in use (10 dialogs) |
| `ConfirmDialog` | `confirm_dialog.rs` | Overlays | in use (10) |
| `LeaveDialog` | `leave_dialog.rs` | Overlays | in use (`App`: the editor navigation guard) |
| `ExportDialog` | — | Overlays | planned in the Tasks PR |
| `use_session`, `use_week_start` | `session.rs` | Helpers | in use |
| `use_running_timer` | `timer_widget.rs` | Helpers | in use |
| `today`, `UtcOffset` | `today.rs` | Helpers | in use |
| `use_debounced` | `debounce.rs` | Helpers | in use (9) |
| `permission_description` | `permission_description.rs` | Helpers | in use (6) |
| `Theme` | `theme.rs` | Helpers | in use (Settings) |

### Catalog

#### Shell and navigation

- `AppLayout` (`layout.rs`) is the signed-in shell: the sidebar, its resize handle, the mobile navigation toggle and the `main` region that renders the route. It provides the session, the running timer and the person's UTC offset to everything below it. The router mounts it with `#[layout(AppLayout)]`; pages never render it. No props. See [Layout](#layout).
- `Sidebar` (`sidebar.rs`) is the left rail: brand, `TimerWidget`, the navigation sections the person may open, `VersionInfo` for administrators and the account menu. Only `AppLayout` renders it. Props: `collapsed: Signal<bool>`, `on_navigate`. See [Sidebar](#sidebar-srccomponentssidebarrs).
- `AdminShell` (`admin_shell.rs`) wraps the Workspace and Data sections (Settings, Plugins, API tokens, Apps, Importers, Audit log) with their subnavigation and checks that the person may open the section. The router mounts it with `#[layout(AdminShell)]`. No props. People is a main screen outside it (see [People](#people)). It sets no maximum width (see Form width in [Layout](#layout)).
- `PageHeader` (`page_header.rs`) and `BackLink` (`back_link.rs`) draw the top of every screen. Props of `PageHeader`: `title`, `title_id` (makes the heading a focus target), `leading`, `context`, `meta`, `actions`. See [Page header](#page-header-srccomponents).
- `NavItem` (`nav.rs`) is a rail row with an optional text icon. Today it is a `button` with an `onclick` and draws a trailing dot on the active row, so the rail renders its own `Link`s with the same `nav-item` classes instead, and `NavItem` only appears in the gallery. When a screen starts using it, it must take a `Route` and navigate with `Link` (see [Component Conventions](#component-conventions)), and drop the dot, which [Sidebar](#sidebar-srccomponentssidebarrs) no longer has. Props today: `label`, `icon`, `active`, `onclick`.
- `NavIcon` (`icons.rs`) draws one of the 16px stroked glyphs (`timesheet`, `clients`, `projects`, `arrow-left`, `chevron-down`, `pencil`, `info` and the rest) in `currentColor`. Use it for every icon these names cover rather than a text glyph. An unknown name renders nothing. Props: `name`, `class`.
- `HoraeMark` (`logo.rs`) is the inline Horae logo, coloured by the `--icon-*` tokens so it follows the theme. It is used only by the sidebar brand. No props.
- `TimerWidget` (`timer_widget.rs`) is the rail's timer: a "Start timer" button, the project and task picker, then the running time with a stop button. It is mounted once, by the sidebar. To start, stop or edit time from a page, call `use_running_timer().refresh()` after the change instead of rendering a second widget. No props. See [Timer Widget](#timer-widget-srccomponentstimer_widgetrs).
- `VersionInfo` (`version_info.rs`) shows the server and front-end build labels and offers a reload when they differ. The sidebar shows it to administrators. No props.
- `ThemeInit` (`theme.rs`) applies the saved theme before the first paint and defines `setHoraeTheme()`. `app.rs` mounts it once in the head. No props.

#### Actions

- `Button` (`button.rs`) is the way to draw an action: a `button` with the `btn` classes of its variant. Use it instead of writing `button { class: "btn …" }`. Props: `variant` (a `ButtonVariant`: Primary, the default, Secondary, Accent, Danger, DangerSolid or Ghost; see [Buttons](#buttons-srccomponents)), `small` (`btn-sm`), `label` (required: the text), `onclick` (optional) and `children` (only an icon, drawn before the label).
  - Any native button attribute passes through `#[props(extends = GlobalAttributes, extends = button)]`: `id`, `r#type`, `disabled`, `aria_*`, `"data-testid"`, and a `class` of extra utilities, appended to the button's own. The type is `button` unless one is passed, so a button only submits its form with `r#type: "submit"`.
  - It re-renders with its page only when its props change. Pass handlers made with `use_callback`: an inline closure is a new callback on every render. `children` compares by pointer, so a button with an icon re-renders whenever its page does; keep those out of hot rows. `tests/button.rs` counts these renders.
- `IconButton` (`button.rs`) is a square icon-only `btn-icon` button. `label` is required and is its accessible name (`aria-label`); it replaces any `aria_label` passed. Use it for an action shown only as an icon, which goes in `children`. Props: `variant` (default Ghost), `label`, `onclick`, `children`, and the native attributes `Button` takes.
- `SplitButton` (`button.rs`) is the `.btn-split` pair: a main action and a caret button. The caret only fires `ontrigger`; opening a menu of variants is the caller's job. Native attributes go to the main button, except `disabled`, which disables both. Props: `variant`, `label`, `trigger` (default `▾`), `onclick`, `ontrigger`. See Split button in [Buttons](#buttons-srccomponents).

#### Menus and pickers

- `Menu`, `MenuItem`, `MenuDivider` (`menu.rs`) are the dropdown: a trigger button with a caret that opens a native popover of items. Use `Menu` for a row's or a page's "Actions" and for a filter with a few fixed values (see [Selects, filters and search](#selects-filters-and-search-srccomponentsselect_fieldrs-menurs)). It is not a form field: to choose a value in a form, use `SelectField`. `MenuItem` takes `selected` (for a filter's current value), `danger` (Delete, last, after a `MenuDivider`) and `disabled`. Props of `Menu`: `id` (stable and unique; it ties the trigger to the menu), `label`, `align_right`, `trigger_class`, `disabled`, `busy`, `icon`, `children`. `icon` makes the trigger the 32px «⋮» button of a table row, named by `aria_label` alone. Rows use it through `RowActions` (`id`, `name`, `busy`, `children`), which names the trigger "Actions for {name}" and anchors the menu to the right. `trigger_class` adds optional utilities to the trigger: its default remains `btn btn-secondary btn-sm`, and a surface may opt into `text-sm px-4` without changing other callers or adding page-specific descendant overrides. `disabled` opts into a native disabled trigger; Projects uses it when its selection is empty or a bulk operation is pending, and other menus remain enabled. Rules in [Data Tables](#data-tables-srccomponentstablers), [Destructive actions](#destructive-actions) and [Accessibility](#accessibility).
- `SelectField` (`select_field.rs`) is the form selector: a full-width trigger styled as an input, opening a listbox on the menu surface. Give it a `query` signal to add a search box for long lists, and `pending` while the options load. Use it for every single-value choice in a form, in place of `Select` and native `<select>`. Props: `id`, `label`, `options: Vec<(value, label)>`, `selected`, `onselect`, `error_id`, `disabled_values`, `query: Option<Signal<String>>`, `pending`, `children` (rendered under the list). See [Selects, filters and search](#selects-filters-and-search-srccomponentsselect_fieldrs-menurs).
- `Combobox` (`combobox.rs`) is a compact searchable picker drawn as a `btn-sm` trigger, with an "All" row that clears the choice and options that can be grouped under headings (`ComboOption::grouped`). Its one call is the client filter in Projects. Use it for a filter with many values, where search helps; a filter with a few fixed values uses `Menu`, and in a form use `SelectField` with a `query`. Props: `options: Vec<ComboOption>`, `value`, `placeholder`, `all_label`, `trigger_class`, `onselect`. `trigger_class` works as in `Menu`: optional utilities on a trigger whose default remains `btn btn-secondary btn-sm`, so a surface may opt into `text-sm px-4` without changing other callers or adding page-specific descendant overrides. Unlike `Menu` and `SelectField` it draws its own overlay; PR-5 decides whether it moves to the native popover.
- `DatePicker` (`date_picker.rs`) is the month calendar panel. It highlights a day or, with `week`, the whole week, and emits the picked date. It draws only the panel: the caller supplies the trigger and the popover, as `DateField` and the Timesheet pager do. Props: `selected: NaiveDate`, `week`, `first_day`, `onpick`. See [Formats](#formats).
- `DateField` (`date_field.rs`) is a form's date field: a trigger showing the date in the app's format (`01 Sep 2026`) that opens `DatePicker` in a popover, with a clear button. Its value is an ISO date or empty. New Project's dates and a new invoice's custom period use it; PR-6 moves the remaining `Input { kind: "date" }` fields to it. Props: `id`, `label`, `placeholder`, `value`, `error_id`, `onchange`.
- `ProjectTaskPicker` (`project_task_picker.rs`) is the pair of project and task fields for new time. It offers only the projects and tasks the server says the person can track, and clears a selection that falls out of that list. Use it wherever new time is entered; the timer uses it today. Props: `project` and `task` (`Signal<String>`), `projects` and `tasks` (resources), `disabled` (keeps an existing entry's names read-only). It still renders native `<select>`s, which PR-5 replaces.
- `PeoplePicker`: planned in the Tasks PR, for choosing several people (a task's default assignees). Its API is decided in that PR.
- `FilterBar` and `SearchField` (`filter_bar.rs`) are the bar of filters and search below the header. People and the audit log use them; PR-5 moves the other lists to them. Props of `FilterBar`: `filters`, `search` (optional, drawn on the right); of `SearchField`: `id`, `label`, `query`. `PeriodNav` (`period_nav.rs`) is the week pager, with a date picker and "This week", used by the Timesheet and People; Approvals moves to it in PR-6. Props: `id`, `week_start`, `day`, `today`, `day_paged`, `on_step`, `on_pick`. See [Selects, filters and search](#selects-filters-and-search-srccomponentsselect_fieldrs-menurs).

#### Forms

- `FormCard` (`form.rs`) is the card around an inline create or edit form, at most `--width-form` wide: a `card-title`, the form's error as an `.alert-danger`, then the fields. Props: `title`, `error: ReadSignal<Option<String>>`, `children`.
- `FormGroup` (`form.rs`) pairs a label with its control and an optional hint. Give it the control's `id` so the label points at it. Props: `label`, `id`, `hint`, `children`. See [Forms](#forms-srccomponentsformrs).
- `Input` (`form.rs`) is the text input (`kind` sets the HTML type). Use `label` when there is no visible `FormGroup` label, and `error_id` to point at the field's visible error. Props: `kind`, `id`, `label`, `class`, `value`, `placeholder`, `disabled`, `readonly`, `error_id`, `oninput`. Do not use `kind: "date"`: dates use `DatePicker` (see [Formats](#formats)). Until PR-6, copy the trigger and popover of `pages/new_project/date_field.rs` around it.
- `Textarea` (`form.rs`) is the multi-line input. Props: `id`, `value`, `placeholder`, `rows` (default 3), `disabled`, `error_id`, `oninput`.
- `Select` (`form.rs`) renders a native `<select>` and is retired. Do not add calls; use `SelectField`. See [Selects, filters and search](#selects-filters-and-search-srccomponentsselect_fieldrs-menurs).
- `Checkbox` (`controls.rs`) is the labelled two-state check drawn as a `button` with `role=checkbox`. `compact` keeps only the accessible name (table rows), `mixed` shows the indeterminate state of a select-all, and `error_id`/`hint_id` link it to its messages. Use it instead of a native checkbox. Props: `checked`, `label`, `id`, `compact`, `mixed`, `disabled`, `error_id`, `hint_id`, `onclick`. It keeps its labelled, two-state defaults: opt into `compact` for an accessible-name-only table control, `mixed` for select-all's indeterminate state, and `disabled` while submitting. `--size-checkbox` preserves the existing 18px square. Projects adds its selection track only through `.proj-grid-selectable`; read-only rows and other shared controls retain their existing layout.
- `Radio` (`controls.rs`) is one labelled option in a list where exactly one is chosen. Today it is a `button` with `role=radio`. Props today: `selected`, `label`, `onclick`. *Pending code:* it is to wrap a native `<input type=radio>`, keeping its look and the arrow-key movement of a native radio group, and to take an optional help text under the label. It then replaces the hand-written radios of New Project's three option lists: the rate mode, report visibility and task access. New Project's project-type cards are not radios in this sense: they stay their own pattern of choice cards.
- `Toggle` (`controls.rs`) is an on/off switch (`role=switch`) with an optional label, for a setting that applies at once. Props: `on`, `label`, `onclick`.
- **`Toggle` or `Checkbox`.** A `Toggle` changes something at once, with no Save button. A `Checkbox` is part of a form that saves. A setting with several named values that applies at once (the theme: Dark or Light) is a `Segmented`, because it is a choice between names, not on or off. A filter is neither: it goes in the `FilterBar`. Today My settings' Theme is a native `<select>`, the scoped Reports filter with an "Active projects only" checkbox, and New Project has 8 native checkboxes and 6 native radios in a form that saves.
- `Segmented` (`controls.rs`) shows several labelled options with one active, and emits the chosen label. Use it to switch between views of the same content (Timesheet day, week and calendar; the Approvals status switch), or for a setting with a few named values that applies at once (Theme), not to filter a list (see `FilterBar` in [Selects, filters and search](#selects-filters-and-search-srccomponentsselect_fieldrs-menurs)). One exception: a short choice inside a form may also use it, as New Project's fixed-fee mode does (Single fee, Milestones, Monthly). Props: `items: Vec<String>`, `active`, `onselect`. *Pending code:* the fixed-fee mode is still a hand-written radio group and moves to `Segmented`. For that use `Segmented` needs radio-group semantics (`role=radiogroup` with `role=radio` options) instead of its current `role=tablist`, added in the PR that moves it.

#### Data display

- `DataTable` (`table.rs`) wraps a `table` in `.table-container` (border and panel radius). Put every data table in it. Props: `label` (names the scroller and makes it a focusable region), `flush`, `compact`, `striped`, `sticky_header`, `busy`, `children`. `TableSkeleton` (`columns`, `label`, `rows`) and `TableMessage` (`columns`, `children`) are rows for its `tbody`, and `NameCell` (`secondary`, `children`) is its first column. See [Data Tables](#data-tables-srccomponentstablers).
- `Badge` (`badge.rs`) is the status pill with a dot. Show every status with it, never as plain text. Props: `variant` (`success`, `warning`, `danger`, `info`, `neutral` or `pine`; the class is `badge-{variant}`), `children`. The small size (`badge-sm`) has no prop yet, which is why screens write some badges by hand. See [Status Badges](#status-badges-srccomponentsbadgers).
- `Card` (`card.rs`) is one section of a screen: a `section.card` whose optional title is an `h2.card-title` that names it (by `aria-labelledby` when `id` is given, or by `aria-label`). `focusable` makes the title a focus target, for a retry that moves focus there. `flush` drops the padding for a table or list that draws its own rows. Props: `title`, `id`, `focusable`, `flush`, `class` (layout utilities such as `max-w-form`), `children`.
- `MetricCard` (`card.rs`) is one figure: an uppercase label, a mono value and an optional delta (`direction` is `up`, `down` or empty). Props: `label`, `value`, `delta`, `direction`.
- `Avatar` (`avatar.rs`) is a person's round avatar with an image or initials; `empty` draws the dashed placeholder of an unassigned slot. Props: `initials`, `image_url`, `size` (`sm`, empty or `lg`), `empty`. Initials are always two letters, from one shared function. Today there are two functions: `first_initial` in this file gives one letter, and `initials` in `sidebar.rs` gives two; they are to be merged into the two-letter one.
- `Chip` (`avatar.rs`) is a compact person chip (avatar and name), or with `plain` a tag chip without the avatar. It has no remove button, so it does not cover New Project's removable tags. Props: `label`, `initials`, `plain`, `variant`.
- `BulkBar` (`table.rs`): while rows of a list are selected, a bar floats at the bottom of the screen with how many are selected, the actions that apply to all of them (Secondary `btn-sm` buttons, Delete last as `btn-danger`) and "Clear". It sits on the menu surface: `--color-menu`, the menu border and `--shadow-menu`. Render it only while something is selected. Props: `count`, `on_clear`, `children`.
- **Lists of data have no bullets.** Tasks, a team, permissions: rows with dividers (`ul.data-list`), or a table. A list of more than about ten items is grouped or folded. *Pending code:* Project detail's team (26 people in the demo data) is not folded yet.

#### Feedback

- `Toast` and `ToastContainer` (`toast.rs`) confirm a completed action: a `role=status` notice with an optional action ("Undo") and close button, inside a fixed stack. Today each page renders its own container; PR-9 moves the single container into `AppLayout` and gives the toast its bottom-centre look. Props of `Toast`: `message`, `variant` (`success`, `danger`, `warning`, `info` or empty), `icon`, `dismissible`, `ondismiss`, `action_label`, `action_busy`, `onaction`. See [Toast](#toast-srccomponentstoastrs).
- `NoTrackingHint` (`no_tracking.rs`) explains why the person cannot track time (no project assigned) wherever a start or add control would sit disabled without a reason, and links managers to Projects. Point the disabled control's `aria-describedby` at its `id`. Props: `id`, `assignee`, `can_manage`, `on_follow`. `no_tracking_message` gives the same sentence as text.
- `EmptyState`, `ErrorState`, `LoadError`, `Loading` and `Skeleton` (`screen_state.rs`) draw a region with nothing to show, a region that failed to load and a region that is loading. `LoadError` picks the right `ErrorState` for a failed server read, and `loaded` (re-exported as `pages::loaded`) renders a resource with all three. Props and rules in [Empty, loading and error states](#empty-loading-and-error-states-srccomponentsscreen_staters).

#### Overlays

- `Modal` (`modal.rs`) is the native `dialog` every dialog is built on. Keep it mounted and toggle `open`: closing returns focus to the opener, and `busy` blocks Escape and backdrop dismissal while a mutation runs. Props: `id`, `labelledby` (the title's id), `describedby` (the id of the text that says what the dialog is about), `open`, `busy`, `large`, `focus_fallback`, `primary`, `secondary`, `cancel`, `on_dismiss`, `children`.
  - `children` holds the header and the body: `ModalHeader` (props `id`, the `labelledby` value, and `title`) and `ModalBody` (the padded content).
  - The footer is not a child: `Modal` draws it from `primary` and `secondary` (each an `Option<ModalAction>`) and `cancel` (an `Option<ModalCancel>`, "Cancel" by default; `None` hides it), always in that order, and only while open. A `ModalAction` is built with `ModalAction::click(label, callback)`, `ModalAction::submit(label, form_id)` (it submits a form anywhere in the dialog) or `ModalAction::link(label, href, callback)`, then `.variant(…)` (Primary by default), `.id(…)` and `.disabled(…)`. `ModalCancel::new(label)` takes `.id(…)` (default `{id}-cancel`), `.disabled(…)` and `.autofocus()` (opening focuses Cancel); Cancel always calls `on_dismiss` and is disabled while `busy`.
  - When the buttons depend on state that only a component inside the dialog holds (a form that loads, a multi-step editor), that component renders `ModalActions` with the same three props. `Modal` draws those instead of its own while it is mounted. Mount one at a time and pass handlers made with `use_callback`.
  - Never write the `.modal-*` classes, a `dialog` element, a title of your own or footer buttons, and never ask through the browser's `confirm`, `alert` or `prompt`: `tests/dialog_anatomy.rs` fails if a file outside `modal.rs` does. See [Dialogs](#dialogs-srccomponentsmodalrs) and [Accessibility](#accessibility).
- `ConfirmDialog` (`confirm_dialog.rs`) confirms one destructive, irreversible or otherwise significant action, on top of `Modal`, which draws its footer. `children` is the body: what will happen, and the error of a failed try; it is the dialog's description (`{id}-body`, `aria-describedby`). Its buttons are `{id}-confirm` and `{id}-cancel`; the content renders only while it is open, and the Danger tone opens with Cancel focused. It may open over another dialog (deleting a time entry from its editor). Props: `id`, `labelledby`, `open`, `title`, `confirm_label`, `cancel_label` (default "Cancel"), `tone` (a `ConfirmTone`: Danger, the default, with Danger solid; Neutral with Primary), `busy`, `confirm_disabled` (holds confirm while what it confirms is not ready, such as a preview still loading), `confirm_word` (the word to type before confirm enables), `focus_fallback`, `on_confirm`, `on_dismiss`, `children`. No screen uses `confirm_word` yet; it stays, as part of the prototype's component, for the most serious actions. See [Dialogs](#dialogs-srccomponentsmodalrs) and [Destructive actions](#destructive-actions).
- `ExportDialog`: planned in the Tasks PR, the dialog a list's "Export" action opens. Its API is decided in that PR.

#### Helpers

These modules render nothing (or almost nothing) but are shared the same way.

- `session.rs`: `use_session()` returns the signed-in person (`me`) and their permissions, loaded once by `AppLayout`. Use it to show or hide UI, never to authorize. Call `refresh()` after a change to the person's own access. `use_week_start()` returns the organization's first weekday, fetched on first use.
- `timer_widget.rs`: `use_running_timer()` returns the running timer that the rail shows. After any change to time entries, call its `refresh()`. Its `changes()` counter lets a page's entry list follow a timer started from the rail.
- `today.rs`: `today()` is the person's current date in their profile's time zone. Use it instead of `chrono::Local` or `Utc::now()` in UI code.
- `debounce.rs`: `use_debounced(signal, SEARCH_DEBOUNCE)` gives a copy of a search or filter value that updates once typing pauses, so a resource reading it fetches once per pause.
- `permission_description.rs`: `permission_description(permission)` is the one sentence that names each permission in the UI. It is display copy and grants nothing.
- `theme.rs`: `Theme` lists the dark and light palettes for the Settings choice (`ThemeInit` is above).

### Sidebar (`src/components/sidebar.rs`)

Full-height left rail. The brand row leads with the Horae mark and wordmark plus the golden accent dot. Nav rows default to `--color-text-secondary`; hover darkens the background to `--color-bg-tertiary`; the active row raises to `--color-bg-tertiary` with a `--color-border` hairline and tints its glyph `--color-primary`; that highlight is its only marker, with no trailing dot. Section labels: all-caps, wide letter-spacing, muted warm gray. Navigation labels follow the [glossary](#glossary) ("People", not "Team").

The rail offers only the sections the person can open: Invoices and Approvals are filtered by permission like Clients and Tasks, so a member never follows a link to "Access unavailable". The Organize group links to People for people who may see people; People is not in the admin subnavigation (see [People](#people)).

### Page header (`src/components/`)

One `PageHeader` component (`page_header.rs`, with `back_link.rs`) renders the top of every screen: the title (see [Type scale](#type-scale)) and the screen's actions, with an optional `BackLink` on its own row above it. Header actions use the normal button size, never `btn-sm` (header menus pass `trigger_class: "text-sm py-3 px-5"`). They sit at the right while they fit beside the title and wrap onto a left-aligned row of their own when they do not, the same way on every screen. One shared header keeps the title, the way back and the actions in the same place everywhere.

`PageHeader` has three optional slots besides the actions. Each is for the one thing named here, so headers do not drift apart again:

- **`context`**: one line above the title, only when it carries data about the record: the client of a project (a link when the viewer may open it), the date a client was added. Never a fixed label such as "Projects"; the back link already names the section.
- **`meta`**: inline after the title, for status badges and short qualifiers: a currency or project-type pill, "Archived", the connection status of an integration, the source of an import run, the Timesheet's week total.
- **`leading`**: before the title, only for an integration's own logo (Harvest), as a 36px mark (`--size-page-mark`) with the button radius. The logo is a local asset shown unaltered (see `AGENTS.md`, *Harvest parity*); no other icon or brand goes here.

Anything longer, such as a status sentence or a description, goes below the header, not in it.

- **The main action is in the header.** A screen's Primary action sits in `PageHeader`'s actions. A form's submit goes at the end of the form instead (New Project's fixed footer, the Save of each Invoice settings card), and a row's actions stay in the row. A new invoice is its own screen in two steps (`/invoices/new`), so each step has one Primary ("Continue", then "Create draft") and the way out is the back link, not a header Cancel. A record's other actions go in the header's menu (Invoice detail's "More": Edit draft, Duplicate and "Void…"; Client detail's Edit and Archive).

- **No search or filters in the header.** The header holds actions only; search and filters go in the `FilterBar` below it (see [Selects, filters and search](#selects-filters-and-search-srccomponentsselect_fieldrs-menurs)). Clients and Projects put their search among the header actions today. *Pending code (PR-5).*

- **Back link**: the kit's *Back link*. A 16px arrow and a 14px label on their own row above the title, muted ink that brightens to pine on hover. It points to the parent in the hierarchy ("Back to Projects" from a project, "Back to Importers" from Harvest), not to the screen the user came from, and only screens that sit under a section have one. Sections reached from the sidebar (Timesheet, Projects, People, Settings, API tokens, Importers, the audit log and the rest) have none, because the sidebar is always there; the `AdminShell`'s "Back to Timesheet" link is removed.

  - **Exception: editors return where they were opened.** An editor of an existing record goes back to the screen that opened it, because leaving it is closing a task, not moving up the hierarchy. Edit project returns to the project ("Back to project") when opened from its detail and to Projects ("Back to Projects") from the list, as its `return_to` says. The back control of an editor with a draft is a button with the back-link look, because leaving first saves the draft or asks to discard it.

- **No breadcrumbs.** The back link replaces a breadcrumb; never show both. The Harvest section's `BackLinks` breadcrumb (`Importers / Harvest / …`) is replaced by a back link to the parent. No other style of "back" is allowed: no `btn-ghost` or `btn-secondary` back buttons and no top-right back button.

### Timer Widget (`src/components/timer_widget.rs`)

- **Idle**: no time is shown. The rail has a full-width "Start timer" button in the Primary fill (`--color-primary`, hover brightens with `--glow-primary`) with a small clock glyph; it opens the project and task picker.
- **Running**: a panel on the pine wash (`--color-primary-bg` with a `--color-primary-border` hairline) holds a pine live dot, the elapsed time as `H:MM:SS` (hours not zero-padded) in IBM Plex Mono `lg`/600 and `--color-text-strong`, the "Project · Task" line in `xs` muted ink, and the stop button. Stop is a red (`--color-danger`) square inside a small neutral button, never a word, so the panel keeps its shape.
- **Collapsed rail**: the running time shortens to `H:MM`.

### Data Tables (`src/components/table.rs`)

Every table is a `DataTable`, and its props are the only variants; a screen never styles a table frame of its own. The frame is `.table-container` (border + 16px `--radius-xl`, see [Border Radius](#border-radius)). Headers: uppercase, 0.1em letter-spacing, transparent surface, label-tone ink (`--color-label`); no screen recolours them. Rows are ruled by default, with a light rule between them, and hover sinks to `--color-row-hover`. Last row has no bottom border. Totals go in `tfoot`, with the label as `th scope=row`. The first column is the record's name in bold with an optional secondary line under it (`NameCell`). Numbers, money and durations are right-aligned in `text-mono`, the one mono utility; dates are mono in the secondary colour. Status is always a `Badge`, never plain text.

The variants:

- **Striped** (`striped`): every other row on `--color-row-stripe`, with no rules. Hover takes `--color-row-hover-striped`, which stands out from both bands. Only for row-by-row lists with few columns, where ruled rows read as one grey block: Clients, Tasks, People and Approvals. Tables of figures and totals (Reports, Invoices, Projects) stay ruled. A selected row (`tr.selected`) takes `--color-row-selected`, which replaces the stripe.
- **Compact** (`compact`): 10px × 16px cells, for dense data that people scan: the detailed reports, the importer's errors and invoice lines.
- **Sticky header** (`sticky_header`): for a list that can scroll past one screen. The scroller takes a maximum height of 70% of the viewport and the header stays at its top.
- **Flush** (`flush`): inside a card that is already the frame. The table keeps only a rule under the card's title block, and "Load more" or "Show all" sits in the card under it. Never a bordered table inside a bordered card, and never a bare table with no frame at all.
- **Busy** (`busy`): a reload keeps the rows, dimmed, with `aria-busy` on the frame.
- **Label** (`label`): names the scroller and makes it a focusable region (`role=region`, `tabindex=0`), so a keyboard can scroll a table wider or taller than its frame. Every table that can scroll gets one; the financial reports, People, Plugins, Your access, Invoices, Uninvoiced and the invoice lines have one so far.

States live inside the frame, under the real header, so the page does not jump: `TableSkeleton` rows while the table loads for the first time, and one `TableMessage` row spanning every column, holding an inset `EmptyState`, `ErrorState` or `LoadError`, for empty, no match and error. The full-page `EmptyState` is only for a screen whose whole purpose is the list and that has never had a record (Plugins with none installed). *Pending code:* only Plugins, Invoices, Uninvoiced and API tokens put their states in the frame so far.

A record's actions (edit, archive, permissions, rates…) go in one «⋮» menu per row (`RowActions`), a 32px icon button in the last column, the same way on every table, rather than as loose buttons that wrap at narrow widths. One exception: Approvals keeps "Approve" and "Reopen" inline in each row, as Secondary `btn-sm` buttons, because approving row by row is the screen's job and a menu would add a click to every approval. The menu shows only the actions the viewer may take on that row; when none remain, the row has no menu. Its trigger is named after the record ("Actions for Acme Corp"), and while an action chosen from it is still running the trigger is `aria-busy` and dimmed, since the closed menu cannot say so. Actions on several selected rows go in the `BulkBar`, never in the page header. Inline row buttons are named after their row the same way ("Approve Ana's week", "Edit Acme Website Redesign, Design"). A row whose only action is opening its record has no menu and no "View" button: the record's link in the first column covers the whole row (`tr.row-link`), as Invoices does; Uninvoiced does the same with the new invoice for its client. Any other link in the row (Uninvoiced's last invoice) sits above it and opens its own record. See [Layout](#layout) for column widths and mobile behaviour.

**Catalogues load whole; histories load more.** Clients, Projects, Tasks and People load every record at once and narrow it with the `FilterBar` and its search, so finding a record never means paging through 25 at a time. Only histories (the audit log, Harvest import runs, the detailed report) page, with a single "Load more" at the end of the list that appears only when there is more. Never Previous/Next, and never a pager under an empty state or an error. Today Tasks pages by 25 with Previous/Next. *Pending code:* the Tasks PR for Tasks, and PR-C for the scoped detailed report.

### Forms (`src/components/form.rs`)

Inputs on `--color-bg` (void — darkest), 8px radius, 44px tall, warm border (`--color-border-input`). Idle hover lifts the border to `--color-border-strong` and warms the fill; focus ring is `--color-primary` border + 18% pine glow. Instrument Sans for all form text.

### Selects, filters and search (`src/components/select_field.rs`, `menu.rs`)

- **Never a native `<select>`.** Picking one value from a list always uses Horae's own dropdown. In forms that is `SelectField` (a full-width trigger on the shared menu surface), given a `query` for long catalogs so it gets a search box. In filters it is a `Menu` when the filter has a few fixed values, and a `Combobox` (the compact searchable trigger with an "All" row) when it has many. PR-5 decides whether `Combobox` moves to the native popover that `Menu` and `SelectField` use. The kit asks for custom dropdowns everywhere, and a native `<select>` draws a different arrow and list in each browser.
- **`SelectField` and `form::Select`.** `form::Select` renders a native `<select>`, so it is retired: its callers move to `SelectField`, and the two are unified into one form selector. Until that lands, use `SelectField` in new code.
- **`FilterBar`.** Every list that can be filtered puts its filters in one shared `FilterBar`: a row of filter triggers, `Menu` or `Combobox` as above (with a trailing caret when they open a menu and a pine count when active), plus the search field when the list has one. This replaces the four current shapes (menus in Projects and Clients, native selects labelled "Show" in Tasks and People, a card of labelled selects in Reports). The Approvals status switch stays a `Segmented` control because it switches views, not filters. The bar sits below the page header, never inside it, and keeps one order on every screen: the filters together on the left, the search on the right (Clients and Projects put their two filters at opposite ends today). Every catalogue has a search: Tasks and People have none today. The Reports screen has the same bar whatever the viewer's scope: the scope changes the data, not the layout.
- **`SearchField`.** One shared search field, always with the magnifier glyph inset on the left. It has the same border, height and focus ring as any other input.
- **Dates** are chosen with the shared `DatePicker` (`src/components/date_picker.rs`), never a native `<input type=date>`, whose display format depends on the browser locale (`11/01/2026` in US English). See [Formats](#formats).

### Status Badges (`src/components/badge.rs`)

Pill shape (9999px radius, `4px 12px`). Every variant — including neutral — carries a 7px status dot (via `::before`) over a tinted background + warm border. Neutral uses the `--color-menu` surface with a `--color-border-strong` dot.

### Buttons (`src/components/`)

Control-lg is 44px tall (`12px 20px`). Brand fills **brighten** on hover with a soft glow; neutral controls lift a shade and raise their border. Weights: primary/accent 700, secondary/danger/ghost 500.

The `Button` component (`button.rs`) takes the look as `variant: ButtonVariant`, which defaults to Primary.

- **Primary**: pine-300 fill, dark green text `#0d211b`; hover → `--color-primary-hover` + `--glow-primary`. The main action of a screen or form, including form submits. Use at most one Primary per screen or form, so the next step is always obvious. A screen made of several independent forms (Invoice settings saves each card on its own) has one Primary per form.
- **Secondary**: tertiary surface, `--color-border-input` border; hover raises the border to `--color-border-strong`
- **Accent**: brass fill `#D99A3C`, dark text — "Send invoice" and billing actions; hover → `--color-accent-hover` + `--glow-accent`
- **Danger** (`btn-danger`): transparent with dark red border `#6e3634`; hover tints and lifts the border to `--color-border-danger-hover`. Only for the button that opens a destructive confirmation (see [Destructive actions](#destructive-actions)); never for the confirmation itself and never for archiving.
- **Danger solid**: red fill with dark ink, the kit's `hb-danger-solid` from `ConfirmDialog.dc.html`. Only for the confirm button of an irreversible action. The class is `.btn-danger-solid` (`ButtonVariant::DangerSolid`); `ConfirmDialog` uses it for the Danger tone. In the light theme the fill is `--color-danger-fg` with white ink, because `--color-danger` reaches only 4.47:1 against white, and it deepens on hover instead of brightening, as the light Primary does, because a lighter red would fall below 4.5:1 with white ink.
- **Ghost**: transparent pine-ink link; hover → pine-tint background
- **Split button** (`.btn-split`): a main action with real variants behind a caret, such as submitting a week with or without notifying. Its body is Primary. Use it only when the variants exist: a caret that opens nothing is not offered. The Timesheet's "Submit week" is a plain Primary in the header until a second way to submit exists, and then becomes this split button.

**Solid is retired.** The deep-pine button (`.btn-solid`) is no longer a variant. The kit and several prototypes use it for submits ("Submit week", "Update info", "Save", "Approve"), and this file overrides them: Approvals' "Approve all", Invoice settings' submits and "Submit week" are Primary, and the per-row "Approve" is Secondary `btn-sm`. `.btn-solid` and `--glow-pine` are deleted. `--color-pine` and `--color-pine-hover` stay, because segmented controls, toggles, the calendar and `.badge-pine` use them. One Primary look for every main action means no screen's next step looks different from the others.

### Destructive actions

- **Irreversible actions** (deleting, disconnecting Harvest, revoking a token, voiding an invoice, and anything else that cannot be undone) are confirmed in a `ConfirmDialog` whose confirm button is Danger solid. A red fill marks the one click that loses data. When the action sits in an "Actions" menu, its item is a danger item with an ellipsis ("Void…"), last, after a divider.
- **The button that opens that dialog** uses Danger (`btn-danger`, the red outline). The outline warns; the fill commits.
- **Archiving is not destructive.** It can be undone, so it never uses either red button, and its success toast offers "Undo". Archiving one record (a project, a task, a person) happens at once, with no dialog: the "Undo" is enough. It asks for confirmation only when it carries other records with it (a client and its projects) or acts on several at once (bulk archive); that dialog uses the neutral tone with a Primary button, as the prototypes' archive dialogs do. Restoring works the same way.
- **Record actions** such as Edit, Archive, Restore and Delete are offered through the row's or the page's "Actions" menu, the same way on every screen. Delete sits last, after a divider, as a danger item.

### Dialogs (`src/components/modal.rs`)

Every dialog is built on the shared `Modal`, with fixed slots (header, body, footer) so none of them render unpadded or with a title of their own making. `.ts-modal-*` is gone, and hand-made `h2`/`p` dialogs are retired.

**`ConfirmDialog`** is the one shape for confirming a destructive, irreversible or otherwise significant action, after `ConfirmDialog.dc.html`: an icon tile (red for danger, green for a neutral confirm such as archiving), a serif title, a short body that says what will happen, then the buttons. For the most serious actions (deleting a workspace, for instance) it can require typing a confirmation word before the confirm button enables. Its props follow the prototype: title, body, confirm label, tone (danger or neutral), busy, confirm and dismiss handlers, and the optional confirmation word. The Danger tone opens with Cancel focused, so a stray Enter cannot destroy anything; the Neutral tone focuses its first control.

**No native dialogs.** Every confirmation is a `ConfirmDialog`: never the browser's `confirm`, `alert` or `prompt`, and never a confirmation built by hand in a page or panel (an alert with buttons, a checkbox that unlocks a button). This covers the editor navigation guard (`assets/js/project-edit-navigation.js`). When a link, Back or Forward would leave an editor with unsaved changes, it holds the navigation and `LeaveDialog` (`leave_dialog.rs`, mounted once in `App`) asks whether to discard them, then repeats the navigation on confirm. While a save is still unresolved, the dialog says why the page must wait and offers no way out. A browser may suppress its own dialogs and answer "stay" without showing anything, which would block navigation silently. Only leaving the document (reload, closing the tab, a full-page link) still uses the browser's `beforeunload` prompt, which a page cannot replace.

**Footer order and alignment.** The primary action comes first, on the left, then a second action when the dialog has one ("Delete…" in the time entry, "Back to editing" in the permission editor), then Cancel. The buttons stretch to share the footer's width, and with large text each takes its own line. This follows `ConfirmDialog.dc.html` and most prototype screens; the kit's *Modal*, which puts the primary action on the right, is overridden. Pages never draw the footer: `Modal` draws it from its `primary`, `secondary` and `cancel` props (or `ModalActions`), so the order cannot drift. One order everywhere means the next step is always in the same place.

### Toast (`src/components/toast.rs`)

Every successful action is confirmed with a toast, the one in `Toast.dc.html`, so the user always gets the same signal in the same place:

- Bottom centre of the viewport, on a black (`#000`) panel with a hairline border, in both themes.
- A leading icon by kind: success, info, warning or error.
- The message, an optional inline action ("Undo" after archiving or restoring) and a close button. A long message wraps onto several lines instead of being truncated, so the toast never cuts off what it confirms; the prototype's single truncated line is overridden.
- One container in `AppLayout` shows them, so pages raise toasts instead of rendering their own.

`.alert-success` stays only for a state that must remain visible on the page (an import that finished, say), not to confirm an action. Inline "Saved." text, green `text-success` notes and `role=status` paragraphs used as confirmations are replaced by the toast. The kit's *Form dirty state* asks for the same: saving confirms with a toast, never silence.

### Empty, loading and error states (`src/components/screen_state.rs`)

- **`EmptyState`**: the Projects variant is the base of `.empty-state` for every screen: a `--color-bg-secondary` card with a border and `--radius-xl` (16px), the icon in a 52px tile (`--radius-icon`, 22px glyph), a Newsreader `xl` title, copy limited to `--width-empty-copy`, and an optional action (Primary, or Secondary when it only resets filters) plus an optional text link. The copy says what to do next. The kit and `EmptyState.dc.html` draw the icon in a round disc; Horae uses the tile. The `success` variant (an all-clear moment such as nothing to approve) keeps the green icon colours. Props: `icon` (a `NavIcon` name), `title`, `body`, `success`, `inset`, `id`, `actions`.
  - **One icon per kind of emptiness.** The icon names what is missing: a key for API tokens, a puzzle piece for plugins, a history glyph for the audit log. Never one generic icon shared by unrelated sections.
- **`ErrorState`**: for a region or screen that failed to load. It keeps the empty state's silhouette so the eye lands in the same place, including the 52px icon tile (the prototype's round disc is overridden here too), with the icon in the danger tint, a title and body that say what happened and what to do, an optional mono request line for support, and "Try again" as the action. "Try again" is always a Secondary button, whatever else the screen shows, so a failed region never adds a second Primary. A permission error points to who can grant access instead of offering a retry, and shows a lock instead of the warning glyph (see [Screen States](#screen-states)). The request line shows only the server's own message, never transport text ("error sending request…"). Props: `title`, `body`, `request`, `onretry` (no retry when absent), `retry_id`, `retry_disabled`, `focus_on_recovery`, `icon`, `inset`, `id`, `actions`. `LOAD_FAILED` is the body for a failure the page cannot explain.
- **Retrying**: a failed region stays mounted while its retry runs, with "Try again" disabled; only the first load shows `Loading`. When the retry succeeds, focus moves to the region's heading (`focus_on_recovery`, a heading with `tabindex=-1`), so it never falls back to the page.
- **`LoadError`**: the `ErrorState` for a failed server read, chosen by what the failure means. A refusal (401, 403) names who can grant access and offers no retry; a missing record (404) takes the server's "… not found" as its title and offers no retry; anything else is `LOAD_FAILED` with "Try again". Props: `error`, `what` (as in "Could not load {what}"), `retry`, `pending`, `inset`, `focus_on_recovery`, `retry_id`.
- **Inset**: inside a panel that already has its own border and heading (a card section, the client detail's Projects and Invoices), `inset` drops the state's card and titles it with an `h3`, so a card never sits inside a card.
- **`Loading`**: one component for every inline loading message, a `role=status` line, so assistive technology hears the same thing everywhere. Text is only for small regions (a picker's options, a status badge reading "Checking…"); a list, grid, form or detail loads with a `Skeleton` (see [Screen States](#screen-states)). The message itself is not `aria-busy`, which would let a screen reader hold the announcement back; a container that keeps its old content while it reloads (the Timesheet grid, the Projects list) carries `aria-busy` instead. It replaces the ad hoc `p { "Loading…" }` variants. Props: `label` (default "Loading…"), `class`.
- **`Skeleton`**: a placeholder shaped like the content that will arrive (the rows and columns of a list, the Timesheet week's grid, a form's fields, a detail's sections), so nothing jumps when the data lands. It appears only after 150 ms, so a fast answer shows no flash; until then it is hidden but keeps its space (`appear-late` in `horae.css`, which also delays `Loading` and `TableSkeleton`). Only the data region is replaced; the header, filters and actions stay live. Its rows are `aria-busy` and hidden from assistive technology, and its `label` is a visually hidden `role=status` text (`.sr-only`). Props: `rows`, `columns`, `grouped` (a group heading above the rows, for a listing grouped by day such as the audit log), `label`. The Invoices screens, People and the audit log use it. *Pending code:* the Uninvoiced view draws 5 columns for its 6. A region inside a card with its own title needs an `inset` like `EmptyState`'s, which `Skeleton` does not have yet.
- **`loaded`** (`pages::loaded`) renders a resource with all three: `Loading` before the first answer, a `LoadError` while the latest answer is a failure (a pending retry included), and the content otherwise. Its `Failure` argument carries `what`, `retry`, `pending`, `inset` and `focus`.

## Formats

- **Durations: configurable.** Each organization sets the default display format, `H:MM` (`13:00`) or decimal hours (`13.00`), and each person can override it for themselves. Teams and people read hours in one of the two, so neither fits everyone as a fixed rule.
- **One formatter.** All duration formatting lives in `horae-core`: a single function that takes integer minutes and the chosen format. The UI, the server and the reports call it the same way, so the same project never shows `13h` in one place and `13:00` in another. Page-local formatters (such as the one in `pages/projects.rs`) are removed.
- **Exports stay decimal.** CSV and XLSX exports always write decimal hours, whatever the display preference, because spreadsheets need a number they can sum.
- **Dates** use the app's format (`16 Nov 2026`) everywhere and are picked with the shared `DatePicker`.
- **Money: one formatter.** Every amount is shown through a single formatter in `horae-core`: the currency code, grouped thousands and the currency's own number of decimals, without losing exactness (`EUR 1,234.00`). The UI, the server, PDFs and emails call it the same way, and no page builds an amount by hand (such as `"{currency} "` followed by `format_cents_plain`). `format_cents_plain` stays only for the values of editable fields and for interchange formats.
- **Exports keep plain numbers.** CSV and XLSX exports write amounts as unformatted numbers, for the same reason as hours.
- **Separators are English for now** (`,` for thousands, `.` for decimals). When internationalization arrives, the money formatter takes the locale.
- *Pending code (PR-6).* Today `format_cents` always writes two decimals, and invoices and New Project build their amounts by hand without grouping (`EUR 1234.00`). PR-6 brings both in line with the rule above.

## Glossary

One word for one thing, across the sidebar, titles, buttons, menus and messages.

| Use | Instead of | Notes |
|---|---|---|
| People | Team | The section that lists the workspace's people, including its sidebar and subnavigation entries. |
| Team | — | Only the people assigned to one project: the project's Team tab and section. |
| Person | Team | The Reports grouping by person, which groups people, not teams. |
| Restore | Reactivate, Unarchive | The opposite of Archive, for projects, clients, people and tasks. Toasts read "Project restored". |
| Settings | Workspace settings | The workspace settings page (`/admin/workspace`) and its entry in the Workspace subnavigation. |
| My settings | Settings | The personal settings page (`/settings`), its title and the account-menu entry, so the two settings pages never share a name. |
| Try again | Retry, Retry report, Retry tags… | Every retry action, on error states, banners and alerts. |
| Back to *parent* | "Back to Timesheet" on a screen whose parent is another section, breadcrumbs | The back link names the parent screen (see [Page header](#page-header-srccomponents)). |
| Discard changes | Cancel changes | The escape action of a form with unsaved changes, after the kit's *Form dirty state*. |
| Cancel | Cancel over-invoicing, Cancel changes | Closes a dialog without acting, everywhere. |
| Close | Dismiss | Dismisses something that has nothing to cancel, such as a toast or a read-only panel. |

Conventions:

- **Sentence case** for titles, buttons, menu items and labels: "New invoice", "New project", "Archive client", not "New Invoice".
- **An ellipsis** ends the label of an action that opens a dialog asking for more input or a confirmation ("Delete…", "Archive client…" when it carries its projects), and only those. Labels that act at once, or that navigate, have none, so archiving a single record is "Archive".
- **Empty-state titles** have no closing period ("No projects yet"); body copy is full sentences that end in one.
- **Toast messages** name the object and what happened, in that order: "Project archived", "Client created", "Settings saved".
- **Copy is for the people who use Horae.** The interface never names environment variables, directories, internal file extensions or test cases ("Drop .wasm files into the plugins/ directory.", "If you changed accounts, reopen this page.", "Unknown currency 0.00"), and never repeats a specification's wording ("The estimate reserves nothing; generation rechecks available charges."). What the operator needs goes in `docs/`, or, on an administration screen, under a collapsed "For the operator" section: the `HORAE_HARVEST_*` variables of the Harvest OAuth screen fold there. *Pending code (PR-T).*

## Component Conventions

These rules come from the [`dioxus-best-practices`](.agents/skills/dioxus-best-practices/SKILL.md) skill, which gives the reasons and the source behind each one. `cargo run -p horae-dioxus-lint` checks several of them.

- One family per file in `crates/horae/src/components/`, declared in `components.rs`. Components that form one unit share a file (`button.rs`, `menu.rs`, `form.rs`, `toast.rs`, `card.rs`, `avatar.rs`); a control that stands on its own gets its own file. `controls.rs` is a catch-all rather than a family: `Radio`, `Segmented`, `Checkbox` and `Toggle` move to their own files when someone next changes them (`Radio` with its native-input fix, `Segmented` in PR-5), not in a PR made only to move them. A component that needs several files uses `foo.rs` beside a `foo/` directory, not `foo/mod.rs`. Add its entry to the [catalog](#catalog).
- Components are `#[component]` functions returning `Element`. Props use `snake_case`, and optional ones take `#[props(default)]`.
- Props must compare equal when nothing changed, or the component re-renders with its parent. Pass handlers made with `use_callback`, not inline closures; `EventHandler::new` and `Callback::new` are banned by `clippy.toml`. Pass `Signal`, `Memo` or `Resource` handles. Never put a `ReadSignal` inside a struct prop or an `Option<ReadSignal>` built with `.into()`. For large data, pass an id and a handle and read it inside the component.
- A `children: Element` prop makes the component re-render whenever its parent does. Take children only when the caller really supplies markup (`Modal`, `Menu`, `FormGroup`), and keep components that take them out of large table rows.
- State uses `use_signal` (local) or `use_resource` (server data), and mutations call `#[server]` functions. *Pending code:* today most pages call the server from a `spawn` of their own, each with its own busy, error and notice handling. A PR of its own unifies them on one path (`run_action` in `pages.rs`, or its successor). Shared state comes from a context that `AppLayout` provides (`use_session`, `use_running_timer`), never from a global.
- Internal navigation uses `Link`, never `a { href }`, which reloads the app.

## Tokens & utilities

Empty and error states use one look: `.empty-state` is the Projects card (see
[Empty, loading and error states](#empty-loading-and-error-states-srccomponentsscreen_staters)),
with `.empty-state-icon` as the 52px tile (`.success` and `.danger` tint it),
`.empty-state-title`, `.empty-state-text` (copy limited to 380px with 1.55 line
height), `.empty-state-code`, `.empty-state-actions` and the `.inset` modifier.
`.loading` is the inline loading line and `.sr-only` keeps text for assistive
technology without drawing it.
Screens render them through the components, not by hand. The older sunken card
with a round icon, `.empty-state-icon-tile` and `.empty-state-copy` are gone.

Everything is driven by CSS custom properties in `:root` (`assets/css/horae.css`) —
never hardcode a colour, spacing, or radius in a rule; reference the token so the
palette stays themeable and consistent. Notable token groups:

- Colour: `--color-{bg,bg-secondary,bg-tertiary,bg-overlay,menu,row-hover,row-stripe,row-hover-striped,row-selected}`, `--color-{text-strong,text,text-secondary,text-subtle,text-muted,label}`, `--color-primary*`, `--color-accent*`, and semantic `--color-{success,warning,danger,info}` with `-bg` / `-fg` / `-line` tints.
- Foreground-on-fill: `--color-on-{primary,accent,pine}` (text over a solid control).
- Calendar events: `--color-cal-{event,today}-{bg,border,ink,fg,meta}` for the Timesheet calendar's blocks (pine on most days, brass on today), defined for both themes. Ink is the project name, fg the time and task, meta the client.
- Chrome: `--color-border`, `--color-border-input`, `--color-border-strong`, `--color-border-danger`, `--color-border-danger-hover`, `--ring` / `--ring-soft` (focus glows).
- Hover fills: `--color-{primary,accent}-hover`, `--color-pine-hover`, and the brand glows `--glow-{primary,accent,pine}`.
- Elevation: `--shadow-{sm,md,menu,modal}`.
- Scale: `--space-1..16`, `--font-size-*` (`xs`–`4xl`, `display`), `--radius-*` (`sm`, base, `btn`, `lg`, `xl`, `panel`, `full`). `rounded-btn` uses the existing 8px control radius; `rounded-xl` is the 16px panel radius (see [Border Radius](#border-radius)).

On top of the tokens is a **Tailwind-style utility layer** — `flex`,
`items-center`, `justify-between`, `gap-4`, `p-4`, directional `pt-/pb-/pl-/pr-`,
`text-sm`, `text-display`, `font-semibold`, `text-secondary`, `text-label`,
`bg-secondary`, `bg-menu`, `rounded-lg`, `rounded-panel`, `shadow-menu`, … plus
responsive variants (`md:flex-row`, `lg:grid-cols-3`). The numeric spacing scale
matches Tailwind's (`p-4` = `--space-4` = 1rem). Compose utilities in markup for
layout/spacing; reach for a semantic component class (`.btn`, `.card`, `.badge`,
`.banner`, `.collapse`, `.nav-item`) for anything reused.

This layer is **generated** from the design scale by `crates/horae/build.rs` (the
Node-free equivalent of a Tailwind build) into `assets/css/horae-utils.css`, which
`app.rs` loads alongside `horae.css`. It is plain committed CSS at runtime. The
build script regenerates it on `cargo build` and only rewrites the file when the
content changes — so editing the scale in `crates/horae/build.rs` and rebuilding
keeps `horae-utils.css` in sync, and a stale committed copy can't slip through
(the sandboxed `nix build` fails if the checked-in file doesn't match).

Tokens and semantic component classes stay hand-written in `horae.css`; the
generator owns only the mechanical utility + responsive matrix.

The utility classes and options a shared component accepts (such as the
`trigger_class` of `Menu` and `Combobox`) are described in its
[catalog](#catalog) entry.

## Pages

Routes of the Dioxus router (`crates/horae/src/route.rs`). Sign-in lives outside it, in `horae-api`.

| Route | Component | Description |
|---|---|---|
| `/auth/login` | served by `horae-api` (`crates/api/src/auth/`) | Sign in through the configured OIDC provider, or the development login |
| `/` | redirect | Lands on this week's Timesheet |
| `/timesheet/:view/:date` | `Timesheet` | Day, week and calendar views, timer, entry editing and weekly submission |
| `/clients` | `ClientList` | Clients with search and filters |
| `/clients/:id` | `ClientDetail` | A client's projects, hours and invoices |
| `/projects` | `ProjectList` | Projects grouped by client, with budgets and exports |
| `/projects/client/:client` | `ProjectsForClient` | Projects filtered to one client |
| `/projects/new` | `NewProject` | Project creation with a private draft (manager/admin) |
| `/projects/:id` | `ProjectDetail` | Project dashboard, tasks and team |
| `/projects/:id/edit` | `EditProject` | The New Project form for an existing project |
| `/tasks` | `TaskCatalog` | The workspace's task catalog (see [Tasks](#tasks)) |
| `/approvals` | `Approvals` | Submitted timesheets to approve or reopen |
| `/reports` | `Reports` | Grouped and detailed time reports with exports |
| `/invoices?tab&status&client&period&q` | `InvoiceList` | Invoices and the Uninvoiced view, its filters in the query (see [Invoices](#invoices)) |
| `/invoices/new` | `NewInvoice` | New invoice, step 1: the client |
| `/invoices/new/client/:client` | `NewInvoiceForClient` | New invoice, step 2: period, projects, values and preview |
| `/invoices/new/client/:client/from/:source` | `DuplicateInvoice` | Step 2 started from invoice `source` (Duplicate) |
| `/invoices/settings` | `InvoiceSettings` | Issuer, payment details and invoice defaults, one save per card (manager/admin) |
| `/invoices/:id` | `InvoiceDetail` | One invoice as issued, its Primary by status |
| `/invoices/:id/edit` | `EditInvoice` | The draft editor |
| `/people` | `People` | People, their week's hours, rates and permissions (see [People](#people)); `/admin/users` redirects here |
| `/admin/workspace` | `WorkspaceSettings` | Settings: the workspace's own settings, one form (see [Workspace settings](#workspace-settings)) |
| `/admin/plugins` | `WorkspacePlugins` | The plugins installed on the server (see [Workspace settings](#workspace-settings)) |
| `/admin/importers` | `ImporterIndex` | Available importers |
| `/admin/importers/harvest` | `HarvestOverview` | Harvest connection, preview and import |
| `/admin/importers/harvest/oauth` | `HarvestOAuth` | Harvest OAuth app credentials |
| `/admin/importers/harvest/csv` | `HarvestCsv` | Import from a Harvest CSV export |
| `/admin/importers/harvest/runs` | `HarvestRuns` | Import run history |
| `/admin/importers/harvest/runs/:id` | `HarvestRun` | One import run |
| `/admin/audit?kind&person&period` | `PermissionAudit` | Permission audit log, its filters in the query (see [Audit log](#audit-log)) |
| `/admin/api-tokens` | `AdminApiTokens` | Every API token created in the workspace, with revocation |
| `/admin/apps` | `AdminApps` | OAuth apps people can allow to use their account (see [Apps](#apps)) |
| `/admin/apps/:id` | `AdminApp` | One app: details, actions and its authorizations |
| `/settings/profile` | `SettingsProfile` | My settings: name, email, sign-in and time zone (see [My settings](#my-settings)); `/settings` redirects here |
| `/settings/preferences` | `SettingsPreferences` | My settings: theme |
| `/settings/api-tokens` | `SettingsApiTokens` | My settings: the person's own API tokens |
| `/settings/access` | `SettingsAccess` | My settings: Your access |
| `/components` | `Gallery` | Component gallery |
| anything else | `NotFound` | Not found |

### New Project

`crates/horae/src/pages/new_project.rs` composes basic details, visibility, billing/budgets,
tasks, team and invoice defaults from sibling section modules. The static creation route
precedes the project-ID route and keeps Projects active in the existing shell; it does not
add a sidebar item or reproduce the handoff's developer controls.

The form has an independently scrolling main panel and persistent actions. Its 1,180px
maximum width, 220px label column, compact numeric fields and responsive gutters use
tokens and generated utilities. Structural `np-*` rules and the shell's `.np-page` opt-in
are local to this surface. Existing controls retain their defaults; form selectors, date
fields, compact checkboxes and validation links opt into shared capabilities.

Draft status reflects server acknowledgements. Cancel and the form's Back action preserve
the draft; Discard requires confirmation. Conditional billing fields retain raw values when hidden, and
server validation applies the selected type/mode. Recognized field errors link to their
controls and restore focus; task/person/milestone identities use UUIDs rather than row
positions. Email availability reflects the deployment configuration, not a simulated toggle.

The [New Project contract](specs/011-new-project-screen/contracts/new-project.md) maps all
handoff panels to components/tokens and records the persistence, authorization and recovery
requirements. The [task checklist](specs/011-new-project-screen/tasks.md) tracks remaining
acceptance work; this page is not evidence that every final gate has passed.

The subsections below record the redesign of four screens decided on 2026-11-20, plus where plugins live. People, My settings, Workspace settings and the audit log are built; Tasks names the PR in the inventory that builds it, and keeps its current behaviour until then.

### People

People is a main screen at `/people`, like the Timesheet, outside the admin shell (`/admin/users` redirects to it).

- **Navigation.** The main sidebar's Organize section links to it for people who may see people, under the [glossary](#glossary) word People. The admin subnavigation does not list it.
- **Header.** The title "People" and "Add person" (Primary), which opens a `Modal` with the name, the email and the profile, chosen with `Radio` cards.
- **Week and summary.** A `PeriodNav` week selector (PR-6 reuses it in Approvals), then a summary card with the week's total, billable and non-billable hours and the bar that splits them.
- **Filters.** A `FilterBar` with a search by name or email, a profile `Menu`, and a status `Menu` (Active, Archived, All; Active by default, each option with its count), as in the other catalogues. It is not a `Segmented`, which switches views and never filters a list.
- **Table.** Columns: the person (avatar, name, profile badge, email), hours with a billable/non-billable bar, billable hours, billable %, and an "Actions" `Menu`. The name is not a link, since View timesheet covers it. The menu shows only what the viewer may do: View timesheet; Edit details…, Permissions…, Rates…, Permission history (the audit log filtered to that person); a divider; Archive. *Pending code:* Edit details… waits for its own PR, since no operation edits a person's name or email yet. Archive follows [Destructive actions](#destructive-actions): it applies at once with an "Undo" toast, with no ellipsis and no danger styling.
- **Loading.** The whole list loads at once, with no pagination. Changing the week dims the rows until the new hours arrive: the screen keeps its data on reload.
- **Phases.** Phase 1 is built. Phase 2, its own PR, adds a weekly capacity per person (a new column and its editor), then utilization and available capacity. An assignments tab and Export come later.

### Tasks

*Pending code (the Tasks PR, after PR-5).*

- **Editing** happens in a dialog, not in the row.
- **Bulk selection** shows a `BulkBar` at the bottom of the screen.
- **Columns** include "Assigned by default".
- **Whole list with search**: no Previous/Next and no "Refresh tasks" (see [Data Tables](#data-tables-srccomponentstablers)).
- **Default assignees** are chosen with `PeoplePicker`; exports go through `ExportDialog`.
- **Radius** 16px, like every panel (see [Border Radius](#border-radius)).
- **Reloads** keep the list on screen, as part of the Tasks PR (see Loading in [Screen States](#screen-states)).

### My settings

My settings is split into subpages in a side menu, the way the admin shell is (`/settings/profile`, `/settings/preferences`, `/settings/api-tokens`, `/settings/access`; `/settings` opens Profile).

- **SETTINGS group:** Profile, Preferences, API tokens.
- **ACCOUNT group:** Your access. It is not a setting, so it has its own group.
- **Your access** depends on who is looking:
  - A member sees their access as a table of areas against View, Edit and Approve, then the line "To change your access, ask an administrator of this workspace."
  - An administrator sees no table, only one sentence ("You can see and change everything here…") that links to People.
  - Both see People you manage and Projects you manage.
  - The link "See changes to your access" (the audit log filtered to the person) shows only to someone who may read the audit log, today an administrator. A member gets no view of their own history and no link to the log.
  - No one sees the amber "Only an Administrator can edit your permissions" notice, and there is no "Refresh permissions". The person's profile is shown here once, not also as a disabled field in Profile.
- **Plugins** are not in My settings: they are installed per server, so they belong to [Workspace settings](#workspace-settings).

### Workspace settings

Workspace settings (`/admin/workspace`) is one form of at most `--width-form` with one Primary ("Save changes"), holding only the settings Horae stores (today the workspace time zone). Installed plugins have their own page beside it in the Workspace subnavigation (`/admin/plugins`), open to the same people: a table of name, version, status and hooks, or the full-width `EmptyState` when none is installed. Only plugins that loaded are listed. A plugin is loaded by the server installation (`HORAE_PLUGINS_DIR`, read at startup), not chosen by a person, so it is not a personal setting. How to install one is operator documentation and goes in `docs/`, not in the interface (see the copy convention in the [Glossary](#glossary)).

### Apps

Apps (`/admin/apps`) sit in the Workspace subnavigation after API tokens, open to administrators. They are the OAuth apps people can allow to use their account (spec 024).

- **List.** One `DataTable`: the app with a secondary line (its type in words, "Installed on devices" or "Runs on a server", then the start of its client id), the access in short words ("Time, projects", "Time (read)"), an Enabled or Disabled badge and People, the count of active authorizations, then the row's ⋮ menu. Five columns, so the list fits the admin panel at 1280px. The name column has the shared name width (`name-table`), and the whole row opens the app (`row-link`). Scope codes such as `time:read` never appear; the approval page, My settings and this page share the scope sentences of the catalog. Horae's own Android app always comes first. "Register app" is the page's Primary.
- **Actions.** The row menu and the app page's "Actions" menu hold Edit…, Replace secret… (an app that runs on a server only), Enable or Disable, then Delete… last as a danger item. Horae's own app has only Enable or Disable: it cannot be edited or deleted. Disabling takes effect at once and can be undone, so it has no confirmation; replacing a secret and deleting confirm through `ConfirmDialog` in the danger tone.
- **Not ready.** When the operator has not set the public address, a warning notice says that apps cannot connect yet and that registering still works. It never names a setting or a variable.
- **Register and edit** share one `Modal`: name, type (radios with their consequence; fixed once registered), redirect addresses one per line, and the access as checkboxes with the scope sentences. Choosing to add and change time entries also ticks seeing them. The server validates the addresses, and its message sits under the field it names.
- **Secret.** An app that runs on a server gets its secret once, after registering or replacing it, in the one-time reveal API tokens use (`RevealedSecret`), with its client id and one button, "I've stored it", the only way to close it (not Escape, not a click beside it). A new app's page opens after it; an app installed on devices goes straight to its page.
- **App page** (`/admin/apps/:id`): a back link to Apps, the name with its status badge, a card of details (client id with Copy, type, redirect addresses, access in sentences, who registered it and when), then its authorizations: person with email, access, allowed on, last used, and "Revoke…" in each row's ⋮ menu through `ConfirmDialog` (danger tone).

### Audit log

The audit log (`/admin/audit`) lists every permission change, newest first. Only someone who may read it opens it, today an administrator; every way into it (the admin subnavigation, People's "Permission history", Your access's "See changes to your access", the names inside the log) shows only to them.

- **Loading more.** No refresh button. A single "Load more" at the end, only when there is more.
- **Filters.** A `FilterBar` with the change type (Person, Profile, Managers), the person and the period (the last 7, 30, 90 or 365 days, or all time). The filters are the URL's query (`?kind=person&person=<id>&period=30d`), so a filtered view can be shared. The person filter shows the changes that person made and the changes made to them: their permissions, their management of projects and people, a profile they were detached from.
- **Times** are in the viewer's time zone (their own, else the workspace's), not UTC, and the line under the title names it.
- **Grouping.** Events are grouped by the viewer's day ("Today", "Yesterday", then dates). Each row is a readable sentence ("Admin User changed the permissions of Maria Ruiz", "Admin User made Jamie Chen a manager of Website Redesign") with a "+2 −1" summary and a badge for its kind (Person, Profile, Managers). The sentence says what kind of change it was (a move to a profile, becoming an administrator, a manager added); the detail says which permissions moved, since a list of permissions cannot be written as one translatable sentence.
- **Detail.** A chevron button at the end of the row (`aria-expanded`, `aria-controls` naming the detail, described by the sentence) opens and closes the detail. It is a control of its own beside the sentence, never around it, so the sentence's links stay separate focus stops; the row is not a `<details>`, whose `<summary>` would put the links inside a button. Expanding a row shows the same table of areas against View, Edit and Approve as Your access (`AccessTable`), limited to the areas that changed and with the changed cells highlighted ("All → Managed", tinted by whether the access widened or narrowed). Permissions outside the table, managers added or removed and people detached from a deleted profile are listed under it, then the receipt id with a Copy action.
- **Links.** A person's name, actor or affected, filters the log to that person, since people have no page of their own; a project links to its page.
- **Reloads** (a filter, "Load more") keep the rows on screen, dimmed and `aria-busy`, until the new rows arrive.
- **States.** Empty: its own history icon, "No permission changes yet" and a button to People. With filters and no match: "No changes match these filters" and "Clear filters". Loading: a skeleton shaped like the grouped rows (`Skeleton` with `grouped`).
- **Later, in its own PR** (server work): an optional reason on a permission change, CSV export, and "Restore this version" for custom profiles.

### Invoices

Decided on 2026-11-22, step 1 of the invoice plan (see the inventory). Statuses stay draft, sent, paid and void; Overdue is computed, never stored.

- **List.** The header holds "New invoice" (Primary) and a "More" menu with Invoice settings, for those who may edit them. Under it a summary of `MetricCard`s: Outstanding (sent and unpaid), Overdue, and Ready to invoice (clients with unbilled work, shown only to those who may prepare invoices). Amounts in different currencies are listed side by side, never added. "Paid this month" waits for payments (step 3), which bring a paid date.
- **Views.** A `Segmented` switch between Invoices and Uninvoiced, for those who may prepare invoices. Uninvoiced lists each active client with unbilled time or fees due up to today, as a new invoice for "Everything up to today" would find them, with its last invoice (not void) and its own "New invoice" (Secondary `btn-sm`, named after the client). The server answers it with one aggregated read (`ops::invoices::uninvoiced`), never a preview per client, so a long history costs one query and has no row limit.
- **Filters.** A `FilterBar`: status (`Menu`: Any status, Draft, Sent, Overdue, Paid, Void; Sent includes the overdue ones, Overdue narrows to them), client (`Combobox`), period of the issue date (`Menu`: any time, this or last month, this or last year) and a search by number or client. The whole list loads at once and the filters narrow it on the screen; "Clear filters" resets them when nothing matches. The view and the filters are the URL's query (`?tab=uninvoiced&status=overdue&client=<id>&period=last-year&q=<search>`), as in the audit log, so a filtered list can be shared and Back returns to it. A menu choice adds a history entry; the search replaces it once typing pauses.
- **Table.** One table, not a table inside a card: Number, Client, Issued, Due, Amount, Status. The number's link covers the whole row, so there is no Actions column. Status is a `Badge` (Draft warning, Sent info, Overdue danger, Paid success, Void neutral); an overdue due date is red and a void amount is struck through.
- **Overdue.** A sent invoice whose due date is before the organization's today (`horae_core::invoice::is_overdue`). The server computes it for the list and the detail, so every viewer sees the same answer.
- **New invoice, step 1** (`/invoices/new`). The client, chosen with `SelectField` and its search, then "Continue". A page, not a dialog, so the URL can be shared and Back works.
- **New invoice, step 2** (`/invoices/new/client/:client`). The client (with "Change client"), the period (`Menu`: Since the last invoice, This month, Last month, Everything up to today, or Custom dates with two `DateField`s; by default the period starts the day after the last day the client's latest non-void invoice billed, or its issue date when it billed neither, and runs to today, and a client never invoiced starts at This month; Everything up to today is never the default, so old or imported time is billed only on request), the client's projects to tick with what each has unbilled, the invoice values, the fees to invoice, then the preview: the lines the draft will hold, currency on every amount, totals inside the table. The preview follows the form once edits pause; there is no separate review button. "Create draft" is the one Primary and opens the draft editor. Fee limits and the over-invoicing confirmation are unchanged. Only what exists is offered: no blank invoice and no grouping of time into lines yet, and no disabled placeholders for them.
- **Unapproved time.** When the period holds billable time that waits for approval, step 2 says how much, that it stays out of this invoice until approved, and links to Approvals. Open time that was never submitted is invoiceable, as before, so it is not part of the warning. A period with nothing billable, for example because all of its time waits for approval, is never an error: the preview is empty ("No billable time or fees in this period") and the warning stays.
- **Failed previews.** A preview that fails keeps the form and offers "Try again"; a request the server refuses as out of date is previewed again by itself.
- **Draft editor** (`/invoices/:id/edit`). Its own screen, back to the invoice. The subject, the invoice values and the fee lines on the left; a totals card on the right, sticky on wide screens, with "Save draft" (Primary) and "Discard changes". The totals follow the form as it changes; totals that no longer match the form are dimmed until they do, and a failed review offers "Try again". Saving confirms with a toast and stays in the editor.
- **Subject.** One optional line (at most 200 characters) saying what the invoice is for. Edited in the draft editor, shown on the detail and printed on the PDF under Bill to.
- **Detail** (`/invoices/:id`). The status badge beside the title; a summary (client, issued, due, payment terms, PO number, total, subject); the lines with currency on every amount and the totals in the table's footer; the notes. The Primary follows the status: Draft "Mark as sent", Sent "Mark as paid", Paid and Void none. "Download PDF" is Secondary. "More" holds Edit draft (drafts), Duplicate, then "Void…" (danger, after a divider, through `ConfirmDialog`). Status changes confirm with a toast.
- **Duplicate.** Opens step 2 at `/invoices/new/client/:client/from/:source`, with the source's projects ticked and its values (terms, PO number, discount, taxes) filled in. The new draft takes the source's subject and notes when it is created. Time already on an invoice is never copied: the lines come from what is unbilled now. A void invoice can be duplicated, which is how one is issued again.

## Accessibility

- Keyboard navigation throughout
- ARIA labels on icon-only buttons
- Color contrast meets WCAG AA — `#EFEAE0` on `#1A1813` is ~9:1. The faintest text tokens, `--color-text-muted` and `--color-label`, are at least 4.5:1 on every resting surface in both themes, placeholders use `--color-text-muted`, and calendar event text is at least 4.5:1 on its block.
- Exception for hover: a momentary hover state may drop below 4.5:1, never below 4:1. Today that is muted and label text on `--color-bg-overlay` (the sidebar footer on hover: muted 4.20:1 dark, 4.26:1 light) and on `--color-row-hover` in the light theme (table and Projects rows on hover: muted 4.38:1, label 4.25:1). Reaching 4.5:1 there would make muted indistinguishable from `--color-text-secondary`, which is itself 4.45:1 on the light overlay.
- Focus is the `--ring`: buttons, links and focusable regions (`tabindex="0"`) show it on `:focus-visible`, and controls inside a clipping shell draw it inside instead (`--ring-inset`, or a 2px inset pine outline on the Projects scroll region, whose rows would cover an inset shadow). Elements with `tabindex="-1"` are focus targets, not controls, and get no ring.
- Form inputs have associated `<label>` elements
- Shared `Modal` dialogs stay mounted and use native `showModal()` / `close()` for background inertness and focus restoration. Escape, backdrop clicks and Cancel dismiss them unless a mutation is pending; the panel scrolls within the viewport on short screens.
- Shared `Menu` uses a native popover in the top layer so tables cannot clip it. It opens above or below its trigger within the viewport, dismisses on outside click or anchor scrolling/resizing, and supports arrows, Home/End, initial-letter navigation, Escape and Tab. Each caller supplies a stable, unique ID for the trigger/menu association. The browser owns visibility; the shared script supplies positioning and [menu keyboard behavior](https://www.w3.org/WAI/ARIA/apg/patterns/menu-button/).
- Status indicated by text + color (never color alone) — badges include both dot and text label

## Screen States

Every new or changed screen and data-bearing component covers these states before it is done; record the check in the feature's `quickstart.md` validation, or its `visual-review.md` where one exists (as in `specs/012-clients-design/increments/mvp/`). Build each state from the shared components and classes below; add a variant to them rather than a new one-off style.

- **Empty**: `EmptyState` with copy that says what to do next and, where the user can act, the action itself; not a blank table or a loose line of text.
- **Loading** works the same way on every screen:
  - The shell and the page header come first. The title, filters and actions are visible from the first frame; nothing above or around the header waits on data (Invoices no longer shows "Checking invoice recovery…" above the title: its actions wait for the check instead).
  - The session is rendered on the server, so the sidebar, its sections and every action that depends on a permission are in the first HTML, with no "Not signed in", and screens that waited on the session first lose one wave. One `use_loader` in `use_session_provider` loads the person, their permissions, the week start and the workspace name.
  - The content region shows a `Skeleton` shaped like what arrives, after 150 ms (`appear-late`). `Loading` text is only for small regions.
  - Nothing that depends on the data shows before it: no pager, no "Refresh", no totals as "—", no filters without their options.
  - A reload (a filter, a new week, a refresh after a mutation) keeps the old data on screen, dimmed and `aria-busy`, until the new data replaces it. It never swaps the table back to a loading line, and the dimming makes clear that what is shown is not yet the new answer (the Timesheet's previous week under the new dates).
  - No cascades: a screen asks for what it needs at once instead of in waves (the Timesheet waits on three today, Projects for client on three, Reports on two). Data every screen needs, such as the week start, travels with the session.
  - *Pending code:* only the session is rendered on the server; screens still load their own data on the client. Clients, Tasks and Reports unmount their table on reload, because they check `state() != Ready`; PR-C fixes that screen by screen, unless a screen's own PR (the Tasks PR) has already done it (see the inventory).
- **Error**: `.alert.alert-danger` for form errors and for a failed action on a page that otherwise loaded, inline `text-danger` for field errors, `ErrorState` (with "Try again") for a region or screen that failed to load, and `.banner-danger` for an error that needs an action inside a page. Form fields keep the values the user entered after a failed submit.
- **Success**: a toast (see [Toast](#toast-srccomponentstoastrs)); never a silent save.
- **No permission**: non-disclosing (spec 015). Name the role that can grant access ("Ask an administrator of this workspace", as `NoAccess` does) without revealing the resource, its owner or other people's permissions; out-of-scope direct links behave as not found. Hide actions the user can never take in this context; disable, with the reason, actions that are only temporarily unavailable.
  - **One form for no access.** The title is always "Access unavailable" and the body names who can grant it. Around it the screen keeps its header without actions, and shows no filters, pager or "Refresh". `NoAccess` draws it, and `LoadError` shows it for any refused read (403). The sidebar never offers a section the person cannot open (see [Sidebar](#sidebar-srccomponentssidebarrs)).
- **Long content**: 60-character names, large amounts and many rows wrap or truncate inside their column without overlapping (see Layout → Narrow content).
- **Narrow**: usable at 390px and at the 768px mobile-navigation breakpoint. How tables behave on phones is pending (see [Layout](#layout)).
- **Keyboard**: the whole flow completes without a mouse, with a visible `--ring` focus and focus returned after dialogs and menus close (see Accessibility).

## Interaction Principles

- Timer state is reactive: `TimerWidget` re-renders once a second from a task it spawns, and only while a timer runs; the timer itself is shared through `use_running_timer`
- All data mutations go through `#[server]` functions — never direct fetch calls
- Optimistic UI where appropriate; rollback on error
- Loading states shown inline, not full-page spinners, the same way on every screen (see Loading in [Screen States](#screen-states))
- **No refresh buttons.** A list refreshes by itself after the mutations made on its screen and when the browser tab regains focus (one `visibilitychange` listener in `AppLayout` that the screens' resources follow). No screen shows "Refresh", and no copy tells the user to refresh. A refresh keeps the old data on screen while it runs, so the table never blinks. Today these remain: "Refresh tasks", "Refresh details" and "Refresh access" in Projects, and "Refresh report", "Refresh breakdown" and "Refresh access" in the scoped Reports. *Pending code:* the Tasks PR removes "Refresh tasks"; PR-C adds the `visibilitychange` listener and removes the rest.
- **Catalogues load whole**, with search; only histories use "Load more" (see [Data Tables](#data-tables-srccomponentstablers)).
- **`Toggle` for what applies at once, `Checkbox` for what a form saves** (see the [catalog](#forms)).
- **Dialogs.** `Modal` draws every footer: the primary action first and on the left, then the secondary, then Cancel, the buttons sharing the width. Every confirmation goes through `ConfirmDialog`, and no dialog is made by hand (see [Dialogs](#dialogs-srccomponentsmodalrs)).
