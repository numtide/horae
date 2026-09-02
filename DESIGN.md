# Horae Design Language

Horae follows the Invoicer design system: editorial warm-dark surfaces, serif display type, humanist sans-serif for UI, and monospace numerals — an accountancy aesthetic that suits time-tracking and invoicing. Visual prototypes with pixel-accurate examples live in `design/project/dark.dc.html`.

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
| `--color-text-strong` | `#F6F2E9` | Headings, wordmark, solid-button ink |
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

Sizes are `--font-size-*` tokens with matching `text-*` utilities: `xs` 12 · `sm` 14 (body, numerals) · `base` 16 · `lg` 18 (H2) · `xl` 20 · `2xl` 24 · `3xl` 30 (H1) · `display` 44 (hero totals, e.g. invoice "Total due"). Headings resolve to `--color-text-strong`.

## Border Radius

| Token | Value | Usage |
|---|---|---|
| `--radius-sm` | `4px` | Micro elements |
| `--radius` | `6px` | Menu items, segmented inner, small controls |
| `--radius-btn` | `8px` | Buttons, inputs |
| `--radius-lg` | `11px` | Small cards, menus / popovers, toasts |
| `--radius-panel` | `20px` | Tables, modals, large panels, auth card |
| `--radius-full` | `9999px` | Badge pills, toggles |

## Layout

- **Sidebar** (`--sidebar-width: 264px`): full-height left rail on the warm dark surface (`--color-bg-secondary`), 1px right border, sticky. Holds the brand, a start-timer action, grouped navigation, and the signed-in user; there is no separate top nav. Collapses to a 68px icon strip.
- **Content area**: scrollable, generous padding, `--color-bg` (`#100F0C`) background.

## Component Inventory

### Sidebar (`src/components/sidebar.rs`)

Full-height left rail. The brand row leads with the Horae mark and wordmark plus the golden accent dot. Nav rows default to `--color-text-secondary`; hover darkens the background to `--color-bg-tertiary`; the active row raises to `--color-bg-tertiary` with a `--color-border` hairline and swaps its glyph for a leading pine dot. Section labels: all-caps, wide letter-spacing, muted warm gray.

### Timer Widget (`src/components/timer_widget.rs`)

`HH:MM:SS` in IBM Plex Mono (`--font-family-mono`), pine-300 when stopped, green (`--color-success`) when running.

### Data Tables (`src/components/table.rs`)

Wrapped in `.table-container` (border + 20px panel radius). Headers: uppercase, 0.1em letter-spacing, transparent surface, label-tone ink (`--color-label`). Row hover sinks to `--color-row-hover`. Last row has no bottom border.

### Forms (`src/components/form.rs`)

Inputs on `--color-bg` (void — darkest), 8px radius, 44px tall, warm border `#3d382e`. Idle hover lifts the border to `--color-border-strong` and warms the fill; focus ring is `--color-primary` border + 18% pine glow. Instrument Sans for all form text.

### Status Badges (`src/components/badge.rs`)

Pill shape (9999px radius, `4px 12px`). Every variant — including neutral — carries a 7px status dot (via `::before`) over a tinted background + warm border. Neutral uses the `--color-menu` surface with a `--color-border-strong` dot.

### Buttons (`src/components/`)

Control-lg is 44px tall (`12px 20px`). Brand fills **brighten** on hover with a soft glow; neutral controls lift a shade and raise their border. Weights: primary/accent 700, solid 600, secondary/danger/ghost 500.

- **Primary**: pine-300 fill, dark green text `#0d211b`; hover → `--color-primary-hover` + `--glow-primary`
- **Solid**: deep-pine fill, `--color-text-strong` ink — form submits; hover → `--color-pine-hover` + `--glow-pine`
- **Secondary**: tertiary surface, `--color-border-input` border; hover raises the border to `--color-border-strong`
- **Accent**: brass fill `#D99A3C`, dark text — "Send invoice" and billing actions; hover → `--color-accent-hover` + `--glow-accent`
- **Danger**: transparent with dark red border `#6e3634`; hover tints and lifts the border to `--color-border-danger-hover`
- **Ghost**: transparent pine-ink link; hover → pine-tint background

## Component Conventions

- One component per file in `src/components/`
- Props use `snake_case`
- Components are `#[component]` functions returning `Element`
- State uses `use_signal` (local) or `use_resource` (async server data)
- No global mutable state — pass data down via props or context

## Tokens & utilities

Everything is driven by CSS custom properties in `:root` (`assets/css/horae.css`) —
never hardcode a colour, spacing, or radius in a rule; reference the token so the
palette stays themeable and consistent. Notable token groups:

- Colour: `--color-{bg,bg-secondary,bg-tertiary,bg-overlay,menu,row-hover}`, `--color-{text-strong,text,text-secondary,text-muted,label}`, `--color-primary*`, `--color-accent*`, and semantic `--color-{success,warning,danger,info}` with `-bg` / `-fg` / `-line` tints.
- Foreground-on-fill: `--color-on-{primary,accent,pine}` (text over a solid control).
- Chrome: `--color-border`, `--color-border-input`, `--color-border-strong`, `--color-border-danger`, `--color-border-danger-hover`, `--ring` / `--ring-soft` (focus glows).
- Hover fills: `--color-{primary,accent}-hover`, `--color-pine-hover`, and the brand glows `--glow-{primary,accent,pine}`.
- Elevation: `--shadow-{sm,md,menu,modal}`.
- Scale: `--space-1..16`, `--font-size-*` (`xs`–`3xl`, `display`), `--radius-*` (`sm`, base, `btn`, `lg`, `panel`, `full`).

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

## Pages

| Route | Component | Description |
|---|---|---|
| `/auth/login` | `Login` | Email + password form |
| `/auth/register` | `Register` | Registration (admin only after first user) |
| `/` | `Dashboard` | Stats overview + active timer |
| `/clients` | `ClientList` | Table of clients |
| `/clients/:id` | `ClientDetail` | Projects under client |
| `/projects` | `ProjectList` | All projects |
| `/projects/:id` | `ProjectDetail` | Project tasks + time entries |
| `/time` | `TimeList` | Time entry list with filters |
| `/invoices` | `InvoiceList` | Invoice table |
| `/invoices/:id` | `InvoiceDetail` | Invoice line items |
| `/admin/users` | `AdminUsers` | User management |
| `/settings` | `Settings` | App + plugin settings |

## Accessibility

- Keyboard navigation throughout
- ARIA labels on icon-only buttons
- Color contrast meets WCAG AA — `#EFEAE0` on `#1A1813` is ~9:1
- Form inputs have associated `<label>` elements
- Status indicated by text + color (never color alone) — badges include both dot and text label

## Interaction Principles

- Timer state is reactive: the running timer increments via `use_interval`
- All data mutations go through `#[server]` functions — never direct fetch calls
- Optimistic UI where appropriate; rollback on error
- Loading states shown inline, not full-page spinners
