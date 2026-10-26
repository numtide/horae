# Progress: remove the Project Detail fee balances panel

## 2026-10-26 — iteration 1

- Done: P001–P002 (spec, plan, tasks, checklist).
- Spec Kit: `check-prerequisites.sh --json --require-tasks --include-tasks` ran with
  `SPECIFY_FEATURE_DIRECTORY=specs/004-project-dashboard/fee-panel-removal` for
  analyze and again for implement (exit 0). `FEATURE_DIR` resolved to this directory.
  The script persists the selector, so `.specify/feature.json` was then restored
  to `specs/016-expense-parity` (no diff). The increment checklist is 9/9; the
  parent checklists and G01/O1/015 gates are untouched.
- Analysis (read-only): C1 MEDIUM, US1's "no fee-balance read" had no explicit
  task. In SSR, compilation proves there is no fee caller: the
  `get_project_fee_balances` mock is deleted, so any remaining page call would
  not compile. The SSR `auxiliary_requesters` assertion covers only the other
  auxiliary reads recorded by the tag, spend and budget-progress mocks. At runtime, the browser request counter
  in P003 proves no fee read is issued. C2 LOW: monthly occurrence labels
  confirmed as "<project> — YYYY-MM fee". No constitution conflicts; coverage 100%.

## 2026-10-26 — iteration 2

- Done: P003–P007. Order deviation: the P006 production edit was made before the
  P003–P005 tests, and the relocated tests were not run against a baseline build
  to observe a red result.
- Scope extension (coordinator): web-only `expect(dead_code)` on
  `ProjectFeeBalance`; see plan step 3.
- Results (exit codes in `/tmp/horae-fee-panel-logs-20261026/status.txt`):
  - `nix build .#default`: 0.
  - SSR `detail_navigation`: 73 passed. One test was removed and two panel-only
    navigation assertions were dropped.
  - Server Clippy (all targets, `-D warnings -W clippy::perf`): 0.
  - WASM check: 0.
  - Strict WASM Clippy: 101 before the model change, 0 after.
  - `nix fmt -- --ci`: 0.
  - Browser `invoice-preparation`, `project-detail-header` and
    `project-read-permissions`: 0.
- The browser server binary was built before the model attribute and the final
  `project-detail-header.cjs` diagnostic check. The attribute does not apply to
  server builds, and that suite was rerun with the final test (0).
- Next: P008, Spec Kit reanalysis and two independent reviews (coordinator).
  Then P009, publication. The full flake has not been run.

## 2026-10-26 — iteration 3

- Done: P008. Two independent read-only reviews of `b8126096` both approved
  with no blocking findings. Their low notes were documentation and test
  precision only: the stale-amount recovery wording, the SSR no-fee-read proof
  and optional extra SSR status-denial coverage. The coordinator also approved
  the production code and tests.
- Corrected docs: the plan relocation row and PSC-002 now separate the removed
  panel's stale-amount hiding from preparation's unchanged "Last review"
  behavior. The C1 note above now says SSR proof comes from compilation and
  runtime proof from the browser counter. No product code or test changed.
- Spec Kit re-analysis: `check-prerequisites.sh --json --require-tasks --include-tasks` with this `SPECIFY_FEATURE_DIRECTORY` exited 0; the selector
  was restored to `specs/016-expense-parity`. Spec, plan and tasks are
  consistent, with 100% coverage, 0 critical and no new findings.
- Open: P009 (publication), the full `nix flake check` and exact-head hosted CI.
  The parent 004, 015, G01 and O1 gates stay open.
