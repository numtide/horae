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
