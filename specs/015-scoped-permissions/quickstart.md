# Permission verification

## User directory payload (T145–T147)

Run in the Nix shell with `DATABASE_URL` pointing only to disposable PostgreSQL:

```sh
SQLX_OFFLINE=true CARGO_INCREMENTAL=0 cargo test -p horae --features server --bin horae job_endpoints_enforce_session_role_and_organization --locked
SQLX_OFFLINE=true CARGO_INCREMENTAL=0 cargo test -p horae --features server --test detail_navigation --test permission_editor_ui --test own_permissions_ui --test admin_shell --locked
SQLX_OFFLINE=true CARGO_INCREMENTAL=0 cargo clippy -p horae --features server --all-targets --locked -- -D warnings
SQLX_OFFLINE=true CARGO_INCREMENTAL=0 cargo clippy -p horae --features web --target wasm32-unknown-unknown --locked -- -D warnings
```

The initial real-session test fails on ten response keys versus the five allowed
by `contracts/people-directory.md`. The repaired registered HTTP matrix passes;
the final offline-compiled run takes 12.81s after compilation. It exercises all
three legacy roles, both inactive-filter values, unauthenticated/inactive denial,
tenant isolation, duplicate names with distinct IDs, exact keys and values,
unchanged own-user data and same-cookie demotion of inactive-directory access.
The production query selects only the list fields, not a full `User` followed by
redaction. The existing browser response shape changes together with its typed
client; no compatibility API, authentication, mutation, schema or CSS changes.

Full SQLx preparation with `--features server --all-targets` initially omits cached
integration-test queries. Cleaning only rebuildable Horae artifacts and repeating
preparation completes in 53.97s: 1,409 descriptors, replacing only the old list
query and adding the non-null-sensitive-field fixture query. No unrelated query
descriptor is lost. Focused source review checks every consumer, exact SQL/DTO
fields, tenant/activity predicates, server-derived identity and unchanged legacy
guards. This is not new-model directory/shell authorization, in-flight read
revocation, fresh browser acceptance or a full-flake verification.

All 89 selected consumer/permission tests pass (23 detail-navigation, 50 editor,
11 own-permissions, five admin-shell). Offline all-targets server Clippy passes
in 1m04s and WASM Clippy in 13.38s, both with warnings denied. Formatting and
whitespace checks pass. The disposable PostgreSQL is stopped after verification.
Spec Kit prerequisites pass; its unavailable skills were not executed. T145–T147
are complete for this payload repair; full integration/acceptance remains open.

## Permission-editor subject picker (T142–T144)

Run inside the Nix shell:

```sh
SQLX_OFFLINE=true CARGO_INCREMENTAL=0 cargo test -p horae --features server --test permission_editor_ui --test own_permissions_ui --test admin_shell --locked
SQLX_OFFLINE=true CARGO_INCREMENTAL=0 cargo clippy -p horae --features server --all-targets --locked -- -D warnings
SQLX_OFFLINE=true CARGO_INCREMENTAL=0 cargo clippy -p horae --features web --target wasm32-unknown-unknown --locked -- -D warnings
```

Four initial handler tests fail for the missing picker. After implementation,
66 selected tests pass (50 editor, 11 own-permissions, five admin shell), including
the additional empty/error/retry case. Coverage includes cursor navigation,
hidden stale pages, inactive/duplicate names identified by UUID, declined/failed
dirty confirmation, independently loaded targets, changed requester rejection,
pending previews and uncertain saves. Existing recovery and revocation tests
remain green. The latter caught a picker retaining names after the form was
denied; both now disappear together. Server/WASM Clippy pass (1m06s / 13.17s).

The full server/WASM bundle builds in 78.82s. The extended
`permission-editor-recovery` browser suite passes against its own disposable
PostgreSQL on Chromium 148.0.7778.96 / Playwright 1.60.0. It checks keyboard menu
opening, focus, Escape without dismissing the editor, cancelled/confirmed dirty
person switches, a clean switch back and no selection-triggered mutation, then
reruns the existing real-command/lost-response/recovery cases. Six captures of
editor/menu/recovery at desktop-dark 1440 and mobile-light 390 were inspected in
`.scratch/permission-subject-evidence/`. This is not Windows Chrome/MCP, full
cross-browser, complete T018 or full-policy acceptance. Services stop on exit.

The picker uses existing Menu, Modal and utilities; no CSS, shared-component,
database, dependency or authority-policy change. The previously unused wire
projection is consumed and its temporary lint expectation removed. Spec Kit
prerequisites pass, but its skills are unavailable locally; no skill execution
is claimed. Impeccable context/detector cannot run without its missing engine;
source and screenshot checks substitute, without installing it.

Fresh independent finish review returned `ship` for this extension, with no
material findings in the changed code and six captures. Pagination/duplicate/
inactive/error cases have handler evidence rather than rendered captures.
The generic agent interface could not select the shipped reviewer type; it used
the full reviewer packet in a fresh context. No full-feature verdict is implied.
Independent documentation review confirms no new durable system rule or contract
correction. Existing DESIGN.md path/tool-format drift is unchanged. Formatting
and whitespace checks pass; T142–T144 close, while T018 remains open.

## Permission-editor subject discovery (T139–T141)

Run in the Nix shell against the owned disposable PostgreSQL:

```sh
CARGO_INCREMENTAL=0 cargo test -p horae --features server --bin horae permissions:: -- --nocapture
CARGO_INCREMENTAL=0 cargo test -p horae --features server --bin horae job_endpoints_enforce_session_role_and_organization -- --nocapture
```

The final selected run passes 134 tests, including six new subject-discovery
cases, in 56.28s after compilation. The registered HTTP matrix passes in 11.67s.
The first compilation failed for the absent reader; the first implemented run
exposed a disposable-fixture FK assumption, not an authorization failure.

Cases cover 53 local people across two pages, exact 50-row termination, inactive
people without canonical state, duplicate names, missing/deleted/foreign cursors,
empty pages, explicit Administrator versus all grants/legacy Admin, invalid
policy/storage, concurrent revocation/deactivation and cancellation with immediate
single-connection reuse. Inherited connection defaults remain unchanged. HTTP
checks exercise the registered server function, session-derived identity,
missing authentication, forged authority, policy denial, exact minimal fields
and sanitized malformed-storage responses. No general-directory, UI/browser or
activation acceptance is implied. Shared authority/transaction code is reused.

Full SQLx preparation after package-only artifact cleanup preserves the cache
and adds three descriptors (1,408 total, 54.24s). Offline all-targets server
Clippy passes in 1m04s; final WASM Clippy passes in 12.69s. The new page response
has one web-only `expect(dead_code)` because its picker is not connected yet;
connecting the consumer must remove that now-unfulfilled expectation. No broad
lint exemption or runtime fallback is added. No new full-flake, full server-suite
or browser result is claimed, and the owned disposable PostgreSQL is stopped.

## Combined-approval record guard (T136–T138)

Pure FR-006/024 checks use existing canonical selections and record coverage.
They are not a replacement for the pending database approval transaction. Run:

```sh
cargo test -p horae-core permissions::approvals --locked
cargo test -p horae-core --locked
cargo clippy -p horae-core --all-targets --locked -- -D warnings
cargo check -p horae-core --target wasm32-unknown-unknown --locked
```

The first test run failed for the missing guard. After implementation, all 180
core tests pass, including 11 new approval tests. A deliberate mutation changing
the expense approval/read conjunction to OR makes the mixed-hidden-expense test
fail. Restored the conjunction and reran all 180 successfully; final core Clippy
and WASM compilation pass (0.66s / 9.21s), with formatting and whitespace checks.

Coverage includes independent expense read/approval scope truth tables, every
catalog grant's inability to substitute for approval or expense reads, inactive
and foreign provenance, mixed visible/hidden inputs in both orders, both domains'
out-of-scope rows, overlapping/removed assignments, duplicate/missing dimensions,
empty domains and reevaluation after revocation. The canonical approval grants
already require their matching time reads; the guard still checks both explicitly.
No invalid stored selection or fabricated runtime authority is used as a fixture.

Focused adversarial self-review checked the intersection of capabilities versus
the union of managed scopes, own-read retention, empty-input identity checks and
non-disclosing all-or-nothing output. No independent agent or backend/browser
acceptance is claimed. Passing empty record slices does not authorize empty-date
locks, and passing an own record does not settle self-approval. The caller must
load the full intended set without visibility filtering and evaluate current
facts inside the eventual atomic transaction. T012/T013, feature 016 expenses,
T042 and full cutover gates remain open. No SQL, schema, UI or live data changed.

## Project-delegation activity fences (T133–T135)

Run in the Nix shell with the owned disposable PostgreSQL database, never the
application database:

```sh
SQLX_OFFLINE=true CARGO_INCREMENTAL=0 cargo test -p horae --features server --bin horae permissions:: -- --nocapture
```

Three initial regressions failed on `c88ca6d`: actor and added-manager activity
were not retained until receipt commit, and inherited READ ONLY rejected the
organization lock. After sharing the administration setup and locking activity
reads, all 128 selected permission tests pass (54.22s test execution).

Six new cases cover both subjects in both race orders, local transaction defaults,
and cancellation/rollback with immediate exact retry through a one-connection
pool. Tests observe PostgreSQL blocker dependencies; they do not infer ordering
from sleeps. Existing project tests also pass for retained inactive/incompatible
managers, archived projects, no-ops, exact replay, revocation, audit failure,
membership/history preservation and the project NOWAIT rollback.

Focused source review checks authorization before replay, tenant-bound activity
reads, sorted added-manager locks, compatible SHARE modes and absence of user or
project-parent writes. The existing organization gate serializes canonical
writers; direct SQL changing grants outside that protocol remains unsupported.
The new command setup preserves stricter inherited statement/idle limits and
does not change session defaults. No new grant, endpoint, migration or dependency
is introduced. This is not global policy activation or browser acceptance.

Complete SQLx regeneration after cleaning only Horae build artifacts produces
1,405 descriptors. The sole removal is the replaced plain EXISTS activity query;
its locking replacement already has a shared descriptor. The first cached run
omitted still-used integration queries and was not accepted. Offline all-targets
server Clippy passes with warnings denied (1m03s). Formatting and whitespace
checks pass. No new WASM, browser, full server-suite or full-flake result is
claimed for this internal server-only repair.

## Real-browser request recovery (T018 subset)

`permission-editor-recovery.cjs` runs against the design runner's disposable
PostgreSQL and a complete Dioxus server/WASM bundle, not mocked server outcomes.
It is included in the default browser check. To focus it, provide the same
`HORAE_TEST_SERVER`, `PLAYWRIGHT_MODULE` and `PLAYWRIGHT_BROWSERS_PATH` inputs as
`nix/checks/browser.nix`, then run in the Nix shell:

```sh
bash crates/horae/tests/browser/run-design-checks.sh permission-editor-recovery
```

The server needs its matching `public/` beside it. The verification bundle was
built from application commit `1ecfa21` with `dx bundle --web --fullstack true --debug-symbols false --locked` in `crates/horae/` (164.14s). This is a complete
local build, not a full `nix flake check` result. The test refuses ordinary app
ports and database connections outside the runner's `/tmp/horae-browser.*` socket,
requires empty canonical fixture state/history, and restores policy 0, legacy
login candidates and fixture state under the organization gate before shutdown.

The headless Chromium 148.0.7778.96 / Playwright 1.60.0 run verifies:

- A real person command commits before its response is deliberately lost; reload
  offers explicit recovery, sends nothing automatically and replays the identical
  command/requester. PostgreSQL contains one receipt and the expected grant.
- A real template creation remains recoverable after switching accounts and
  returning to the original user through a new login session. The other user is
  not offered the original slot. A later current-authority denial retains it;
  restoring authority replays the same receipt without another template.
- A deleted template is not required for recovery: its exact deletion request
  replays successfully without recreating it or duplicating its receipt.
- A browser storage quota failure prevents the first HTTP mutation. Failure to
  remove a record after successful self-demotion offers cleanup, which sends no
  second command under the now non-administrative identity. Local storage is
  empty after acknowledged recovery.
- Tab reaches enabled controls, the modal keeps background controls inert,
  Escape respects dirty-discard refusal/confirmation, focus returns to the opener,
  and Enter triggers recovery. Wide dark (1440×900) and narrow light (390×844)
  checks find no horizontal dialog overflow; the panel fits each viewport.

Optional `HORAE_BROWSER_ARTIFACTS` captures editor/recovery screenshots. Inspected
captures live in this worktree's `.scratch/permission-browser-evidence/` with
`permission-editor-*` and `permission-recovery-*` names. They use disposable
fixture data only. Initial test failures came from addressing a closed native
details section and over-specifying focus wrap order; the test now opens the
section and checks actual keyboard access, inertness and focus restoration.
No production UI/CSS change was required by this browser pass.

This is Linux headless Chromium, not Windows Chrome/MCP or a physical mobile
device. It establishes these recovery cases, not full permissions/Workspace
acceptance, migration, canonical shell/directory integration or all browser engines.
Full T018 remains open; the earlier unit-only limitation below is superseded only
for the browser cases enumerated here.

## Durable tab recovery (T018, in progress)

Run in the Nix shell without a live database:

```sh
SQLX_OFFLINE=true CARGO_INCREMENTAL=0 cargo test -p horae --features server --test permission_editor_ui --test own_permissions_ui --test admin_shell
node --test crates/horae/tests/browser/editor-navigation.cjs crates/horae/tests/browser/permission-recovery-storage.cjs
```

All 61 selected UI tests pass (45 editor, 11 Settings, five admin shell; 39.56s
build). All 14 shipped-script unit tests pass (eight navigation, six storage).
Production handlers require acknowledged storage before server submission and
retain the exact command/requester across a VirtualDom remount with no selected
person. Cases cover person/template recovery, no automatic submission, duplicate
clicks, session/read/write/acknowledgement failures, malformed/misbound/oversized
records, account/workspace isolation, denied-retry retention, explicit checked
discard, conditional cleanup and acknowledged cleanup without resubmission.

Source review corrected permanent `aria-busy` during idle recovery and kept
authentication diagnostics separate from public validation messages. The cached
acknowledgement is bound to its whole command/requester; cleanup cannot erase a
different record. Existing Modal, Checkbox and CSS are unchanged. Server commands,
authorization, SQL and schema are unchanged, so no SQLx regeneration is needed.
Final offline all-targets server Clippy (1m02s) and WASM Clippy (13.26s) pass with
warnings denied. Formatting CI and whitespace checks also pass.

These are controlled handler/remount and Node VM tests, not Chrome persistence,
keyboard, focus, viewport or theme acceptance. No browser tool is loaded in this
session. Full T018, full-flake verification and policy integration remain open;
no real data or active policy changed. Spec Kit prerequisites pass, but its
implement skill is not available through the current local catalog or tools;
the existing contract/plan/task workflow was followed directly.

## Original requester binding (T130–T132)

