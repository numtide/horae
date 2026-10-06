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
| Legacy report/invoice readers from #217, [#220](https://github.com/numtide/horae/pull/220) | `fix/report-reader-authority-master`, `.worktrees/report-reader-authority-master` | `02f7b58` | Draft at `bc0c7a0`; 1,127 tests passed, 11 existing ignored; SQLx, offline server/WASM lint, formatting and GitHub Flake Check passed; Nixbot build pending |
| Pure record scopes and grant catalog, [#219](https://github.com/numtide/horae/pull/219) | `refactor/permission-domain-foundation`, `.worktrees/permission-domain-foundation` | `02f7b58` | Draft at `ec7ddbd`; 158 core tests, core Clippy, formatting and GitHub Flake Check passed; Nixbot build pending; no runtime integration |
| Pure rate/management/approval rules, [#221](https://github.com/numtide/horae/pull/221) | `refactor/permission-domain-gates`, `.worktrees/permission-domain-gates` | #219 `ec7ddbd` | Draft at `539316c`; 187 core tests, core Clippy, formatting and full local Flake Check passed; CI required after retargeting |
| Non-activating permission storage, [#222](https://github.com/numtide/horae/pull/222) | `refactor/permission-storage-foundation`, `.worktrees/permission-storage-foundation` | #219 `ec7ddbd` | Draft at `e9695fd`; 1,170 tests passed, 11 existing ignored; SQLx, offline server/WASM lint, formatting and full local Flake Check passed; remote checks pending |
| Legacy import report conversion lock order, [#223](https://github.com/numtide/horae/pull/223) | `fix/import-report-lock-order`, `.worktrees/import-report-lock-order` | `02f7b58` | Draft at `c8f95ac`; 1,125 tests passed, 11 existing ignored; SQLx, offline server/WASM lint format and current-head GitHub Flake Check passed; Nixbot pending |
| Interrupted import session cleanup, [#224](https://github.com/numtide/horae/pull/224) | `fix/import-session-cleanup`, `.worktrees/import-session-cleanup` | `02f7b58` | Draft at `be57f0e`; 1,123 tests passed, 11 existing ignored; SQLx, offline server/WASM lint format and current-head GitHub Flake Check passed; Nixbot pending |
| Current authority for organization branding writes, [#225](https://github.com/numtide/horae/pull/225) | `fix/branding-current-authority`, `.worktrees/branding-current-authority` | `02f7b58` | Draft at `f2d6bd4`; full suite, SQLx, offline server/WASM lint format and current-head GitHub Flake Check passed; Nixbot pending |
| Internal reusable-profile commands, [#226](https://github.com/numtide/horae/pull/226) | `refactor/permission-template-commands`, `.worktrees/permission-template-commands` | #222 `e9695fd` | Draft at `82d15f3`; 1,189 tests, SQLx, offline server/WASM Clippy, format and full local Flake Check passed; CI required after retargeting; no endpoints or activation |
| Current authority for user creation/role/activity, [#227](https://github.com/numtide/horae/pull/227) | `fix/user-mutation-authority`, `.worktrees/user-mutation-authority` | `02f7b58` | Draft at `142eda1`; 1,127 tests, SQLx, offline server/WASM Clippy, format and current-head GitHub Flake Check passed; Nixbot build pending |
| Assignment authority and project writer coordination, [#228](https://github.com/numtide/horae/pull/228) | `fix/project-access-lock-order`, `.worktrees/project-access-lock-order` | #227 `142eda1` | Draft at `0e1e675`; 1,137 tests, SQLx, offline server/WASM Clippy, format and full local Flake Check passed; master-targeted CI required after retargeting |
| Durable CSV preparation outside SQL transactions, [#231](https://github.com/numtide/horae/pull/231) | `fix/csv-batch-transaction-boundary`, `.worktrees/csv-batch-transaction-boundary` | `02f7b58` | Draft at `e9898ed`; 1,122 tests, SQLx, offline server/WASM Clippy and format passed; required CI pending |
| Financial snapshot reader authority, [#232](https://github.com/numtide/horae/pull/232) | `fix/financial-snapshot-authority`, `.worktrees/financial-snapshot-authority` | #220 `bc0c7a0` | Draft at `0bb5721`; 1,141 tests, SQLx, offline server/WASM Clippy and format passed; full local Nix running; required CI after retargeting |
| Invoice writer/revocation ordering, [#233](https://github.com/numtide/horae/pull/233) | `fix/invoice-write-authority`, `.worktrees/invoice-write-authority` | Integration base `0046dad` combining #227/#228 and #220/#232 | Draft at `8a6cb2a`; 1,162 tests, SQLx, offline server/WASM Clippy and format passed; full local Nix running; retarget to master after prerequisites, do not merge into integration base |
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
| `757f43d` | Enforce tenant and administrator boundaries for assignments | legacy-access-writers | Rust/test/cache changes in #228 with subsequent coordination repair; specification hunks retained |
| `d3a4ff3` | Document profile reapplication and import permission boundaries | specification-history | Held in original backup; extraction pending |
| `dcf21ef` | Specify permission migration safeguards and rate-scope verification | specification-history | Held in original backup; extraction pending |
| `b3ee8da` | Align authorization governance with scoped permission profiles | specification-history | Held in original backup; extraction pending |
| `b569c75` | Extend permission specification to confirmed web domains | specification-history | Held in original backup; extraction pending |
| `f5cf02d` | Record expense scope defaults and lifecycle permission gaps | specification-history | Held in original backup; extraction pending |
| `6ce9071` | Document expense action scope and independent lock states | specification-history | Held in original backup; extraction pending |
| `b8b105e` | Document current-account permission research and evidence gaps | specification-history | Held in original backup; extraction pending |
| `6443d56` | Record current Harvest permission evidence and reference conflicts | specification-history | Held in original backup; extraction pending |
| `e8dcb77` | Add typed permission catalog and built-in profile selections | scope-domain | Code/tests in #219; specification hunks retained for reconciliation |
| `b80f8ab` | Recheck administrator authority during user access changes | legacy-access-writers | Three exact Rust blobs and regenerated cache in #227; specification hunks retained for reconciliation |
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
| `6e61593` | Validate person-management grant compatibility and self-links | scope-domain | Code/tests in #221; specification hunks retained for reconciliation |
| `ec35446` | Store versioned permission profiles without activating new policy | permission-storage | Storage/schema/name-validator code/tests in #222; specification hunks retained for reconciliation |
| `d50c979` | Add audited permission template commands | permission-profile-transactions | Command/test/receipt/cache hunks in #226 with later hardening; specification hunks retained for reconciliation |
| `f5e0dde` | Apply permission profiles with atomic scope changes | permission-profile-transactions | Model serialization in #222; strict template receipt comparison in #226; profile commands and remaining hunks retained |
| `9a7e05d` | Add audited project manager delegation | project-manager-delegation | Held in original backup; extraction pending |
| `fc85231` | Add administrator-only permission audit lookup | permission-audit | Held in original backup; extraction pending |
| `c3d17cb` | Prevent deadlocks during legacy import report conversion | import-transaction-lifecycle | Conversion source/tests/SQLx in #223; specification hunks retained for reconciliation |
| `3ae8e08` | Coordinate project access changes before locking resources | legacy-access-writers | Rust/test/cache changes in #228; organization SHARE query also reused in #220; specification hunks retained |
| `907bc88` | Recheck authority when saving organization branding | branding-authority | Source/test/cache hunks in #225; specification hunks retained for reconciliation |
| `d7a5a21` | Revalidate administrator authority for Harvest connection changes | import-authority | Held in original backup; extraction pending |
| `4fac6af` | Revalidate import job command and status authority | import-authority | Exact executor-based `jobs::cancel` in #231 with CSV preparation; other command/status and specification hunks retained |
| `b4672a4` | Revalidate authority during import error downloads | import-authority | Held in original backup; extraction pending |
| `e5fcc5a` | Prepare durable CSV batches before opening transactions | import-transaction-lifecycle | Two exact Rust blobs and regenerated cache in #231; local verification passed, CI pending; specification hunks retained |
| `482b7c5` | Retain the original requester of import jobs | import-requester-provenance | Held in original backup; extraction pending |
| `e949e4c` | Drain interrupted import transactions before releasing reservations | import-transaction-lifecycle | Production/test/cache hunks in #224; specification hunks retained for reconciliation |
| `c0cfb8f` | Scope rate permissions to their owning resource | scope-domain | Code/tests in #221; specification hunks retained for reconciliation |
| `5d51b0e` | Expose the current person's permission snapshot | own-permissions | Held in original backup; extraction pending |
| `22ffdab` | Recheck manager access for financial snapshots | manager-snapshot-consumers | Shared helper/queries in #220; consumer/tests/cache in #232, local suite and both target lints passed, Nix pending; specification hunks retained |
| `15c82ef` | Recheck manager access in invoice editor snapshots | manager-snapshot-consumers | Consumer/tests/cache in #232; fixture adaptations documented, local suite and both target lints passed, Nix pending; specification hunks retained |
| `dc822a0` | Recheck manager authority before delivering exports | export-authority | Held in original backup; extraction pending |
| `108594e` | Revalidate project scope before delivering exports | export-authority | Held in original backup; extraction pending |
| `dcf4ac8` | Recheck current permissions during CSV downloads | export-authority | Held in original backup; extraction pending |
| `0793ce7` | Revalidate budget email authority before delivery | budget-email-authority | Held in original backup; extraction pending |
| `8aac739` | Add read-only permission migration diagnostics | permission-preflight | Held in original backup; extraction pending |
| `8c15bfe` | Show own permissions in Settings | own-permissions | Held in original backup; extraction pending |
| `300d1e9` | Expose administrator permission history | permission-audit | Held in original backup; extraction pending |
| `03e90b1` | Connect permission editor previews and commands | permission-editor | Template DTOs, command hardening/tests and administration helpers in #226; editor/profile/remaining hunks retained |
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
| `3f45b7c` | Validate combined approval record coverage | scope-domain | Code/tests in #221; specification hunks retained for reconciliation |
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
| `3308926` | Keep permission profile name uniqueness independent of database locale | permission-storage | Migration/storage regressions in #222; command lookup changes remain with template commands |
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
| `a25e544` | Serialize invoice writes before user revocation | legacy-access-writers | Five Rust/test changes and eight SQLx additions in #233 on integrated #220/#227/#228/#232 prerequisites; suite/cache/offline lints passed, Nix running; specification hunks retained |
| `774f60a` | Record invoice revocation verification and next integration gates | specification-history | Held in original backup; extraction pending |
| `1879b8a` | Preserve requester identity when reloading permission editors | permission-editor | Held in original backup; extraction pending |
| `dab6885` | Record editor reload verification and remaining directory integration | specification-history | Held in original backup; extraction pending |
| `ee16165` | Connect scoped People directory and requester-bound editing | people-directory | Held in original backup; extraction pending |
| `1b41033` | Record People integration verification and remaining report scope | specification-history | Held in original backup; extraction pending |
| `75f13a1` | Add scoped detailed time report reads | time-report-readers | Held in original backup; extraction pending |
| `cbc78a8` | Apply scoped permissions to time report spreadsheets | time-report-exports | Held in original backup; extraction pending |
| `09bd15f` | Apply scoped permissions to streamed time exports | time-report-exports | Native stored-row decoder in #222; streamed exports and remaining hunks retained |
| `93aaa68` | Support multi-selection filters in time downloads | time-report-exports | Held in original backup; extraction pending |
| `a23804f` | Include full-period totals in scoped time reports | time-report-readers | Held in original backup; extraction pending |
| `7266abb` | Connect scoped time reports with bound downloads | time-report-consumer | Held in original backup; extraction pending |
| `41ff137` | Add scoped time report grouping with exact totals | time-report-readers | Held in original backup; extraction pending |
| `e2e66fb` | Connect scoped time report groups and detail navigation | time-report-consumer | Held in original backup; extraction pending |
| `ca170c0` | Export scoped time groups to Excel with release authorization | time-report-exports | Held in original backup; extraction pending |
| `2b59b58` | Stream grouped time reports with scoped authorization | time-report-exports | Held in original backup; extraction pending |
| `ecac66b` | Add scoped individual time reports and nested breakdowns | time-report-consumer | Held in original backup; extraction pending |
| `de8f9ad` | Filter time reports to active projects | time-report-consumer | Held in original backup; extraction pending |
| `2497dbe` | Enforce scoped permissions in the project editor | project-editor-permissions | Pure RateEdit code/tests in #221; project editor and remaining hunks retained |
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

### 2026-10-06 — Dependent domain rules published; storage isolated

#221 contains the six unchanged source/test files for person-management
prerequisites, financial field gates (including explicit rate intent) and combined
approval record coverage, plus their module exports. All source blobs match
`db3935d`. Its 187 core tests passed, including the 158 tests of #219 and 29
additional tests; core all-target Clippy and formatting passed. These predicates
do not implement assignment writes, rate mutations or approval lifecycle.
Their documented trusted-input, complete-selection and independent lock boundaries
remain unchanged; source review found no high/critical issue within this scope.

The repository Actions workflow filters pull requests to base `master`.
Consequently #221 does not inherit #219's check result and cannot claim its own
Actions run. Full `nix flake check -L --max-jobs 1 --cores 2` is running locally
in session `98201` on committed head `539316c`. Required CI must run again after
eventual retargeting; no workflow filter or required check was weakened.
At the latest observation #219/#220 Format and nix-eval passed, while their
full Flake Check/build jobs remained in progress.

Storage is a separate sibling branch on #219, not dependent on #221. It carries
unchanged migrations 0042/0047, the final strict storage loader/models and storage
regressions, and the original profile-name validator. Command modules, transaction
configuration helpers and runtime readers are deliberately excluded. The existing
non-test dead-code expectation documents that this internal storage is not yet
activated; no test is disabled or weakened. Serializer additions originate in
`f5e0dde`; the native stored-row decoder originates in `09bd15f`. Those source
commits' commands/export consumers remain separate pending work.

The migration identities and checksums are retained. A private database first
applied this subset (42/47), then the preserved original migration directory:
43–46/48 applied successfully and all seven versions were verified present.
This verifies deferred migration compatibility without renumbering migrations
or modifying a real database. Full storage server/core tests are now running in
session `96597`; formatting passed unchanged. SQLx/offline/lint/Nix acceptance
for storage remains pending, and no storage PR has been published yet.

Next: collect storage tests, regenerate its SQLx cache and verify offline/lint;
collect the live local #221 Flake Check without restarting it. Then publish the
storage extraction and continue command/read-consumer groups and spec reconciliation.

### 2026-10-06 — Storage published; independent conversion repair started

Storage session `96597` finished successfully: 830 app tests, 180 integration
tests and 160 core tests passed (1,170 total), with the 11 pre-existing manual
scale/stress tests still ignored. This includes all 11 original storage tests
and both original name-validation tests. No Rust source changed after that run.
SQLx preparation passed in `31386`: all 1,003 base descriptors are unchanged;
the 19 additional descriptors match the original `db3935d` blobs exactly.
Offline all-target server Clippy (`2494`), WASM Clippy (`92443`) and final
formatting (`39928`) passed, with warnings denied and no format changes.

Unsigned commit `e9695fda224d1f8bc22e8e7fb7e8fc0a43fa4625` is published in
draft #222. Its 28 paths comprise nine Rust/schema paths and 19 query descriptors.
Storage review checked composite tenant FKs, exact restoration without inference,
restricted template deletion, non-activation, Unicode collision rollback and
the absence of endpoints/callers. No high/critical finding within that boundary;
future command authorization, profile capacity and cutover remain out of scope.
The two migrations, models, storage tests and final catalog files match the
preserved final source. Full local Flake Check is running on this committed head
in session `50374`; master-only Actions filtering still requires later CI.

The existing #221 local Flake Check (`98201`) remains live: release server/WASM
build completed and browser checks are progressing. This is not a terminal pass.
The latest GitHub observations still show #219/#220 Flake Check and nix-build
running, with Format and nix-eval successful. No check was restarted or bypassed.

Created an independent `fix/import-report-lock-order` worktree from `02f7b58`.
Its two source/test files match `c3d17cb` exactly: discovery without the job lock,
organization SHARE before job UPDATE, READ COMMITTED revalidation and rediscovery
after another worker/converter changes the candidate. All five original races
are retained. Existing size-one pool, archived-error and state-preservation tests
remain intact. The later `482b7c5` requester-provenance assertions stay with that
schema/feature, not this extraction. No original worktree was modified.
Full workspace server/core tests are running on a private database in `58270`.

Next: collect conversion tests, regenerate its SQLx cache, complete adversarial
and offline/lint checks, and publish its independent draft PR. Collect the live
#221/#222 Nix outcomes without restarting either process. Continue the remaining
behavior groups and specification-hunk mapping; the overall separation is not
complete and no merges or original-PR closures have been performed.

### 2026-10-06 — Conversion repair published; session cleanup isolated

Session `58270` passed the full conversion workspace suite: 824 app tests,
180 integration tests and 121 core tests (1,125 total), with 11 pre-existing
manual scale/stress tests ignored. All five original concurrency regressions
passed, along with the existing legacy conversion, size-one pool and Clients
regressions. Formatting (`33716`) passed unchanged. SQLx preparation (`62415`)
preserved 1,002 base descriptors, removed the superseded locking-discovery
descriptor and added 14 including its replacement (1,016 total). All 14 match
the original `c3d17cb` source. Offline all-target server Clippy (`33078`) passed;
WASM Clippy (`79587`) also passed. No Rust source changed after the full suite.

Unsigned commit `c8f95ac5586dfed5549d4e394516f12b02339741` is published as draft
#223 on master. Adversarial review traced the one-connection conversion and its
archive/checkpoint, claim, cleanup and startup callers: exact tenant/job recheck,
fresh post-wait payload, compatible organization SHARE versus worker FK KEY SHARE,
no duplicate archive or invalidation of an already-converted live lease, and
atomic failure rollback. No high/critical issue was found in this extraction.
This accounts for T065–T067's code, not full T039/T040/T042 or policy activation.
Required exact-head CI remains pending.

The next independent branch carries exactly the `e949e4c` source/test hunks;
stable patch ID `3168407d233c130834dd0fa7328552224993c1f9` matches the original.
It drains abandoned SQLx responses, tolerates only disposal-time `3B001`, sends
full ROLLBACK before advisory unlock and closes the reserved connection. The
original tests cover one/two nested savepoints, an untracked server transaction,
retained committed values, immediate single-connection retries and backend failure.
Source review checked the pinned SQLx 0.8.6 flush/transaction implementation and
both API/CSV worker-join paths against the existing T089–T091 contract. Format
passed in `66687`; runtime/SQLx/lint acceptance is still pending.
The full workspace server/core suite passed in session `31728` through
`.scratch/verify-cleanup.sh` on a fresh private PostgreSQL cluster: 822 app,
180 integration and 121 core tests (1,123 total), with 11 pre-existing ignored.
All three cleanup regressions and existing API/CSV cancellation regressions
passed. No source changed afterward; SQLx preparation is the next gate.

Also isolated the independent branding-write repair on `02f7b58`. Both files
match `907bc88` exactly, carrying all five new authority/rollback regressions and
all prior field/no-op tests. The only production caller passes its authenticated
actor ID; the transaction retains organization UPDATE before actor SHARE, checks
current active same-tenant Manager/Admin authority even for no-ops, and commits
before the unchanged conditional plugin event. Current role writers use the
compatible organization-first order. This neither changes the branding read
endpoint nor activates future CompanyWrite policy. Formatting (`81218`) passed;
runtime, SQLx, lint and PR publication remain pending.

The durable CSV preparation candidate `e5fcc5a` has a real test dependency on
`4fac6af`'s executor-based `jobs::cancel`: master accepts only a pool, while the
original cancellation race calls it inside its transaction barrier. Preserve
that production-call regression; do not replace it with a weaker fixture write
or copy unrelated import-authority commands merely to make the tests compile.
Resolve the small shared helper or its dependency when extracting that candidate.

Next: finish session-cleanup SQLx/offline/lint verification and its
scoped PR; then verify the branding extraction. #223's exact remote head was
confirmed and CI run [37479980256](https://github.com/numtide/horae/actions/runs/37479980256)
has Format/nix-eval passed, with Flake Check/nix-build in progress.
Keep collecting the live #221/#222 local
Flake Checks (`98201`/`50374`) without restarting them. All remaining source and
specification groups still require final mapping and verification.

### 2026-10-06 — Cleanup published; completed foundation gates

The preceding response supplied the requested goal text but made no repository
progress. Resumed the existing live processes rather than restarting them.
Session `22335` completed SQLx preparation successfully: all 1,003 existing
descriptors are unchanged and seven additions match `e949e4c` exactly. Offline
all-target server/workspace Clippy (`37045`) and WASM Clippy (`77529`) passed with
warnings denied and performance lints enabled. The source/test patch still has
the original stable patch ID. Unsigned commit
`be57f0e5ebf449f446aed69ad62a84e9ee5583b2` is published as draft #224, independent
on master. Required CI is pending; no original branch was changed.

Session `98201` completed `nix flake check -L --max-jobs 1 --cores 2` with
`all checks passed!` on clean #221 head `539316c`, including its #219 dependency.
This covers the compatible x86_64-linux checks, not the omitted incompatible
systems. The PR description now records the result; master-targeted Actions
must still run when the PR is retargeted. #222's existing full check (`50374`)
is still live and has passed its release build and progressed through browser
checks. It was not restarted.

Exact-head GitHub checks were queried once after new results became available:
#219 run [37474017234](https://github.com/numtide/horae/actions/runs/37474017234)
and #220 run [37475284999](https://github.com/numtide/horae/actions/runs/37475284999)
both passed Flake Check and Format. Their separate Nixbot builds were still in
progress; do not report every remote check green. #212/#217/#208 heads remain
the preserved originals.

Branding verification is running in session `45526` on a fresh private database,
reusing the existing ignored verification wrapper. No Rust changes were made
after the original-blob comparison. The next shared extraction is now isolated
on #222: receipt migration 0043, the complete template-command implementation and
all 19 original tests match `db3935d`. It also carries the exact three command DTOs
from `permission_editor.rs`, the existing bounded administration helpers and
module registration. Only those DTOs are server-gated at this stage; editor
screens, endpoints, profile application and policy activation are not included.
Formatting passed unchanged (`42874`). Tests and SQLx are not yet accepted.

Branding's complete workspace suite subsequently exited successfully (`45526`).
SQLx preparation (`53498`) passed with all 1,003 base descriptors unchanged and
six additions matching `907bc88` exactly. Offline server Clippy is now running in
`50256`; WASM lint and publication are still pending. The template extraction's
administration helpers were additionally compared byte-for-byte with `db3935d`.
Source review checked authorization-before-replay, strict canonical intent,
tenant/principal receipt separation, atomic count/name limits and deletion,
grant/identity preservation, revision overflow and rollback. No high/critical
source finding within the internal boundary; runtime acceptance is still pending.

Branding offline workspace/server Clippy (`50256`) and WASM Clippy (`26523`)
subsequently passed. Source files still match `907bc88` byte-for-byte. Unsigned
commit `f2d6bd45a9ef0ac141797d8745430bb0d8260f3e` is published as draft #225 on
master; required CI is pending. #224's exact remote head was confirmed; run
[37482514740](https://github.com/numtide/horae/actions/runs/37482514740) has Format
passed and Flake Check in progress (Nixbot evaluation passed, build in progress).

The template-command full suite is running in session `60494`, through the
existing private-database wrapper with quiet test output to keep complete result
summaries. It tests the combined #219/#222 foundation and commands, not a mocked
standalone command. No shared-target Cargo jobs are running concurrently.

Next: collect template-command tests, regenerate SQLx and verify offline/lint
before publishing the dependent PR. Continue collecting the live storage check
(`50374`, now running app tests after browser checks). Reconcile remaining shared
writers, consumers and all specification hunks before claiming this separation
complete.

### 2026-10-06 — Template command verification and storage integration gates

Previous iteration made progress: #224/#225 were published and the next internal
command extraction was isolated. Resumed its live suite (`60494`), which passed
849 app, 180 integration and 160 core tests (1,189 total), with 11 existing manual
scale/stress tests ignored. The 19 original command tests are included unchanged.
SQLx preparation (`86502`) passed with all 1,022 base descriptors unchanged and
41 additions matching `db3935d` (1,063 total).

The first offline Clippy run (`33332`) failed to resolve `horae_core::permissions`.
The source and committed core root both export it and were unchanged; this shared
target had previously compiled the master-only branding branch without that
module. Forcing recompilation by touching only `crates/core/src/lib.rs` made the
same all-target workspace/server command pass (`58343`), with no source, cache or
lint changes. Treat this as stale shared-target build evidence, not a product
fix or a reason to weaken checks. WASM lint is the next gate; the independent
Nix build will also verify the committed source in a clean build environment.

#222's live full Flake Check (`50374`) completed with `all checks passed!` on
`e9695fd`, including its #219 foundation, browser checks and NixOS end-to-end
checks. The PR body records that compatible-system result. Its master-targeted
Actions checks remain required after retargeting; Nixbot is still in progress.
One remote query confirmed #223–#225 have Format passed and Flake Check running;
no CI rerun or merge was requested.

Dependency inspection for subsequent deliveries: profile-application commands
consume the template commands and #221's `has_person_management_grant`; their
tests also exercise real template commands. Preserve that dependency rather
than replacing it with fixtures or duplicating the domain predicate. The next
independent legacy-writer extraction is `b80f8ab` (user creation/role/activity
reauthorization), followed by assignment/project writer coordination. Later
`a25e544` only broadens `change_user_role` visibility for invoice race tests; it
does not change this helper's organization query. Keep its invoice-specific
tests and wiring with that later delivery.

WASM Clippy passed (`21514`). Unsigned commit
`82d15f33d2b7376c0a0973367ad012fbac5e752d` is published as draft #226 against
#222, with 6 source/test/schema paths and 41 generated query descriptors. The
production command, receipt migration and complete test file still match the
original blobs. Full `nix flake check -L --max-jobs 1 --cores 2` is running in
session `46319` on this clean committed head. Required Actions checks will also
be needed after master retargeting; none were weakened.

Next: isolate the three Rust files of `b80f8ab` on master, review all original
user-authority and last-administrator regressions, then verify the extraction.
Their pre-change source matches `02f7b58` exactly, so no unrelated directory or
invoice code is needed. Collect the live #226 Nix check without restarting it.
Profile application still needs both the template stack and #221; resolve that
integration explicitly before extracting its consumers. The goal remains
incomplete: many source groups and specification hunks still lack final PRs.

### 2026-10-06 — User mutation authority extraction

The preceding response only supplied goal text and made no repository progress.
Revalidated the actual goal, #216's merge and the original preserved worktree;
the original 18 unpublished paths remain untouched. Resumed live #226 full Nix
check `46319` rather than restarting it; its client release build passed and
the overall check is still running.

Created `fix/user-mutation-authority` in `.worktrees/user-mutation-authority`
from `02f7b58`. Its three Rust files match `b80f8ab` exactly: user transaction
helpers, seven new authority regressions and the updated last-admin tests.
The two existing files at `b80f8ab^` are identical to the extraction base.
No directory DTOs, invoice-test visibility change, new policy or schema was
included. This extracts the implementation of original T030/T031; T032 requires
fresh evidence, not the original quickstart's historical results. Associated
specification hunks remain preserved for reconciliation.

Source review traced the authenticated actor IDs through all three wrappers,
organization-before-actor locking, same-organization target queries, last-admin
serialization, duplicate rollback and event dispatch only after commit. Tests
exercise completed/concurrent revocation for all three commands, foreign/missing
actors, authority lock lifetime and all existing last-admin cases using actual
PostgreSQL lock observations. Directory/rate exposure and later project-writer
coordination remain outside this boundary. No high/critical source finding in
this extraction; runtime gates are not yet accepted.

Formatting passed unchanged (`57627`). Complete workspace tests are running in
`59860` through the existing private temporary PostgreSQL wrapper. Forced local
source recompilation by touching core/app/test entrypoints before reusing the
shared target; no source change resulted. Next: collect this suite, regenerate
the complete SQLx cache, verify offline server/WASM lint and publish the scoped
PR. Continue collecting #226's existing Nix handle; do not restart it.

The full workspace suite subsequently passed (`59860`): 826 app, 180 integration
and 121 core tests, 1,127 total, with 11 existing manual checks ignored. Complete
SQLx preparation is running in `39314`. #226's release server/client build has
passed; its full check has moved on to Clippy and remains live in `46319`.

Next-boundary review: `757f43d`'s assignment repair adds an actor lock before
resource/FK work. Preserve its later `3ae8e08` organization/project lock-order
repair together with the affected project-editor, invoice and time-entry race
regressions; do not call the early assignment commit alone the finished writer
boundary. Most pre-change paths of `3ae8e08` match the current base; the project
creation helper differs because #216 introduced shared client validation, which
must be retained. Its user/editor race additionally depends on this user-write
extraction. These dependencies were inspected, not implemented or certified.

SQLx preparation passed (`39314`): all 1,003 base descriptors unchanged, four
additions byte-identical to `b80f8ab`. Offline all-target workspace/server Clippy
passed (`19350`) with warnings and performance lints denied. Unsigned commit
`142eda198f8c3aa94eef17285cfe6b19ca2f4b05` is published as draft #227 on master;
its three Rust files are unchanged from the tested extraction. WASM Clippy is
running in `59244`; required CI remains pending. #226's existing full check
continues through browser checks in `46319`.

WASM Clippy subsequently passed (`59244`). #227's remote head and draft/master
base were confirmed; [CI run 37486457726](https://github.com/numtide/horae/actions/runs/37486457726)
has Flake Check/Format running, Nixbot evaluation passed and build running. No
merge or rerun was requested. Next: extract the assignment/project coordination
boundary (`757f43d` plus `3ae8e08`) with its original race tests on #227, retaining
#216's client validation; verify that combined stack. Keep collecting #226's
live full check without restarting it. Remaining source/specification groups
still require mapping and verification; the goal is not complete.

### 2026-10-06 — Project writer coordination extraction

The preceding iteration made progress: #227 was published with its full local
suite, SQLx and both target lints passed. Reconfirmed #216's merge and resumed
#226's live `46319` check; browser checks continue, without a restart.

Created `fix/project-access-lock-order` in `.worktrees/project-access-lock-order`
on #227 `142eda1`. Applied the Rust changes of `757f43d` and `3ae8e08` together.
Fourteen of fifteen files match `3ae8e08` byte-for-byte; the sole difference is
the client validation already merged in #216, which remains intact. Original
assignment, task, editor, invoice-FK, entry-cascade and user-revocation tests are
preserved. No UI, schema, policy activation or real-data change is included.

The dependency on #227 is real: the editor/revocation race exercises its actual
actor-aware user creation and role-change helpers. This delivers original
T024–T026 assignment work and T068–T070 project-family coordination, not the
complete T042 lock hierarchy or canonical-permission enforcement. Specification
hunks remain preserved in the originals pending reconciliation.

Formatting passed unchanged (`6430`). Full workspace tests for this combined
stack are running in `75426` on a fresh private PostgreSQL cluster. Next: finish
source/trigger/caller review, collect tests, regenerate SQLx and verify offline
server/WASM lint before publication; retain #226's live full-check handle.

The suite passed (`75426`): 836 app, 180 integration and 121 core tests, 1,137
total, with 11 existing manual tests ignored. This includes all ten added
regressions and the dependent #227 tests. Source review traced every production
creation-actor/task-enablement caller, checked the retained editor isolation and
parent lock mode, the existing revision triggers/member cascades, tenant-safe
assignment recheck and post-commit events. No high/critical finding in this
bounded extraction; full canonical/historical-writer integration is not claimed.

Complete SQLx regeneration passed (`30003`): 28 additions match `3ae8e08`, four
superseded descriptors were removed and 1,003 base descriptors are unchanged
(1,031 total). The removed queries are the replaced unscoped assignment deletion,
two late currency SHARE reads and task-link resource query, not lost test cache.
Unsigned commit `0e1e6759cff0b80a046ced4694a78022f096ab3d` is published as draft
#228 against #227. Offline server lint runs in `17183`; full Nix check runs on
the clean committed stack in `1662`. WASM lint remains to run. Master-targeted
Actions will also be required after retargeting.

Meanwhile isolated the independent CSV preparation work in
`.worktrees/csv-batch-transaction-boundary`, branch
`fix/csv-batch-transaction-boundary`, based on `02f7b58`. Both parser/test files
match `e5fcc5a`; the only additional source hunk is the exact generic
`jobs::cancel` from `4fac6af`, needed by the original cancellation barrier test.
No other import admission/status commands were copied and no test was weakened.
Formatting passed (`26556`); runtime/cache/lint gates are not yet accepted.
Do not run concurrent local Cargo checks in the shared target; start its suite
after #228's server/WASM checks finish. #226's full check remains live in `46319`.

### 2026-10-06 — Resume extraction verification

The preceding response restated a goal rather than advancing repository state
(no progress). Read the actual saved objective and revalidated #216 as merged
at `02f7b58`. Existing extraction branches and unpublished changes remain in
place. Both original Nix handles (`46319`, `1662`) were confirmed live and
resumed, not restarted. #226 has passed SQLx preparation and its test derivation;
the overall check remains pending. #228's client release build has passed.

#228's offline all-target workspace/server Clippy passed (`17183`), followed by
WASM Clippy (`40125`); neither required source changes. Its full Nix check and
master-targeted Actions after retargeting remain acceptance gates.

Rechecked the CSV extraction against `e5fcc5a`: both original source/test blobs
are unchanged. The sole `jobs.rs` adaptation is the original executor-generic
cancellation helper, keeping its SQL predicates unchanged and allowing the
original test to cancel through its held publication barrier. Reviewed the
preparation loop, first/subsequent batch boundaries, preview rollback/resume,
parser shutdown and unchanged pool callers. No high/critical source finding in
this bounded extraction; runtime verification remains pending. Started the full
workspace suite on a fresh private PostgreSQL cluster after #228's local lint
finished; no concurrent local Cargo command uses the shared target.

Next: collect the CSV suite, regenerate its complete SQLx cache, verify both
offline targets and publish only after these gates pass. Collect both live Nix
checks. Many original source/specification groups still require extraction and
reconciliation; the separation goal is not complete.

#226's original full Nix check completed successfully (`46319`, exit 0,
`all checks passed!`), including browser, SQLx, tests and NixOS end-to-end
checks on the compatible host system. No restart was required. A single remote
query confirmed current-head GitHub Flake Check and Format success for #223,
#224 and #225; their Nixbot builds remain in progress. #227's Format passed
and Flake Check remains running. These results do not waive master-targeted CI
for stacked PRs after retargeting.

Isolated the next writer boundary in `.worktrees/invoice-write-authority`,
branch `fix/invoice-write-authority`, on #228 `0e1e675`. It contains exactly the
five Rust/test changes from `a25e544`, not its earlier financial-reader changes.
The new 564-line test file matches original blob `36ca710` byte-for-byte.
Existing #216 invoice client filtering and #228 coordination tests remain.
Formatting passed unchanged (`41998`); no runtime/cache acceptance yet.
The real dependencies are #228's organization helper and #227's actor-aware
user-role command. Original T192–T194 retain separate reader, source/fee/entry
order and full T042 gates. Verify this boundary after the CSV local checks;
do not replace its real opposing commands with fixture writes.

Further signature tracing found an additional genuine invoice dependency:
`a25e544`'s unchanged tests call `preview::prepare` with `(org_id, actor_id)`,
introduced by `22ffdab`. Master/#228 still accepts only `org_id`. Therefore
the isolated writer patch is not yet a compilable delivery and must not be
published as ready or tested by dropping the actor argument. Preserve it in
the new worktree until the financial snapshot consumer extraction and its
shared #220 helper are integrated explicitly. `editing::load/review` also have
separately retained `15c82ef` snapshot changes; do not copy whole invoice files
and overwrite #216 filtering. CSV verification remains independent.

The CSV workspace suite passed (`95407`): 821 app, 180 integration and 121 core
tests, 1,122 total, with 11 existing manual tests ignored. Complete SQLx
regeneration is running in `55336` on a fresh private PostgreSQL cluster.

CSV SQLx regeneration passed (`55336`): 1,003 base descriptors unchanged, two
additions matching `e5fcc5a` byte-for-byte, 1,005 total. Both new queries belong
to the preserved transaction-observation/publication-barrier tests. Unsigned
commit `e9898ed` contains exactly three Rust files and those two descriptors.
Offline all-target workspace/server Clippy is running in `17809`; WASM lint
and publication remain next. #228's original full Nix check is still live in
`1662`, without a restart. The unfinished invoice worktree is preserved and
must wait for its genuine financial-preview dependency before runtime checks.

Offline server Clippy passed (`17809`), followed by WASM Clippy (`15392`), with
warnings/performance lints denied and no source changes. Published commit
`e9898edbbe53d64a16ab77f9bca9961ff520a4a8` as independent draft #231 on master
(five files, 257 insertions and 10 deletions). Required current-head CI is
running in [37490375038](https://github.com/numtide/horae/actions/runs/37490375038).
The next safe source boundary is the retained financial snapshot consumers from
`22ffdab`/`15c82ef`, reusing #220's helper and preserving #216 queries. Resolve
their dependencies before completing the staged invoice writer extraction.
Keep #228's verified-live Nix handle `1662`; no merge or original-PR closure is
authorized or performed. Newly observed #229/#230 concern Darwin CI/platform
support, not this extraction; they remain untouched.

### 2026-10-06 — Financial snapshot consumer extraction

The preceding turn made progress: #231 was published with all local gates
passed and the ledger was pushed. Reconfirmed #216's merge, read the saved
objective and resumed #228's original Nix handle `1662`; server/client release
builds passed and Clippy continues. No process was restarted.

Created `fix/financial-snapshot-authority` in
`.worktrees/financial-snapshot-authority` on #220 `bc0c7a0`. Extracted the
consumer/test hunks of `22ffdab` and `15c82ef` together, reusing #220's existing
manager-snapshot helper without copying or changing it. This covers original
T098–T103: project fee balances, invoice preview and invoice editor load/review.
The original HTTP matrix is wired into the existing real-cookie harness beside
the retained legacy-reader checks. #216 invoice filtering remains intact;
there is no schema, UI, writer-policy or dependency change.

Necessary fixture adaptations avoid importing unrelated permission storage or
project writer commands: existing organization-gate helpers are expanded to
their exact `FOR NO KEY UPDATE` SQL, as already done by #220's reader tests.
The original revision-zero assertion becomes an unchanged PostgreSQL `xmin`
assertion before/after actor revocation (the organization tuple must not change
at all). The separate organization-revision refresh case uses `SET name=name`
to create a new tuple version without changing business values. Both cases
still observe real blocked readers, require fresh authorization/business
snapshots and retain all original assertions about denied access, amounts,
invoice revisions, cancellation and business-row preservation. Test names
describe tuple changes rather than a permission column absent from master.
The canonical revision-specific originals remain preserved for later integration.

Formatting passed (`8739`, only the two adapted test files changed). The full
workspace suite is running in `49719` on a fresh private PostgreSQL cluster;
core/app/test entrypoints were touched to avoid stale shared-target artifacts,
without content changes. No other local Cargo check shares that target.
Next: review exact source/fixture differences, collect the suite, regenerate
SQLx, verify offline server/WASM and publish with #220 as its real dependency.
The prepared invoice writer worktree remains untouched until this dependency
can be composed without duplicating PR contents. The overall goal is incomplete.

The full suite passed (`49719`): 840 app, 180 integration and 121 core tests,
1,141 total, with 11 existing manual tests ignored. All 14 added snapshot tests
and the extended registered-session matrix ran. Preview, editor implementation
and fee-test files match `15c82ef` exactly; HTTP matrix/root snapshot tests also
match their original blobs. The only original invoice-test block absent here is
the assignment/FK regression already delivered in #228, not a removed test.
Source review confirmed unchanged financial helpers/queries, all four current
session identities, same-tenant resource predicates, no mutation guards changed
and materialization/commit before delivery. No critical/high finding within this
bounded extraction. Complete SQLx regeneration is the next running gate.

SQLx regeneration passed (`29604`): all 1,021 #220 descriptors unchanged, 24
additions (22 match `15c82ef`, two describe the documented `xmin`/no-op tuple
fixtures), 1,045 total. Saved unsigned financial commit `0bb5721`; offline
server Clippy runs in `85779`, full Nix checks on that committed head in `80170`.

Resolved the invoice writer's multiple prerequisites without modifying or
retargeting their PRs: `.worktrees/invoice-authority-prerequisites`, branch
`integration/invoice-authority-prerequisites`, starts at #228 `0e1e675` (which
includes #227) and replays #220 as `41e595d` and the financial reader extraction
as `0046dad`. Existing identical SQLx descriptors are shared rather than
duplicated. This branch is an integration/review base, not a separate delivery
or a request to merge another PR. Its source must be verified together with the
writer, and eventual master retargeting still requires all prerequisite PRs.

Saved the prepared five-file writer patch as `a428015`, backed it up at
`backup/invoice-write-extraction-before-integration`, then rebased only the new
unpublished `fix/invoice-write-authority` onto that integration base (`680407c`).
Its diff still consists of the five original `a25e544` source/test changes and
the 564-line test blob remains exactly `36ca710`. Repository `rebase.updateRefs`
also moved the new backup automatically; restored it immediately to `a428015`
with a compare-and-swap ref update. Original #212/#217 backup refs were checked
unchanged. Future extraction rebases must explicitly use `--no-update-refs`.
No PR was merged and no real data was changed. Run writer integration tests
after the financial reader local checks finish; do not run shared Cargo targets
concurrently. Its additional SQLx descriptors are not prepared yet.

#232 is published on #220 with clean head `0bb5721`. Offline server Clippy
passed (`85779`), followed by WASM (`86132`). Full Nix remains live in `80170`;
#228's original `1662` progressed through browser checks into test builds.
`git range-diff` confirms the invoice writer's rebased `680407c` is patch-equal
to saved `a428015`; its preview/snapshot sources are unchanged from #232 and its
editor/test roots match original `a25e544`. Full combined workspace verification
is now running in `37438` on private PostgreSQL. Next: collect that suite,
regenerate its cache, run both offline target lints and publish the writer only
with explicit prerequisite/retargeting instructions. Keep all live checks;
do not duplicate their runs or claim the remaining original groups accounted.

### 2026-10-06 — Invoice writer integration verification

The immediately preceding conversational turn only supplied a proposed goal
prompt (no authoritative progress). Re-read the active saved objective and
revalidated the actual worktrees and process handles instead of restarting
work. #216 is confirmed merged at `02f7b58`; the original worktrees remain
preserved. The earlier extraction work did make progress as recorded above.

The combined writer suite `37438` completed successfully on `680407c`: 861
app, 180 integration and 121 core tests, 1,162 total, with 11 existing manual
tests ignored. Formatting `92986` already passed unchanged. Complete SQLx
regeneration now runs in `28714` on another fresh private PostgreSQL cluster;
do not interpret its temporary cache removals as a final diff before it exits.

#228's original full Nix handle `1662` exited successfully with `all checks passed!`, including browser, SQLx and the NixOS end-to-end test; incompatible
platforms were omitted as reported by Nix. #232's `80170` is still live and
has completed its client release build. No check was restarted or bypassed.
Next: compare the writer's final SQLx cache with its integration base, run
offline server/WASM lints, publish the narrowly scoped draft with its explicit
prerequisites, and retain the single ledger as the accounting source.

Writer SQLx preparation passed (`28714`): 1,069 integration-base descriptors
unchanged and eight additions, all byte-identical to `a25e544`, 1,077 total.
Committed only those descriptors as unsigned `8a6cb2a`; source/tests remain
unchanged from the fully tested `680407c`. Offline all-target server Clippy
passed (`8831`). Full Nix runs on the clean committed head in `1215`; WASM
Clippy follows server lint without sharing concurrent Cargo work.
The source review was rechecked against T192–T194 and the original invoice
contract: current same-tenant actor before replay/write, common organization
and invoice serialization prefix, test-only actorless adapters remain test-only,
and events remain outside committed transactions. No broader policy or full
resource-lock hierarchy acceptance is claimed.

WASM Clippy passed (`75047`). Published the prerequisite review base `0046dad`
and writer `8a6cb2a`, then opened draft [#233](https://github.com/numtide/horae/pull/233).
Its diff is 13 files, 811 insertions and eight deletions: five original Rust/test
changes plus eight query descriptors. The body explicitly prohibits merging
into the integration base; first deliver #227 → #228 and #220 → #232, then
retarget/rebase the writer-only change onto master and run current-head CI.
No prerequisite branch was changed. #228's PR body now records its successful
full Nix check. Original #212/#217 backup refs and all 18 unfinished local paths
were rechecked unchanged; the writer's own pre-integration backup is `a428015`.

Next-boundary inspection found that extracting only the first profile command
commit `f5e0dde` would omit later actor/target/last-admin SHARE locks and bounded
READ WRITE transactions from `03e90b1`. Preserve those repairs with the commands.
Final `profiles.rs` also uses the shared `ProfileDraft`/command DTOs and the
historical `RemovedRelationship` type (the latter moved in `8db19ba`); it does
not require activating the editor UI. Final profile tests add unrelated child
modules after `03e90b1`; separate their module wiring without dropping original
profile cases. The next extraction must resolve #226 plus #221 prerequisites,
retain migration 0044 and current profile behavior, and account separately for
later editor/subject/directory tests. This is inspected dependency evidence,
not a completed extraction. Keep Nix handles `80170` (#232) and `1215` (#233)
without restarting them. The overall original-change reconciliation is incomplete.

Remote verification confirms #233 is draft on the intended review base, head
`8a6cb2a`, with exactly the 13-file writer diff. #227's unchanged `142eda1` now
has successful GitHub Flake Check and Format in run `37486457726`; Nixbot build
is still pending. #231's `e9898ed` remains on the original live Flake Check run
`37490375038` with Format passed; no rerun or repeated polling was requested.

### 2026-10-06 — Person-profile command extraction

The preceding iteration made progress: #233 was published, its combined tests
and offline checks passed, and current #227/#228 evidence was recorded and
pushed. Re-read the saved objective and confirmed #216 remains merged before
editing. Existing Nix handles `80170` and `1215` were both verified live; no
restarts. No original branch or unfinished work was changed.

Created `integration/profile-command-prerequisites` in its isolated worktree
at `46f02f7`: #226 `82d15f3` plus the exact #221 domain patch, replayed without
conflicts. This is a review/test base, not a separate delivery or merge target.
Created `refactor/person-profile-commands` in `.worktrees/person-profile-commands`
on that base. It carries original migration 0044, byte-identical final
`profiles.rs` from `db3935d`, and all 27 profile-command PostgreSQL tests from
`03e90b1`. The only removed test-file lines wire the separately retained editor
child module; no profile assertion or helper was changed. Later additions to
this test root only wire other consumer modules and remain separately accounted.

The four needed profile DTOs and historical `RemovedRelationship` retain their
exact original fields and serde attributes. They extend #226's existing
server-only command models without including unrelated UI/audit reader DTOs.
Template commands/tests and their shared transaction prelude are unchanged.
This preserves the later actor/target/survivor SHARE locks, bounded local
transaction settings, strict identity and replay checks, exact relationship
confirmation, audit rollback and last-administrator protection. Commands stay
internal and refuse policies 0/future; no endpoint, UI, backfill or activation.

Read the current person-profile and editor contracts and original T056–T058 /
T126–T129 task hunks. This extraction covers the internal command boundary and
its later hardening, not the separately retained editor/session/UI acceptance.
The combined core permission modules match the final original exactly.
Formatting passed unchanged (`1025`); the complete workspace suite is running
in `7729` on private PostgreSQL. A separate disposable migration-upgrade check
`59932` starts from the prerequisite schema including 0047 and then applies the
retained 0044, to verify actual delivery order without renumbering migrations.
Next: collect both checks, regenerate SQLx, run offline server/WASM gates,
review/publish the bounded draft and continue the remaining original accounting.
