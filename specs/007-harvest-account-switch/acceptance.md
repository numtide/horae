# Acceptance: Safe Harvest Account Switching

## Baseline and workflow

- Branch `feat/harvest-account-switch`, worktree `/tmp/horae-harvest-account-switch`, based on `513afaf` (PR #199). No automatic merge.
- Spec Kit specify, clarify, plan, tasks and read-only analyze completed using local skills/templates/scripts. No critical clarification needed; assumptions are explicit in spec.md. No extension hooks exist.
- Requirements checklist: 16 total, 16 complete, zero incomplete (PASS).
- Analyze: all 11 functional requirements and five success criteria have task coverage. No critical/high findings. Two incorrect file paths in plan/tasks were corrected before implementation: core lives in crates/core; UI harness is tests/import_jobs_ui.rs.
- Implement skill loaded; existing Git ignore rules cover build, local state and secrets. No new dependencies or unrelated ignore changes required.
- Rust best-practices, testing, async-patterns and ponytail skills guide implementation: pure eligibility, existing locks/modal/transport and tests before code.
- Tests use a separate disposable database; the live preview and its Harvest credentials are outside the mutation scope.

## Validation evidence

Implementation is complete; final Nix validation and PR delivery are pending. No successful real-account import or live deployment is claimed by this feature.

- Initial RED: the pure eligibility test failed to compile before the new policy/metadata existed. Core suite subsequently passes: 88 tests.
- Initial database checkpoint: six generation/change/concurrency tests passed. Further acceptance cases cover all 12 business-table snapshots, rollback after credential deletion, running-work exclusion, old-generation execution and retry races.
- Rendered UI checkpoint: 18 tests pass, including connected/disconnected confirmation, cancel, blockers, busy state, stale error and recoverable authorization failure. Native dialog cancel events are exercised while idle and busy; real-browser focus/Tab traversal is not claimed by this harness.
- Full-suite iteration exposed old-schema report fixtures calling the current enqueue path; those fixtures now insert the original schema directly. The same run caught the OAuth session backend's MessagePack/arbitrary-precision integer mismatch. JSON-text encoding fixes the persisted attempt format; a real-session HTTP assertion protects this boundary.
- No automatic submission retries exist in the CLI. The implementation captures generation once, preserves uncertain-submission behavior and rejects previously accepted cross-generation identities on deliberate resubmission.

## Completed local gates

- Core: 88 passed.
- In-crate server suite: 523 passed, zero failed, 11 existing long benchmarks ignored.
- Integration executables: admin shell 5, CLI contract 5, CLI restart 1, detail navigation 4, importer UI 18, domain/database integration 36, timer widget 1 — all passed (70 total).
- Total local acceptance: 681 passed. The final mixed callback/change race refinement also passes the focused 10-test account-switch suite.
- `cargo clippy -p horae --features server --all-targets -- -D warnings`: passed on the final source.
- `cargo build -p horae --features web --target wasm32-unknown-unknown`: passed.
- Existing live preview `/health`: `{"status":"ok"}`; root worktree's tracked files unchanged.
- SQLx metadata is regenerated against `horae_account_switch_dev_20260915`. Incremental preparation omitted unchanged integration-test metadata; a package-scoped clean followed by preparation rebuilds all query callsites. Only generated temporary build artifacts are cleared, never source or live database records.
