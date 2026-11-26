# Tasks: Harvest mirror mode, parity checks and the durable jobs crate

**Input**: `specs/027-harvest-mirror/` (spec.md, plan.md, research.md, data-model.md, contracts/,
quickstart.md)

**Prerequisites**: spec 025 merged. The maintainer answered OQ-1 to OQ-4 on 2026-11-26 (spec.md,
Clarifications): read only while mirrored (PR 6), a 5-minute default interval (PR 4), and the exit
criterion and validators for the validation period. The mirror is not turned on anywhere before
spec 026's second delivery has merged.

**Tests**: requested (FR-036, SC-003, SC-007). Test tasks come before the code they cover.

**Deliveries**: ten PRs (plan.md, Delivery). Each phase below is one PR.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an unfinished task)
- **[Story]**: the user story in spec.md (US1 to US9)

______________________________________________________________________

## Phase 1: PR 1, extract `horae-jobs` (US1)

**Goal**: the queue lives in `crates/jobs` with no behaviour change (FR-001 to FR-004).

- [ ] T001 [US1] Measure and note the baseline build times of research R-6 on master (warm, after touching `crates/server/src/jobs.rs`): `cargo build -p horae --features server` and `cargo test -p horae-server --no-run`
- [ ] T002 [US1] Create `crates/jobs/Cargo.toml` (`horae-jobs`; sqlx, tokio, serde, serde_json, uuid, chrono, chrono-tz, anyhow, thiserror, tracing; `test-support` feature) and add it to the workspace members and the flake's checks
- [ ] T003 [US1] Move the outbox functions (`enqueue_outbox`, `claim_outbox`, `mark_outbox_*`, `stop_outbox_delivery`) and `OutboxEvent` from `crates/server/src/jobs.rs` to `crates/jobs/src/outbox.rs`; leave the consumer (`jobs/outbox.rs`, which knows plugin events) in the server
- [ ] T004 [US1] Move `JobLease` (`crates/server/src/jobs/lease.rs`) to `crates/jobs/src/lease.rs` as `JobContext`, without `check_account_generation`, which stays in the server as a function over the context's id and token
- [ ] T005 [US1] Move report chunk storage (`jobs/report.rs`) to `crates/jobs/src/report.rs`
- [ ] T006 [US1] Move claim, `fail_claimed`, `run_claimed`, the worker loop, `cleanup`, `status`, `list`, `cancel`, `retry_in` and the payload envelope to `crates/jobs/src/{claim,worker,lib}.rs`; replace the `match` on `JobPayload` in `execute` with the `Registry` and `Handler` of contracts/jobs-crate.md, and `ApiImportError::is_permanent`/`job_error_kind` with `JobError { permanent, error_kind }`
- [ ] T007 [US1] Keep in `crates/server/src/jobs.rs` only `JobPayload`, the import start and commit functions, account generation checks and two handlers (`harvest_api_import`, `harvest_csv_import`) registered at startup in `main.rs`
- [ ] T008 [US1] Point `crates/server`'s and `crates/horae`'s callers at the new paths; keep `crate::jobs::…` names where tests use them through re-exports
- [ ] T009 [US1] Give `horae-jobs` a test migrator over `crates/server/migrations` and move the queue-only tests (claim, lease fencing, outbox) into `crates/jobs/src/…` tests
- [ ] T010 [US1] Regenerate `.sqlx/` (AGENTS.md procedure, adding `crates/jobs/src/lib.rs` to the `touch` list in AGENTS.md) and run the offline full test build
- [ ] T011 [US1] Add a test that a job row written in the pre-extraction shape is claimed and finished by the extracted worker (US1 scenario 2); run the whole suite unchanged: `cargo test -p horae-jobs`, `-p horae-server`, `-p horae-api`, `-p horae --features server`, `nix flake check`; no assertion edited
- [ ] T012 [US1] Measure the same build times as T001 and put both in the PR body; update AGENTS.md's Architecture and Commands for the new crate

**Checkpoint**: US1 acceptance scenarios pass.

## Phase 2: PR 2, concurrency key, heartbeat, retries, delay, priority (US9)

