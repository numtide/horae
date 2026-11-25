# Implementation Plan: Harvest mirror mode, parity checks and the durable jobs crate

**Branch**: `spec/harvest-mirror` (documents); one branch per PR below | **Date**: 2026-11-25 |
**Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/027-harvest-mirror/spec.md`

**Order**: the jobs PRs (1 to 3) can start now. The mirror PRs need spec 025 (merged) and must not be
turned on before spec 026's second delivery (task rate templates) has landed, or rate parity will
differ by construction.

## Summary

The durable queue moves out of `horae-server` into `crates/jobs` (`horae-jobs`) without behaviour
change, then gains concurrency keys, heartbeats, per-kind retry policies with an attempts table,
delayed and prioritized jobs, and a single-leader scheduler with time-zone schedules. On top of it,
`horae-server` registers three new kinds: a mirror pass (incremental per entity with
`updated_since`, reusing the importer's resolve and apply code), a deletions reconciliation, and a
parity check in three layers whose findings are stored and exported as CSV. A guard in the shared
operations keeps mirrored records read only. A status page and `horae mirror` commands let an
administrator watch and control it. Write-through for pilot people comes last and is optional.

## Technical Context

**Language/Version**: Rust 2024 (workspace toolchain from the flake)

**Primary Dependencies**: sqlx (compile-time checked queries), tokio, serde, chrono and chrono-tz
(already in the workspace), Dioxus 0.7 for the status page, Axum for the CSV route, `horae-harvest`
(the existing blocking HTTP client) for Harvest calls. No new external dependency.

**Storage**: PostgreSQL. Migrations `0011_*` onwards in `crates/server/migrations` (numbers taken
when written): job columns and attempts and schedules tables (PR 2, PR 3), mirror and parity tables
(PR 4, PR 7), write-through enrollments (PR 10).

**Testing**: `cargo test -p horae-jobs` (new; database tests with `#[sqlx::test]` against the
server's migrator), `cargo test -p horae-server` (mirror and parity against the Harvest test server in
`crates/harvest/src/http/test_server.rs` and fixture data), `cargo test -p horae-api` (CSV route,
read-only refusals on `/api/v1`), `cargo test -p horae-harvest-api` (read-only refusals on `/v2`),
`cargo test -p horae --features server` (server functions, CLI, status page), browser script for the
status page, and a rehearsal against the Harvest test account (T071 to T074).

**Target Platform**: Linux server plus WASM client

**Project Type**: web application (virtual workspace; this feature adds `crates/jobs`)

**Performance Goals**: a pass with no changes in under 30 s using one request per entity (SC-002);
a change visible within two intervals (SC-001); a dead pass reclaimed within about a minute (FR-006)

**Constraints**: Harvest limits of 100 requests per 15 s (general) and 100 per 15 min (reports);
integer minutes and minor units; i18n catalog for every string; `dioxus-lint` ratchet; `.sqlx/`
regenerated; the CLI contract of spec 006 unchanged

**Scale/Scope**: one organization on shipyard; an account in the order of 100,000 time entries,
2,000 invoices and a few hundred projects

## Constitution Check

*Before Phase 0 and again after Phase 1.*

| Principle | Check | Result |
| --- | --- | --- |
| I. Exactness | The mirror and parity convert with the importer's exact conversions (decimal hours to minutes, decimal amounts to minor units). Parity compares integers; the rounding rule is `2 × abs(difference) ≤ n` in minor units | Pass |
| II. Domain purity | The rounding-rule predicate, the finding classification and the schedule cadence arithmetic are pure functions; the rounding predicate and classification go in `horae-core` with unit and property tests. The jobs crate holds no domain rule | Pass |
| III. Single datastore | New tables in PostgreSQL with UUID v7 keys and `org_id`; no external queue | Pass |
| IV. Shared operations | Mirror passes write through the importer's job path, as imports do (004, 005, 019, 025), authorized when an administrator turns the mirror on through a server function and rechecked at the start of every pass (FR-017). The read-only guard lives in the shared operations, so every surface gets it. Write-through stays inside the time operations. Mirror controls are server functions calling new `ops::harvest_mirror` operations; the CLI reaches them over HTTP like `horae jobs` | Pass, with one Complexity Tracking entry |
| V. Builds and formatting | `nix fmt`, `nix flake check`, `.sqlx` check, the new crate added to the flake checks | Pass |

After design: unchanged.

## Project Structure

### Documentation (this feature)

```text
specs/027-harvest-mirror/
├── spec.md
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── jobs-crate.md        # crate API, job kinds, retry policies, schedules
│   ├── mirror.md            # server functions, status page, read-only error, config
│   ├── parity.md            # parity tables, classes, rules, CSV, alerts
│   └── cli.md               # horae mirror, horae jobs additions
├── checklists/requirements.md
└── tasks.md
```

