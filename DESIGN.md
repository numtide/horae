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
| `--color-text-muted` | `#918B7B` | Placeholders, section headers |
| `--color-label` | `#8D8777` | Uppercase micro-labels, table headers |

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
- **Wide tables at desktop width**: at 1280px with the expanded sidebar, every column of a table or grid is visible without scrolling. The Timesheet week (name, seven days, Total and the delete column) fits in that space, and so do all of Projects' columns. The name column has a minimum width, so names wrap inside it instead of breaking one word per line: `--width-name-column` for the Timesheet, Projects and any table marked `.name-table` (People, Tasks, Approvals, Reports), and the wider `--width-client-name` for `.client-table`. In Projects the type badge moves below the name when both do not fit on one line. Below that width the grid or table scrolls inside its own container.
- **Tables on mobile: *Pending design*.** Neither this file nor the prototypes (which stop at 900px) cover phones. The direction under review is a mixed rule: list tables stack each row as a card, and wide data tables (the Timesheet week, reports) keep horizontal scrolling with a visible indicator that more columns exist. The stacked card and the Timesheet at 390px still need designing, so do not build either until this note is replaced by a rule.

## Component Inventory

### Sidebar (`src/components/sidebar.rs`)

Full-height left rail. The brand row leads with the Horae mark and wordmark plus the golden accent dot. Nav rows default to `--color-text-secondary`; hover darkens the background to `--color-bg-tertiary`; the active row raises to `--color-bg-tertiary` with a `--color-border` hairline, tints its glyph `--color-primary` and closes with a trailing pine dot. Section labels: all-caps, wide letter-spacing, muted warm gray. Navigation labels follow the [glossary](#glossary) ("People", not "Team").

### Page header (`src/components/`)

One `PageHeader` component (`page_header.rs`, with `back_link.rs`) renders the top of every screen: the title (see [Type scale](#type-scale)) and the screen's actions, with an optional `BackLink` on its own row above it. Header actions use the normal button size, never `btn-sm` (header menus pass `trigger_class: "text-sm py-3 px-5"`). They sit at the right while they fit beside the title and wrap onto a left-aligned row of their own when they do not, the same way on every screen. One shared header keeps the title, the way back and the actions in the same place everywhere.

`PageHeader` has three optional slots besides the actions. Each is for the one thing named here, so headers do not drift apart again:

- **`context`**: one line above the title, only when it carries data about the record: the client of a project (a link when the viewer may open it), the date a client was added. Never a fixed label such as "Projects"; the back link already names the section.
- **`meta`**: inline after the title, for status badges and short qualifiers: a currency or project-type pill, "Archived", the connection status of an integration, the source of an import run, the Timesheet's week total.
- **`leading`**: before the title, only for an integration's own logo (Harvest), as a 36px mark (`--size-page-mark`) with the button radius. The logo is a local asset shown unaltered (see `AGENTS.md`, *Harvest parity*); no other icon or brand goes here.

Anything longer, such as a status sentence or a description, goes below the header, not in it.

- **Back link**: the kit's *Back link*. A 16px arrow and a 14px label on their own row above the title, muted ink that brightens to pine on hover. It points to the parent in the hierarchy ("Back to Projects" from a project, "Back to Importers" from Harvest), not to the screen the user came from, and only screens that sit under a section have one. Sections reached from the sidebar (Timesheet, Projects, People, Settings, API tokens, Importers, the audit log and the rest) have none, because the sidebar is always there; the `AdminShell`'s "Back to Timesheet" link is removed.
  - **Exception: editors return where they were opened.** An editor of an existing record goes back to the screen that opened it, because leaving it is closing a task, not moving up the hierarchy. Edit project returns to the project ("Back to project") when opened from its detail and to Projects ("Back to Projects") from the list, as its `return_to` says. The back control of an editor with a draft is a button with the back-link look, because leaving first saves the draft or asks to discard it.
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
- **Danger solid**: red fill with dark ink, the kit's `hb-danger-solid` from `ConfirmDialog.dc.html`. Only for the confirm button of an irreversible action. The class is `.btn-danger-solid` (`ButtonVariant::DangerSolid`); PR-3 moves the confirmations to it. In the light theme the fill is `--color-danger-fg` with white ink, because `--color-danger` reaches only 4.47:1 against white.
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

- One component per file in `src/components/`
- Props use `snake_case`
- Components are `#[component]` functions returning `Element`
- State uses `use_signal` (local) or `use_resource` (async server data)
- No global mutable state — pass data down via props or context

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
- Calendar events: `--color-cal-{event,today}-{bg,border,ink,fg,meta}` for the Timesheet calendar's blocks (pine on most days, brass on today), defined for both themes. Ink is the project name, fg the time and task, meta the client.
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
- **Loading**: `Loading` inline, per region (see Interaction Principles), or a `Skeleton` for a listing; keep the layout stable so content doesn't jump when data arrives.
- **Error**: `.alert.alert-danger` for form errors and for a failed action on a page that otherwise loaded, inline `text-danger` for field errors, `ErrorState` (with "Try again") for a region or screen that failed to load, and `.banner-danger` for an error that needs an action inside a page. Form fields keep the values the user entered after a failed submit.
- **Success**: a toast (see [Toast](#toast-srccomponentstoastrs)); never a silent save.
- **No permission**: non-disclosing (spec 015). Name the role that can grant access ("Ask an Administrator", as in `own_permissions.rs`) without revealing the resource, its owner or other people's permissions; out-of-scope direct links behave as not found. Hide actions the user can never take in this context; disable, with the reason, actions that are only temporarily unavailable.
- **Long content**: 60-character names, large amounts and many rows wrap or truncate inside their column without overlapping (see Layout → Narrow content).
- **Narrow**: usable at 390px and at the 768px mobile-navigation breakpoint. How tables behave on phones is pending (see [Layout](#layout)).
- **Keyboard**: the whole flow completes without a mouse, with a visible `--ring` focus and focus returned after dialogs and menus close (see Accessibility).

## Interaction Principles

- Timer state is reactive: the running timer increments via `use_interval`
- All data mutations go through `#[server]` functions — never direct fetch calls
- Optimistic UI where appropriate; rollback on error
- Loading states shown inline, not full-page spinners
