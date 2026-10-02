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

## Iteration: contextual workflow entry points

- Resumed the existing worktree at `9a08466`; the intervening prompt-rewrite turn
  did not advance implementation. Prior test session handles were missing, so
  reran the otherwise unobserved checks rather than treating them as successful.

- Tests-first evidence: four route/draft tests failed against absent contextual
  routes and draft-prefill placeholders. Two invoice-context tests subsequently
  failed against their placeholder. The new detail-link authorization test failed
  because the contextual links were absent.

- Chromium `invoice-preparation` failed on the previous built artifact precisely
  at the new contextual-recovery assertion: the route rendered 404 instead of
  the stored invoice request. The disposable cluster
  `/tmp/horae-browser.ucLKRK` was stopped by the runner. This is expected RED
  evidence, not a passing recovery check.

- Added separate `/projects/new/client/:client`, `/projects/client/:client` and
  `/invoices/new/client/:client` entries delegating to existing screens. Bare
  routes stay unchanged. Context uses a string so malformed links reach draft/
  recovery precedence before validation; keyed children reset per-client state.

- Project initialization restores any saved draft first, including an empty
  client. Without a draft, it resolves the exact authorized active client and
  leaves currency inherited. The initial prefill remains dirty until normal
  autosave acknowledges it; it is not falsely treated as persisted.

- Invoice context is applied once only after RecoveryGate is ready, validates an
  exact active-picker identity, and never selects the first client on failure.
  Existing recovery storage, payloads and request IDs are unchanged.

- Detail links retain client identity and current role boundaries. View in
  Projects opens the existing filtered list after an authorized client lookup.
  New project/invoice links are limited to active clients with manager billing
  authority; this does not change historical billing server policy.

- Nix checks passed: all 44 `detail_navigation` tests, five production route
  tests and seven project draft-state tests. No database/migration or SQL macro
  changes in this iteration.

- Added `client-context` Chromium coverage for real filtering, prefill, unchanged
  saved/empty-client drafts, invalid/missing client links and full before/after
  project/invoice rows. Added it to the default isolated browser runner.

- The isolated fullstack build completed successfully in 75 seconds using the
  cached dependencies. Both WASM and server output remain under this worktree.

- `run-design-checks.sh client-context invoice-preparation new-project-navigation clients`
  passed in Chromium against `/tmp/horae-browser.bsM3yA`; the runner stopped the
  temporary cluster afterward. Contextual invoice links for another client and
  a malformed client both preserve the exact stored recovery request. Recovery
  still handles lost acknowledgements and storage-clear failures without
  creating another invoice. Context navigation leaves project/invoice snapshots
  unchanged, preserves the entire saved draft row, and permits subsequent
  intentional client selection in the invoice form.

- Existing project pending/dirty navigation, identical retry, Cancel/discard and
  finalization regressions passed. Existing client form recovery, historical-row
  preservation and import entry-point checks passed as well.

- `cargo clippy -p horae --features server --all-targets -- -D warnings` passed.
  `nix fmt -- --ci` passed with zero changes; `git diff --check` passed.

T017–T020 are complete. Next: T021 role/direct-access/payload and failure/retry
browser coverage, including contextual loading failures. Then the bounded visual
matrix and cross-feature regressions, full Nix gates, adversarial review and
PR/CI delivery remain required. No new Harvest observation or merge is claimed;
the increment and the goal are not yet complete.

## Iteration: browser authorization and failure recovery

- The previous turn made implementation progress. Its final commit operation
  timed out during automatic permission review, not during Git execution; the
  permitted single retry succeeded as `c015c7d` without a signature.
- Added `clients-access` using a real authenticated browser session and observed
  server-function URLs, not guessed endpoint names. All fixture writes are
  restricted to the runner's disposable PostgreSQL socket. The test restores the
  synthetic actor's original active/admin role on exit.
- Chromium passed the administrator, manager, member/project-lead and assigned
  member cases. Assertions cover actual JSON billing omission, project rate
  omission, visible-only counts/currencies, client-filtered projects/invoices,
  foreign/missing IDs with indistinguishable errors, forbidden direct mutations,
  contextual editor denial, inactive and anonymous sessions, and invalid direct
  profile submissions. Denied/no-op requests leave the client row unchanged.
