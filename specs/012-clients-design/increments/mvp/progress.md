# Clients MVP delivery record

## Objective and completion boundary

Implement the user-authorized [increment](spec.md), not merely its documentation.
Completion requires published PRs, green CI, actual browser/permission/regression
evidence and no critical/high review findings. Parent feature 012 stays incomplete.

## 2026-10-02 — Reuse and specification reconciliation

- Goal attachment read in full. Root worktree remains untouched, including its
  pre-existing `.playwright-mcp/` directory. Read AGENTS.md and constitution 1.0.0.
- GitHub check: #209 is OPEN at `f5bacfbd13ae4268cd1c9d8e69b4f9200a520fbc`, first in
  the merge queue. Created `.worktrees/clients-mvp` / `feat/clients-mvp` at that
  exact revision. No queued branch, merge request or real data was changed.
- Read specify/clarify/plan/tasks instructions. No `.specify/extensions.yml` is
  present. Resolved the active spec template through `resolve_template`; retained
  the parent specification and created an explicitly nested increment package.
  The worktree-local feature pointer selects this package, not a new numbered feature.
- Parent checklist remains 11/16: full contact/lifecycle/permission gates are
  not waived. Clarifications above come from the explicit new goal, not silence.
- Read both Clients handoffs in full and existing client UI/server paths. The
  detail is still a UUID placeholder; existing catalogue reads and manager edits
  must not become authority for invoice or project-private data.
- Refreshed official client/currency documentation. Existing 2026-09-30 browser
  evidence is retained. Revalidated the existing browser process and MCP tabs;
  navigation to Harvest Clients is read-only. No new connection or fixture writes.
- Impeccable context launcher failed because its engine cache is unavailable;
  announced fallback and read existing DESIGN.md directly. No dependency install
  or new visual identity is authorized. Design authority remains the handoff.
- Spec Kit phase-0 read-only research delegated for current access and editor
  context/recovery contracts. No application implementation or test pass claimed.

Next: finish read-only Harvest inspection, record reference and scope decisions,
complete clarify/checklist, execute setup-plan and setup-tasks, analyze the
increment, then implement with failing tests and disposable database fixtures.

## 2026-10-02 — Contracts and executable tasks

- Re-read goal attachment and revalidated worktree/PR #209 (still open, same head).
  The intervening prompt rewrite was not implementation progress; resumed the
  next safe task without changing the objective or touching queued branches.
- Revalidated live MCP session 44272; read-only new-client navigation and DOM
  inspection confirm visible fields and required name. No form submitted. List
  snapshot has no rows plus console errors, so it is not evidence of an empty
  account or verified filters. Recorded limitations and current official sources.
- Completed read-only contract research; corrected the spec's ambiguous blanket
  inactive-client statement using actual existing invoice/project policies.
- Executed setup-plan and setup-tasks with the nested increment directory. Their
  logical branch output is `mvp`; actual Git branch is `feat/clients-mvp`.
  Replaced only the generated plan template with the concrete plan. No existing
  user document was deleted. No extension hooks or agent-context script exist.
- Added research, data-model, UI/server contracts, quickstart, 10/10 specification
  readiness checks and 25 traceable implementation/verification tasks. This does
  not mark parent checklist gates or any implementation/test as complete.

Next: run read-only analyze, then disposable database setup and failing client
projection tests. Remaining UI, browser, adversarial review, PR and CI work stays
required; no completion claim.

## 2026-10-02 — Analysis and isolated test foundation

Read-only analyze ran after tasks generation using check-prerequisites with
`--require-tasks --include-tasks`. No artifact edits during analysis; no open
findings. Coverage: 18/18 requirements/success criteria, 25 tasks, no unmapped
tasks, ambiguity/duplication/critical counts zero. Constitution gates pass at
architecture level, not execution level.

