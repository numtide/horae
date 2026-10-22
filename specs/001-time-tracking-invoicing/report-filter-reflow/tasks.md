# Tasks: Legacy Time report filter reflow and label association (R01/X01)

**Input**: [spec.md](spec.md), [plan.md](plan.md), [research.md](research.md),
[contracts/filter-panel.md](contracts/filter-panel.md), [quickstart.md](quickstart.md)

**Tests**: Requested by the acceptance plan (browser suite plus native capture).

**Gate**: Source, test and generator edits are authorised. No compilation, browser
or DB work until the build slot is granted. No commits until the coordinator
collects the work.

## Format: `[ID] [P?] [Story] Description`

## Phase 1: Setup

- [x] T001 Confirm the base is `8ae8458e` and that only the spec directory is new
- [x] T002 Record native measurements M1/M2 and the coordinator's utility candidate in specs/001-time-tracking-invoicing/report-filter-reflow/research.md

## Phase 2: Foundational

- [x] T003 Record D1 (utility-only markup) and D2 (four `RESPONSIVE` entries) with the static cross-screen audit in specs/001-time-tracking-invoicing/report-filter-reflow/research.md
- [x] T004 Read `rust-best-practices` (chapters 1, 5 and 8), `rust-testing` and the impeccable craft floor in full before editing
- [x] T005 Add `p-6`, `px-3`, `border` and `bg-secondary` to `RESPONSIVE` in crates/horae/build.rs, with the same declarations as their base utilities. Do not hand-edit horae-utils.css

## Phase 3: User Story 2 - Labels name and activate their controls (P1)

**Goal**: all seven labels paired. **Independent test**: the label checks in the new suite.

- [x] T006 [US2] Draft the label section of crates/horae/tests/browser/reports-filter-reflow.cjs: `labels.length`, exact `getByLabel`, click-to-focus, tab order. It is expected to fail on the base for six controls. The seed has no project tags, so the Project tag pairing stays proved by the existing new-project.cjs `getByRole("combobox", { name: "Project tag" })` check, and this read-only suite does not create tags
- [x] T007 [US2] In crates/horae/src/pages/reports.rs, add `id` to `FilterSelect` (label `for`, select `id`) and pass `report-filter-client`, `report-filter-project` and `report-filter-teammate`; give Group by `report-filter-group` inline
- [x] T008 [US2] In crates/horae/src/pages/reports.rs, give the From/To `FormGroup`s and `Input`s `report-filter-from` and `report-filter-to`

## Phase 4: User Story 1 - Every filter on a narrow screen with large text (P1)

**Goal**: no page overflow; full, editable dates; desktop unchanged except the approved select height (42px to 44px). **Independent test**: the matrix plus the native capture.

- [x] T009 [US1] Draft the matrix in crates/horae/tests/browser/reports-filter-reflow.cjs:
  - overflow and in-viewport/in-panel boxes;
  - font size and the 44px height;
  - computed-style equality at 640px and wider;
  - screenshots, with the date's intrinsic width logged as a diagnostic only.
- [x] T010 [US1] Draft keyboard date editing at 320px with 32px text in crates/horae/tests/browser/reports-filter-reflow.cjs: type From by segments, wait for the report request, check the export `href` `from`
- [x] T011 [US1] In crates/horae/src/pages/reports.rs:
  - add `p-0 sm:p-6 border-0 sm:border bg-base sm:bg-secondary` to the panel;
  - wrap or mark each group, including the Project tag `FormGroup`, with `min-w-0 max-w-full`, and add the same classes to each control;
  - add `px-1 sm:px-3` to the From/To `Input` `class`.

## Phase 5: User Story 3 - Nothing else changes (P1)

**Goal**: behaviour, requests and exports identical. **Independent test**: the behaviour checks plus the existing suites, unmodified.

- [x] T012 [US3] Draft behaviour checks in crates/horae/tests/browser/reports-filter-reflow.cjs:
  - client → project reset;
  - each Group by choice loads its own report;
  - a long option name does not widen the page;
  - report error alert and Retry inside the viewport, and recovery;
  - empty-range message;
  - export `href`s equal the current strings for a fixed state.
- [x] T013 [US3] Append `reports-filter-reflow` to the end of the default `suites=(…)` list in crates/horae/tests/browser/run-design-checks.sh without reordering anything
- [x] T014 [US3] Static checks only, before the build slot:
  - `node --check` on the new suite;
  - `bash -n` on the runner;
  - `git diff --stat` and `git diff --check`;
  - confirm that no existing suite changed.

## Phase 5b: Review corrections (after the first frozen build gates)