Run in the Nix shell against the owned disposable PostgreSQL database:

```sh
SQLX_OFFLINE=true CARGO_INCREMENTAL=0 cargo test -p horae --features server --bin horae job_endpoints_enforce_session_role_and_organization --locked
SQLX_OFFLINE=true CARGO_INCREMENTAL=0 cargo test -p horae --features server --bin horae permissions::profile_tests --locked
SQLX_OFFLINE=true CARGO_INCREMENTAL=0 cargo test -p horae --features server --test permission_editor_ui --test own_permissions_ui --test admin_shell --locked
```

RED reproduced an HTTP 200 person save under a different Administrator's session
instead of the required 403. After binding both save transports to the requester
returned by the authorized load, the registered HTTP matrix passes (11.63s).
Cases cover different users/organizations, omitted or forged binding, unchanged
state after denial, success and exact historical replay after same-user
reauthentication. Matching one's own identity is not authority: the non-admin
negative-payload case still exercises canonical authorization, not only a
mismatched pair. All 34 profile/editor PostgreSQL regressions pass (15.41s).

All 48 selected UI tests pass; the production person/template controls supply the
loaded requester on both initial submission and identical-command retry. Existing
command intent, audit/receipt storage, policy checks and CSS are unchanged. New
fixtures reuse existing checked SQL; no new query descriptor or migration is
needed. This is a recovery prerequisite, not browser persistence or full T018.
Offline all-targets server Clippy (1m02s), WASM Clippy (12.67s) and formatting CI
(491 files, zero changes) pass.
`cargo sqlx prepare --workspace --check -- --features server --all-targets`
also succeeds (52.24s) with a potentially-unused-descriptor warning. Existing
cache files are unchanged, not pruned on the basis of that warning.

## Editor navigation and dismissal (T018, in progress)

Run without a database in the Nix shell:

```sh
node crates/horae/tests/browser/editor-navigation.cjs
SQLX_OFFLINE=true CARGO_INCREMENTAL=0 cargo test -p horae --features server --test permission_editor_ui --test own_permissions_ui --test admin_shell --locked
SQLX_OFFLINE=true CARGO_INCREMENTAL=0 cargo clippy -p horae --features server --all-targets --locked -- -D warnings
SQLX_OFFLINE=true CARGO_INCREMENTAL=0 cargo clippy -p horae --no-default-features --features web --target wasm32-unknown-unknown --locked -- -D warnings
```

Observed: eight Node unit tests pass against the shipped history guard; 48 selected
UI tests pass (editor 32, Settings 11, admin shell five). Node uses a simulated
DOM/history, not a browser. Dioxus tests invoke production controls with controlled
server replies and a test document provider for native confirmation responses.
The default browser CI runner also includes the Node suite.
Offline all-targets server Clippy (1m03s) and WASM Clippy (12.76s) pass with
warnings denied. Formatting CI passes (491 files, zero changes).

| Requirement | Increment evidence |
| --- | --- |
| FR-004 draft preservation | Grant reversal/dependency loss, source/template revision and independent identity; person edits survive template operations |
| FR-012 distinct states | Initial/reload/preview/save pending; errors release read locks; uncertain saves remain pending; clean dismissal needs no confirmation |
| FR-016 existing controls | Shared Modal unchanged; no CSS; Close/Cancel/native cancel/backdrop share discard confirmation, refusal/error preserves input |
| FR-018 entry-point regression | Actual Rust handlers, permission/project/invoice push/replace/pop/unload cases, Dioxus scroll slots and legacy project attributes |

RED reproduced missing navigation state and the unprotected initial load. Source
review identified that read gap and existing unconfirmed modal dismissal; both
were corrected and re-reviewed. Test-fixture corrections account for static
Dioxus IDs, explicit pointer data and grant removal cascading to dependents.
The source/documentation reviews found no further issue within this increment.
They do not establish rendered acceptance. Full T018, browser keyboard/themes/
viewports and durable same-request recovery after forced reload remain open.
No SQL, migration, dependency, shared Modal or active-policy change is included.

## Management-loss names (T018, in progress)

Run in the Nix shell against the owned disposable database:

```sh
cargo test -p horae --features server --bin horae permissions::profile_tests --locked
cargo test -p horae --features server --test permission_editor_ui --test own_permissions_ui --test admin_shell --locked
cargo test -p horae --features server --bin horae job_endpoints_enforce_session_role_and_organization --locked
cargo sqlx prepare --workspace -- --features server --all-targets
```

Observed: RED reproduced missing preview names; 34 database tests, 42 selected UI
tests and the registered HTTP matrix passed. The first UI run exposed two test
expectations for named rather than numeric HTML entities; corrected expectations
now verify Dioxus's escaped output. Cached SQLx preparation omitted 91 unchanged
integration-test descriptors despite passing offline lint; cleaning only Horae's
rebuildable artifacts restored complete generation: 1,406 descriptors, five added
and none removed or modified. Repeated offline all-targets server Clippy (1m02s)
and WASM Clippy (12.74s) passed with warnings denied.

FR-010/018 coverage includes other managers/tenants, inactive subjects and denied
requesters. FR-013/025/029/030 coverage preserves exact confirmation, strict
name-free historical decoding and preview/save effects. FR-012/016 coverage
checks escaped names, hidden internal IDs and shared wrapping utilities.
Independent source and design-documentation reviews found no actionable issue
in their scope. Browser acceptance and full T018 remain open; no activation,
real-data migration, new schema, dependency or global CSS change is included.

## Reusable profile controls (T018, in progress)

Use the same offline Nix UI/lint commands in the next section. The editor suite
now contains 26 tests, including ten new control-level template scenarios.
Initial RED failed on missing creation/deletion controls. After wiring both
expected deletion revisions and the existing stable control-ID pattern, all 24
passed (38.54s compilation, 0.02s execution). Selected regressions initially
passed 40 tests, followed by offline native/WASM lint. Source review's pending
deletion-preview lock and undefined margin-class findings are corrected and
re-reviewed. Adding cancellation/reload coverage then passed 25/26, reproducing
an unchanged-revision reload that retained the old form. The shared reload now
changes a local component-generation key. Final post-fix selected regressions
passed all 42 tests: editor 26, Settings 11, admin shell five (39.42s build).
Offline all-targets server Clippy (1m02s) and WASM Clippy (12.53s) passed with
warnings denied. The independent source verdict found the reload fix sound;
the read-only documentation review confirmed existing design primitives suffice.

The tests invoke production UI handlers with controlled server replies. They
cover final grants, blank/duplicate/limit/validation failures, explicit identity,
cancellation retaining the unsaved person draft, exact affected-person review,
stale/mismatched previews, revoked authority and identical create/delete retries.
Server preservation and real PostgreSQL/HTTP results remain the separate earlier
backend evidence, not newly rerun or established by these UI tests.

| Requirement subset | UI evidence |
| --- | --- |
| FR-004/015/032 explicit reusable selections and names | Final-grant capture, no person save, count/identity gates, server duplicate/limit/validation responses |
| FR-010/011/018 current authority and recovery | Denial hiding, exact request retries, mismatch/stale rejection, pending-preview lock, explicit fresh reload |
| FR-012/016 shared explanations and preservation | Shared grant descriptions, affected names, confirmation even for empty profiles, cancellation retaining person edits |

No CSS, SQL, migration or dependency changed. A completed template mutation still
requires an explicit reload/discard before editing the person again; navigation
recovery and browser/theme/viewport/keyboard checks remain open. No full T018,
canonical activation, migration acceptance or visual pass follows.

## Person permission editor UI (T018, in progress)

These checks need no running database. Run in the pinned Nix shell with the
committed SQLx cache:

```sh
SQLX_OFFLINE=true cargo test -p horae --features server --test permission_editor_ui --test own_permissions_ui --test admin_shell --locked
SQLX_OFFLINE=true cargo clippy -p horae --features server --all-targets --locked -- -D warnings
SQLX_OFFLINE=true cargo clippy -p horae --no-default-features --features web --target wasm32-unknown-unknown --locked -- -D warnings
nix fmt -- --ci
```

The editor suite includes eight local draft tests, four SSR/error-state tests,
the shared description test and three controlled-response VirtualDom tests. The
latter mount the production dialog and invoke its actual event handlers, rather
than a duplicate state machine. They verify double-click suppression, no writes
during review, identical-command retry after a lost response, revocation hiding,
explicit keep-project grants followed by a new review, independent confirmation
of person losses and stale-save recovery. Controlled server replies complement
the backend's registered HTTP/PostgreSQL tests; they are not browser evidence.

Observed: the initial draft failed to compile before implementation, then six
tests passed. Expanded native state/SSR coverage passed 12. The first interaction
build exposed sibling module path resolution; explicit paths fixed it. The final
selected regression run passed 32 tests: editor 16, own-permission Settings 11 and
existing admin shell five. Clippy then requested a simpler equivalent boolean
condition; that is corrected. The post-fix editor run passed all 16 tests, and
offline all-targets server Clippy (1m01s) and WASM Clippy (12.60s) both passed
with warnings denied. No full database suite or full flake check was rerun for
this UI-only increment; the backend results below belong to its earlier run.

No CSS or SQL changed. Full T018 remains open for template creation/deletion UI,
readable management-loss names, navigation recovery and browser/keyboard/theme/
viewport acceptance. The legacy AdminShell/directory still needs its reviewed
canonical-policy integration; this consumer does not activate policy or make the
full six-profile feature ready. Only isolated fixtures may enable policy 1.

## Authenticated editor backend (T126–T129)

Use the owned disposable PostgreSQL database and the Nix dev shell; never enable
policy 1 on application data for these tests. The local contract is
`contracts/permission-editor.md`. No UI, migration or activation acceptance follows.

```sh
cargo test -p horae --features server --bin horae server_fns::permissions:: --locked
cargo test -p horae --features server --bin horae job_endpoints_enforce_session_role_and_organization --locked
cargo test -p horae --features server --bin horae --locked
cargo sqlx prepare --workspace -- --features server --all-targets
SQLX_OFFLINE=true cargo clippy -p horae --features server --all-targets --locked -- -D warnings
SQLX_OFFLINE=true cargo clippy -p horae --no-default-features --features web --target wasm32-unknown-unknown --locked -- -D warnings
nix fmt -- --ci
```

| Requirement subset | Production evidence |
| --- | --- |
| FR-010/011 current actor and last active Administrator | Profile/template command tests cover direct deactivation before replay, target activation, survivor deactivation in both orders and retention through receipt insertion |
| FR-004/015 explicit final proposal and template lifecycle | `editor_tests` compare every built-in preview with actual saved state, exact no-op and template detachment with preserved grants/identity |
| FR-025/029/030 confirmed relationship losses | `editor_preview_matches_saved_effects_without_writing` compares exact joint effects with save, rejects missing confirmation and stale revisions; existing keep-access regressions remain |
| FR-013/018/032 authenticated delivery and errors | `authorization_tests/permission_editor.rs` uses registered routes/cookies for load, preview, save, create/apply/delete, historical replay, forged authority, policy denial and sanitized failures |

Initial RED reproduced two direct-deactivation races and READ ONLY SQLSTATE 25006.
The first command GREEN run passed nine selected tests. Missing editor modules
then failed compilation; shared evaluation/read implementation passed the first
104 permission tests (45.61s). Registered-route RED found zero editor endpoints
before the wrappers were added. Final full-suite and cache/lint results follow
when observed; these intermediate runs are not final acceptance.

Read-only independent reviews found no high/critical implementation issue. Added
their requested template-deletion preview/save/stale-set and demotion-winning
survivor-lock cases. Scoped Spec Kit analysis maps all four new tasks to ten
requirement subsets, with no unmapped task, ambiguity, duplication or constitutional
conflict in this increment. Full T006/T042, UI and activation remain open.

Full server-binary regression: 1,030 passed, zero failures, 11 pre-existing ignored
manual measurements (250.14s). This includes all 15 new PostgreSQL cases and the
registered editor lifecycle. No new test was ignored. WASM Clippy passed with
warnings denied (12.72s); root transport DTOs explicitly document their pending
Workspace consumer without silencing server warnings or unrelated modules.
The expanded registered HTTP matrix passed separately (11.53s), including
initial-authentication cancellation/recovery, denial of every new endpoint under
legacy/future policy and invalid-command validation. Final cache/lint gates follow.

Complete SQLx regeneration finished in 52.55s: 14 added descriptors, six obsolete
profile-query descriptors removed, zero modified; 1,401 total. Each removal was
checked against the replaced leaf-lock/activity query, not an unexplained cache
loss. The preparatory package clean removed only 1.6 GiB of regenerable build
artifacts. An overlapping format check saw SQLx's transient cache removal and
formatted a new progress paragraph; rerun formatting after preparation, never
restore obsolete descriptions or report that interrupted check as passing.

Fresh offline all-targets server Clippy passed with warnings denied (1m01s).
The subsequent formatting CI run passed with zero changed files; diff checks
passed. T126–T129 are complete for the authenticated backend only. No real-data
operation, browser/UI acceptance, full `nix flake check`, policy activation or
merge was performed. Spec Kit and Rust/testing/async/simplicity guidance kept
preview/save calculation shared, preserved wire intent and avoided dependencies
or new persisted preview state.

## Authenticated permission history (T123–T125)

Use disposable PostgreSQL with the checked-in migrations, inside the Nix shell:

```sh
cargo test -p horae --features server --bin horae server_fns::permissions:: --locked
cargo test -p horae --features server --bin horae job_endpoints_enforce_session_role_and_organization --locked
```

The second command exercises actual registered Dioxus routes and real session
cookies, not a direct-function substitute. Its permission-audit submatrix covers
canonical Administrator identity independent of legacy role, forged identity
claims, foreign/missing IDs, expired/inactive sessions, typed writer-shape/no-op
round-trips, operator attribution and omission of private replay data. Corrupt
history/authority and a cancelled initial session-user query must produce the
same sanitized unavailable message. The cancellation targets only that test
database's query observed waiting behind its own table lock.

| Requirement subset | Reader evidence |
| --- | --- |
| FR-010: deactivation winning / reader winning | `direct_deactivation_winning_user_lock_denies_audit_read`, `audit_reader_holds_requester_until_materialization_finishes` |
| FR-011/013: current authority, exact historical projection | Existing `audit_tests` plus `authorization_tests/permission_audit.rs` |
| FR-018: inherited settings, bounded waits, cancellation and connection reuse | `audit_reader_overrides_transaction_defaults_without_changing_connection`, `audit_timeout_preserves_stricter_limits_and_releases_locks`, `cancelled_audit_read_releases_both_gates_and_single_connection` |

RED reproduced the missing registered route, unguarded direct-deactivation race
and inherited READ ONLY failure. The implemented reader passes all 97 permission
tests (45.19s). Independent re-review found an unsanitized initial user lookup;
the new HTTP cancellation case reproduced that failure before the wrapper fix.
Final HTTP and post-fix gates are recorded below when completed. No UI/history
browser, policy activation, live-data migration or full-feature acceptance follows.