| Requirement | Tasks |
| --- | --- |
| MVP-001 | T006–T009 |
| MVP-002 | T010–T012 |
| MVP-003 | T006–T007, T010–T011, T021 |
| MVP-004 | T013–T015, T021 |
| MVP-005 | T013–T014 |
| MVP-006 | T009, T013, T016 |
| MVP-007 | T017–T020 |
| MVP-008 | T009, T012, T015, T022 |
| MVP-009 | T015, T022 |
| MVP-010 | T001, T025 |
| MVP-011 | T004–T005, T021–T023 |
| MVP-012 | T003, T023–T025 |
| MVP-SC-001 | T008, T021 |
| MVP-SC-002 | T010, T021 |
| MVP-SC-003 | T013–T015, T021 |
| MVP-SC-004 | T017–T020, T021 |
| MVP-SC-005 | T022 |
| MVP-SC-006 | T024–T025 |

Implementation readiness checklist: 10/10, zero incomplete. Read implement/Rust
skills; ignore patterns already cover target, scratch and worktrees. Existing
`tests/browser/run-design-checks.sh` creates isolated fixtures, unsets mail and
rejects occupied ports. Existing `test_seed::seed` supplies synthetic tenants.

Executed `nix develop --command bash -c ...` to initialize only
`/tmp/horae-clients-mvp-pg.pSefis` via `mktemp -d`, `initdb -U postgres`,
`pg_ctl` with that Unix socket and empty listen_addresses, and `createdb` for
`horae_clients_mvp`. Applied migrations 1–41 only there with sqlx. Test URL:
`postgres://postgres@localhost/horae_clients_mvp?host=/tmp/horae-clients-mvp-pg.pSefis`.
No existing database was migrated or seeded.

Added first projection tests before implementation. Next: demonstrate failing
tests, implement the narrow read projection, then validate authorization behavior.

## 2026-10-02 — First implementation: authorized client list

- Added `ClientSummary` without rates/invoices and a single-query, session-gated
  `list_client_summaries`. Project counts/currencies use `project_read_access`;
  organization, active actor and project/client organization are enforced in SQL.
  Existing picker DTO/endpoint remains unchanged.
- TDD: projection tests first failed to compile because the function was absent.
  First implementation run passed 3/4; fourth failed on an incomplete synthetic
  project-settings fixture (missing required rate_mode), not permission behavior.
  Corrected fixture; all 17 client server tests passed. Hidden-project test also
  includes historical time, proving history alone grants no progress/counts.
- Added filter tests before implementation (missing symbols confirmed red), then
  case-insensitive name, preferred/visible-project currency and lifecycle matching.
  Status counts share name/currency predicates; clearing a query keeps other filters.
- Client list now uses real detail links, active/inactive/all status, authorized
  currency options, visible project counts, loading/error/retry and distinct empty
  versus no-match states. Retains single-client edit/activation; adds existing
  admin importer destination without starting any import. No bulk/fake controls.
- Reused Menu, badge, input/table/empty-state semantics and token utilities. No
  shared component, CSS, generator or dependency modified; removed the existing
  inline spacing style in the touched client rows. Both target handoffs and relevant
  imported design components were read; no prototype logic used as policy.
- Latest command in Nix with the explicit disposable URL and shared build cache:
  `cargo test -p horae --features server --bin horae clients`: **22 passed** (17
  client server, 4 filter, 1 existing invoice regression), zero failed/ignored.
- The initial all-target test compilation reported unused `ClientSummary` in the
  isolated detail-navigation harness. Resolve through upcoming client-detail
  coverage; do not silence global warnings. No all-target clippy pass claimed.

T006–T009 mark implemented code/unit boundaries only. Browser acceptance remains
T021/T022. Detail is still the existing placeholder; shared form/default-rate
editing and context navigation are not yet delivered. Next: cache/format this
checkpoint, then T010–T012 client-detail access tests and real detail.

- Checkpoint SQLx prepare completed with `--workspace -- --features server --all-targets`: six new cache entries, no existing entries deleted. `nix fmt`
  completed (seven files formatted); `git diff --check` clean.
- Restored the repository's original `.specify/feature.json` after planning so
  the delivery does not change a global active-feature pointer. Further helpers
  must explicitly set `SPECIFY_FEATURE_DIRECTORY=specs/012-clients-design/increments/mvp`.

## 2026-10-02 — Detail read contracts and authorization

- The preceding conversational turn only rewrote the goal prompt (no implementation
  progress). Revalidated the objective attachment, worktree, constitution, tasks
  and Spec Kit readiness before continuing. The prior web-check process handle
  no longer existed; a fresh Nix web check completed successfully, exit 0.