- [x] T021 Add `min-h-control` (`min-height: var(--height-form-control)`) to the Sizing family in crates/horae/build.rs, and apply it to every filter input and select in crates/horae/src/pages/reports.rs. No `.form-*` default changes
- [x] T022 [US1] [US3] In crates/horae/tests/browser/reports-filter-reflow.cjs:
  - presence checks before any interaction;
  - every response wait awaited together with its action (no orphan waiters);
  - wait for the chosen client's `list_projects` response and pick a project from it;
  - a bounded disposable fixture (long-named client, project tag link), removed in `finally`;
  - long names chosen and opened;
  - Project tag in the label, tab-order and matrix checks;
  - a control-height assertion against `--height-form-control`.
- [x] T023 Correct the research Labels section (local `form-group` for `FilterSelect` and Group by) and record the control-height exception in research, plan, contract and quickstart

## Phase 6: Gates (need the build slot or the coordinator)

- [x] T015 Build to regenerate crates/horae/assets/css/horae-utils.css. Verify that its diff is exactly the 13 new selectors (12 responsive, `.min-h-control`). The first build produced the 12 responsive selectors; it is rerun after T021
- [x] T016 Run clippy (server), the wasm `cargo check`, `nix fmt`, and the existing `pages::reports` tests
- [x] T017 Run the focused browser suites from quickstart.md, the shared style audit (record base, compare candidate), and the cross-screen suites. Inspect every 320/390 32px screenshot for complete digits and a visible calendar button.
  - The audit's compare exits 1 by design whenever anything differs.
  - A full key-by-key diff of both recordings shows exactly six property differences, all on `/reports`:
    - at 320px, `main` height 996.5 → 851.5px (bare panel);
    - at 320px, From `.form-input` width 144 → 128px and padding `12px` → `12px 4px`;
    - at 320px, To `.form-input` width 144 → 128px and padding `12px` → `12px 4px`;
    - at 768px, `main` height 624.5 → 626.5px (the approved select height).
  - The other seven screens, and `/reports` at 1440px, are identical.
- [ ] T018 Coordinator native capture: the 24 cases (16 plus the short windows), keyboard and calendar editing, and desktop panel pixel identity except the approved select height exception (42px to 44px). Native inspection also confirms the long-name select popups open visibly, which headless keyboard opening cannot prove
- [x] T019 Scoped impeccable `audit`/`adapt` on crates/horae/src/pages/reports.rs, then the design-implement sweep (`rg -n 'style="'`, hex literals).
  - The coordinator ran the detector independently twice; it reported nothing (`[]`).
  - The coordinator reviewed the native source; the static sweep of the added lines found no inline styles or hex colours.
- [ ] T020 Restore `.specify/feature.json` to `specs/016-expense-parity` (after every Spec Kit step), then two independent reviews and exact-head `nix flake check`

## Acceptance status (2026-10-22)

**Local gates on the frozen candidate package** (all pass unless noted):

- The new suite fails cleanly on the merged base: `the From control has id report-filter-from`.
- The regenerated utilities add exactly 13 selectors and remove none.
- clippy (server), the wasm check and the 8 `pages::reports` tests pass;
  `nix fmt -- --ci` changes nothing.
- The focused suites from quickstart.md pass.
- The full default order (39 suites, the new suite last) exits 0. The
  coordinator verified this independently.
- The shared style audit differs only on `/reports`, and only by the intended
  changes: narrow date padding, a shorter page at 320px, and the select height
  exception at 768px.

**Native Chrome (coordinator):**

- The 24-case matrix and keyboard editing of both dates pass.
- The native date picker passes:
  - clicking the calendar button opens it (`:open` is true);
  - ArrowRight then Enter changes From (05 → 06) and To (28 → 29);
  - both at 1440px and at 320px with 32px root text.
- Screenshots dismiss native popups, so popup evidence is taken from `:open`
  before capture.
- The long client and tag selects open natively (`:open` true) with zero page
  overflow.
- The stabilized desktop comparison passes in both themes at 1440px:
  - once screenshots had settled fonts and transitions, the only differences from
    the base were the four filter select heights (42px → 44px);
  - every measured panel and control width, padding, border, background, colour
    and font matches the base.
- Still pending: visual proof that the long-name popups show the full names.
  `:open`, the full selected text and zero overflow are verified natively, but a
  screenshot closes the popup. Direct CDP is unavailable and OS policy blocks
  Windows screen capture. A one-time manual confirmation has been requested. The
  requirement is unchanged.

**Reviews:**

- Two independent source reviews approved the frozen hashes.
- A bounded independent visual review approved 12 native and 4 headless panels.

**Still open:**

- T018 (visual proof that the long-name popups show the full names);
- T020 (exact-head remote CI and `nix flake check`).

## Dependencies

- T005 is independent of the page edits. T015 needs T005.
- In each story, tests are drafted before markup (T006 → T007/T008, T009/T010 → T011).
- Phase 6 comes after all stories. T020 is last.

## Implementation strategy

1. Labels first: small, and reviewable on their own.
1. Narrow layout with the generator entries, red first.
1. Preservation checks, then the gated builds, matrices and reviews.
