# Contract: legacy Time report filter panel

UI contract for `LegacyReports` in `crates/horae/src/pages/reports.rs`. Anything
not listed here keeps its current behaviour.

## Controls, labels and ids

| Order | Visible label | Control | id | Shown |
|-------|---------------|---------|----|-------|
| 1 | From | native `input[type=date]` | `report-filter-from` | always |
| 2 | To | native `input[type=date]` | `report-filter-to` | always |
| 3 | Group by | native `select` (Project, Client, Task, Person) | `report-filter-group` | always |
| 4 | Client | native `select`, "All clients" first | `report-filter-client` | always |
| 5 | Project | native `select`, "All projects" first | `report-filter-project` | always |
| 6 | Teammate | native `select`, "All teammates" first | `report-filter-teammate` | always |
| 7 | Project tag | native `select`, "All tags" first | `report-tag` (unchanged) | tags offered or one chosen |

Each label is a `label` element whose `for` equals the control's id. The control's
accessible name is exactly the visible label. The DOM and tab order is the order
above.

## Unchanged behaviour

- Defaults, option values and labels, the client → project reset, and the disabled
  "Selected tag unavailable" entry.
- Signals feeding the report resources, the request arguments, and the export
  query string. CSV/XLSX `href`s for a given state are byte-identical.
- The tag error alert with "Retry tags", the report tabs, the loading, error/retry
  and empty states, and the total row.

## Layout

| Element | Classes added | Below 640px | 640px and wider |
|---------|---------------|-------------|-----------------|
| Panel (`card mb-6` kept) | `p-0 sm:p-6 border-0 sm:border bg-base sm:bg-secondary` | bare: no padding, border or fill | same padding, border and fill as `.card` today |
| Filter row (`flex gap-4 items-end flex-wrap`) | none | unchanged | unchanged |
| Each group (or its wrapper) | `min-w-0 max-w-full` | may shrink to the row | natural width, as today |
| Each control | `min-w-0 max-w-full min-h-control` | capped to the group; at least `--height-form-control` tall | as today, except selects grow from 42px to 44px (the one recorded exception) |
| From/To inputs | `px-1 sm:px-3` | 4px horizontal padding (×2 at 200% text) | 12px, same as `.form-input` today |

Text size, the native calendar button and the DOM order do not change. Control
height changes in one approved way only: every filter control is at least
`--height-form-control` (44px at base text), so the selects grow from 42px to
44px. Inputs already measured 44px. The visible result must satisfy FR-R01-002/003: no
horizontal page scroll, and every date digit and the calendar button visible and
editable. That is judged on rendered screenshots and native editing, never on
intrinsic-width arithmetic.

## Shared code

- `FormGroup`, `Input`, the `.form-*` rules, `horae.css` and the tokens keep their
  defaults.
- The only shared changes are in the generator:
  - four entries in its `RESPONSIVE` list (`p-6`, `px-3`, `border`,
    `bg-secondary`), with the same declarations as their base utilities;
  - one Sizing utility, `min-h-control`.
- The build regenerates `horae-utils.css`, adding 13 opt-in selectors (12
  responsive, 1 base) that nothing else uses ([research.md, D2](../research.md#d2-make-the-responsive-candidate-real)).