- Added `clients-errors` for loading/error/retry, explicitly injected empty
  payload versus no matches, combined name/state/currency filtering and counts,
  independent panel failures, editor-load retry and retained failed-save input.
  Empty-list injection verifies presentation only; it does not assert that the
  seeded database is empty.
- Tests exposed a real contextual-invoice failure: a failed client catalog read
  rendered the raw transport error without an accessible alert or a retry.
  RED evidence is retained in the stopped `/tmp/horae-browser.4g31Jt` run.
  Added a scoped loading/alert/Retry clients branch to the existing invoice
  form; RecoveryGate and request storage remain unchanged. No shared helper or
  CSS behavior was changed.
- The extended filter test initially filled an SSR search control before the
  reloaded list was ready; the resulting snapshot showed an empty search and
  unfiltered counts. It now waits for its actual fixture row, then verifies the
  entered query and filtered rows before opening the status menu. Assertions
  were retained, with no sleep or blanket retry.
- After rebuilding, the failure/retry and contextual suites passed, including
  the repaired invoice catalog retry. The 44 navigation/state tests and server
  all-target clippy also passed. Invoice recovery and the existing full
  responsive regression subsequently passed too: eleven routes at five widths,
  plus mobile project editing, timesheet controls and admin/import navigation.
  The stopped passing cluster is `/tmp/horae-browser.5NiKo9`. These automated
  geometry checks do not replace the outstanding client visual matrix.
- Updated two pre-existing browser resource waits from `list_clients` to the
  actual Clients-page `list_client_summaries` call, preserving their assertions.
  Added both new suites to the default isolated browser runner.

Next: complete keyboard and the list/detail/form viewport/theme/enlarged-text
matrix. T021 remains unchecked
until its remaining keyboard acceptance is verified. Full Nix/cache checks,
adversarial review, PR publication and green CI remain required; no merge or
real-data mutation has been performed.

## Iteration: bounded visual inspection and regression fixes

- The intervening prompt rewrite made no implementation progress. Revalidated
  `1393796` and the untracked visual suite; continued the existing worktree.
- Completed the first visual inspection round across 64 captures. Added a
  [visual record](visual-review.md) with the pinned surface contract, exact
  matrix, findings and keyboard evidence.
- Added durable browser assertions for identity-column width, desktop search
  alignment, primary/detail panel balance and computed text contrast. They
  reproduced the inspected defects before edits in `/tmp/horae-browser.Zu4AvW`.
- Applied one scoped correction batch: existing search width utility, readable
  client identity token, title/count/link order, content-relative detail stacking,
  and secondary-text tokens for small labels and hints. Shared tables, forms,
  modal behavior and global colors retain their existing defaults.
- Added the matrix/keyboard suite to the default disposable browser runner.
  Rebuilding the fullstack artifact before the second confirmation round.

Next: confirm the correction batch, then independent finish/documentation
handoffs, full Nix gates, adversarial analysis and PR/CI delivery. T022 and the
goal remain incomplete; no merge or real-data mutation has occurred.

### Confirmation results

- Rebuilt both targets with `dx build --platform web --fullstack true --force-sequential --locked`, under Nix with `SQLX_OFFLINE=true` and four jobs.
- Ran `run-design-checks.sh clients-visual clients clients-errors clients-access client-context invoice-preparation responsive-layout` against a newly created
  disposable database. All seven suites passed; cluster `/tmp/horae-browser.xEKOcc`
  stopped on exit. No existing database was used.
- Opened all 64 confirmation images. The five batched corrections are visible;
  the manifest has no browser or geometry/contrast findings. Both themes and
  the complete keyboard, enlarged-text and short-screen matrix passed.
- The cross-screen regression checks passed eleven routes at five widths,
  project table/action navigation, timesheet controls and mobile admin/import.
- `cargo test -p horae-core`: 121 passed. Server all-target clippy with warnings
  denied and the WASM web check passed. Formatting normalized one new Markdown
  file; the final format gate will be repeated after recording review results.
