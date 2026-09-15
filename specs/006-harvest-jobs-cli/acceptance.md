# Acceptance Evidence: Harvest Jobs CLI

**Status**: Implemented and validated; all 19 tasks complete. Delivered in PR #199.

## Environment and workflow

- Worktree `/tmp/horae-harvest-jobs-cli`, branch `feat/harvest-jobs-cli`, PR #199, based on merged master `16eca3d`.
- Nix toolchain: cargo 1.96.1, rustc 1.96.1. Shared build cache `/tmp/horae-durable-harvest-jobs/target`; no second build overlaps compilation.
- Root worktree preserved; only pre-existing `.playwright-mcp/` is untracked there.
- Existing `.gitignore` covers target, local stack state, worktrees, direnv and scratch. Secret session fixtures use disposable temp directories, never tracked files.
- Default PostgreSQL port 5432 was unavailable; feature database tests use an isolated instance on 55440, not production/development data.
- Spec Kit: read specify/clarify/plan/tasks/analyze/implement skills; ran prerequisite resolution, setup-plan and setup-tasks scripts. No extension hooks configured. Standalone CLI and agent-context update script are absent; no invocation of those is claimed.
- Clarify: zero new interactive answers; implementation uses the recommended server-client design with explicit documented assumptions. Requirement checklist 16/16.
- Analyze: 13 FR and 6 SC mapped to 19 tasks; no uncovered requirement or constitutional exception. Implementation evidence is recorded below.

## Test evidence

