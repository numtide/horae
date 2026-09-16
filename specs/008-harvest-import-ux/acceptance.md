# Acceptance Status: Harvest Import UX

## Current delivery

Implementation on `feat/harvest-import-ui`, based on planning commit `5c9955b`, initially validated on 2026-09-15. On 2026-09-16, the operator separately authorized a real Preview and a coordinated local upgrade. The branch includes master through merge commit `fcadde6`; application code, SQLx metadata and build configuration are unchanged from `3eac906`. Full Nix revalidation passed on integrated commit `91fb512`. All feature acceptance tasks are complete; PR #202 can leave draft, with required remote checks still governing merge. Persistent Git signing configuration remains unchanged. No automatic PR merge or committing import is included.

## Implementation baseline

Implementation starts from planning commit `5c9955b` on `feat/harvest-import-ui`, in an isolated worktree. No real-account operations are authorized by fixture validation.

- Read the complete Importers handoff, Design System, Components and supporting runtime source, plus `DESIGN.md` and the applicable implementation/Rust/testing skills.
- Reuse the existing admin shell, native `Modal`, `.card`, `.badge`, `.banner`, `.integration-*`, `.counter-*` and utility classes. Map panel spacing to `p-4`/`gap-3`/`gap-4`, secondary copy to text utilities, and borders/radii to existing tokens.
- Baseline source problems: picker assumes availability while loading; connection errors lack a recheck action; account change is separate from management; disconnect/connect lack pending guards; expiry copy promises fixed-time recovery; results claim every record was written; history exposes wire identifiers.
- Keep actual OAuth redirects, account identifiers, server blockers, retained reports and NDJSON errors. Do not copy the prototype secret editor, popup, invented last-sync values, 25 MB limit or arbitrary-tracker promise. No global shell redesign.
- Existing tests deliberately preserve the submitted CSV file when another file is selected during a preview. Preserve that captured-source behavior and label the report source explicitly; do not silently confirm the replacement file.
- Spec Kit prerequisites and requirements checklist pass (16/16); no extension hooks are installed. Existing ignore rules cover build outputs, local state and worktrees; no unrelated ignore/config changes are needed.

## Spec Kit execution

- `specify`: local template resolved, feature 008 created, quality checklist 16/16.
- `clarify`: prerequisite paths checked; zero questions needed. Scope, actors, entities/lifecycle, interaction/error/accessibility, quality bounds, integration failures, constraints, terminology and completion criteria are covered. Read-model details are resolved in planning.
- `plan`: setup script executed; research, model, UI contract and validation guide created. Pre/post constitution gates pass. Bounded read-only read-model investigation completed.
- `tasks`: setup script executed; 26 dependency-ordered implementation tasks created. Execution progress is tracked in `tasks.md`, not inferred from planning artifacts.
- `implement`: prerequisites and requirements checklist checked before changes; presentation/connection/result/history/retry/browser cases observed failing before the matching implementation. Changes stay in `feat/harvest-import-ui` and its own worktree.
- No extension hooks or preset overrides are installed. The agent-context update script is absent; technical context is recorded in the plan.
- Workflow uses checked-in `.claude/skills/speckit-*` instructions and `.specify/scripts/bash/`; no standalone `specify` executable is installed.

## Implementation evidence

Environment: Nix development shell; isolated PostgreSQL `horae_account_switch_dev_20260915` for compile-time validation and SQLx-managed throwaway test databases. Browser fixtures use separately created/seeded `horae_import_ux_dev_20260915`, port 8092, matching Dioxus server/WASM, and no Harvest environment credentials. Browser importer/provider operations are intercepted. The user's port 8080 app and database were not migrated or restarted.

