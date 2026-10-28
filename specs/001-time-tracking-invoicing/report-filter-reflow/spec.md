# Feature Specification: Legacy Time report filter reflow and label association (R01/X01)

**Feature Branch**: `fix/report-filter-reflow`

**Created**: 2026-10-22

**Status**: Draft (clarified). The coordinator approved the plan and the application
edits are implemented. On 2026-10-28 the coordinator authorized a rebase onto
`8a796361` and refreshed gates. Acceptance is still open: T018 (native long-name
popup proof) and T020 (exact-head checks and reviews).

**Parent**: `specs/001-time-tracking-invoicing`. Refines the presentation of the
FR-016 report filters on one surface. It does not amend the parent spec and does
not change `specs/015-scoped-permissions/contracts/time-reports.md` (the scoped
reader keeps its own filter row). It builds on
[report-currency-totals](../report-currency-totals/spec.md) and leaves that
increment's behaviour and tests intact.

**Input**: Bounded UI increment R01/X01: correct the legacy Time report filter
reflow on narrow and enlarged-text displays and connect visible filter labels to
their native controls, keeping every existing filter, request, state, grouping
and export.

## Context

At a 320px-wide window with text enlarged to 200%, the legacy Time report page
scrolls sideways by about 52px (native Chrome; 54px headless). The overflow comes
from the From/To date fields and their filter groups, not from the report table,
which already scrolls inside its own container. Every other measured width and
text size fits. Separately, the From, To, Group by, Client, Project and Teammate
labels are visible but not attached to their controls, so clicking a label does
nothing and assistive technology does not announce the label as the control's
name. Project tag is already attached correctly.

Two quick fixes were tried on a live page and rejected: letting the fields shrink
removes the sideways scroll but squeezes each date field to about 149px, and
tightening the padding leaves the last year digit partly hidden beside the
calendar button. Neither shows the full date. Evidence and measurements are in
[research.md](research.md#evidence).

Harvest is the behaviour reference and is unchanged here. The design kit's
Reports screen (`design/project/app/07_Reports.dc.html`) shows the empty report
builder with a bare, wrapping filter bar; it does not describe the populated
legacy layout, and this increment does not invent one.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Use every filter on a narrow screen with large text (Priority: P1)

A manager on a phone-width window, with text enlarged to 200%, opens the Time
report. Every filter fits on screen without sideways scrolling of the page. Both
dates are shown in full, with every digit and the calendar button visible, and
each can be changed by typing or with the calendar.

**Why this priority**: This is the defect. Today the page scrolls sideways and the
first approaches either hide the overflow or hide digits.

**Independent Test**: Open the report at 320px wide with 200% text in each theme,
read both dates, change each by keyboard and by the calendar, and check that the
page does not scroll sideways.

**Acceptance Scenarios**:

1. **Given** a 320px window with 200% text, **When** the Time report loads,
   **Then** the page has no horizontal scroll and every filter control is fully
   inside the window.
1. **Given** the same window, **When** the user reads the From and To fields,
   **Then** each shows its complete date (day, month and four-digit year) with no
   character clipped or covered, and the calendar button is visible beside it.
1. **Given** the same window, **When** the user focuses the From field and types a
   new date segment by segment, **Then** each segment stays visible while edited
   and the report reloads for the new period as it does today.
1. **Given** the same window, **When** the user opens the calendar from either
   date field and picks a day, **Then** the field shows the full picked date.
1. **Given** windows of 390px, 768px and 1440px, at normal and 200% text, and a
   short window (height about 568px), **When** the report loads, **Then** the
   same holds: no horizontal page scroll and complete, editable dates.

______________________________________________________________________

### User Story 2 - Labels name and activate their controls (Priority: P1)

A keyboard or screen-reader user moves through the filters and hears each visible
label as the control's name. A pointer user can click a label to reach its control.

**Why this priority**: Unlabelled form controls fail the project's accessibility
rule ("form inputs have associated label elements") and WCAG 1.3.1 / 4.1.2.

**Independent Test**: For each filter, check the control's accessible name and
that activating the label focuses (or, for a date, focuses the first segment of)
the matching control.

**Acceptance Scenarios**:

1. **Given** the Time report, **When** an assistive technology reads the filters,
   **Then** the From, To, Group by, Client, Project, Teammate and (when shown)
   Project tag controls are named exactly by their visible labels.
1. **Given** any of those labels, **When** the user clicks it, **Then** focus moves
   to its control.
1. **Given** the filters, **When** the user tabs through them, **Then** the order is
   From, To, Group by, Client, Project, Teammate, Project tag, as today.

______________________________________________________________________

### User Story 3 - Nothing else about the report changes (Priority: P1)

A manager who uses the report on a desktop sees the same filters in the same order,
with the same choices, defaults and results, and the same exports.

**Why this priority**: The fix must not trade one defect for a regression.

**Independent Test**: Run the existing report suites and compare request
parameters, export links and filter behaviour before and after.

**Acceptance Scenarios**:

1. **Given** a desktop window, **When** the report loads, **Then** the filters keep
   their order and wrapping row arrangement inside the existing filter panel.
1. **Given** a chosen client, **When** the user picks another client, **Then** the
   project filter resets to All projects, as today.
1. **Given** any combination of dates, grouping and filters, **When** the report
   loads, **Then** the requests made and the CSV/XLSX link targets are exactly the
   ones made today.
1. **Given** the Time or Detailed tab, **When** data is loading, fails, is retried
   or is empty, **Then** the existing loading text, error alert with retry and
   empty message appear as today.

### Edge Cases

- Very long client, project, teammate or tag names: no control widens the page.
  The closed control may truncate the chosen name; the open list shows full names.
- A previously chosen project tag that is no longer offered keeps its existing
  "Selected tag unavailable" entry.
- Project tags failing to load keep the existing alert and Retry tags button; the
  Project tag control is hidden as today when there are no tags and none chosen.
- Empty date fields (cleared by the user) keep today's behaviour; this increment
  does not add validation.
- The dark-theme calendar button stays visible on dark surfaces (existing rule).
- Users whose report access uses the scoped policy see the scoped filter row, which
  is unchanged by this increment.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-R01-001**: Each visible filter label on the legacy Time report (From, To,
  Group by, Client, Project, Teammate, Project tag) MUST be programmatically
  associated with its control, so the control's accessible name equals the
  label text and activating the label focuses the control.
- **FR-R01-002**: At widths of 320, 390, 768 and 1440 CSS pixels, at default text
  and with text enlarged to 200%, in both themes and on a short window, the legacy
  Time report MUST NOT scroll horizontally because of the filter area.
- **FR-R01-003**: In every case of FR-R01-002, each date field MUST show its
  complete value with no clipped or covered character and a visible calendar
  button, and every segment MUST remain visible and editable by keyboard and
  through the calendar.
- **FR-R01-004**: Filter text MUST NOT be rendered smaller than the size the page
  uses for its controls today, and the native date and select controls MUST be
  kept (no custom picker, no new dependency).
- **FR-R01-005**: Select filters MUST NOT be wider than the space available to
  them; long option names MUST NOT widen the page.
- **FR-R01-006**: The change MUST NOT alter filter values, defaults, choices,
  ordering, the client-to-project reset, the unavailable-tag entry, report
  requests, grouping, tabs, totals, loading/error/retry/empty states, permissions
  or the CSV/XLSX links.
- **FR-R01-007**: On windows 640px and wider the filters MUST keep their current
  order, wrapping row and filter-panel appearance (padding, border, fill and
  control padding).
- **FR-R01-008**: Shared component and stylesheet defaults MUST remain unchanged
  for every other screen. Any shared change needs a recorded justification and a
  check of every screen that uses it.

### Key Entities

None. The increment changes presentation and labelling only; no data is added,
stored or exported.

## Success Criteria *(mandatory)*

- **SC-R01-001**: In all 32 combinations of four widths, two text sizes, two themes
  and both report tabs, plus the short-window cases, horizontal page overflow is
  zero, and a screenshot shows both full dates and their calendar buttons.
- **SC-R01-002**: In native Chrome at 320px with 200% text, each date can be
  changed by keyboard and by the calendar, with the full new value visible.
- **SC-R01-003**: Seven of seven visible filter labels name their controls, and
  activating each focuses its control.
- **SC-R01-004**: Report requests, export links and the existing report, currency
  and permission suites are unchanged and pass without weakened checks.
- **SC-R01-005**: No other screen's appearance changes (no shared default touched,
  or every touched screen checked).