- TDD parser regression first failed on the original CLI's unknown `--session-file` (exit 101), then passed after implementation.
- Initial CLI suite: 11 passing tests covering documented arguments, rejected flags/limits, preserved serve defaults, secure session files, URL/cookie rejection and no-redirect behavior.
- Content-bound enqueue regressions first failed for changed API mode/scope and changed CSV bytes (exit 101), then both passed against disposable PostgreSQL.
- Stable-route regression first failed on missing `/api/import/harvest/start` (exit 101), then passed.
- Importer endpoint suite: 7 passed, 1 existing manual report-memory stress ignored. Includes actual PostgreSQL sessions and existing role/organization/error-download matrix. This is not yet the full CLI acceptance matrix.
- Native PostgreSQL SHA-256 verified against the known `test` digest; no extension or dependency added.
- Executable configuration test passed: JSON configuration failure without loading invalid `DATABASE_URL` or `HORAE_JOB_MAX_ATTEMPTS`.
- Expanded client suite: 12 passed, including streamed archive publication/no-clobber, failed download preserving an existing file, confirmed partial reports and a deadline interrupting an in-flight request.
- Cancellation/completion race regression first failed (`cancellation_requested` versus `already_complete`), then passed after reporting actual terminal state.
- Expanded importer suite: 8 passed, 1 existing manual stress ignored. Its real-session matrix invokes the CLI parser/transport for every operation under member/manager/inactive/demoted/missing/foreign identities; checks API preview/commit submission and resubmission, cancellation/retry/deadline, complete archived errors and paginated history. The real worker completes CSV preview/commit/reimport with exactly 0/1/1 stored test entries.
- Queue regression suite: all 32 tests passed. Existing recovery, fencing, retention and policy checks remain green.
- Full baseline validation before the additional acceptance fixtures: 87 core tests; 507 server unit tests passed with 11 existing manual probes ignored; integration binaries passed 5 admin-shell, 4 CLI, 4 navigation, 14 import-UI, 36 database and 1 timer-widget tests. All-target server Clippy passed with warnings denied.
- Expanded executable suite: 5 passed, including terminal failure/cancellation/partial-success/timeout/forbidden exits across wait/retry/API submission. The signal regression now observes the same running job from a new process and asserts zero cancellation calls.
- Expanded client suite: 15 passed, adding bounded/malformed JSON, missing/mismatched jobs and invalid CSV files before network submission.
- Concurrent identical CSV submissions return one ID. After upload removal, identical resubmission neither changes the ID nor recreates the upload; conflicting bytes are rejected. Regression passed against disposable PostgreSQL.
- The user-requested pause interrupted a later full-suite run (exit 130); that run did not reach SQLx preparation and is not counted as passing evidence.
- Actual-binary acceptance passed using `HORAE_CLI_TEST_BINARY` with the registered server functions and PostgreSQL sessions. Every operation rejects member/manager/inactive/demoted/missing/expired sessions; foreign job data and archives remain inaccessible. Expiry is persisted through the session store, not represented by a malformed cookie.
- API preview/commit/reimport submitted from separate processes finish through the production adapter and durable lease against loopback Harvest HTTP; stored entries are exactly 0/1/1 and CLI/web job snapshots agree. The external Harvest service is a fixture, not a live account.
- CSV preview/commit/reimport finish with 0/1/1 stored entries after the submitting process exits and the local CSV is removed. Each run starts a fresh worker after submission, then observes completion from a new process. Existing checkpoint/recovery tests cover interrupted attempts; no production worker or URL override is added.
- Final expanded server suite passed: 512 unit tests, 11 existing manual probes ignored; all integration binaries passed (5 admin, 5 CLI, 4 navigation, 14 import UI, 36 database, 1 timer). The real-session matrix used the actual executable. All-target Clippy and WASM compilation passed.
- `cli_restart` passed against a disposable database and real `horae serve` processes: block domain writes, submit CSV, kill the server, delete the local file, restart, and observe the same job succeed with exactly one entry. The test advances only the dead claim's expiry to avoid a five-minute wait.
- SQLx metadata regenerated after cleaning only the app's development artifacts and disabling incremental compilation. An earlier incremental run emitted only touched queries and was not accepted as the final cache. Fresh preparation also removed three obsolete preview-query entries from the baseline; current preview queries remain cached.
- Full local `nix flake check -L` passed on x86_64-linux (exit 0): fresh SQLx cache verification, offline builds, core/server/integration tests including `cli_restart`, Clippy, formatting, production server/WASM package, NixOS e2e and OIDC e2e. Other platform checks were not run.
- GitHub [CI run 34910899078](https://github.com/numtide/horae/actions/runs/34910899078) passed for implementation commit `40ab6fc`: Format and full Flake Check, including their cache/cleanup steps. The final acceptance update changes documentation only.
- Final scope audit preserves the root worktree and introduces no dependency, crate, migration, scheduler or new job kind. No `after_implement` extension hooks are configured.

## Completion audit

Every FR-001–FR-013 and SC-001–SC-006 has implementation and passing acceptance evidence below. All task checks and local/CI release gates passed before closure.

| Requirement | Implementation and evidence |
|---|---|
| FR-001, FR-003; SC-001 | Actual-binary real-session acceptance submits both sources/modes, exits before execution, then reads the retained jobs. API fixture and CSV worker complete; local CSV deletion does not affect execution. |
| FR-002 | Parser rejects conflicting flags; executable HTTP contract checks Full/Incremental; existing API source tests cover watermark fallback and incremental behavior. |
| FR-004 | Status/history commands use the same org-scoped server functions; actual-binary acceptance pages retained history and compares API job snapshots with web requests. |
| FR-005; SC-002 | Actual-binary acceptance compares the complete archived errors byte-for-byte with web; client tests cover incomplete summaries, terminal states, oversized archives, truncation and no-clobber publication. |
| FR-006; SC-003 | Actual-binary cancel/retry retains IDs; existing queue/checkpoint tests exercise policy, confirmed progress, expired-lease fencing and recovery; both source acceptance fixtures reimport without duplicates. |
| FR-007; SC-006 | Executable signal test interrupts a waiter, reads the same running job from a new process and asserts zero cancel requests; client deadline test interrupts a pending request. |
| FR-008; SC-004 | All nine operations have executable JSON success cases and real-session rejection cases. Exit matrix covers failure, cancellation, partial success, deadline, interruption, configuration and indeterminate acknowledgement. |
| FR-009, FR-010; SC-005 | Private origin-bound session loading; no redirects/proxies or plaintext remote transport. Actual-binary role/org/expired-session matrix asserts no session value in stdout/stderr. |
| FR-011 | Atomic content-bound enqueue; identical/conflicting mode/scope/bytes tests, concurrent resubmission and post-upload-cleanup identity preservation; age/version validation. |
| FR-012 | Existing API/CSV adapters, credential store and source validators; source suites retain exact numeric, user-matching and account-binding coverage. |
| FR-013 | Remote dispatch precedes deployment configuration; executable tests use invalid server-only configuration. Real-session acceptance starts workers after detached submissions; no CLI worker/DB branch exists. |

Recovery evidence combines feature-005 checkpoint tests, feature-006 client
interruption tests and `cli_restart` using actual server processes against a
disposable database. A live external Harvest account or changes to a production
deployment are not claimed.
