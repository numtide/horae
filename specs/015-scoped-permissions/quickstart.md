# Permission verification

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
