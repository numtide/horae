# Tasks: Harvest OAuth Client Settings

**Input**: [plan.md](./plan.md), [spec.md](./spec.md), [data-model.md](./data-model.md), [contracts/client-settings.md](./contracts/client-settings.md), [research.md](./research.md)

**Tests**: requested (TDD). Each test task is written first and must fail before its implementation task.

## Phase 1: Setup

- [X] T001 Start a disposable PostgreSQL with `CREATEDB` on a free port under `.data/` of this worktree (never `127.0.0.1:5432`) and export `DATABASE_URL` for the session

## Phase 2: Foundational

- [X] T002 [P] RED core unit tests for `validate_save` (client ID, secret, redirect URL rules) and `save_disconnects` in `crates/core/src/importers/harvest/types.rs`
- [X] T003 Add `HarvestClientSettings`, `ClientSource`, `SaveClient`, `validate_save`, `save_disconnects` to `crates/core/src/importers/harvest/types.rs` (T002 green)
- [X] T004 [P] Add migration `crates/horae/migrations/0051_harvest_oauth_clients.sql` per data-model (table, unique org FK, length check, `set_updated_at` trigger)
- [X] T005 Replace `AppConfig.harvest: Option<HarvestConfig>` with `HarvestDeployment` (env pair, redirect URL, key) in `crates/horae/src/config.rs`, `crates/horae/src/state.rs`, `crates/horae/src/main.rs`; update `crates/horae/src/config/tests/environment.rs`
- [X] T006 RED integration tests for `resolve`: env pair wins over stored row; partial env pair ignored; stored row used when env pair absent; env redirect URL wins; no key → `None`; unreadable stored secret → `None`; in `crates/horae/src/importers/harvest/client_settings/tests.rs`
- [X] T007 Implement `client_settings::resolve` and `client_settings::view` in `crates/horae/src/importers/harvest/client_settings.rs` (T006 green)
- [X] T008 Resolve the effective client per organization in `crates/horae/src/server_fns/importers.rs` (connect start, status, API start), `crates/horae/src/importers/harvest.rs` (`complete_connect`) and `crates/horae/src/jobs.rs` (worker); adjust existing tests that construct state (`authorization_tests.rs`, `authorization_tests/cli.rs`)

**Checkpoint**: existing importer tests pass with the environment client; behaviour unchanged.

## Phase 3: User Story 1 — Configure Harvest from the importer (P1) 🎯 MVP

**Independent test**: with only the key set, save a client and connect without a restart.

- [X] T009 [US1] RED tests in `client_settings/tests.rs`: save seals the secret (stored bytes ≠ secret, decrypt with key = secret, other key fails); `view` never carries the secret; save then `resolve` returns the new client without rebuilding state; save writes one `audit_log` row without the secret
- [X] T010 [US1] Implement `client_settings::save` (import lock, `authorize_change`, gate, active-import refusal, upsert, revision bump, audit, tracing) in `crates/horae/src/importers/harvest/client_settings.rs`
- [X] T011 [US1] Add `harvest_client_settings` and `save_harvest_client` server functions with named codes in `crates/horae/src/server_fns/importers.rs`
- [X] T012 [US1] Add the OAuth app view component in `crates/horae/src/pages/importers/client_settings.rs` (redirect URI + Copy, client ID, write-only secret with Replace, Save credentials, status pill, environment footer), styled with utilities and tokens per `design/project/app/10_Importers.dc.html`
- [X] T013 [US1] Wire entry points in `crates/horae/src/pages/importers.rs`: "Set up OAuth app" when not configured, "OAuth app credentials" in Manage connection, back link; refresh connection status after a save

## Phase 4: User Story 2 — Environment keeps working and wins (P1)

**Independent test**: env pair plus a stored row: env used; page locked.

- [X] T014 [US2] RED tests: save/remove refused while the env pair is set; `view` reports `Environment` and `stored_ignored`; in `client_settings/tests.rs`
- [X] T015 [US2] Enforce the environment lock in `save`/`remove` and render the locked panel in `crates/horae/src/pages/importers/client_settings.rs`
- [X] T016 [US2] Extend the HTTP matrix in `crates/horae/src/server_fns/importers/authorization_tests.rs`: member and manager get `FORBIDDEN` on all three endpoints; signed-out callers are refused; admin settings response contains no environment secret; save is `CONFLICT` under the environment client; routes registered at the contract paths

## Phase 5: User Story 3 — Replace or remove safely (P2)

**Independent test**: connected; secret-only change keeps tokens; ID change and removal disconnect; active import refuses.

- [X] T017 [US3] RED tests in `client_settings/tests.rs`: secret-only replace keeps credentials; new client ID without a secret refused; new client ID deletes credentials and keeps binding and generation; removal deletes the row and credentials; queued job and held import lock refuse save and remove with no change; an OAuth attempt version from before a save is stale for `store_for_attempt`
- [X] T018 [US3] Implement `client_settings::remove` and the disconnect-on-change path; add `remove_harvest_client` server function in `crates/horae/src/server_fns/importers.rs`
- [X] T019 [US3] Add the disconnect warning and Remove action with confirmation to `crates/horae/src/pages/importers/client_settings.rs`

## Phase 6: User Story 4 — No encryption key (P3)

- [X] T020 [US4] RED test: save refused without a key; `view.encryption_key_set == false`; in `client_settings/tests.rs`; implement refusal and the explanatory panel state

## Phase 7: Polish

- [X] T021 Regenerate `.sqlx/` (touch + `cargo sqlx prepare --workspace -- --features server --all-targets`) and verify with an offline test build
- [X] T022 [P] Document the page option in `README.md` and `.env.example`
- [X] T023 Run `cargo clippy -p horae --features server --all-targets -- -D warnings`, `cargo check -p horae --features web --target wasm32-unknown-unknown`, `nix fmt`, full `cargo test -p horae --features server` and `cargo test -p horae-core`
- [X] T024 Live browser check of the OAuth app view (Playwright MCP's own tab) against a local server on the disposable database
- [ ] T025 Run relevant nix checks (`clippy`, `treefmt`, `sqlx-prepare`), push, open a PR against `master`, stop the disposable PostgreSQL

## Dependencies

- Phase 2 blocks all stories. US1 → US2 (lock reuses save) → US3 (remove) → US4. T012/T013 depend on T011.
- Parallel: T002 ∥ T004; T022 any time after T013.

## Implementation Strategy

MVP is Phases 1–3: a page-managed client that connects without a restart. US2–US4 harden precedence, safety and the no-key state; all ship in one PR.
