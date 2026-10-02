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

## Iteration: real client detail and route isolation

- Replaced the UUID placeholder with persisted identity/address/tax ID and
  manager-authorized default rate; absent rates remain distinct from explicit
  zero. The invoice panel mounts only when the server returns billing authority.
- Related projects reuse existing progress-authorized reads; invoices use the
  exact-client read. Amounts retain each record's ISO currency. Missing project
  totals render an em dash, never a fabricated zero.
- Client identity, project rows, totals and invoices have separate pending/error
  states and real retry actions. Keyed detail content cancels old requests and
  discards all client-local state when the route ID changes. Clients remains
  selected in the sidebar on a detail route.
- Eight client component regressions exercise production components/router with
  controlled endpoint responses: navigation/back, pending/error identity,
  member restrictions, independent invoice/project/totals errors, unset versus
  zero, real destination links and cancellation of an old invoice request.
  The initial four tests failed against the old placeholder before implementation.
  Expanded `cargo test -p horae --features server --test detail_navigation` passed
  all 35 tests, including existing project/invoice and helper regressions.
- CSS adds only a scoped two-column structure and its 1180px stack breakpoint;
  spacing/type/color reuse the utility layer. Removed an unsupported `list-none`
  class; flex list items already suppress markers and explicit list semantics
  remain. No global CSS defaults or utility-generator changes.
- Data-honest handoff differences: “Added to Horae” describes the local creation
  timestamp, not an invented Harvest relationship date; “Invoices” lists actual
  stored rows without claiming an unimplemented recent limit. No fake contacts,
  payment terms, financial cards or inert action controls. Edit/context actions
  remain T015/T020, not omitted from the increment.
- Nix formatting passed. All three `route::tests` passed, server all-target clippy
  (`--locked -- -D warnings`) and the web/WASM check passed. These checks cover
  the detail checkpoint, not the subsequent shared-form implementation.
  Browser interaction/visual proof and independent review remain T021–T024.

Next: T013 validation and transactional-save tests, then shared form/context
integration. No new PR, merge, real-data write or migration in this iteration.

## Iteration: shared client validation foundation

- Saved the verified detail checkpoint as unsigned commit `6c08dae`.

- Added six tests first for the core profile contract: Unicode character limits,
  blank/overlong/NUL names, supported currencies, NUL billing fields, absent/zero
  and exact large rates, negative/fractional-cent/overflow rejection, and optional
  text normalization without stripping legitimate nonempty address text.

- All six failed against the unimplemented functions, then passed after adding
  validation using the existing currency set and exact `parse_cents` parser.
  Full `cargo test -p horae-core`: 121 passed; core all-target clippy with
  `--locked -- -D warnings`: passed. No dependency, float or I/O in core.

- Existing project-client creation now calls this shared core validation instead
  of duplicating the name/currency/rate rules. Its transaction/authorization and
  SQL are unchanged. The dedicated DB regression passed against the previously
  created disposable cluster at `/tmp/horae-clients-mvp-pg.pSefis`, explicitly
  selected through DATABASE_URL; the cluster was verified accepting connections.

- T013/T014 remain incomplete: transactional explicit-rate profile saves,
  concurrent financial edits, history preservation and event assertions are
  still required. The old list form is not yet replaced and this foundation is
  not claimed as a completed create/edit journey.

- `client_dialog_validates_details_and_persists_an_explicit_zero_rate`: passed,
  including persisted zero, rejected invalid name/currency/negative rate,
  rejected member creation and no extra inserted records. `nix fmt -- --ci`
  passed with zero changed files. No SQL changed, so no cache regeneration needed.

Next: add failing profile-save persistence/concurrency tests and implement their
transaction before T015 shared form. No new Harvest question required repeating
the documented evidence, and no browser verification is claimed for this step.

## Iteration: atomic profile saves and financial concurrency

- Added typed profile, explicit keep/replace/clear rate intent and original
  currency/rate snapshot contracts, rejecting unknown serialized fields.
  New create/update server functions require manager authority from the session.
- Each save locks and rechecks the active same-org manager. Updates then lock the
  client and compare its current currency/rate with the editor's original values.
  Stale financial state yields conflict; currency changes cannot reinterpret a
  saved zero or positive rate through implicit keep. Explicit replacement/clear
  is atomic with identity/address changes. No new schema, dependency or policy.
- Seven initial DB tests failed against unimplemented save helpers, then passed.
  Added four regression cases for unset-rate currency changes, independently
  stale currency, a concurrent financial edit and concurrent actor demotion.
  Concurrency tests wait for actual PostgreSQL lock dependencies, not sleeps.
- Persistence checks cover normalized optional fields, exact absent/zero/positive
  rates, UUID v7, invalid writes, active/org/role guards, missing/foreign clients,
  unchanged inactive status, no-op row versions and the no-update-event signal.
  Full JSON snapshots of linked project/invoice rows stay identical after a
  profile update, including historical currency, totals and generated terms.
  Event delivery itself is not simulated: the wrapper dispatches after commit
  and only on the tested changed flag; HTTP/browser gates remain outstanding.