## Clarifications

### Session 2026-10-22

- Q: Is "fits" measured only by page geometry? → A: No. Zero horizontal overflow
  is necessary but not sufficient; dates must be seen in full in screenshots and
  edited in native Chrome (FR-R01-003, SC-R01-001/002).
- Q: May the dates shrink or lose padding until they fit? → A: Only if the full
  value and the calendar button stay visible; the two measured shrink attempts
  failed this and are rejected.
- Q: Does this cover the scoped-policy report? → A: No. Its filter row already
  pairs labels and is owned by `015`; it is not measured here and stays unchanged.
- Q: Should the filters move to the handoff's Dropdown and PeriodNav bar? → A: No.
  That bar belongs to the unbuilt report builder and would change behaviour.
  Native controls stay (FR-R01-004).
- Q: The kit's Components page says "never native `<select>`" and has no date
  field. Does that override FR-R01-004? → A: No. Replacing native controls is
  report-builder work with behaviour changes; this increment keeps them and records
  the deviation in [research.md](research.md#kit-references-and-deviation).
- Q: Can the select show a truncated chosen name on a narrow window? → A: Yes,
  provided the control fits and the open list shows full names (Edge Cases).
- Q: Which narrow-width arrangement shows the full date at 320px with 200% text?
  → Below 640px the filter panel drops its padding, border and fill, and the
  dates use a smaller horizontal padding, using existing utilities with `sm:`
  variants that restore today's desktop panel and date values. The one desktop
  change is the approved select height (next entry). Native measurements are in
  [research.md](research.md#layout-d1-date-layout-at-320px-with-200-text). FR-R01-003
  applies unchanged; no exception has been approved.
- Q: The base selects are 42px, below the kit's 44px control floor. Must desktop
  stay pixel-identical? → No, for this one property only: the filter inputs and
  selects opt into `--height-form-control` (44px at base text), so selects grow
  from 42px to 44px. Everything else in FR-R01-007 holds, and the change is verified
  explicitly ([research.md](research.md#control-height)).

## Assumptions

- "Text enlarged to 200%" is tested by setting the root font size to 32px, as in
  the earlier report evidence. This is not browser zoom or a physical device.
- The shared page gutter (`--app-pad`, rem-based) and control text size stay as
  they are; changing either would affect every screen.
- Out of scope: the report builder, period navigation, multi-select filters,
  Harvest's additional filter candidates, grouped exports, the scoped report, and
  other Reports parity work.