### Source code

```text
crates/jobs/                                  # new: horae-jobs
├── Cargo.toml
└── src/
    ├── lib.rs                                # Registry, Handler, Enqueue, re-exports
    ├── claim.rs                              # claim, heartbeat, stale recovery, concurrency key
    ├── lease.rs                              # moved from server jobs/lease.rs (JobContext)
    ├── retry.rs                              # RetryPolicy, attempts
    ├── outbox.rs                             # moved from server jobs.rs + jobs/outbox.rs mechanism
    ├── report.rs                             # moved: report chunk storage
    ├── schedule.rs                           # cadences, next due time in a zone (pure)
    ├── scheduler.rs                          # single-leader loop
    └── worker.rs                             # worker loop, shutdown
crates/server/src/jobs.rs                     # becomes the import kinds' handlers and start functions
crates/server/src/scheduler.rs                # removed; its sweeps become handlers + schedules (PR 3)
crates/server/src/mirror.rs, mirror/          # new: pass driver, steps, reconcile, guard, limiter use
crates/server/src/parity.rs, parity/          # new: layers, normalize, classify, store, alert
crates/server/src/ops/harvest_mirror.rs       # new: enable, pause, resume, run, interval, cutover, write-through enrollment
crates/server/src/ops/{clients,projects,people,time,invoices}/…   # guard calls
crates/server/src/importers/harvest/…         # steps reusable by the mirror, assignment mapping
crates/harvest/src/http.rs, http/limiter.rs   # per-account two-bucket limiter, time-entry writes (PR 10)
crates/core/src/parity.rs                     # rounding rule, finding classification
crates/types/src/harvest_mirror.rs            # status and parity DTOs
crates/api/src/harvest_parity.rs              # CSV download route
crates/horae/src/server_fns/importers/mirror.rs   # server functions, beside the import ones
crates/horae/src/pages/importers/mirror.rs    # status page, in the Harvest section
crates/horae/src/cli/mirror.rs                # horae mirror …, horae jobs findings
crates/i18n/locales/en/horae.ftl              # strings
crates/server/migrations/00NN_*.sql
```

## Phases

- **Phase 0** (research.md): Harvest limits (R-1), watermarks (R-2), importer reuse (R-3), background
  work inventory (R-4), underway (R-5), crate boundary and build measurement (R-6), read-only guard
  (R-7), parity (R-8), alerts (R-9), write-through (R-10), exit criterion (R-11).
- **Phase 1**: data-model.md, contracts/, quickstart.md.
- **Phase 2**: tasks.md.

## Delivery

Ten PRs. Each leaves `nix flake check` green and is reviewable on its own.

1. **Extract `horae-jobs`** (US1; FR-001 to FR-004). Move the mechanism, keep table names and
   behaviour; the server registers the two import handlers. Build times before and after in the PR
   body.
1. **Concurrency key, heartbeat, retries, delay, priority** (US9; FR-005 to FR-008). One migration:
   `concurrency_key`, `priority`, `heartbeat_at`, the partial unique index, `horae_job_attempts`.
1. **Scheduler with time zones** (US9; FR-009, FR-010). `horae_job_schedules`, the leader loop, and
   the three existing sweeps moved onto it.
1. **Mirror engine** (US2; FR-011 to FR-018). Mirror table, the pass kind, per-entity steps with
   watermarks, assignment mapping, the rate limiter, enable and pause operations without UI.
1. **Deletions reconciliation** (US6; FR-019).
1. **Read-only guard and marking, status page, CLI** (US3, US4; FR-020 to FR-026).
1. **Parity checker** (US5; FR-027 to FR-033, FR-035, FR-036 except the rehearsal).
1. **Parity alerts and the rehearsal on the test account** (US5; FR-034, FR-036).
1. **Cutover** (US7; FR-037, FR-038).
1. **Write-through, optional** (US8; FR-039 to FR-041). Only after the maintainer authorizes it.

LISTEN/NOTIFY wake-ups (R-5) are an optional task after PR 3; they change no contract.

## Open items for implementation

- OQ-1 to OQ-4 (spec): the plan follows the recommendations. If OQ-1 is answered no, PR 6 drops the
  guard and parity gains an "edited in Horae" class.
- User assignments' per-person hourly rate has no Horae column (research R-3). If the production
  account bills by person, layer 2's billable amounts will differ; check in the rehearsal and decide
  whether to add the column before validation.

## Complexity Tracking

| Choice | Why needed | Simpler alternative rejected because |
| --- | --- | --- |
| Scheduled passes write on the authority an administrator gave when turning the mirror on, rechecked at the start of each pass, rather than on a fresh request | A mirror runs every few minutes with nobody signed in | Asking an administrator to start each pass defeats the mirror; a system actor with no person behind it would be a new kind of authority the constitution does not have |
