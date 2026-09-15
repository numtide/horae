# Implementation Plan: Consistent Harvest Import Experience

**Branch**: `feat/harvest-import-ux` | **Date**: 2026-09-15 | **Spec**: [spec.md](spec.md)

**Input**: `specs/008-harvest-import-ux/spec.md`

## Summary

Align the existing Importers surface with Horae's handoff while making connection management, durable work and results understandable. Keep the coordinator and handlers, add small testable presentation helpers, and reuse the admin shell, native Modal, banners, counters and utility CSS.

The only non-visual change is one backwards-compatible read-only retry-availability field. No new dependencies, crates, migrations, provider requests, queue, secret editor or import-engine changes.

## Technical Context

**Language/Version**: Rust edition 2024; toolchain pinned by the existing Nix flake.

**Primary Dependencies**: Existing Dioxus 0.7 fullstack, serde, chrono, sqlx and CSS utilities. Existing Playwright-compatible browser-test convention.

**Storage**: Existing PostgreSQL records; no schema change. Changed status/history queries require SQLx cache regeneration.

**Testing**: Presentation cases, existing Dioxus UI harness, sqlx projection/authorization tests, CLI compatibility tests and isolated-instance browser regression.

**Target Platform**: Linux server and WASM browser; existing English product copy.

**Project Type**: Existing fullstack application; one administrative surface.

**Performance Goals**: Preserve polling cadence and bounded pagination (default 20, maximum 100); no per-history-row requests, external lookups or whole-history scans. Add availability to existing bounded projection queries.

**Constraints**: Preserve features 005–007. Unknown data remains unknown. Real-account validation requires separate operator authorization.

**Scale/Scope**: API and CSV import flows; viewport widths 360/768/1440, short height 640 and 200% zoom. No retention changes.

## Constitution Check

Pre-research and post-design gates pass by design; runtime verification remains pending implementation.

| Principle | Compliance |
|---|---|
| I. Exactness | Preserve integer report counts and business values; no display-layer monetary/duration rules. |
| II. Domain purity | No I/O enters core; presentation helpers stay in the app. |
| III. Single datastore | Existing PostgreSQL data only, no schema or identifier changes. |
| IV. Server functions | Existing authenticated admin-scoped server functions own reads and mutations; no ad-hoc browser fetches. |
| V. Reproducibility | Nix formatting, server/core tests, Clippy, WASM, SQLx preparation and full flake check remain pre-merge gates. |

## Project Structure

### Documentation (this feature)

```text
specs/008-harvest-import-ux/
  spec.md
  plan.md
  research.md
  data-model.md
  contracts/importer-ui.md
  quickstart.md
  tasks.md
  checklists/requirements.md
  acceptance.md
```

### Source Code (repository root)

```text
crates/horae/src/
  pages/importers.rs                    # existing coordinator/views
  pages/importers/presentation.rs       # small display helpers + unit cases
  models/jobs.rs                       # additive retry snapshot type
  jobs.rs                              # status/list projection + DB tests
  server_fns/importers/authorization_tests.rs
  components/{admin_shell,modal,badge,toast}.rs  # reuse, not redesign
  cli/imports/tests.rs                  # compatibility fixtures
crates/horae/assets/css/horae.css        # necessary importer structure only
crates/horae/tests/
  import_jobs_ui.rs                     # extend existing harness
  browser/importers.cjs                 # new isolated browser acceptance
.sqlx/                                 # regenerated projection metadata
```

**Structure Decision**: Preserve source selection, job watching, stale-response guards and preview origin. No generic workflow framework or global store. Use sibling `importers.rs` + `importers/` only for testable presentation logic; extract view components only when the scoped implementation needs them.

## Phase 0: Research

[research.md](research.md) resolves the read-model questions. Reports already carry source/mode at enqueue; actual retry availability is missing. A bounded read-only research task confirmed the minimal addition. No new technology choice requires external research.

## Phase 1: Design and Contracts

- [data-model.md](data-model.md) separates viewing context, job state, report completeness and action eligibility.
- [contracts/importer-ui.md](contracts/importer-ui.md) defines state/actions, copy, compatibility and design deviations.
- [quickstart.md](quickstart.md) defines fixtures, browser checks and the separately authorized real dry-run.
- Keep `ConnectionStatus` and `JobStatus::can_retry()` unchanged. Add advisory `JobStatus.retry_availability`; keep partial-report classification independent from it. `jobs::retry()` remains authoritative.
- Keep HTTP paths, payloads, existing JSON fields, kind/state wire values and CLI exit behavior. Missing/future availability defaults to unknown and disables UI retry with guidance.
- The agent-context update script is absent from `.specify/scripts/`. Do not invent/download it; this plan provides technical context. No unrelated `AGENTS.md` changes.

## Phase 2: Implementation Preparation

Generate user-story tasks and perform read-only Spec Kit analysis, then deliver a documentation PR. Implementation is a subsequent phase. Read applicable design/Rust/testing skills before implementation; write matching acceptance tests red before behavior changes.

Priority order: shared display decisions → connection UX (MVP) → preview/results → history/progress → cross-state browser verification. Preserve independently testable stories even where shared-file edits require serialization.

## Complexity Tracking

No constitutional exceptions. One additive read projection prevents known-invalid retry affordances without granting mutation authority or changing imported data.