- Attempted the design detector once; unavailable engine/cache permission is
  recorded, not a pass. Started a fresh independent finish review over the
  handoffs, implementation and all second-round captures.

T021 and T022 are complete. Next: finish/documentation handoffs and T023 full
Nix/cache checks, followed by T024 adversarial analysis and T025 PR/CI delivery.
The goal remains active; no PR or green CI is claimed yet.

## Iteration: final review and base reconciliation

- The intervening prompt rewrite was no implementation progress. Re-read the
  attached objective, AGENTS.md and constitution; revalidated the same Nix check
  process (session 26296), which remained live. No replacement run was started.
- Independent visual review returned `ship` for all 64 supplied captures and
  the scoped client UI. The independent documentation handoff confirmed an
  ordinary extension of the incumbent system and preserved DESIGN.md. See
  [finish-review.md](finish-review.md) and
  [design-system-review.md](design-system-review.md); neither certifies backend
  security or whole-application accessibility.
- Completed the author-led [adversarial review](adversarial-review.md), including
  direct authorization, serialization, financial concurrency, history, route
  cancellation, legacy callers and draft/recovery precedence. No new critical
  or high issue found. Earlier corrected findings and their regression evidence
  are explicitly retained in the report.
- Executed the Spec Kit analysis prerequisite and cross-artifact analysis again:
  18/18 requirements covered by 25 tasks, no unmapped tasks or specification
  conflicts found. Restored the helper's feature-pointer side effect. T024 is
  complete; task coverage does not establish the unfinished delivery gates.
- GitHub confirms #209 merged at `30dd25b` on 2026-10-02T00:42:45Z. The previous
  working base `f5bacfbd` is the pre-squash feature branch, not the merge commit.
  Current remote master is `386cb58`; no PR exists for `feat/clients-mvp`.
  Fetched master read-only with respect to branches. Its differences from the
  working base are specifications/context only, not production code.

Next: commit the verified visual batch and reviews unsigned, reconcile this
unpublished branch onto current master without touching queued branches, then
publish its scoped PR. The original Nix process is still running; full checks,
actual green CI and the final requirement audit remain required. No merge,
real-data mutation or successful CI result is claimed.

### Publication and full-suite regression follow-up

- Saved the visual batch and reviews, then rebased the nine unpublished
  increment commits onto `386cb58`. The comparison before/after rebase was empty
  for `crates/`, `.sqlx/` and `nix/`; no production or test behavior changed.
  The new base's specification pointer remains intact.
