# Horae Design Language

Horae follows the Invoicer design system: editorial warm-dark surfaces, serif display type, humanist sans-serif for UI, and monospace numerals — an accountancy aesthetic that suits time-tracking and invoicing.

## Sources and precedence

This file is the single source of truth for Horae's design. When it disagrees with anything else, this file wins.

- **`DESIGN.md`** (this file) holds the rules: tokens, components, screen states, layout and wording. A rule here overrides the kit and the prototypes, and the sections below say so wherever they differ.
- **`crates/horae/assets/css/horae.css`** implements the tokens and component classes. Where the CSS and this file disagree, the CSS is out of date and the PR that touches it brings it in line.
- **`design/project/`** is the claude.ai/design export of 2026-09-21: the kit (`foundations/Components.dc.html`), the shared prototype components (`app/*.dc.html`) and the screen prototypes (`app/01_*` to `13_*`). It is a historical visual reference. Use it for layout, proportions and copy ideas, never as a rule that overrides this file, and never edit it (see [design/README.md](design/README.md)).
- **Decisions on open questions** were taken after the [design audit of 2026-11-18](docs/design/design-audit.md). Each one is written into the section it governs, with a line on why. Two of them are still under review and are marked *Pending* where they apply: dialog footer order ([Dialogs](#dialogs-srccomponentsmodalrs)) and tables on mobile ([Layout](#layout)).
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
| `--color-text-muted` | `#7C7565` | Placeholders, section headers |
| `--color-label` | `#6A6353` | Uppercase micro-labels, table headers |

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

**Page title.** Every screen's title is Newsreader 34/600 (`4xl`) with −0.015em tracking (`tracking-tight`), in `--color-text-strong`, rendered by `PageHeader` (see [Page header](#page-header-srccomponents)). This is the size every prototype uses, so one title reads the same on every screen. The 24/500 `.page-title`, the `text-2xl` titles and the 404's 32/700 heading are retired. The sign-in shell keeps its own layout but uses the same 34/600 heading.

## Border Radius

| Token | Value | Usage |
|---|---|---|
| `--radius-sm` | `4px` | Micro elements |
| `--radius` | `6px` | Menu items, segmented inner, small controls |
| `--radius-btn` | `8px` | Buttons, inputs |
| `--radius-lg` | `11px` | Small cards, menus / popovers |
| `--radius-icon` | `12px` | Empty- and error-state icon tiles, dialog icon tiles, toasts |
| `--radius-xl` | `16px` | Grouped lists, empty- and error-state cards |
| `--radius-panel` | `20px` | Tables, modals, large panels, auth card |
| `--radius-full` | `9999px` | Badge pills, toggles |

The kit draws modals and `ConfirmDialog` at 16px; Horae's dialogs use `--radius-panel`, as the kit's own token table says.

## Layout

- **Sidebar** (`--sidebar-width: 264px`): full-height left rail on the warm dark surface (`--color-bg-secondary`), 1px right border, sticky. Holds the brand, a start-timer action, grouped navigation, and the signed-in user; there is no separate top nav. Collapses to a 68px icon strip.
- **Content area**: scrollable, generous padding, `--color-bg` (`#100F0C`) background.
- **Mobile navigation** (up to 768px): a labelled toggle expands the same sidebar above the content. It stays in normal document flow, so short screens can scroll to every link. Selecting a link closes the panel and focuses the content; Escape closes it and returns focus to the toggle. Desktop width and collapse state are preserved independently.
- **Narrow content**: page headers and actions wrap; the admin subnavigation moves above its panel when the two columns no longer fit. Dense project/time grids and the day-total strip scroll within their own containers. Project names and badges wrap inside an aligned name column instead of overlapping budget figures.
- **Wide tables at desktop width**: at 1280px with the expanded sidebar, every column of a table or grid is visible without scrolling. The Timesheet week (name, seven days, Total and the delete column) must fit in that space; today Total and delete fall outside it, which is a bug. The name column has a minimum width, as `.client-table` already does with `--width-client-name`, so names wrap inside it instead of breaking one word per line.
- **Tables on mobile: *Pending design*.** Neither this file nor the prototypes (which stop at 900px) cover phones. The direction under review is a mixed rule: list tables stack each row as a card, and wide data tables (the Timesheet week, reports) keep horizontal scrolling with a visible indicator that more columns exist. The stacked card and the Timesheet at 390px still need designing, so do not build either until this note is replaced by a rule.

## Component Inventory

Every shared component lives in `crates/horae/src/components/`, one module per file, declared in `components.rs`. File names below are relative to that directory; the paths in the rule headings further down (`src/components/…`) are relative to `crates/horae/`. The catalog says which component to reach for. The subsections after it (from [Sidebar](#sidebar-srccomponentssidebarrs) to [Empty, loading and error states](#empty-loading-and-error-states-srccomponents)) describe how each family must look and behave. Where the code does not match those rules yet, the status column names the PR that closes the gap, and [`design/IMPLEMENTATION-GAPS.md`](design/IMPLEMENTATION-GAPS.md) tracks it.

Status:

- **in use**: screens call it, and any hand-written copies of its markup are fewer than the calls.
- **underused**: it exists, but screens write the same markup by hand more often than they call it, or never call it. The figure is calls outside `components/` and the gallery (`pages/gallery.rs`) against hand-written copies, counted on 2026-11-19. In new code, call the component.
- **planned in PR-N**: the rule exists and the component does not. Build the state by hand, as the rule describes, until that PR lands, and do not invent its API.

### Summary

| Component | File | Family | Status |
|---|---|---|---|
| `AppLayout` | `layout.rs` | Shell and navigation | in use (route layout) |
| `Sidebar` | `sidebar.rs` | Shell and navigation | in use (inside `AppLayout`) |
| `AdminShell` | `admin_shell.rs` | Shell and navigation | in use (route layout); its back link goes in PR-2 |
| `PageHeader` | — | Shell and navigation | planned in PR-2 |
| `NavItem` | `nav.rs` | Shell and navigation | underused (0 calls; the rail renders its own `nav-item` links) |
| `NavIcon` | `icons.rs` | Shell and navigation | in use (13) |
| `HoraeMark` | `logo.rs` | Shell and navigation | in use (sidebar brand) |
| `TimerWidget` | `timer_widget.rs` | Shell and navigation | in use (sidebar) |
| `VersionInfo` | `version_info.rs` | Shell and navigation | in use (sidebar) |
| `ThemeInit` | `theme.rs` | Shell and navigation | in use (`app.rs`) |
| `Button` | `button.rs` | Actions | underused (0 calls, 263 hand-written `btn` classes) |
| `IconButton` | `button.rs` | Actions | underused (0 calls) |
| `SplitButton` | `button.rs` | Actions | underused (0 calls; the Timesheet builds `.ts-submit` by hand) |
| `Menu`, `MenuItem`, `MenuDivider` | `menu.rs` | Menus and pickers | in use (10, 17 and 3) |
| `SelectField` | `select_field.rs` | Menus and pickers | in use (11) |
| `Combobox` | `combobox.rs` | Menus and pickers | in use (1) |
| `DatePicker` | `date_picker.rs` | Menus and pickers | in use (2; 7 `Input { kind: "date" }` move to it in PR-6) |
| `ProjectTaskPicker` | `project_task_picker.rs` | Menus and pickers | in use (`TimerWidget`); renders native `<select>`s until PR-5 |
| `FilterBar`, `SearchField` | — | Menus and pickers | planned in PR-5 |
| `PeriodNav` | — | Menus and pickers | planned in PR-6 |
| `FormCard` | `form.rs` | Forms | in use (2) |
| `FormGroup` | `form.rs` | Forms | in use (47; 6 hand-written `form-group`) |
| `Input` | `form.rs` | Forms | in use (51; 9 hand-written `form-input`) |
| `Textarea` | `form.rs` | Forms | in use (3) |
| `Select` | `form.rs` | Forms | retired: 7 calls and 8 bare `select {}` move to `SelectField` in PR-5 |
| `Checkbox` | `controls.rs` | Forms | in use (20; 1 native checkbox) |
| `Radio` | `controls.rs` | Forms | underused (0 calls, 6 native radios in New Project) |
| `Toggle` | `controls.rs` | Forms | underused (0 calls, gallery only) |
| `Segmented` | `controls.rs` | Forms | underused (2 calls, 5 hand-written groups) |
| `DataTable` | `table.rs` | Data display | in use (9; 8 hand-written table wrappers) |
| `Badge` | `badge.rs` | Data display | in use (15; 14 hand-written `badge` spans) |
| `Card` | `card.rs` | Data display | underused (0 calls, 31 hand-written `card` classes) |
| `MetricCard` | `card.rs` | Data display | underused (0 calls, gallery only) |
| `Avatar` | `avatar.rs` | Data display | in use (3, plus the sidebar) |
| `Chip` | `avatar.rs` | Data display | underused (0 calls; New Project's tag chips are hand-written) |
| `Toast`, `ToastContainer` | `toast.rs` | Feedback | in use (3 pages, each with its own container); one container in `AppLayout` and the new look in PR-9 |
| `NoTrackingHint` | `no_tracking.rs` | Feedback | in use (Timesheet, `TimerWidget`) |
| `EmptyState`, `ErrorState`, `Loading`, `Skeleton` | — | Feedback | planned in PR-4 |
| `Modal` | `modal.rs` | Overlays | in use (18); fixed slots in PR-3 |
| `ConfirmDialog` | — | Overlays | planned in PR-3 |
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
- `AdminShell` (`admin_shell.rs`) wraps the Workspace and Data sections (People, Settings, API tokens, Importers, Audit log) with their subnavigation and checks that the person may open the section. The router mounts it with `#[layout(AdminShell)]`. No props. PR-2 removes its "Back to Timesheet" link (see [Page header](#page-header-srccomponents)).
- `PageHeader`: planned in PR-2. See [Page header](#page-header-srccomponents).
- `NavItem` (`nav.rs`) is a rail row drawn as a `button` with an `onclick`, an optional text icon and the active dot. The rail itself renders `Link`s with the same `nav-item` classes, because navigation must be a link (see [Component Conventions](#component-conventions)), so `NavItem` only appears in the gallery. Do not use it for navigation. Props: `label`, `icon`, `active`, `onclick`.
- `NavIcon` (`icons.rs`) draws one of the 16px stroked glyphs (`timesheet`, `clients`, `projects`, `arrow-left`, `chevron-down`, `pencil`, `info` and the rest) in `currentColor`. Use it for every icon these names cover rather than a text glyph. An unknown name renders nothing. Props: `name`, `class`.
- `HoraeMark` (`logo.rs`) is the inline Horae logo, coloured by the `--icon-*` tokens so it follows the theme. It is used only by the sidebar brand. No props.
- `TimerWidget` (`timer_widget.rs`) is the rail's timer: a "Start timer" button, the project and task picker, then the running time with a stop button. It is mounted once, by the sidebar. To start, stop or edit time from a page, call `use_running_timer().refresh()` after the change instead of rendering a second widget. No props. See [Timer Widget](#timer-widget-srccomponentstimer_widgetrs).
- `VersionInfo` (`version_info.rs`) shows the server and front-end build labels and offers a reload when they differ. The sidebar shows it to administrators. No props.
- `ThemeInit` (`theme.rs`) applies the saved theme before the first paint and defines `setHoraeTheme()`. `app.rs` mounts it once in the head. No props.

#### Actions

- `Button` (`button.rs`) is a `button` with the `btn btn-{variant}` classes, an optional `sm` size, `disabled` and `onclick`. It is the intended way to draw an action, but it takes no `type`, `id`, `aria-label` or test attributes, so screens write `button { class: "btn …" }` by hand instead. In new code, use the component when those props are enough; otherwise write the classes, using only the variants in [Buttons](#buttons-srccomponents). Props: `variant` (default `primary`), `size`, `disabled`, `onclick`, `children`.
- `IconButton` (`button.rs`) is a square icon-only `btn-icon` button with a required `label` for its accessible name. Use it for an action shown only as an icon. Props: `variant` (default `ghost`), `label`, `disabled`, `onclick`, `children`.
- `SplitButton` (`button.rs`) is the `.btn-split` pair: a main action and a caret button. The caret only fires `ontrigger`; opening a menu of variants is the caller's job. Props: `variant`, `label`, `trigger`, `onclick`, `ontrigger`. See Split button in [Buttons](#buttons-srccomponents).

#### Menus and pickers

- `Menu`, `MenuItem`, `MenuDivider` (`menu.rs`) are the dropdown: a trigger button with a caret that opens a native popover of items. Use `Menu` for a row's or a page's "Actions" and for list filters. It is not a form field: to choose a value in a form, use `SelectField`. `MenuItem` takes `selected` (for a filter's current value), `danger` (Delete, last, after a `MenuDivider`) and `disabled`. Props of `Menu`: `id` (stable and unique; it ties the trigger to the menu), `label`, `align_right`, `trigger_class`, `disabled`, `children`; see [Tokens & utilities](#tokens--utilities) for `trigger_class` and `disabled`. Rules in [Data Tables](#data-tables-srccomponentstablers), [Destructive actions](#destructive-actions) and [Accessibility](#accessibility).
- `SelectField` (`select_field.rs`) is the form selector: a full-width trigger styled as an input, opening a listbox on the menu surface. Give it a `query` signal to add a search box for long lists, and `pending` while the options load. Use it for every single-value choice in a form, in place of `Select` and native `<select>`. Props: `id`, `label`, `options: Vec<(value, label)>`, `selected`, `onselect`, `error_id`, `disabled_values`, `query: Option<Signal<String>>`, `pending`, `children` (rendered under the list). See [Selects, filters and search](#selects-filters-and-search-srccomponentsselect_fieldrs-menurs).
- `Combobox` (`combobox.rs`) is a compact searchable picker drawn as a `btn-sm` trigger, with an "All" row that clears the choice and options that can be grouped under headings (`ComboOption::grouped`). Its one call is the client filter in Projects. Use it to filter a list by one of many values; in a form, use `SelectField` with a `query`. Props: `options: Vec<ComboOption>`, `value`, `placeholder`, `all_label`, `trigger_class`, `onselect`.
- `DatePicker` (`date_picker.rs`) is the month calendar panel. It highlights a day or, with `week`, the whole week, and emits the picked date. It draws only the panel: the caller supplies the trigger and the popover, as `pages/new_project/date_field.rs` and the Timesheet pager do. Props: `selected: NaiveDate`, `week`, `first_day`, `onpick`. See [Formats](#formats).
- `ProjectTaskPicker` (`project_task_picker.rs`) is the pair of project and task fields for new time. It offers only the projects and tasks the server says the person can track, and clears a selection that falls out of that list. Use it wherever new time is entered; the timer uses it today. Props: `project` and `task` (`Signal<String>`), `projects` and `tasks` (resources), `disabled` (keeps an existing entry's names read-only). It still renders native `<select>`s, which PR-5 replaces.
- `FilterBar` and `SearchField`: planned in PR-5. `PeriodNav`: planned in PR-6. See [Selects, filters and search](#selects-filters-and-search-srccomponentsselect_fieldrs-menurs).

#### Forms

- `FormCard` (`form.rs`) is the card around an inline create or edit form: a small uppercase title, the form's error as an `.alert-danger`, then the fields. Props: `title`, `error: ReadSignal<Option<String>>`, `children`.
- `FormGroup` (`form.rs`) pairs a label with its control and an optional hint. Give it the control's `id` so the label points at it. Props: `label`, `id`, `hint`, `children`. See [Forms](#forms-srccomponentsformrs).
- `Input` (`form.rs`) is the text input (`kind` sets the HTML type). Use `label` when there is no visible `FormGroup` label, and `error_id` to point at the field's visible error. Props: `kind`, `id`, `label`, `class`, `value`, `placeholder`, `disabled`, `readonly`, `error_id`, `oninput`. Do not use `kind: "date"`: dates use `DatePicker` (see [Formats](#formats)).
- `Textarea` (`form.rs`) is the multi-line input. Props: `id`, `value`, `placeholder`, `rows` (default 3), `disabled`, `error_id`, `oninput`.
- `Select` (`form.rs`) renders a native `<select>` and is retired. Do not add calls; use `SelectField`. See [Selects, filters and search](#selects-filters-and-search-srccomponentsselect_fieldrs-menurs).
- `Checkbox` (`controls.rs`) is the labelled two-state check drawn as a `button` with `role=checkbox`. `compact` keeps only the accessible name (table rows), `mixed` shows the indeterminate state of a select-all, and `error_id`/`hint_id` link it to its messages. Use it instead of a native checkbox. Props: `checked`, `label`, `id`, `compact`, `mixed`, `disabled`, `error_id`, `hint_id`, `onclick`. See [Tokens & utilities](#tokens--utilities).
- `Radio` (`controls.rs`) is one labelled option drawn as a `button` with `role=radio`. Props: `selected`, `label`, `onclick`.
- `Toggle` (`controls.rs`) is an on/off switch (`role=switch`) with an optional label, for a setting that applies at once. Props: `on`, `label`, `onclick`.
- `Segmented` (`controls.rs`) shows several labelled options with one active, and emits the chosen label. Use it to switch between views of the same content (Timesheet day, week and calendar; the Approvals status switch), not to filter a list (see `FilterBar` in [Selects, filters and search](#selects-filters-and-search-srccomponentsselect_fieldrs-menurs)). Props: `items: Vec<String>`, `active`, `onselect`.

#### Data display

- `DataTable` (`table.rs`) wraps a `table` in `.table-container` (border and panel radius). Put every data table in it. Props: `children`. See [Data Tables](#data-tables-srccomponentstablers).
- `Badge` (`badge.rs`) is the status pill with a dot. Show every status with it, never as plain text. Props: `variant` (`success`, `warning`, `danger`, `info`, `neutral` or `pine`; the class is `badge-{variant}`), `children`. The small size (`badge-sm`) has no prop yet, which is why screens write some badges by hand. See [Status Badges](#status-badges-srccomponentsbadgers).
- `Card` (`card.rs`) is a `.card` panel with an optional `h3` title. Its fixed `h3` and padding do not fit the screens' sections, which write `.card` by hand. Props: `title`, `children`.
- `MetricCard` (`card.rs`) is one figure: an uppercase label, a mono value and an optional delta (`direction` is `up`, `down` or empty). Props: `label`, `value`, `delta`, `direction`.
- `Avatar` (`avatar.rs`) is a person's round avatar with an image or initials; `empty` draws the dashed placeholder of an unassigned slot. Props: `initials`, `image_url`, `size` (`sm`, empty or `lg`), `empty`. `first_initial` in the same file gives the one-letter fallback.
- `Chip` (`avatar.rs`) is a compact person chip (avatar and name), or with `plain` a tag chip without the avatar. It has no remove button, so it does not cover New Project's removable tags. Props: `label`, `initials`, `plain`, `variant`.

#### Feedback

- `Toast` and `ToastContainer` (`toast.rs`) confirm a completed action: a `role=status` notice with an optional action ("Undo") and close button, inside a fixed stack. Today each page renders its own container; PR-9 moves the single container into `AppLayout` and gives the toast its bottom-centre look. Props of `Toast`: `message`, `variant` (`success`, `danger`, `warning`, `info` or empty), `icon`, `dismissible`, `ondismiss`, `action_label`, `action_busy`, `onaction`. See [Toast](#toast-srccomponentstoastrs).
- `NoTrackingHint` (`no_tracking.rs`) explains why the person cannot track time (no project assigned) wherever a start or add control would sit disabled without a reason, and links managers to Projects. Point the disabled control's `aria-describedby` at its `id`. Props: `id`, `assignee`, `can_manage`, `on_follow`. `no_tracking_message` gives the same sentence as text.
- `EmptyState`, `ErrorState`, `Loading` and `Skeleton`: planned in PR-4. See [Empty, loading and error states](#empty-loading-and-error-states-srccomponents).

#### Overlays

- `Modal` (`modal.rs`) is the native `dialog` every dialog is built on. Keep it mounted and toggle `open`: closing returns focus to the opener, and `busy` blocks Escape and backdrop dismissal while a mutation runs. Props: `id`, `labelledby` (the title's id), `open`, `busy`, `large`, `focus_fallback`, `on_dismiss`, `children`. The header, body and footer slots arrive in PR-3; until then use the `.modal-*` classes. See [Dialogs](#dialogs-srccomponentsmodalrs) and [Accessibility](#accessibility).
- `ConfirmDialog`: planned in PR-3. See [Dialogs](#dialogs-srccomponentsmodalrs) and [Destructive actions](#destructive-actions).

#### Helpers

These modules render nothing (or almost nothing) but are shared the same way.

- `session.rs`: `use_session()` returns the signed-in person (`me`) and their permissions, loaded once by `AppLayout`. Use it to show or hide UI, never to authorize. Call `refresh()` after a change to the person's own access. `use_week_start()` returns the organization's first weekday, fetched on first use.
- `timer_widget.rs`: `use_running_timer()` returns the running timer that the rail shows. After any change to time entries, call its `refresh()`. Its `changes()` counter lets a page's entry list follow a timer started from the rail.
- `today.rs`: `today()` is the person's current date in their profile's time zone. Use it instead of `chrono::Local` or `Utc::now()` in UI code.
- `debounce.rs`: `use_debounced(signal, SEARCH_DEBOUNCE)` gives a copy of a search or filter value that updates once typing pauses, so a resource reading it fetches once per pause.
- `permission_description.rs`: `permission_description(permission)` is the one sentence that names each permission in the UI. It is display copy and grants nothing.
- `theme.rs`: `Theme` lists the dark and light palettes for the Settings choice (`ThemeInit` is above).

### Sidebar (`src/components/sidebar.rs`)

Full-height left rail. The brand row leads with the Horae mark and wordmark plus the golden accent dot. Nav rows default to `--color-text-secondary`; hover darkens the background to `--color-bg-tertiary`; the active row raises to `--color-bg-tertiary` with a `--color-border` hairline, tints its glyph `--color-primary` and closes with a trailing pine dot. Section labels: all-caps, wide letter-spacing, muted warm gray. Navigation labels follow the [glossary](#glossary) ("People", not "Team").

### Page header (`src/components/`)

One `PageHeader` component renders the top of every screen: an optional back link on its own row, then the title (see [Type scale](#type-scale)) and the screen's actions. Header actions use the normal button size, never `btn-sm`, and wrap the same way on every screen. One shared header keeps the title, the way back and the actions in the same place everywhere.

- **Back link**: the kit's *Back link*. A 16px arrow and a 14px label on their own row above the title, muted ink that brightens to pine on hover. It points to the parent in the hierarchy ("Back to Projects" from a project, "Back to Importers" from Harvest), not to the screen the user came from, and only screens that sit under a section have one. Sections reached from the sidebar (Timesheet, Projects, People, Settings, API tokens, Importers, the audit log and the rest) have none, because the sidebar is always there; the `AdminShell`'s "Back to Timesheet" link is removed.
- **No breadcrumbs.** The back link replaces a breadcrumb; never show both. The Harvest section's `BackLinks` breadcrumb (`Importers / Harvest / …`) is replaced by a back link to the parent. No other style of "back" is allowed: no `btn-ghost` or `btn-secondary` back buttons and no top-right back button.

### Timer Widget (`src/components/timer_widget.rs`)

`HH:MM:SS` in IBM Plex Mono (`--font-family-mono`), pine-300 when stopped, green (`--color-success`) when running.

### Data Tables (`src/components/table.rs`)

Wrapped in `.table-container` (border + 20px panel radius). Headers: uppercase, 0.1em letter-spacing, transparent surface, label-tone ink (`--color-label`). Row hover sinks to `--color-row-hover`. Last row has no bottom border. Status is always a `Badge`, never plain text. A record's actions (edit, archive, permissions, rates…) go in one compact "Actions" menu per row (`Menu`), the same way on every table, rather than as loose buttons that wrap at narrow widths. One exception: Approvals keeps "Approve" and "Reopen" inline in each row, as Secondary `btn-sm` buttons, because approving row by row is the screen's job and a menu would add a click to every approval. See [Layout](#layout) for column widths and mobile behaviour.

### Forms (`src/components/form.rs`)

Inputs on `--color-bg` (void — darkest), 8px radius, 44px tall, warm border (`--color-border-input`). Idle hover lifts the border to `--color-border-strong` and warms the fill; focus ring is `--color-primary` border + 18% pine glow. Instrument Sans for all form text.

### Selects, filters and search (`src/components/select_field.rs`, `menu.rs`)

- **Never a native `<select>`.** Picking one value from a list always uses Horae's own dropdown. In forms that is `SelectField` (a full-width trigger on the shared menu surface, with optional search); `Combobox` is the searchable variant for long catalogs. The kit asks for custom dropdowns everywhere, and a native `<select>` draws a different arrow and list in each browser.
- **`SelectField` and `form::Select`.** `form::Select` renders a native `<select>`, so it is retired: its callers move to `SelectField`, and the two are unified into one form selector. Until that lands, use `SelectField` in new code.
- **`FilterBar`.** Every list that can be filtered puts its filters in one shared `FilterBar`: a row of `Menu` triggers (with a trailing caret when they open a menu and a pine count when active), plus the search field when the list has one. This replaces the four current shapes (menus in Projects and Clients, native selects labelled "Show" in Tasks and People, a card of labelled selects in Reports). The Approvals status switch stays a `Segmented` control because it switches views, not filters.
- **`SearchField`.** One shared search field, always with the magnifier glyph inset on the left. It has the same border, height and focus ring as any other input.
- **Dates** are chosen with the shared `DatePicker` (`src/components/date_picker.rs`), never a native `<input type=date>`, whose display format depends on the browser locale (`11/01/2026` in US English). See [Formats](#formats).

### Status Badges (`src/components/badge.rs`)

Pill shape (9999px radius, `4px 12px`). Every variant — including neutral — carries a 7px status dot (via `::before`) over a tinted background + warm border. Neutral uses the `--color-menu` surface with a `--color-border-strong` dot.

### Buttons (`src/components/`)

Control-lg is 44px tall (`12px 20px`). Brand fills **brighten** on hover with a soft glow; neutral controls lift a shade and raise their border. Weights: primary/accent 700, secondary/danger/ghost 500.

- **Primary**: pine-300 fill, dark green text `#0d211b`; hover → `--color-primary-hover` + `--glow-primary`. The main action of a screen or form, including form submits. Use at most one Primary per screen or form, so the next step is always obvious. A screen made of several independent forms (Invoice settings saves each card on its own) has one Primary per form.
- **Secondary**: tertiary surface, `--color-border-input` border; hover raises the border to `--color-border-strong`
- **Accent**: brass fill `#D99A3C`, dark text — "Send invoice" and billing actions; hover → `--color-accent-hover` + `--glow-accent`
- **Danger** (`btn-danger`): transparent with dark red border `#6e3634`; hover tints and lifts the border to `--color-border-danger-hover`. Only for the button that opens a destructive confirmation (see [Destructive actions](#destructive-actions)); never for the confirmation itself and never for archiving.
- **Danger solid**: red fill with dark ink, the kit's `hb-danger-solid` from `ConfirmDialog.dc.html`. Only for the confirm button of an irreversible action. The class does not exist in the app yet (PR-3 adds it).
- **Ghost**: transparent pine-ink link; hover → pine-tint background
- **Split button** (`.btn-split`): a main action with real variants behind a caret, such as submitting a week with or without notifying. Its body is Primary. The Timesheet's "Submit week" is a `.btn-split` with a Primary body; its own `.ts-submit` button is retired.

**Solid is retired.** The deep-pine button (`.btn-solid`) is no longer a variant. The kit and several prototypes use it for submits ("Submit week", "Update info", "Save", "Approve"), and this file overrides them: Approvals' "Approve all" and Invoice settings' submits are Primary, the per-row "Approve" is Secondary `btn-sm`, and "Submit week" is the Primary split button above. Once those move, PR-8 deletes `.btn-solid` and `--glow-pine`. `--color-pine` and `--color-pine-hover` stay, because segmented controls, toggles, the calendar and `.badge-pine` use them. One Primary look for every main action means no screen's next step looks different from the others.

### Destructive actions

- **Irreversible actions** (deleting, disconnecting Harvest, revoking a token, and anything else that cannot be undone) are confirmed in a `ConfirmDialog` whose confirm button is Danger solid. A red fill marks the one click that loses data.
- **The button that opens that dialog** uses Danger (`btn-danger`, the red outline). The outline warns; the fill commits.
- **Archiving is not destructive.** It can be undone, so it never uses either red button, and its success toast offers "Undo". Archiving one record (a project, a task, a person) happens at once, with no dialog: the "Undo" is enough. It asks for confirmation only when it carries other records with it (a client and its projects) or acts on several at once (bulk archive); that dialog uses the neutral tone with a Primary button, as the prototypes' archive dialogs do. Restoring works the same way.
- **Record actions** such as Edit, Archive, Restore and Delete are offered through the row's or the page's "Actions" menu, the same way on every screen. Delete sits last, after a divider, as a danger item.

### Dialogs (`src/components/modal.rs`)

Every dialog is built on the shared `Modal`, with fixed slots (header, body, footer) so none of them render unpadded or with a title of their own making. `.ts-modal-*` and hand-made `h2`/`p` dialogs are retired.

**`ConfirmDialog`** is the one shape for confirming a destructive, irreversible or otherwise significant action, after `ConfirmDialog.dc.html`: an icon tile (red for danger, green for a neutral confirm such as archiving), a serif title, a short body that says what will happen, then the buttons. For the most serious actions (deleting a workspace, for instance) it can require typing a confirmation word before the confirm button enables. Its props follow the prototype: title, body, confirm label, tone (danger or neutral), busy, confirm and dismiss handlers, and the optional confirmation word.

**Footer order and alignment: *Pending decision*.** The kit's *Modal* puts the primary action on the right with Cancel to its left. `ConfirmDialog.dc.html`, most prototype screens and the app today put the primary first, on the left (the app also stretches the buttons to full width). Until this is decided, keep each dialog's current order and do not reorder footers as part of other work.

### Toast (`src/components/toast.rs`)

Every successful action is confirmed with a toast, the one in `Toast.dc.html`, so the user always gets the same signal in the same place:

- Bottom centre of the viewport, on a black (`#000`) panel with a hairline border, in both themes.
- A leading icon by kind: success, info, warning or error.
- The message, an optional inline action ("Undo" after archiving or restoring) and a close button. A long message wraps onto several lines instead of being truncated, so the toast never cuts off what it confirms; the prototype's single truncated line is overridden.
- One container in `AppLayout` shows them, so pages raise toasts instead of rendering their own.

`.alert-success` stays only for a state that must remain visible on the page (an import that finished, say), not to confirm an action. Inline "Saved." text, green `text-success` notes and `role=status` paragraphs used as confirmations are replaced by the toast. The kit's *Form dirty state* asks for the same: saving confirms with a toast, never silence.

### Empty, loading and error states (`src/components/`)

- **`EmptyState`**: the Projects variant is the base of `.empty-state` for every screen: a `--color-bg-secondary` card with a border and `--radius-xl` (16px), the icon in a 52px tile (`--radius-icon`, 22px glyph), a Newsreader `xl` title, copy limited to `--width-empty-copy`, and an optional action (Primary, or Secondary when it only resets filters) plus an optional text link. The copy says what to do next. The kit and `EmptyState.dc.html` draw the icon in a round disc; Horae uses the tile. The `tone="success"` variant (an all-clear moment such as nothing to approve) keeps the green icon colours.
- **`ErrorState`**: for a region or screen that failed to load. It keeps the empty state's silhouette so the eye lands in the same place, including the 52px icon tile (the prototype's round disc is overridden here too), with the icon in the danger tint, a title and body that say what happened and what to do, an optional mono request line for support, and "Try again" as the action. A permission error points to who can grant access instead of offering a retry (see [Screen States](#screen-states)).
- **`Loading`**: one component for every inline loading message, always with `role=status` and `aria-busy`, so assistive technology hears the same thing everywhere. It replaces the ad hoc `p { "Loading…" }` variants.
- **`Skeleton`** (optional): a placeholder that mirrors a listing's grid, for tables and lists where the layout is known before the data arrives. Only the data region is replaced; the header, filters and actions stay live.

## Formats

- **Durations: configurable.** Each organization sets the default display format, `H:MM` (`13:00`) or decimal hours (`13.00`), and each person can override it for themselves. Teams and people read hours in one of the two, so neither fits everyone as a fixed rule.
- **One formatter.** All duration formatting lives in `horae-core`: a single function that takes integer minutes and the chosen format. The UI, the server and the reports call it the same way, so the same project never shows `13h` in one place and `13:00` in another. Page-local formatters (such as the one in `pages/projects.rs`) are removed.
- **Exports stay decimal.** CSV and XLSX exports always write decimal hours, whatever the display preference, because spreadsheets need a number they can sum.
- **Dates** use the app's format (`16 Nov 2026`) everywhere and are picked with the shared `DatePicker`.

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

## Component Conventions

These rules come from the [`dioxus-best-practices`](.agents/skills/dioxus-best-practices/SKILL.md) skill, which gives the reasons and the source behind each one. `cargo run -p horae-dioxus-lint` checks several of them.

- One component per file in `crates/horae/src/components/`, declared in `components.rs`. A component that needs several files uses `foo.rs` beside a `foo/` directory, not `foo/mod.rs`. Add its entry to the [catalog](#catalog).
- Components are `#[component]` functions returning `Element`. Props use `snake_case`, and optional ones take `#[props(default)]`.
- Props must compare equal when nothing changed, or the component re-renders with its parent. Pass handlers made with `use_callback`, not inline closures; `EventHandler::new` and `Callback::new` are banned by `clippy.toml`. Pass `Signal`, `Memo` or `Resource` handles. Never put a `ReadSignal` inside a struct prop or an `Option<ReadSignal>` built with `.into()`. For large data, pass an id and a handle and read it inside the component.
- A `children: Element` prop makes the component re-render whenever its parent does. Take children only when the caller really supplies markup (`Modal`, `Menu`, `FormGroup`), and keep components that take them out of large table rows.
- State uses `use_signal` (local) or `use_resource` (server data), and mutations call `#[server]` functions. Shared state comes from a context that `AppLayout` provides (`use_session`, `use_running_timer`), never from a global.
- Internal navigation uses `Link`, never `a { href }`, which reloads the app.

## Tokens & utilities

Empty states use one variant: `.empty-state` takes the Projects card (see
[Empty, loading and error states](#empty-loading-and-error-states-srccomponents)).
Until PR-4 folds them into the base class, that look is composed from
`.empty-state-icon-tile` (the 52px tile, 12px radius and 22px glyph tokens),
`.empty-state-copy` (copy limited to 380px with 1.55 line height) and card
spacing/radius utilities in markup. The older sunken card with a round icon is
retired.

Everything is driven by CSS custom properties in `:root` (`assets/css/horae.css`) —
never hardcode a colour, spacing, or radius in a rule; reference the token so the
palette stays themeable and consistent. Notable token groups:

- Colour: `--color-{bg,bg-secondary,bg-tertiary,bg-overlay,menu,row-hover}`, `--color-{text-strong,text,text-secondary,text-subtle,text-muted,label}`, `--color-primary*`, `--color-accent*`, and semantic `--color-{success,warning,danger,info}` with `-bg` / `-fg` / `-line` tints.
- Foreground-on-fill: `--color-on-{primary,accent,pine}` (text over a solid control).
- Chrome: `--color-border`, `--color-border-input`, `--color-border-strong`, `--color-border-danger`, `--color-border-danger-hover`, `--ring` / `--ring-soft` (focus glows).
- Hover fills: `--color-{primary,accent}-hover`, `--color-pine-hover`, and the brand glows `--glow-{primary,accent,pine}`.
- Elevation: `--shadow-{sm,md,menu,modal}`.
- Scale: `--space-1..16`, `--font-size-*` (`xs`–`4xl`, `display`), `--radius-*` (`sm`, base, `btn`, `lg`, `xl`, `panel`, `full`). `rounded-btn` uses the existing 8px control radius; `rounded-xl` adds the refreshed handoff's 16px grouped-list radius without changing existing cards.

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

`Menu` and `Combobox` accept optional `trigger_class` utilities. Their default
remains `btn btn-secondary btn-sm`; a surface may opt into `text-sm px-4` without
changing other callers or adding page-specific descendant overrides.

`Menu` can opt into a native `disabled` trigger. Projects uses this when its
selection is empty or a bulk operation is pending; other menus remain enabled.

`Checkbox` keeps its labelled, two-state defaults. Opt into `compact` for an
accessible-name-only table control, `mixed` for select-all's indeterminate state,
and `disabled` while submitting. `--size-checkbox` preserves the existing 18px
square. Projects adds its selection track only through `.proj-grid-selectable`;
read-only rows and other shared controls retain their existing layout.

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
| `/tasks` | `TaskCatalog` | The workspace's task catalog |
| `/approvals` | `Approvals` | Submitted timesheets to approve or reopen |
| `/reports` | `Reports` | Grouped and detailed time reports with exports |
| `/invoices` | `InvoiceList` | Invoice table |
| `/invoices/new/client/:client` | `NewInvoiceForClient` | Invoice preparation for one client |
| `/invoices/settings` | `InvoiceSettings` | Issuer, payment details and invoice defaults, one save per card (manager/admin) |
| `/invoices/:id` | `InvoiceDetail` | Invoice line items |
| `/admin/users` | `AdminUsers` | People, their rates and permissions |
| `/admin/workspace` | `WorkspaceSettings` | Settings: the workspace's own settings |
| `/admin/importers` | `ImporterIndex` | Available importers |
| `/admin/importers/harvest` | `HarvestOverview` | Harvest connection, preview and import |
| `/admin/importers/harvest/oauth` | `HarvestOAuth` | Harvest OAuth app credentials |
| `/admin/importers/harvest/csv` | `HarvestCsv` | Import from a Harvest CSV export |
| `/admin/importers/harvest/runs` | `HarvestRuns` | Import run history |
| `/admin/importers/harvest/runs/:id` | `HarvestRun` | One import run |
| `/admin/audit` | `PermissionAudit` | Permission audit log |
| `/admin/api-tokens` | `AdminApiTokens` | Every API token created in the workspace, with revocation |
| `/settings` | `Settings` | My settings: profile, appearance, own permissions, API tokens and plugins |
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

## Accessibility

- Keyboard navigation throughout
- ARIA labels on icon-only buttons
- Color contrast meets WCAG AA — `#EFEAE0` on `#1A1813` is ~9:1. Table headers, muted text and placeholders currently fall short of 4.5:1 in places; PR-1 raises those tokens.
- Form inputs have associated `<label>` elements
- Shared `Modal` dialogs stay mounted and use native `showModal()` / `close()` for background inertness and focus restoration. Escape, backdrop clicks and Cancel dismiss them unless a mutation is pending; the panel scrolls within the viewport on short screens.
- Shared `Menu` uses a native popover in the top layer so tables cannot clip it. It opens above or below its trigger within the viewport, dismisses on outside click or anchor scrolling/resizing, and supports arrows, Home/End, initial-letter navigation, Escape and Tab. Each caller supplies a stable, unique ID for the trigger/menu association. The browser owns visibility; the shared script supplies positioning and [menu keyboard behavior](https://www.w3.org/WAI/ARIA/apg/patterns/menu-button/).
- Status indicated by text + color (never color alone) — badges include both dot and text label

## Screen States

Every new or changed screen and data-bearing component covers these states before it is done; record the check in the feature's `quickstart.md` validation, or its `visual-review.md` where one exists (as in `specs/012-clients-design/increments/mvp/`). Build each state from the shared components and classes below; add a variant to them rather than a new one-off style. `EmptyState`, `ErrorState`, `Loading` and `Skeleton` arrive in PR-4; until then, build these states from the `.empty-state` classes in [Tokens & utilities](#tokens--utilities).

- **Empty**: `EmptyState` with copy that says what to do next and, where the user can act, the action itself; not a blank table or a loose line of text.
- **Loading**: `Loading` inline, per region (see Interaction Principles), or a `Skeleton` for a listing; keep the layout stable so content doesn't jump when data arrives.
- **Error**: `.alert.alert-danger` for form errors and for a failed action on a page that otherwise loaded, inline `text-danger` for field errors, `ErrorState` (with "Try again") for a region or screen that failed to load, and `.banner-danger` for an error that needs an action inside a page. Form fields keep the values the user entered after a failed submit.
- **Success**: a toast (see [Toast](#toast-srccomponentstoastrs)); never a silent save.
- **No permission**: non-disclosing (spec 015). Name the role that can grant access ("Ask an Administrator", as in `pages/settings/own_permissions.rs`) without revealing the resource, its owner or other people's permissions; out-of-scope direct links behave as not found. Hide actions the user can never take in this context; disable, with the reason, actions that are only temporarily unavailable.
- **Long content**: 60-character names, large amounts and many rows wrap or truncate inside their column without overlapping (see Layout → Narrow content).
- **Narrow**: usable at 390px and at the 768px mobile-navigation breakpoint. How tables behave on phones is pending (see [Layout](#layout)).
- **Keyboard**: the whole flow completes without a mouse, with a visible `--ring` focus and focus returned after dialogs and menus close (see Accessibility).

## Interaction Principles

- Timer state is reactive: `TimerWidget` re-renders once a second from a task it spawns, and only while a timer runs; the timer itself is shared through `use_running_timer`
- All data mutations go through `#[server]` functions — never direct fetch calls
- Optimistic UI where appropriate; rollback on error
- Loading states shown inline, not full-page spinners