- The legacy create signature delegates to the shared save and emits one event.
  Legacy update validates at the public boundary and retains its existing
  locked-row currency guard; picker signatures remain unchanged.
- First fixture compilation caught that invoice `terms_days` is generated, not
  insertable. Corrected the synthetic fixture to derive 21 days from its dates.
  This compilation error is not counted as the tests-first failure above.
- In Nix against the explicit disposable socket URL: `cargo test -p horae --features server --bin horae clients` passed 39 tests; the dedicated
  project-client creation regression passed. Server all-target clippy with
  `--locked -- -D warnings` passed. Web/WASM compilation passed with four dead-code
  warnings for the new profile contracts, which T015 must consume; no suppression
  or warning-free web claim. The shared modal is not wired yet.
- SQLx prepare and prepare-check (`--workspace -- --features server --all-targets`)
  passed after forcing the three SQL-bearing target roots before each command.
  Audited cache diff: eight added entries for new saves/tests, one removed entry
  for the superseded legacy create query; unrelated cached queries remain intact.

Next: T015 shared modal form from list
and detail with pending/error/uncertain-create recovery. Backend save tests do
not prove the user journey, browser acceptance, independent review or PR/CI gates.

Form integration notes from existing sources: both handoff dialogs use the same
name, currency/rate row and billing-address composition. Keep existing Tax ID
despite its omission from the prototype editor. Reuse the native Modal's pending
dismissal guard and focus restoration; keep it mounted when closing. Existing
`project-edit-navigation.js` already recognizes `data-editor-state`; extend only
its client-facing copy if reusing it, preserving project/invoice recovery rules.
Do not copy the prototype's cascading archive handler or hardcoded defaults.

## Iteration: shared client editor and browser acceptance

- Replaced the inline list editor with a shared native modal used from list and
  detail. The editor loads fresh authorized billing data, retains invalid/failed
  input, distinguishes zero from an unset rate, and requires explicit rate intent
  when changing its currency. Closing keeps the modal mounted for focus return.
- Pending saves disable fields, submit and dismissal. Ambiguous creation errors
  require inspecting a refreshed list in another tab before an explicit new
  attempt; the original entered values remain available. Existing navigation
  guards now use client-specific pending/dirty copy without changing the shared
  history machinery. Modal width is scoped to the client dialog using a token.
- Form-state tests ran tests-first (five failed against placeholders, then
  passed). The navigation/state suite passed all 41 tests; server all-target
  clippy and web/WASM checks passed without warnings in the form implementation
  checkpoint. Those checks do not substitute for browser acceptance.
- Added a browser suite for create/reload/edit/cancel, explicit currency/rate
  intent, duplicate/pending guards, interrupted requests and a committed creation
  whose acknowledgement is lost. The suite verifies list inspection without
  losing fields or repeating the committed write. It asserts the test-only
  localhost port and disposable PostgreSQL socket before performing any writes.
- Updated the existing action-error regression's client selectors/endpoints and
  lifecycle filter setup for the shared modal; its assertions are retained.
- Built with `SQLX_OFFLINE=true dx build --platform web --fullstack true --force-sequential --locked` in Nix. `CARGO_TARGET_DIR` points to this worktree's
  `target/`; only Rust dependency cache directories are linked to the existing
  root target. Dioxus output stays in this worktree and does not overwrite the
  root development server's public assets. Both client and server completed
  successfully; the first isolated build took 965 seconds, including server
  dependency compilation. Subsequent iterations can reuse this build output.
- `run-design-checks.sh clients action-errors`: passed in headless Chromium,
  including real persisted creation followed by an intercepted/lost response.
  A second run, `run-design-checks.sh clients action-errors new-project-navigation`,
  also passed after adding the import entry-point check. Both runs used the Nix
  environment, built worktree server and existing Nix Playwright packages.
- The browser checks prove create/reload/edit/cancel, focus return/error focus,
  zero-rate persistence and explicit denomination changes, dirty/pending exit
  guards, blocked duplicate submissions/dismissal and uncertain-save recovery.
  Full before/after JSON snapshots of every project and invoice remain identical
  across client deactivation/reactivation. The existing admin Import link opens
  Importers; no import or real data mutation was initiated.
- Existing action-error regressions passed for client activation, project archive
  and assignment removal. The project navigation regression passed pending-save,
  lost-acknowledgement, back/forward, reload, cancel, draft-discard and finalization
  cases. This checks shared guard behavior, not the outstanding full visual matrix.
- Test clusters `/tmp/horae-browser.BUae4A` and `/tmp/horae-browser.Lmakdd` were
  stopped by the runner after success; their server/PostgreSQL logs are retained.
  Added `clients` to the default browser runner so CI executes the suite too.
- The first format check corrected only Markdown wrapping in this log and
  reported that change; the subsequent `nix fmt -- --ci` check passed.

T015/T016 are complete. Next: T017 tests-first for contextual project/invoice
navigation, retaining existing drafts and recovery precedence. The full role/
payload browser coverage, visual matrix, cross-feature final checks, adversarial
review and PR/CI gates remain outstanding. No new Harvest investigation was
needed for these already documented contracts; no additional Harvest behavior is
claimed as observed. This checkpoint does not complete feature 012 or the goal.