- Published [PR #216](https://github.com/numtide/horae/pull/216), head `c76e3bf`,
  as draft. Its initial Format check passed; Flake Check was still running in
  run `36960815197`. No merge or queue operation was performed.
- The full local browser check exposed an obsolete Clients readiness wait in
  `mobile-navigation.cjs` (`list_clients` instead of `list_client_summaries`).
  Inspection also found obsolete labels in New Project's cross-screen client
  editing regression. These were test compatibility findings, not a reason to
  omit either suite or change production behavior.
- Updated mobile readiness/button labels and adapted the project regression to
  the shared modal's actual field labels, currency select and explicit-rate
  confirmation. Retained rejection/input/persistence checks and added a rate
  assertion after reload. Address persistence is checked in the reopened editor
  because the list does not display addresses. The legacy case-insensitive
  currency guard remains covered by its database regression.
- Ran `run-design-checks.sh mobile-navigation new-project` under Nix with the
  verified fullstack artifact and a fresh disposable database: both suites
  passed completely, including draft concurrency, lost acknowledgements,
  project finalization, reports/exports and real invoice preparation/editing.
  No UI source changed, so the finish/design-system dispositions still apply.

Next: publish the regression-test correction, let the original full Nix process
finish its remaining checks, then rerun the corrected full gate and verify CI
on the updated PR head. T023 and T025 are incomplete. The initial local browser
gate is failed, not green; a successful focused rerun alone does not close it.

### Full-gate status after the correction

- Published the verified test correction as `eee87ba`. Format passed on that
  head; Flake Check is running in GitHub Actions run `36961259026`.
- The original `nix flake check --keep-going --max-jobs 1 --cores 4` finished
  with exit 1 solely for the stale mobile-navigation wait documented above.
  Its package, clippy, SQLx cache verification, Rust tests, formatting and both
  NixOS deployment checks completed successfully. No failed gate was skipped.
- The test log includes 121 core tests, 817 passing server tests (11 existing
  ignored cases, with subprocess probes reported separately), 44 detail-
  navigation tests and the remaining integration/UI test targets, all without
  failures. SQLx prepare-check used its own fresh database. OIDC and normal
  deployment VM scripts completed in about 22 and 70 seconds respectively.
- Started the corrected full gate on `eee87ba` only after that process exited:
  `nix fmt -- --ci && nix flake check --keep-going --max-jobs 1 --cores 4`.
  Formatting passed unchanged; the flake process remains active. Its browser
  derivation is `/nix/store/hw1bsynwhc6wp9d3fcg2ww7b3pkj444d-horae-browser-checks.drv`.

Next action: observe the existing corrected flake process and run `36961259026`,
investigate any actual failure, and close T023/T025 only after the complete
checks, final requirement audit and ready-for-review publication. The PR stays
draft; the goal remains active. No additional feature or merge is authorized.

## Iteration: requirement-by-requirement completion audit

- Previous iteration made implementation/delivery progress: independent reviews,
  the scoped PR and the verified cross-screen regression correction. Re-read the
  attached objective and revalidated `eee87ba`, the draft PR head and the same
  corrected Nix process before proceeding.
- Added [completion-audit.md](completion-audit.md), checking every MVP requirement,
  success criterion and process boundary against source and test coverage. The
  four journeys have functional evidence; T023/T025 and MVP-SC-006 remain open
  until the full corrected gate and published-head CI pass. Readiness and task
  coverage are explicitly not substituted for delivery evidence.
- Verified unsigned increment commits, unchanged dependency/lockfile/migration/
  design-system paths, ignored private capture artifacts, intact parent-feature
  requirements and absence of Spec Kit extension hooks.
- CI run `36961259026` remains active with Format passed. A watcher observes that
  specific run at 60-second intervals; no CI retry was requested. The corrected
  local flake process also remains live, with no additional failure reported.

Next: collect those existing executions, investigate failures if any, then
finalize and commit the delivery audit/tasks and publish the ready-for-review PR
only with verified green checks. Do not start another flake run merely because
the current compilation has not printed recent output.

### Verified wait: corrected package and clippy complete

Revalidated the same local flake process and CI watcher; both remain active.
The corrected release package completed server and WASM builds successfully
(`w86n5gwp961yq466bap4csrw6kjylysd-horae-0.1.0.drv`, about nine minutes).
Clippy completed without warnings (`rwx0ni4maji927wphs9nd16f478rwx0z-horae-clippy-0.1.0.drv`).
The local process has now started the corrected full browser derivation named
above. No new execution, code edit or CI retry was started during this wait.
Next remains the existing full-browser/flake and published-head CI results;
T023/T025 are still open.

## Iteration: remove cross-suite fixture assumptions

- The intervening prompt-only turn did not advance implementation. Re-read the
  objective and revalidated the same Nix process and CI watcher. The local
  process is now terminal: every non-browser check passed, but the browser
  suite failed in `client-context.cjs`; T023 remains open.
- The failure had two fixture causes. `project-edit.cjs` renames ACME-01;
  `invoice-preparation.cjs` moves TECH-01 to Acme for fee preparation. Thus the
  original name and assumed TechStart ownership were invalid after those suites.
  The client filter correctly matched the modified data; no UI fix was needed.
- Recovery/visual readiness assertions now identify ACME-01 by its stable code.
  Context navigation creates two dedicated synthetic projects before taking
  its baseline snapshot, proves both are visible without a filter, then checks
  inclusion/exclusion for each client by exact project ID. It still compares
  full project/invoice snapshots after navigation and preserves all draft tests.
- `project-edit client-context clients-errors clients-visual` passed on a fresh
  disposable database with the first selector correction. After discovering
  the fee suite's ownership mutation, added the independent context fixtures
  and started all 21 browser suites in their unchanged default order against
  the exact release package built for `eee87ba`.
- The previous full gate's Rust tests passed (817 server, 11 existing ignored;
  other targets also passed), SQLx prepare-check passed, and both deployment
  checks completed (normal 69.84s, OIDC 21.08s). These results do not override
  the failed browser gate. No production source, stylesheet, schema or
  dependency changed in this correction.