Post-fix full server-binary run: 1,015 passed, zero failures, 11 existing ignored
manual measurements (240.64s); no test was newly excluded. This includes all 97
permission tests and the registered HTTP matrix with initial-query sanitization
and recovery. Independent re-review closes the error-projection finding and
reports no further blocking issue. This is not a full `nix flake check` result.

Complete SQLx preparation passes (51.45s), adding 14 descriptors with none removed
or changed (1,393 total). Final offline all-targets server Clippy passes (1m00s)
and offline WASM Clippy passes (11.21s), both with warnings denied. The shared
root DTO has a web-only unused-code expectation until the history UI consumes it;
no runtime check or test is disabled. The disposable database is stopped with
its data retained. No real data or policy mode was changed.
Formatting and `git diff --check` pass; T123–T125 are complete for this delivery
boundary. Full T041/history UI, activation, browser and flake acceptance remain open.

## Legacy source preflight (T117–T119)

Run `cargo test -p horae --features server --bin horae preflight_tests --locked`
inside the Nix shell against disposable PostgreSQL with all checked-in migrations.
No application server, Chrome, mail transport or real account is needed. Follow
with `server_fns::permissions::` regressions, clean complete SQLx preparation,
offline all-targets Clippy/WASM and formatting. The seven production-reader
tests in `permissions/tests/preflight.rs` map to the bounded contract:

| Requirement subset | Executable evidence |
| --- | --- |
| FR-006/014: tenant anomalies without foreign identities | `preflight_counts_cross_tenant_memberships_from_both_ends`, `preflight_approval_diagnostics_preserve_stored_history` |
| FR-014/017: unknown requester versus source corruption, unchanged artifacts | `preflight_separates_unknown_import_requesters_without_changing_artifacts` |
| FR-006/010: current legacy authority, not staged canonical privileges | `preflight_requires_current_legacy_administrator_not_staged_grants`, `preflight_rechecks_revocation_after_organization_and_actor_waits` |
| FR-017/018: cancellation/failure cleanup and size-one pool | `preflight_cancellation_releases_organization_and_single_connection`, `preflight_lock_failure_releases_organization_without_mutating_data` |

The initial count test fails against the empty reader (0 versus 1 cross-tenant
membership). The first implemented snapshot passes six tests. The subsequent
snapshot adds historical inactive-approver and lock-timeout coverage; its final
results are recorded separately below. Counts are neither unique combined totals
nor approval to activate. Full M01–M08, reviewed mappings and T019 remain open.

Final permission regression: 92 passed, zero failures/ignored (40.77s), including
all seven preflight tests and the inactive historical approver. Independent
static re-review found no blocking defect; it suggested an additional isolated
approver-only inbound fixture, beyond the existing combined inbound fixture.
The fixed numeric type and one aggregate statement are also source-reviewed;
no full migration or full-feature acceptance is inferred from this suite.

Clean SQLx preparation passes (51.94s): 17 new descriptors, none removed or
modified, 1,379 total. Offline all-targets server Clippy with denied warnings
passes (1m00s); denied-warning WASM passes (9.71s). This isolated internal reader
has no runtime callers or public surface; the affected permission suite above,
not a new full-server or `nix flake check` run, is this increment's regression
evidence. Full merge gates remain required before making the draft PR ready.

## Budget email preparation (T114–T116)

Run `cargo test -p horae --features server --bin horae notifications:: --locked`
inside the Nix shell against disposable PostgreSQL; no configured real mail
transport is used. `notifications/tests/authority.rs` exercises production
delivery with local executable stubs. Map current identity/eligibility to
FR-006/007, lock-wait revocation and late claim fencing to FR-010, unchanged
message/retry/disabled behavior to FR-017 and all concurrency/cleanup/regression
checks to FR-018 and SC-006. The exact cases are in
`contracts/budget-email-authority.md`. Follow with outbox/full server regression,
clean complete SQLx preparation, offline all-targets Clippy/WASM and formatting.
Record actual outcomes here and in `progress.md`; planned cases are not passes.

| Requirement subset | Executable evidence |
| --- | --- |
| FR-006/007/010: current recipient, project and relationship eligibility | `budget_delivery_waits_for_organization_and_rechecks_recipient`, `budget_delivery_refreshes_after_recipient_and_project_waits`, `budget_delivery_observes_winning_activation_and_alert_enable` |
| FR-007/010: trusted claim and actual lease after waits | `budget_delivery_rechecks_claim_and_clock_after_final_wait`, `budget_delivery_uses_current_payload_after_recipient_wait` |
| FR-007/010: no disclosure or terminal mutation on identity drift | `budget_delivery_skips_retargeted_notification_without_terminalizing`, `budget_delivery_skips_project_drift_or_disappearance`, `budget_delivery_skips_notification_appearing_after_discovery` |
| FR-017: preserved recipient rules and terminal behavior | `budget_delivery_rejects_missing_settings_without_waiting_on_child_locks`, `budget_delivery_terminal_rejections_preserve_existing_reasons`, existing retry/identity/disabled-worker tests |
| FR-018 / SC-006 subset: settings, cancellation and transport lock lifetime | `budget_preparation_overrides_read_only_without_leaking_transaction_settings`, `budget_preparation_cancellation_and_deadline_release_single_connection`, `budget_delivery_releases_all_gates_before_blocked_transport` |

The initial organization-wait test reproduced the missing preparation gate.
The corrected focused suite passes 30 notification tests, including all 13 new
authority tests (2.86s). Independent review's cancellation-observer and FIFO
lifetime findings are fixed and included in that pass. Full server-binary
regression passes 993 tests, zero failures and 11 existing exclusions (238.69s).
Clean SQLx preparation passes (52.89s): 44 new descriptors and only the two
replaced notification queries removed, 1,362 total. Offline all-targets Clippy
passes (1m00s), as does denied-warning WASM (9.48s). Formatting passes. Scoped
analysis covers six requirement subsets through three tasks with no unmapped
task or unresolved local critical/high review finding. No real mail or canonical
activation occurred; full feature acceptance and merge gates remain separate.

## CSV delivery (T110–T113)

Use the Nix shell and the owned disposable PostgreSQL only. Run the server
binary's `streamed_` tests, then `job_endpoints_enforce_session_role_and_organization`
and the full server regression. The cases in `contracts/csv-exports.md` cover
FR-006/007 (current actor and project scope), FR-010 (revocation under waits and
backpressure), FR-017 (unchanged source snapshot/CSV amounts), FR-018 and SC-006
(all three real routes, native transport, limits and cancellation). Follow with
complete SQLx preparation, offline Clippy/WASM and formatting. Actual outcomes
are recorded below and in `progress.md`; full permission activation is separate.

| Requirement subset | Executable evidence |
| --- | --- |
| FR-006/007: active same-tenant actor and correct operation predicate | `streamed_project_exports_deny_inactive_actors`, `streamed_exports_require_current_tenant_bound_actors_even_when_empty`, real-cookie export matrix |
| FR-010: fresh checks after organization, actor and parent waits | `streamed_exports_refresh_after_organization_or_actor_wait`, `streamed_projects_capture_scope_gained_during_initial_wait`, `streamed_project_release_refreshes_scope_after_parent_wait` |
| FR-007/010: current release authority without backpressure locks | `streamed_timesheet_rechecks_after_backpressure_without_retaining_locks`, `streamed_pending_project_block_checks_all_captured_ids_after_capacity`, `streamed_header_and_total_blocks_recheck_manager_after_capacity` |
| FR-017: exact frozen source values across new authority checks | `streamed_source_stays_frozen_across_batches_while_authority_is_fresh`, `streamed_invoice_metadata_and_lines_share_one_snapshot`, `streamed_invoice_keeps_original_metadata_lines_and_totals_between_batches`, existing escaping/fee/integer tests |
| FR-018: native transport and bounded buffering | `streamed_native_batches_keep_crossing_rows_and_reject_invalid_limits`, `streamed_compressible_unicode_payload_bounds_native_prefetch`, `streamed_native_projection_mismatch_fails_instead_of_coercing`, existing 10,001-row production export |
| FR-018 / SC-006: settings, cleanup and regression | `streamed_authority_releases_successful_locks_and_overrides_inherited_settings`, `streamed_cancelled_authorization_reclaims_its_single_connection`, `streamed_cancelled_fetch_reclaims_connection_without_authority_locks`, existing body-drop/deadline tests, HTTP and full server suites |

RED reproduced inactive-actor HTTP 200. Initial CSV regression passed nine tests;
the expanded report family passed 84 with two existing manual exclusions.
The real-cookie matrix passed (11.46s). Final-snapshot server regression passes
980 tests with zero failures and 11 existing exclusions (257.91s), including
the strengthened mixed-block and empty-response checks from adversarial review.
The first expanded build's generated-column fixture error was corrected, not
suppressed. Clean SQLx regeneration passes (56.54s; 1,320 descriptors, 41 new
and two obsolete), followed by offline all-target Clippy (1m05s) and
denied-warning WASM (10.41s). The initial cached preparation omitted unchanged
integration-test descriptors; cleaning only rebuildable Horae artifacts restored
them. This increment has no unresolved local critical/high review finding and
does not imply full canonical policy, UI or migration acceptance.

## Materialized project exports (T107–T109)

Use disposable PostgreSQL and the Nix shell. Run the `project_exports` subset
of the server-binary tests, the registered HTTP matrix
`job_endpoints_enforce_session_role_and_organization`, and then the full server
suite. The required cases in `contracts/project-exports.md` map to FR-006/007
(tenant/actor/current project scope), FR-010 (both race orders and scope gains),
FR-017 (unchanged exact payload) and FR-018 / SC-006 (limits, cleanup and HTTP).
Record actual outcomes below or in `progress.md`; planning is not a test pass.

| Requirement subset | Executable evidence |
| --- | --- |
| FR-006/007: current tenant-bound actor and project scope | `project_exports_require_current_tenant_bound_actor`, `project_exports_preserve_membership_visibility_and_history_rules`, `project_exports_reject_missing_foreign_and_unreadable_captured_ids` |
| FR-010: fresh scope after either authority wait | `project_exports_refresh_scope_after_winning_relationship_changes`, `project_exports_deny_actor_revoked_during_either_authority_wait`, real finalize/editor HTTP races in `exports::projects::check` |
| FR-010: current captured scope and both parent-lock race orders | `project_exports_refresh_release_scope_after_parent_wait`, `project_exports_retain_parent_authority_until_release_check_finishes`, `project_exports_release_every_captured_project_without_render_locks` |
| FR-017/018: coherent bounded payload and preservation | `project_exports_bound_rows_and_text_after_scope_filtering`, `project_exports_share_one_size_and_payload_snapshot`, existing privacy/format/filter/export regressions |
| FR-018 / SC-006 subset: cancellation, settings and real delivery | `project_exports_release_cancelled_and_timed_out_checks`, `project_exports_cancel_loading_without_retaining_authority`, registered real-cookie HTTP matrix and server suite |

RED reproduced invalid-actor acceptance. The corrected HTTP matrix passes
(12.71s), including actual finalize/editor writers; report regression passes
68 tests with zero failures and two existing manual exclusions (50.80s),
including all twelve project cases. Full server-binary regression passes:
964 passed, zero failed and 11 existing exclusions (327.07s). All four
adversarial findings were corrected and re-reviewed; scoped analysis has no
unmapped task or local consistency/constitution finding. Final verification and
publication are recorded in `progress.md`. Complete SQLx preparation passes
(1,281 descriptors, 33 new/two obsolete), as do corrected offline Clippy (2m12s),
WASM (21.70s), the twelve affected tests (12.64s), and formatting. Clippy's initial
failure was confined to redundant test dereferences and was fixed, not silenced.
This is not canonical policy, CSV or full-feature acceptance.

## Materialized export authorization (T104–T106)

Use the pinned Nix shell and disposable PostgreSQL only:

```sh
DATABASE_URL=postgres://horae@127.0.0.1:55416/horae_storage SQLX_OFFLINE=false CARGO_INCREMENTAL=0 cargo test -p horae --features server --bin horae materialized_exports --locked
DATABASE_URL=postgres://horae@127.0.0.1:55416/horae_storage SQLX_OFFLINE=false CARGO_INCREMENTAL=0 cargo test -p horae --features server --bin horae job_endpoints_enforce_session_role_and_organization --locked
```

| Requirement subset | Executable check |
| --- | --- |
| FR-006/007: trusted, current same-tenant actor | `materialized_exports_deny_revoked_managers`, `materialized_exports_require_same_tenant_active_actor`, real export HTTP sessions |
| FR-010: winning revocation and revision refresh | `materialized_exports_deny_winning_legacy_revocations`, `materialized_exports_refresh_after_access_revision_change` |
| FR-010/018: reader-first authority, cancellation and pool reuse | `materialized_exports_retain_authority_and_release_cancelled_reads`, `materialized_exports_release_rendered_body_when_final_check_is_interrupted` |
| FR-017/018: exact coherent checked payload, existing limits/deadlines | `materialized_exports_preserve_checked_snapshots`, `materialized_exports_keep_deadlines_and_restore_pool_defaults`, existing bounded exports and streaming regressions |
| FR-007/010: current authority after rendering, no locks during rendering | `materialized_exports_recheck_after_render_without_retaining_authority` |
| FR-018 / SC-006 subset: real delivery, non-disclosure and preservation | `authorization_tests::exports::check` plus full server-binary regressions |

RED reproduced entries still being returned after the fixture Manager was
demoted. Both authorization boundaries preserve the current Manager/Admin rule;
this is not canonical policy activation or complete CSV/Member-scope acceptance.
The final check authorizes response release, not continuous revocation while the
browser consumes bytes. Focused GREEN passes all nine tests (14.96s), and the
real-cookie HTTP matrix passes (12.66s). Full server-binary regression passes:
952 passed, zero failed and 11 existing exclusions (333.10s). Independent
adversarial review's fixture-source correction and final-check interruption
coverage are included in that passing run; no remaining local blocker was found.
Scoped analysis maps the five FR subsets and SC-006 regression subset to all
three tasks, with no unmapped task or local consistency/constitution finding.
Complete SQLx regeneration passes (1m52s), adding 15 descriptors with none
modified/deleted, 1,250 total. Offline all-targets Clippy (2m13s) and web/WASM
check (22.21s) pass with warnings denied. Final formatting, cleanup and
publication are recorded in `progress.md`. Full flake/browser and feature
acceptance remain open.

## Invoice editor snapshots (T101–T103)

Run in the pinned Nix shell against disposable PostgreSQL:

```sh
DATABASE_URL=postgres://horae@127.0.0.1:55416/horae_storage SQLX_OFFLINE=false CARGO_INCREMENTAL=0 cargo test -p horae --features server --bin horae snapshot_tests --locked
DATABASE_URL=postgres://horae@127.0.0.1:55416/horae_storage SQLX_OFFLINE=false CARGO_INCREMENTAL=0 cargo test -p horae --features server --bin horae --locked -- --quiet
```

