# Quickstart: verifying R01/X01

Run builds and browser checks only in a granted slot. The browser runner binds
`127.0.0.1:8093` and `/tmp/horae-browser.*`, and creates its own database.

```sh
SQLX_OFFLINE=true cargo build -p horae --features server   # runs build.rs: regenerates horae-utils.css
git diff --stat crates/horae/assets/css/horae-utils.css     # expect only 13 new selectors: 12 sm:/md:/lg: plus .min-h-control
SQLX_OFFLINE=true cargo clippy -p horae --features server -- -D warnings
cargo check -p horae --features web --target wasm32-unknown-unknown
DATABASE_URL=… cargo test -p horae --features server pages::reports   # existing page tests, unchanged
nix fmt
HORAE_TEST_SERVER=… PLAYWRIGHT_MODULE=… \
  crates/horae/tests/browser/run-design-checks.sh \
  reports-currency-totals reports-filter-reflow reports-permissions responsive-layout mobile-navigation clients-visual
```

- **Suite order.** `reports-currency-totals` stays first in any focused run
  because it needs fresh seed data. The new suite creates and removes its own
  bounded fixture in the disposable database (one long-named client, one project
  tag linked to a seeded project), so it can run anywhere after that. In the default list it is appended
  at the end, and the existing order is unchanged.
- **Cross-screen audit (mandatory, because the generator changes).**
  - Record the shared style audit on a base build with `HORAE_STYLE_RECORD`, then
    compare the candidate with `HORAE_STYLE_BASELINE`. Only `/reports` may differ.
  - The generated-CSS diff must contain exactly the 13 new selectors and nothing
    else.
  - `nix flake check` covers the sandboxed check that the committed
    `horae-utils.css` matches the generator.

## Browser suite (`reports-filter-reflow.cjs`)

Legacy policy, admin dev login, seeded data plus the suite's own fixture. The
fixture makes the Project tag control render, so all seven controls are covered.

- **Labels.** For each of the seven ids in the [contract](contracts/filter-panel.md):
  - `control.labels.length === 1`, and the label text equals the visible label;
  - `getByLabel(<label>, { exact: true })` resolves to that control;
  - clicking the label focuses the control;
  - Tab visits the controls in contract order.
- **Matrix.** Each case is checked after its response settles and fonts are ready.
  - Cases: widths 320/390/768/1440 at a height of 900, plus 320×568 and 1440×600;
    root text base and 32px; both themes; Time and Detailed tabs.
  - Checks: `documentElement.scrollWidth ≤ documentElement.clientWidth`; every
    filter control's box inside the viewport and inside the filter panel; control
    font size equal to `--font-size-sm`; control height at least
    `--height-form-control` at every text size.
  - At 640px and wider, the panel's computed padding, border and background, and
    the dates' horizontal padding, equal the `.card` and `.form-input` values.
    Below 640px the panel has no padding, no border width and the page
    background, and the dates' horizontal padding is 0.25rem.
  - Screenshots of every case go to `HORAE_BROWSER_ARTIFACTS`.
  - **Not a pass/fail signal:** the date's intrinsic width is logged as a
    diagnostic only. Full-date visibility is judged from the screenshots and the
    native capture.
- **Editing at 320px with 32px text.** Focus From and type a full date by keyboard,
  segment by segment. The value updates, the next report request uses it, and the
  CSV/XLSX `href`s carry the new `from`.
- **Behaviour kept:**
  - picking a client resets Project to "All projects";
  - each Group by choice loads the report for that grouping;
  - the long client and tag names, once chosen, keep the full chosen name and do
    not widen the page at 1440px or at 320px with 32px text, including while the
    list is opened by keyboard. Headless opening does not prove the native popup
    rendered visibly; the native capture covers that;
  - choosing a client waits for that client's own project list and picks a project
    from it;
  - a forced report failure shows the alert and "Retry report" inside the
    viewport, and Retry recovers;
  - an empty range shows "No time tracked in this range."
- **Exports.** The `href`s equal the strings the current code builds, in four
  states: dates only; client and project; the long-name client and tag; and a
  teammate. Grouping is not part of the export query today.

## Native Chrome (coordinator capture)

The coordinator captures on a candidate preview, in their own temporary tab, with
nothing stored:

- the 24-case matrix: 320/390/768/1440 × base/32px × both themes (16 cases),
  plus the short windows;
- screenshots of the filter panel with both dates;
- typing into each segment;
- opening the calendar from each date and picking a day;
- opening each long-name select and confirming the popup shows the full names;
- desktop compared with the base for pixel identity of the panel, with one recorded exception: filter selects at 44px instead of 42px.

Native Chrome is the acceptance for SC-R01-001/002. Root-font enlargement is not
browser zoom or a physical device.

## Requirement → proof

| Requirement | Proof |
|-------------|-------|
| FR-R01-001, SC-R01-003 | Label checks in the new suite |
| FR-R01-002 | Matrix overflow and box checks |
| FR-R01-003, SC-R01-001/002 | Rendered screenshots and the native capture (no intrinsic-width threshold) |
| FR-R01-004 | Font-size and control-height checks; diff review: native controls kept |
| FR-R01-005 | Long-name check |
| FR-R01-006, SC-R01-004 | Behaviour and export checks; existing currency and permission suites unmodified |
| FR-R01-007 | ≥640px computed-style equality and the native desktop comparison |
| FR-R01-008, SC-R01-005 | Generated-CSS diff of exactly 13 selectors, the shared style audit, and the cross-screen suites |