- Red/green: five pure presentation cases; four connection cases; result copy/primary action/source navigation; missing/future retry metadata and status/list projection; history/progress/retry prerequisites; browser keyboard/error-disclosure/long-identifier regressions.
- Final review added a reproduced cross-tab account-change regression: refreshing connection metadata must not retarget an earlier API preview. Accepted API origins now retain their original generation; refreshed mismatches remove confirmation, and requests remain server-fenced.
- Existing CSV replacement behavior is retained: an in-flight preview of `original.csv` never commits a newly selected replacement file. History/reload/navigation cannot confirm a historical preview. Polling still follows the same ID every second and keeps its existing version guard; history still uses 20-row keyset pages.
- Connection tests cover checking, unavailable, unconfigured, unbound, connected, expired and disconnected/bound states, both change blockers, cancelled/busy/stale confirmations and release-then-authorization failure. The ten existing account-preservation/race tests pass; authorization tests retain non-admin, inactive and foreign-organization boundaries.
- Retry projection is advisory and computed in each existing bounded status/list query. Two prior SQL cache entries were replaced and one test-query entry added; there are no migrations, new dependencies or per-row client requests. Existing `can_retry()` and write-side policy are unchanged. The preservation test explicitly allows only this derived availability to change while asserting every retained job/report field is identical.
- Browser harness passes six scenarios: native modal focus/dismissal/pending safeguards, keyboard CSV choice and one primary confirmation, old-account partial report plus error disclosure/download, same-ID monitoring recovery, layout at 360/768/1440 × 640, and actual 200% browser zoom. An isolated test-only extension uses `chrome.tabs.setZoom`; the browser reports 2.0 zoom, layout width 1440→720 and DPR 1→2. This is not device-scale-factor emulation.
- Main-panel alignment reuses existing semantic cards, banners, counters, badges and utilities. Scoped structural CSS handles unbroken identifiers, shrinking tiles, selected history and native file-input focus. No inline palette or global shell changes. OAuth redirects, operational report retention and NDJSON downloads intentionally differ from unsupported prototype promises.

## Gate status

| Gate | Status |
|---|---|
| Read-model/UI implementation | Implemented |
| Red/green presentation and interaction tests | Passed; final UI suite 35/35 |
| Projection/authorization and CLI compatibility | Passed |
| Core/server tests | Core 88 passed; final server suite 618 passed across all targets, 11 pre-existing ignored tests |
| Clippy / WASM / formatting | Final all-target Clippy with `-D warnings -W clippy::perf`, explicit WASM build and `nix fmt` passed |
| SQLx | Passed: forced online/non-incremental regeneration reproduced the reviewed 574-entry cache exactly |
| Full Nix | Passed on both `3eac906` and integrated-master commit `91fb512` on 2026-09-16 with bounded resources, x86_64-linux |
| Browser layout, keyboard and zoom | All six scenarios passed again on 2026-09-16 against the deployed bundle on isolated port 8092, including actual 200% zoom |
| Design comparison/deviation evidence | Recorded above |
| Real Harvest dry-run and data comparison | Passed on 2026-09-16 with separate operator authorization; one Preview, 12 unchanged business tables, readable retained report and complete error download. Source-user mapping errors are recorded below. |
| Commit / implementation PR | Conflicts resolved, acceptance complete and PR #202 updated for review; no automatic merge (T026) |

Commands executed in the Nix shell (shared compiled artifacts only; isolated database URL as above):

```sh
cargo test -p horae-core
SQLX_OFFLINE=true cargo test -q -p horae --features server -- --test-threads=2
SQLX_OFFLINE=true cargo clippy -p horae --features server --all-targets -- -D warnings -W clippy::perf
SQLX_OFFLINE=true cargo build -p horae --features web --target wasm32-unknown-unknown
SQLX_OFFLINE=false CARGO_INCREMENTAL=0 cargo sqlx prepare --workspace -- --features server --all-targets
nix fmt
HORAE_TEST_URL=http://127.0.0.1:8092 node crates/horae/tests/browser/importers.cjs
```

The browser command used existing Playwright and Chromium 152 through the documented environment overrides. A repeated SQLx invocation on the warm target found zero queries; its working-tree cache removal was restored from the staged index, then forced regeneration reproduced all 574 entries without a diff. No metadata loss is included in this change.

The initial full Nix attempt evaluated the checks and started builds, but did not pass. During that attempt, the ten-second synchronization wait in `durable_api_reclaimed_worker_cannot_commit_its_next_page` timed out under contention. The complete suite subsequently passed with two test threads and no heavy parallel Nix builds; no timeout or assertion was weakened. On 2026-09-16, `nix flake check --no-update-lock-file --max-jobs 1 --cores 4` passed on `3eac906`: core 88, server/integration 618, 11 pre-existing ignored tests, Clippy, SQLx cache, formatting, package and NixOS deployment/OIDC checks. The same full command subsequently passed on integrated-master commit `91fb512`. Both runs covered x86_64-linux; other architectures were not executed.

