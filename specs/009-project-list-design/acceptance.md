# Acceptance evidence

Validated on 2026-09-16 on x86_64-linux, using the pinned Nix dev shell.

## Local checks

- `cargo clippy -p horae --features server --locked -- -D warnings` passed with
  `SQLX_OFFLINE=true`.
- `cargo check -p horae --features web --target wasm32-unknown-unknown --locked`
  passed.
- `dx build --platform web --fullstack true --force-sequential true --locked`
  produced matching server and WASM assets.
- `nix fmt -- --ci` and `git diff --check` passed.
- `projects-design.cjs`: eight checks passed, covering supported columns and
  native progress, resetting all three filters, 320/768/1440px short viewports,
  keyboard row-menu/edit access, CSV/XLSX export URLs, loading and failed spend
  (including retry), and empty-state action visibility for admin/manager/member.
  Role/read fixtures test presentation, not a replacement for server permission
  tests. Mutating API requests are blocked by this harness.
- `action-errors.cjs`: all four existing checks passed, including project status
  failures remaining visible while the form is opened and cancelled.

The initial design test failed against the previous bundle because the expected
New project action did not exist, before production changes were built.

## Isolation

The final browser checks used a newly created, demo-seeded database
`horae_projects_design_dev_20260916` and a loopback server on port 8092. The
temporary process had no Harvest configuration. The existing live service on
port 8080 and its imported business data were not changed. The error-regression
harness creates and toggles a disposable client only in the demo database.

## Review and boundaries

Reviewed the diff against the handoff, existing permission helpers, export
contract and utility framework. No new dependency, backend query, migration,
inline style or color literal was added. Existing project billing calculations
are unchanged. Unsupported scheduling/Delta placeholders were removed rather
than populated with invented data. Prototype-only bulk controls, pin/delete,
manager filtering and the inline-form-to-modal conversion remain deferred, as
listed in the specification.

This is a scoped list-page alignment, not pixel-perfect acceptance of every
prototype state. Prototypes were inspected as source, not rendered or captured.
Full Nix CI must pass before this delivery is merged.

## CI fixture correction

The first CI run exposed a compile failure in `detail_navigation`: its reduced
test crate includes the production Projects page but did not expose the icon
module or importer route added to that page. The application build and browser
checks alone did not compile this integration-test target.

The fixture now reuses the production icons and declares the importer route with
a minimal destination component. A new regression renders the empty Projects
list, checks both importer links and navigates to that destination and back.
The production UI and backend are unchanged by this correction.

- Reproduced the original missing-module/missing-route errors locally.
- `cargo test -p horae --features server --test detail_navigation --locked`:
  all five tests passed.
- `cargo clippy -p horae --features server --all-targets --locked -- -D warnings`:
  passed, including integration-test targets omitted from the earlier check.
- `cargo sqlx prepare --check --workspace -- --features server --all-targets`:
  passed against the isolated design-test database with `SQLX_OFFLINE=false`;
  the committed query cache is unchanged.

## Visual fidelity and shared-system regression check — 2026-09-17

The initial layout did not meet the supplied side-by-side reference: heading
size, row density, billing-type placement and monetary-column spacing differed.
The follow-up uses additive framework utilities and existing chips, removes
duplicated page CSS, and shares content-sized numeric tracks across all rows.
Existing utility definitions and shared component defaults are unchanged.