| Requirement subset | Executable evidence |
| --- | --- |
| FR-006/007: current same-tenant active actor and session-derived identity | `editor_snapshots_preserve_rows_and_override_inherited_read_only` and editor cases in registered-route `authorization_tests::financial_snapshots::check` |
| FR-010: revocation wins or reader retains authority until materialization | `editor_snapshots_deny_winning_legacy_revocation`, `editor_snapshots_retain_authority_and_release_cancelled_reads` |
| FR-010/017: consistent metadata/revision/lines and no stale-edit rebasing | `editor_snapshots_refresh_after_revision_change_without_rebasing_edits`, `editor_snapshots_keep_invoice_metadata_revision_and_lines_together` |
| FR-017/018: unchanged business rows, inherited settings and pool reuse | Full-row preservation, inherited READ ONLY/SERIALIZABLE and single-connection assertions in the editor tests; existing shared-prelude retry/timeout tests |
| FR-018 / SC-006 subset: non-disclosing HTTP errors and financial regression | Registered HTTP checks for missing/foreign invoices, inactive/member sessions, forged identity, stale edits and non-draft conflicts; full server-binary suite |

RED reproduced `cannot execute SELECT FOR SHARE in a read-only transaction`.
The implementation replaces only the two reader preludes with the existing
helper; save/generation/status mutation guards and arithmetic are unchanged.
Focused GREEN passes all 14 snapshot tests (12.63s). Independent adversarial
review found no blocker or missing discriminating case. Cancellation proves
eventual rollback after the blocked query is released, not immediate query
cancellation. Full server-binary regression passes: 943 passed, zero failed,
11 existing exclusions (271.23s). Complete SQLx regeneration added seven
descriptors, with none modified/deleted (1m41s); offline all-targets Clippy
(1m52s) and web/WASM check (21s) pass with warnings denied. Formatting passes
with zero changes (3.668s). Scoped analysis maps all five FR subsets to the
three tasks with no unmapped task or local consistency/constitution finding.
Publication and cleanup results are recorded in `progress.md`.
This is not canonical policy activation or whole-feature acceptance.

## Materialized financial snapshots (T098–T100)

Run inside the Nix shell against the disposable PostgreSQL instance. No browser,
reference-account mutation, schema change or policy activation is required.

```sh
DATABASE_URL=postgres://horae@127.0.0.1:55416/horae_storage SQLX_OFFLINE=false CARGO_INCREMENTAL=0 cargo test -p horae --features server --bin horae financial_snapshots --locked
DATABASE_URL=postgres://horae@127.0.0.1:55416/horae_storage SQLX_OFFLINE=false CARGO_INCREMENTAL=0 cargo test -p horae --features server --bin horae job_endpoints_enforce_session_role_and_organization --locked
```

| Requirement subset | Executable evidence |
| --- | --- |
| FR-006/007: same-tenant active Manager/Admin, no request-supplied authority | `financial_snapshots_require_current_same_tenant_manager` and registered-route `authorization_tests::financial_snapshots::check` |
| FR-010: revocation before reading, including legacy writes without revision changes | `financial_snapshots_deny_winning_actor_revocation_without_revision_change`, both reader types and organization-gated/direct actor updates |
| FR-010/017: reader-first authority retention and consistent financial values | `financial_snapshots_hold_authority_until_materialization`, `financial_snapshots_keep_fee_values_from_their_initial_snapshot`, `financial_snapshots_retry_revision_change_and_refresh_amounts` |
| FR-018: bounded retry, non-disclosing errors and connection cleanup | `financial_snapshots_retry_only_serialization_failures_and_stop_after_three`, `financial_snapshots_override_isolation_but_restore_connection_settings`, `financial_snapshots_honor_stricter_lock_timeout_during_authorization`, `financial_snapshots_release_cancelled_reads_for_one_connection` |

RED reproduced an absent actor accepted by invoice preparation. Initial GREEN:
all eight snapshot tests passed (5.45s). The retry fixture uses a test-only view
and non-transactional sequence to count real attempts; production has no injected
test hook. Cancellation verifies eventual rollback and pool reuse after releasing
the blocked query, not instantaneous cancellation of PostgreSQL execution.

Independent adversarial review found no blocker; both suggested timeout checks
were added. The settings matrix includes all three inherited isolation levels
and 0/250ms/10s timeouts. Holding the actual organization gate verifies that the
stricter timeout applies during authorization, not only after it. Scoped Spec Kit
analysis maps all five FR subsets to T098–T100 with no unmapped task, ambiguity,
duplication or constitutional conflict. Full feature analysis remains open.

The existing invoice-transition test now checks the revoked actor's preview is
denied, then uses a separate same-tenant administrator to verify the reserved
balance is unchanged. Its focused rerun passed (1.15s). Final server-binary
regression passes: 938 passed, zero failed, 11 pre-existing exclusions (258.34s),
including the nine snapshot cases, HTTP matrix and existing financial fixtures.
The offline WASM check with warnings denied passed (43.95s). Complete SQLx
regeneration added 22 descriptors without modifying/deleting existing ones
(1m18s). Offline all-targets Clippy passes with warnings denied (1m41s). After
Clippy's equivalent boolean simplification, all nine snapshot tests passed
again using the offline cache (8.19s). Final formatting/publication results are
recorded in the progress register.
This increment preserves legacy Manager/Admin authority; it is not full scoped
policy, report streaming, Member export or invoice-editor acceptance.

## Own-permission Settings consumer (T120–T122)

Run in the pinned Nix shell without a database:

```sh
SQLX_OFFLINE=true CARGO_INCREMENTAL=0 cargo test -p horae --features server --test own_permissions_ui --locked
SQLX_OFFLINE=true cargo clippy -p horae --features server --all-targets --locked -- -D warnings
RUSTFLAGS="-D warnings" cargo check -p horae --features web --target wasm32-unknown-unknown --locked
nix fmt -- --ci
```

| Requirement subset | Executable evidence |
| --- | --- |
| FR-012: own explanation, exact grants and independent administrative identity | `own_permissions_view_preserves_exact_grants_without_profile_inference`, `own_permissions_view_admin_identity_is_explicit_not_derived_from_grants`, exhaustive distinct description test |
| FR-012/016: loading, legacy, empty, forbidden, error and recovery | Remaining `own_permissions_view_*` SSR tests and `refresh_uses_a_fresh_read_hides_stale_grants_and_recovers_from_errors` |
| FR-018: presentation disclosure and real resource wiring | Internal-field omission test, pending stale-content suppression and production-component refresh with controlled responses |

Nine SSR tests failed against the empty view, then passed against the
implementation. The final interaction binary passes 11 tests, zero failures or
exclusions (38.16s compilation). This uses the actual component/resource but
controlled server responses; it does not replace the existing endpoint's real
cookie/authorization tests or prove browser accessibility/visual acceptance.
Final lint/build/format results are recorded in `progress.md`.

Design boundary: reuse the handoff permission heading and administrator callout
with existing classes; preserve General/Plugins and the shell. Exact configured
grants replace the prototype's unbacked profile radios/update button. Counts
explain management relationships without fabricating names or exposing UUIDs.
No claim of full handoff alignment, Workspace integration or policy activation.
The saved-report labels describe inactive owners, not inactive report features.

## Authenticated own-access explanation (T095–T097)

Use the pinned Nix shell and the owned disposable PostgreSQL database; only
fixtures enable policy 1. No reference-account or deployment mutation is needed.

```sh
DATABASE_URL=postgres://horae@127.0.0.1:55416/horae_storage SQLX_OFFLINE=false CARGO_INCREMENTAL=0 cargo test -p horae --features server --bin horae own_permissions --locked
DATABASE_URL=postgres://horae@127.0.0.1:55416/horae_storage SQLX_OFFLINE=false CARGO_INCREMENTAL=0 cargo test -p horae --features server --bin horae job_endpoints_enforce_session_role_and_organization --locked
```

| Requirement subset | Executable evidence |
| --- | --- |
| FR-006/012: exact own grants and relationships, no implied directory access | `own_permissions_preserve_exact_grants_and_independent_identity`, `own_permissions_scope_is_sorted_and_excludes_membership_and_other_managers` |
| FR-007/017: preserve legacy policy, tenant/activity boundary and state | `own_permissions_legacy_mode_never_discloses_staging`, `own_permissions_deny_invalid_identity_policy_and_state`, unchanged-state snapshot in the scope test |
| FR-010: current authority across both race orders | `own_permissions_wait_for_current_actor_deactivation`, `own_permissions_reload_after_winning_gate_under_repeatable_read_defaults`, `own_permissions_reader_retains_actor_activity_until_snapshot_is_loaded` |
| FR-018: registered delivery, session-only selection and non-disclosing errors | `authorization_tests::own_permissions::check` within the existing HTTP matrix; absent/expired/inactive cookies, forged selectors, unsupported catalog and malformed grants |

RED: the legacy-mode test failed against the unavailable stub. Initial GREEN:
seven reader tests passed in 3.71s and the HTTP matrix passed in 9.11s. Independent
adversarial review found no blocker; its two-project ordering and HTTP catalog
error coverage suggestions were added before final regression.

Scoped Spec Kit analysis: six FRs above map to T095–T097, with no unmapped task,
local ambiguity, duplication or constitutional conflict. SC-002/003/006 gain
reader/HTTP evidence only; SC-005 and full US4 acceptance still need rendering
and browser checks. The general requirements checklist remains 12/16, and full
operation/migration/activation analysis is not declared complete. Final build,
regression, cache and formatting results belong in the progress register.

Final verification: all 929 server-binary tests passed, zero failed, 11
pre-existing exclusions (202.37s). Complete SQLx regeneration added 26 descriptors
without changing/deleting existing ones (51.20s); fresh offline all-targets server
Clippy with warnings denied passed (59.74s). The offline WASM build with warnings
denied passed (41.00s). Only the shared DTO carries a non-server dead-code
expectation for the pending T018 consumer; no global warning suppression exists.

This endpoint supplies display facts, never later request authority. It neither
replaces legacy guards nor completes other-person administration or the UI.

## Pure rate-field policy (T092–T094)

Run in the pinned Nix shell; no database or reference account is required:

```sh
cargo test -p horae-core permissions::rates --locked
cargo test -p horae-core --locked
cargo clippy -p horae-core --all-targets --locked -- -D warnings
```

`permissions::rates::tests` maps FR-021 to the action/owner table, cross-dimension
and grant-removal tests; FR-022 to independent costs and six-profile defaults;
FR-006/010 to foreign/inactive/provenance denial and changed grants/relationships;
FR-008 to every unrelated catalog grant, including combined report permissions.
The mixed all-read/managed-write case must allow unrelated reads but deny writes.
Read-prerequisite removal must deny both actions. These borrowed, I/O-free
checks preserve FR-017 by making no data or permission changes.

RED: five positive-behavior tests failed against deny-all stubs; two negative
tests passed. GREEN before the review addition: all 168 core tests passed, zero
failed/ignored; core all-targets Clippy passed with warnings denied. Independent
review found no blocker and requested the added prerequisite-revocation case.
After adding that case, all 169 core tests pass, zero failed/ignored (0.02s),
and all-targets core Clippy passes with warnings denied (0.62s). Final formatting
and publication results are recorded in the progress register.

This covers the financial dimension only. T014/T015 still require authenticated
consumer projections, independent resource checks, actual overrides/history and
transactional revocation; no endpoint or UI enforcement is claimed here. No SQL
or schema changed, so cache regeneration is unnecessary for this increment.

## Interrupted import cleanup (T089–T091)

Run against the owned disposable PostgreSQL instance in the Nix shell:

```sh
DATABASE_URL=postgres://horae@127.0.0.1:55416/horae_storage SQLX_OFFLINE=false CARGO_INCREMENTAL=0 cargo test -p horae --features server --bin horae import_cleanup --locked
DATABASE_URL=postgres://horae@127.0.0.1:55416/horae_storage SQLX_OFFLINE=false CARGO_INCREMENTAL=0 cargo test -p horae --features server --bin horae --locked -- --quiet
```

The focused fixture deliberately releases a server-side savepoint while keeping
SQLx's transaction objects alive, then drops the objects. This constructs the
pending rollback failure at depths two and three without depending on scheduler
timing. Require successful cleanup, unchanged previously committed content,
rolled-back current writes and immediate reservation acquisition with a
one-connection pool. Also exercise untracked server transactions and propagate
nonrecoverable backend errors. Full regressions must retain API/CSV producer
joins, cancellation, lease fencing, resume and preview guarantees.

| Requirement subset | Executable evidence |
| --- | --- |
| FR-007/010 cleanup prerequisite, FR-017 preservation | `import_cleanup_recovers_unacknowledged_savepoint_release` and `import_cleanup_rolls_back_an_untracked_server_transaction` |
| FR-018 explicit failure, no success on broken transport | `import_cleanup_does_not_hide_a_backend_failure` |
| SC-006 import subset: actual API producer join and committed pages | `durable_api_cancel_waits_for_the_producer_before_acknowledging`, `durable_api_cancellation_preserves_confirmed_pages_for_manual_retry` and the corresponding preview case |
| SC-006 import subset: actual CSV producer join and committed batches | `durable_csv_cancel_joins_parser_and_preserves_committed_rows`, `durable_csv_cancel_preserves_its_completed_batches_for_manual_retry` and the corresponding preview case |

RED reproduced the exact missing-savepoint cleanup error in 0.34 seconds.
After the shared correction, the three focused tests pass in 1.07 seconds.
Independent review found no critical/high production issue and corrected a
backend-termination fixture race: its timeout overload now confirms actual
termination before cleanup. Full server-binary verification after that correction
passes: 922 passed, zero failed, 11 pre-existing exclusions, 933 discovered,
189.83 seconds. This includes the actual adapter cancellation and preservation
regressions above, not just the constructed savepoint state.

Complete SQLx regeneration passes in 51.47 seconds with seven added descriptors
and no existing cache changes/deletions. Fresh offline all-targets Clippy passes
with warnings denied in 59.11 seconds. The disposable PostgreSQL instance is
stopped. Final Nix formatting passes with zero changes in 2.092 seconds;
T089–T091 are complete. No full-flake/browser acceptance is claimed.

This is disposal verification, not a timed driver-race reproduction, worker
permission activation, a library upgrade or permission to change real data.

## Original import requester (T086–T088)

Run in the Nix shell against the owned disposable PostgreSQL instance with
migrations through 0045. No real deployment migration or Harvest mutation is
needed. The upgrade fixture creates its own pre-0045 database.

```sh
DATABASE_URL=postgres://horae@127.0.0.1:55416/horae_storage SQLX_OFFLINE=false CARGO_INCREMENTAL=0 cargo test -p horae --features server --bin horae server_fns::importers:: --locked
```

