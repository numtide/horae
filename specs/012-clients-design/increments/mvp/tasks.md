# Tasks: Clients MVP

Input: [plan.md](plan.md), [spec.md](spec.md), research, data model and contracts.
All four P1 stories are required; US1 alone is not goal completion.

## Phase 1: Setup

- [x] T001 Reconcile authorized increment and Harvest evidence in specs/012-clients-design/increments/mvp/spec.md and research.md (MVP-010).
- [x] T002 Generate plan/contracts and readiness checklist in specs/012-clients-design/increments/mvp/ (MVP-003, MVP-007, MVP-008).
- [x] T003 Run cross-artifact analysis and record findings in specs/012-clients-design/increments/mvp/progress.md (MVP-012).

## Phase 2: Foundation

- [x] T004 Initialize disposable Nix/PostgreSQL test environment; record exact commands and isolation in specs/012-clients-design/increments/mvp/progress.md (MVP-011).
- [x] T005 Verify existing ignore patterns and test/fixture entry points in .gitignore and nix/checks/; prepare only necessary synthetic fixtures (MVP-011).

## Phase 3: US1 — Find a client (P1)

Independent check: combinations of name/lifecycle/currency, counts, direct links,
empty/error/retry and hidden-project exclusion under current roles.

- [x] T006 [US1] Add failing catalog/projection access tests in crates/horae/src/server_fns/clients/tests.rs covering two orgs, inactive users, hidden currencies/counts and existing catalog access (MVP-001, MVP-003).
- [x] T007 [US1] Implement narrow authorized summaries in crates/horae/src/models/client.rs and crates/horae/src/server_fns/clients.rs without per-client queries (MVP-001, MVP-003).
- [x] T008 [US1] Test filter predicates/counts in crates/horae/src/pages/clients/filters.rs then implement combined search/lifecycle/currency filtering (MVP-001, MVP-SC-001).
- [x] T009 [US1] Align list layout and truthful loading/empty/error/retry states in crates/horae/src/pages/clients.rs; preserve import/lifecycle entry points and real detail links (MVP-001, MVP-006, MVP-008).

## Phase 4: US2 — Review client work (P1)

Independent check: known/unknown/foreign client detail by role, persisted billing,
real related rows, no hidden fields and no mixed-currency totals.

- [x] T010 [US2] Add failing detail/rate/invoice authorization and serialization tests in crates/horae/src/server_fns/clients/tests.rs and client-filter regression in server_fns/projects/privacy_tests.rs (MVP-002, MVP-003, MVP-SC-002).
- [x] T011 [US2] Implement exact-client and manager billing reads in crates/horae/src/server_fns/clients.rs, reusing current related-project/invoice authorities (MVP-002, MVP-003).
- [ ] T012 [US2] Build actual detail with independent panel error/retry and stale-route protection in crates/horae/src/pages/clients/detail.rs; fix detail sidebar selection in crates/horae/src/route.rs (MVP-002, MVP-008).

## Phase 5: US3 — Maintain existing information (P1)

Independent check: create/reload/edit/cancel/errors with zero/unset rates,
concurrent currency edits, unchanged history and preserved lifecycle behavior.

- [ ] T013 [US3] Add failing core validation tests in crates/core/src/client.rs and persistence/history/rate-concurrency/no-op event tests in crates/horae/src/server_fns/clients/tests.rs (MVP-004, MVP-005, MVP-006).
- [ ] T014 [US3] Implement/reuse core validation and atomic explicit-rate profile saves in crates/core/src/client.rs and crates/horae/src/server_fns/clients.rs; preserve existing picker/legacy callers (MVP-004, MVP-005).
- [ ] T015 [US3] Share labeled form in crates/horae/src/pages/clients/form.rs for list/detail, retaining entered data, pending guards, uncertain-create recovery and focus restoration (MVP-004, MVP-008, MVP-009).
- [ ] T016 [US3] Verify existing activation/deactivation and import entry points in crates/horae/src/pages/clients.rs and server_fns/clients/tests.rs remain truthful and non-cascading (MVP-006).

## Phase 6: US4 — Continue existing workflows (P1)

Independent check: context without/with drafts and exact pending recovery, invalid
IDs and direct navigation; no business record created by navigation.

- [ ] T017 [US4] Add failing context route/prefill/recovery precedence tests in crates/horae/src/route.rs, pages/new_project/draft.rs and pages/invoices/recovery.rs (MVP-007, MVP-SC-004).
- [ ] T018 [US4] Add optional project context in crates/horae/src/route.rs and pages/new_project.rs, preserving all drafts and active-client finalization (MVP-007).
- [ ] T019 [US4] Add optional invoice context in crates/horae/src/pages/invoices.rs and pages/invoices/preparation.rs under unchanged RecoveryGate (MVP-007).
- [ ] T020 [US4] Wire contextual and existing-row links in crates/horae/src/pages/clients/detail.rs; retain client filter for Projects and existing navigation guards (MVP-007).

## Phase 7: Cross-cutting verification and delivery

- [ ] T021 Add/run durable browser acceptance for clients in nix/checks/ covering failure/retry, keyboard/focus, reload, negative/direct access and historical-row preservation; inspect response payloads (MVP-003, MVP-004, MVP-011, MVP-SC-001 through MVP-SC-004).
- [ ] T022 Capture/inspect list/detail/form at required viewport/theme/text matrix and rerun Projects/import/Invoices regressions; record evidence in specs/012-clients-design/increments/mvp/progress.md (MVP-008, MVP-009, MVP-011, MVP-SC-005).
- [ ] T023 Run core/server/web/clippy tests, regenerate .sqlx/ with server/all-targets and run Nix formatting/flake checks; record exact outcomes in specs/012-clients-design/increments/mvp/progress.md (MVP-011, MVP-012).
- [ ] T024 Perform adversarial code/cross-feature review and correct all critical/high findings; record independent versus self-review and rerun analysis/tests in specs/012-clients-design/increments/mvp/progress.md (MVP-012).
- [ ] T025 Publish scoped unsigned commits/PR(s), investigate CI until green without weakening checks; record PRs, evidence, deferred parent requirements and final completion audit in specs/012-clients-design/increments/mvp/progress.md (MVP-010, MVP-012, MVP-SC-006).

## Dependencies and execution

Setup → foundation → US1 → US2 → US3 → US4 → verification/delivery.
Tests precede corresponding implementation; keep failures and their resolution
visible. No task authorizes touching real data, queued branches or merging.

Parallel opportunities (commands only, not automatic agent delegation): after
server projection tests, US1 filter unit checks and UI reference inspection can
run independently; US2 payload and visual checks can run independently after its
read contracts exist; US3 core tests and disposable DB tests are independent;
US4 route tests and browser capture preparation are independent. Shared-file
edits remain sequential. All stories must pass, not just the first checkpoint.
