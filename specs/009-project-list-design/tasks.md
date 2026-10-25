# Tasks

- [x] T001 Reconcile the refreshed handoff with supported project-list behavior.
- [x] T002 Add browser assertions for the scoped design and empty states.
- [x] T003 Align the existing page and token-based structural styles.
- [x] T004 Verify server/WASM builds, formatting and isolated browser checks.
- [x] T005 Record evidence and deviations for a scoped review PR.
- [x] T006 Correct the navigation fixture dependencies and validate all targets.
- [x] T007 Refine Projects typography and content-sized columns using utilities.
- [x] T008 Verify shared defaults and cross-page visual/interaction regressions.
- [x] T009 Correct empty-state geometry and table-header tokens without changing shared defaults.
- [x] T010 Add reproducible shared-style comparisons and wire isolated browser checks into Nix CI.

## Client-detail navigation (2026-10-25)

One P1 story (US1, spec "Client-detail navigation"). No setup or foundational
phase: the route, destination authorization and test harnesses already exist.
Tests come first and must fail before T014.

- [ ] T011 [US1] Add a failing SSR test with a legacy overview of two same-named clients whose projects interleave by name plus a case-only name variant and a row without client details, asserting one `/clients/{id}` link per client band, each band holding only its own projects, "Unknown client" without a link, and no client-catalog read or extra overview request, in crates/horae/tests/detail_navigation/overview.rs
- [ ] T012 [US1] Extend the same file to assert the scoped-policy fixture renders its client label without a `/clients/` link, and that activating a legacy band link opens the Client Detail route and Back returns to Projects, in crates/horae/tests/detail_navigation/overview.rs
- [ ] T013 [P] [US1] Add a failing browser check that signs in, sets the disposable actor to manager and then member (with a project assignment for the member), focuses the client band link with the keyboard, presses Enter and lands on that client's detail heading, restoring the actor afterwards, in crates/horae/tests/browser/client-context.cjs
- [ ] T014 [US1] Group bands by `client_id` with a name/`client_id`/project-name sort, render a `Link` to `Route::ClientDetail` for known clients when `!canonical_permissions`, and plain labels in the existing band color otherwise, in crates/horae/src/pages/projects.rs
- [ ] T015 [US1] Run T011–T013 green, then `detail_navigation`, the legacy and foreign-organization client detail server tests in crates/horae/src/server_fns/clients/page_tests.rs, and the projects-design, project-bulk-actions and client-context browser suites via crates/horae/tests/browser/run-design-checks.sh
- [ ] T016 Run server Clippy (all targets), WASM check and `nix fmt -- --ci`, then record commands, exits and limits in specs/009-project-list-design/acceptance.md

Dependencies: T011 → T012 → T014; T013 parallel with T011/T012; T015 after
T014; T016 last.

### Delivery gates (coordinator-owned, pending until evidence exists)

- [ ] T017 Native desktop and mobile checks in light and dark themes, including keyboard navigation from a client band to Client Detail and back, recorded in specs/009-project-list-design/acceptance.md
- [ ] T018 Two independent code reviews of the branch diff, findings resolved or recorded in specs/009-project-list-design/acceptance.md
- [ ] T019 Full local `nix flake check` on the final head, result recorded in specs/009-project-list-design/acceptance.md
- [ ] T020 Mandatory CI green on the exact published head before merge, recorded in specs/009-project-list-design/acceptance.md

This slice closes one navigation gap; it does not complete the UI parity goal.