- [x] T013 [US9] Tests first in `crates/jobs`: a second enqueue with the same concurrency key returns `Existing`; once the first job finishes a new one is created; the key is per organization and kind
- [x] T014 [P] [US9] Tests first: a running job whose heartbeat stops is claimable after its stale limit and not before; a job whose heartbeat continues past its stale limit (a 15 s limit, run for 20 s in real time while another worker polls) is never claimed by another worker. Claims never look at a job's age, only at the lease the heartbeat renews, so running longer does not change the result
- [x] T015 [P] [US9] Tests first: a policy of 5 attempts, 10 s, ×2, 10 min gives delays 10, 20, 40, 80 s; a permanent error ends the job at once; each attempt has a row in `horae_job_attempts` with its outcome and error
- [x] T016 [P] [US9] Tests first: a job with `run_at` in the future is not claimed before it; a higher priority is claimed first; a job whose kind has no registered handler stays queued with zero attempts while a registered kind behind it is claimed, and is claimed once a worker with that handler starts (contracts/jobs-crate.md)
- [x] T017 [US9] Migration: `concurrency_key`, `priority`, `heartbeat_at`, `stale_after_secs`, the partial unique index and `horae_job_attempts` (data-model.md)
- [x] T018 [US9] `enqueue_in` with `Enqueue` and `Enqueued` (`max_attempts` required); the insert uses `ON CONFLICT DO NOTHING` and finds the conflicting job by a lookup bounded to three tries. No advisory lock at claim: the partial unique index already admits one queued or running job per key (research R-5)
- [x] T019 [US9] Heartbeat in the existing lease monitor: write `heartbeat_at` and renew `lease_until` to `heartbeat_at + stale_after_secs` every 15 s (four times per stale limit when that is shorter); the claim writes `stale_after_secs` from the handler, clamped to 15..3600; drop the fixed 300 s lease
- [x] T020 [US9] Retry delays from the kind's `RetryPolicy`; the import kinds keep today's curve (2 s, ×2, 300 s) and `HORAE_JOB_MAX_ATTEMPTS` (contracts/jobs-crate.md)
- [x] T021 [US9] Attempts rows written at claim and at finish; `interrupted` for a reclaimed attempt
- [x] T022 [US9] Claim order `priority DESC, available_at, created_at`, restricted to the kinds in the worker's registry (`kind = ANY($n)`), so an unregistered kind stays queued; log each such kind once per worker start; regenerate `.sqlx/`; spec 005/006 tests still pass

## Phase 3: PR 3, scheduler with time zones (US9)

- [ ] T023 [P] [US9] Tests first for the pure `next_due` in `crates/jobs/src/schedule.rs`: every N seconds; daily 09:00 Europe/Madrid across the March and October changes (02:30 on the spring day fires at 03:00, 02:30 on the autumn day fires once); weekly Monday 09:00 in America/New_York; a person zone
- [ ] T024 [US9] Tests first: two schedulers started together, one runs; a firing missed for three slots enqueues one job; a firing is idempotent by `schedule:<id>:<due>`
- [ ] T025 [US9] Migration: `horae_job_schedules` (data-model.md)
- [ ] T026 [US9] `schedule.rs` (cadences, `next_due`) and `scheduler.rs` (leader by `pg_try_advisory_lock` on a dedicated connection, a 15 s tick, enqueue due firings, advance `next_due_at`); `ZoneResolver` implemented in the server from `timezone.rs`
- [ ] T027 [US9] Move the long-timer sweep, budget sweep and OAuth cleanup from `crates/server/src/scheduler.rs` to three handlers and per-organization schedules (every 60 s), upserted at server start for every organization and by `init` (in Rust, so their ids are UUID v7); delete `scheduler.rs`
- [ ] T028 [US9] Check that the sweeps' existing tests pass unchanged and that the long-timer event still fires at most once per timer
- [ ] T029 [P] [US9] Optional: LISTEN/NOTIFY wake-up for workers beside the 2 s poll (research R-5)

**Checkpoint**: US9 acceptance scenarios pass.

## Phase 4: PR 4, mirror engine (US2)

- [ ] T030 [US2] Tests first with the HTTP test server (`crates/harvest/src/http/test_server.rs`): a pass applies one change of every entity in dependency order; a second pass with no changes writes nothing and sends one request per entity; a time entry for a project created during the pass is retried next pass without an error, because the time-entry watermark stays where it was
- [ ] T031 [P] [US2] Tests first: a pass stopped after the projects step resumes from the stored watermarks; a step with record errors keeps its watermark; the watermark is the step start minus 5 minutes
- [ ] T032 [P] [US2] Tests first for the limiter in `crates/harvest/src/http/limiter.rs`: 90 general requests per 15 s and 90 reports per 15 min per account; a 429 with `Retry-After: 3` waits 3 s and the status reports the wait
- [ ] T033 [P] [US2] Tests first: expenses and estimates are counted as unsupported, never errors; unsupported assignment fields are counted
- [ ] T034 [US2] Migration: `harvest_mirrors`; `harvest_entity_type` gains `task_assignment` and `user_assignment`
- [ ] T035 [US2] Limiter in `horae-harvest`, used by every request of the client
- [ ] T036 [US2] Catalog steps with `updated_since` for people, clients, projects and tasks over the existing resolve and apply code (research R-3), saving each entity's watermark in `harvest_credentials.synced_watermark` when its step finishes
- [ ] T037 [US2] Task assignment step onto `project_tasks`; user assignment step onto `project_management_assignments` and `project_member_budgets`, with provenance rows for both
- [ ] T038 [US2] Time entry and invoice steps reusing the import's streaming and invoice code with the mirror's watermarks
- [ ] T039 [US2] `harvest_mirror_pass` handler (contracts/jobs-crate.md): takes the import lock, rechecks the authority and the connection (FR-017), runs the steps in order, pauses the mirror with a reason on a refused connection or a lost authority
- [ ] T040 [US2] `ops::harvest_mirror::{enable, pause, resume, run_pass, set_interval, status}` (contracts/mirror.md) with audit entries; enable checks a completed full import and spec 026's delivery 2 migration, creates the schedules and enqueues a first pass
- [ ] T041 [US2] Refuse manual imports and account switches while the mirror is on or paused (FR-018), in `start_harvest_*` and `account_switch`
- [ ] T042 [US2] `HORAE_HARVEST_MIRROR` in `config.rs`; register the mirror kinds only when set
- [ ] T043 [US2] Regenerate `.sqlx/`; quickstart section 2 tests pass