- Read-only GitHub verification now reports #209 merged at `2026-10-02T00:42:45Z`,
  head `f5bacfbd13ae4268cd1c9d8e69b4f9200a520fbc`. No queue/PR mutation was made.
  The unpublished increment branch still starts at that head; reconcile its
  eventual PR base with current master before publication.
- Reused the existing disposable PostgreSQL socket; `pg_isready` confirmed it
  live. No new database cluster, migration or real-data write this iteration.
- Added detail tests before code; Nix compilation failed with the expected missing
  detail/invoice helper symbols (exit 101). Implemented session-org-scoped exact
  client reads and optional manager/admin billing. SQL suppresses member rates;
  serialization omits the entire restricted billing section. Authorized unset,
  zero and positive rates stay distinct. Inactive clients remain readable.
- Added manager-gated client invoice reads, sharing the existing invoice query
  with an optional client filter rather than creating a separate billing flow.
  Missing/foreign clients are not-found; empty authorized results differ from
  access denial. Existing invoice-list status filtering remains supported.
- First green run: `cargo test -p horae --features server --bin horae clients`
  passed 27 tests. Expanded those tests to check admin rates, inactive invoice
  viewers and unfiltered/status-filtered invoice regression; added a Projects
  privacy regression for exact client scope, same-org unrelated projects,
  inactive projects/users and rate redaction. Final expanded run pending below.
- Spec Kit prerequisite discovery writes `.specify/feature.json` even with the
  environment override. Restored its original 011 pointer again; do not commit
  this workflow side effect.

Next: finish expanded tests/cache/format verification, then T012 real detail UI
and detail-navigation regression coverage. The UUID placeholder, shared form,
context navigation, browser acceptance, independent review and PR/CI gates are
still pending. This is not a completed Clients increment.

Expanded verification completed in Nix with the same explicit disposable URL:

- `cargo test -p horae --features server --bin horae clients`: 27 passed.
- `cargo test -p horae --features server --bin horae privacy_tests`: 9 passed,
  including the added client-project filter regression and existing export guards.
- `cargo test -p horae --features server --bin horae server_fns::invoices`:
  69 passed, including existing invoice history/imported-rate/recovery/concurrency
  coverage. These suite counts overlap on one existing invoice test; they are not
  105 distinct tests.
- `nix fmt` completed successfully; `git diff --check` clean.

T010/T011 cover server reads and persistence-level access assertions. They do not
prove browser/direct-HTTP behavior; those gates remain in T021/T022. No new policy,
CSS, shared component, schema or dependency changed in this iteration.

- Web target check and server-bin clippy (`-- -D warnings`) passed after the new
  reads. Web currently warns about detail DTOs not yet consumed by the placeholder;
  the isolated detail-navigation harness also reports unused client projections.
  T012 must consume/test them; no warning suppression or all-target lint pass claimed.
- Cache audit caught an incremental-build problem: prepare returned success but
  omitted 92 still-used integration/CLI query entries, in addition to replacing
  the changed invoice query. Touching only integration regenerated that target
  but omitted the cached app target. The installed CLI reports 0.9.0; its
  [prepare source](https://github.com/launchbadge/sqlx/blob/v0.9.0/sqlx-cli/src/prepare.rs)
  confirms regeneration relies on recompilation/mtime invalidation. Force all
  SQL-bearing target roots together (`src/main.rs`, `tests/integration.rs`,
  `tests/cli_restart.rs` under crates/horae) before prepare/check. Source contents
  are unchanged by this mtime update. A broad package-clean dry run would remove
  73.1 GiB of shared build artifacts, so no clean/delete was performed.
- Final regeneration and `cargo sqlx prepare --workspace --check -- --features server --all-targets` both passed after forcing the three target roots before
  each command. Audited final diff: 13 added cache files and exactly one removed
  entry, the superseded invoice-list query. All unrelated entries restored by
  regeneration; no source-content changes to main/integration/CLI files.

Next action remains T012. Save this implementation checkpoint on the isolated
branch, then replace the UUID detail placeholder and cover route changes before
moving to the shared form. No new PR is published yet; no merge performed.
