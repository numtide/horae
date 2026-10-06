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
| Durable CSV preparation outside SQL transactions, [#231](https://github.com/numtide/horae/pull/231) | `fix/csv-batch-transaction-boundary`, `.worktrees/csv-batch-transaction-boundary` | `02f7b58` | Draft at `e9898ed`; 1,122 tests, SQLx, offline server/WASM Clippy, format and current-head GitHub Flake Check passed; Nixbot build pending |
| Financial snapshot reader authority, [#232](https://github.com/numtide/horae/pull/232) | `fix/financial-snapshot-authority`, `.worktrees/financial-snapshot-authority` | #220 `bc0c7a0` | Draft at `0bb5721`; 1,141 tests, SQLx, offline server/WASM Clippy, format and full local Flake Check passed; required CI after retargeting |
| Invoice writer/revocation ordering, [#233](https://github.com/numtide/horae/pull/233) | `fix/invoice-write-authority`, `.worktrees/invoice-write-authority` | Integration base `0046dad` combining #227/#228 and #220/#232 | Draft at `8a6cb2a`; 1,162 tests, SQLx, offline server/WASM Clippy, format and full local Nix passed; retarget to master after prerequisites, do not merge into integration base |
| Internal person-profile commands, [#234](https://github.com/numtide/horae/pull/234) | `refactor/person-profile-commands`, `.worktrees/person-profile-commands` | Integration base `46f02f7` combining #226/#221 | Draft at `45d219e`; 1,245 tests, schema upgrade, SQLx, offline server/WASM Clippy, format and full local Nix passed; retarget to master after prerequisites, no activation |
| Harvest connection change authority, [#235](https://github.com/numtide/horae/pull/235) | `fix/harvest-connection-authority`, `.worktrees/harvest-connection-authority` | #228 `0e1e675` | Draft at `2fcecd1`; 1,146 tests, SQLx, offline server/WASM Clippy, format and full local Nix passed; retarget after #227/#228 |
| Import job command and download authority, [#236](https://github.com/numtide/horae/pull/236) | `fix/import-job-authority`, `.worktrees/import-job-authority` | Integration base `26d6159` combining #235/#231 | Draft at `95bdf4a`; 1,163 tests, SQLx, offline server/WASM Clippy, format and full local Nix passed; retarget to master after #227/#228/#235 and #231, not an integration-base merge |
| Original import requester provenance, [#237](https://github.com/numtide/horae/pull/237) | `feat/import-job-requester`, `.worktrees/import-job-requester` | Integration base `cfb8240` combining #236/#219/#222 | Draft at `2242361`; 1,218 tests, schema upgrade, SQLx, offline server/WASM Clippy and format passed; full Nix passed on unchanged diagnostic rerun; initial inherited-menu failure retained; retarget after both prerequisite chains, no worker-policy activation |
| Budget email preparation authority, [#238](https://github.com/numtide/horae/pull/238) | `fix/budget-email-authority`, `.worktrees/budget-email-authority` | Master `1b8fa4f` | Draft at `7a7cede`; 1,133 source-head tests, SQLx, format and cache-inclusive server/WASM Clippy passed; full current-head local Nix passed; no real mail or policy activation |
| Approval tenant isolation, [#239](https://github.com/numtide/horae/pull/239) | `fix/approval-tenant-isolation`, `.worktrees/approval-tenant-isolation` | Master `1b8fa4f` | Draft at `66a256f`; three source/test files and complete cache patch preserved exactly; 1,124 source-head tests, source review, SQLx, format and offline server/WASM Clippy passed; full current-head local Nix passed |
| Identity response projections, [#240](https://github.com/numtide/horae/pull/240) | `fix/identity-response-projections`, `.worktrees/identity-response-projections` | Master `1b8fa4f` | Draft at `1ce993f`; 1,122 source-head tests, cache provenance, bounded review and format passed; cache-inclusive server/WASM lint passed; full Nix `16586` running; no activation |
| Time-writer account activity, [#241](https://github.com/numtide/horae/pull/241) | `fix/time-write-activity`, `.worktrees/time-write-activity` | #228 `0e1e675` | Draft at `7820f8d`; 1,142 tests, cache provenance, bounded review, format, offline server/WASM lint and full Nix `85167` passed; prerequisite integration/retarget/current-head CI still required; no delegated writes or activation |
| Time-entry invoice identity boundary, [#242](https://github.com/numtide/horae/pull/242) | `fix/time-entry-payload`, `.worktrees/time-entry-payload` | Master `1b8fa4f` | Draft at `43337fc`; 1,122 tests, cache/source provenance, bounded review, format, offline server/WASM lint and full Nix `73808` passed; current-head GitHub checks/delivery review remain; no policy or UI change |
| Session-bound project-manager delegation, [#243](https://github.com/numtide/horae/pull/243) | `feat/project-manager-delegation`, `.worktrees/project-manager-delegation` | Review base `e44433e` combining #234/#228 | Draft at `3404c85`; 1,295 tests/cache passed before web-only lint adaptation; final WASM lint passed, current-head full Nix `87574` running; last offline server pass `9c6b121`, final-head server check pending; no form wiring or activation |
| Own-permission explanation and Settings, [#244](https://github.com/numtide/horae/pull/244) | `feat/own-permission-settings`, `.worktrees/own-permission-settings` | #234 `45d219e` | Draft at `6c4e4d1`; 1,273 Rust tests, complete SQLx provenance, eight original-file comparisons, scoped review/Spec Kit analysis/format/detector passed; offline server/WASM lint `53333` and isolated Chromium `47268` passed; desktop/mobile captures inspected; full Nix `19313` running; no activation |
| Permission audit history, [#245](https://github.com/numtide/horae/pull/245) | `feat/permission-audit-history`, `.worktrees/permission-audit-history` | Review base `59d2798` combining #243 `3404c85` and #244 `6c4e4d1` | Draft at `533922a`; 1,356 Rust tests and complete SQLx provenance passed; original provenance, bounded review, scoped Spec Kit analysis, format and detector passed; offline lint `72147` and full Nix `61137` running, history browser verification pending |
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
| `2abce9a` | Enforce approval isolation and record permission boundaries | approval-isolation | Three Rust/test files and original cache patch in independent #239 (`66a256f`); 1,124 tests, source review, SQLx, format and offline server/WASM lints passed; full local Nix passed; mixed specification hunks retained |
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
| `f5e0dde` | Apply permission profiles with atomic scope changes | permission-profile-transactions | Model serialization in #222; strict template receipt comparison in #226; profile commands, migration 0044 and tests/cache in #234 with later hardening; specification hunks retained |
| `9a7e05d` | Add audited project manager delegation | project-manager-delegation | Final command and all production-command tests in #243 (`3404c85`), including later hardening/composition; suite/cache/format passed before web-only annotation, final WASM lint passed, final Nix running; specification hunks preserved separately |
| `fc85231` | Add administrator-only permission audit lookup | permission-audit | Complete reader/model/tests extracted in `feat/permission-audit-history` (`c96d787`); suite passed, cache/gates pending; specification hunks retained |
| `c3d17cb` | Prevent deadlocks during legacy import report conversion | import-transaction-lifecycle | Conversion source/tests/SQLx in #223; specification hunks retained for reconciliation |
| `3ae8e08` | Coordinate project access changes before locking resources | legacy-access-writers | Rust/test/cache changes in #228; organization SHARE query also reused in #220; specification hunks retained |
| `907bc88` | Recheck authority when saving organization branding | branding-authority | Source/test/cache hunks in #225; specification hunks retained for reconciliation |
| `d7a5a21` | Revalidate administrator authority for Harvest connection changes | import-authority | Eight Rust/test files and regenerated cache in #235 on #228; suite/SQLx/offline lints/format/full local Nix passed; specification hunks retained for reconciliation |
| `4fac6af` | Revalidate import job command and status authority | import-authority | Exact executor-based `jobs::cancel` owned by #231; remaining command/status Rust changes with the subsequent shared guard in #236; specification hunks retained for reconciliation |
| `b4672a4` | Revalidate authority during import error downloads | import-authority | All eight combined command/download Rust/test files byte-identical in #236, plus eight regenerated SQLx additions matching original; suite/cache/offline lints/format/full local Nix passed; specification hunks retained |
| `e5fcc5a` | Prepare durable CSV batches before opening transactions | import-transaction-lifecycle | Two exact Rust blobs and regenerated cache in #231; local verification passed, CI pending; specification hunks retained |
| `482b7c5` | Retain the original requester of import jobs | import-requester-provenance | Seven original source/schema/test paths and exact regenerated cache patch in #237 on combined #236/#222 prerequisites; 1,218 tests, schema order, cache, offline lints and format passed; full Nix passed on unchanged diagnostic rerun; initial inherited-menu failure retained; specification hunks retained |
| `e949e4c` | Drain interrupted import transactions before releasing reservations | import-transaction-lifecycle | Production/test/cache hunks in #224; specification hunks retained for reconciliation |
| `c0cfb8f` | Scope rate permissions to their owning resource | scope-domain | Code/tests in #221; specification hunks retained for reconciliation |
| `5d51b0e` | Expose the current person's permission snapshot | own-permissions | Reader, DTO, complete DB/HTTP tests and registrations in #244 (`6c4e4d1`) on #234; 1,273 Rust tests and cache provenance passed, lints/Nix running; specification hunks retained |
| `22ffdab` | Recheck manager access for financial snapshots | manager-snapshot-consumers | Shared helper/queries in #220; consumer/tests/cache in #232, local suite, both target lints and full Nix passed; specification hunks retained |
| `15c82ef` | Recheck manager access in invoice editor snapshots | manager-snapshot-consumers | Consumer/tests/cache in #232; fixture adaptations documented, local suite, both target lints and full Nix passed; specification hunks retained |
| `dc822a0` | Recheck manager authority before delivering exports | export-authority | Held in original backup; extraction pending |
| `108594e` | Revalidate project scope before delivering exports | export-authority | Held in original backup; extraction pending |
| `dcf4ac8` | Recheck current permissions during CSV downloads | export-authority | Held in original backup; extraction pending |
| `0793ce7` | Revalidate budget email authority before delivery | budget-email-authority | Four source/test files byte-identical in #238 (`7a7cede`) on master `1b8fa4f`; 1,133 tests, SQLx, format and offline server/WASM lints passed; full local Nix passed; specification hunks retained |
| `8aac739` | Add read-only permission migration diagnostics | permission-preflight | Held in original backup; extraction pending |
| `8c15bfe` | Show own permissions in Settings | own-permissions | Complete original component, shared descriptions, SSR and resource tests in #244 (`6c4e4d1`); suite/cache passed, new isolated browser verification pending, no full T018 claim; specification hunks retained |
| `300d1e9` | Expose administrator permission history | permission-audit | Complete historical DTO/HTTP/fencing tests extracted in `c96d787`; suite passed, cache/gates pending; specification hunks retained |
| `03e90b1` | Connect permission editor previews and commands | permission-editor | Template DTOs, hardening/tests and administration helpers in #226; profile DTOs, shared calculation and command hardening/tests in #234; editor/session/remaining hunks retained |
| `7f7fd1c` | Record permission editor delivery and UI follow-up | specification-history | Held in original backup; extraction pending |
| `98b1692` | Add reviewed person permission editing | permission-editor | Held in original backup; extraction pending |
| `9e6d8bd` | Record person editor delivery and template follow-up | specification-history | Held in original backup; extraction pending |
| `0f97cb2` | Add custom permission profile controls | permission-editor | Held in original backup; extraction pending |
| `8db19ba` | Show affected names in permission reviews | permission-editor | Profile command's historical relationship-type reuse in #234; display names, editor/UI/tests and specification hunks retained |
| `e7d8a36` | Protect permission drafts during navigation and dismissal | permission-editor | Held in original backup; extraction pending |
| `6bba224` | Bind permission saves to the original requester | permission-editor | Exact shared requester DTO extracted in `be787ca`; editor/session/UI remainder retained for permission-editor delivery |
| `1ecfa21` | Recover interrupted permission saves across reloads | permission-editor | Held in original backup; extraction pending |
| `c88ca6d` | Exercise permission recovery in a real browser | permission-editor | Held in original backup; extraction pending |
| `202ee96` | Protect project delegation against concurrent deactivation | project-manager-delegation | Complete final command/activity tests in #243 (`3404c85`); suite/cache/format passed before web-only annotation, final WASM lint passed, final Nix running; specification hunks preserved separately |
| `3f45b7c` | Validate combined approval record coverage | scope-domain | Code/tests in #221; specification hunks retained for reconciliation |
| `eb56af3` | Define scoped approval transaction and coverage gates | specification-history | Held in original backup; extraction pending |
| `8d49421` | Add authorized permission editor subject discovery | permission-editor | Held in original backup; extraction pending |
| `b735b3a` | Add safe person switching to permission editor | permission-editor | Held in original backup; extraction pending |
| `7f14e4b` | Limit user directory responses to consumed fields | identity-projections | Source/tests together in #240 at `1ce993f`; 1,122 tests, regenerated cache, format and offline lints passed; full Nix running; canonical tests and mixed specification hunks separately preserved |
| `1eb13ec` | Add scoped people directory reads | people-directory | Held in original backup; extraction pending |
| `981d0e3` | Resolve approval names without directory access | identity-projections | Source/tests together in #240 at `1ce993f`; 1,122 tests, regenerated cache, format and offline lints passed; full Nix running; canonical tests and mixed specification hunks separately preserved |
| `6b5dbae` | Authorize identity-only project team choices | project-team-choices | Held in original backup; extraction pending |
| `4c00660` | Document project form permission integration boundaries | specification-history | Held in original backup; extraction pending |
| `3bb62ac` | Limit session identity responses to display fields | identity-projections | Source/tests together in #240 at `1ce993f`; 1,122 tests, regenerated cache, format and offline lints passed; full Nix running; canonical tests and mixed specification hunks separately preserved |
| `4ac30fa` | Add scoped time-entry reads without financial metadata | time-readers | Held in original backup; extraction pending |
| `c80233b` | Keep invoice identities out of time-entry responses | time-entry-payload | Exact final model and independently registered original legacy HTTP assertions in #242 (`43337fc`); 1,122 tests, complete SQLx, format, offline lints and full Nix passed; canonical fixture remainder and specification hunks separately preserved |
| `5ec183a` | Fence time-entry writes against account deactivation | time-writer-activity | Source/tests and regenerated cache in #241 (`7820f8d`), original configuration SQL inlined without canonical module; 1,142 tests, both offline lints and full Nix passed; specification hunks retained |
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
| `8722320` | Add browsable permission change history | permission-audit | Own-reader authentication-error sanitization in #244 (`6c4e4d1`); audit reader/UI/navigation, Settings link, profile labels and shell tests extracted in `c96d787`; original editor-dependent browser assertions retained for verification reconciliation; specification hunks retained |
| `2078a13` | Format permission history verification notes | specification-history | Held in original backup; extraction pending |
| `2f5357f` | Verify permission history and scoped Timesheet browser flows | browser-fixture-tooling, permission-audit, timesheet-consumer-commands, permission-editor | Held in original backup; extraction pending |
| `68bbaae` | Fix Timesheet modal focus and long-label layout | timesheet-consumer-commands | Held in original backup; extraction pending |
| `e29f4d8` | Reload Timesheet state when switching people | timesheet-consumer-commands | Held in original backup; extraction pending |
| `5f7895c` | Preserve selected dates and drag offsets in Timesheet | timesheet-consumer-commands | Held in original backup; extraction pending |
| `84d5352` | Expose authenticated project manager delegation | project-manager-delegation | DTOs, session wrappers, reader and tests in #243 (`3404c85`); one web-only DTO lint expectation is the recorded extraction adaptation; HTTP audit-visibility block and audit-fixture adaptation remain owned by permission-audit delivery; specification hunks retained |
| `c4e83c8` | Record project delegation verification and next integration gate | specification-history | Held in original backup; extraction pending |
| `a25e544` | Serialize invoice writes before user revocation | legacy-access-writers | Five Rust/test changes and eight SQLx additions in #233 on integrated #220/#227/#228/#232 prerequisites; suite/cache/offline lints/full Nix passed; specification hunks retained |
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
| `2497dbe` | Enforce scoped permissions in the project editor | project-editor-permissions | Pure RateEdit code/tests in #221; composable delegation and its transaction tests in `be787ca`; project editor and remaining hunks retained |
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

The prerequisite-schema upgrade passed (`59932`), applying only original
migration 0044 after the already-applied 0047. All four command/draft DTO blocks
were compared byte-for-byte with the original and match. Template implementation
and test files also match final `db3935d`; no substitute fixture commands are used.
Saved the seven-file extraction as unsigned `9005b1b` (1,744 insertions, two
deletions); cache generation and publication remain pending, not claimed ready.
Source review found no critical/high issue within this inactive internal boundary;
runtime verification remains mandatory. The full suite `7729` is still live in
compilation; no duplicate local Cargo run has been started. #232's existing Nix
run progressed through Clippy into browser checks, while #233 completed its SQLx
derivation and client build; both full checks remain pending.

Read-only inspection for a later import-authority extraction confirms `d7a5a21`
requires the existing organization lock helper and threads the authenticated
actor through connect/disconnect/account-switch commits, including original
credential tests. `4fac6af` and `b4672a4` add command/status/download authority;
later `482b7c5` adds requester provenance and has a separate storage dependency.
Do not copy final import files blindly across those boundaries or duplicate
the executor-based cancel change already extracted in #231.

The full combined workspace suite passed (`7729`) on the unchanged `9005b1b`
source: 876 app, 180 integration and 189 core tests, 1,245 total, with 11
existing manual tests ignored. All 27 original profile tests ran alongside the
template and domain prerequisites. The initial compilation took 4m25s and the
app tests 120s; long-running notices were not failures or a reason to restart.
Full SQLx preparation now runs in `56264` on another private temporary database.
Next: compare the final cache with the prerequisite base and original descriptors,
then serialize offline server/WASM lint before publishing the draft.

SQLx preparation passed (`56264`): all 1,063 prerequisite descriptors unchanged,
35 additions byte-identical to original `db3935d`, 1,098 total. Offline all-target
workspace/server Clippy is running in `89503`; WASM must follow it sequentially.
No source or test repair was needed after the full suite. Original migration
checksums and prerequisite source remain intact.

Published draft [#234](https://github.com/numtide/horae/pull/234) at `45d219e`
against the explicitly documented review base `46f02f7`. Verified remote diff:
42 files, 2,437 insertions and two deletions (seven source/schema/test paths plus
35 query descriptors). Server Clippy passed (`89503`); WASM runs in `13136`.
Full Nix runs on the clean committed head in `41624`. No new source changes
followed the successful suite; the second commit only adds its verified cache.
The PR requires #219, #221/#222 and #226 on master before rebase/retarget and
fresh required CI. It must not merge into its integration review base. No merge,
original-PR closure or policy activation was performed; remaining consumer and
specification accounting is still incomplete.

WASM Clippy passed (`13136`) with no code changes. #234 now has all local
suite/cache/offline-lint/format gates passed; full Nix remains in `41624`.
Next: collect existing Nix handles `80170` / `1215` / `41624` without reruns,
then continue the retained import-authority boundary from its actual source
dependencies, preserving #231's already-delivered cancellation adapter.

### 2026-10-06 — Harvest connection authority extraction

The intervening conversational turn drafted a goal but made no repository
progress. Re-read the active saved objective and resumed the existing ledger;
the last implementation iteration delivered #234. Confirmed #216 merged at
`02f7b58` before editing, and verified all three existing Nix handles live
(`80170`, `1215`, `41624`); none was restarted. #234's Nix SQLx derivation and
client release build have now passed, but its full check is not yet terminal.

Created `fix/harvest-connection-authority` in its isolated worktree on #228
`0e1e675`. This dependency supplies the organization SHARE helper and coordinated
legacy access writers; no duplicate helper or new integration branch is needed.
The eight Rust/test files match original `d7a5a21` byte-for-byte, including the
seven production-writer authority tests (`afc063dd8c8a64668475a4865a813619b6bffff4`).
Read original T074–T076 and the connection-management contract. Later changes to
the credential writer, account switch and this test module are absent; later
job/streaming work in `harvest.rs` remains separately retained.

The extracted path obtains the actor only from the authenticated wrapper or
validated OAuth attempt, reserves the import nonblockingly, then checks the
organization and active same-tenant Administrator under READ COMMITTED/SHARE
before the existing generation gate. Authority remains locked through commit.
External OAuth exchange stays outside that transaction; account binding,
generation checks, encryption, import history and watermarks are unchanged.
Safe forbidden projections cover both callback and server-function errors.
No new policy activation, schema change, UI or external Harvest mutation.

Adversarial source review covers admission-to-write races, actor-only waits,
inherited REPEATABLE READ, missing/foreign/inactive actors, writer-first retention,
rollback and reservation cleanup. No critical/high issue found within this
bounded extraction; tests remain necessary evidence. Formatting passed unchanged
(`43759`). The complete workspace suite runs in `5611` using private PostgreSQL;
next regenerate SQLx, run offline server/WASM checks, then publish the bounded
draft. Full original-change and specification reconciliation remains incomplete.

Saved the exact source extraction as unsigned `fcffd12`; its stable patch ID
matches the original Rust delta (`1d297236d8c4fa3a0abdd49c41ec2d53e682470a`).
The full suite has finished compilation and is running tests. Original tracked
edits still compare equal to the saved uncommitted snapshot, and all six untracked
files compare equal to the recovery archive. #231's existing CI run `37490375038`
now reports successful Flake Check and Format on unchanged `e9898ed`; its Nixbot
build remains in progress. No rerun, merge or closure was requested.

Read-only next-block inspection: `4fac6af` introduces the importer command
transaction plus executor-based queue helpers, and `b4672a4` shares its authority
guard with paged error downloads. They form a cohesive command/result boundary
on the same organization helper. Later `482b7c5` only extends these callers for
requester attribution and migration 0045's `(org_id, id)` user foreign key,
which needs the #222 schema prerequisite. Preserve that distinction rather
than silently carrying storage or activating worker policy. The cancel adapter
already owned by #231 must be accounted as an identical shared prerequisite,
not a second independent delivery of that change.

The complete combined suite passed (`5611`): 845 app, 180 integration and 121
core tests, 1,146 total, with 11 existing manual tests ignored. This exercises
all seven retained authority tests and the #227/#228 prerequisite behaviors
together. No source/test adaptation was needed. Full SQLx regeneration is next,
then offline server/WASM lints and publication; old results are not used to claim
these remaining checks passed.

Full SQLx preparation passed (`9128`): 1,030 prerequisite descriptors unchanged,
ten additions byte-identical to `d7a5a21`, and only the superseded post-HTTP
authority query removed, for 1,040 total. Two added descriptors already existed
earlier in the original history (session REPEATABLE READ and transaction READ
COMMITTED), explaining the difference from that original commit's eight additions.
Saved the cache as unsigned `2fcecd1`; source remains identical to tested
`fcffd12`. Final bounded diff: 19 files, 739 insertions and 49 deletions (eight
Rust/test paths plus eleven cache changes). Offline server lint runs in `91709`;
full Nix runs on clean head `2fcecd1` in `72580`. WASM follows server lint.

Published draft [#235](https://github.com/numtide/horae/pull/235), head `2fcecd1`,
base `fix/project-access-lock-order`. Its description includes the exact source
provenance, tests, pending checks and #227 → #228 → #235 integration order.
No migration, runtime policy activation, merge or original-PR closure. Complete
this extraction's remaining lints before editing the next command/result block;
retain all existing Nix handles without restarting them.

#232's original full Nix check (`80170`) completed successfully on unchanged
`0bb5721` with `all checks passed!`, including browser, SQLx and NixOS end-to-end
checks; incompatible systems were explicitly omitted by Nix. Its PR description
now records that result. #235's offline server/all-target lint also passed
(`91709`); WASM runs in `21947`. #233's original full Nix check (`1215`) passed
its server release build and VM e2e script and continues its browser matrix;
#234 (`41624`) and #235 (`72580`) remain live full checks, not completed claims.

#235's WASM lint passed (`21947`) without edits. Its published draft now records
all local suite/cache/offline-lint/format checks passed, with full Nix pending.
Next: retain `1215` / `41624` / `72580` until terminal, and extract the original
`4fac6af` + `b4672a4` job command/result boundary after resolving the already-owned
#231 cancel adapter and actual test/base dependencies. No original work is lost
or relabeled complete; broad original-code and specification accounting remains
unfinished, so the overall goal remains active.

### 2026-10-06 — Import job commands and bounded downloads

The preceding iteration made progress: #235 was published, its local gates
passed, and #232's full Nix result was recorded. Re-read the saved goal, verified
#216 remains merged, and resumed the three existing Nix handles (`1215`, `41624`,
`72580`) confirmed live. Repository instructions and the constitution are unchanged.

Created isolated review base `integration/import-job-authority-prerequisites`
at `26d6159`: #235 `2fcecd1` plus exact #231 `e9898ed`, replayed without conflict.
The #231 executor-based cancel adapter is therefore inherited, not duplicated
in this extraction. No prerequisite branch was changed. This integration base
is not a delivery PR or merge target; #227 → #228 → #235 and #231 must reach
master before rebasing/retargeting the bounded command/result diff and fresh CI.

Created `fix/import-job-authority` in its own worktree on that base. The eight
Rust/test files match original `b4672a4` byte-for-byte: both `4fac6af` command
authority and the subsequent shared download guard are preserved together.
Read T077–T082 and both transaction contracts. The six command/status helpers
use trusted actor IDs and retain current same-tenant Administrator authority
through the existing generation/job/upload operations and returned projection.
Upload buffering precedes authorization locks. Duplicate/no-op paths still
authorize; generation, payload, retry, upload retention and queue semantics remain.

The shared guard also authorizes initial report preparation, each bounded page
of up to 16 fragments and the captured inline/empty tail. Transactions commit
before client-paced output. Already authorized buffered bytes may drain, but
revocation aborts the next page/tail instead of producing successful truncated EOF.
Snapshot boundaries, exact retained errors, headers and missing-fragment failures
remain unchanged. Original HTTP tests revoke access after endpoint admission
and before upload acceptance/first body consumption. Test-only queue adapters
remain test-only; no worker policy, requester migration, UI or cutover is included.

The final-original guard, download implementation and stream tests have no later
changes; the command test root only later adds the separately retained requester
module. Reviewed production callers, tenant/error projections, transaction and
connection lifetimes, cancellation/retry, wait ordering and prerequisite ownership.
No critical/high source finding in this bounded extraction; runtime evidence is
still required. Formatting passed unchanged (`2345`); full combined workspace
tests run on private PostgreSQL in `48997`. Next collect the suite, regenerate
SQLx, run offline server/WASM gates and publish with the real dependency order.

Saved the eight-file extraction as unsigned `c384d48` (1,519 insertions,
111 deletions); all eight hashes match `b4672a4`. The inherited CSV preparation,
connection authority and account-switch files remain unchanged from the review
base. No new lower-level queue bypass or worker grant was introduced.

Read-only inspection of the next original increment confirms `482b7c5` has five
requester tests, a four-line migration 0045 and focused command/HTTP/historical
report adaptations. It needs migration 0042's `users_org_id_id_key`, supplied
by #222, in addition to these command helpers. Both the requester test module
and migration remain unchanged at final `db3935d`. Preserve their historical
NULL and duplicate-first-author rules, private DTOs and NO ACTION tenant FK;
this is attribution only, not authority to execute jobs for a revoked requester.
An eventual extraction must test the actual prerequisite-schema upgrade order,
including already-delivered migration 0047, without renumbering original migrations.

### 2026-10-06 — Command/download verification and publication

The preceding user-facing turn only supplied the requested goal text (no
implementation progress); the existing extraction remained preserved. Re-read
the attached objective and repository instructions, confirmed #216 merged at
`02f7b58`, and resumed existing verification handles rather than restarting them.
The command/download suite (`48997`) finished successfully: 862 app, 180
integration and 121 core tests, 1,163 total, with 11 existing manual tests ignored.
All eight source/test files remain at `c384d48`. Complete SQLx regeneration runs
in `73391` against private PostgreSQL; offline server/WASM lints follow.

The previously observed terminal result for #233's full Nix run (`1215`) was
successful on `8a6cb2a`, including browser and NixOS checks; incompatible systems
were omitted. Recorded this in the delivery table and PR description. Its original
handle is now closed, not a reason to restart the completed check. #234 (`41624`)
and #235 (`72580`) were confirmed still live; neither is counted as complete.

Full SQLx regeneration passed (`73391`): all 1,042 inherited descriptors unchanged
plus eight additions byte-identical to `b4672a4`, 1,050 total. Unsigned cache commit
`95bdf4a` leaves the eight source/test files unchanged. Published draft
[#236](https://github.com/numtide/horae/pull/236) on review base `26d6159`:
16 files, 1,632 insertions and 111 deletions. All added lines are original code,
tests or regenerated cache, not new functionality. Offline server lint runs in
`96055`; WASM follows. Full Nix runs on clean `95bdf4a` in `65754`.

Rechecked preservation: original tracked work matches saved snapshot
`backup/pr212-split-20261006-uncommitted` and the untracked archive comparison
passed unchanged. The next attribution increment requires #222 (and its #219
base) plus #236; original migration 0045 relies on 0042's composite user key.
Keep its five original requester tests, HTTP forgery checks and legacy report
adaptations together. Test the actual upgrade from the combined base already
containing 0047; no original migration renumbering, backfill or worker-policy grant.

Offline server lint (`96055`) and WASM lint (`36645`) both passed on unchanged
#236 source/cache. Its PR description records all local gates passed and full
Nix still running (`65754`). Remote inspection confirms draft head `95bdf4a`,
the intended review base and exactly the bounded 16-file diff.

#234's original full Nix run (`41624`) completed with `all checks passed!` on
unchanged `45d219e`, including browser and NixOS e2e checks; incompatible systems
were omitted. Recorded the result without restarting the successful run.

Created isolated review base `integration/import-requester-prerequisites` at
`cfb8240`: #236 `95bdf4a` plus exact #219 `ec7ddbd` and #222 `e9695fd`, replayed
without conflict. Created `feat/import-job-requester` in its own worktree and
extracted only the seven source/schema/test paths from original `482b7c5`.
Unsigned source commit `e0293d7` has 409 insertions and 14 deletions; stable patch
ID `36ce831fe633f5e8882627d7785cbf49f501c28c` equals that original filtered patch.
The report-test hunk is preserved without duplicating #223's separate lock-order
tests. Migration 0045 and all five requester test bodies remain unchanged.

Formatting passed unchanged (`77129`). Combined full workspace tests run in
`49490` on private PostgreSQL. A separate private-database migration-order check
(`73733`) applies the actual prerequisite schema including 0047, then 0045, and
repeats migration execution. It passed with the exact applied set 0042/0045/0047;
the populated historical-state fixture remains covered by the running Rust suite.
Next: collect the suite result, regenerate requester SQLx, run offline lints and
full Nix, then publish its bounded draft with both prerequisite chains. Continue
the existing `72580` / `65754` Nix handles; no restart or merge is authorized.

### 2026-10-06 — Requester verification and next independent boundary

The preceding iteration made progress: #236 was published with passing local
gates, #234's full Nix result was recorded, and the original requester increment
was extracted with its migration-order check passed. Re-read the attached goal,
confirmed #216 merged, and verified repository/skill instructions unchanged.
The three existing verification handles (`49490`, `72580`, `65754`) remain live;
no process was restarted because of quiet compilation. Published review base
`cfb8240` unchanged. Requester suite compilation finished in 4m28s and began the
889-test app binary; completion is not yet claimed.

Read-only next-boundary inventory: `0793ce7` changes four notification source/test
paths plus cache/specification material, with 13 authority tests. Those four
paths have no later original changes through `db3935d`; the two pre-existing
notification files match master before that commit. The preparation contract
preserves the existing recipient predicate, bounded send, attempts and stable
message identity; OP37 canonical mapping and enqueue integration remain open.
Inspect the complete implementation and test dependencies before extracting.
This looks independent of the permission-schema chains, but no completed review
or runtime acceptance is claimed. Use disposable databases and executable local
sender stubs only; never send real budget mail during verification.

Requester suite `49490` passed: 878 app, 180 integration and 160 core tests,
1,218 total, with 11 existing manual tests ignored. This includes all five
original requester cases, HTTP forged-identity assertions and the populated
historical migration fixture on the combined prerequisite code. No source or
test changes were needed. Complete SQLx regeneration runs in `92846`; offline
server/WASM lints must finish before reusing the shared local Cargo target.

Read the complete notification preparation/delivery implementation, 13 new
authority tests, existing sender stub/lifecycle tests, outbox claim/acknowledgement
helpers and migration 0039's parent revision triggers. Actual prerequisites are
already on master: there is no permission-state storage, new grant, actor helper
or external transport dependency to import. Tests use only local executable
stubs and temporary databases. The transaction takes organization, recipient,
project then outbox locks, rechecks stored claim/payload/attempt/clock and current
eligibility in fresh statements, and commits before external delivery. Retargeted
or replaced claims skip without corrupting replacement state; terminal rejection
stays under the claim lock. No critical/high source finding in this bounded
review; full runtime evidence remains necessary, and post-release recall or
exactly-once transport is not claimed.

Created independent branch/worktree `fix/budget-email-authority` /
`.worktrees/budget-email-authority` on `02f7b58`. Unsigned `bf22776` contains four
source/test files, 1,015 insertions and 49 deletions. All four file blobs match
`0793ce7` (and final `db3935d`) exactly; stable patch ID
`48c6efde7e99c4a446dfb2991f1a40563a7d6385` matches the original filtered patch.
Formatting passed unchanged (`29642`). Do not run its local Cargo suite until
the requester SQLx and offline gates release the shared target. No real mail,
new schema, worker activation or original branch changes occurred.

Requester SQLx preparation passed (`92846`): 1,065 prerequisite descriptors
unchanged, thirteen additions byte-identical to `482b7c5`, and four superseded
descriptors removed, 1,078 total. The entire regenerated cache patch has the
same stable ID as the original (`1f6cf80b1a0b38424bf0f7aa109e6eedffc157af`).
Saved unsigned cache commit `2242361` and published draft
[#237](https://github.com/numtide/horae/pull/237) on review base `cfb8240`.
Bounded diff: 22 files, 629 insertions and 75 deletions. The original source and
five requester tests are unchanged. Offline server lint runs in `16577`; WASM
follows. Full Nix runs on clean `2242361` in `63735`. No merge into the review
base is authorized; deliver both prerequisite chains, then retarget and rerun CI.

#235's existing Nix run (`72580`) completed its server release build and advanced
to browser/NixOS checks. #236 (`65754`) completed its Nix test derivation and is
still running remaining checks. Neither full result is yet claimed successful.

Requester offline server lint (`16577`) and WASM lint (`11771`) passed unchanged.
#237 now records all local gates passed and full Nix (`63735`) running on the
published head. Remote verification confirms `2242361`, base `cfb8240`, draft
status and the bounded 22-file diff. The original tracked snapshot and untracked
archive were compared again and remain intact.

With the shared local target released, started the independent budget-email
workspace suite in `57086` using the existing private-PostgreSQL wrapper and
original local sender stubs. Next collect this suite, regenerate its complete
SQLx cache, run offline server/WASM lints, full Nix and publish the independent
draft. Preserve existing Nix sessions `72580` (#235), `65754` (#236) and `63735`
(#237) until terminal. Specification reconciliation and many other original
behavior groups remain unassigned; the overall goal is not complete.

### 2026-10-06 — Notification checks and bounded artifact analysis

The preceding iteration made progress: #237 was published with all local gates
passed, and the independent notification source/tests were extracted at `bf22776`.
Re-read the attached goal and confirmed all four existing verification handles
live. #216 remains merged. Budget-email suite `57086` compiled in 4m06s and is
running its 843-test app binary; no suite completion is claimed yet.

Master advanced externally to `1b8fa4f` through #230. Inspection shows only
`flake.nix` changed: Linux remains the checked package systems, with ARM Darwin
retained for local development. No application, schema or dependency lock changed.
Do not modify the worktree underneath a live verification process. After its
current suite finishes, rebase only the unpublished notification extraction onto
this master using `--no-update-refs`, verify patch preservation, and run current-head
checks. Keep old-head evidence labelled; do not silently certify a rebased head.

Ran the actual Spec Kit analyze prerequisite command once in the original
worktree, resolving feature 015 without overrides. No before/after hooks exist.
Read constitution 1.1.0 and the relevant spec stories/requirements, plan gates,
T086–T088/T114–T116 and their previously reviewed contracts. Read-only analysis
covers six requirement IDs (FR-006/007/010/017/018 and SC-006) and six tasks, all
with traceable bounded coverage, not full-feature acceptance. No new constitutional
conflict or high/critical finding was identified within these two increments.
Retained findings: medium stale T161 prose contradicts its closed checkbox and
recorded later evidence; low distinction needed between historical T113→T114
work order and the independently verified notification release dependency.
No feature artifact or constitution was modified during analysis.

Documentation preservation must use the original `9301112..db3935d` change set,
not a wholesale old-tree replacement over current master. Direct old-tree versus
master comparison misleadingly lists newer Clients/parity specs as deletions;
those are upstream additions and must remain. Governance amendment, committed
feature artifacts and unfinished local Client contract changes remain separate
accounting work, not permission to declare their underlying features complete.

### 2026-10-06 — Notification rebase and completed connection checks

The intervening response only restated the requested goal and made no repository
progress. Revalidated the four existing process handles rather than restarting
them, and confirmed #216 remains merged at `02f7b58`.

Notification suite `57086` passed on old head `bf22776`: 832 app, 180 integration
and 121 core tests, 1,133 passed with 11 existing manual tests ignored. Rebased
only the unpublished extraction using `--no-update-refs` onto master `1b8fa4f`,
yielding unsigned `88149495025faecb059a0c4f13b5651918f4bec9`. The four source/test
blobs and stable source patch ID remain unchanged. Current-head suite `89138`
also passed all 1,133 tests; the same private-database session is now regenerating
the complete SQLx cache. Offline lints, full Nix and publication remain pending.

#235's original Nix process `72580` exited successfully on `2fcecd1`, including
the remaining browser matrix and NixOS checks. Updated its PR verification record.
This proves the local x86_64-linux gate for that head, not other architectures or
a future retargeted commit. #236 (`65754`) and #237 (`63735`) remain live; neither
is being restarted or counted as a completed full check.

Notification SQLx preparation in `89138` completed successfully. Regenerated
cache has 1,001 unchanged base descriptors, 54 byte-identical original additions
and two obsolete removals, 1,055 total. All original cache changes are represented;
ten additional descriptors already existed in the original parent through other
code, but are required by this standalone source/tests (transaction settings,
row locks and active-recipient fixtures). Therefore the entire cache patch ID
differs from `0793ce7`, while its contents are individually preserved. Saved
unsigned cache commit `7a7cede` and published independent draft
[#238](https://github.com/numtide/horae/pull/238), 58 files / 1,985 insertions /
53 deletions including generated metadata. Server offline lint `98267` and full
Nix `24069` run on the cache-inclusive head; WASM follows the local server lint.

Reviewed the next independent boundary: original `2abce9a`, tasks T021–T023.
Read the complete approval implementation, shared server helpers, four new
isolation tests and fixtures; traced every total/approve/reopen caller. The
three source paths and fixtures before this original commit match current master.
All external mutation routes still require the session's active Manager/Admin;
the helper's organization comes from that trusted user. Only tenant-local
submitted rows transition, only returned approval IDs select entries, foreign
reopen returns not-found, and invoiced entries remain untouched. Transactions
and post-commit plugin events retain their existing ordering. No new critical/high
finding within the tenant repair; this is not transactional revocation or the
future scoped/flexible approval policy.

Created isolated `fix/approval-tenant-isolation` /
`.worktrees/approval-tenant-isolation` on `1b8fa4f`, unsigned `d72b830`.
All three source/test blobs match `2abce9a` exactly, 248 insertions/31 deletions;
stable source patch ID is `3fad1ed12dbf8b71c61b579f56da244a39a7faf6` on both.
Formatting passed unchanged (`18944`). The new isolation test file is unchanged
through final `db3935d`. Later approval-name projection (`981d0e3`) and Timesheet
submission context (`b8b1c60`) remain separately preserved, not silently discarded
or folded into this invariant repair. Runtime suite/cache/lints/publication follow
once notification lints release the shared local target.

Recompared the original tracked backup and untracked archive: both remain intact.
No merges, original PR closures, production-data changes or real emails occurred.

Notification offline server lint (`98267`, 3m35s) and WASM lint (`43884`, 57s)
passed on cache-inclusive `7a7cede` without source changes. Updated #238's body;
remote inspection confirms draft status, master base, exact head
`7a7cede9d9ad8457e7bd78f049830d3144b468ac` and the bounded 58-file diff. Its full
Nix handle `24069` remains live, alongside #236 `65754` and #237 `63735`.

Released the shared local target and started approval isolation suite followed
by complete SQLx regeneration in `78215`, using the existing disposable-database
wrapper. Next: collect that run, verify cache provenance, run offline lints,
commit generated metadata unsigned, publish an independent draft and start its
clean-head Nix gate. Do not start another local Cargo process before `78215`
terminates. The original branches/backups and remaining specification/UI/domain
groups are not changed or declared complete by this progress.

### 2026-10-06 — Approval verification and identity-boundary inventory

The previous iteration made progress: #238 was published with local gates passed
and approval isolation was extracted without changing its original source/tests.
Re-read the goal, confirmed #216 merged and revalidated the four existing live
handles (`78215`, `65754`, `63735`, `24069`). No restarted builds or old-head
claims. Repository instructions and original working state remain unchanged.

Read-only next-boundary inventory groups `7f14e4b`, `981d0e3` and `3bb62ac` by
their shared identity-projection responsibility, rather than making three tiny
deliveries. The first introduces a legacy identity-only user-list DTO; the second
returns approval names in the authorized result instead of fetching a directory;
the third restricts the account-menu session DTO. The final user model, approval
model/page/UI tests and three HTTP test modules remain unchanged through `db3935d`.
The directory HTTP test must use its final version: `3bb62ac` replaces its former
expectation of sensitive own-user fields with the explicit session-identity
regressions. Do not restore those superseded expectations or drop the replacement
checks. Keep compatibility API financial projections outside this payload repair.

Current-master caller inventory includes Clients and invoice recovery added after
the original branch fork; preserve those consumers and compile them against the
narrowed identity. New Project's rate-bearing person DTO is a separate endpoint,
not permission to retain unnecessary rates in the directory response. Existing
registered-session test helpers are present on master: carry only the three
relevant modules/calls, not the original parent harness's unrelated canonical
authorization tests. This is dependency inventory, not a completed source review
or runtime acceptance, and no identity extraction has been created yet.

Approval isolation suite `78215` passed on `d72b830`: 823 app, 180 integration
and 121 core tests, 1,124 total, with 11 existing manual tests ignored. The four
original two-tenant helper regressions and the weekly behavior checks remain
unchanged. Complete SQLx preparation continues in the same private-DB session.

#236's original full Nix handle `65754` exited zero with all checks passed on
`95bdf4a`, including the completed browser matrix and NixOS e2e. Updated its PR
record. This is the local x86_64-linux result on the combined review base; it does
not replace future master-retargeted CI or authorize merging into that base.

Approval SQLx preparation completed (`78215`): 997 unchanged base descriptors,
eight byte-identical original additions and six superseded removals, 1,005 total.
The complete cache stable patch ID `260a95e0438066d47c0ac3aaa5d30ae13776a730`
matches original `2abce9a` exactly. Saved unsigned `66a256f8f2e8696e42d30eaca981b7e62dfbdfe4`
and published independent draft [#239](https://github.com/numtide/horae/pull/239),
11 files, 318 insertions/47 deletions including generated metadata. Offline
server then WASM Clippy run sequentially in `79148`; full clean-head Nix runs
independently in `87204`. No new schema or scoped approval activation.

#237's first full Nix process `63735` terminated with exit 1 after a successful
release build. Its browser derivation
`/nix/store/rahz5l50fgmd9qsmnazjwyzm2x71kh2j-horae-browser-checks.drv`
failed at `menu-popovers.cjs:204`: the last project-row menu remained visible
five seconds after decrementing its table's horizontal scroll offset. Read the
full failure log and the assertion/scroll handler. No pending request or browser
error was reported. The test, menu JavaScript/component and CSS are byte-identical
to #236, whose full browser suite just passed; requester code changes no UI path.
That comparison does not establish the root cause or turn the failed run green.
Updated #237's body and started one unchanged-head diagnostic full Nix rerun in
`30447`, preserving the first failure. Do not weaken the assertion or claim a
confirmed flaky cause. If reproduced, investigate with retained failure evidence
while continuing independent extractions; no unrelated UI repair is authorized.

Remote inspection confirms #239 is draft, targets master and contains exactly
`66a256f8f2e8696e42d30eaca981b7e62dfbdfe4` with the 11-file bounded diff.
Its combined lint process `79148` completed the offline workspace/server phase
successfully (3m13s) and advanced to WASM; do not count the combined process as
finished until that second phase exits. #237 diagnostic `30447` is rebuilding
only the three unfinished browser/VM derivations, reusing the successful compiled
package and other checks rather than restarting its release compilation.

#239's combined lint `79148` exited zero after WASM completed in 46.54s. Both
offline targets pass on published `66a256f`; the shared local Cargo target is
now free. Updated its PR body. #238's Nix test derivation completed successfully,
but full Nix `24069` remains live. #239 `87204` and #237's unchanged diagnostic
`30447` are also live. Keep the original #237 failure visible until the diagnostic
result is known; a partial log without that assertion is not proof of success.

Next independent work is the inventoried identity-projection group: complete
its source/consumer review before extraction, preserve current-master Clients
and invoice recovery, retain final HTTP/actual-page tests and avoid canonical
permission dependencies. No identity code has yet been changed. Mixed spec hunks,
governance and other UI/domain groups still require ownership and verification;
the overall separation goal remains incomplete.

### 2026-10-06 — Identity extraction and requester diagnostic result

The intervening response drafted the requested goal but changed no repository
state: classified as no progress. Re-read the attachment, confirmed #216 merged,
fetched unchanged master `1b8fa4f` and revalidated the three live Nix handles.
Tracked original backup and untracked archive comparisons both pass.

#237 diagnostic `30447` exited zero on unchanged `2242361`: all local
x86_64-linux Nix checks passed. Its browser log explicitly reports PASS for the
exact scroll/resize assertion that failed in `63735`. Updated the PR without
erasing the first failure or claiming a proven cause. Retargeted CI is still
required. #238 `24069` and #239 `87204` remain live; #239's tests, SQLx and
server-Clippy derivations completed, not its whole flake.

Completed bounded identity review with ponytail and Rust skills: minimal session
and directory fields, same-org approval-name join without excluding archived
submitters, preserved totals/filters/actions, no policy activation. New Project's
financial DTO and Harvest compatibility API stay separate. Current-master Clients
and invoice recovery retain their consumed fields and navigation coverage.

Correction to earlier inventory: the final directory HTTP file also includes
`1eb13ec`'s canonical `check_scoped`. Extracted the original `7f14e4b` legacy
matrix with exactly `3bb62ac`'s four superseded own-user assertions removed;
the preserved session matrix replaces them. The canonical helper/call stays owned
by `1eb13ec` in the original backup, not deleted or waived.

Created independent branch/worktree `fix/identity-response-projections` /
`.worktrees/identity-response-projections`, unsigned `6ff00f0`: 14 files,
689 insertions/88 deletions. Six complete model/page/test blobs match both
`3bb62ac` and final `db3935d`. Remaining production changes are selected original
hunks; harness changes only register the three relevant matrices. Navigation
fixture changes preserve master's member branch and expanded Clients tests.
No new schema, CSS, dependency or canonical endpoint. Format `25858` passed
unchanged. Suite followed by SQLx regeneration runs in `19675` on disposable
PostgreSQL. The initial touch used a nonexistent core path; no file was created,
and compiler output confirms actual `crates/core` and app are rebuilt here.

Ran Spec Kit analyze's prerequisite command once in the original worktree;
feature 015 resolves and no extension hooks exist. Applicable spec/plan/tasks,
contracts and constitution 1.1.0 were read. Nine tasks T145–T147/T151–T153/
T162–T164 map to FR-002/006/008/010/018 and the relevant SC-006 regressions.
Six requirement IDs have bounded task coverage; no unmapped tasks or new
ambiguity, duplication, critical/high or constitutional finding. One low note:
historical T150→T151 sequencing is not a runtime dependency on the canonical
directory. No feature artifact changed and no full-feature acceptance is claimed.

Next: collect `19675`, verify cache provenance, run offline server/WASM lints,
publish the draft and start clean-head full Nix. Do not use the shared local
Cargo target until this suite/cache process finishes. Mixed specification hunks
and remaining groups still need ownership; original worktrees and #208 untouched,
no merges, original closures or real data changes.

Identity suite/cache `19675` exited zero: 818 app, 183 integration and 121 core
tests, 1,122 passed with 11 existing manual tests ignored. The production-page
approval tests, real-session matrices and 44 detail-navigation tests passed.
Regenerated cache has 1,001 unchanged base descriptors, eight byte-identical
original additions and two obsolete removals, 1,009 total. Seven additions
originated in the two selected query commits; one fixture descriptor already
existed through `3ae8e08` in the original parent, with no related implementation
imported. Unsigned cache commit `1ce993f75092ca5dc81eb5f96d2a51a6537a8d6d`.

Published independent draft [#240](https://github.com/numtide/horae/pull/240).
Remote inspection confirms master base, exact head, 23 files and 871 insertions/
185 deletions including generated metadata (Git recognizes one cache rename).
Offline server then WASM lint runs sequentially in `38653`; full clean-head
Nix runs separately in `16586`. Both are live, not counted as passed. The shared
local Cargo target remains occupied by `38653`.

Read-only next-boundary inventory: `5ec183a` fences six interactive time
transaction entry points against current account deactivation; original
`activity_tests.rs` is unchanged through final `db3935d`. Its dependencies are
the organization lock helper and bounded READ COMMITTED configuration, not the
complete canonical permission module. Read its contract, source diff and all
four activity tests. Later `60f60f9`, `5faed76`, `48a6533` and `02c4245`
modify the shared time file and stay separately accounted. Complete caller,
service-barrier and current-base review before choosing its smallest valid base;
no time extraction has been created. `c80233b` separately repairs time-entry
invoice-ID serialization, with legacy HTTP checks embedded in the canonical
time matrix: retain those checks if extracting it independently, not the unrelated
canonical setup. Neither inventory item is runtime verification or feature work.

### 2026-10-06 — Time-writer activity extraction

The intervening goal-prompt response made no separation progress. Revalidated
the next safe action: #216 is merged at `02f7b58`, original tracked backup and
untracked archive comparisons pass, and the next boundary has not been extracted.
No original branch, worktree or #208 was changed.

#238 full Nix `24069` exited zero on `7a7cede`: all local x86_64-linux checks,
including browser and both NixOS VM suites, pass. Its PR description now records
this result; kept draft for coordinated review. #239 `87204` and #240 `16586`
were confirmed live. #240 offline server/WASM lint `38653` passed and PR-body
update `1841` completed; the shared local Cargo target became available.

Completed the time-writer caller/service/approval review. The existing submission
path takes the exclusive user advisory barrier before entries and only compatible
organization/user FK locks; the new interactive prefix never upgrades its
organization SHARE lock. Legacy user deactivation and project access ordering
are provided by #227/#228. Imports still use the unchanged lower-level advisory
helper; no service execution is recast as a user operation.

Created `fix/time-write-activity` in `.worktrees/time-write-activity` on #228
`0e1e6759cff0b80a046ced4694a78022f096ab3d`. Unsigned source commit `3ea235f`
extracts `5ec183a` T171–T173: four files, 402 insertions and 22 deletions.
`db.rs` and `update_tests.rs` match the original commit. The 300-line activity
test file matches both `5ec183a` and final `db3935d`, blob
`2935efd06339ea75164a7708da637652c5c3ce81`. Every original assertion is retained.
The time module differs only by preserving the base's absence of the unrelated
canonical reader and inlining the original two transaction-configuration queries
from `permissions::configure_administration`. SQL strings and behavior are
unchanged; this avoids importing the complete canonical permissions module for
one call. No schema, CSS, API surface, dependency or policy activation is added.

Read and applied ponytail, Rust best-practice, testing and async guidance. Ran
Spec Kit analyze's prerequisite command in the original worktree (015 resolves;
no extension hooks), read the relevant spec/plan/tasks/contract and constitution.
Bounded coverage: FR-010 and SC-003 map to T171/T172; FR-018 and SC-006 map to
T171/T173. Four applicable requirement/criterion IDs, three tasks, all mapped;
no ambiguity, duplication, constitutional conflict or critical/high finding in
this boundary. This is not full-feature acceptance. Original specification
hunks remain preserved for the documentation reconciliation, not silently lost.

Formatting `46067` passed with zero changes; staged diff checks pass. Disposable
PostgreSQL workspace suite followed by SQLx regeneration is live in `64428`.
It owns the shared local Cargo target; do not start another local Cargo process.
Next: inspect its result and regenerated cache provenance, then run offline
server/WASM lint, publish the bounded draft stacked on #228 and start clean-head
Nix. Original full cross-command/delegated-policy gates remain separate work.

#239 full Nix `87204` subsequently exited zero on unchanged `66a256f`:
all local x86_64-linux checks passed, including browser and both VM suites.
The time extraction compiled successfully in `64428` and advanced to tests;
the combined suite/cache process is not yet complete. #240 `16586` remains live.

Next-boundary read-only inventory: `c80233b`'s complete time-entry model change
is independent of canonical grants and UI. Its original real-session legacy
matrix precedes policy setup inside `scoped_time.rs`; preserve the fixture and
all assertions when registering it independently in the existing singleton
HTTP harness. Current-master time UI uses state, not `invoice_id`; invoice
recovery and the Harvest compatibility API use distinct models. No payload
extraction has been created yet and this consumer inventory is not runtime proof.

Time suite/cache `64428` exited zero: 841 app, 180 integration and 121 core tests
(1,142 passed; 11 existing manual tests ignored). All five extracted regressions
and existing submission, task-revocation, user-access and invoice tests pass on
source `3ea235fa271d9caf3de9674cba949b9c7fb4e118`. Full SQLx regeneration
retains all 1,031 base descriptors unchanged and adds 11, each byte-identical to
`5ec183a`; six were already present in its original parent. No removals or
modified base descriptors. Unsigned cache commit
`7820f8da64ef52169c23e2967a3c16a1a74070a8`.

Published draft [#241](https://github.com/numtide/horae/pull/241), stacked on
`fix/project-access-lock-order` (#228). Its 15-file diff contains four source/test
files and 11 generated descriptors, 614 insertions and 22 deletions. Required
integration order is #227 → #228 → #241; retarget and revalidate after prerequisites
land. Combined offline server/WASM lint `74243` and clean-head full Nix `85167`
are live on `7820f8d`, not counted as passed. The shared local Cargo target is
owned by `74243`. #240 full Nix `16586` remains live, with its test derivation
passed but no whole-flake completion yet.

This iteration is progress: original time-writer work is now isolated, published,
source-accounted and suite/cache-verified; #238/#239 full local Nix results are
recorded in their PRs. Next collect the live gates and continue the separately
inventoried time-entry payload boundary without importing canonical reader setup.
Remaining original feature/UI/specification/governance hunks and unfinished local
Clients work still prevent completion of the overall separation goal.

### 2026-10-06 — Time-entry response boundary

Previous iteration classified as progress: #241 was published with suite/cache
evidence and the preservation ledger pushed. Re-read the objective, confirmed
#216 merged, fetched unchanged master `1b8fa4f` and revalidated all three live
handles. Original tracked snapshot comparison still passes. #241 offline lint
`74243` subsequently exited zero: server and WASM passed on `7820f8d`. Updated
its PR body and the delivery/accounting tables, including #238/#239 full local
Nix completion. #241 full Nix `85167` and #240 `16586` remain live.

Created independent `fix/time-entry-payload` in `.worktrees/time-entry-payload`
on master `1b8fa4f`. Unsigned source `3ff9b8818716473c1ec2a674480cd634ad532ae3`
contains three source/test files, 128 additions, extracting `c80233b` T168–T170.
The complete shared model matches both that commit and final `db3935d`, blob
`cbfba91bba08e68f38be22b33cc643c423e177a8`. No later original commit modifies it.

Preserved the original populated-invoice HTTP fixture and complete legacy-role
loop in `authorization_tests/time_entry_payload.rs`, registered in the existing
singleton HTTP matrix. Only the unrelated canonical import, unused actor-cookie
lookup and subsequent canonical setup/checks were excluded. Those checks remain
owned by the canonical time reader, not discarded. Session-only identity, foreign
input, exact field omission, one returned owned entry and the unchanged stored
invoice relationship are still asserted for all three legacy roles.

Applied ponytail and Rust/test guidance. Bounded review traced shared response
sites, SQLx decoding, model input/output and all invoice-id consumer references:
the time UI uses state; plugin payloads and invoice/report/Harvest projections
use separate models. No production query, schema, CSS, dependency, action grant
or canonical activation changed. Existing contract and task provenance remains
FR-008/018; this is not a fresh full-feature Spec Kit acceptance claim.

Formatting `88301` passed unchanged; diff checks and model provenance pass.
Workspace suite plus complete SQLx regeneration runs on disposable PostgreSQL
in `37602`, using the shared local Cargo target now released by #241's lints.
Do not start another local Cargo command until it finishes. Next verify cache
provenance, run both offline lints, publish the independent draft and start Nix.

Read-only subsequent-boundary inventory: project-manager delegation was introduced
by `9a7e05d`, hardened by `202ee96`, exposed by `84d5352` and made composable
with project editing by `2497dbe`. Final models/readers/commands and transaction
tests must be reviewed together; `PermissionRequester` is a small shared DTO
absent from the current #234 model. Actual organization-lock and schema dependencies
must be resolved before extraction. No delegation code was changed or certified.

Time-payload suite/cache `37602` exited zero: 821 app, 180 integration and 121
core tests (1,122 passed; 11 existing manual tests ignored). Complete SQLx
regeneration retains all 1,003 base descriptors unchanged and adds three,
byte-identical to original `c80233b`; 1,006 total, no removals or modified base
descriptors. Verified the extracted HTTP fixture/legacy loop equals the original
slice after only the documented import/cookie/comment adaptations. Unsigned cache
commit `43337fc4180b8a0c590408f5525a2bddf4c7567f`.

Published independent draft [#242](https://github.com/numtide/horae/pull/242) on
master, six files and 173 additions (three Rust/test files and three generated
descriptors). Remote inspection confirms the exact head, base, draft flag and
diff size. No critical/high finding in the bounded payload review; full gates
are still pending. Offline server/WASM lint `22698` and clean-head Nix `73808`
are live on `43337fc`. The shared local Cargo target is occupied by `22698`.
#240 `16586` has completed release compilation and advanced to browser/VM checks;
#241 `85167` has passed its SQLx and server-Clippy derivations but remains live.
Neither whole-flake result is inferred from partial success.

Further delegation dependency evidence: the shared requester DTO originated in
`6bba224`; preserve its exact eight-line definition without importing the entire
editor. #234 has the receipt/assignment schema and profile/template foundations
but lacks `db::lock_organization`, so it alone is not a valid base for the final
composable command. Read the final session wrappers and strict DTOs; subsequent
whole-file command, transaction/reader/HTTP test and contract review remains
required before creating that extraction. No new decision or feature is needed.

This iteration is progress: #242 is isolated, source-accounted, suite/cache-verified
and published. Next collect the four live checks without restarting them, then
resolve and review the project-delegation boundary. Remaining canonical/UI/spec
and unpublished Clients groups still require final ownership; goal not complete.

### 2026-10-06 — Composable project delegation extraction

The immediately preceding turn only supplied the requested goal text: no-progress
for the existing separation goal. This iteration re-read that goal, reconfirmed
#216 merged at `02f7b58`, and checked the three existing Nix handles live without
restarting them. Original tracked snapshot and untracked archive comparisons both
pass. #242 offline server/WASM lint `22698` completed successfully on `43337fc`;
its PR body is updated. #240 `16586`, #241 `85167` and #242 `73808` remain live.

Read the complete final delegation command, strict DTOs, session wrappers, all
three PostgreSQL test files, HTTP fixture, shared transaction helper and the
project-management command contract. The final command needs both #234's schema/
profile-template foundations and #228's shared organization-lock helper. Created
isolated review base `integration/project-delegation-prerequisites` at
`e44433e3dea8ab92b26149f2b98a2af122086092`, combining exact #234 `45d219e` and
#228 `0e1e675` without conflicts. This local integration base is not a delivery
PR or a GitHub merge target. No existing branch was moved.

Created `feat/project-manager-delegation` in its own worktree on that base.
Unsigned source commit `be787cad1405ff42a3f93c0ef2604fa5bc3587eb` has 12 files,
2,334 additions and one deletion. This is one coherent command/API boundary,
mostly existing tests, not a new permissions feature. Original provenance:
`9a7e05d`, `202ee96`, `84d5352`, composability hunks from `2497dbe`, and the
eight-line requester DTO from `6bba224`.

Six full files match final `db3935d` byte-for-byte: project-manager DTOs, session
wrappers, command, command tests, read tests and transaction tests. Root module
registrations are minimal. The requester DTO is shared on server/WASM; existing
profile/template command types retain their server-only compilation boundary
using item-level cfg attributes. No schema, dependency, UI, style or activation
change was added. Legacy project saves remain unchanged.

The original HTTP test is preserved except for its audit-endpoint block starting
at `let request_id: Uuid` and ending immediately before the subsequent
`grants(...ProjectWriteManaged...)` call. Its two forbidden assertions for
`get_permission_audit` and `list_permission_audit` require the separately
retained audit API and explicitly belong to that extraction/integration suite.
They are not waived or reported passed here. The original file and its fixture
remain recoverable unchanged. All delegation route, requester mismatch, payload,
replay, stale revision, retained inactive target, self-removal, malformed-state,
same-cookie revocation and receipt-count assertions are otherwise exact.

Bounded adversarial review traced session identity through current policy/grant/
designation checks, organization-first serialization, actor/added-target SHARE,
project KEY SHARE NOWAIT, exact normalized intent, durable receipt/audit,
overflow, no-op preservation and whole-transaction rollback. Current authority
is checked before replay; retained targets are not silently removed for lost
eligibility. Membership, rates, costs, history and other management relationships
remain unchanged. Composition does not commit or change caller isolation.
No critical/high finding in this extracted boundary; runtime tests remain gates.

Ran Spec Kit analyze's prerequisite command once with
`SPECIFY_FEATURE=015-scoped-permissions`, from the original worktree; required
artifacts exist and no extension hooks are configured. Read its constitution,
relevant specification/plan/task sections and full delegation contract, without
editing original specs. Scoped report:

| Requirements | Tasks | Coverage / limitation |
| --- | --- | --- |
| FR-005/011/026 | T059–T061, T189–T191 | Distinct scope, current project-edit authority, compatible additions; no grant promotion |
| FR-010 | T133–T135, T189–T191 | Both activity race orders, revocation, cancellation and current replay authority |
| FR-013 | T059–T061, T189–T191 | Atomic receipt/audit and sanitized payload; audit-reader assertions owned separately |
| FR-017 | T059–T061 | Membership, money and history preserved; composition commit/rollback tested |

Six scoped FRs, nine mapped tasks, 100% task coverage, zero unmapped tasks and
zero critical/high specification findings in this boundary. SC-002/003/006 have
corresponding payload/concurrency/regression tests but full-feature success is
not claimed. Master constitution 1.0 and the original proposed 1.1 governance
reconciliation remain separately inventoried; this extraction adopts no amendment.
Next analysis action is to validate the extracted source, then restore audit
integration assertions with their owning API; no speculative spec edits needed.

Formatting `82393` passed unchanged and diff checks pass. Workspace suite plus
complete SQLx regeneration `4502` is running on disposable PostgreSQL, occupying
the shared local Cargo target. Next collect it, verify every generated descriptor,
run offline server/WASM lint and full Nix, publish the bounded draft and update
this record. Overall separation remains incomplete.

Delegation suite/cache `4502` exited zero: 926 app + 180 integration + 189 core =
1,295 passing tests; 11 existing manual tests ignored. Regeneration preserves all
1,125 base SQLx descriptors unchanged and adds 46 byte-identical to final original
`db3935d`, 1,171 total; no deletion, modification or unmatched addition.
Verified the HTTP file equals the original after removing exactly the recorded
audit block. Unsigned cache commit `1818bbf38453e13ea59e49fde16832ac84c4b2ab`.

Published draft [#243](https://github.com/numtide/horae/pull/243); remote verification
confirms the exact head/base, draft flag and 58-file diff (12 Rust source/test files
plus 46 SQLx descriptors, 3,197 additions and one deletion). Both branches are
pushed. Offline lint `56569` and full Nix `22335` run on the cache-inclusive head;
Nix's formatting derivation has passed but the whole run is not yet complete.
The shared local Cargo target is occupied by `56569`.

Read-only next-boundary inventory: audit history comes from `fc85231`,
`300d1e9` and `8722320`. Read the complete final audit backend, historical DTOs,
two session wrappers and audit-lookup contract; inspected test/UI dependency
registrations, not the full test/UI implementation. Audit fixtures use actual
template/profile/delegation commands, so #243 is a direct prerequisite. The
browsable page also depends on shared permission descriptions, own-access
navigation and canonical shell admission; resolve that coherent delivery boundary
before extraction rather than dropping those implemented UI paths. Restore the
two deferred delegation/audit HTTP assertions with the audit API. No audit
extraction or UI certification is claimed yet.

This iteration is PROGRESS: #243 is published, original-code/accounting evidence
and 1,295 tests plus complete cache validation are recorded. Next collect its
offline lint and existing Nix handles, then review/extract the audit-history
boundary. #240 `16586`, #241 `85167`, #242 `73808` remain live; #241 advanced
through release build into browser/VM checks, not a whole-flake pass. No merges,
original closures, policy activation, data changes or new features.

### Own-permission extraction and delegation web lint boundary

The preceding prompt-only response made no implementation progress. Revalidated
the next safe action: #216 is merged at `02f7b58`; the original tracked snapshot
still matches its backup. Polled the actual verification handles, not CI on a
timer. #241 full Nix `85167` exited zero on `7820f8d`, including browser and VM
checks; its PR body now records the result. #240 `16586` and #242 `73808` remain
live, so no complete result is inferred from silence or individual derivations.

#243 initial offline lint `56569` passed server but failed WASM on unused DTOs.
The separate UI consumer is not in this extraction. The first conditional
expectation change (`9c6b121`, twelve lines) was too broad: `40252` passed server
but reported two unfulfilled nested expectations in WASM. Corrected to a single
`expect(dead_code)` on the unused `ProjectManagers` response root, following the
existing command/outcome convention. Final unsigned `3404c85` differs from
`1818bbf` by only four conditional-attribute lines; DTO fields, wire types, SQL,
runtime code and tests are unchanged. Offline WASM `49707` exited zero on this
source. Five complete command/wrapper/database-test files still match the
original; the DTO file now has this explicitly accounted adaptation. Current-head
full Nix `87574` is running. Older Nix `22335` (`1818bbf`) and `90130` (`9c6b121`)
were explicitly interrupted after confirming their PID, worktree and start time
(`3340917` and `3413583`); both handles exited one with the expected interruption.
They are not verification of the final head. The final-head check is retained.
Final-head server
verification remains pending, despite the unchanged server compilation path.

Audit-history dependency review identified the existing own-access Settings
consumer as a coherent prerequisite. Created `feat/own-permission-settings` and
its isolated worktree on #234 `45d219e`, preserving the original reader, display
DTO, seven database tests, registered-cookie checks, Settings section, exhaustive
grant descriptions, nine SSR tests and production-resource refresh test. Unsigned
source commit `0b0da87` contains 14 files and 1,115 added lines; no migration,
dependency, CSS, legacy guard replacement or policy activation.

Six complete files match final `db3935d` byte-for-byte; the component and shared
description file match `8c15bfe`. Their later audit link and `profile_label` helper
remain owned by the audit extraction, not discarded. `get_my_permissions` retains
the final `8722320` authentication-error sanitization. Existing `get_me` stays
unchanged here because its separate projection is #240. General and Plugins are
unchanged apart from insertion of the independently loading section.

Scoped Spec Kit analyze ran the prerequisite script against original feature 015
with no extension hooks. FR-006/007/010/012/016/017/018 map to T095–T097 and
T120–T122: seven requirements, six tasks, all mapped, no critical/high,
duplication or ambiguity finding in this boundary. SC-002/003/005/006 remain
partial; T018 browser/Workspace acceptance and the original proposed governance
amendment are not declared complete or silently adopted.

Source review traced session-only identity, explicit policy/active-actor checks,
organization-before-actor locking, strict canonical restore, sorted own scopes,
non-mutating reads and sanitized errors. UI review verified exact selected grants,
independent administrator identity, no raw IDs/revisions, loading suppression,
legacy/empty/error distinctions and guarded refresh. No critical/high source
finding in this boundary. `nix fmt` `92652` passed with zero changes; Impeccable's
manual detector returned `[]`. These are not contrast, keyboard or viewport
certification. No rendered browser acceptance is claimed.

Disposable full workspace suite and SQLx generation run in `50483`; the shared
local Cargo target is now occupied by that process. Next collect its result,
verify cache provenance, run offline lints and full Nix, then publish a scoped
draft with explicit browser limitations. Continue with audit history afterwards,
restoring #243's deferred audit-denial assertions. This iteration is PROGRESS;
the overall separation is still incomplete. No merges or original closures.

### Own-permission delivery published; isolated browser verification added

Previous iteration is PROGRESS: the original own-permission flow was extracted
and the delegation lint boundary corrected. This iteration revalidated #216 as
merged and polled the existing live handles without restarting them. #242 full
Nix `73808` exited zero on `43337fc`, including browser and VM gates; its PR body
now records the confirmed result. #240 `16586` remains live and quiet. A read-only
diagnostic retrieved an empty current browser-derivation log; that supplies no
failure diagnosis or reason to restart the live check.

Own-permission suite/cache `50483` exited zero: 893 app + 191 integration + 189
core = 1,273 tests; 11 existing manual tests ignored. Source under test is
`0b0da87`; browser-only follow-up `12cac34` does not alter that Rust source.
Complete regeneration retains all 1,098 base SQLx descriptors byte-for-byte and
adds 26 identical to final original `db3935d`, 1,124 total, no removals or modified
descriptors. Unsigned cache commit `6c4e4d134b9c2924085e56d504c3a684752fb7c4`.

Inspection of the original browser suite found its Settings visit only exercises
the later audit link, not this independent own-permission consumer. Added the
bounded `own-permissions.cjs` verification and registered it in the existing
disposable runner (`12cac34`, 113 additions/one deletion, no production edits).
The test validates the private test socket/URL before fixture writes, exercises
real session reads, legacy-to-canonical explanation, exact grants, independent
administrator identity, native keyboard refresh, pending-content suppression,
invalid-state recovery and deactivation. Geometry checks cover both themes,
320/390/768/1440 widths, a short viewport and CSS zoom 2. This last case is not
native text zoom. Optional two-theme screenshots are available for inspection.
Fixtures are restored before the runner stops its temporary PostgreSQL.

Node syntax `7136` and formatting `32758` passed; no rendered-browser pass is
claimed yet. This is verification of preserved functionality, not a new feature.
The original files, legacy controls and CSS remain unchanged. The broader T018
Workspace/native text-zoom/full visual acceptance is still separate.

Published draft [#244](https://github.com/numtide/horae/pull/244), stacked on #234's
`refactor/person-profile-commands` (`45d219e`), head `6c4e4d1`: 42 files (16
source/test/registration files plus 26 SQLx), 1,674 additions and one deletion.
Offline server/WASM lint `53333` and full clean-head Nix `19313` are running on
that exact head. The full gate includes the new isolated browser test. The shared
local Cargo target is occupied by `53333`; do not overlap another local build.

Next collect those results and inspect the optional Settings captures using the
built package once available. Continue the coherent audit-history extraction on
#243 + #244: source inventory confirms its backend/UI derive from `fc85231`,
`300d1e9` and `8722320`. The `8722320` admin-shell revision gates only Audit with
own canonical identity and preserves the incumbent People/Importers guards;
later People/Tasks shell changes must retain their separate ownership. Restore
the deferred Settings audit link, profile labels and #243 audit-denial assertions
with those endpoints. No audit implementation is claimed in this iteration.

This iteration is PROGRESS: #244 is published with full Rust/cache evidence and
explicit browser/lint/Nix limitations; #242's full gate is now confirmed. #243
current-head Nix `87574` and #240 `16586` remain live. Original tracked backup
comparison still passes. No merges, original closures, policy activation or real
data changes; the overall separation remains incomplete.
### Permission history extracted; own-permission browser evidence confirmed

The preceding user-facing turn only supplied a goal prompt: NO PROGRESS toward
the existing separation objective. This iteration reread that objective,
revalidated #216 as merged, and made concrete progress without changing scope.

#244 offline server/WASM lint `53333` exited zero on `6c4e4d1`. The exact Nix
package subsequently became available at
`/nix/store/2kv7laqd6z31ibgrc5c0hfnq5ckvaimx-horae-0.1.0`.
Focused disposable browser run `47268` exited zero in Chromium 148.0.7778.96:
real session reads, legacy/canonical distinction, exact grants, explicit admin
identity, keyboard refresh, pending suppression, malformed-state recovery,
deactivation and both-theme responsive/CSS-zoom cases passed. Desktop-dark and
390px-light full-page captures were inspected in
`.worktrees/own-permission-settings/.scratch/browser-own-permissions-6c4e4d1/`;
no section clipping or overlap was observed. The test deliberately sets the DOM
theme, so the unchanged General selector can still say Dark in the light capture.
No native text-zoom, touch gesture, cross-browser or contrast certification is
claimed. The PR body now records the actual evidence; a transient GitHub GraphQL
failure was retried successfully.

Created and published review base
`integration/permission-audit-prerequisites` at
`59d27981db88bfa2c26bd96f39bff995b45901e3`, combining exact #243 `3404c85` and
#244 `6c4e4d1`. Two module/test-registration conflicts were resolved by retaining
both modules and both HTTP checks. This is a local composition, not a GitHub PR
merge or a future merge target. After both prerequisite chains reach master,
retarget/rebase and verify the delivery there.

Created and published `feat/permission-audit-history` in its own worktree.
Unsigned source head `c96d78765162a9a9a695e88840c2bcd34677cd0c` has 18 files,
3,211 additions and 13 deletions. It extracts the complete historical DTO,
single-record and paged readers, strict decoder, database/concurrency tests,
registered-cookie tests, native history details, requester-bound pagination,
Settings navigation and canonical Audit-only shell gate. Existing People and
Importers guards remain unchanged; no later People/Tasks shell changes, editor,
migration, CSS, dependencies or activation are included.

Ten complete files match final original `db3935d` byte-for-byte. Admin shell and
its tests match `8722320` after only adapting the two links and mocked route to
the base's existing Timesheet signature (no delegated-user selector). Restored
the complete 14-line audit-denial block deferred from #243's registered-session
test, plus #244's deferred Settings link and shared profile labels. The original
browser recovery suite's audit section remains preserved; it relies on the
not-yet-extracted editor. Reconcile that browser ownership and independently
exercise the audit consumer before declaring the delivery verified.

Scoped Spec Kit analyze ran original feature 015 prerequisites with no extension
hooks. Coverage is five requirements / ten tasks: FR-010/011/013 map to
T062–T064, T123–T125 and T182–T184; FR-016 maps to T183/T185; FR-018 maps to
T064/T125/T184/T185. All five have task coverage, no unmapped scoped task, no
critical/high ambiguity, duplication or constitutional conflict in this boundary.
T185 delivery evidence is pending here, not inherited from original #212.
Full T018/T041/T042, the broader SC-002/003/005/006 outcomes and proposed
governance amendment are not closed or adopted.

Bounded source review verified authorization before receipt lookup, tenant-only
25+1 paging, fresh READ COMMITTED with local limits, organization-before-actor
locks, strict historical decoding without raw intent/replay disclosure, explicit
admin identity, revocation/cancellation coverage and non-mutating projection.
UI review verified original escaped detail rendering, native disclosure controls,
requester binding, stale-content suppression and safe error/retry states.
No critical/high finding in that source boundary. Formatting `72211` passed with
zero changes; one detector invocation returned `[]`. These do not certify
rendered history or all accessibility dimensions.

Full Rust workspace execution in `96253` passed on `c96d787`: 969 app +
198 integration + 189 core = 1,356 tests; 11 existing manual tests ignored.
Its subsequent SQLx preparation did NOT pass: the combined process exited 143
after the private socket disappeared. The temporary PostgreSQL log
`/tmp/horae-import-cleanup-pg.94GG9k/log` records a fast shutdown at 19:49:11 UTC;
the PID file is gone and no local Cargo process remains. The source of the
termination is not established. Do not treat partially regenerated caches as valid.
Cache-only retry `62209` uses a fresh disposable database; the successful tests
are not rerun. Shared local Cargo target is occupied by that retry.

Next collect `62209`, prove complete SQLx provenance, commit the cache, run
offline native/WASM lint and full Nix on the final head, then publish a scoped
draft with its browser limitations. Continue the independent browser history
verification and remaining original ownership inventory. #244 Nix `19313`,
#243 `87574` and #240 `16586` were confirmed live this iteration; no full-pass
claim yet. Original tracked work still exactly matches its saved snapshot.

This iteration is PROGRESS. No merges, original closures, real-data writes or
policy activation occurred. The overall goal remains incomplete.

### Audit cache recovered and draft published

Cache-only `62209` exited zero but provenance correctly rejected its output:
one existing CLI-restart query was missing. Touching only that test and preparing
again (`6814`, exit zero) produced only three descriptors, proving that a
successful command can reuse other targets without regenerating their caches.
Neither incomplete result was committed. Invalidated timestamps for all Rust
source files in this isolated worktree and ran the same complete preparation
again (`69078`, exit zero). No source content changed and no successful test
suite was repeated.

Final provenance passed: all 1,195 base descriptors unchanged, 30 additions
byte-identical to original `db3935d`, 1,225 total, zero removals/modifications or
unmatched additions. Unsigned cache head
`533922ad904b4d93abed4a1db917086bb15feeed` is published in draft
[#245](https://github.com/numtide/horae/pull/245), on review base `59d2798`.
Offline server/WASM lint `72147` and full clean-head Nix `61137` are running.
The Nix formatting gate has passed; no full Nix result is claimed.

Next collect these exact handles, preserve the original editor-dependent history
browser assertions, and verify this historical consumer independently with real
writer-produced receipts in disposable fixtures. #243 already exposes the
session-bound project-manager command, so real history can be generated without
extracting the unfinished editor merely to obtain browser fixtures. All existing
template/profile historical tests and the original browser checks remain owned,
not replaced or dropped. Continue remaining original ownership after this
delivery's verification. No merges or activation.
