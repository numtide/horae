# Permission PR separation

## Objective and limits

Split the existing work in #212 and #217 into reviewable deliveries, preserving
all original changes. This is not implementation of the remaining permission
requirements or completion of Harvest parity. Do not merge or close #212/#217,
activate canonical policy, alter real data, or modify #208.

This is the single separation ledger. Existing feature contracts remain the
source of product requirements; do not restart or duplicate them.

## Verified starting state — 2026-10-06

- #216 merged at 13:16:59 UTC as `02f7b58acdcf126415f9ec89215da8cdada7d03f`.
  Queue CI run [37464771137](https://github.com/numtide/horae/actions/runs/37464771137)
  passed Flake Check and Format. This proves the base, not any extraction.
- #212 remains open/draft at `db3935db364f2a8aa193f0e938ce40ecc01a2f92`.
  Its original change set from `9301112c6a02ae3c92716273534f38889db241d1`
  contains 145 commits and 1,214 file changes, including 841 SQLx descriptors.
  Fifty commits change only specification/governance documents; 95 also change
  code, tests, or tooling. File counts are not delivery boundaries.
- #217 remains open/draft at `dd141c5cc4f8dea4f41a4c7cfbb724323ff6d198`,
  based on `feat/scoped-permissions` at `5faed76`. Its sole additional commit
  is not present in #212. It has no current-head CI.
- The worktree named `.worktrees/report-reader-authority` actually holds
  `feat/delegated-timesheet-commands` at `a0ea691`, not #217. Its commits
  `8360b20` and `a0ea691` are patch-equivalent to #212's `48a6533` and
  `02c4245`; preserve the original ref without extracting duplicates.
- Root `master` and the original worktrees were not reset or switched.
  The root's untracked `.playwright-mcp/` and ignored runtime/evidence files
  remain untouched in place.

## Preservation and recovery

Private local backup directory:
`.scratch/pr212-split-backup-20261006.BwoORH/`.

| Reference | Preserved object |
| --- | --- |
| `backup/pr212-split-20261006-original` | `db3935db364f2a8aa193f0e938ce40ecc01a2f92` |
| `backup/pr217-split-20261006-original` | `dd141c5cc4f8dea4f41a4c7cfbb724323ff6d198` |
| `backup/timesheet-split-20261006-original` | `a0ea69108416f2cc380d2145d9b21b44d4c0338f` |
| `backup/pr212-split-20261006-uncommitted` | `d364270a6c7684457734dff66ff54f54e1066f83` |

The uncommitted snapshot was created with `git stash create`, not a stash
operation that clears the index/worktree. No original edits were removed.

| Backup file | SHA-256 |
| --- | --- |
| `originals.bundle` | `b66db9b106948ca2d1bc8c28040fd945a76691d350f4b524b535ac3ee64aa305` |
| `unstaged.patch` | `d90b1797021e6d9d2fcb383d98d8db8993516675c887588a93203c6606fddc78` |
| `staged.patch` (empty) | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| `untracked.tar` | `3b76bc925c05943a3732c4c2c5bdbb8ad58e1e90680390b3400c03b5b244aabc` |

Recovery verification passed: `git bundle verify`; independent no-checkout
clone of the bundle into `restore-check/`; checkout of the uncommitted snapshot;
extraction of all six untracked files; archive comparison against the source;
zero tracked diff between the source worktree and saved snapshot. Regenerating
the original-to-snapshot binary diff produced the exact saved patch hash.
The original worktree's status remains unchanged.

To inspect recovery, use the existing private `restore-check/` copy. For a new
recovery, clone the bundle into a new empty directory, check out the saved
snapshot and extract the untracked archive there. Never restore over an
original worktree.

### Unpublished work preserved, not delivered

All 18 paths below belong to unfinished client compatibility/permission work.
The three deleted caches and three replacement caches are preserved together
with their queries. T237/T238 remain incomplete; do not treat #216's legacy
Clients MVP as canonical-permission acceptance.

| Original status | Path |
| --- | --- |
| ` D` | `.sqlx/query-63f2ec39d8821f1ba1534ddfb43cae8142e9b137e4d1c17d2e262e4ff66a272e.json` |
| ` D` | `.sqlx/query-cfb2a46e0259c64f406084030f1e3bb6e4ac424bc615880032a02ea0b7c07ad2.json` |
| ` D` | `.sqlx/query-f64bd1dac9e7dfaba8f8df5e299dc3370b32773b909a0ead9245492364e08aad.json` |
| ` M` | `crates/horae/src/harvest/mod.rs` |
| ` M` | `crates/horae/src/harvest/pagination_tests.rs` |
| ` M` | `specs/015-scoped-permissions/contracts/dependent-spec-reconciliation.md` |
| ` M` | `specs/015-scoped-permissions/contracts/operation-matrix.md` |
| ` M` | `specs/015-scoped-permissions/plan.md` |
| ` M` | `specs/015-scoped-permissions/progress.md` |
| ` M` | `specs/015-scoped-permissions/quickstart.md` |
| ` M` | `specs/015-scoped-permissions/spec.md` |
| ` M` | `specs/015-scoped-permissions/tasks.md` |
| `??` | `.sqlx/query-0565720011fa00b20930e95efb31d75581de152f75a5aad3a13ebdaa04b898e2.json` |
| `??` | `.sqlx/query-0574b2ae32e7f4df176d2bc39f263c8444e4cb526a8bd4f93199ebf633045590.json` |
| `??` | `.sqlx/query-f88711969f759f5466db07ed2dff6b46f0e353cd7015f004ace804a7516cec4d.json` |
| `??` | `crates/horae/src/harvest/client_reads.rs` |
| `??` | `crates/horae/src/harvest/pagination_tests/client_permissions.rs` |
| `??` | `specs/015-scoped-permissions/contracts/client-permissions.md` |

## Extraction order and current deliveries

| Delivery | Branch/worktree | Base | Status / acceptance |
| --- | --- | --- | --- |
| Separation ledger | `docs/permission-pr-separation`, `.worktrees/permission-pr-separation` | `02f7b58` | Inventory recorded; no completed-extraction claim |
| Legacy report/invoice readers from #217, [#220](https://github.com/numtide/horae/pull/220) | `fix/report-reader-authority-master`, `.worktrees/report-reader-authority-master` | `02f7b58` | Draft at `bc0c7a0`; 1,127 tests passed, 11 existing ignored; SQLx, offline server/WASM lint and formatting passed; CI pending |
| Pure record scopes and grant catalog, [#219](https://github.com/numtide/horae/pull/219) | `refactor/permission-domain-foundation`, `.worktrees/permission-domain-foundation` | `02f7b58` | Draft at `ec7ddbd`; 158 core tests, core Clippy and formatting passed; full CI pending; no runtime integration |
| Remaining #212 behavior groups | Original refs plus candidate inventory below | To be resolved from actual dependencies | Not submitted or certified; preserve every group until assigned to a resulting PR |

Candidate groups below are review units, **not a commitment to 31 PRs**.
Combine or subdivide only after tracing code and test dependencies. Prefer
independent PRs; stack when required. The final record must identify resulting
PRs, equivalents already integrated, and expressly retained unfinished work.
The current candidate classification does not yet satisfy that final mapping.

### First extraction: legacy report and invoice reader authority

Purpose: reauthorize `report_detailed`, `list_invoices` and `get_invoice`
after the initial session lookup, and keep invoice headers/lines in one
snapshot. Preserve existing Manager/Admin policy, result sizes, filters and
errors; introduce neither canonical grants nor UI changes.

Dependencies verified by source inspection:

- `snapshot::manager` from `22ffdab`: bounded REPEATABLE READ authorization,
  organization/actor SHARE locks, retry and timeout restoration.
- `db::OrganizationLock`/`lock_organization` from `3ae8e08`; absent from
  #216. Do not cherry-pick the entire project-access commit merely for this helper.
- Existing base already supplies executor-based report reads,
  connection-based invoice projection, test seed/blocking helpers and the
  registered-session HTTP harness.
- Preserve #216's `fetch_invoices` query with its optional client filter and
  stable `created_at, id` ordering. Wrap it rather than copying #217's older
  list query.
- Bring only the relevant HTTP test additions and exact endpoint matching;
  do not copy the parent's unrelated authorization test modules.
- Include all helper/query SQLx descriptors and regenerate/verify the extracted
  cache, not only the eight descriptors in #217's own commit.

Acceptance scope on the new head: all seven reader regressions, authenticated
HTTP cases, tenant/filter/not-found behavior, direct and gated revocation in
both orders, coherent invoice data, cancellation, inherited settings, #216
client-invoice regressions, cross-writer lock review, full server/core tests,
offline SQLx compilation, server/WASM lint, formatting and required Nix gates.
All local checks above passed as recorded below; required CI/Nix gates remain
pending. Historical tests on `5faed76` do not prove this extraction.

### Shared foundations and migration constraints

- First domain delivery contains `AccessScope`, the grant catalog, built-in
  selections and strict restoration of saved selections. Its five new files
  match `524c29e` exactly; the only adaptation is adding `pub mod permissions`
  to the current core root while preserving #216's `client` module.
  This accounts for the code/test hunks of `a7727f1`, `e8dcb77` and `524c29e`;
  their specification hunks remain preserved for specification reconciliation.
  The later profile-name validator from `ec35446` stays with profile storage;
  person-management, rates and approval coverage are separate dependants.
- Pure scope/catalog/rate/approval-record rules can be evaluated separately
  from runtime policy activation.
- Storage and command receipts/assignment models precede canonical consumers.
  Editor, People, Timesheet, Reports, project and task consumers must keep
  policy-zero behavior until the separately approved full cutover.
- `0042` adds policy version defaulting to zero, storage and
  `users(org_id,id)` uniqueness; it performs no legacy mapping.
- `0043` receipts and `0045` import requester provenance depend on that
  compound user key. Provenance is not a clean standalone cherry-pick as written.
- `0044` keeps management separate from tracking; `0047` name collation
  depends on `0042` template storage.
- `0048` task lifecycle/view logic depends on `0042` policy version and must
  stay with corresponding task commands/readers. Its backfill is canonical-only.
- `0046` supplies the native CSV cursor function; review source/release
  authorization and streaming consumers together before extracting it.
- Preserve migration identities/checksums unless a necessary adaptation is
  explicitly documented and verified on disposable databases. Do not run
  migrations against existing user data.
- The sole Nix check change makes crate JS assets visible to browser scripts.
  Keep it with tests that require those assets. #212 changes no Cargo dependency
  files or CI workflow.
- Shared legacy authority/lock and export repairs need cross-command review;
  do not declare them independent from titles alone.

## Spec Kit reconciliation

The existing `015-scoped-permissions` spec/plan/tasks were resolved using
`check-prerequisites.sh --json --require-tasks --include-tasks`, with no
feature-directory override or file changes. No Analyze hooks were present.
This was a separation-focused partial consistency analysis, not a clean
full-feature acceptance report.

The working-copy task inventory has 238 unique IDs: 208 checked and 30 unchecked.
These are bookkeeping counts, not a percentage of application completion.
All 36 FRs have candidate task or section-context references; all nine success
criteria still require full acceptance. Do not infer coverage from task-title
regexes alone.

Unchecked tasks: T237, T238, T230, T234, T203, T042, T006, T007, T008, T009, T010, T011, T037, T038, T012, T013, T043, T044, T045, T046, T014, T015, T039, T040, T016, T017, T041, T018, T019, T020.

Evidence reconciliation:

- T161's earlier prose says pending, but `1b81680` explicitly closes the
  checkbox and reconciles `2497dbe` to frozen-source verification
  (repeat `52467`, 1,751 tests, browser/lint/SQLx/VM gates). Preserve that
  historical completion; clarify the stale paragraph when carrying the docs.
  It proves neither later heads nor the extracted branch.
- The dependent-spec register's #216 reference `5a459c4` predates its
  authority and browser corrections. The integration base is the actual
  `02f7b58` merge above.
- The source contains constitution 1.1.0, while master uses 1.0.0. Preserve the
  existing amendment as its own governance/specification material; it is not
  a prerequisite for a legacy-reader security repair.
- Full approval/withdrawal, company locks, migration/cutover, financial report
  and unresolved operation contracts remain unfinished. Do not finish them
  as part of splitting or silently turn their drafts into accepted behavior.

## #208 overlap — read-only

#208 remains outside this work. Its `48a4156` branch shares 38 changed paths
with #212, seven of them SQLx descriptors. Path overlap is not proof of a text
conflict. Recheck integration later without modifying #208.

Non-cache overlaps:

- `.specify/feature.json`
- `crates/core/src/budget.rs`
- `crates/core/src/lib.rs`
- `crates/horae/assets/css/horae.css`
- `crates/horae/src/components/sidebar.rs`
- `crates/horae/src/main.rs`
- `crates/horae/src/models/assignment.rs`
- `crates/horae/src/models/project.rs`
- `crates/horae/src/pages/new_project.rs`
- `crates/horae/src/pages/projects.rs`
- `crates/horae/src/pages/projects/fee_balances.rs`
- `crates/horae/src/pages/reports.rs`
- `crates/horae/src/reports.rs`
- `crates/horae/src/reports/limits.rs`
- `crates/horae/src/reports/streaming.rs`
- `crates/horae/src/reports/streaming/database_tests.rs`
- `crates/horae/src/route.rs`
- `crates/horae/src/server_fns.rs`
- `crates/horae/src/server_fns/budgets.rs`
- `crates/horae/src/server_fns/invoices/tests.rs`
- `crates/horae/src/server_fns/invoices/tests/imported_rates.rs`
- `crates/horae/src/server_fns/organization.rs`
- `crates/horae/src/server_fns/projects.rs`
- `crates/horae/src/server_fns/reports.rs`
- `crates/horae/tests/browser/action-errors.cjs`
- `crates/horae/tests/browser/new-project-permissions.cjs`
- `crates/horae/tests/browser/new-project.cjs`
- `crates/horae/tests/browser/project-task-rates.cjs`
- `crates/horae/tests/browser/run-design-checks.sh`
- `crates/horae/tests/detail_navigation.rs`
- `crates/horae/tests/new_project_screen.rs`

## Original commit accounting

Every original commit is retained in the verified bundle and refs. Entries
below assign a candidate responsibility, not yet a final destination PR.
Mixed commits may supply multiple deliveries; record exact adaptations and
avoid duplicate shared helpers/queries when finalizing them.

| Source | Original change | Candidate responsibility | Disposition |
| --- | --- | --- | --- |
| `05448a8` | Specify scoped roles and permissions | specification-history | Held in original backup; extraction pending |
| `1d45191` | Require Harvest parity for permissions and scoped approvals | specification-history | Held in original backup; extraction pending |
| `a7727f1` | Add record scope evaluation for permissions | scope-domain | Code/tests in #219; specification hunks retained for reconciliation |
| `2abce9a` | Enforce approval isolation and record permission boundaries | approval-isolation | Held in original backup; extraction pending |
| `757f43d` | Enforce tenant and administrator boundaries for assignments | legacy-access-writers | Held in original backup; extraction pending |
| `d3a4ff3` | Document profile reapplication and import permission boundaries | specification-history | Held in original backup; extraction pending |
| `dcf21ef` | Specify permission migration safeguards and rate-scope verification | specification-history | Held in original backup; extraction pending |
| `b3ee8da` | Align authorization governance with scoped permission profiles | specification-history | Held in original backup; extraction pending |
| `b569c75` | Extend permission specification to confirmed web domains | specification-history | Held in original backup; extraction pending |
| `f5cf02d` | Record expense scope defaults and lifecycle permission gaps | specification-history | Held in original backup; extraction pending |
| `6ce9071` | Document expense action scope and independent lock states | specification-history | Held in original backup; extraction pending |
| `b8b105e` | Document current-account permission research and evidence gaps | specification-history | Held in original backup; extraction pending |
| `6443d56` | Record current Harvest permission evidence and reference conflicts | specification-history | Held in original backup; extraction pending |
| `e8dcb77` | Add typed permission catalog and built-in profile selections | scope-domain | Code/tests in #219; specification hunks retained for reconciliation |
| `b80f8ab` | Recheck administrator authority during user access changes | legacy-access-writers | Held in original backup; extraction pending |
| `b7e730c` | Define permission persistence and transactional access contracts | specification-history | Held in original backup; extraction pending |
| `412035d` | Map permission operations and transaction constraints | specification-history | Held in original backup; extraction pending |
| `324084c` | Clarify permission boundaries for jobs and authentication | specification-history | Held in original backup; extraction pending |
| `5fb78a7` | Specify permission preservation when deleting templates | specification-history | Held in original backup; extraction pending |
| `f17fafa` | Clarify report-specific financial access | specification-history | Held in original backup; extraction pending |
| `8455740` | Document resource-specific managed rate proposal | specification-history | Held in original backup; extraction pending |
| `168d536` | Specify resource-scoped billable rate permissions | specification-history | Held in original backup; extraction pending |
| `5bf841d` | Specify explicit cost rate permissions | specification-history | Held in original backup; extraction pending |
| `ece40f5` | Clarify company lock scheduling and approval boundaries | specification-history | Held in original backup; extraction pending |
| `7bcb3e2` | Require full record visibility for combined approvals | specification-history | Held in original backup; extraction pending |
| `0fe8f56` | Document project manager assignment retention evidence | specification-history | Held in original backup; extraction pending |
| `a1cc799` | Specify project manager retention with read access | specification-history | Held in original backup; extraction pending |
| `9c07d60` | Clarify delegation evidence and approval history boundaries | specification-history | Held in original backup; extraction pending |
| `ab9b1a4` | Define project-editor delegation and cross-feature permission contracts | specification-history | Held in original backup; extraction pending |
| `80e2e42` | Track person delegation and project lifecycle permission gaps | specification-history | Held in original backup; extraction pending |
| `eb85bc1` | Specify migration preservation checks for assignment dependencies | specification-history | Held in original backup; extraction pending |
| `b8a1ef5` | Record pending person-management policy decision | specification-history | Held in original backup; extraction pending |
| `3cc9afe` | Restrict person-management assignment changes to administrators | specification-history | Held in original backup; extraction pending |
| `524c29e` | Reject malformed stored permission selections | scope-domain | Code/tests in #219; specification hunks retained for reconciliation |
| `ec9408b` | Record pending person-assignment eligibility decision | specification-history | Held in original backup; extraction pending |
| `d965718` | Require compatible grants for new person-management assignments | specification-history | Held in original backup; extraction pending |
| `0c56dc6` | Document managed-person removal on Harvest role downgrade | specification-history | Held in original backup; extraction pending |
| `6474382` | Confirm person-assignment removal after permission loss | specification-history | Held in original backup; extraction pending |
| `3a19a26` | Specify explicit project-access retention choice | specification-history | Held in original backup; extraction pending |
| `91f7cc3` | Distinguish profile selection from unchanged permission saves | specification-history | Held in original backup; extraction pending |
| `602b04f` | Separate displayed permission profiles from assignment provenance | specification-history | Held in original backup; extraction pending |
| `1105b75` | Record remaining permission decision gates | specification-history | Held in original backup; extraction pending |
| `cf5d635` | Define rejection of person-management self-assignments | specification-history | Held in original backup; extraction pending |
| `804b1d8` | Separate permission increment readiness from activation gates | specification-history | Held in original backup; extraction pending |
| `6e61593` | Validate person-management grant compatibility and self-links | scope-domain | Held in original backup; extraction pending |
| `ec35446` | Store versioned permission profiles without activating new policy | permission-storage | Held in original backup; extraction pending |
| `d50c979` | Add audited permission template commands | permission-profile-transactions | Held in original backup; extraction pending |
| `f5e0dde` | Apply permission profiles with atomic scope changes | permission-profile-transactions | Held in original backup; extraction pending |
| `9a7e05d` | Add audited project manager delegation | project-manager-delegation | Held in original backup; extraction pending |
| `fc85231` | Add administrator-only permission audit lookup | permission-audit | Held in original backup; extraction pending |
| `c3d17cb` | Prevent deadlocks during legacy import report conversion | import-transaction-lifecycle | Held in original backup; extraction pending |
| `3ae8e08` | Coordinate project access changes before locking resources | legacy-access-writers | Organization SHARE query reused in #220; writer/helper consumers and remaining hunks retained |
| `907bc88` | Recheck authority when saving organization branding | branding-authority | Held in original backup; extraction pending |
| `d7a5a21` | Revalidate administrator authority for Harvest connection changes | import-authority | Held in original backup; extraction pending |
| `4fac6af` | Revalidate import job command and status authority | import-authority | Held in original backup; extraction pending |
| `b4672a4` | Revalidate authority during import error downloads | import-authority | Held in original backup; extraction pending |
| `e5fcc5a` | Prepare durable CSV batches before opening transactions | import-transaction-lifecycle | Held in original backup; extraction pending |
| `482b7c5` | Retain the original requester of import jobs | import-requester-provenance | Held in original backup; extraction pending |
| `e949e4c` | Drain interrupted import transactions before releasing reservations | import-transaction-lifecycle | Held in original backup; extraction pending |
| `c0cfb8f` | Scope rate permissions to their owning resource | scope-domain | Held in original backup; extraction pending |
| `5d51b0e` | Expose the current person's permission snapshot | own-permissions | Held in original backup; extraction pending |
| `22ffdab` | Recheck manager access for financial snapshots | manager-snapshot-consumers | Shared snapshot helper/queries in #220; original financial consumers and remaining hunks retained |
| `15c82ef` | Recheck manager access in invoice editor snapshots | manager-snapshot-consumers | Held in original backup; extraction pending |
| `dc822a0` | Recheck manager authority before delivering exports | export-authority | Held in original backup; extraction pending |
| `108594e` | Revalidate project scope before delivering exports | export-authority | Held in original backup; extraction pending |
| `dcf4ac8` | Recheck current permissions during CSV downloads | export-authority | Held in original backup; extraction pending |
| `0793ce7` | Revalidate budget email authority before delivery | budget-email-authority | Held in original backup; extraction pending |
| `8aac739` | Add read-only permission migration diagnostics | permission-preflight | Held in original backup; extraction pending |
| `8c15bfe` | Show own permissions in Settings | own-permissions | Held in original backup; extraction pending |
| `300d1e9` | Expose administrator permission history | permission-audit | Held in original backup; extraction pending |
| `03e90b1` | Connect permission editor previews and commands | permission-editor | Held in original backup; extraction pending |
| `7f7fd1c` | Record permission editor delivery and UI follow-up | specification-history | Held in original backup; extraction pending |
| `98b1692` | Add reviewed person permission editing | permission-editor | Held in original backup; extraction pending |
| `9e6d8bd` | Record person editor delivery and template follow-up | specification-history | Held in original backup; extraction pending |
| `0f97cb2` | Add custom permission profile controls | permission-editor | Held in original backup; extraction pending |
| `8db19ba` | Show affected names in permission reviews | permission-editor | Held in original backup; extraction pending |
| `e7d8a36` | Protect permission drafts during navigation and dismissal | permission-editor | Held in original backup; extraction pending |
| `6bba224` | Bind permission saves to the original requester | permission-editor | Held in original backup; extraction pending |
| `1ecfa21` | Recover interrupted permission saves across reloads | permission-editor | Held in original backup; extraction pending |
| `c88ca6d` | Exercise permission recovery in a real browser | permission-editor | Held in original backup; extraction pending |
| `202ee96` | Protect project delegation against concurrent deactivation | project-manager-delegation | Held in original backup; extraction pending |
| `3f45b7c` | Validate combined approval record coverage | scope-domain | Held in original backup; extraction pending |
| `eb56af3` | Define scoped approval transaction and coverage gates | specification-history | Held in original backup; extraction pending |
| `8d49421` | Add authorized permission editor subject discovery | permission-editor | Held in original backup; extraction pending |
| `b735b3a` | Add safe person switching to permission editor | permission-editor | Held in original backup; extraction pending |
| `7f14e4b` | Limit user directory responses to consumed fields | identity-projections | Held in original backup; extraction pending |
| `1eb13ec` | Add scoped people directory reads | people-directory | Held in original backup; extraction pending |
| `981d0e3` | Resolve approval names without directory access | identity-projections | Held in original backup; extraction pending |
| `6b5dbae` | Authorize identity-only project team choices | project-team-choices | Held in original backup; extraction pending |
| `4c00660` | Document project form permission integration boundaries | specification-history | Held in original backup; extraction pending |
| `3bb62ac` | Limit session identity responses to display fields | identity-projections | Held in original backup; extraction pending |
| `4ac30fa` | Add scoped time-entry reads without financial metadata | time-readers | Held in original backup; extraction pending |
| `c80233b` | Keep invoice identities out of time-entry responses | time-readers | Held in original backup; extraction pending |
| `5ec183a` | Fence time-entry writes against account deactivation | time-writer-activity | Held in original backup; extraction pending |
| `228e151` | Clarify timesheet context and locked calendar behavior | specification-history | Held in original backup; extraction pending |
| `4294aa3` | Isolate permission browser fixtures and retain test assets | browser-fixture-tooling | Held in original backup; extraction pending |
| `3308926` | Keep permission profile name uniqueness independent of database locale | permission-storage | Held in original backup; extraction pending |
| `8af562e` | Record passing permission regression gates | specification-history | Held in original backup; extraction pending |
| `9b53182` | Verify profile capacity and confirm timesheet discovery | permission-editor, specification-history, time-readers | Held in original backup; extraction pending |
| `60f60f9` | Add scoped Timesheet person discovery | time-readers | Held in original backup; extraction pending |
| `5faed76` | Bind Timesheet page reads to requester and subject | time-readers | Held in original backup; extraction pending |
| `e1ddd9a` | Connect Timesheet to complete scoped page reads | timesheet-consumer-commands | Held in original backup; extraction pending |
| `48a6533` | Define person-bound Timesheet command contracts | timesheet-consumer-commands | Held in original backup; extraction pending |
| `02c4245` | Authorize person-bound Timesheet commands atomically | timesheet-consumer-commands | Held in original backup; extraction pending |
| `a0632a8` | Bind Timesheet navigation and actions to the selected person | timesheet-consumer-commands | Held in original backup; extraction pending |
| `b8b1c60` | Bind weekly submission to the active Timesheet context | timesheet-consumer-commands | Held in original backup; extraction pending |
| `8722320` | Add browsable permission change history | permission-audit | Held in original backup; extraction pending |
| `2078a13` | Format permission history verification notes | specification-history | Held in original backup; extraction pending |
| `2f5357f` | Verify permission history and scoped Timesheet browser flows | browser-fixture-tooling, permission-audit, timesheet-consumer-commands, permission-editor | Held in original backup; extraction pending |
| `68bbaae` | Fix Timesheet modal focus and long-label layout | timesheet-consumer-commands | Held in original backup; extraction pending |
| `e29f4d8` | Reload Timesheet state when switching people | timesheet-consumer-commands | Held in original backup; extraction pending |
| `5f7895c` | Preserve selected dates and drag offsets in Timesheet | timesheet-consumer-commands | Held in original backup; extraction pending |
| `84d5352` | Expose authenticated project manager delegation | project-manager-delegation | Held in original backup; extraction pending |
| `c4e83c8` | Record project delegation verification and next integration gate | specification-history | Held in original backup; extraction pending |
| `a25e544` | Serialize invoice writes before user revocation | legacy-access-writers | Held in original backup; extraction pending |
| `774f60a` | Record invoice revocation verification and next integration gates | specification-history | Held in original backup; extraction pending |
| `1879b8a` | Preserve requester identity when reloading permission editors | permission-editor | Held in original backup; extraction pending |
| `dab6885` | Record editor reload verification and remaining directory integration | specification-history | Held in original backup; extraction pending |
| `ee16165` | Connect scoped People directory and requester-bound editing | people-directory | Held in original backup; extraction pending |
| `1b41033` | Record People integration verification and remaining report scope | specification-history | Held in original backup; extraction pending |
| `75f13a1` | Add scoped detailed time report reads | time-report-readers | Held in original backup; extraction pending |
| `cbc78a8` | Apply scoped permissions to time report spreadsheets | time-report-exports | Held in original backup; extraction pending |
| `09bd15f` | Apply scoped permissions to streamed time exports | time-report-exports | Held in original backup; extraction pending |
| `93aaa68` | Support multi-selection filters in time downloads | time-report-exports | Held in original backup; extraction pending |
| `a23804f` | Include full-period totals in scoped time reports | time-report-readers | Held in original backup; extraction pending |
| `7266abb` | Connect scoped time reports with bound downloads | time-report-consumer | Held in original backup; extraction pending |
| `41ff137` | Add scoped time report grouping with exact totals | time-report-readers | Held in original backup; extraction pending |
| `e2e66fb` | Connect scoped time report groups and detail navigation | time-report-consumer | Held in original backup; extraction pending |
| `ca170c0` | Export scoped time groups to Excel with release authorization | time-report-exports | Held in original backup; extraction pending |
| `2b59b58` | Stream grouped time reports with scoped authorization | time-report-exports | Held in original backup; extraction pending |
| `ecac66b` | Add scoped individual time reports and nested breakdowns | time-report-consumer | Held in original backup; extraction pending |
| `de8f9ad` | Filter time reports to active projects | time-report-consumer | Held in original backup; extraction pending |
| `2497dbe` | Enforce scoped permissions in the project editor | project-editor-permissions | Held in original backup; extraction pending |
| `2631186` | Enforce scoped project reads across pages and exports | project-read-permissions | Held in original backup; extraction pending |
| `1b81680` | Record project permission delivery acceptance | specification-history | Held in original backup; extraction pending |
| `f6e8bf1` | Enforce task catalog and tracking read permissions | task-permissions-lifecycle | Held in original backup; extraction pending |
| `8dd61d4` | Enforce current task creation and project scope permissions | task-permissions-lifecycle | Held in original backup; extraction pending |
| `facfb49` | Protect task rate edits with explicit intent and current permissions | task-permissions-lifecycle | Held in original backup; extraction pending |
| `ac4c90c` | Authorize task activity changes and guard running timers | task-permissions-lifecycle | Held in original backup; extraction pending |
| `0591407` | Preserve project task archival across restores and imports | task-permissions-lifecycle | Held in original backup; extraction pending |
| `979a594` | Enforce scoped project task linking and rate currency | task-permissions-lifecycle | Held in original backup; extraction pending |
| `5561f14` | Add permission-aware task catalog management | task-consumers | Held in original backup; extraction pending |
| `dcadcee` | Add atomic task creation to the task catalog | task-consumers | Held in original backup; extraction pending |
| `8c1bf9b` | Add task archive and restore controls to project editing | task-consumers | Held in original backup; extraction pending |
| `db3935d` | Filter time reports and downloads by billability | time-report-consumer | Held in original backup; extraction pending |

Additional original #217 commit `dd141c5`: replaced by #220 for delivery, with
its original branch left untouched and open. All production/test changes are
carried with the documented #216 adaptation. Its existing feature record is
preserved with historical evidence explicitly separated from new-head results.
Seven of its eight added SQLx descriptors are emitted by the new preparation;
`query-7d5c8693570ee23f36fdb48ff0c4412ba250383ee2fa399d94fefe9d16767381.json`
already exists in `02f7b58`. Do not merge both #217 and #220 as separate fixes.

## Iteration log and next action

The ledger is published as draft [#218](https://github.com/numtide/horae/pull/218)
at `c0ce2bd`. Subsequent delivery results will update that same PR; no separate
tracking documents are needed.

### 2026-10-06 — Inventory, gate and recoverability

Read-only inventory ran while #216 was queued. The existing watch process
observed queue CI success without rerunning it or requesting a merge.
The pre-existing queue request merged #216; this separation work performed
no merge. Then backups and recovery checks passed, and two clean isolated
worktrees were created from the verified merge.

The ledger contains all 145 original commits and all 18 unpublished paths.
Targeted `nix fmt` passed. The dev shell does not expose `mdformat` directly;
the repository formatter was used instead. No Rust extraction tests or
new-head Nix gates have run yet.

### 2026-10-06 — Legacy-reader extraction started

Extracted the shared manager snapshot and all seven reader regressions plus the
registered-session test from #217. The three production readers now use that
snapshot. The #216 invoice query, optional client filter and stable ordering
remain unchanged. The HTTP harness receives only the exact endpoint matcher
and the reader cases; no unrelated canonical consumers are copied.

Necessary adaptation: inline the exact organization SHARE query from the
existing helper into snapshot authorization. The two test fixtures retain the
original NO KEY UPDATE query directly. This avoids importing unused writer
modes or modifying master writers merely to support the extracted reader.
No schema, new dependency, UI, policy activation or ordinary writer changes.

Source review confirms current user revocations acquire the organization lock
before user rows; the extracted readers take organization then actor SHARE and
commit before returning. Invoice headers/lines use the same repeatable-read
transaction. Full adversarial acceptance and runtime results remain pending.

Focused validation is running in session `63131` through the Nix shell, using
`.scratch/verify-readers.sh` in the extraction worktree. Each invocation starts
a fresh private PostgreSQL cluster and applies only the base migrations there;
no existing database is used. It reuses ignored Cargo artifacts, not source
files, from the existing target directory. No test has been declared passed
while dependency compilation is still running.

### 2026-10-06 — Focused reader regressions passed

Session `63131` completed successfully: all seven snapshot reader tests passed
(2.96 seconds after the initial 10m57s build). They exercise all three readers,
both revocation orders, direct and organization-gated writers, tenant/filter/
not-found behavior, coherent invoice headers/lines, cancellation and inherited
connection settings. Other test binaries were filtered, not verified by this run.

`nix fmt` completed successfully, changing only the new snapshot module's
formatting. The post-format full workspace server/core suite is running in
session `71462`, through the same disposable-database wrapper. This includes
the registered HTTP harness and #216's client regressions; no result is claimed
until it finishes. No original source worktree was changed.

Next: collect the full suite, regenerate SQLx and run offline/lint/Nix gates.
Review and publish the scoped PR only with honest readiness/evidence, then
continue the remaining behavior groups. The next foundation candidate is the
pure scope/catalog code: retain master’s new `client` module when adding
`permissions`, and leave rates/approval/person-management consumers with their
actual dependencies. Do not copy the old core module root wholesale.

### 2026-10-06 — Pure scope/catalog extraction started

Created `refactor/permission-domain-foundation` independently from `02f7b58`.
Reused the existing source and tests without changing their semantics. Reviewed
the existing `record-scope.md` and `grant-catalog.md` contracts: coverage denies
inactive/foreign/misattributed facts, assignments alone grant nothing, saved
selections reject missing prerequisites rather than adding authority, and an
Administrator selection does not prove Administrator identity. There are no
runtime callers in this delivery and no legacy role conversion or activation.

Core tests run in session `34224` with a separate temporary Cargo target, so
the reader suite's binaries and artifact lock are not disturbed. Formatting
check is session `29855`. Results and the resulting PR are still pending.

### 2026-10-06 — Full reader suite and foundation delivery

Reader session `71462` finished with exit zero: 826 app-unit tests, 180 tests
across the nine integration binaries and 121 core tests passed (1,127 total).
The 11 existing ignored scale/stress tests remain unchanged, not counted as passed.
This includes the authenticated reader cases, all seven snapshot regressions
and all 35 Clients tests from #216. No Rust source changed after this run.
SQLx preparation now runs in session `84393` against another private database,
with `--workspace -- --features server --all-targets`. All test entry points
were touched before preparation to avoid losing cached test-query metadata.

Foundation sessions `34224`, `29855` and `11685` finished successfully:
158 core tests (37 extracted plus 121 existing), `nix fmt -- --ci`, and core
Clippy with all targets, warnings denied and performance lints. All five new
source blobs match `524c29e` exactly; the sixth file adds only the module export.
Unsigned commit `ec7ddbd` is published in draft #219. It is independent of the
reader extraction and does not change app behavior. Full Flake Check remains
pending; local core checks alone do not make it ready to merge.
Its head was confirmed as `ec7ddbd1f1271db523abc35d75a82d8b6fc3bb29`;
CI run [37474017234](https://github.com/numtide/horae/actions/runs/37474017234)
started Flake Check and Format. No outcome is assumed from their running state.

Next: collect SQLx preparation, verify offline all-target compilation and lint,
then publish the reader extraction. Collect #219's own CI before accepting it.
Continue the remaining source groups and specification-hunk reconciliation;
the original PRs remain open and unchanged.

### 2026-10-06 — Independent reader PR published

SQLx session `84393` passed, preserving all 1,003 existing descriptors byte for
byte and adding 18. Offline server Clippy with all targets passed in `85490`;
WASM Clippy passed in `53119`, both with warnings denied, performance lints and
`DATABASE_URL` unset. Final `nix fmt -- --ci` passed in `59584`.
No Rust source changed after the full test run; only generated SQLx metadata
and the carried feature verification record were added afterward.

Unsigned commit `bc0c7a0bf2822c1e61568a95b53df651fe40e7c4` is published as draft
#220. Its 27 paths comprise eight Rust source/test paths, 18 SQLx descriptors
and the existing feature record. Adversarial review found no high/critical
issue in the extracted source; that is not a replacement for the pending
required Flake Check. #219 and #220 are independent deliveries from the same
#216 base, so neither requires the other to merge.

Next: collect the exact-head CI outcomes for #219/#220 without frequent polling.
Continue with domain-dependent rules and storage extraction, keeping storage
policy at zero and carrying the complete relevant tests. Reconcile the remaining
specification hunks and every source group; the overall separation is unfinished.