| Requirement / boundary | Regression |
| --- | --- |
| FR-007/010 prerequisite: retain trusted original identity, not client authority | `new_import_jobs_record_the_authorized_original_requester` and HTTP `job_endpoints_enforce_session_role_and_organization` |
| FR-017: concurrent duplicate cannot replace the first actor | `waiting_duplicate_keeps_the_first_committed_requester` uses a real blocked enqueue |
| FR-017/018: preserve known or unknown attribution, independent of current actor activity | `duplicates_and_job_lifecycle_never_replace_or_invent_the_original_requester` covers second-admin replay, cancel/retry and claim/completion |
| FR-007/017: tenant-bound identity and no cascade/erasure | `original_requester_foreign_key_preserves_tenant_and_known_identity` |
| FR-017: populated old states, payload/checkpoint/report/lease and artifacts unchanged | `requester_migration_preserves_unknown_authors_in_all_job_states` checks upgrade and repeated migration |
| FR-017: earlier report conversion preserves metadata and cannot invent a requester | `legacy_upgrade_preserves_job_states_and_existing_archives` compares old fields and separately asserts NULL provenance |
| FR-017/018: conflicts, failed upload insertion and revocation remain atomic | Existing complete-row snapshot tests in `commands/tests.rs` |

The RED command test observed missing attribution before the column/insertion
change. Initial importer verification passes 22 tests with one existing stress
exclusion, 17.46 seconds. The HTTP fixture now actually sends forged CSV metadata
as query parameters, as well as forged JSON on the API path; SQL must still record
the session actor. Full server verification initially exposed an old metadata
comparison that included the new NULL field only after upgrade. It now compares
all old fields and explicitly checks NULL provenance. After correction, the full
server-binary suite passes: 919 passed, zero failed, 11 existing exclusions,
930 discovered, 195.85 seconds. This includes the concurrent duplicate case and
corrected HTTP fixture. The historical fixture checks storage preservation,
not execution of its intentionally synthetic payloads.

Complete SQLx regeneration passes in 57.02 seconds (four replaced descriptors,
nine additional test queries, no unrelated removal). Fresh offline all-targets
Clippy passes with warnings denied in 66 seconds. Scoped analysis maps all three
tasks to the stated requirement subsets; adversarial review finds no remaining
critical/high issue in this increment. Full-flake and browser acceptance are
not claimed for this storage-only change.

Nix formatting passes with zero changes in 2.908 seconds; the owned disposable
PostgreSQL cluster is stopped. T086–T088 are complete.

This increment does not activate current-authority worker checks, choose retry
delegation or resolve unknown historical requesters. Original identity is not a
stored authorization grant and remains absent from external job DTOs.

## Durable CSV preparation (T083–T085)

Run in the Nix shell against the owned disposable PostgreSQL instance:

```sh
DATABASE_URL=postgres://horae@127.0.0.1:55416/horae_storage SQLX_OFFLINE=false CARGO_INCREMENTAL=0 cargo test -p horae --features server --bin horae importers::harvest::engine_tests::csv_streaming:: --locked
```

| Requirement / boundary | Regression |
| --- | --- |
| FR-007/010 prerequisite: no transaction across durable input waits | `durable_csv_waits_for_input_without_an_open_transaction`: first/subsequent batch, Commit/DryRun, exact reserved backend PID |
| FR-017: incomplete prepared input cannot advance domain or checkpoint state | `durable_csv_discards_incomplete_preparation_and_resumes_its_checkpoint`: transport failure before/after checkpoint, both modes, exact resumed counts |
| FR-017/018: cancellation and expired lease cannot publish an applied batch | `durable_csv_cancel_joins_parser_and_preserves_committed_rows`, `durable_csv_stale_commit_is_fenced_without_a_heartbeat`: SQL job-row barriers before publication |
| FR-017/018: absolute offsets, row-error boundaries and preview recovery | Existing six `interrupted_csv_batches` cases and single-connection preview tests |
| FR-017/018 and SC-006 import subset: reports, legacy adapter and SQL backpressure | Existing report-size, whole-run rollback and backpressure regressions |

The initial RED observed a live transaction during a parser wait. The bounded
preparation collects at most 500 normalized rows, not a fixed-byte memory budget.
It neither changes accepted upload size nor collects the whole source. The first
post-change run exposed a duplicate-email fixture and two old tests waiting for
one-row application before EOF. Unique emails and explicit publication barriers
retain their original denial/rollback assertions under the prepared-batch flow.
The focused suite passed 18 tests with four existing stress exclusions. The
adversarial follow-up found a possible fixture-only SKIP LOCKED reclaim race;
wait for the expired attempt's cleanup before reclaiming. Separate token-reclaim
tests retain concurrent coverage. The full server-binary run after this correction
passes: 914 passed, zero failed, 11 pre-existing exclusions, 925 discovered,
174.14 seconds. Complete SQLx regeneration passes in 44.08 seconds with one added
test query and no existing cache changes/deletions. Fresh offline all-targets
Clippy passes with warnings denied in 51.62 seconds. Nix formatting passes with
zero changes in 2.752 seconds; the owned PostgreSQL cluster is stopped. No
critical/high review finding remains in this increment. This is not worker
permission activation, browser/full-flake verification or full feature acceptance.

## Bounded import error downloads (T080–T082)

Use the Nix shell and owned disposable PostgreSQL instance. No real account or
Harvest data is changed. The focused stream tests call the production response
builder/body; the importer matrix uses registered HTTP and remote CLI paths.

```sh
DATABASE_URL=postgres://horae@127.0.0.1:55416/horae_storage SQLX_OFFLINE=false CARGO_INCREMENTAL=0 cargo test -p horae --features server --bin horae jobs::report::stream_tests:: --locked
DATABASE_URL=postgres://horae@127.0.0.1:55416/horae_storage SQLX_OFFLINE=false CARGO_INCREMENTAL=0 cargo test -p horae --features server --bin horae server_fns::importers:: --locked
```

| Requirement / boundary | Regression |
| --- | --- |
| FR-007/010: current authority before report preparation | `download_preparation_denies_revoked_authority` |
| FR-010: recheck before first page, captured tail and empty EOF | `download_denies_revocation_before_first_page_and_inline_tail` |
| FR-010: only the already authorized bounded page may drain | `download_rechecks_after_buffered_pages_and_before_the_captured_tail` |
| FR-007/018: tenant-bound actor/job, malformed metadata and single-connection rollback | `download_preparation_is_tenant_bound_and_releases_failed_transactions` |
| FR-010: current authority after organization/actor waits under inherited REPEATABLE READ | `download_boundaries_recheck_after_organization_and_actor_waits` |
| FR-010: reader retains authority until the bounded read completes | `download_readers_retain_authority_through_their_bounded_reads` |
| FR-017: append-only worker progress cannot alter the captured response | `download_keeps_its_snapshot_when_later_progress_archives_the_inline_tail` |
| FR-017/018: missing retained fragments fail instead of truncated success | `download_reports_missing_fragments_after_its_buffered_page` |
| FR-017: lazy 16-fragment buffering with no connection held for client consumption | `download_reads_only_when_consumed_and_releases_its_connection` |
| FR-007/010/018: registered HTTP abort after preparation-time revocation; headers and CLI preserved | `job_endpoints_enforce_session_role_and_organization` |

Initial RED reproduced preparation/tail disclosure. Expanded verification found
one malformed-report fixture targeting the wrong source: status prefers the
checkpoint. The fixture now clears that checkpoint before corrupting the final
report. The full post-fix server-binary run passes: 912 passed, zero failed,
11 pre-existing exclusions, 923 discovered, 176.57 seconds. It includes all nine
stream tests, the HTTP abort case and existing importer/CLI checks. Complete SQLx
regeneration passes in 43.26 seconds: three new test-query descriptions, no
existing cache changes/deletions. Fresh offline all-targets Clippy passes with
warnings denied in 52.05 seconds. Nix formatting passes with zero changes in
2.139 seconds after Rust/Markdown formatting. The owned test cluster is stopped.
No worker authority, canonical policy activation, browser, full flake or complete
feature acceptance is claimed.

## Import job control and status authority (T077–T079)

Run in the Nix shell with the owned disposable PostgreSQL instance. The tests
invoke the production helpers and registered HTTP/remote CLI surface, not a copy
of the authorization SQL. No live Harvest mutation is required.

```sh
DATABASE_URL=postgres://horae@127.0.0.1:55416/horae_storage SQLX_OFFLINE=false CARGO_INCREMENTAL=0 cargo test -p horae --features server --bin horae server_fns::importers:: --locked
```

| Requirement / boundary | Regression |
| --- | --- |
| FR-007/010: active tenant-bound Administrator on all six operations | `commands_and_status_require_current_active_tenant_administrator` |
| FR-010: replay, malformed upload and no-op paths cannot bypass revocation | `duplicate_submission_and_noop_cancellation_do_not_bypass_revocation` |
| FR-010: fresh checks after organization/actor waits, including inherited REPEATABLE READ | `import_access_rechecks_revocation_after_organization_and_actor_waits` |
| FR-010: command and result remain authorized through commit | `import_commands_and_results_hold_actor_authority_until_commit` |
| FR-007/018: no foreign/missing-record disclosure or writes | `foreign_and_missing_jobs_never_reveal_or_change_another_tenant` |
| FR-017: retained state, idempotency and single-connection operation | `authorized_job_lifecycle_and_duplicate_retention_work_with_one_connection` |
| FR-017: late failure rolls back job/upload and releases locks | `failed_upload_insert_rolls_back_the_job_and_releases_authority` |
| FR-017: payload conflicts and connection-generation fencing | `conflicting_and_stale_commands_preserve_payload_policy_and_retained_state` |
| FR-017: submission can queue while an import owns its reservation | `authorized_submissions_do_not_wait_for_the_running_import_reservation` |
| FR-007/010/018: registered HTTP rejects revocation during body reading; existing CLI remains covered | `job_endpoints_enforce_session_role_and_organization` |

The initial RED reproduced API and duplicate-CSV acceptance after demotion.
Initial importer verification passed 11 tests with one existing stress exclusion;
the expanded run passed 17 with the same exclusion. Final server-binary regressions
after the reservation and malformed-CSV additions pass: 906 passed, zero failed,
11 pre-existing exclusions, 917 discovered, 172.52 seconds. Complete SQLx
regeneration passes in 43.97 seconds: four new test-query descriptions, no existing
cache changes or deletions. Fresh offline all-targets Clippy passes with warnings
denied in 51.89 seconds. Nix formatting passes with zero changes in 2.076 seconds
after applying Markdown spacing. The owned test cluster is stopped. No full-feature acceptance,
worker execution authority, report-download reauthorization, browser or flake
result follows from this increment.

## Harvest connection transaction authority (T074–T076)

Use the Nix shell and owned disposable database; SQLx creates an isolated database
per test. No external Harvest mutation or real-data migration is required.

```sh
DATABASE_URL=postgres://horae@127.0.0.1:55416/horae_storage SQLX_OFFLINE=false cargo test -p horae --features server --bin horae credentials::authority_tests --locked
```

| Boundary | Production-writer regression |
| --- | --- |
| Active same-org Administrator; Member/Manager/inactive/foreign/missing denial for all three writers | `connection_changes_require_current_tenant_bound_administrator` |
| Repeated disconnect still needs authority; no generation side effect | `disconnected_noop_still_requires_authority` |
| Denied first connect and missing organization produce no connection state | `denied_first_connection_creates_no_binding_credentials_or_generation` |
| Fresh authority after organization wait with an inherited REPEATABLE READ default | `connection_changes_recheck_authority_after_organization_wait` |
| Demotion/deactivation during actor-only wait | `connection_changes_recheck_authority_after_actor_only_wait` |
| Authorized writer finishes before concurrent revocation; next request denied | `connection_changes_hold_authority_until_commit` |
| Late revision-write failure rolls back secrets/binding/generation and releases locks/reservation for retry | `failed_connection_changes_roll_back_and_release_authority_and_reservation` |
| Safe forbidden response without private error context | `revoked_callback_authority_returns_only_a_safe_forbidden_message`, `revoked_connection_authority_maps_to_forbidden_without_private_context` |

RED reproduced successful connect/disconnect by an unauthorized Member. The first
focused run passed 20/21; the failing test was its temporary rollback constraint
validating earlier fixture rows. NOT VALID restricts only subsequent writes.
After correction, the Harvest-filtered suite passes: 204 passed, zero failed,
8 existing scale-test exclusions, 34.52 seconds. After Rust formatting the full
server binary suite passes: 897 passed, zero failed, 11 pre-existing exclusions,
908 discovered, 164.51 seconds. This includes the server-function forbidden-error
mapping and authenticated importer-route tests. Complete nonincremental SQLx
regeneration adds eight test-query descriptions and removes only the obsolete
post-HTTP authority query (`3e9078ea…`); no other existing description changes.
Fresh offline all-targets Clippy passes with warnings denied in 50.24 seconds.
Nix formatting initially inserted a blank line in the progress log; the repeated
CI check passes with zero changes in 2.214 seconds. The owned test cluster is
stopped. No separate integration binary, browser or full-flake result is claimed.

The session actor comes from existing authenticated server wrappers or the
validated OAuth attempt. The guard reuses organization SHARE, explicit READ
COMMITTED and actor SHARE before generation locks, retaining authority through
commit. Reservation and HTTP ordering remain unchanged. Legacy Administrator
semantics remain active; this does not activate the six-profile system or finish
service import/refresh authorization, full OP28/T042, browser or full-flake gates.

## Branding transaction authority (T071–T073)

Run through the Nix shell against the owned disposable compilation database;
SQLx creates a separate database for each test. No real account or schema change
is required. The existing Manager/Admin policy remains active.

```sh
DATABASE_URL=postgres://horae@127.0.0.1:55416/horae_storage SQLX_OFFLINE=false CARGO_INCREMENTAL=0 cargo test -p horae --features server --bin horae server_fns::organization::tests:: --locked
```

| Boundary | Production-helper regression |
| --- | --- |
| Active same-tenant authority before both change and no-op disclosure | `branding_requires_current_tenant_bound_authority_even_for_noops` |
| Revocation after organization wait, even with a REPEATABLE READ connection default | `branding_rechecks_authority_after_the_organization_wait` |
| Demotion/deactivation committed during actor-row wait | `branding_rechecks_authority_after_an_actor_only_wait` |
| Writer retains authority until commit; later request denied | `branding_holds_actor_authority_until_its_write_commits` |
| Failed UPDATE preserves row, releases locks and permits a later valid change/no-op | `failed_branding_write_rolls_back_and_releases_authority` |
| Every branding field, NULL/empty values and competing no-op behavior | Five existing organization tests |

RED: the first two tests returned successful branding to unauthorized callers,
including after observed concurrent demotion. GREEN: all ten organization tests
passed. The final rollback probe uses a bounded wait, not a scheduling-dependent
NOWAIT assertion, because SQLx transaction drop queues the rollback.