- Reproduced the heading mismatch with a failing browser assertion (24px instead
  of the handoff's 34px). Added coverage for 14px rows, inline billing types and
  large/negative currency amounts staying inside their columns at three widths.
- `trigger_utilities`: four passing tests cover Menu/Combobox compact defaults
  and opt-in trigger classes. `detail_navigation`: all five tests passed.
- Server Clippy with `--all-targets --locked -- -D warnings`, WASM check and
  matching fullstack build passed in the Nix shell.
- `projects-design.cjs`: ten checks passed, including compact default controls
  in the component gallery and no business mutations.
- `menu-popovers.cjs`: five checks passed, including gallery keyboard controls,
  filters, calendar menus, short screens and first/last project-row actions.
- `mobile-navigation.cjs`: three checks passed for focus, mobile navigation,
  account navigation and preservation of desktop collapse/resize preferences.
- `responsive-layout.cjs`: 55 header checks plus four interaction checks passed
  with `HORAE_TEST_WEEK=2026-09-14`. The original fixed 2027 date had no entries
  in this demo. The harness now accepts the seeded Monday explicitly, without
  rewriting the database; navigation assertions preserve query parameters.
- Compared computed typography, colors, padding, spacing, radii and dimensions
  before/after on Clients, Invoices, Reports, Users, Settings, Importers, the
  component gallery and Timesheet week at 320/768/1440px. All 24 comparisons
  matched for the sampled shared controls, headings, main panels and sidebar.
  This is a measured shared-style check, not a whole-application pixel snapshot.
- Inspected the running Projects screenshot: the two demo projects retain
  inline billing types and separated USD amounts, with the larger heading and
  accented client groups. No prototype rendering was required.

Browser checks use Chromium and the isolated `horae_projects_design_dev_20260916`
database on port 8092. The live imported workspace on 8080 remains unchanged and
healthy. Other browser engines were not exercised. Full Nix CI and human visual
review remain merge gates; this follow-up does not authorize merging the PR.

## Review corrections and reproducible browser checks

The follow-up review found a wrong table-header color, an oversized empty-state
radius, a circular undersized glyph, inherited link typography, and remaining
page CSS that duplicated utilities. Projects now opts into the handoff's 16px
card, 52px icon tile with a 22px briefcase glyph, 64px/24px padding, 380px copy
measure, 14px import link and label-colored table header. Heading/paragraph
margins are reset locally so the 12px stack gap controls spacing. Existing
empty states, navigation glyphs and compact menu/combobox defaults are unchanged.

`projects-design.cjs` asserts those dimensions and tokens, role-dependent
actions, the light-theme copy color, and the shared semantic defaults. The
header-color assertion failed against the previous preview before the fix.

### Reproducing shared-style comparisons

The previously temporary comparison is now `shared-style-audit.cjs`. Set
`HORAE_TEST_URL` to an isolated demo instance (not port 8080), `HORAE_TEST_WEEK`
to its seeded Monday, and `PLAYWRIGHT_MODULE`/`CHROMIUM_PATH` if needed. Before
rebuilding that preview, run:

```sh
node crates/horae/tests/browser/shared-style-audit.cjs record /tmp/horae-shared-before.json
```

After updating its bundle, without changing its data or browser configuration:

```sh
node crates/horae/tests/browser/shared-style-audit.cjs compare /tmp/horae-shared-before.json
```

The capture refuses to overwrite a baseline. Both runs block remote fonts so
availability of Google Fonts does not affect measurements. This compares sampled
computed styles on eight other routes at three widths, not pixel screenshots
or every element/state of the application.

### Continuous browser checks

On Linux, `nix build .#checks.x86_64-linux.browser -L` builds the application and
runs Projects, responsive layout, menus and mobile-navigation checks with pinned
Playwright/Chromium. The existing `nix flake check` CI job discovers this check.
`run-design-checks.sh` creates its own PostgreSQL cluster on a temporary Unix
socket, migrates/seeds it, derives the seeded week and starts the server on 8093.
It refuses an occupied HTTP test port and stops its server/database on exit.
The baseline comparison remains an explicit before/after review tool; CI uses
the persistent design/default assertions and cross-page interaction checks.

Validation of the final corrections on 2026-09-17:

- Nine Rust tests, server Clippy with all targets, WASM check and matching
  fullstack build passed again.
- All 77 browser checks passed inside the Nix build sandbox using the locally
  built debug bundle substituted for the release package. This validates the
  new runner, isolation, pinned browser and font setup; the release package and
  full CI remain a separate gate.
- The first sandbox attempt exposed Chromium crashing without Fontconfig/fonts;
  the check now explicitly supplies a pinned font configuration and DejaVu.
- The final preview passed the expanded Projects suite, including dark/light
  copy colors, exact empty-state dimensions and unchanged shared defaults.
- All 24 shared-style comparisons matched after the final rebuild. Inspected
  the application empty-state screenshot; prototype files were not rendered.
- Formatting and diff checks passed. Preview 8092 is updated; imported workspace
  8080 remains healthy and untouched. Bulk selection/actions remain a separate PR.
