# Tasks: Safe Harvest Account Switching

**Input**: spec.md, plan.md, research.md, data-model.md, contracts/account-switch.md and quickstart.md.

## Phase 1: Setup

- [X] T001 Record the baseline, checklist and traceability review in specs/007-harvest-account-switch/acceptance.md; verify ignore rules and prepare an isolated PostgreSQL database.

## Phase 2: Foundation

- [X] T002 Add failing pure eligibility tests in crates/core/src/importers/harvest/types.rs and database generation/change tests in crates/horae/src/importers/harvest/account_switch.rs.
- [X] T003 Add migration 0029_harvest_connection_generations.sql, connection metadata/policy in crates/core/src/importers/harvest/types.rs and shared transaction gate in crates/horae/src/importers/harvest/account_switch.rs.

## Phase 3: US1 — Explicit safe account change (P1)

- [X] T004 Add preservation, disconnected binding and stale confirmation tests in crates/horae/src/importers/harvest/account_switch.rs before implementing the atomic change.
- [X] T005 Implement atomic credential/binding removal and counter advancement in crates/horae/src/importers/harvest/account_switch.rs; expose administrator status/change functions in crates/horae/src/server_fns/importers.rs.
- [X] T006 Add revision-bound OAuth/disconnect tests in crates/horae/src/importers/harvest.rs and credentials.rs, then implement single-use actor-bound OAuth state and transactional revision validation there.
- [X] T007 Add connected/disconnected/confirmation UI tests in crates/horae/tests/import_jobs_ui.rs and implement Change account with the existing Modal in crates/horae/src/pages/importers.rs.

## Phase 4: US2 — Explain and enforce blocked changes (P1)

- [X] T008 Test provenance, queued/running API/CSV and competing reset cases in crates/horae/src/importers/harvest/account_switch.rs; implement authoritative eligibility and actionable errors there.
- [X] T009 Extend real-session authorization coverage in crates/horae/src/server_fns/importers.rs for non-admin, inactive and cross-organization actors; verify status/error output excludes secrets.
- [X] T010 Implement blocked-state explanation and busy/cancel/error behavior in crates/horae/src/pages/importers.rs and verify labelled dialog accessibility in crates/horae/tests/import_jobs_ui.rs.

## Phase 5: US3 — Fence stale work (P1)

- [X] T011 Add generation/idempotency/retry/concurrent enqueue tests in crates/horae/src/jobs.rs before implementing generation-bound acceptance, duplicate checks and retries with the short gate.
- [X] T012 Add old-generation execution tests in crates/horae/src/importers/harvest.rs and enforce generation checks after the import lock, before credential access/HTTP.
- [X] T013 Capture expected generation in crates/horae/src/server_fns/importers.rs and crates/horae/src/pages/importers.rs; test omitted generation remains zero and old requests fail closed.
- [X] T014 Add CLI connection-status/submission contract tests in crates/horae/src/cli/imports.rs and neighboring tests; capture generation once per request and preserve the existing no-automatic-retry behavior on uncertain submission.
- [X] T015 Verify reconnect, A→B→A, retained reports, expired identities, restart persistence and competing callback/change scenarios across crates/horae/src/importers/harvest/account_switch.rs, credentials.rs and jobs.rs.

## Phase 6: Validation and delivery

- [ ] T016 Regenerate .sqlx/ against the isolated migrated database and pass core/server tests, Clippy, WASM build and nix fmt without modifying the live preview database.
- [ ] T017 Update affected specs/004-*/contracts/, specs/005-*/contracts/ and specs/006-\*/contracts/ with compatibility notes; record actual checks and remaining limits in specs/007-harvest-account-switch/acceptance.md and quickstart.md.
- [ ] T018 Review the complete diff for races, authorization, preservation and scope; run applicable Nix checks and open the stacked PR against feat/harvest-jobs-cli (no automatic merge).

## Dependencies and execution

Execute setup and foundation first. US1 supplies the status/change contract used by US2. US3 shares the gate and must complete before any account-switch flow is delivered. Add tests before each implementation change; run focused tests at each checkpoint and mark tasks only after evidence. Tasks touching the same files execute sequentially. Independent read-only checks may run concurrently; no implementation delegation is required.

All three P1 stories constitute the minimum safe delivery; do not ship the UI without stale-work fencing.

## Traceability

| Requirement | Tasks |
| --- | --- |
| FR-001, FR-002, FR-003 | T002, T005, T007, T010 |
| FR-004 | T008, T011, T015 |
| FR-005 | T004, T005, T015 |
| FR-006, FR-009 | T006, T007, T015 |
| FR-007, FR-008 | T006, T011–T015 |
| FR-010 | T007, T009, T010 |
| FR-011 | T002, T004, T006–T016 |
| SC-001, SC-005 | T007, T010, T016 |
| SC-002, SC-003 | T004, T006, T008, T011–T015 |
| SC-004 | T009, T018 |