Focused self-review traced the sole authenticated wrapper, organization-first
UPDATE, current actor SHARE, explicit READ COMMITTED, all commit/error paths and
post-commit event dispatch. No public signature, CSS, dependency, profile mapping,
business state or new-policy activation changed. These tests use real helper SQL
and observed PostgreSQL blockers, not a duplicate implementation or timed sleeps.
Revocation fixtures update the actor directly; this is not an end-to-end browser
or complete cross-command acceptance claim. Full regression/cache/Clippy/formatting
results are recorded after execution below; full T042 and OP27 remain open.

Full server binary verification after the rollback-test hardening: 888 passed,
zero failed and 11 pre-existing ignored in 180.24 seconds. All 161 core tests
pass; formatting CI passes with zero changes. No separate integration-binary,
browser or full-flake run is claimed for this bounded repair.

Complete non-incremental SQLx preparation adds three test-query descriptions and
changes/deletes no existing cache entry. Fresh offline all-targets Clippy passes
with warnings denied (50.43 seconds). Cleaning removed only 1.5 GiB of regenerable
package artifacts. Bounded adversarial self-review found no remaining critical/
high defect in this repair; it is not independent full-feature acceptance.

## Legacy report lock integration (T065–T067)

Run in the Nix shell against the owned disposable PostgreSQL compilation DB:

```sh
CARGO_INCREMENTAL=0 cargo test -p horae --features server --bin horae jobs::report:: --locked
CARGO_INCREMENTAL=0 cargo test -p horae --features server --bin horae --locked
```

New production-path concurrency coverage in `jobs/report/tests.rs`:

- `legacy_upgrade_preserves_a_concurrently_archived_workers_lease`: organization-first
  worker versus converter; preserve the exact saved checkpoint, claim and archive.
- `concurrent_legacy_converters_archive_each_error_once`: both discover before
  the organization gate opens; job recheck prevents duplicate conversion/chunks.
- `legacy_converter_rediscovers_after_candidate_deletion`: remove the discovered
  job while waiting and continue with another organization's oversized report.
- `legacy_converter_uses_replaced_payload_after_waiting`: convert the current
  replacement with a connection default of REPEATABLE READ and a size-one pool.
- `legacy_converter_gate_allows_a_job_locked_workers_chunk_foreign_keys`: an
  ungated worker holding the job can obtain organization FK KEY SHARE and finish
  while the converter holds SHARE and waits; its bounded checkpoint keeps its claim.

Existing migration fixtures cover bounded paging, oversized Unicode errors,
rollback of chunk inserts, lease invalidation for actual conversions, all job
states, existing archive prefixes, size-one pools and repeated startup. These
tests do not prove the full permission hierarchy or authorize policy activation.

RED: the first test failed with PostgreSQL `deadlock detected` before the repair.
The first focused run after repair passed that case and the simultaneous/replaced
cases; one deletion assertion incorrectly used a current-schema status reader on
the pre-0028 fixture. Replaced that assertion with a tenant-scoped existence query,
without changing production behavior or weakening archive checks.

Focused verification now passes 11/11, including all five new races. Local
adversarial review traced the one-connection lifetime, exact tenant/predicate
recheck, compatible chunk FK mode and commit-before-rediscovery paths. Tests use
observed PostgreSQL blockers, not sleeps; JoinSet aborts remaining test tasks on
failure. Existing malformed/archive rollback and lease-fencing cases still pass.
No critical/high finding remains in this bounded repair. This is self-review
supported by earlier independent design research, not independent code review or
complete T042 acceptance. No browser, separate integration binary or full flake
run is claimed by these results.

Final verification: 878 server binary tests passed, zero failed and 11 pre-existing
ignored (185.13 seconds); all 161 core tests passed. Complete non-incremental SQLx
preparation added 11 descriptions and replaced only the obsolete discovery query
(one deletion). Fresh offline all-targets Clippy passed with warnings denied.
Formatting passed with zero changes. The package clean removed only 1.4 GiB of
regenerable worktree artifacts. No schema or real data was changed.

## Foundation (no database)

From this worktree:

```sh
nix develop --command cargo test -p horae-core permissions
nix develop --command cargo test -p horae-core
nix develop --command cargo clippy -p horae-core --all-targets -- -D warnings
nix fmt -- --ci
```

The evaluator is not connected to application authorization yet. A green core suite is not evidence of six-profile or approval parity.

### Foundation evidence (2026-09-30)

- RED: the focused test command failed with unresolved scope types before implementation (exit 101).
- GREEN: `cargo test -p horae-core` through the Nix shell passed all 121 tests, including six new permission tests. The coverage table exercises 16 scope combinations against 12 resource cases.
- `cargo clippy -p horae-core --all-targets -- -D warnings` through the Nix shell passed.
- `nix fmt -- --ci` passed after normalizing Markdown code fences; `git diff --check` passed.
- No database, browser, application integration or full-flake validation is claimed for this isolated foundation. Runtime guards, role types and schema are unchanged.

### Incremental analysis

The foundation contract covers FR-006 record union, identity activation and organization isolation, with explicit limits for capability resolution, locks and freshness. No foundation contract inconsistency was found. Full-feature gaps remain: FR-002's operation matrix, FR-015's exact custom dependency/lifecycle contract, FR-019's remaining reference edge cases, and FR-014's reviewed migration/governance. All twenty functional requirements map to the required later work packages in `tasks.md`, but those packages are not yet a complete executable runtime plan. The requirements checklist remains 12/16; no full-story or outcome acceptance is claimed.

## Full-feature acceptance (pending implementation)

### Person-management prerequisites — verified 2026-10-03

T050–T052 implement `contracts/person-management-validation.md`. Run from this
worktree:

```sh
nix develop --command cargo test -p horae-core permissions::person_management --locked
nix develop --command cargo test -p horae-core --locked
nix develop --command cargo clippy -p horae-core --all-targets -- -D warnings
nix fmt -- --ci
```

- RED: focused command failed with unresolved new functions/error (exit 101).
- GREEN: seven new tests pass; all 159 core tests pass. The fixtures enumerate
  all 50 grants and 2,500 ordered pairs against an independent 19-grant expected
  set; the own-work floor and combined unrelated grants do not qualify.
- Loss/restoration uses the real selection edit methods; a remaining read-only
  grant suffices. Self-links fail alone or at any position in a mixed proposal.
  Empty removals and other-person sets pass the identity prerequisite. Borrowed
  selections and ID lists remain unchanged; no relationship is implicitly changed.
- Mutation checks: excluding `ApprovalWithdrawManaged` caused three failures;
  checking only the first proposed person caused the self-link test to fail.
  Both mutations were restored before the final successful suite.
- Core/all-targets Clippy with warnings denied and Nix formatting pass. Focused
  adversarial self-review checked false authority inference, caller/manager
  identity confusion, removal eligibility, full-batch rejection and the complete
  grant classification. Renamed the identity helper to
  `validate_no_self_management` so its name does not imply complete validation.
  An exhaustive match forces new catalog variants to be classified explicitly.
- No new dependencies, schema, server consumer, legacy guard or real data changed.
  No PostgreSQL/browser/full-flake or complete US2 result is claimed. Current
  authority, tenancy, revision, confirmation and atomic audit remain server work.

### Planned persistence/transaction fixtures