- Re-ran Spec Kit prerequisites for the increment; readiness remains 10/10.
  Restored the helper's feature-pointer side effect to the inherited
  `specs/016-expense-parity` value. No extension hooks exist.

Next: collect the full sequential browser run, investigate any remaining
failure, then publish the correction and run the complete Nix gate and
published-head CI. PR #216 remains draft; no merge or queue change.

### Session-identity follow-up

The full sequential run passed the corrected context suite, then exposed a
second isolation assumption in `clients-access.cjs`: it changed the role of
`admin@example.com`, although dev login had selected the additional administrator
created by `new-project-permissions.cjs`. The visible New client action correctly
belonged to that still-administrative session. The access suite now obtains the
actual session actor through its observed `get_me` endpoint before creating
assignments or changing roles. All negative-access, payload, foreign-org and
inactive-user assertions remain unchanged; cleanup restores that same actor.
Started the focused sequence `new-project-permissions client-context clients-access clients-errors clients-visual` on another fresh disposable DB to
exercise this case. The complete gate is still required, not marked green.

The five-suite follow-up completed with exit 0: both permission matrices,
context/draft protection, client error recovery and all 18 visual/keyboard
configurations passed against the release artifact. Format also passed without
changes. Publishing the four browser-test corrections and audit record next;
the new full Nix run and updated-head CI must still pass before delivery.

### Published correction and live delivery checks

Published unsigned commit `5de6a8b95e4191e7724a310d22ca6ca61411dacf` to
PR #216. The formatter normalized one wrapped inline command in this log;
the subsequent format check passed unchanged before commit/push.

Started the complete local command
`nix fmt -- --ci && nix flake check --keep-going --max-jobs 1 --cores 4`
on that committed head (session `20496`). Format passed and flake evaluation
completed; the process is live. The browser derivation is
`601iq1ksxzhdb4j9ri264plcsl4l92l9-horae-browser-checks.drv`.
Current-head CI is run `36964454445`, watched at 60-second intervals (session
`89221`). The superseded `eee87ba` run is not evidence for the new head.

Next: resume these exact executions, inspect any actual failure, and finish
T023/T025 plus the final delivery audit only after full green checks. Do not
restart because an observation times out. No merge or queue action occurred.

### Verified wait: published-head package complete

The preceding turn made progress by publishing the fixture-isolation corrections.
This iteration revalidated the same local process `20496` and CI watcher `89221`;
neither was restarted. Current-head CI Format passed in 56 seconds; Flake Check
remains running. Locally the release package
`k33y0yhvd05z8kfq2bwsjfwkwr10qwn0-horae-0.1.0.drv` completed and the process
advanced to Clippy. A read-only capacity check found 22 GB available; no artifacts
were removed. T023/T025 remain open. Next: collect the remaining checks from
these same executions, especially the complete sequential browser suite.

### Full sequential browser gate passed

The browser derivation for `5de6a8b` completed all 21 suites successfully in
the default order, including the previously failing context/access tests,
client persistence/error recovery, all 18 visual/keyboard configurations and
the project/invoice/import-entry regressions. Reviewed the completed log
`601iq1ksxzhdb4j9ri264plcsl4l92l9-horae-browser-checks.drv`; no suite was skipped.
Clippy also completed successfully (`gng0ykcmhpw6ih1zk711j84dak31j1c9`, 1m40s).
The same local process has advanced to SQLx prepare-check. CI Flake Check is
still active; its Format job passed. Next: finish the remaining local checks
and current-head CI, then close T023/T025 and the delivery audit.

