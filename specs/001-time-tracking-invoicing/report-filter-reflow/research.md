# Research: Legacy Time report filter reflow (R01/X01)

## Evidence

All measurements are the coordinator's, taken on 2026-10-22 in native Windows
Chrome on disposable previews, with the root font set to 32px to simulate 200%
text. No database, account or Harvest writes were made.

- **Before state** (preview of `0243a386`, matching `8ae8458e` for this page):
  the page overflows by 52px only at 320px with 32px text (54px headless). Every
  other case (1440/390/320 × base/32px, both themes) shows −10px, which is the
  classic scrollbar, not overflow. The overflowing elements are the filter
  `.form-group`, `.form-label` and `.form-input` nodes outside `.table-container`.
  The table already scrolls within its container.
- **Discarded probe 1** (DOM only, merged preview): adding `min-w-0`/`max-w-full`
  to the groups and controls removes the page overflow but squeezes each date to
  148.67px. Page geometry alone cannot tell whether date segments are clipped.
- **Discarded probe 2**: card and input `p-2` widens the dates to 212.67px with no
  overflow, but a screenshot shows the last year digit partly covered by the
  calendar button.
- **Labels**: the From/To `FormGroup`s get no `id`, so their `label`s have no
  `for`. Group by and the local `FilterSelect` render bare `label`s. Only Project
  tag (`FormGroup { id: "report-tag" }` + `select { id: "report-tag" }`) is paired.

## Geometry at 320px with 32px root text

Every spacing token is rem-based, so it doubles with the text.

| Quantity | Value | Source |
|----------|-------|--------|
| Window / client width | 320 / 310px | classic 10px scrollbar |
| Page gutter `--app-pad` (≤768px) | 2 × 32px | `--space-4` |
| Content column (= card width) | 246px | measured |
| Card border + padding | 2 × (1 + 48)px | `.card`, `--space-6` |
| Card content box | 148.67px | measured |
| Date input, laid out at intrinsic size | 291.33px | measured |
| Date input border + padding | 2 × (1 + 24)px | `.form-input`, `--space-3` |
| **Date content (text + native calendar button + native gaps)** | **≈241.33px** | 291.33 − 50 |

The ≈241.33px figure is not a fixed property of the date text. It is the native
control's laid-out content as currently styled: the date segments, the gaps
Chrome puts between and around them, and the calendar button with its spacing.
None of these parts has been measured on its own, so this table cannot show how
much of the 241.33px is readable text and how much is native spacing.

If that content width were held constant, a date would fit only when
`246 − 2·(card border + card padding) − 2·(input border + input padding) ≥ 241.33`.

- The card as it is leaves 148.67px.
- Probe 2 (card and input `p-2`, 16px) leaves 178.67px of content, about 62.67px
  short. This matches the covered digit in the screenshot.
- With no card chrome, input padding would have to be about 1.33px per side.

**What this does and does not show**: card and input padding changes alone did not
show the full date in either probe. Whether a layout can do so depends on the
native rendering, not on this arithmetic; see the D1 measurements. FR-R01-003
stays the acceptance bar in every case.

### 390px with 32px text

At 390px with 32px text the card content box is 390 − 10 − 64 − 98 = 218px. The
291.33px date fits within the page (no overflow was recorded) but should extend
about 73px past the card's content box, which is about 24px beyond the card's
right border. Page-geometry checks cannot detect this. Capture M2 confirmed in
native screenshots that the input extends outside the card.

## D1: date layout at 320px with 200% text

**Decided: utility-only markup.** No `horae.css` rules and no shared component
change. The only generator changes are the four responsive variants in D2 and the
`min-h-control` base utility (see Control height).

### Native measurements (coordinator, M1/M2)

These come from native Windows Chrome on the merged preview, at 320×900 with a
32px root, using DOM-only probes. Nothing was stored.

| Probe on the From input | Width |
|-------------------------|-------|
| `padding: 0`, intrinsic | 243.33px |
| calendar button hidden | 211.33px |
| button `margin-inline-start: 0` | 243.33px (no gap to remove) |

M2: screenshots of the base at 390px with 32px text confirm the date input
extends outside the card.

The full date can display at less than the input's max-content width. These
intrinsic widths are therefore **diagnostic only**. They are never used as a
pass/fail threshold, and they do not prove clipping or visibility either way.
Acceptance is the rendered date in screenshots and native editing (FR-R01-003).

### Measured candidate

The coordinator's DOM candidate keeps `card mb-6` and adds:

- on the panel: `p-0 sm:p-6 border-0 sm:border bg-base sm:bg-secondary`;
- on the groups and controls: `min-w-0 max-w-full`;
- on the dates: `px-1 sm:px-3`.

Results, inspected by the coordinator in native screenshots at 320 and 390 with
32px text, in both themes:

- every date digit and the calendar button are visible;
- text size is not reduced.

That was the DOM-probe stage. The implemented candidate's gate and native results
are in the acceptance status of [tasks.md](tasks.md#acceptance-status-2026-10-22).

### D2: make the responsive candidate real

The coordinator's matrix confirmed that the base utilities fit at mobile widths.
It also confirmed that four of the candidate's responsive classes are not
generated today:

- `sm:p-6`
- `sm:border`
- `sm:bg-secondary`
- `sm:px-3`

The generator's `RESPONSIVE` list (`crates/horae/build.rs`) covers only `hidden`,
`block`, `flex`, `flex-row`, `flex-col`, `px-6`, `grid-cols-2/3/4` and
`text-right`. Browsers ignore unknown classes, so the probe left the panel bare
and the dates at `px-1` on desktop too. **There is no desktop pass yet.**

**Decision (coordinator-authorised, scoped):** add four existing base utilities
to `RESPONSIVE`, unchanged:

| Utility | Declaration (already in the base matrix) | What its `sm:` form restores |
|---------|-------------------------------------------|------------------------------|
| `p-6` | `padding: var(--space-6)` | `.card` padding |
| `px-3` | `padding-left/right: var(--space-3)` | `.form-input` horizontal padding |
| `border` | `border: 1px solid var(--color-border)` | `.card` border |
| `bg-secondary` | `background: var(--color-bg-secondary)` | `.card` fill |

`horae-utils.css` is regenerated by the build, not by hand. The change adds no
tokens, defaults or `horae.css` overrides, and leaves every other utility
definition and the generator logic unchanged. The project adds responsive
variants exactly this way (DESIGN.md: the generator "owns only the mechanical
utility + responsive matrix").

**Cross-screen audit (static):**

- No source, test or doc outside these spec files uses any of the four names with
  an `sm:`, `md:` or `lg:` prefix. A search over `*.rs`, `*.cjs`, `*.js` and
  `*.md` found none.
- The output gains 12 selectors (four per breakpoint) inside the existing
  `@media` blocks. No existing selector changes.
- So only markup that opts in can change. A runtime check is still required: the
  shared style audit (record base, compare candidate) and the cross-screen suites
  in quickstart.md.

**Why not an existing equivalent:**

- D2-a (panel `px-0 sm:px-6` with dates `px-1`) keeps the panel exact, because
  `sm:px-6` already exists. It has no way to restore the dates' 12px desktop
  padding, and it keeps the panel border at narrow widths, which has not been
  measured.
- D2-c (bare panel at every width) changes desktop and breaks FR-R01-007.
- Duplicating the panel per breakpoint is excluded: it would duplicate ids and
  controls.

**Cascade:** `horae-utils.css` loads after `horae.css`, and the responsive
`@media` blocks come after the base utilities in that file. At equal specificity:

- below 640px, `p-0`, `border-0`, `bg-base` and `px-1` override `.card` and
  `.form-input`;
- at 640px and wider, the `sm:` rules re-apply the original values, identical to
  the `.card` and `.form-input` declarations. Desktop is expected to be
  pixel-identical except for the one approved exception, filter selects at 44px
  instead of 42px (see Control height). The matrix and the native capture confirm
  both (FR-R01-007).

## Labels

- **Decision**: every control gets a stable, legacy-specific id:
  - `report-filter-from`, `-to`, `-group`, `-client`, `-project` and `-teammate`;
  - `report-tag` is kept.
- **Two pairing paths, both using `for`/`id`:**
  - From, To and Project tag keep the shared `FormGroup`, which receives the id and
    renders the label's `for`. `Input` and the tag `select` take the same id.
  - `FilterSelect` (Client, Project, Teammate) and the inline Group by keep their
    existing local `div.form-group` markup. Their `label` gets `for`, and their
    `select` gets `id`. `FilterSelect` gains an `id: String` prop.
- **Why the local markup stays:** it keeps the diff small and keeps the
  `.form-group` element that the currency suite locates by its label. The shared
  `FormGroup` adds nothing for these selects.
- **Rationale**: `FormGroup` and `Input` already accept `id`, so the shared
  defaults are unchanged. The pairing matches the scoped report and Project tag's
  existing contract.
- **Alternatives rejected:**
  - wrapping each control in its `label`, which changes the `.form-label` layout;
  - `aria-label`, which gives no click target and duplicates the visible text;
  - the scoped report's `report-from`, `report-to` and `report-group-*` ids, because
    its suites address those ids and the two views mean different things.

## Control height

The coordinator measured the base in native Chrome at 1440px with default text:

- dates are 44px tall;
- selects are 42px (14px font, `line-height: normal`, 12px padding, 0.67px border,
  no minimum height).

The suite's control-height assertion is therefore not met by the base, and it is
not weakened. Instead, this page's inputs and selects opt into one base utility,
`min-h-control` (`min-height: var(--height-form-control)`, the existing 2.75rem
token). It is added to the generator's Sizing family, and no `.form-*` default
changes.

**Recorded exception to desktop pixel identity:** the filter selects grow from
42px to 44px. This is the intended alignment with the kit's 44px control floor.
Width, padding, colour and order stay as they are. This is the only accepted
desktop difference, and the matrix and the native capture verify it explicitly.
It is not a blanket waiver.

## Selects and long names

- **Decision**: `min-w-0 max-w-full` on each filter group (existing utilities).
  The select keeps `width: 100%` from `.form-select`.
- **Rationale**: a flex item's automatic minimum width is the widest option, so a
  long name widens the page today. With the cap, the closed control may truncate
  the chosen name and the open list still shows it in full (spec Edge Cases).

## Kit references and deviation

Sources, read in full:

- `design/README.md`;
- `design/project/app/07_Reports.dc.html`, `Dropdown.dc.html`, `PeriodNav.dc.html`
  and `DatePicker.dc.html`;
- `design/project/Design System.dc.html` and `foundations/Components.dc.html`;
- the three `support.js` runtimes.

The runtimes contain prototype machinery only (placeholders, print baseline,
atomic helpers). They have no product layout or form rules.

What applies to this panel:

- **Reports (07)**: the empty builder. The filter bar sits on the page with no
  card: `flex-wrap`, 8px gap, Dropdown and PeriodNav controls 44px tall. Below
  900px the page padding becomes 20px. There is no populated legacy layout and no
  guidance below that width.
- **Fields** (Components "Inputs & fields"; Design System "Inputs & controls"):
  - visible labels sit above the field, as 12px uppercase micro-labels;
  - fields have 12px padding and an 8px radius;
  - "Interactive height never falls off these four — 44px is the floor for a
    touch target". Any narrow treatment therefore changes horizontal padding only
    and keeps controls at least 44px tall;
  - the scale note says "padding · margin · gap only. No pt/pb/pl/pr", so
    padding stays symmetric (for example `px-2`), not one-sided.
- **Selects**: "Custom dropdowns (never native `<select>`)". The Dropdown `fill`
  mode truncates the chosen label with an ellipsis, which supports the spec's
  truncated-closed-select edge case.
- **Dates**: the kit has no date input. PeriodNav shows the period as mono text
  (for example `06 – 12 Jul 2026`), and the DatePicker "Opens from the period
  stepper's label, never from an icon of its own". The Design System assigns IBM
  Plex Mono to "Numerals only — hours, rates, amounts, dates".
- **Skeleton**: "Chrome (header, filters, actions) stays live — only the data
  region is replaced". This matches keeping the filters usable during loading.
- **Kit inconsistencies** (recorded, not resolved here):
  - the type scale lists "base 16px · inputs" while every sample field uses 14px;
    the app uses `--font-size-sm`, which this increment keeps;
  - the kit page padding is a fixed 44px (20px in Reports below 900px), while the
    app's gutter is rem-based. That is a shared token and out of scope.

Deviation: this increment keeps the native date and select controls and their
calendar button (FR-R01-004, as the brief requires), and the current Instrument
Sans control font. From the kit it takes only the bare-bar treatment at narrow
widths (D1). Moving to Dropdown, PeriodNav or a mono date display is
report-builder work.

## Scoped report (not in scope)

`pages/reports/scoped.rs` renders its own filter row with no card, already-paired
ids and the same native date. It may behave similarly at 320px with 200% text,
because it uses the same native control. That has not been measured. It is owned
by `015` and stays unchanged. The D1 utilities could later be applied there as a
separate, measured change.

## Tooling notes

- `.specify/scripts/bash/update-agent-context.sh` is not installed, so the plan
  step's agent-context update was not run. `AGENTS.md` needs no change for this
  increment.
- No `.specify/extensions.yml`, so no hooks ran.
- `impeccable context` (scoped to `pages/reports.rs`) reported no PRODUCT.md and
  allowed narrow refinement on the incumbent system. No init was run and nothing
  was written.