Cases in [permission-state.md](contracts/permission-state.md#executable-acceptance-cases-to-add-after-the-gates)
map to T035–T041. They are not implemented or passed. After the gates and test
modules exist, use isolated PostgreSQL and the Nix dev shell:

```sh
cargo test -p horae --features server --bin horae server_fns::permissions:: --locked
cargo test -p horae --features server --test integration --locked
```

A filter matching zero tests is not success: check that storage, changes and
audit cases ran. Verify two organizations, explicit administrative identity,
grant-equivalent custom profiles, stale revisions, both revoke/write orders,
rollback and replay disclosure. Full acceptance also requires the operation
matrix, business regressions and Nix gate; these commands alone are insufficient.

### Confirmed grant catalog evidence (2026-10-02)

Run `nix develop --command cargo test -p horae-core permissions::catalog --locked`
for the focused increment. The same full-core and Clippy commands above apply.

- RED: initial tests failed with unresolved catalog/profile/selection types (101).
- GREEN: all 143 core tests pass, including 22 new catalog tests and the six
  existing scope tests. Direct defaults are independently enumerated for all six
  profiles; all 50 grants, 33 prerequisite-bearing nodes and 2,500 ordered grant
  pairs are covered. Unknown wire names, floor removal and unrelated escalation
  are checked without adding a test dependency.
- Mutation check: temporarily removing the managed-invoice → draft-write edge
  caused three tests to fail (catalog completeness, exact prerequisite graph and
  dependent removal). Restored the edge and reran all 143 tests successfully.
- Core/all-targets Clippy with warnings denied and full `nix fmt -- --ci` passed.
  Focused adversarial review checked unknown grants, floor/prerequisite invariants,
  finite graph traversal, separate profile identity and lack of runtime consumers.
- No new dependency, schema, SQLx cache, legacy role conversion or runtime guard
  changes. No browser/DB/full-flake or cross-surface authorization result is claimed.
  Passing selection tests is not proof of administrative identity or persisted
  access enforcement. T006–T020 and the full feature remain open.

### Strict saved-grant restoration evidence (2026-10-02)

- RED: the nine new `permissions::catalog::stored_tests` failed to compile before
  the restoration API existed (exit 101).
- GREEN: all 152 core tests pass. New checks cover six built-ins, custom grants,
  changed/revoked selections, arbitrary input order, unsupported versions, every
  missing floor/dependency and every duplicated catalog grant. The pair exercise
  covers 2,500 ordered grant pairs and their valid removal outcomes.
- Mutation check: bypassing the prerequisite-closure check made the negative test
  fail on `TimeWriteManaged` without `TimeReadManaged`. Restored the check and
  reran all 152 tests successfully.
- Core/all-targets Clippy with warnings denied passed. Formatting and diff
  whitespace checks passed. Self-review verified no editor normalization in the
  loader, no default-profile lookup and no runtime callers. No independent-agent
  review is claimed for this increment.
- No schema, SQLx query/cache, app authorization, data or dependency changes.
  T035/T036 and full feature acceptance remain open; this is a pure structural
  boundary, not authenticated PostgreSQL loading or new-policy enforcement.

Run with `nix develop --command cargo test -p horae-core permissions::catalog::stored_tests --locked`.

### Remaining full-feature scenarios

For FR-023 use the acceptance matrix in [company locks](contracts/company-locks.md)
and T043–T046. Verification uses disposable data and an injected clock, not a
real lock or settings change in Harvest. Include retained protections and
concurrent disable/reconfigure cases; owner-visible controls alone are not a pass.

The [current access inventory](contracts/current-access.md), [Harvest evidence register](contracts/harvest-evidence.md) and [observed profiles](contracts/reference-profiles.md) now guide the remaining checks. Browser configuration/source inspection is not a substitute for saved-permission enforcement tests.

1. Complete the documented/observed parity matrix and confirm custom dependencies using disposable Harvest fixtures.
1. Exercise each profile and custom configuration through direct server calls, screens, exports, API, jobs and downloads; verify redacted payloads and cross-organization denial.
1. Verify approved C02 locally with report-only, ordinary-rate-only, both and neither configurations. Report-only access includes the report's defined financial fields and corresponding exports within its scope, not direct rate/history/source access or edits. Rate-only access does not open financial reports. Check unrelated report families, forged report selectors, foreign/out-of-scope rows and revocation before generation/download. This is an approved Horae rule, not a claim of restricted-user Harvest verification.
1. Revoke authority between preview, execution and download; race revocation against mutation and concurrent administrator changes.
1. Verify C04/FR-022: Accounting/Executive read organization costs but cannot write; Administrator has both; other unchanged built-ins have neither. Exercise custom grants and per-person adjustments, cost history and supported project overrides, independent resource checks, revocation and foreign organizations. Cost read must not grant unrelated person/project writes, and report-only amounts retain FR-008's separate boundary.
1. Verify approved C03/FR-021 with managed person B, unrelated person C, managed project P and unmanaged Q. General person rates follow person management; project/person/task overrides follow project management, with independent read/write rate grants. Check inherited-rate display without unrelated history, global task defaults, read-only denial, loss of either grant or relationship and foreign organizations. A permitted person-default change may flow to inheriting projects but must not change overrides or expose unauthorized project identities.
1. Submit mixed-project dates, approve only A, verify B remains pending and approved empty cells reject new entries. Exercise filters, another approver, self-approval settings and scoped/whole-week withdrawal against independent locks.
1. Verify approved C06/FR-024 using every case in [approval visibility](contracts/approval-visibility.md): deny the whole approval if any selected time/expense record is unreadable, without private error details or silent time-only filtering. Check authorized combined approval, absent approval authority, expense-free selections and revocation/new-record races. Feature 016 expense fixtures are required; do not claim combined acceptance from current time-only tests or infer withdrawal behavior.
1. Verify the approved C01 deletion behavior locally: unchanged effective grants/scope for ordinary and individually adjusted assignees, person-specific configuration after detachment, deleted template unavailable for new applications, confirmation explaining preservation and cancellation without writes. Verify migration preserves records and import/identity linking never overwrites privileges. Actual Harvest deletion remains unverified; the local contract is a user-approved decision.
1. Verify Settings/Workspace themes, keyboard, narrow/short viewports and enlarged text; run database integration tests and the full flake gate before merge.
1. Verify C07/FR-025 with existing project-manager designations and managed/all project reads: removing editing retains the designation without restoring editing; removing project read previews losses and requires confirmation. Exercise every retention case in `contracts/current-account-investigation.md`, including cancel, stale/direct requests, audit, unchanged membership/history and independent scopes. This does not validate new-assignment/promotion authority or Harvest enforcement.
1. Verify FR-026 separately: project editors can add/remove manager designations only within authorized projects; read-only actors cannot. Add to compatible managed-read targets with no prior designation and all-project readers; deny incompatible, foreign/inactive targets. Remove without requiring the target's read grants. Check direct/editor and multi-person saves, authority/eligibility races, no partial writes, scope-only audit, no full target-grant disclosure and no changes to profiles, global grants, membership or history. These are local acceptance obligations, not completed tests or proof of Harvest custom-profile enforcement.

## Independent approval tenant-isolation repair

Run PostgreSQL with the repository development stack and migrations applied. Tests use SQLx-created throwaway databases; the database role needs `CREATEDB`.

```sh
nix develop
process-compose up postgres migrate
# In a second development shell, with DATABASE_URL pointing to that stack:
cargo test -p horae --features server --bin horae server_fns::approvals:: -- --nocapture
cargo sqlx prepare --workspace -- --features server --all-targets
SQLX_OFFLINE=true cargo clippy -p horae --features server --all-targets -- -D warnings
```

Evidence from 2026-09-30:

- Tests invoked the actual approval/reopen transactions. Before adding tenant filters, three isolation tests failed: foreign approval returned, bulk returned two tenants, and foreign reopening succeeded. The same-org invoice-lock preservation case passed.
- After the correction, all 14 approval-module tests passed, including existing submission concurrency tests and four new isolation tests.
- SQLx preparation completed against a separate development database on port 55415. Generated cache entries replace the obsolete query hashes; no application data or migrations changed.
- Server/all-targets Clippy passed with `SQLX_OFFLINE=true` and warnings denied. Formatting and diff whitespace checks passed.
- Existing same-org whole-week semantics and bulk skip/count behavior are retained. Scoped project/date approvals, atomic full-selection policy and transactional permission revocation remain pending; this repair does not claim those outcomes.

The browser preview on port 8092 and the user's Harvest account were not changed by these database tests. The whole feature remains draft; no full-flake or end-to-end permission acceptance is claimed here.

## Person-management writer authority acceptance

After the full implementation gates pass, verify FR-027 through the UI and direct
requests: an active same-org Administrator can commit valid add/remove/replace
operations; People Admin, Executive Manager and custom all-people writers cannot,
including when editing their own managed-person set. Revoke administrative status
between preview and save and change relationship revisions concurrently; no stale
or partially authorized batch may commit. Check scope-change audit and unchanged
profiles, global grants, project membership and historical work. Ordinary permitted
person editing and own-access explanations remain available to non-administrators.
FR-028 now settles new-assignment grant eligibility. Test each compatible family
(including read-only time/expense/people/billable access and scoped withdrawal),
the corresponding organization-wide grants, and custom combinations without
people-directory/edit access. Reject own-only and unrelated-only grants, including
mixed replacement batches with invalid new relationships. Recheck after concurrent
grant removal; do not add privileges or mutate history. The first assignment must
not require a pre-existing managed-person set. For FR-029 test retention with any
compatible grant; last-grant loss with confirmed atomic removal; cancel, missing
confirmation, stale/changed affected sets, revoked actor and audit/write failure;
preserved people, incoming relationships, project memberships and history; and
no automatic restoration when grants return. Test simultaneous FR-025/029 losses:
confirm both affected sets and commit all changes atomically, preserving tracking
membership and history. For FR-031 reject direct and mixed-batch self-links even
for Administrators with compatible grants; preserve own/all access and allow
otherwise valid edits to an Administrator's own set of other people. Do not infer
self-approval or delete historical data. These are test obligations, not executed
tests or Harvest parity proof.

## Planned keep-project-access acceptance

Before accepting the permission editor, run every FR-030 case in
[keep-project-access](contracts/keep-project-access.md) against production
permission and assignment commands and the shared browser flow. These are pending
tests, not proof of Harvest persistence or of implemented Horae behavior.

## Independent assignment isolation and revocation repair

Using the same isolated PostgreSQL stack:

```sh
cargo test -p horae --features server --bin horae server_fns::projects::assignment_tests::
cargo test -p horae --features server --bin horae server_fns::projects::
CARGO_INCREMENTAL=0 cargo sqlx prepare --workspace -- --features server --all-targets
SQLX_OFFLINE=true cargo clippy -p horae --features server --all-targets -- -D warnings
```

Evidence from 2026-09-30:

- RED: four tests failed against the extracted production SQL: foreign assignment creation, foreign removal, stale administrator admission and failure to wait for concurrent revocation. The same-org lifecycle test passed before the fix.
- GREEN: all five assignment tests passed; all 63 project-module tests then passed, including existing privacy, bulk-action and lifecycle regressions.
- Complete SQLx regeneration passed: five new cache entries replace one obsolete query. Server/all-targets Clippy passed with `SQLX_OFFLINE=true` and warnings denied; `nix fmt -- --ci` and `git diff --check` passed.
- Creation validates both project and person organization. Removal returns no foreign/malformed assignment data and preserves those rows. Missing/repeated removal remains an idempotent no-op for an authorized administrator.
- Both mutations reload and lock the active administrator until transaction commit. The concurrency test observes an actual PostgreSQL lock dependency, commits demotion and checks that the waiting mutation fails with `FORBIDDEN`; it does not rely on sleeps.
- No profile, schema, CSS or existing data migration is included. Current administrator-only assignment authority is preserved. Full assignment parity, schema provenance, other entry-point revocation and HTTP/plugin end-to-end checks remain separate work.

The initial incremental SQLx preparation omitted cached integration-test queries even with `--all-targets`. Regeneration disables incremental compilation so the complete target set emits query metadata; do not commit the incomplete intermediate cache.

## User-administration transactional revocation

Using the isolated PostgreSQL stack on port 55415, through the Nix dev shell:

```sh
cargo test -p horae --features server --bin horae server_fns::users::tests:: --locked
cargo test -p horae --features server --bin horae --locked
cargo test -p horae-core --locked
CARGO_INCREMENTAL=0 cargo sqlx prepare --workspace -- --features server --all-targets
SQLX_OFFLINE=true cargo clippy -p horae --features server --all-targets --locked -- -D warnings
```

Evidence from 2026-10-02:

- RED: four negative tests failed against the extracted production helpers;
  revoked/foreign/unknown actors and concurrent revocations could still create
  users. Nine existing/positive checks passed. Each negative test now exercises
  creation, role changes and deactivation, not just one mutation path.
- GREEN: the initial 13 user tests passed. Added rollback/lock-lifetime cases;
  the full server binary suite passed 795 tests with 11 pre-existing manual
  measurement tests ignored, including all 15 user checks. All 143 core tests
  passed. This is not the separate integration binary or full Nix flake suite.
- Creation, role and activation helpers take actor IDs from the authenticated
  wrapper, serialize on the organization, reload active administrator authority
  and retain its row lock through commit. Revocation is checked after waiting
  for either organization or actor locks; foreign/unknown actors fail without
  changes. Concurrent self-demotions/deactivations retain one active admin.
- Tests observe actual PostgreSQL blocking relationships, not sleeps. A separate
  case proves revocation waits until the access transaction commits. Duplicate
  creation rolls back and releases locks so a subsequent authorized change works.
- Complete SQLx regeneration adds only two test-query cache files and removes
  none. The production actor-lock query already exists in the cache. No schema,
  UI, CSS, legacy-role mapping or real account data changed.
- Server/all-targets offline Clippy and full formatting passed. Clippy's initial
  redundant-dereference finding in a test was corrected, not suppressed.
- Focused adversarial review traced all three authenticated wrappers and helpers,
  organization-before-actor locking, commit/error paths and event dispatch after
  successful commit. No unresolved high/critical finding in this increment.
  Durable audit, stale-form revisions, six-profile runtime enforcement and the
  complete operation matrix remain pending; this is not full SC-003 acceptance.

## Operation/lock-contract checks — 2026-10-02

At `b7e730c`, a source-to-document check found all 80 public async symbols under
`crates/horae/src/server_fns/` (excluding test fixtures) in
`contracts/operation-matrix.md`. Authentication, exports, the compatibility API,
jobs, plugins and operator paths are separately listed. This checks inventory
coverage, not whether an operation's target predicate is settled or enforced.

The isolated development PostgreSQL on port 55415 was reachable. Through the
Nix shell, this diagnostic against its existing organization table:

```sh
nix develop --command psql -X 'postgres://horae@127.0.0.1:55415/horae' \
  -v ON_ERROR_STOP=1 \
  -c 'BEGIN READ ONLY; SELECT 1 FROM organizations LIMIT 1 FOR SHARE; ROLLBACK;'
```

returned `cannot execute SELECT FOR SHARE in a read-only transaction` and
nonzero exit status, as expected. The failed transaction was rolled back on
connection close; no business record or schema was modified. This demonstrates
why existing READ ONLY report/preview transactions cannot accept the proposed
row gate unchanged. It is not a concurrency test of the future permission layer.

The concrete inventory in `contracts/permission-state.md` also records the
snapshot/revision-fence and network-paced import cases that T039 must test against
the actual production helpers after T006–T009/T042 are settled. No Rust suite,
browser acceptance or full-flake result is claimed for this documentation-only
increment; the runtime results above remain those of the preceding code changes.

## Non-activating permission storage — 2026-10-03

Use a disposable PostgreSQL cluster, not the user's application database.
The run below used a newly initialized UTF-8 cluster on port 55416, a fresh
`horae_storage` compilation database and SQLx-created per-test databases.
The migration was not applied to any existing application database.

```sh
nix develop
export DATABASE_URL=postgres://horae@127.0.0.1:55416/horae_storage
sqlx migrate run --source crates/horae/migrations
cargo test -p horae --features server --bin horae server_fns::permissions:: --locked
cargo test -p horae-core --locked
cargo sqlx prepare --workspace -- --features server --all-targets
SQLX_OFFLINE=true cargo clippy -p horae --features server --all-targets --locked -- -D warnings
SQLX_OFFLINE=true cargo test -p horae --features server --bin horae --locked
nix fmt -- --ci
```

- RED: the new tests failed compilation because permission tables, policy columns,
  typed loaders and name validation did not exist. No passing placeholder API.
- GREEN: nine real PostgreSQL storage tests passed. They cover empty state/legacy
  mode, installation over populated legacy records, six built-ins and adjusted
  selections, independent identity/provenance, strict malformed-grant rejection,
  tenant constraints/lookups, invalid source shapes/revisions, direct-delete
  protection and unchanged grants/identity on fixture detachment.
- FR-032: ASCII and accented case collisions are tested against PostgreSQL's
  single `lower(name)` comparison, including concurrent inserts. Other tenants
  may reuse names. Two pure core tests cover outside Unicode whitespace,
  preserved display/internal spacing, blank input and 100/101 Unicode scalars.
  All 161 core tests and core/all-targets Clippy passed.
- SQLx regeneration added 18 cache files and removed none. Server/all-targets
  offline Clippy passed with warnings denied, checking the complete cached query
  set. Full server binary regression passed: 804 tests passed, zero failed and
  11 pre-existing manual checks remained ignored (815 total). Formatting and
  `git diff --check` passed. The separate integration binary and full Nix flake
  suite were not run; compilation of all targets is not execution of all tests.
- Bounded adversarial design review corrected an unsupported coupling of
  administrative identity to provenance and complete Administrator grants.
  Focused implementation self-review checked SQL NULL/source-shape constraints,
  restrictive composite tenant FKs, current catalog decoding, case-index atomicity
  and every loader caller. Only tests call these helpers; no guard is replaced.
  The typed read models are server-only and not deserializable authority inputs.
- Template detachment and arbitrary identity/source combinations are storage
  fixtures, not successful authenticated commands. Command authority, 50-profile
  races, revisions/audit/replay, runtime activation, browser parity and the full
  flake gate remain pending. No UI, CSS, external account or real data changed.

## Internal template commands — 2026-10-03

T053–T055 implement the create/delete subset of US4 under
`contracts/template-commands.md`, not a publicly accessible permissions editor.
Only disposable PostgreSQL fixtures enable policy version 1. The compile database
on port 55416 is likewise disposable; no application database was migrated.

```sh
cargo test -p horae --features server --bin horae template_tests --locked
CARGO_INCREMENTAL=0 cargo sqlx prepare --workspace -- --features server --all-targets
SQLX_OFFLINE=true cargo clippy -p horae --features server --all-targets --locked -- -D warnings
cargo test -p horae-core --locked
SQLX_OFFLINE=true cargo test -p horae --features server --bin horae --locked
nix fmt -- --ci
```

Observed focused evidence:

- The initial tests failed compilation for the missing command API and an
  incorrect test enum variant, which was corrected. The first five tests passed
  after implementation. Expanded coverage then passed all 17 command tests.
- A deliberate mutation replaced affected-person validation with template grants.
  `malformed_assignee_aborts_all_detachments` failed, proving it detects this
  unsafe shortcut. The original strict validation was restored immediately.
- Production helpers, not duplicated test SQL, perform create/delete/replay.
  Schema fixtures separately verify tenant FKs, mutually exclusive user/operator
  attribution and principal-scoped unique request identities. No operator command
  endpoint is provided.

| Contract boundary | Executable evidence |
| --- | --- |
| Canonical creation and exact historical replay | `create_canonical_retry_returns_one_historical_change`, `concurrent_exact_retry_creates_one_receipt_without_changing_people` |
| Current explicit administrator, tenant and policy checks | `legacy_future_missing_foreign_and_non_admin_authority_deny`, `invalid_stored_authority_and_template_grants_fail_closed` |
| Confirmed grants, prerequisite closure and FR-032 names | `equivalent_names_and_invalid_confirmed_selections_roll_back`, `unknown_grants_and_authority_fields_cannot_deserialize_as_commands`; storage tests also cover concurrent case collisions |
| Limit under concurrent creation | `concurrent_creators_never_exceed_fifty_profiles` |
| Exact grant/identity preservation, revisions and audit | `delete_preserves_adjusted_grants_identity_and_business_rows`, `malformed_assignee_aborts_all_detachments` |
| Deleted-source retries and same-name replacements | `delete_replay_does_not_touch_a_same_name_replacement` |
| Stale/foreign/exhausted revisions | `stale_foreign_and_exhausted_revisions_never_partially_delete` |
| Fresh authority after waiting, including replay | `revocation_winning_org_gate_denies_pending_replay_and_new_command` |
| Atomic audit failure and compatible user locks | `failed_audit_insert_rolls_back_creation_and_detachment`, `actor_share_lock_does_not_block_template_command` |
| Tenant/principal/version-scoped history | `receipts_enforce_tenant_exclusive_principal_and_request_uniqueness`, `command_receipts_are_not_shared_between_administrators`, `unsupported_receipt_version_fails_without_repeating_the_change` |

The first incremental SQLx preparation omitted 91 existing cache entries despite
`--all-targets`. Non-incremental preparation recovered them: the final cache adds
36 entries and deletes none. Offline all-targets server Clippy passed with
incremental compilation disabled and warnings denied. All 161 core tests passed;
the restored-code server binary suite passed 821 tests with zero failures and
11 pre-existing ignored checks in 152.48 seconds. Formatting and diff checks
passed; the separate integration binary, browser and full flake suite were not run.

Focused adversarial self-review checked authorization before replay, historical
outcomes after deletion, canonical request equivalence, complete affected-set
validation before writes, revision overflow, tenant/principal constraints,
rollback and the local lock order including FK locks. Expanded the initial
coverage to test malformed assignees (not only templates), full-floor duplicate
and missing-prerequisite selections, receipt principal isolation and unsupported
receipt versions. No high/critical finding remains in this bounded implementation;
this is not independent implementation review or full-feature security acceptance.
No new dependency, generic service layer, public endpoint, UI/CSS change or real
data operation was introduced. Full T042, profile application, permission editing
UI, cross-surface enforcement, migration and full-feature acceptance remain open.

## Internal person-profile commands (T056–T058)

Use only an owned disposable PostgreSQL cluster with `CREATEDB`; the application
database and Harvest accounts are not test targets. Migration 0044 is additive,
has no legacy backfill and leaves the permission policy inactive. Fixtures alone
explicitly enable version 1. The command and schema contract is
`contracts/person-profile-commands.md`; no public endpoint is delivered here.

```sh
# Set DATABASE_URL to the disposable compilation DB, then enter the Nix shell.
cargo sqlx migrate run --source crates/horae/migrations
cargo test -p horae --features server --bin horae profile_tests --locked
CARGO_INCREMENTAL=0 cargo sqlx prepare --workspace -- --features server --all-targets
SQLX_OFFLINE=true CARGO_INCREMENTAL=0 cargo test -p horae --features server --bin horae --locked
cargo test -p horae-core --locked
SQLX_OFFLINE=true CARGO_INCREMENTAL=0 cargo clippy -p horae --features server --all-targets --locked -- -D warnings
nix fmt -- --ci
```

| Contract boundary | Production-command regression |
| --- | --- |
| Confirmed adjusted grants and exact replay | `explicit_application_and_adjustments_preserve_confirmed_grants` |
| Unchanged state, timestamps, provenance and no change event | `unchanged_edit_preserves_identity_source_timestamp_and_revisions` |
| Independent identity, inactive targets and invalid Administrator proposals | `explicit_identity_changes_and_inactive_targets_do_not_change_activation`, `reduced_administrator_and_stale_person_proposals_are_rejected` |
| Actual active explicit Administrator count, including concurrent demotions | `last_active_administrator_cannot_be_demoted_or_replaced_by_equivalent_grants`, `concurrent_self_demotions_leave_one_active_explicit_administrator` |
| Exact joint loss confirmation, history/incoming preservation and no restoration | `simultaneous_losses_require_exact_confirmation_preserve_history_and_never_restore_links` |
| Read-only retention and explicit final keep-access grants | `read_only_retention_and_explicit_keep_project_access_preserve_independent_person_losses` |
| Template adjustment/reset, deletion replay and shared request namespace | `adjusted_template_reset_and_replay_after_deletion_use_explicit_intent`, `all_grant_template_cannot_confer_identity_and_cross_command_keys_conflict` |
| Current authority after a real gate wait, including replay | `revocation_winning_gate_denies_new_and_replayed_changes` |
| Atomic rollback and compatible legacy user locks | `audit_failure_rolls_back_profile_and_both_relationship_sets`, `existing_user_share_lock_does_not_block_profile_command` |
| Tenant/self/duplicate schema boundaries and caller/target isolation | `management_schema_rejects_foreign_parents_self_links_and_duplicate_pairs`, `canonical_actor_policy_tenant_and_target_checks_precede_mutation` |
| Strict saved/input validation, stale references and overflow | `malformed_person_or_remaining_administrator_state_fails_closed`, `invalid_confirmations_and_noncanonical_grants_are_not_silently_repaired`, `stale_template_and_org_revision_and_exhausted_revisions_roll_back` |
| No accidental historical cleanup and remove/recreate fencing | `unchanged_and_unrelated_edits_do_not_clean_up_preexisting_incompatible_links`, `recreated_relationship_invalidates_waiting_confirmation_without_partial_removal` |

Read-only independent contract-to-code review found no high/security defect in
the bounded transaction. It requested the final two regressions above and clearer
wording for the earlier generic organization-revision test; both were added.
Relationship replacement in the fencing test is fixture SQL under the required
gate, not an implemented assignment-addition endpoint. The pending profile save
uses the actual production command. Full T042/mixed-policy safety is not proved.

The initial SQLx-offline RED run also lacked new query entries; the corrected
live-disposable-DB run failed solely on the absent command module. After
implementation, 5, 13 and 18 focused tests passed; the final 20-test suite also
passed with no failures or ignored cases. The full server binary regression
suite passed (852 tests discovered, 11 pre-existing ignored), followed by all
161 core tests. No separate integration-binary execution, browser run or full
flake acceptance is claimed.

SQLx preparation can omit unchanged integration-target metadata even with
`CARGO_INCREMENTAL=0`: this run initially removed 91 integration query entries
despite successful preparation and warm offline Clippy. A warm compilation is
not proof of a complete cache. If this occurs, first finish other Cargo tasks,
then clean **only this worktree's package build artifacts** with
`cargo clean -p horae`, repeat the non-incremental all-targets preparation above,
and verify deleted-cache count plus a fresh offline all-targets check. Do not
delete source, database state or manually fabricate missing query descriptions.

The clean-package regeneration recovered all 91 omitted entries: final cache
adds 30 descriptions with zero changed or deleted existing entries. Fresh offline
all-targets server Clippy then passed with warnings denied. No source/database
data was removed; only regenerable package build artifacts were cleaned.
Formatting CI and diff checks also passed. T056–T058 are complete for this
internal boundary; full T037/T038, authenticated wrappers and runtime acceptance
remain open. Rust/testing/async/simplicity guidance kept this in existing modules,
dependencies and SQLx transactions, with no generic policy framework or UI change.

## Internal project delegation (T059–T061)

Use the same owned disposable PostgreSQL compilation database and test role with
CREATEDB as above. No new migration is introduced; fixtures alone enable policy 1.
The internal command is not wired to the existing project editor or a public
endpoint. See `contracts/project-management-commands.md` for its closed boundary.

```sh
cargo test -p horae --features server --bin horae project_management_tests --locked
cargo test -p horae --features server --bin horae --locked
cargo test -p horae-core --locked
# Finish other Cargo processes before refreshing the complete cache.
cargo clean -p horae
CARGO_INCREMENTAL=0 cargo sqlx prepare --workspace -- --features server --all-targets
SQLX_OFFLINE=true CARGO_INCREMENTAL=0 cargo clippy -p horae --features server --all-targets --locked -- -D warnings
nix fmt -- --ci
```

| Contract | Production-command regression |
| --- | --- |
| Project editing grants, current managed designation, no legacy membership or identity bypass | `project_editor_delegates_existing_read_grants_without_promotion`, `canonical_actor_grants_activity_tenant_and_policy_are_required`, `managed_scope_never_uses_another_project_or_legacy_membership` |
| Entire-set eligibility and strict tenant/target validation | `mixed_invalid_addition_preserves_entire_previous_set`, `foreign_missing_inactive_and_malformed_additions_fail_without_disclosure` |
| Retained identities, no-op audit, archived projects and eligible removal | `retained_ineligible_managers_and_archived_projects_allow_noop_and_removal` |
| Exact audit delta; preserve membership, cost override, hours and unrelated scopes | `replacement_audits_exact_delta_and_preserves_membership_history_and_other_scopes` |
| Reordered/historical replay, cross-command intent, version rejection and self-removal | `historical_replay_does_not_reapply_removed_or_now_ineligible_designations`, `receipt_conflicts_precede_decoding_and_replay_requires_current_authority`, `actual_template_command_receipt_cannot_be_reused_for_project_delegation`, `designation_is_required_before_managed_editor_can_delegate` |
| Stale/duplicate/overflow denial and concurrent replacements | `stale_duplicate_and_exhausted_revisions_never_replace_the_set`, `concurrent_replacements_commit_only_one_current_revision` |
| Revocation after an actual gate wait | `revocation_winning_gate_denies_waiting_delegation` |
| Legacy project-parent lock conflict, whole rollback and identical retry | `legacy_project_lock_returns_busy_instead_of_forming_an_org_fk_cycle` |
| Actor/target FK compatibility and audit failure rollback | `legacy_user_share_locks_allow_manager_and_receipt_foreign_keys`, `audit_failure_rolls_back_removal_addition_and_revision` |

Initial RED compilation failed because the command module did not exist. The
first implementation run failed in fixture setup because its INSERT omitted
non-null administrative identity; fixed the fixture to insert explicit false.
All 17 focused tests then passed. Independent read-only review found no high
security defect in the production command and requested four additional coverage
cases; all are included in the passing suite above. This does not prove T042 or
mixed-policy safety. Full server binary regressions passed with 858 passing tests,
zero failures and 11 pre-existing ignored cases; all 161 core tests passed.
Formatting CI passed with zero changes. Clean-package, non-incremental SQLx
preparation adds 28 query descriptions and changes/deletes no existing entries.
Fresh offline all-targets server Clippy passed with warnings denied. Cleaning
removed 5.7 GiB of regenerable package artifacts only, not source or database data.
No separate integration-binary execution, browser or full flake run is claimed.
T059–T061 are complete for this internal command; full user-story and activation
gates remain open. No new crate, dependency, schema migration or UI/CSS change.

## Internal historical audit lookup (T062–T064)

Reuse the owned disposable PostgreSQL cluster and compilation database above.
Fixtures alone enable policy 1; no current application database is migrated or
activated. `contracts/audit-lookup.md` owns this receipt-ID read boundary.

```sh
cargo test -p horae --features server --bin horae audit_tests --locked
cargo test -p horae --features server --bin horae --locked
cargo test -p horae-core --locked
# After all test/build processes have finished:
cargo clean -p horae
CARGO_INCREMENTAL=0 cargo sqlx prepare --workspace -- --features server --all-targets
SQLX_OFFLINE=true CARGO_INCREMENTAL=0 cargo clippy -p horae --features server --all-targets --locked -- -D warnings
nix fmt -- --ci
```

| Contract | Reader/decoder regression |
| --- | --- |
| Current explicit Administrator, not legacy role or grant equivalence | `legacy_admin_and_all_grants_without_explicit_identity_cannot_read_audit` |
| Tenant, activity, policy and strict authority precede receipt decoding | `tenant_activity_and_policy_checks_precede_receipt_existence_and_decode` |
| Other authors, later inactivity and missing current permission state | `current_administrator_reads_inactive_authors_history_without_live_state` |
| Deleted templates and exact historic projection without private request data | `administrator_reads_historical_template_without_intent_or_replay_result`, `template_detachment_history_preserves_exact_grants_and_provenance` |
| Real profile/project changes, no-ops and removed scopes survive later changes | `profile_and_project_history_decode_exact_changes_and_noops`, `profile_history_keeps_removed_relationships_after_grants_change_again` |
| Distinct operator attribution, no intent or private replay outcome | `operator_attribution_is_not_a_user_and_excludes_private_replay_payload` |
| Failed command has no successful receipt; reads do not mutate | `failed_mutation_has_no_success_audit_and_reads_do_not_write` |
| Real lock waits in both revocation orders | `revocation_winning_gate_denies_waiting_historical_reader`, `reader_winning_gate_finishes_before_revocation_and_later_reads_fail` |
| Required nullable fields, supported formats, strict grants/provenance and revisions | The four `historical_*` decoder tests in `permissions/tests/audit.rs` |

Initial RED compilation failed on the missing reader module. Implementation
compilation caught the timestamp type inference and additional fixture schema/PID
mismatches; these were corrected without changing schema or weakening assertions.
The first focused GREEN run passed all 15 tests. A subsequent real stored malformed
document check was added before the full regression run, which passed 873 tests
with zero failures and 11 pre-existing ignored cases (172.11 seconds). All 161
core tests passed. Clean-package, non-incremental SQLx preparation added 14 query
descriptions and changed/deleted no existing entries. Fresh offline all-targets
Clippy passed with warnings denied; formatting CI passed with zero changes.
Cleaning removed 5.8 GiB of regenerable package build artifacts only. T062–T064
are complete for the internal lookup; T041 remains open for authenticated history
delivery and integration. No separate integration-binary, browser or full flake
execution is claimed.

Adversarial self-review traced all three production writers to the projected
historical shapes, verified authorization precedes receipt lookup/decoding, and
checked that the organization gate remains held until projection and commit.
Historical reads perform no FK inserts or later row locks. Coverage gaps for
reader-first revocation, inactive authors and actual removed/detached snapshots
were closed with production-reader tests. No critical/high defect was identified
in this bounded review; this is not an independent full-feature review or proof
of runtime activation, HTTP authentication, audit browsing or UI acceptance.

## Project-family organization-first integration (T068–T070)

Reuse the owned disposable PostgreSQL on 55416. No real database, permission
activation, UI/CSS, external service or migration is changed by this increment.
All commands run in the Nix shell with the disposable `DATABASE_URL` above.

```sh
cargo test -p horae --features server --bin horae gate --locked
cargo test -p horae --features server --bin horae --locked
# After all test/build processes have finished:
cargo clean -p horae
CARGO_INCREMENTAL=0 cargo sqlx prepare --workspace -- --features server --all-targets
SQLX_OFFLINE=true CARGO_INCREMENTAL=0 cargo clippy -p horae --features server --all-targets --locked -- -D warnings
nix fmt -- --ci
```

| Requirement / boundary | Production-path acceptance |
| --- | --- |
| FR-007/010: all ten draft/editor entry points gate before actor/resource locks; no writes after revocation | `every_project_entry_gates_before_actor_and_rechecks_revocation`, including fresh retries and unchanged draft/client/project/receipt counts |
| FR-010: editor-first commits before revocation; revocation-first rejects the stale snapshot and fresh retry | `project_editor_and_user_revocation_commit_in_gate_order`, calling actual editor and user-change transactions |
| FR-007/010: assignment add/remove, task link and creation with/without a project reload active authority | `project_membership_and_task_callers_wait_before_authorizing`, preserving assignment/task/link counts on denial |
| FR-017: assignment cascade permits an already-authorized entry FK to finish and denies the next entry | `assignment_cascade_allows_an_inflight_entry_project_fk_to_finish` |
| FR-017: exclusive organization gate permits an in-flight invoice's FK writes | `assignment_gate_allows_inflight_invoice_organization_fks_to_finish` |
| FR-018: legacy tenant, malformed-link, archived-task, rate, draft replay and editor concurrency behavior remains | Existing assignment/project/creation, invoice, entry and user regressions |

RED reproduced the inverse actor/organization order in actual inline-client
creation: the actor NOWAIT assertion failed with PostgreSQL 55P03. After the
implementation, the focused `gate` filter passed 12 tests, including both actual
editor/revocation orders and invoice FK compatibility. A subsequent strengthening
checks that finalization/editor changes exclude SHARE readers. The complete
server binary regression, including the strengthened gate assertion and all five
new tests, passed: 883 passed, zero failed and 11 pre-existing ignored cases
(168.58 seconds). All 161 core tests passed. Fresh complete SQLx preparation
adds 26 query descriptions and removes the five replaced queries; no unrelated
cache descriptions change. Offline all-targets server Clippy passed with warnings
denied. The first formatting CI check inserted one missing Markdown blank line;
the corrected files are checked again before publication. The package-local
clean removed 1.5 GiB of regenerable build artifacts, not source or data. The owned
PostgreSQL cluster is stopped. No separate integration-binary, browser or full
flake execution is claimed.

The bounded review covers all callers of the changed helpers, tenant rechecks,
gate modes, actor snapshot checks, revision triggers and the two FK counterexamples.
No new dependency or grant rule is introduced. This is not full-feature analysis,
browser acceptance, a deadlock-free proof of all writers or completed T042.
T068–T070 close this named integration boundary only. The remaining policy,
authenticated surfaces, approvals, UI, migration and full acceptance work remains.