### T023 complete: full local Nix gate green

The existing process `20496` finished with exit 0 and `all checks passed!` on
published code head `5de6a8b`. This closes T023: format, release server/WASM,
Clippy, the full default browser suite, SQLx prepare-check, domain/server/test
targets and both NixOS deployment checks passed. The check ran for the current
`x86_64-linux` system; it did not claim cross-platform checks on incompatible
Darwin/aarch64 systems. Server results include 817 passing cases and 11 existing
ignored cases; all additional test targets passed. SQLx used a fresh isolated DB.

Current-head GitHub run `36964454445` remains active through watcher `89221`,
with Format passed. T025 and MVP-SC-006 stay open until green current-head CI,
final documentation publication and ready-for-review state. The final local
changes are evidence/task documents only; no functional changes followed the
validated head. Next: observe that existing CI run and finalize delivery after
its actual result. Do not rebuild or rerun CI merely because it is still active.

Updated PR #216's description with the full local pass and the explicit pending
CI gate; the edit completed successfully. Revalidated watcher `89221`: run
`36964454445` is still executing Flake Check (about 29 minutes after trigger),
with Format passed. Only delivery documents are dirty locally. The completed
Nix process is terminal/successful; do not attempt to resume or restart it.
Next remains this exact CI run, followed by the final documentation commit,
verification of its own published-head CI and ready-for-review transition.

### Delivery review check while CI runs

The previous iteration completed the full local gate and updated the delivery
record/PR description. Revalidated CI watcher `89221`; run `36964454445` remains
live with Format passed. Checked PR #216's actual head and review state: still
open/draft at `5de6a8b`, with no submitted reviews or comments to address. This
does not replace the recorded adversarial/independent UI reviews or the CI gate.
Continue observing the same run; no new CI execution or merge was requested.

### Verified CI wait

Observed the existing watcher `89221` throughout this iteration. Its latest
response still reports run `36964454445` / Flake Check job `110705041039` in
progress about 44 minutes after trigger, with Format passed. This is a verified
wait, not a terminal failure or a product blocker. No run was restarted and no
additional implementation was invented while waiting. Next: resume this same
watcher, inspect the actual terminal result, then publish the final documentation
and verify its head before ready-for-review delivery. Goal remains active.

## Delivery closure

CI watcher `89221` finished successfully. Run
[36964454445](https://github.com/numtide/horae/actions/runs/36964454445)
verified exact implementation head `5de6a8b95e4191e7724a310d22ca6ca61411dacf`:
Flake Check passed in 48m36s and Format in 56s. Re-read run/PR metadata to confirm
the head, success conclusions and absence of pending reviews/comments, then
marked [PR #216](https://github.com/numtide/horae/pull/216) ready for review.
There was no merge or queue operation.

All 25 increment tasks now have evidence. The four authorized journeys are
implemented and verified, with the requirement-by-requirement mapping in
[completion-audit.md](completion-audit.md). Local verification passed all 21
browser suites, 121 domain tests, 817 server tests plus the other targets,
release server/WASM, Clippy, SQLx cache and both deployment checks. The existing
11 ignored server cases were not changed or newly introduced by this increment.
Author-led adversarial review and independent UI reviews are distinguished;
there are no unresolved critical/high findings from those reviews.

Harvest evidence remains deliberately bounded: official documentation and
available read-only browser observations, with unverified behavior and approved
differences recorded in research.md. No real records, outbound messages, schema,
dependencies, permission-model cutover or merge queue were changed. Parent 012's
contacts, broader permissions, bulk/export/destructive actions, archive policy,
new billing terms and summary/provenance panels remain deferred requirements.

The closing commit updates delivery documents only. Next: verify its formatting,
publish it unsigned and await its own GitHub checks before marking the goal
complete. Do not infer a later head's CI result from the green implementation
head. Subsequent delivery-only status and the final-head verification are
recorded in the linked PR description/checks, avoiding another source commit
solely to record that source commit's check result. If a real check fails,
investigate and record the correction here before republishing. No more feature
work or merge is authorized by this goal.
