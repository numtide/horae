# Implementation Plan: Harvest OAuth Client Settings

**Branch**: `feat/harvest-oauth-client-settings` | **Date**: 2026-11-03 | **Spec**: [spec.md](./spec.md)

## Summary

Store one Harvest OAuth client per organization, with the secret sealed by the existing token AEAD and the deployment key. Split today's all-or-nothing environment `HarvestConfig` into the deployment settings read at startup and an effective `HarvestConfig` resolved per use from the environment first, then the stored row. Every reader of `state.harvest` (connect, callback, status, enqueue, worker) resolves it for its organization, so a save applies without a restart. Saves and removals reuse the existing connection-change transaction: import lock, administrator recheck, connection gate, refusal while imports are active, `connection_revision` bump. A client ID change or removal also deletes the tokens, as Disconnect does. The importer gains the design's "Harvest OAuth app" view.

## Technical Context

- Rust 2024, Dioxus 0.7 fullstack (SSR + WASM), PostgreSQL via compile-time-checked sqlx, Tokio. No new dependencies.
- One additive migration: `harvest_oauth_clients` (UUIDv7 PK, unique `org_id` FK, client ID, sealed secret, redirect URL, `updated_by`, timestamps with the shared `set_updated_at` trigger).
- Reuse `credentials::{encrypt, decrypt, authorize_change}`, `account_switch::gate`, `lock_import`/`release_import`, the existing audit log table and `require_admin`.
- Validation and the "does this save disconnect?" rule are pure and live in `horae-core` beside `ConnectionStatus`.
- Tests: core unit tests for validation; `#[sqlx::test]` integration tests for precedence, sealing, no key, unreadable secret, disconnect-on-change, active-import refusal, OAuth revision fencing and change without restart; the existing real-session HTTP matrix for administrator-only access and secret-free responses. The importer page's rendered-UI tests (`crates/horae/tests/import_jobs_ui.rs`) cover the OAuth app view; the browser design checks do not include the importer, so it is also checked once in a live browser.
- Scale: one row per organization, read once per connect, status, enqueue or job execution. One primary-key query; no cache.

## Constitution Check

*Pre-research*: pass.

- I Exactness: no time or money touched.
- II Domain Purity: validation and the disconnect rule are pure and live in `horae-core`. Sealing stays in the server crate beside the token sealing it reuses.
- III Single Datastore: PostgreSQL table, UUIDv7 key, `org_id` FK.
- IV Mutations Through Server Functions: save and remove are admin-checked `#[server]` functions. The read is a server function too. No client-side fetches. The local CLI is not extended.
- V Reproducible Builds: `.sqlx` regenerated; `nix fmt`, clippy and the relevant `nix flake check` attributes run before the PR.

*Post-design*: pass, no exceptions. Nothing goes in Complexity Tracking.

## Project Structure

```text
specs/018-harvest-oauth-client-settings/
├── spec.md  plan.md  research.md  data-model.md  quickstart.md  tasks.md
└── contracts/client-settings.md

crates/horae/migrations/0051_harvest_oauth_clients.sql
crates/core/src/importers/harvest/types.rs           # HarvestClientSettings, ClientSource, validation, disconnect rule
crates/horae/src/config.rs                           # HarvestDeployment (env) replaces Option<HarvestConfig> in AppConfig
crates/horae/src/state.rs                            # AppState.harvest: HarvestDeployment
crates/horae/src/importers/harvest/client_settings.rs  # resolve, view, save, remove (+ tests)
crates/horae/src/importers/harvest.rs                # mod declaration; callback resolves per org
crates/horae/src/jobs.rs                             # worker resolves per job org
crates/horae/src/server_fns/importers.rs             # settings read/save/remove; existing fns resolve per org
crates/horae/src/pages/importers.rs                  # OAuth app view entry points
crates/horae/src/pages/importers/client_settings.rs  # HarvestOAuthApp component
crates/horae/assets/css/horae.css                    # only if a structural rule is missing
README.md, .env.example                              # document the page option
```

**Structure Decision**: existing single app crate plus `horae-core`. New submodules use the `foo.rs` + `foo/` layout already used by `importers/harvest`.

## Phases

1. Core types and validation (RED unit tests first).
1. Migration, `HarvestDeployment`, `resolve`, call-site migration. Existing tests stay green with the environment client.
1. `save`/`remove` transaction and server functions (RED integration tests first), audit row, HTTP matrix additions.
1. Importer OAuth app view, matching `design/project/app/10_Importers.dc.html`.
1. `.sqlx`, docs, clippy, fmt, WASM check, relevant nix checks, live browser check, PR.

## Workflow Notes

`setup-plan.sh` reports a branch name derived from the directory; the Git branch is `feat/harvest-oauth-client-settings`. No extension hooks exist. The agent-context updater is not shipped in this repo.

## Complexity Tracking

None.