## Phase 5: PR 5, deletions reconciliation (US6)

- [ ] T044 [US6] Tests first: a deleted time entry is removed; a deleted invoice is removed and its entries reopen; a deleted project with time is archived and reported; a deleted assignment is removed; records without provenance are untouched
- [ ] T045 [US6] `harvest_mirror_reconcile` handler: list ids per entity (time entries in yearly windows), compare with `harvest_import_map`, apply FR-019, report counts; hourly or daily schedule from `harvest_mirrors.reconcile_cadence`

## Phase 6: PR 6, read only, marking, status page and CLI (US3, US4)

- [ ] T046 [US3] Tests first in `horae-server`: every guarded operation (contracts/mirror.md, Read-only refusal) refuses a mirrored record and a new record of a mirrored kind with `harvest_mirror_read_only`; permissions, tokens and settings stay editable; with the mirror off nothing is refused
- [ ] T047 [P] [US3] Tests first in `horae-api` and `horae-harvest-api`: the refusal's mapping on `/api/v1` (409 problem) and `/harvest/v2` (422)
- [ ] T048 [US3] `mirror::guard` (research R-7) and its calls in the operations under `ops/{clients,projects,people,time,invoices}`; `OpError::Conflict` code mapping in the server functions, `/api/v1` and `/harvest/v2`
- [ ] T049 [US3] `synced_from_harvest` on the read DTOs in `crates/types`; the badge and disabled actions on time entries, timesheet rows, projects, clients, tasks, people and invoices; strings in `horae.ftl`; `dioxus-lint` ratchet
- [ ] T050 [US4] Tests first: `harvest_mirror_status` figures against seeded jobs; non-administrators refused on every mirror function
- [ ] T051 [US4] Server functions of contracts/mirror.md in `crates/horae/src/server_fns/importers/mirror.rs`
- [ ] T052 [US4] Status page in `crates/horae/src/pages/importers/mirror.rs` (contracts/mirror.md, Status page), with the clock skew warning and rate-limit wait; strings in `horae.ftl`
- [ ] T053 [US4] CLI `horae mirror status|enable|pause|resume|run|interval` and `horae jobs list --kind` (contracts/cli.md) in `crates/horae/src/cli/mirror.rs`, with tests in `crates/horae/tests/cli_imports.rs` style; document the commands and `HORAE_HARVEST_MIRROR` in AGENTS.md
- [ ] T054 [US4] Browser script for the status page (pause, resume, start a pass) under `crates/horae/tests/browser/`

**Checkpoint**: US2, US3, US4 and US6 acceptance scenarios pass.

## Phase 7: PR 7, parity checker (US5)