Record the tested commit, scenario/command, actual result and limitations for every executed gate. Never substitute another check's success for an unavailable check. Do not commit credentials, sessions or real data.

## Authorized local upgrade and real Preview — 2026-09-16

- Resolved the two planning/implementation documentation conflicts with master in
  `acceptance.md` and `tasks.md`, preserving completed implementation evidence and
  incomplete acceptance gates. Commit `fcadde6` contains the integrated design
  references; no application-code changes were needed.
- Built matching server/WASM from that worktree using
  `CARGO_BUILD_JOBS=2 SQLX_OFFLINE=true dx build --platform web --fullstack true --force-sequential true --locked`
  inside the Nix shell. Copied the bundle into a private local acceptance directory
  so later builds cannot replace assets underneath the running instance.
- Repeated `tests/browser/importers.cjs` against that same bundle on port 8092
  with the isolated fixture database and an empty Harvest configuration. All six
  scenarios passed, including the current-preview primary action and actual 200%
  zoom (1440 to 720 CSS pixels, DPR 1 to 2). Stopped the temporary fixture server
  afterward; the authorized local instance remains on port 8080.
- Confirmed zero queued/running imports and zero running timers. Created a
  mode-0600 PostgreSQL custom-format backup in a mode-0700 directory and checked
  its archive listing. This verifies archive readability, not a restore drill.
- Stopped the old feature-006 server/worker before starting the new bundle on
  loopback port 8080 with the existing private configuration. Startup applied
  migration 0029 from feature 007; no other migration was pending. Health passed.
  The existing PostgreSQL process and database were preserved.
- Read-only repeatable-read snapshots compare row counts and ordered full-row
  digests for organizations, users, clients, projects, tasks, project_tasks,
  assignments, time_entries, approvals, audit_log, invoices and invoice_line_items.
  All 12 tables were identical before/after both deployment and Preview.
- Used the real browser and existing connected Harvest account. Feature-008
  connection-management controls were present; credentials were not expired.
  Submitted exactly one `DryRun` / `Full` operation through Preview. A browser
  request guard allowed that one preview and status/history/connection reads,
  while blocking import confirmation, retry and account mutations.
- The job reached `succeeded`, producing a readable Preview report. Its predicted
  changes were 8 clients, 4 projects and 12 tasks, with 1 skipped task. All 750
  time entries were reported as errors because one source email has no matching
  Horae user. These are preview predictions, not created business records; no
  successful real import is claimed. No user or email mapping was changed.
- Verified the retained result from history, absence of historical confirmation,
  and the complete 750-record NDJSON error download. Every downloaded error has
  the same missing-user cause; browser checks reported no application exceptions.
  The bound account and synchronization watermark match the pre-upgrade backup.
- The initial one-off browser harness used an obsolete success-banner label after
  the job completed. Corrected that assertion and verified the same retained
  result read-only; did not enqueue another Preview. The live run does not claim
  a fresh current-preview primary-action assertion; fixture coverage above
  supplies that evidence.
- Private backup, snapshots, report, error download and operational logs remain
  outside the repository. No tokens, session data, account identifiers, source
  emails or business records are included in committed acceptance evidence.

This satisfies T025/SC-006 (readable preview with truthful record-level errors and
unchanged business data). A real committing import needs a separate decision and
resolution of the source-user mapping; neither is part of this acceptance.

## Final review and integrated checks

- Reviewed the connection/result/history/retry contract against the existing
  fixture evidence and authorized live result. The master integration adds design
  references and resolves planning-document conflicts, not application changes.
- Integrated production package:
  `/nix/store/5lli9bx8ri2hf62ga294mcbskrw5q8pi-horae-0.1.0.drv`.
- Integrated test suite:
  `/nix/store/sxsp7b881hzxlnd1w61gnm8vmgc5x6xj-horae-tests-0.1.0.drv`.
- Both integrated NixOS deployment/crash-recovery and OIDC checks passed. The
  final acceptance closure changes only Markdown evidence/task state; application
  sources, queries and deployment configuration remain unchanged.
- No Spec Kit extension hooks are configured. T024, T025 and T026 are complete;
  required GitHub checks must still pass on the published head before merge.