- [ ] T055 [P] [US5] Tests first in `crates/core/src/parity.rs`: the rounding predicate (`2 × |d| ≤ n` in minor units, and Horae equals the sum of its lines) at the bound, past it, with negative differences and zero lines; the class order of contracts/parity.md
- [ ] T056 [US5] Tests first in `horae-server`: plant one difference of every class in every layer (missing entry, extra entry, changed minutes, changed rate, moved invoice link, report total off by a cent, invoice total off within the rounding bound, an expense amount, a record changed after the watermark, a deleted record before reconciliation) and assert each finding's class, rule and fields
- [ ] T057 [P] [US5] Tests first: a finding seen once is pending; seen in two consecutive runs it is confirmed; gone in the second run it is not counted; an imported and untouched account reports zero unexplained (SC-003)
- [ ] T058 [P] [US5] Tests first: layer 2 requests stay under 100 per 15 minutes for the nightly window and the rotation; a 3-year history is covered within 30 nightly runs
- [ ] T059 [US5] Migration: `harvest_parity_runs`, `harvest_parity_findings` (data-model.md)
- [ ] T060 [US5] `crates/core/src/parity.rs`: the rounding predicate and the classification
- [ ] T061 [US5] Layer 1 in `crates/server/src/parity/records.rs`: list, normalize with the importer's conversions, pair by Harvest id, compare the fields of contracts/parity.md
- [ ] T062 [US5] Layer 2 in `parity/totals.rs`: Harvest time and uninvoiced reports per month against `ops::reports::time` and the uninvoiced view, under the mirror authority's actor
- [ ] T063 [US5] Layer 3 in `parity/invoices.rs`
- [ ] T064 [US5] Fingerprints, confirmation against the previous run, storage, 90-day and 1-year retention in `jobs::cleanup`'s schedule
- [ ] T065 [US5] `harvest_parity_check` handler and nightly schedule at `parity_local_time`; `run_parity` and `list_harvest_parity_runs` operations and server functions
- [ ] T066 [US5] CSV route `GET /api/import/harvest/parity/{run_id}/findings.csv` in `crates/api/src/harvest_parity.rs`, with tests for columns, quoting and administrator-only access
- [ ] T067 [US5] CLI `horae mirror parity`, `horae jobs findings`, and `jobs report` on parity and pass jobs (contracts/cli.md)
- [ ] T068 [US5] Parity on the status page: latest run, counts, history, download links, days since the last unexplained difference and error

## Phase 8: PR 8, alerts and rehearsal (US5)

- [ ] T069 [US5] Tests first: an unexplained run sets the alert, a clean run clears it, one email per changed count when mail is configured, none otherwise
- [ ] T070 [US5] `alert_run_id` handling, navigation dot and banner, email through the existing mail transport; strings in `horae.ftl`
- [ ] T071 [US5] Record the rehearsal's authorization, date and fixture plan in `specs/harvest-parity.md` (test account only; never name the account or its id)
- [ ] T072 [US5] Rehearsal against the Harvest test account with disposable fixtures (quickstart section 4): one fixture per class; record every fixture id; measure an empty pass's duration and request count (SC-002) and the time from a change in Harvest to its appearance in Horae (SC-001)
- [ ] T073 [US5] Answer research R-2's open points (payment moves the invoice, project moved between clients, assignment changes, inclusivity), R-3's per-person rate question and R-8's fixed-fee check; update research.md and, if needed, the steps
- [ ] T074 [US5] Delete the fixtures and record the cleanup in `specs/harvest-parity.md`

## Phase 9: PR 9, cutover (US7)

- [ ] T075 [US7] Tests first: cutover after a clean run turns the mirror off, deletes the schedules and makes records editable; with unexplained findings it is refused unless the override equals the count; the audit entry has the watermarks and the parity run
- [ ] T076 [US7] `harvest_mirror_cutover` handler (final pass, final parity, then off) and `cut_over` operation, server function, CLI `horae mirror cutover`, and the page's confirmation dialog
- [ ] T077 [US7] Runbook in `specs/027-harvest-mirror/quickstart.md`: shipyard backup before cutover, reset, full import, mirror on, validation, cutover, rollback by going back to Harvest and importing again

## Phase 10: PR 10, write-through (US8, optional)

Starts only after the maintainer authorizes it (FR-041).

- [ ] T078 [US8] Tests first with the HTTP test server: create, edit, delete, start and stop for an enrolled person call Harvest first with the person's `user_id` and store the returned id; a Harvest refusal or timeout refuses the change with Harvest's reason; a failed Horae commit after Harvest accepted is repaired by the next pass; a person not enrolled gets the read-only refusal
- [ ] T079 [US8] Migration `harvest_write_through`; `HORAE_HARVEST_WRITE_THROUGH`; `set_write_through` operation, server function and the People screen's switch for administrators
- [ ] T080 [US8] Time-entry writes in `horae-harvest` (`POST`, `PATCH`, `DELETE`, `restart`, `stop`) and the calls in the time operations (research R-10)
- [ ] T081 [US8] Test-account trial recorded in `specs/harvest-parity.md` with fixture ids and cleanup; production enablement only with the maintainer's recorded authorization

## Dependencies

- PR 1 → PR 2 → PR 3 → PR 4. PR 5 and PR 6 need PR 4 and are independent of each other. PR 7
  needs PR 4 (and PR 5 for `deletion_pending`). PR 8 needs PR 7. PR 9 needs PR 6 and PR 8. PR 10
  needs PR 6 and the maintainer's authorization.
- Inside a phase, tests first, then migration, then code.
- [P] tasks touch different files.

## Parallel opportunities

- T013 to T016 (different test modules), T023 with T024's setup, T030 to T033, T046 with T047, T055
  to T058.
- PR 5 and PR 6 can be developed in parallel after PR 4.
