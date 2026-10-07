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
| Non-activating permission storage, [#222](https://github.com/numtide/horae/pull/222) | `refactor/permission-storage-foundation`, `.worktrees/permission-storage-foundation` | #219 `ec7ddbd` | Draft at `516b023`; original README ICU/migration note restored; 1,170 source-head tests, SQLx and offline lints passed; final documentation-head format and full local Flake Check `69471` passed; remote checks pending |
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
| Approval tenant isolation, [#239](https://github.com/numtide/horae/pull/239) | `fix/approval-tenant-isolation`, `.worktrees/approval-tenant-isolation` | Master `1b8fa4f` | Draft at `66a256f`; three source/test files and complete cache patch preserved exactly; 1,124 source-head tests, source review, SQLx, format and offline server/WASM Clippy passed; full current-head local Nix and required GitHub checks passed; initial remote browser failure and unchanged rerun retained |
| Identity response projections, [#240](https://github.com/numtide/horae/pull/240) | `fix/identity-response-projections`, `.worktrees/identity-response-projections` | Master `1b8fa4f` | Draft at `a19ea63`; source suite/cache/lints, corrected Clients fixture, complete local Nix `99785` and GitHub Flake Check passed; original failures retained; remote Nixbot failures remain unresolved; no activation |
| Time-writer account activity, [#241](https://github.com/numtide/horae/pull/241) | `fix/time-write-activity`, `.worktrees/time-write-activity` | #228 `0e1e675` | Draft at `7820f8d`; 1,142 tests, cache provenance, bounded review, format, offline server/WASM lint and full Nix `85167` passed; prerequisite integration/retarget/current-head CI still required; no delegated writes or activation |
| Time-entry invoice identity boundary, [#242](https://github.com/numtide/horae/pull/242) | `fix/time-entry-payload`, `.worktrees/time-entry-payload` | Master `1b8fa4f` | Draft at `43337fc`; 1,122 tests, cache/source provenance, bounded review, format, offline server/WASM lint and full Nix `73808` passed; required GitHub checks passed on unchanged rerun; initial cancellation-test failure retained; delivery review and Nixbot diagnosis remain; no policy or UI change |
| Session-bound project-manager delegation, [#243](https://github.com/numtide/horae/pull/243) | `feat/project-manager-delegation`, `.worktrees/project-manager-delegation` | Review base `e44433e` combining #234/#228 | Draft at `3404c85`; suite/cache, final WASM lint and full local Nix passed (cached result confirmed in `10972`); final server gate closed; no form wiring or activation |
| Own-permission explanation and Settings, [#244](https://github.com/numtide/horae/pull/244) | `feat/own-permission-settings`, `.worktrees/own-permission-settings` | #234 `45d219e` | Draft at `6c4e4d1`; 1,273 Rust tests, SQLx, provenance, review/Spec Kit/format/detector, offline server/WASM lint and isolated Chromium passed; desktop/mobile captures inspected; full local Nix passed (cached result confirmed in `30710`); no activation |
| Permission audit history, [#245](https://github.com/numtide/horae/pull/245) | `feat/permission-audit-history`, `.worktrees/permission-audit-history` | Review base `59d2798` combining #243 `3404c85` and #244 `6c4e4d1` | Draft at `14ad9ca`; source suite/cache/lints, corrected session-actor fixture and complete final local Nix `56923` passed, including own/history browser suites; original failures retained; remote Nixbot failures, prerequisite integration and retargeted gates remain open |
| Read-only legacy permission diagnostics, [#246](https://github.com/numtide/horae/pull/246) | `feat/permission-preflight`, `.worktrees/permission-preflight` | Review base `61c90bc` combining #237 `2242361` and #226 `82d15f3` | Draft at `ff482c8`; suite/cache/lints passed; full Nix `99711` failed on inherited editor loading timeout; unchanged focused transport/editor sequence and full unchanged-head rerun `41382` passed; initial root cause unproven; no endpoint, UI, migration or activation |
| Materialized XLSX/PDF authorization, [#247](https://github.com/numtide/horae/pull/247) | `fix/materialized-export-authority`, `.worktrees/materialized-export-authority` | Review base `3edc0b8` combining existing `0046dad` (#227/#228 + #220/#232) and #222 `e9695fd` | Draft at `d9717e7`; 1,229 tests, full SQLx/provenance (1,135 descriptors), source review/Spec Kit/format, offline native/WASM lint and complete local Nix `32625` passed; remote Nixbot failures, prerequisite integration and retargeted gates remain open; no policy activation |
| CSV delivery authorization, [#249](https://github.com/numtide/horae/pull/249) | `fix/csv-export-authority`, `.worktrees/csv-export-authority` | Exact #247 head `d9717e7` | Draft at `71232dc`; 1,245 source-head tests, full SQLx/provenance (1,174 descriptors), format/source review/scoped analysis and final-head offline native/WASM lint passed; complete local Nix `72947` passed; retargeted required checks remain; no canonical activation |
| Existing permission specification and history, [#248](https://github.com/numtide/horae/pull/248) | `docs/permission-specification`, `.worktrees/permission-specification` | Master `1b8fa4f` | Draft at `49843b2`; all 54 original feature documents preserved, six contextualized; all 43 requirements/criteria and 236 task lines unchanged; original New Project transition and AGENTS cache guidance preserved; provenance/format passed, full local Nix `51945` and required GitHub checks passed; final reconciliation pending; no code or constitution adoption |
| Requester-bound editor API, [#250](https://github.com/numtide/horae/pull/250) | `feat/permission-editor-api`, `.worktrees/permission-editor-api` | Review base `0117991` combining merged master `ed558f6` and #245 `14ad9ca` | Draft at `c727bc8`; four owned commits patch-equivalent to preserved `c82a5b3`; full native Nix `46207` PASSED, including complete browser/deployment/OIDC. Old `90520` selector failure retained; corrected helper inherited from merged master. Combined full native Nix `16434` passed on `7a2d61c`; retargeted gates remain; no UI or activation |
| Scoped people directory, [#253](https://github.com/numtide/horae/pull/253) | `feat/scoped-people-directory`, `.worktrees/scoped-people-directory` | Review base `0117991`, independent of #250 editor operations and #240 legacy projections | Draft at `3a37538`; original reader/DTO/endpoint and seven DB/HTTP tests preserved; tests/Clippy/live SQLx `18891` and full local Nix `2803` passed. Exact browser/e2e/OIDC outputs explicitly materialized from signed cache; combined full native Nix `16434` passed on `7a2d61c`; later reader composition and retargeted gates remain; no UI or activation |
| Identity-only project-team choices, [#254](https://github.com/numtide/horae/pull/254) | `feat/project-people-picker`, `.worktrees/project-people-picker` | #253 `3a37538`, for shared `PeopleCursor` and inherited foundations | Draft at `f498c3f`; original reader/DTO/endpoint, nine DB tests and HTTP assertions preserved; full native Nix `81722` PASSED, including complete browser/deployment/OIDC and exact cache reuse where available. Combined full native Nix `16434` passed on `7a2d61c`; retargeted gates remain; no picker UI, assignment writes or activation |
| Scoped time-entry reader, [#255](https://github.com/numtide/horae/pull/255) | `feat/scoped-time-reader`, `.worktrees/scoped-time-reader` | Review base `0117991`, independent of directory, project-team choices and editor operations | Draft at `d93e1af`; original `4ac30fa` DTO/reader/eight DB tests/HTTP assertions and endpoint preserved; 23 original SQLx descriptors, module registrations adapted only. Formatting/provenance passed; full native Nix `40092` PASSED; wider composition and retargeted checks pending; no Timesheet UI, subject discovery, commands or activation |
| Timesheet person discovery, [#256](https://github.com/numtide/horae/pull/256) | `feat/timesheet-people-discovery`, `.worktrees/timesheet-people-discovery` | Review base `40102ae`, combining #255 `d93e1af` admission reader and #253 `3a37538` shared `PeopleCursor` | Draft at `1552fdb`; original `60f60f9` DTO/reader/eight DB tests, endpoint/HTTP additions and five SQLx descriptors preserved. Tests/Clippy/live SQLx `61768` PASSED; application992 passed, zero failed,11 inherited ignored. Full native Nix `37414` PASSED; wider integration pending; no UI, context-page contract, commands or activation |
| Requester-bound Timesheet page context, [#257](https://github.com/numtide/horae/pull/257) | `feat/timesheet-page-context`, `.worktrees/timesheet-page-context` | #256 `1552fdb`, for subject discovery and shared read admission | Draft at `b30e3cd`; DTO/reader original, all six DB tests retained with a four-line cancellation-barrier synchronization correction. Old `8e09e60` full gate passed; dependent `d2b45ca` exposed a test timeout, retained below. Current-head full native Nix `40402` PASSED, including998 application tests, zero failed,11 inherited ignored, browser and NixOS/OIDC; retargeted checks remain |
| Person-bound Timesheet commands, [#258](https://github.com/numtide/horae/pull/258) | `feat/timesheet-person-commands`, `.worktrees/timesheet-person-commands` | #257 `b30e3cd`, for shared context contracts and foundations | Draft at `b0eacfd`; original commands/tests and39 descriptors unchanged. Old `64524` failure retained; corrected-head tests/Clippy/live SQLx `98625` PASSED (1011 application tests, zero failed,11 inherited ignored). Full native Nix `7633` PASSED; wider integration pending; no UI or activation |
| Selected-person Timesheet UI, [#259](https://github.com/numtide/horae/pull/259) | `feat/timesheet-selected-person-ui`, `.worktrees/timesheet-selected-person-ui` | #258 `b0eacfd`, for page context, discovery, tracking and commands | Draft at `4ce0919`; original UI/submission contract/tests/cache plus recovered original Nix asset path and two-line calendar pointer-target CSS. Nine whole source/test files byte-identical to final original. Failed gates `48062`/`91457` retained below; full native Nix `75088` PASSED; published wider combination `1a87961` gate88592 running; no activation or new feature |
| Scoped People and permission editor UI, [#260](https://github.com/numtide/horae/pull/260) | `feat/people-permission-editor-ui`, `.worktrees/people-permission-editor-ui` | Verified review base `7a2d61c`, combining #240/#250/#253/#254 and foundations; actual consumer dependencies are identity, editor API and directory | Draft at `98857cf`; fifteen complete files original, legacy tasks and current Clients retained. Sixteen JS tests, source review, whitespace/provenance pass; full native Nix `66677` PASSED. Published shared-navigation combination `1a87961` gate88592 running; no activation |
| Cross-PR Timesheet/permission verification only | `integration/timesheet-permission-check`, `.worktrees/timesheet-permission-integration` | Combines #240/#241/#250/#253–#262 and inherited foundations | Published at `1a87961`, no delivery PR; all five navigation guards,29 browser suites and both HTTP registries retained.19 navigation/storage tests and formatting passed; full gate88592 PASSED,1098 application tests/zero failed/11 inherited ignored and all remaining checks. Excludes later #263–#266 exports/access and future Reports consumer |
| Cross-PR reader/editor verification only | `integration/permission-readers-editor-check`, `.worktrees/permission-readers-editor-check` | Combines #240 `a19ea63`, #250 `c727bc8`, #253 `3a37538` and #254 `f498c3f` | Published at `7a2d61c`, no delivery PR or merge target; registration conflicts resolved preserving both sides, dedicated source/test blobs unchanged, original combined users module restored exactly. Tests/Clippy/SQLx `24884` and full native Nix `16434` PASSED; exact browser/deployment/OIDC outputs and logs verified after original process terminated. Later #255–#257 not included |
| Scoped detailed time report, [#261](https://github.com/numtide/horae/pull/261) | `feat/scoped-time-report-reader`, `.worktrees/scoped-time-report-reader` | #257 `b30e3cd`, shared time-read admission | Draft at `bf452dd`; final detailed reader,13 original DB tests, reader HTTP/totals assertions and19 original descriptors. Source/provenance/format pass; full native Nix `37229` PASSED; no UI/export/activation |
| Scoped grouped time report, [#262](https://github.com/numtide/horae/pull/262) | `feat/scoped-time-report-groups`, `.worktrees/scoped-time-report-groups` | #261 `bf452dd`, report contracts/totals and fixture | Draft at `66dbf0b`; final reader and13 DB tests exact; original171-line HTTP fixture and10 original descriptors. Source/provenance/format pass; full native Nix `79456` PASSED,1024 application tests/zero failed/11 inherited ignored plus all remaining gates; no UI/export/activation |
| Scoped XLSX time exports, [#263](https://github.com/numtide/horae/pull/263) | `feat/scoped-time-xlsx`, `.worktrees/scoped-time-xlsx` | Review base `e8b95cc`, combining #261 and #247 with their existing foundations | Draft at `8cc11c3`; original cbc78a8 materialization/release checks,11 DB tests and HTTP assertions preserved. Final query predicates retained;11 original cache additions/one obsolete descriptor removed. Full native gate48482 PASSED,1064 application tests/zero failed/11 inherited ignored and remaining checks; all ten derivations previously matched clean head. Wider exports/UI composition remains open |
| Scoped CSV time exports and shared filters, [#264](https://github.com/numtide/horae/pull/264) | `feat/scoped-time-csv`, `.worktrees/scoped-time-csv` | Review base `4e0ed43`, combining #263 and #249 | Draft at `55d362b`; original10 DB tests/five parser tests/two-format HTTP fixture retained; native cursor final predicates, shared XLSX release helpers and17 original descriptors. Source/provenance/format pass; full native gate40286 PASSED:1095 application tests/zero failed/11 inherited ignored plus all remaining checks; grouped exports/UI/policy binding remain separate |
| Report access and permission-mode-bound downloads, [#265](https://github.com/numtide/horae/pull/265) | `feat/time-report-access`, `.worktrees/time-report-access` | #26455d362b scoped downloads and shared parser | Draft at `f436a29`;7266abb backend preflight/DTOs and mode binding, original parser/HTTP assertions unchanged; format and source review pass; full native gate1825 running; no UI or activation |
| Grouped CSV/XLSX time exports, [#266](https://github.com/numtide/horae/pull/266) | `feat/grouped-time-exports`, `.worktrees/grouped-time-exports` | Review base `da493f6` combining #265 and #262 | Draft at `0eec1a4`; original grouped handlers/source/16 DB tests, shared CSV group authorization and18 descriptors; original grouped-filter HTTP fixture retained. Source/format/provenance pass; full native gate40587 running; later URL controls and consumer/browser integration remain open |
| Time download result filters, [#267](https://github.com/numtide/horae/pull/267) | `feat/time-report-download-filters`, `.worktrees/time-report-download-filters` | #2660eec1a4 and inherited detailed/grouped report/export foundations | Draft at `e029a89`; original active-project/billability URL parser, seven DB tests, strict transport and actual-session fixtures;10 original descriptors. Format/provenance/source review passed; full native gate51012 running; UI/browser filter propagation retained separately |
| Remaining #212 behavior groups | Original refs plus candidate inventory below | To be resolved from actual dependencies | Not submitted or certified; preserve every group until assigned to a resulting PR |
| Scoped Reports screen, [#268](https://github.com/numtide/horae/pull/268) | `feat/scoped-time-report-ui`, `.worktrees/scoped-time-report-ui` | #267 `e029a89`, inherited report access/readers and all four export routes | Draft at `aef180f`; original components,15 component tests and538-line isolated Chromium fixture. Format/source/provenance/syntax pass; gate50800 failed on two test-only user DTO references; minimal API-matching fix published and new full gate64831 running. No CSS/SQL/schema/activation; full T203 remains open |
| Cross-PR Reports/Timesheet/People verification only | `integration/reports-permission-check`, `.worktrees/reports-permission-integration` | Combines verified `1a87961` with #268 `810ce57` and its report/export foundations | Local integration merge `b7836d7`, not a delivery PR or GitHub merge. Both HTTP registration sets and all30 unique browser suites retained; dedicated source/test blobs unchanged. Format/syntax/source comparison pass; full native gate25958 running |
| Scoped project editor, [#269](https://github.com/numtide/horae/pull/269) | `feat/scoped-project-editor`, `.worktrees/scoped-project-editor` | `integration/permission-readers-editor-check` | Draft at `a9ba27b`; canonical fields, preserved hidden values and manager edits extracted. Lost-acknowledgement fixture failed in the full gate but passes isolated; failure remains unresolved. No activation |
| Scoped project overview/detail, [#270](https://github.com/numtide/horae/pull/270) | `feat/scoped-project-reads`, `.worktrees/scoped-project-reads` | `integration/project-read-prerequisites` | Draft at `387f80f`; current reader, budgets and bound UI extracted. Two fixture corrections pass focused Chromium; full native36963 running. ARM readiness and repeated-import failures remain open |
| Scoped project CSV/XLSX delivery, [#271](https://github.com/numtide/horae/pull/271) | `feat/scoped-project-exports`, `.worktrees/scoped-project-exports` | `integration/project-delivery-prerequisites`, combining #270 and #267 | Draft at `255aa94`; requester and financial release guards plus original DB/HTTP tests extracted. Full browser gate failed the inherited fixed project-count assertion; #270 corrections not yet propagated |
| Scoped Harvest-compatible project reads, [#272](https://github.com/numtide/horae/pull/272) | `feat/scoped-harvest-projects`, `.worktrees/scoped-harvest-projects` | #270 | Draft at `abe5aa0`; original list/count/direct-ID boundary and race tests extracted. Full acceptance pending; fresh Nixbot215 reports ARM and browser failures. Parent fixture corrections not yet propagated |
| Scoped task catalog/tracking reads, [#273](https://github.com/numtide/horae/pull/273) | `feat/scoped-task-reads`, `.worktrees/scoped-task-reads` | #272 | Draft at `5a04f89`; original readers, DB/HTTP/browser tests and sidebar interaction correction extracted. Inherited filter-count failure remains until parent correction propagates. Task lifecycle and catalog-management UI are still separate retained work |

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

Documentation ownership now applies across all mixed commits as well as the 50
documentation-only rows: final committed `specs/015-scoped-permissions/` content
is preserved in #248. All intermediate versions remain in the original refs.
Original `specs/011-new-project-screen/spec.md` permission-transition hunks also
belong to #248, composed over master's newer expense requirements. The original
constitution 1.1.0 proposal remains unadopted work in #212 and its backup ref;
the obsolete feature-selector change is retained historically, not reapplied
over feature 016. Original README migration prerequisites are restored in #222
(`516b023`), and AGENTS cache guidance is preserved exactly in #248 (`49843b2`). Unpublished
Clients documents are separate and are not silently included in #248.

| Source | Original change | Candidate responsibility | Disposition |
| --- | --- | --- | --- |
| `05448a8` | Specify scoped roles and permissions | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance; obsolete feature-selector change retained historically, not reapplied over feature 016 |
| `1d45191` | Require Harvest parity for permissions and scoped approvals | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `a7727f1` | Add record scope evaluation for permissions | scope-domain | Code/tests in #219; specification hunks retained for reconciliation |
| `2abce9a` | Enforce approval isolation and record permission boundaries | approval-isolation | Three Rust/test files and original cache patch in independent #239 (`66a256f`); 1,124 tests, source review, SQLx, format and offline server/WASM lints passed; full local Nix passed; mixed specification hunks retained |
| `757f43d` | Enforce tenant and administrator boundaries for assignments | legacy-access-writers | Rust/test/cache changes in #228 with subsequent coordination repair; specification hunks retained |
| `d3a4ff3` | Document profile reapplication and import permission boundaries | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `dcf21ef` | Specify permission migration safeguards and rate-scope verification | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `b3ee8da` | Align authorization governance with scoped permission profiles | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance; constitution 1.1.0 remains a separately preserved, unadopted proposal in original #212 |
| `b569c75` | Extend permission specification to confirmed web domains | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `f5cf02d` | Record expense scope defaults and lifecycle permission gaps | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `6ce9071` | Document expense action scope and independent lock states | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `b8b105e` | Document current-account permission research and evidence gaps | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `6443d56` | Record current Harvest permission evidence and reference conflicts | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `e8dcb77` | Add typed permission catalog and built-in profile selections | scope-domain | Code/tests in #219; specification hunks retained for reconciliation |
| `b80f8ab` | Recheck administrator authority during user access changes | legacy-access-writers | Three exact Rust blobs and regenerated cache in #227; specification hunks retained for reconciliation |
| `b7e730c` | Define permission persistence and transactional access contracts | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `412035d` | Map permission operations and transaction constraints | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `324084c` | Clarify permission boundaries for jobs and authentication | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `5fb78a7` | Specify permission preservation when deleting templates | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `f17fafa` | Clarify report-specific financial access | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `8455740` | Document resource-specific managed rate proposal | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `168d536` | Specify resource-scoped billable rate permissions | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `5bf841d` | Specify explicit cost rate permissions | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `ece40f5` | Clarify company lock scheduling and approval boundaries | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `7bcb3e2` | Require full record visibility for combined approvals | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `0fe8f56` | Document project manager assignment retention evidence | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `a1cc799` | Specify project manager retention with read access | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `9c07d60` | Clarify delegation evidence and approval history boundaries | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `ab9b1a4` | Define project-editor delegation and cross-feature permission contracts | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance; original New Project transition hunks applied over current expense requirements |
| `80e2e42` | Track person delegation and project lifecycle permission gaps | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `eb85bc1` | Specify migration preservation checks for assignment dependencies | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `b8a1ef5` | Record pending person-management policy decision | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `3cc9afe` | Restrict person-management assignment changes to administrators | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `524c29e` | Reject malformed stored permission selections | scope-domain | Code/tests in #219; specification hunks retained for reconciliation |
| `ec9408b` | Record pending person-assignment eligibility decision | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `d965718` | Require compatible grants for new person-management assignments | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `0c56dc6` | Document managed-person removal on Harvest role downgrade | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `6474382` | Confirm person-assignment removal after permission loss | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `3a19a26` | Specify explicit project-access retention choice | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `91f7cc3` | Distinguish profile selection from unchanged permission saves | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `602b04f` | Separate displayed permission profiles from assignment provenance | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `1105b75` | Record remaining permission decision gates | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `cf5d635` | Define rejection of person-management self-assignments | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `804b1d8` | Separate permission increment readiness from activation gates | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `6e61593` | Validate person-management grant compatibility and self-links | scope-domain | Code/tests in #221; specification hunks retained for reconciliation |
| `ec35446` | Store versioned permission profiles without activating new policy | permission-storage | Storage/schema/name-validator code/tests in #222; specification hunks retained for reconciliation |
| `d50c979` | Add audited permission template commands | permission-profile-transactions | Command/test/receipt/cache hunks in #226 with later hardening; specification hunks retained for reconciliation |
| `f5e0dde` | Apply permission profiles with atomic scope changes | permission-profile-transactions | Model serialization in #222; strict template receipt comparison in #226; profile commands, migration 0044 and tests/cache in #234 with later hardening; specification hunks retained |
| `9a7e05d` | Add audited project manager delegation | project-manager-delegation | Final command and all production-command tests in #243 (`3404c85`), including later hardening/composition; suite/cache/format, final WASM lint and full local Nix passed; specification hunks preserved separately |
| `fc85231` | Add administrator-only permission audit lookup | permission-audit | Complete reader/model/tests in #245 (`533922a`); suite, complete cache provenance and offline lints passed; full Nix/browser pending; specification hunks retained |
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
| `5d51b0e` | Expose the current person's permission snapshot | own-permissions | Reader, DTO, complete DB/HTTP tests and registrations in #244 (`6c4e4d1`) on #234; suite, cache, offline lints, isolated browser and full local Nix passed; specification hunks retained |
| `22ffdab` | Recheck manager access for financial snapshots | manager-snapshot-consumers | Shared helper/queries in #220; consumer/tests/cache in #232, local suite, both target lints and full Nix passed; specification hunks retained |
| `15c82ef` | Recheck manager access in invoice editor snapshots | manager-snapshot-consumers | Consumer/tests/cache in #232; fixture adaptations documented, local suite, both target lints and full Nix passed; specification hunks retained |
| `dc822a0` | Recheck manager authority before delivering exports | export-authority | Materialized XLSX/PDF source/tests and regenerated original cache in #247 (`d9717e7`); shared helper already in #220; 1,229 tests, complete cache, offline lints/format passed; full Nix pending; mixed specification hunks retained |
| `108594e` | Revalidate project scope before delivering exports | export-authority | Historical project reader and complete original DB/HTTP tests in #247 (`d9717e7`); suite/cache/lints passed, full Nix pending; later canonical changes and mixed specification hunks separately retained |
| `dcf4ac8` | Recheck current permissions during CSV downloads | export-authority | Legacy CSV cursor/batch-release source, migration 0046, original tests and regenerated cache in #249 (`71232dc`) on #247; 1,245 tests, cache provenance, format and offline lints passed; full Nix `72947` passed; later canonical stream changes and mixed specification hunks retained separately |
| `0793ce7` | Revalidate budget email authority before delivery | budget-email-authority | Four source/test files byte-identical in #238 (`7a7cede`) on master `1b8fa4f`; 1,133 tests, SQLx, format and offline server/WASM lints passed; full local Nix passed; specification hunks retained |
| `8aac739` | Add read-only permission migration diagnostics | permission-preflight | Reader and all seven tests byte-preserved in #246 (`ff482c8`) on #237/#226 review base; 1,244 tests, full cache/provenance, offline lints and unchanged-head full Nix rerun passed; initial inherited browser timeout retained; specification hunks in #248 |
| `8c15bfe` | Show own permissions in Settings | own-permissions | Complete original component, shared descriptions, SSR and resource tests in #244 (`6c4e4d1`); suite/cache, isolated browser and full local Nix passed; no full T018 claim; specification hunks retained |
| `300d1e9` | Expose administrator permission history | permission-audit | Complete historical DTO/HTTP/fencing tests in #245 (`533922a`); suite, complete cache provenance and offline lints passed; full Nix/browser pending; specification hunks retained |
| `03e90b1` | Connect permission editor previews and commands | permission-editor | Template commands/helpers in #226 and person commands/calculation in #234; final editor DTOs, session API, reader and original DB/HTTP tests in #250, with #245 strict audit dependency; source suite/cache passed, final gates pending; specification in #248 |
| `7f7fd1c` | Record permission editor delivery and UI follow-up | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `98b1692` | Add reviewed person permission editing | permission-editor | DTO runtime in #250; web-expectation removal, final editor state/export wiring and complete UI tests now preserved in #260 `98857cf`, executable gates pending; specification in #248 |
| `9e6d8bd` | Record person editor delivery and template follow-up | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `0f97cb2` | Add custom permission profile controls | permission-editor | Template DTO runtime in #250 using #226 commands; template controls and web-expectation removal now preserved in #260 `98857cf`, executable gates pending; specification in #248 |
| `8db19ba` | Show affected names in permission reviews | permission-editor | Profile command's relationship-type reuse in #234; final loss-label DTO/reader/DB/HTTP portions in #250; final page/draft/rendering tests now byte-preserved in #260; specification in #248 |
| `e7d8a36` | Protect permission drafts during navigation and dismissal | permission-editor | Final editor/recovery files in #260; navigation guard preserves current Clients. Cross-#259 union still pending; specification in #248 |
| `6bba224` | Bind permission saves to the original requester | permission-editor | Shared requester DTO already in prerequisites; final session-save binding and HTTP assertions in #250; final editor/recovery UI portions now byte-preserved in #260; specification in #248 |
| `1ecfa21` | Recover interrupted permission saves across reloads | permission-editor | Final recovery/storage/template/UI fixtures byte-preserved in #260; runner retains current suites; specification in #248; full gate pending |
| `c88ca6d` | Exercise permission recovery in a real browser | permission-editor | Final real-browser recovery fixture and runner wiring in #260; specification in #248; full gate pending |
| `202ee96` | Protect project delegation against concurrent deactivation | project-manager-delegation | Complete final command/activity tests in #243 (`3404c85`); suite/cache/format, final WASM lint and full local Nix passed; specification hunks preserved separately |
| `3f45b7c` | Validate combined approval record coverage | scope-domain | Code/tests in #221; specification hunks retained for reconciliation |
| `eb56af3` | Define scoped approval transaction and coverage gates | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `8d49421` | Add authorized permission editor subject discovery | permission-editor | Complete final subject DTO/API/reader, 305-line DB tests and HTTP assertions in #250; exact source/cache provenance and workspace suite passed; final gates pending; specification in #248 |
| `b735b3a` | Add safe person switching to permission editor | permission-editor | Final subject DTO runtime content in #250; person-switching consumer and historical web-expectation removal now byte-preserved in #260; specification in #248 |
| `7f14e4b` | Limit user directory responses to consumed fields | identity-projections | Source/tests together in #240 at `1ce993f`; 1,122 tests, regenerated cache, format and offline lints passed; full Nix running; canonical tests and mixed specification hunks separately preserved |
| `1eb13ec` | Add scoped people directory reads | people-directory | Reader, DTO, session endpoint, seven DB tests and canonical HTTP assertions extracted in draft #253; exact-head tests/Clippy/SQLx and full local Nix gate passed on 3a37538; combined integration remains pending. Specification owned by #248; legacy HTTP assertions remain with #240 |
| `981d0e3` | Resolve approval names without directory access | identity-projections | Source/tests together in #240 at `1ce993f`; 1,122 tests, regenerated cache, format and offline lints passed; full Nix running; canonical tests and mixed specification hunks separately preserved |
| `6b5dbae` | Authorize identity-only project team choices | project-team-choices | DTO, reader, endpoint, nine DB tests, HTTP tests and ten SQLx descriptors extracted unchanged in draft #254 on #253; full native Nix `81722` passed on `f498c3f`; combined browser/deployment pending; specification owned by #248 |
| `4c00660` | Document project form permission integration boundaries | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `3bb62ac` | Limit session identity responses to display fields | identity-projections | Source/tests together in #240 at `1ce993f`; 1,122 tests, regenerated cache, format and offline lints passed; full Nix running; canonical tests and mixed specification hunks separately preserved |
| `4ac30fa` | Add scoped time-entry reads without financial metadata | time-readers | Original DTO, reader, endpoint, eight DB tests and HTTP assertions in draft #255 `d93e1af` on `0117991`; 23 original SQLx descriptors preserved, module registrations adapted only; formatting/provenance and full native Nix `40092` passed; wider composition pending; specification owned by #248 |
| `c80233b` | Keep invoice identities out of time-entry responses | time-entry-payload | Exact final model and independently registered original legacy HTTP assertions in #242 (`43337fc`); 1,122 tests, complete SQLx, format, offline lints and full Nix passed; canonical fixture remainder and specification hunks separately preserved |
| `5ec183a` | Fence time-entry writes against account deactivation | time-writer-activity | Source/tests and regenerated cache in #241 (`7820f8d`), original configuration SQL inlined without canonical module; 1,142 tests, both offline lints and full Nix passed; specification hunks retained |
| `228e151` | Clarify timesheet context and locked calendar behavior | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `4294aa3` | Isolate permission browser fixtures and retain test assets | browser-fixture-tooling | Original browser.nix asset-path hunk in #259 `455c155` and equivalent independent #260; original permission recovery fixture in #260. Other shared tooling hunks still require final accounting |
| `3308926` | Keep permission profile name uniqueness independent of database locale | permission-storage | Migration/storage regressions in #222; command lookup changes remain with template commands |
| `8af562e` | Record passing permission regression gates | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `9b53182` | Verify profile capacity and confirm timesheet discovery | permission-editor, specification-history, time-readers | Original 50-contender HTTP capacity assertions retained in #250's exact final test file and passed in its workspace suite; specification/Timesheet discovery decision in #248, not a claim to deliver later Timesheet implementation |
| `60f60f9` | Add scoped Timesheet person discovery | time-readers | DTO, reader, eight DB tests, endpoint and HTTP additions extracted in draft #256 `1552fdb` on combined #255/#253 review base `40102ae`; five original SQLx descriptors; formatting/provenance, tests/Clippy/SQLx `61768` and full native Nix `37414` passed; wider composition pending. Legacy HTTP block remains owned by #242; specification owned by #248 |
| `5faed76` | Bind Timesheet page reads to requester and subject | time-readers | DTO, reader, six DB tests, endpoint and HTTP additions extracted in draft #257 `8e09e60` on #256 `1552fdb`; all queries reuse existing descriptors; formatting/provenance, tests/Clippy/SQLx `63162` and full native Nix `72055` passed; wider composition pending; specification owned by #248 |
| `e1ddd9a` | Connect Timesheet to complete scoped page reads | timesheet-consumer-commands | Original complete-page loading, drafts and refresh tests in #259 `0dfea8b`; final page/helpers copied exactly. Consumer-only DTO lint removals included. Native/full and combined verification pending; specification owned by #248 |
| `48a6533` | Define person-bound Timesheet command contracts | timesheet-consumer-commands | 67-line contracts extracted with implemented endpoints in draft #258 `d2b45ca`; no standalone stub delivery. Formatting/provenance passed, executable gates running |
| `02c4245` | Authorize person-bound Timesheet commands atomically | timesheet-consumer-commands | Whole command module,13 DB tests, HTTP assertions, implemented endpoints and39 original SQLx descriptors in draft #258 `d2b45ca`; source/format passed, `64524` running. Two DTO web-expectation removals stay with connected UI; approval-covered editing remains incomplete |
| `a0632a8` | Bind Timesheet navigation and actions to the selected person | timesheet-consumer-commands | Original navigation/person/tracking/command UI in #259 `0dfea8b`; final helpers/page exact, route and shared guards adapted to preserve current Clients/audit work. Navigation harness retains every assertion with client coverage substituted for the still-pending permission-editor branch |
| `b8b1c60` | Bind weekly submission to the active Timesheet context | timesheet-consumer-commands | Original weekly-submission context contract, page caller,124-line DB race tests,97-line HTTP fixture and five cache descriptors in #259 `0dfea8b`; obsolete descriptor replaced recoverably. Legacy-own submission boundary unchanged; specification owned by #248 |
| `8722320` | Add browsable permission change history | permission-audit | Own-reader authentication-error sanitization in #244 (`6c4e4d1`); audit reader/UI/navigation, Settings link, profile labels and shell tests extracted in `c96d787`; original editor-dependent browser assertions retained for verification reconciliation; specification hunks retained |
| `2078a13` | Format permission history verification notes | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `2f5357f` | Verify permission history and scoped Timesheet browser flows | browser-fixture-tooling, permission-audit, timesheet-consumer-commands, permission-editor | Timesheet readiness and New Project Timesheet-picker browser hunks in #259 `0dfea8b`, retaining current master Clients/project assertions. Permission-editor/history fixture remainder stays preserved for its owner; specification owned by #248 |
| `68bbaae` | Fix Timesheet modal focus and long-label layout | timesheet-consumer-commands | Original modal focus and long-label picker fixes plus modal/error browser suites in #259 `0dfea8b`. Runner preserves current suites and adds only applicable Timesheet/navigation suites; specification owned by #248 |
| `e29f4d8` | Reload Timesheet state when switching people | timesheet-consumer-commands | Final original keyed person-switch remount and browser-history assertions in #259 `0dfea8b`; full browser gate pending; specification owned by #248 |
| `5f7895c` | Preserve selected dates and drag offsets in Timesheet | timesheet-consumer-commands | Final selected-date/calendar code and assertions in #259; initially omitted original two-line pointer-target CSS restored in `4ce0919` after unchanged test exposed the dependency. Full gate `75088` pending; specification in #248 |
| `84d5352` | Expose authenticated project manager delegation | project-manager-delegation | DTOs, session wrappers, reader and tests in #243 (`3404c85`); one web-only DTO lint expectation is the recorded extraction adaptation; HTTP audit-visibility block and audit-fixture adaptation remain owned by permission-audit delivery; specification hunks retained |
| `c4e83c8` | Record project delegation verification and next integration gate | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `a25e544` | Serialize invoice writes before user revocation | legacy-access-writers | Five Rust/test changes and eight SQLx additions in #233 on integrated #220/#227/#228/#232 prerequisites; suite/cache/offline lints/full Nix passed; specification hunks retained |
| `774f60a` | Record invoice revocation verification and next integration gates | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `1879b8a` | Preserve requester identity when reloading permission editors | permission-editor | Final requester-bound editor and complete browser/UI tests byte-preserved in #260; specification in #248; full gate pending |
| `dab6885` | Record editor reload verification and remaining directory integration | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `ee16165` | Connect scoped People directory and requester-bound editing | people-directory | People consumer, original DTO web-expectation removals/import spelling, historical admin/shell/sidebar and UI fixture preserved in #260. Later task-catalog wiring stays separate; gates pending |
| `1b41033` | Record People integration verification and remaining report scope | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `75f13a1` | Add scoped detailed time report reads | time-report-readers | Reader/DTO/endpoint and detailed DB/HTTP assertions in #261 bf452dd; final reader includes totals/filter refinements, grouping separate; specification in #248; full gate pending |
| `cbc78a8` | Apply scoped permissions to time report spreadsheets | time-report-exports | XLSX reader/release scope,11 unchanged DB tests, HTTP assertions and original snapshot-test adaptation in #2638cc11c3; cache reconciled with final query predicates. Specification preserved in #248; full gate pending |
| `09bd15f` | Apply scoped permissions to streamed time exports | time-report-exports | Native stored-row decoder in #222; scoped streaming source/delivery, shared release helpers and10 DB tests/HTTP assertions in #26455d362b; specification in #248 |
| `93aaa68` | Support multi-selection filters in time downloads | time-report-exports | Shared URL parser, five unit tests and238-line two-format HTTP fixture in #26455d362b; specification in #248 |
| `a23804f` | Include full-period totals in scoped time reports | time-report-readers | One-statement totals, detailed DB/HTTP pagination assertions and exact cache in #261 bf452dd; specification in #248; full gate pending |
| `7266abb` | Connect scoped time reports with bound downloads | time-report-consumer | Backend preflight/DTOs, CSV/XLSX mode binding and parser/HTTP tests in #265f436a29; UI, browser/component fixtures and consumer lint removals in #268810ce57; specification in #248 |
| `41ff137` | Add scoped time report grouping with exact totals | time-report-readers | Grouped reader/contracts,13 final DB tests and original171-line HTTP fixture in #26266dbf0b, full gate79456 passed; export_filters helper additions in #2660eec1a4; UI retained separately; specification in #248 |
| `e2e66fb` | Connect scoped time report groups and detail navigation | time-report-consumer | Grouped consumer and original browser/component assertions in #268810ce57; specification in #248 |
| `ca170c0` | Export scoped time groups to Excel with release authorization | time-report-exports | Grouped XLSX reader/renderer/route and seven DB tests in #2660eec1a4; UI/browser/component assertions retained for Reports consumer; specification in #248 |
| `2b59b58` | Stream grouped time reports with scoped authorization | time-report-exports | Grouped CSV cursor/delivery/route, nine DB tests and shared group-lifetime authorization in #2660eec1a4; UI/browser assertions retained for Reports consumer; specification in #248 |
| `ecac66b` | Add scoped individual time reports and nested breakdowns | time-report-consumer | Three original scoped modules, individual/nested transitions and original browser/component assertions in #268810ce57; specification in #248 |
| `de8f9ad` | Filter time reports to active projects | time-report-consumer | DTO/readers and reader tests in #261/#262; export SQL predicates in #263/#264/#266; strict URL transport and original cross-format snapshot/authority fixtures in #267e029a89. UI/browser/component hunks retained for Reports consumer; specification in #248 |
| `2497dbe` | Enforce scoped permissions in the project editor | project-editor-permissions | Pure RateEdit code/tests in #221; composable delegation and transaction tests in #243; picker reader in #254. Canonical editor DTOs, field/association/save logic, UI, DB/HTTP/browser tests and consumer lint adaptations extracted in #269. Full acceptance remains pending; later task lifecycle changes are not included. Specifications owned by #248 |
| `2631186` | Enforce scoped project reads across pages and exports | project-read-permissions | Ordinary readers, budgets, minimal labels and bound overview/detail UI in #270; CSV/XLSX release boundaries in #271; Harvest-compatible project list/count/direct-ID readers in #272. Shared export helpers compose over earlier export PRs. Tests and original descriptors retained; fixture adaptations and failing gates recorded below. Final shared-file hunk audit and complete acceptance remain pending; specifications owned by #248 |
| `1b81680` | Record project permission delivery acceptance | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `f6e8bf1` | Enforce task catalog and tracking read permissions | task-permissions-lifecycle | Task catalog/tracking and Harvest-compatible readers, DB/HTTP/browser fixtures, original descriptors and sidebar interaction correction extracted in #273. Full acceptance remains pending; later task creation, editing, activity and linking commands are still retained separately. Specifications owned by #248 |
| `8dd61d4` | Enforce current task creation and project scope permissions | task-permissions-lifecycle | Held in original backup; extraction pending |
| `facfb49` | Protect task rate edits with explicit intent and current permissions | task-permissions-lifecycle | Held in original backup; extraction pending |
| `ac4c90c` | Authorize task activity changes and guard running timers | task-permissions-lifecycle | Held in original backup; extraction pending |
| `0591407` | Preserve project task archival across restores and imports | task-permissions-lifecycle | Held in original backup; extraction pending |
| `979a594` | Enforce scoped project task linking and rate currency | task-permissions-lifecycle | Held in original backup; extraction pending |
| `5561f14` | Add permission-aware task catalog management | task-consumers | Held in original backup; extraction pending |
| `dcadcee` | Add atomic task creation to the task catalog | task-consumers | Held in original backup; extraction pending |
| `8c1bf9b` | Add task archive and restore controls to project editing | task-consumers | Held in original backup; extraction pending |
| `db3935d` | Filter time reports and downloads by billability | time-report-consumer | DTO/readers and reader tests in #261/#262; export SQL predicates in #263/#264/#266; grouped HTTP route registrations in #266; strict URL transport and original cross-format DB/HTTP fixtures in #267e029a89. UI/browser/component hunks retained for Reports consumer; specification in #248 |

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

### Final foundation checks confirmed; independent history browser test published

Re-read the active separation objective and confirmed #216 is merged at
`02f7b58acdcf126415f9ec89215da8cdada7d03f`. The preceding conversational turn
only supplied a proposed goal prompt and made no repository progress. This
iteration resumes the actual attached objective, not that proposed replacement.

Old terminal handles for #243/#244 were already closed. Re-evaluated their
unchanged clean final heads through `nix flake check -L`: `10972` (#243,
`3404c85`) and `30710` (#244, `6c4e4d1`) exited zero, all x86_64-linux outputs
already cached, zero checks rebuilt. This confirms final native/browser/SQLx/VM
gates and closes #243's last native-lint uncertainty. No cross-platform build
claim is made. Their published PR descriptions now record these results.

#245 offline server/WASM lint `72147` exited zero on `533922a`. Nix `61137`
remains live; SQLx, Clippy and Rust test build phases completed, but the package
was still unavailable at the focused browser launch attempt. That attempt
stopped at `test -x`, before creating any database or launching a browser.
Do not restart this build on observation timeout or count it as the complete
gate for the subsequent test-only head.

Published `9744184` and `065a96b` on the existing draft #245. They add only
`permission-history.cjs` and its registration, retaining every previous runner
suite. The disposable fixture uses the exact compiled Dioxus endpoint string
(the pinned macro hashes implicit routes), current-session HTTP commands,
complete project grant prerequisites and no fabricated receipt JSON. Its 27
changes plus one no-op drive empty state, exact 25/3 paging, native keyboard
disclosure, narrow/light and desktop/dark layout, stale-history suppression,
revocation/recovery, canonical-admin navigation despite legacy Member role and
deactivation. Fixture cleanup restores only the isolated seeded organization.
The original editor-dependent assertions, including deleted custom profiles,
remain owned by the later editor integration; they are not waived or replaced.

Syntax checks `23309` and `75082` passed; formatter `97076` passed with zero
changes before the final fixture-only prerequisite correction. Runtime and
final-head gates remain pending, explicitly reflected in the PR body. No Rust,
schema, SQLx descriptor, CSS or product behavior changed. Source preservation
check against `backup/pr212-split-20261006-uncommitted` still passed.

#240 Nix `16586` was confirmed live. Read-only process inspection identified
its active Node child `3237506` running the immutable store copy of
`action-errors.cjs` for more than 85 minutes, with Chromium alive. Its Nix log
was empty. That suite has event/response waits without explicit deadlines;
the particular blocked wait and root cause are not yet established. No process
was killed or restarted. Next diagnose with a bounded isolated reproduction
if needed, without treating a stall as either a pass or a product regression.

Next collect `61137`; once its exact package is available, run the focused
history suite against that unchanged production code, inspect both captures,
resolve any proven failure, and verify the final test-inclusive head. Continue
remaining canonical consumer/editor and specification ownership afterward.
This iteration is PROGRESS. No merges, original closures, real-data changes or
policy activation occurred; the overall goal remains incomplete.

### History browser verified; bounded action tests and preflight extraction

The previous goal iteration was PROGRESS (published browser verification and
updated authoritative check evidence). Re-read the attached objective; no scope
or completion criteria changed.

#245's built `533922a` package became available. Focused browser `56883`
failed with `Route is already handled!`: the test released a held request and
removed its interceptor before the handler finished. Preserved this failure;
the application had not been shown faulty. Test-only `93f230d` moves interceptor
removal after the released response renders. Complete focused run `36324`
then exited zero in Chromium 148.0.7778.96. All real-writer, empty/no-op, exact
25/3 paging, keyboard, pending suppression, revoke/recover, canonical navigation
and deactivation assertions passed. Inspected the desktop-dark and narrow-light
captures in `.scratch/browser-history-coordinated/`: no page overflow or overlap
observed. No native text-zoom, touch, contrast or cross-browser certification.
Format `79897` passed without changes. Final-head full Nix `9966` is running
on `93f230d`; old `61137` on `533922a` continues but cannot substitute for it.

#240's original Nix remained live in `action-errors.cjs`. An isolated diagnostic
(`4807`) kept every assertion but set finite waits and logged request paths;
it passed, so the original blocked wait/root cause is still unproven. Committed
and published only `page.setDefaultTimeout(30_000)` as `d2da193`. This bounds
previously infinite event waits rather than increasing assertion timeouts.
Exact uninstrumented focused run `47505` passed all scenarios on the unchanged
built application; format `50726` passed. Explicitly interrupted the superseded
Nix PID `3032398`; handle `16586` exited 1 with interruption, not a green result.
Final-head full Nix `79480` is running. PR descriptions retain the inconclusive
history, bounded reproduction and pending final gates.

Scoped Spec Kit analyze ran feature-015 prerequisites successfully with no
extension hooks. Preflight coverage is five requirements (FR-006/010/014/017/018)
and three mapped tasks (T117–T119), 100% scoped requirement coverage, no unmapped
tasks, ambiguity, duplication or critical/high conflict in this diagnostic
boundary. US5/SC-004 and T019 migration/activation remain incomplete; the proposed
constitution amendment is not adopted. No specification was regenerated.

Created isolated `feat/permission-preflight`. Source reader (91 lines) and full
tests (451 lines, seven cases) match original `db3935d` blobs exactly:
`ed8377fbd45dae76791c766ac14574b3f298220c` and
`73d354d1bbbbfdae86ecb823ee329736581dee0b`. The only other change is module/test
registration. Review covers current legacy Administrator before counts, policy
zero, organization-then-actor locks, post-wait checks, one-statement snapshot,
count-only tenant diagnostics, cancellation/lock failure and before/after full
stored-value comparisons. Zero counts confer no activation readiness. No public
endpoint, new schema, CLI, UI, product behavior or real-data inspection is added.

Initial source `a88e849` used #237 alone. Compile-time SQLx in `26167` caught a
missing prerequisite: the preservation test also snapshots
`permission_change_receipts`, whose migration is in #226. Did not delete or
conditionalize that assertion. Explicitly stopped the already-failed Cargo PID
`3811415`; wrapper exited 101 after its interrupted child. No passing suite or
cache was claimed from that attempt.

Published review-only base `integration/permission-preflight-prerequisites`
at `61c90bc`, combining exact #237 `2242361` and #226 `82d15f3`. Two composition
conflicts retained #226's strict-storage superset and server-only template DTO
registration; no #237 behavior was discarded. Rebased only the new extraction
onto that base as `b159184`, preserving both preflight blobs and both test/module
registrations, 3 files / 547 added lines. Updated only that new remote branch
using an exact lease against `a88e849`; original branches/backups remain intact.
The integration branch is not a delivery merge target. Both prerequisite chains
must reach master before retargeting and repeating final-head gates there.

Corrected full disposable suite and complete SQLx preparation are running in
`28480`; the reusable local Cargo target is occupied. All worktree Rust-source
timestamps were invalidated before preparation to avoid stale cached-target
omissions. Original unpublished tracked work still exactly matches its saved
snapshot. Next collect `28480`, prove complete cache provenance, run offline
native/WASM and full final-head Nix, and publish the scoped draft. Also collect
`9966`/#245 and `79480`/#240 without restarting live handles. Continue canonical
consumer/editor and specification ownership after these deliveries. No GitHub
merges, original closures, real-data writes or policy activation occurred.
This iteration is PROGRESS; the overall goal remains incomplete.

### Legacy export delivery boundaries inventoried

Re-read the active objective and confirmed the previous iteration was PROGRESS.
Existing verification handles `28480`, `9966`, `79480` and `61137` remained live;
the disposable Rust compiler was consuming CPU, not merely leaving a stale lock.
No build was restarted. Prepared the preflight draft description locally;
publication awaits its complete cache proof so no known-broken offline head is
submitted as a ready delivery. The corrected preflight base has 1,119 SQLx
descriptors; original `8aac739` added 17, and neither preserved preflight source
file has any later original commit changing it.

Inventoried the remaining legacy export commits before the canonical consumers:

- `dc822a0` and `108594e` form one materialized XLSX/PDF boundary: bounded
  materialization, rendering/release authorization, financial/project HTTP guards
  and retained project-resource scope. Include invoice PDF, not spreadsheets alone.
  Reconcile the original snapshot visibility changes with #220's existing
  `pub(crate)` manager helper rather than extracting that shared change twice.
  Reuse the already-recorded prerequisite composition `0046dad` as a candidate
  review base rather than creating redundant shared foundations. Its history
  contains #228 directly and equivalents `41e595d`/`0046dad` for #220/#232;
  those are not the original PR commit IDs, so ancestry alone is not proof of
  equivalence. Check the selected source diff before extraction.
- `dcf4ac8` is a dependent CSV boundary: connection-local cursor, bounded batches,
  release checks and migration 0046's `SECURITY INVOKER` fetch function. Its
  public execution privilege is revoked; separate deployment migration owners
  must explicitly grant the runtime role. Preserve that existing deployment
  contract, and do not introduce real-database migration or grants in this goal.
- The authorization-test files were later changed by canonical commits
  `cbc78a8`, `09bd15f` and `2631186`. Do not copy their final versions wholesale
  into a legacy-only extraction. Preserve both the historical legacy assertions
  and the separately owned canonical additions; account for both in final mapping.

This is dependency/ownership inventory, not a completed source review or a claim
that the export extractions are implemented. Next collect the ongoing preflight
verification/cache, then prepare the first legacy export extraction while the
final Nix gates finish. Original work, data and PR merge state remain unchanged.

### Preflight suite/cache verified and draft published

Corrected verification `28480` exited zero on source `b159184`: 904 app,
180 integration and 160 core tests passed, 1,244 total; 11 existing manual tests
ignored. Complete SQLx preparation then finished successfully. Provenance check
`2054` confirmed all 1,119 base descriptors unchanged, 19 additions identical to
original `db3935d`, 1,138 total, zero removals/modifications/unmatched additions.

Original `8aac739` added 17 descriptors, one already present in this review base.
The three other regenerated descriptors are original queries used by this exact
reader/test slice: inherited repeatable-read fixture setting, fixture policy-zero
update, and the policy-version organization SHARE read. Their original byte
matches were verified; no unrelated implementation is introduced with them.

Published unsigned cache head `ff482c847ab8d14cda15c8a3a56deafbf6acb4a0` and
opened draft [#246](https://github.com/numtide/horae/pull/246). It has 22 files,
947 additions: 547 source/test registration lines plus 400 SQLx lines. The actual
reader remains the unchanged 91-line original. Offline native/WASM lint `86929`
and full clean-head Nix `99711` are live; neither is claimed passed yet. The local
Cargo target is occupied by that lint run.

Old #245 source/cache-head Nix `61137` exited zero on `533922a`, all local
x86_64-linux checks passed, including the inherited browser suite. Its later
test-inclusive head `93f230d` still requires `9966`, which remains live; old
results are not substituted. #240 final-head Nix `79480` also remains live.
PR #245's body records the completed older gate and pending current one.

Reading the retained export contracts clarified two inventory details: the
materialized boundary includes invoice PDF as well as XLSX, and #220 already
delivered the manager helper's `pub(crate)` visibility. The snapshot module
itself remains private in candidate base `0046dad`; account for that remaining
visibility hunk without duplicating the helper change. Use the historical
`dcf4ac8` CSV contract for the legacy extraction, not later canonical refinements
in the final original contract. No export source was changed in this iteration.

Next collect the exact live final-head gates, then extract the materialized
XLSX/PDF boundary with the preserved legacy tests and current master behavior.
The original branches, unpublished work, real data and merge state are intact.
This iteration is PROGRESS; full original ownership and the overall goal remain
incomplete.

### Materialized export extraction and schema dependency correction

Re-read the active objective after the intervening goal-prompt conversation
(that conversation alone made no repository progress). GitHub confirms #216
merged as `02f7b58`. Revalidated live final-head Nix handles `99711`, `9966` and
`79480`; no run was restarted. #240 and #245 have now built their final packages
and reached real browser tests; the full gates remain pending. Recorded the
previously completed offline native/WASM lint `86929` as passed for #246 and
updated its PR description. Its handle is now closed, not an active wait.

Created isolated `fix/materialized-export-authority` from existing review base
`0046dad`, extracting original `dc822a0` and `108594e` together. This is one
materialized-download responsibility, including invoice PDF. Seven complete
files match historical `108594e` byte-for-byte: `reports/limits.rs`, its project
reader and two authorization-test files, `reports/privacy_tests.rs`, and the
two HTTP export-test files. Their later canonical changes remain separately
owned. `reports.rs` retains #220's removal of the old global invoice loader;
its only difference from the historical file is that already-reviewed base
change. The remaining adaptations are export HTTP registrations and snapshot
module visibility. The manager helper was already crate-visible in #220.

Verified #220's snapshot source/test directory is identical to composition
`41e595d`; #232's invoice reader/editor, fee tests, snapshot tests and HTTP
financial checks are identical in `0046dad`. Differences elsewhere are the
known #227/#228 writer coordination, including its invoice/assignment lock test.
The recorded master update changes neither crates nor Cargo manifests/lockfile.

Unsigned initial source `c216440` passed formatting (`17075`, zero changes),
but disposable verification `93167` exited 101: three original test queries
require `organizations.access_revision`, absent from `0046dad`. No test ran or
cache succeeded in that attempt. Do not remove those assertions or substitute
an unrelated organization edit. They verify revision refresh and unchanged
revision across real finalization/editor writes.

Created review-only `integration/materialized-export-prerequisites` at
`3edc0b8be31ffc08acd9a17cfb3022da6b0a62fb`, merging exact #222 `e9695fd`
into `0046dad` without conflicts. This supplies existing non-activating storage
and its pure-domain prerequisite; no new migration or activation is invented.
This local composition is not a delivery merge target. Rebased only the new
extraction using `--no-update-refs` to `abdda6077fc35b76178a7d40bdae4facac9dbaea`.
The extracted readers, tests and HTTP harness remain unchanged by the rebase.
The source delta is 10 files, 2,237 additions and 102 removals; 1,935 added lines
are the four preserved authorization/HTTP test files. Complete suite and SQLx
preparation are now running in `20984` with a private PostgreSQL socket and
invalidated Rust-source timestamps. The reusable local Cargo target is occupied.

Scoped Spec Kit analyze ran the real prerequisite command against original 015,
with no extension hooks. Read the retained spec/plan/tasks plus historical
manager/project export contracts. Coverage within T104–T109:

| Requirement | Tasks | Evidence boundary |
| --- | --- | --- |
| FR-006 | T104/T105/T107/T108 | Current active tenant actor; unchanged legacy manager/project scope |
| FR-007 | T105/T106/T108/T109 | All four materialized HTTP handlers; CSV remains separate |
| FR-010 | T104/T105/T107/T108 | Fresh checks after authority/parent waits and rendering |
| FR-017 | T104/T105/T107/T108 | Exact existing values, no business writes or historical recalculation |
| FR-018 | T104/T106/T107/T109 | Real reader, races, cleanup, size and session-cookie checks |

Five requirements and six tasks mapped (100% bounded coverage); zero unmapped
tasks, ambiguities, duplications or critical/high specification findings in this
slice. SC-006 is a regression subset, not full acceptance. Full canonical US3,
T039/T040/T042, CSV, activation and governance remain open. No original artifacts
were edited or their checkboxes treated as proof of this extraction's tests.

Source review read both complete database authorization-test files and both HTTP
test files. Checked session-derived IDs, pre-query manager authorization,
same-snapshot size/payload, one-statement project materialization, nullable
sentinel handling, private captured IDs, sorted parent locks followed by a fresh
visibility query, body/permit drop on denial, and no DB locks during rendering
or client backpressure. Existing CSV transaction configuration remains unchanged
in behavior. No critical/high source finding identified; executable verification
and complete cache/offline/full-Nix gates are still mandatory and pending.

Next collect `20984`, prove SQLx provenance (accounting for legitimately replaced
legacy descriptors), then run offline native/WASM and final-head Nix before
publishing the scoped draft. Collect `99711`/#246, `9966`/#245 and `79480`/#240
without restarting live handles. The original tracked unpublished work still
exactly matches its backup snapshot; original branches, #208 and real data are
unchanged. This iteration is PROGRESS; the overall goal remains incomplete.

### Materialized exports published; final browser failures corrected

The previous iteration was PROGRESS. Re-read the objective and polled the four
specific live handles; no process was restarted because of an observation timeout.
Corrected XLSX/PDF suite/cache `20984` exited zero: 889 app, 180 integration and
160 core tests, 1,229 passed total with 11 existing manual tests ignored. The
complete SQLx preparation succeeded after all targets were invalidated.

Read-only provenance check `23074` confirmed 1,088 base descriptors: 1,086
unchanged, two removed, zero modified; 49 additions match historical `108594e`
exactly, 1,135 total, zero unmatched. The two removed descriptors are the old
separate project size query (`717614…`) and the project-row projection without
private IDs (`57eb29…`); the single bounded materialization and ID-bearing
projection replace them. No unrelated query was discarded.

Unsigned cache head `d9717e7ed9e9982e01668ee611a9e7aef950c358` is clean and
pushed with review base `3edc0b8`. Opened draft
[#247](https://github.com/numtide/horae/pull/247). Formatting `48059` passed
unchanged. Offline native/all-target and WASM lint `49167` passed on that exact
head, warnings and performance lints denied. Full Nix `32625` is live. The PR
records successful source/cache/lint checks separately from its pending full
gate. No GitHub merge or policy activation occurred.

Two previously live final-head Nix gates exited 1; neither is certified green:

- #240 `79480` on `d2da193` failed in inherited `clients-access.cjs` at its
  `actor.active === true` assertion. `get_me` intentionally no longer returns
  activity. Test-only `a19ea63857dd149a64fc3519a19b3cab7a0489b8` now asserts the
  exact five public identity keys and verifies current activity from the
  disposable actor/tenant row. This strengthens the payload check without
  deleting any role/scope/inactive/anonymous assertion. Focused Chromium run
  `59611` passed all four role/scope scenarios and negative write/session cases
  using the exact failed-head package. Formatting `51138` passed. The fix is
  pushed; full Nix `99785` is running on the new head. The older unbounded
  action-errors interruption remains recorded separately.
- #245 `9966` on `93f230d` failed at the empty-history assertion. Its ARIA
  snapshot showed `Other draft owner` authenticated, while the fixture had
  installed canonical state for `admin@example.com`. The preceding New Project
  permission suite creates the additional Administrator; dev login does not
  promise the seed account. Test-only `14ad9ca` obtains the actual session's
  `get_me` endpoint from the exact built server, validates its IDs, legacy
  Administrator role and DB activity, and creates/cleans that actor's fixture
  only. All history assertions and zero-state preconditions remain. Combined
  browser run `84853` passed `new-project-permissions permission-history` against
  the failed-head package; this exercises the additional-admin prerequisite,
  not just an isolated seed login. Formatting `33741` passed. The fix is pushed;
  full Nix `56923` is live on its new head. Earlier isolated-browser and old-head
  Nix passes do not substitute for this pending complete gate.

#246 final Nix `99711` remains live and has reached browser/deployment tests;
do not restart it. #247's local Cargo lint has finished, so the shared target is
free for the next extraction, independently of the Nix sandbox builds.

Documentation ownership inspection identified exactly 59 changed non-cache
Markdown/JSON paths in original #212: 54 new feature-015 documents, the feature
selector, constitution, New Project spec, AGENTS and README. Feature 015 is not
present in recorded master. Keep these as existing artifacts, not a new specify
exercise. Three histories own the governance/selection/New Project deltas:
`05448a8`, `b3ee8da` and `ab9b1a4`. The constitution remains 1.0.0 on master;
original 1.1.0 is preserved proposal material, not adopted by these extractions.

Master's feature selector now points to 016, and its New Project spec has newer
expense-budget/currency acceptance (including FR-026). A whole-file replacement
from #212 would erase that work. Carry the original permission-transition hunks
onto the newer spec without changing those expense requirements; preserve the
old selector as historical work rather than resetting the current feature.
AGENTS' original SQLx timestamp guidance belongs to cache-preparation hygiene;
README's ICU/name-collision prerequisite belongs to #222's migration deployment
documentation. These paths are inventoried, not yet extracted/reviewed deliveries.
Original progress/quickstart are historical evidence, never proof that an
extracted head passed. Unpublished Clients documents/code remain separate.

Next collect final-head `99785`, `56923`, `99711` and `32625`, and continue the
dependent legacy CSV boundary with migration 0046. Remaining canonical
consumers/editor and documentation ownership are still required. The goal is
incomplete; this iteration is PROGRESS, with no original closure or real-data write.

### Legacy CSV boundary extracted on materialized exports

The intervening prompt-only response did not advance repository state (NO
PROGRESS). Re-read the actual saved objective, AGENTS and constitution 1.0.0,
verified #216 is merged as `02f7b58`, inspected current worktrees/ledger, and
resumed the next safe extraction. Specific final-head Nix handles `99785`,
`56923`, `99711` and `32625` were confirmed live; none was restarted. #246 is
still in its browser matrix, the other three are compiling. These are pending,
not passing gates.

Created `fix/csv-export-authority` in `.worktrees/csv-export-authority` directly
from #247 `d9717e7ed9e9982e01668ee611a9e7aef950c358`. Unsigned source commit
`9f2994d` extracts historical `dcf4ac8`: 13 files, 1,571 additions and 113
deletions, including the complete 815-line database authorization module and
68-line session-authenticated project CSV module. Eleven files match that
historical source byte-for-byte. `reports.rs` additionally preserves #220's
removal of the old global invoice loader; the HTTP harness gains only the nine
CSV route registration lines, retaining its existing legacy-reader and importer
boundaries. Later canonical time/project/grouped export changes are not pulled
into this slice. No dependency, policy activation, UI or business-state write
was added.

Reviewed all production cursor/delivery paths, migration 0046, the complete new
authorization tests, HTTP CSV tests and changed invoice snapshot fixture.
Preserved one reserved close-on-drop connection, explicit READ COMMITTED READ
WRITE, initial authorization before cursor declaration, and a frozen source
snapshot. Each nonempty block reserves output capacity before fresh
organization/actor checks; project blocks sort/deduplicate captured parent IDs
and query access separately after parent-lock waits. Successful rollback and
savepoint release precede synchronous send with no intervening await. A later
denial propagates as body failure rather than successful truncated EOF.

Native input limits remain 1 initial row, then at most 128 rows or the
64 KiB logical-payload crossing row. Output independently flushes first/128/64
KiB records; one oversized record is intentionally allowed, not claimed as a
whole-process memory ceiling. The invoker-only fixed-cursor helper keeps PUBLIC
execution revoked; separate migration owners must explicitly grant their
runtime role. The invoice LEFT JOIN sentinel preserves empty/missing distinction,
nullable fee quantities, stored integer totals and original filename/metadata
across source changes. Error, timeout and cancellation preserve admission and
connection cleanup. No critical/high source finding identified; executable
verification remains pending.

Applied the Rust, async, testing and minimal-change skills. Formatting `10571`
passed with zero changed files; `git diff --check` passed. Suite and complete
SQLx preparation `47215` are running against the helper's fresh private
PostgreSQL cluster, with no TCP listener or real data. The shared local Cargo
target is occupied until that handle finishes; do not launch competing local
Cargo work.

Scoped Spec Kit analyze ran the original feature-015 prerequisite script
(`--json --require-tasks --include-tasks`); required artifacts exist and there
are no extension hooks. Read the relevant current spec/plan/tasks and both
historical and evolved CSV contracts. This extraction uses the T110–T113
legacy boundary only; canonical refinements remain separate original work.

| Requirement | Tasks | Bounded coverage |
| --- | --- | --- |
| FR-006 | T110/T111/T112 | Tenant, active actor and captured-project access |
| FR-007 | T110/T111/T112 | All three CSV source and delivery paths |
| FR-010 | T110/T111/T112 | Fresh post-wait/block checks and revocation |
| FR-017 | T111/T112 | Frozen source, integer values, no business mutation |
| FR-018 | T110/T112/T113 | Negative/race/native/HTTP and regression gates |

Five requirements and four tasks mapped, 100% bounded task coverage, zero
unmapped tasks, ambiguities, duplications or critical/high specification
findings in this slice. SC-006 is a regression subset, not full acceptance.
The unchanged constitution 1.0.0 remains authoritative; the original 1.1.0
proposal and full US3/canonical activation gates are not adopted or completed.
No remediation or original artifact edit was needed by this scoped analysis.

Next collect `47215`, then run `.scratch/verify-cache.mjs` in the CSV worktree
to prove the regenerated cache retains every base descriptor except the two
replaced invoice snapshot-fixture UPDATE queries (`548235…`, `560550…`) and
matches historical `dcf4ac8` for additions. The verifier is prepared but has not
run; expected removals are a hypothesis until verified. Commit the proven
cache, run offline native/WASM and final-head Nix, then publish a scoped draft
with exact evidence and its dependency on #247. Collect the four existing Nix
handles at reasonable intervals. Documentation ownership, canonical consumers,
editor/UI and unpublished Clients remain required. This iteration is PROGRESS;
the overall goal remains incomplete.

### Specification preservation published; preflight browser timeout isolated

The previous goal turn was PROGRESS: source CSV extraction, provenance review,
scoped analysis and the ledger were committed. Re-read the saved objective and
confirmed `47215`, `99785`, `56923`, `99711` and `32625` live. CSV `47215` has
finished compilation and is executing its 916-test app binary; no final suite
or cache result is claimed yet. The shared local Cargo target remains occupied.

Created documentation-only worktree `.worktrees/permission-specification`,
branch `docs/permission-specification`, on master `1b8fa4f`. Preservation commit
`c47199296c755be19023e1c2de8a38c0d148ebe3` contains all 54 original committed
feature-015 documents byte-for-byte from `db3935d` (21,814 existing lines,
including historical research and verification records). No original worktree
or dirty Clients file was copied over or cleared.

Read the Spec Kit analyze instructions and ran the feature-015 prerequisite
script in its original worktree, with no hooks or feature-selector changes.
The extracted committed artifacts, not that worktree's newer unpublished
Clients edits, are the documentation source. Bounded extraction review found
two contextual risks: the absent proposed constitution could be mistaken for
the authoritative version, and historical task/test statements could be mistaken
for current-head verification. These are documentation-provenance findings, not
new product decisions or a claim of full-feature analysis completion.

Follow-up `c77abf9edd5ba7a924ce934b114961af920a5fa4` clarifies the spec, plan,
tasks, research, progress and quickstart. It leaves the full requirement and
task statements unchanged, explicitly keeps constitution 1.0.0 authoritative,
and identifies #218 as the only current separation record. Original logs remain
historical; process IDs in them are not live handles or new next actions.
Applied only the original New Project permission-transition hunks on top of
master's file: new expense inclusion, expense/time currency distinctions and
FR-026 remain intact. No code, schema, cache, design, README, AGENTS or selector
change is included.

Read-only verifier `27861` passed: all 54 preservation blobs match, 48 documents
are still unchanged, six have contextual notes, all 43 functional requirements
and success criteria are identical, and all 236 unique task lines retain text
and state (208 checked historically, 28 open). The exact three original New
Project diff hunks compose over current master, with no other edit. There are
55 changed Markdown paths and zero application/governance changes. Formatting
`56406` passed unchanged; diff whitespace checks passed. Published draft
[#248](https://github.com/numtide/horae/pull/248), confirmed by `54903` exit zero.
Full flake/remote checks and final cross-PR reconciliation remain pending. No
fresh Harvest verification or completed permission implementation is claimed.

All 50 documentation-only commit rows now name #248 for their final feature
document state. The general ownership rule also covers specification hunks in
mixed commits. Original history and discarded intermediate wording remain
recoverable in the verified refs/bundle. Governance/selector exceptions and
the still-unpublished Clients documents are explicit, not silently omitted.

#246 full Nix `99711` exited 1 on `ff482c8`. Its complete browser log at
`/nix/store/2xkgr1s2wbiqpqsfcxlyr50lnc2kz992-horae-browser-checks.drv` shows the
preceding suites passing, then inherited `project-edit.cjs` timing out after
five seconds while reloading the saved project at 390px. The ARIA snapshot is
`Loading project…`, not a denied or wrong-value result. This observation does
not establish the timeout's cause.

Read the full editor test and disposable runner. Unchanged focused sequence
`new-project-transport project-edit`, handle `97035`, passed on the exact failed
package `/nix/store/y6alrw4vmcill044y7r0vj2bz18niw7s-horae-0.1.0/bin/horae`.
It verified 390/768/1440px edit/save/reload, original history and identity,
configured monetary values, dirty-navigation protection, acknowledgement-loss
replay and concurrent-edit recovery. No source, assertion or timeout changed.
Started one complete unchanged-head rerun `41382`; the original failure stays
recorded, its root cause remains unproven, and no full pass is claimed. #246's
PR body now records both results and the pending rerun.

Next collect CSV `47215` and complete its prepared cache-provenance/offline/full
gates before publishing. Collect #246 rerun `41382` and existing final-head
Nix `99785`/#240, `56923`/#245 and `32625`/#247 without restarts. Finish #248's
required checks and reconcile remaining README/AGENTS/governance ownership.
Canonical consumers/editor/UI and unpublished Clients still need disposition.
This iteration is PROGRESS; no merge, original closure or real-data change.

CSV `47215` subsequently completed its workspace suite successfully on source
`9f2994d`: 905 app, 180 integration and 160 core tests passed (1,245 total),
with the same 11 existing manual tests ignored. Complete SQLx preparation is
now running in that same handle/private database. No failure or assertion
weakening occurred. Do not start local offline lint or validate a partial cache
until preparation exits. Ledger formatting `49784` passed unchanged; #246 body
update `78194` exited zero.

### CSV delivery published; deployment and cache documentation accounted for

The immediately preceding response supplied a goal prompt, not repository
progress. Re-read the saved active objective, AGENTS and constitution before
continuing; GitHub reconfirmed #216 merged at `02f7b58`. Revalidated the existing
verification handles rather than restarting them. This iteration is PROGRESS.

CSV preparation `47215` exited successfully after its 1,245-test suite. The
complete cache-provenance check `57325` passed: 1,135 base descriptors, 1,133
unchanged, exactly two obsolete invoice-fixture UPDATE descriptors removed,
41 additions matching historical `dcf4ac8`, zero modified or unmatched entries,
1,174 total. Formatting and unsigned cache commit `1633` completed successfully
as `71232dc4257a0c1831ee9e7005a6a2c1562516ed`; the worktree is clean.

Final-head offline native/all-target and WASM Clippy `68186` both passed with
warnings denied and performance lints enabled. Published draft
[#249](https://github.com/numtide/horae/pull/249), confirmed by `76441` exit zero,
against exact #247 `d9717e7`. Its source/review boundaries and pending checks are
explicit. Full Nix `72947` remains live on `71232dc`; it is not a completed gate.
PR body update `55002` records the completed offline checks. The shared local
Cargo target is free again; the Nix checks use isolated build directories.

Read the README skill, entire #222 README and original migration 0047, and
restored only the original six-line PostgreSQL/ICU/name-collision deployment
note. Commit `516b023780eea04d7b1bf73895c5b325b299c872` is pushed to #222.
`git diff --quiet db3935d HEAD -- README.md` passes: the complete README now
matches the original source. Formatting and whitespace checks passed.
Final-head full Nix `69471` passed all compatible x86_64-linux checks; unchanged
code derivations reused their verified outputs and the new formatting derivation
passed. No migration, runtime code, source worktree or descendant branch changed.
Existing descendants still record their exact `e9695fd` prerequisite; integrate
the documentation follow-up when retargeting, not via needless stack rewrites.

#248 follow-up `49843b240c0179a5142c20fdac7f4e74a8bc323e` preserves the original
seven-line AGENTS cache-preparation guidance. Its whole AGENTS file matches
`db3935d`, verified alongside the prior 54-document/43-requirement/236-task and
three-New-Project-hunk assertions. The extraction now has 56 Markdown paths.
Formatting passed with zero changes, the commit and updated PR body are pushed
(`30390` exit zero). README belongs to #222; constitution 1.1.0 remains explicitly
unadopted and the obsolete selector remains historical. Final-head full Nix
`51945` is running. No application behavior or product decision changed.

#246 complete unchanged-head Nix rerun `41382` exited zero on `ff482c8`, including
the full browser suite. The initial `99711` editor-loading timeout and unknown
root cause remain recorded; no source, assertion or timeout was changed.
PR update `99428` records both outcomes. Prerequisite integration, retargeting
and required remote checks remain separate gates, not implied by this local pass.

#240 `99785`, #245 `56923` and #247 `32625` are confirmed live; their app builds
finished and browser/NixOS checks are progressing. Do not restart these handles
or treat the completed app builds as complete flake checks.

Next collect those three handles plus #249 `72947` and #248 `51945` at reasonable
intervals, recording failures as well as passes. Resolve the remaining canonical
consumer/editor/UI ownership and original test-tooling overlaps from the actual
source dependencies; do not implement unfinished consumers to ease extraction.
Unpublished Clients remains preserved separately, not delivered. Final per-change
accounting, cross-PR integration/review and delivery order are still required.
No merge, original PR closure, real-data operation or canonical activation occurred.

### Editor server extraction and live PR-status checkpoint

Previous iteration was PROGRESS: #249 publication, documentation ownership and
exact-head checks were recorded. Re-read the saved objective and resumed the
existing handles. #240 `99785` on `a19ea63`, #245 `56923` on `14ad9ca` and #247
`32625` on `d9717e7` have now all exited zero with complete compatible local Nix
checks. Their original failures remain historical evidence. #247 body update
`76119` was started; #240/#245 body and table updates still need reconciliation.
#248 `51945` and #249 `72947` remain running, not completed gates.

Read the Rust best-practices chapters 1/2/4/5, async and testing skills and the
Spec Kit analyze procedure. Reviewed the complete original editor API, internal
reader, DTOs, 428-line editor tests, 305-line subject tests, 678-line HTTP matrix
and editor contract. Existing profile/template command implementations already
match the original. No page, CSS, navigation or recovery consumer was extracted.

Ran feature-015 prerequisites with no extension hooks. Bounded analysis maps 13
requirements (FR-002/004/006/010/011/012/013/015/018/025/029/030/032) to ten tasks
(T126–T132 and T139–T141), with no unmapped task, ambiguity, duplication or
critical/high specification finding in that boundary. T126 is already owned by
the command foundation; UI portions of T131/T132 and full T018 remain separate.
SC-006 is only a regression subset, and constitution 1.0 remains authoritative.

Created `.worktrees/permission-editor-api`, `feat/permission-editor-api`, first
on #234. Six complete source/test blobs matched original `db3935d` exactly;
only four module/harness composition points differed. Initial source `ad590e5`
passed formatting, but suite `99212` exited 101 at compilation: the strict
historical `ProfileAudit` decoder used by an original assertion belongs to #245.
An existing dead-code expectation also became unfulfilled after endpoint wiring.
No test was removed, replaced or weakened, and SQLx preparation did not run.

Preserved the initial extraction at `backup/permission-editor-api-before-audit`.
Rebased with `--no-update-refs` onto exact #245 `14ad9ca`, retaining every existing
audit/own/delegation module and test invocation, restoring the full original DTO
and removing the obsolete dead-code expectation. The unsigned corrected source
is `58b0c6f8131221e229f28f2f9a94ff965725f346`; worktree clean, not yet published.
All six preserved blobs still match. The initial rebase continuation attempted
an unavailable signing key; explicit unsigned commit completed the rebase without
changing any original backup ref. Corrected suite/cache verification was started;
collect its live handle before offline lints or validating a partial cache.
Prepared `.scratch/verify-extraction.mjs` expects every #245 cache descriptor
unchanged and every addition matching original `db3935d`; it has not run yet.

The user's status request prompted a fresh GitHub read: 30 separation-related
PRs including this ledger, all draft. Required Flake Check is failed on #239 and
#242, running on #240/#248, and successful on #218/#219/#220/#223/#224/#225/#227/
#231/#238. Numerous Nixbot checks also failed; their causes are not established
by the local passes. Do not describe these PRs as merge-ready or bypass checks.
Next prioritize diagnosis of #239/#242 remote failures alongside collecting
already-running gates, then finish the editor API cache and publication. Remaining
consumer/UI ownership, unpublished Clients and final cross-PR accounting are open.

### Remote failure diagnosis and completed editor source verification

Previous goal turn was PROGRESS: it identified the concrete remote failing tests,
not just a red status. Re-read the saved objective, AGENTS and constitution 1.0;
reconfirmed #216 merged as `02f7b58`. Original refs and worktrees remain intact.

Confirmed GitHub master rules require `Flake Check` and `Format`, plus its squash
merge queue. Nixbot is not a required-status rule, but its failures remain evidence
to classify, not something to discard. Stacked PRs do not receive the master-only
Actions workflow until correctly retargeted. No rule or workflow was changed.

- #239 Actions run `37507170660`, attempt 1, failed in the unchanged
  `new-project.cjs` `chooseField` call: payment terms remained `Net 30` after
  selecting `Custom days` (5-second assertion timeout). The test, menu script,
  selector and New Project implementation have no diff against its master base.
  The asynchronous popover opening focus is a candidate cause, not established
  by this observation. Unchanged focused browser reproduction `66596` is running
  against exact built package `lych8wdz3kwlzjjpc5vavn0v5mxjxqkc-horae-0.1.0`,
  with a fresh disposable database and no real account or mail transport.
- #242 Actions run `37513610808`, attempt 1, failed in unchanged
  `cancelled_page_consumer_retains_lock_until_worker_exits_and_rolls_back`:
  immediate reacquisition returned `ApiImportError::Busy`. Review traced the
  cancellation path through dropped `streaming::run_inner`, worker-held session
  and SQLx 0.8 close-on-drop. That path closes asynchronously, while the test
  assumes a replacement pool slot proves PostgreSQL has released the old session
  lock. The normal completion path explicitly awaits unlock for this reason.
  This is an inherited synchronization assumption; no runtime or test patch has
  been made, and eventual rollback/release has not been independently reproduced
  in the failing schedule.
- #242 Nixbot build 71 is distinct: x86 package build hit a 1,200-second timeout;
  x86 tests failed three durable CSV timing cases and the CLI authorization matrix
  (expected rejection exit 1, received indeterminate submission exit 6).
  Browser/VM checks then report dependency failures. ARM tests passed on that
  build. These logs do not prove all Nixbot failures share one cause.

Started one unchanged-commit rerun of each failed Actions run; both are confirmed
`in_progress`, attempt 2. Preserve attempt-1 evidence regardless of the outcome.
No assertions, timeouts or required checks were removed or weakened. #240 now
also has a passing remote Flake Check. Its and #245's PR bodies now correctly
record final local Nix PASS and distinguish unresolved remote checks; top rows
for #240/#245/#247 were reconciled with their completed handles.

Corrected editor suite/cache handle `97735` exited zero on `58b0c6f`: full workspace
suite passed, then complete all-target SQLx preparation succeeded. Provenance
verifier `37593` passed: six original source files exact, 1,225 base descriptors
unchanged, 14 additions matching original `db3935d`, no removed/modified/unmatched
descriptors, 1,239 total. Formatting passed with zero changes. Unsigned cache
commit is `72e97e16a051360ecaae7fc3dcba3457b6acf1a6`.

Final-head offline native/WASM Clippy `19884` and full compatible Nix `64324` are
running. Existing #248 `51945` and #249 `72947` remain live and have advanced into
browser checks; #249's NixOS tests completed, not a substitute for the full gate.
Next finish editor lint/publication and account its exact original hunks, collect
these existing gates and the two Actions reruns, and retain unresolved inherited
test failures explicitly. Canonical/UI consumer ownership, unfinished Clients,
final per-change mapping and cross-PR integration remain open. No merge, original
PR closure, real-data mutation or policy activation occurred.

### Editor publication and completed remote/local checks

The implementation iteration preceding the LTO question was PROGRESS: #250 was
published, its web lint failure was corrected using existing historical
annotations, and completed checks were collected. The LTO question made no goal
code change. On resumption, the exact live handles were revalidated, not restarted.

#239 run `37507170660` and #242 run `37513610808`, attempt 2, both completed
successfully on the unchanged commits. Required `Format` also passed on both.
Their initial failures remain above, not claimed fixed by a retry. #239's unchanged
complete focused `new-project` browser suite `66596` also passed on its exact
package. No assertion, timeout, runtime behavior or build profile changed.

#248 complete compatible local Nix `51945` passed on `49843b2`, including browser
and VM checks; required GitHub Flake Check and Format also passed on that head.
#249 complete compatible local Nix `72947` passed on `71232dc`, including browser
and both VM suites. Its stack still requires prerequisite integration and
retargeted remote checks. PR bodies for #239/#242/#248/#249 record these results
and distinguish required checks from unresolved Nixbot evidence.

Nixbot #245 build 96 was inspected separately: Darwin VM scheduling lacks
`apple-virt` on the selected builder; ARM Linux VM reports `Shell did not start in time`; Darwin tests fail an inherited credential-retry case with `Socket is not connected` (968 passed, one failed). These specific observations do not
classify every remaining Nixbot failure or certify non-native platforms.

Draft [#250](https://github.com/numtide/horae/pull/250) is published on exact #245
`14ad9ca`. Initial cache-inclusive `72e97e1` passed native lint but WASM reported
17 unused DTO types because the UI consumer is intentionally separate.
The attempted module expectation in `fe7d2f8` was itself unfulfilled (`24445`);
it is removed, not broadly allowed. Final unsigned/pushed
`c82a5b397d61e961b4a336223c5b53412c499b65` restores the eight per-type transport
expectations from original `03e90b1` and the subject-page expectation from
`8d49421`. Their later removal belongs with the preserved UI consumers.

Both final strict lint targets passed (`12387`): WASM first, then native workspace
all-targets with warnings/performance lints denied. Provenance `53103` passed:
five original files are exact; the model is exact after removing only those nine
specified historical annotations, whose source text is checked against the
historical commits. All 1,225 base descriptors and 14 original additions remain
unchanged (1,239 total). Formatting passed with zero changes before commit.
Final diff returns `models.rs` to its base; only four module/harness composition
points remain alongside the preserved files and annotations. No query, test
assertion, serialization field, endpoint semantics or policy activation changed.

Superseded Nix handles `64324` and `22155` were deliberately interrupted after
identifying their exact editor-worktree processes; both exited with interruption,
not success. Final clean-head Nix `90520` on `c82a5b3` is confirmed live. Do not
restart it or count the prior interrupted builds as gates. #250's PR body records
the full failure/correction history, source suite/cache evidence and pending gate.
The top delivery row and eight original-commit ownership rows now account for
the API while explicitly retaining the editor UI and its removed lint annotations.

Next collect `90520` and continue source-based ownership of the remaining
canonical consumers, editor UI, browser tooling and unpublished Clients work.
Final full original-change accounting and cross-PR integration remain incomplete.
LTO was explained to the user but not changed; no merge, original PR closure,
real-data operation or policy activation occurred.

### Build performance priority

The user explicitly requested release-profile optimization and Crane adoption,
in isolated PRs, and authorized merging those build PRs after their checks pass.
That priority precedes the remaining separation work; it does not authorize
merging the extraction PRs or closing the preserved originals. Start from
current `origin/master` (`1b8fa4f`) and retain all existing test gates. Measure
build behavior and dependency reuse rather than claiming an unmeasured speedup.
Resume the remaining ownership mapping above after the build PRs are merged.

### Build PRs published; verification in progress

Both priority branches start from exact master `1b8fa4f`; no extraction branch
was merged or rewritten.

- [#251](https://github.com/numtide/horae/pull/251), `perf/release-thin-lto`,
  worktree `.worktrees/release-thin-lto`, unsigned `0004659`: only two release
  values change (`lto = "thin"`, `codegen-units = 16`). Formatting passed;
  complete native/WASM package `42999` passed (build phase 8m56s). Binary size
  increases from 90,176,360 to 119,730,568 bytes; public bundle allocation from
  3,728 to 3,916 KiB. Exact-master control rebuild `91392` remains running;
  concurrent workloads preclude claiming a controlled speedup. Full local gate
  `57447` and remote Flake Check run `37549267934` remain pending. Format passed.
- [#252](https://github.com/numtide/horae/pull/252),
  `build/crane-dependency-cache`, worktree `.worktrees/crane-dependency-cache`,
  stacks on #251. Pins Crane `47b6b27` without changing Rust/Dioxus/nixpkgs or
  Cargo.lock; separates Dioxus release dependencies from shared development
  dependencies for tests/Clippy/SQLx. No assertions or test execution removed.
  Initial `8be49dc` failed minimal-source construction because generated stubs
  were read-only. Unsigned/pushed `8b4c69d` fixes only that generated directory;
  minimal-source build `59698` passed. Superseded full runs `22361`/`22680` were
  deliberately interrupted, not passed; corrected full gate `10968` is running.
  The first dependency preparation requires fetching/unpacking Crane's vendor
  layout. It is not a warm-cache benchmark.

The initial source-only probe changes an actual Rust file temporarily: application
derivation changes, both dependency derivations remain identical. The probe was
removed with a patch and clean diff verified. Initial tests/Clippy/SQLx also
evaluate to the same development dependency derivation. This demonstrates key
stability, not yet successful Cargo artifact reuse. Final Git-source derivations
on `8b4c69d`: package `qck7vabd696gkqh2898j5qz42c32k3nz`, release dependencies
`1250yiw3cd1ba0yafilansgwfll69x1a`, check dependencies
`45fhbl1nxfzlfayfa2g79rgz64a8gc6i`. Dev shell probe `48847` passed (Cargo1.96.1,
Dioxus0.7.9, database environment present). ARM and cross-overlay evaluation
passed, but cross builds are not certified by evaluation.

Next collect priority gates and baseline, inspect actual dependency reuse and
package/browser/VM behavior, correct findings, then merge #251 only when ready.
Retarget #252 to master afterwards and require its own final checks before merge.
Both remain draft. Original editor gate `90520` also remains running; preserve
its result. Resume extraction ownership work only after the build priority.

### Release-profile local gates passed; protected merge requested

On exact #251 `0004659`, full local `nix flake check` (`57447`) completed
successfully, including browser, deployment VM and OIDC VM checks. Additional
`cargo test -p horae-core --release --locked` (`33471`) passed all 121 tests.
The PR is now ready for review, not draft. `gh pr merge --auto --squash` with
`--match-head-commit 0004659a1d77046a12c18533d68cdcdd3b74e76a` succeeded in requesting
the protected merge workflow (`12844`); required remote Flake Check was still
pending, so this is not evidence of an actual merge. No bypass used.

#252 stays draft on `8b4c69d`. Vendor-only preparation `73582` passed after
parallelizing only the download/unpack derivations; full gate `10968` now builds
both dependency caches. Actual final-application artifact reuse remains pending.
Exact-master timing control `91392` and original editor full gate `90520` remain
live. Do not restart them or count any partial result as full success.

Next collect these runs and #251 merge state. After confirmed merge, rebase the
two own Crane commits onto updated master (preserving the published head with a
lease), retarget #252, and run its required final checks before its authorized
merge. Only then resume the original separation goal. Preserve the measured
binary/bundle size trade-off and avoid an unsupported benchmark percentage.

### Dependency-only WASM packaging correction

The previous priority iteration was PROGRESS: #251 passed full local gates and
entered the protected automatic-merge workflow; #252 was published with separate
dependency caches and a verified source-change key probe. At continuation #251
is still open with remote Flake Check and Nixbot build explicitly in progress.

#252 full gate `10968` on `8b4c69d` failed, not timed out: Dioxus compiled the
minimal source but wasm-bindgen could not find `clone_ref` intrinsics in its
empty WASM program. This then caused the missing `_bg.wasm` packaging error.
Published unsigned `6a05046` adds a web-only Dioxus entry point solely to the
generated dependency source. The real application source is unchanged. It does
not ignore the bundling failure or skip final application compilation. Formatting
passed; corrected full gate `45037` is confirmed live. Cold vendor preparation
is already cached. Actual artifact reuse remains to be verified after it passes.

Separately, original editor gate `90520` terminated with failure in the unchanged
New Project browser test: `chooseField` expected `Hours per person` but received
`Total project hours` after 5 seconds (`new-project.cjs`, scenario near line625).
Its test suite, native Clippy and SQLx phases passed, but this is not a complete
gate pass. The browser test and Projects/select paths have no diff against the
exact #245 base. Cause remains unproven; do not label the failure fixed or exempt
the test. Keep #250 draft and revisit the browser evidence after the build
priority. Exact-master Fat LTO control `91392` is still live.

### Baseline completed and final dependency stub verified for WASM

Exact-master control rebuild `91392` completed successfully, including Nix's
output comparison: native/WASM build phase 18m42s, versus 8m56s for #251. These
are indicative local observations under concurrent load, not a controlled
benchmark or a runtime-performance claim. #251 body now includes both times and
the previously measured size increase. Remote required CI is still pending;
automatic merge remains enabled, not completed. Current branch rules were read:
required Flake Check and Format, squash-only PR merges and protected merge queue.

The intermediate #252 stub `6a05046` failed Rust compilation: its qualified RSX
macro expands to an unimported `dioxus_core` name (`E0433`). No dependency was
added to accommodate a temporary stub. Published unsigned
`6ef7bec7fe8fe6dfbb78a223098de6c51b9d4a1d` uses the existing
`dioxus::prelude::VNode::empty` function directly; the pinned library declares
the exact `fn() -> Element` signature required by `launch`. Superseded run
`45037` was deliberately interrupted after identifying its own PID/worktree,
not reported as a passing gate. Corrected full run `2967` is live; its generated
WASM client compiled and bundled successfully at 37.40s. Server dependency
compilation and subsequent real application/check gates are not yet complete.

The source-only key test was repeated on `6ef7bec` using the same Git-source
flake entry point as CI (not an impure path-flake source). A temporary comment
in `crates/core/src/lib.rs` changed the package derivation from
`v2kkhxzdf7kr837376k8za1y89n7296p` to `gyr6zvcpnvwqhf283r8afb04vfnk8qin`.
Release dependency key `41y1cd29z5bzp58s0anfijd1kvlr0gbx` and development dependency
key `2h9fc5lh0vsbff5yfy09ad77gsr3ngm9` stayed identical. The comment was removed
with a patch; `git diff --exit-code` passed. This still does not claim actual
Cargo artifact reuse until the final package is observed rebuilding from them.

Next collect `2967` and #251 required CI/merge state. After confirmed #251 merge,
rebase all four own Crane commits onto current master and retarget #252, preserving
the published head with a lease. No extraction work resumes before build priority
completion. #250's unresolved browser failure is also recorded publicly in its PR.

### Actual Crane reuse and generated-bundle cleanup

On `6ef7bec`, both dependency builds completed successfully. The final package
restored the release cache and recompiled workspace crates/build scripts only:
real WASM completed in 39.85s and the complete build phase in 3m34s. These remain
indicative local timings under concurrent load, not controlled benchmarks.
Clippy and SQLx passed using the shared development cache. Full run `2967`
continues; partial results are not a full gate pass.

Package review found two JS/WASM pairs: the real application plus the dependency
stub's content-addressed assets. Pinned Dioxus clears its executable directory,
not all restored public assets. Published unsigned `774966e` removes only
`target/dx/horae/release/web` inside the package build sandbox before the real
Dioxus build, preserving Cargo compilation artifacts. No user files or database
state are removed. Formatting passed; final-head full gate `68290` is running.
Verify the final bundle contains no stub assets before marking #252 ready.

#251 Nixbot build112 failed its two ARM VM checks. Both raw logs show missing
KVM, fallback to TCG and guest-shell startup timeout while waiting for PostgreSQL,
before application assertions. The limitation is documented on the PR, not
treated as an ARM runtime pass. Required GitHub Flake Check remains pending;
the exact-head local x86_64 suite passed. No checks or protections were disabled.

Next collect both local runs and the existing #251 required-check watcher.
After actual #251 merge, rebase all five own Crane commits and retarget #252 to
master, then require final-head checks before its authorized merge. Original
extraction work remains deferred until these build-priority PRs are handled.

### Crane final local gates passed; release profile in merge queue

Original full Crane run `2967` completed successfully on `6ef7bec`. More
importantly, corrected full run `68290` passed on exact published `774966e`,
including browser, deployment VM and OIDC VM checks. Package derivation
`59mnq73q7hz4qssknzpc9h3pp7njyf5z` produces
`rbikx1qa8bihgqmcwv38dx200jmih142-horae-0.1.0`. The real build took 3m19s,
WASM31.42s (indicative timing, not controlled). Explicit assertions passed:
both stub asset names absent, real WASM/server present. Only one real JS/WASM
pair ships. Store-reference query returns only glibc, not compiler/cache outputs.

Release/development dependency keys remain `41y1cd29z5bzp58s0anfijd1kvlr0gbx`
and `2h9fc5lh0vsbff5yfy09ad77gsr3ngm9`. Final Clippy, tests and SQLx derivations
are identical to those already verified in `2967`, so their Nix reuse is valid.
The changed package/browser/VM checks were rebuilt/retested. #252 is now ready
for review, with explicit prohibition on merge until rebased master CI passes.

#251 required PR checks passed (Flake Check35m54s, Format44s). Its protected
merge-group run `37552522194` is live, watched every120s by session `37872`.
Format passed; Flake Check remains pending. This is queue entry, not a merge.
The old PR-check watcher `56175` has finished successfully. Both local Crane
gate handles are complete and must not be restarted.

Next collect the merge-group result, confirm actual #251 merge, fetch master,
rebase the five own Crane commits with unsigned commits/known-head lease, and
retarget #252. Verify final-head required CI and preserved local derivations
before requesting its protected merge. Resume the original goal only afterwards.

### Remote Crane platform evidence while protected queue runs

Nixbot build121 on `774966e` completed: native ARM package, tests and browser
passed, as did x86 package, browser and both VM checks. Only the two ARM VM
checks failed. Their exact raw logs each show missing KVM, TCG fallback and
guest-shell startup timeout while waiting for PostgreSQL, before application
assertions. This improves native ARM evidence but is not an ARM VM pass or a
cross-build certification. Results and links are recorded on #252.

Created local recovery ref `backup/crane-before-master-rebase` at exact verified
head `774966e7640bc7338b036b75d6e1f86411faa34b`; the worktree is clean. #251's
merge-group run and watcher `37872` are still authoritatively live. Its earlier
successful PR CI uploaded cache paths successfully; no cache-upload failure was
found. No build was restarted and no merge protection changed. Next action
remains to collect that queue result, not to start new functional work.

### Release merge-group failure and selector test synchronization

The previous iteration was PROGRESS (remote Crane evidence and a recovery ref).
The following wait was verified against live queue watcher `37872`. That watcher
is now terminal: merge-group `37552522194` failed after37m39s. #251 remains OPEN,
with no merge commit and no auto-merge request. Do not restart the old watcher.

The failing browser assertion expected `Hours per task` but retained
`Hours per person` in `chooseField`, New Project scenario near629. This is
not an ARM virtualization failure. Source review found that the helper focuses
its target immediately after opening, before asynchronous native `toggle` can
focus the selected option. A browser-only reproduction with the exact shared
menu script (`.scratch/select-focus-order.cjs` in the release worktree) confirmed
that ordering: native pre-toggle focus was body, an early target focus was then
overwritten by the selected option. Waiting for initial selected-option focus
before choosing avoids it. Probe runs `7549` and `30646` passed; they use no DB.

Published unsigned `a2b12458af4052617bfd5f2110ea27cb8eadf8e3` adds one readiness
assertion and a two-line explanation to the existing test helper. No component,
application behavior, selection assertion, timeout or gate is changed. This is
a necessary test synchronization repair within #251, not evidence that Thin LTO
caused the underlying race. Formatting passed (`31490`). Focused complete New
Project suite `37073` runs against the exact already-built Thin LTO binary with
a disposable database; it has passed the previously failing reload/selection
scenario, but the entire suite is not yet complete. Final-head full Nix gate
`48807` is live. PR body distinguishes old-head successes from new verification.

#252 remains unchanged and fully locally verified on `774966e`; its rebase must
inherit this test repair after #251 actually merges. Next collect repaired-head
gates, then re-enable protected auto-merge with exact-head matching. No merges
have occurred, and the original extraction goal remains deferred until priority
build PRs are integrated.

Focused run `37073` subsequently FAILED at a different readiness assertion:
`Draft saved at` remained `Saving draft…` for5s near New Project1052. Its failure
snapshot already showed the saved status and no pending requests; disposable
server log `/tmp/horae-browser.zCVW0N/server.log` recorded a PostgreSQL pool
acquisition timeout during concurrent compilation. This is not a full-suite
pass and is not yet classified as a product defect or purely load-related.
Do not weaken its timeout. Formerly failing selector scenarios did pass.
Wait for isolated Nix gate `48807`; repaired-head required CI run37556278237 is
watched by live session `16075` at120s intervals (Format45s passed). These two
handles, not the failed old queue or focused suite, are the active verification.

### Verify repaired Crane base in parallel with release CI

The last iteration was PROGRESS: selector readiness correction published with
deterministic focus-order evidence, and the separate draft timeout preserved.
At continuation both `48807` and `16075` are confirmed live. Release application
build and Clippy have now passed within `48807`; remaining gates are pending.

To avoid deferring independent priority verification until #251 merges, rebased
the five owned Crane commits onto repaired `perf/release-thin-lto` now. Published
with exact old-head lease: `587119e5e91d31c615ee63064bced7e43e620112`. Compared
directly with original `774966e`, its tree differs only by the three test lines;
its PR diff relative to #251 remains the same six build files. No goal extraction
or application feature work resumed. Full combined-head gate `6287` is live
with cores2/max-jobs1; previous Crane head results are not claimed for it.

Git's existing `rebase.updateRefs=true` also moved the owned recovery branch.
Immediately restored `backup/crane-before-master-rebase` to exact original
`774966e7640bc7338b036b75d6e1f86411faa34b` using a compare-and-swap update and
verified it. Use `--no-update-refs` on subsequent rebases; do not alter global
configuration. Direct original-SHA comparison, not the temporarily moved ref,
proved the inherited diff. Worktree is clean and PR description updated.

Next collect `48807`, `6287` and required-check watcher `16075`; only request
protected #251 merge after repaired-head verification. Once actually merged,
rebase/retarget #252 onto master with an exact `587119e` lease (or its current
verified successor), no-update-refs and unsigned commits, then require its own
master-base CI. Preserve all timeout evidence and do not weaken browser checks.

### Repaired release head passed full local verification

Full isolated gate `48807` completed successfully on exact
`a2b12458af4052617bfd5f2110ea27cb8eadf8e3`: tests, Clippy, SQLx, browser,
deployment VM and OIDC VM. It passed the formerly failing selector scenario
and all subsequent New Project/draft checks without changing timeouts. The
separate focused run's pool/readiness timeout remains a recorded failure; it
did not recur in this complete isolated run. Do not poll `48807` again.

Requested protected automatic merge again for #251 with exact-head matching
(`83041`), pending required GitHub CI watched by `16075`. The PR body now scopes
the full local success to the repaired head and retains all earlier caveats.
This is a merge request, not a confirmed merge. Crane combined-head gate `6287`
remains active; its native compile was independently observed consuming CPU,
so a quiet log was not treated as a stopped process or grounds for restart.

Next collect merge-request confirmation, required CI, and `6287`. Keep the same
ordering: actual protected #251 merge, no-update-refs unsigned rebase/retarget of
#252, final master-base CI, authorized #252 merge, then original split goal.

Merge request `83041` completed successfully: exact repaired #251 head verified,
auto-merge enabled at2026-10-07T01:37:13Z, stateOPEN, mergeCommitnull. No merge yet.
Crane run `6287` has now built the real package on `587119e`:8m53s, WASM76.88s,
with cores2/max-jobs1 and unchanged restored dependency artifacts. This slower
observation is included in the PR body alongside the earlier3m19s observation:
do not present the fastest sample as a guaranteed speedup. Concurrency/load were
not controlled; artifact reuse, not a fixed timing ratio, is the proven benefit.
Remaining Crane checks continue. Live handles are `6287` and `16075` only.

### Both repaired heads verified; explicit protected queue entry

Previous iteration was PROGRESS: repaired release full local gate passed,
Crane inherited the test synchronization and its real package was built.
At continuation, exact-head remote Nixbot builds125 (`a2b1245`) and128 (`587119e`)
both passed package, tests, Clippy, SQLx, formatting and browser checks on x86 and
ARM Linux, plus both x86 VM checks. Only ARM VMs failed; each of the four exact
raw logs again proves missing KVM, TCG fallback and guest-shell startup timeout
while waiting for PostgreSQL, before application assertions. Both PRs have
current-head evidence comments; no check was disabled or ARM deployment claimed.

Crane full local gate `6287` completed successfully on exact `587119e`, including
all browser and VM checks. Final package `kqhd2w13ls03fp972728ldc36vvr7qsm` was
inspected: no stub JS/WASM, one real pair, server present, runtime store references
only glibc. Required #251 CI `37556278237` passed (Flake34m17s, Format45s); watcher
`16075` is complete. These two handles must not be polled/restarted.

Despite successful normal merge requests, GraphQL showed no queue entry for
#251. Re-read actual branch rules: required Flake Check/Format, squash-only,
ALLGREEN protected queue; no rules changed. An explicit normal CLI request
`89212` still returned no entry. Used the documented GitHub `enqueuePullRequest`
API with exact expected head and `jump:false`, not an administrator merge.
GitHub confirmed position1, stateQUEUED at2026-10-07T01:54:36Z, entry
`MQE_lQDOTRPZ888AAAABHBMpBs4AA_LZzgMiH-I`. This does not establish the CLI's
underlying cause and is not a bypass or an actual merge.

New merge-group run `37559430187` is confirmed active (created01:54:54Z), while
the old failed queue run remains terminal. #251 still OPEN, mergeCommitnull.
Next monitor this new group, then confirm actual merge before the no-update-refs
unsigned Crane rebase/retarget with an exact `587119e` lease. #252's current full
local/remote evidence does not replace required CI on its eventual master base.

### Read-only preparation while the protected queue runs

Merge-group watcher `90784` remains live for run37559430187 at120s intervals;
Format passed and Flake Check is pending. No new extraction or application edit
was started before the two priority build merges. Original #212 remains
`db3935d` with its same18 unpublished paths; recovery references are preserved.

Dependency inventory of the retained readers: directory `1eb13ec`, project-team
picker `6b5dbae`, and initial time reader `4ac30fa` all use the storage loader and
`configure_administration` (already carried by #222/#226) plus
`PermissionRequester` (already carried by #243 and descendants). The project
picker additionally imports the directory's `PeopleCursor`; its authorization
is project-operation scope, not directory access. The directory HTTP test also
contains legacy identity assertions already owned by #240: preserve those and
add only the canonical-reader coverage, rather than restoring an older file.

Do not extract final `time_entries.rs` wholesale as the initial reader: its
later history includes `60f60f9` subject discovery, `5faed76` requester/subject
binding and `75f13a1` report composition. These are separate dependencies to
account for. This is source inventory only, not a completed adversarial review
or newly verified extraction. Continue the protected #251 merge first, then
rebase/verify/merge #252 before resuming extraction work.

Read-only blob comparison against all30 extraction heads (#219–#250, excluding
#229/#230) and master `1b8fa4f` provides a lower-bound conservation check for
the1,214 original changed paths:98 non-SQLx/non-spec files,480 SQLx descriptors
and48 spec files have an identical final blob in at least one extraction.
The other220 ordinary files,341 descriptors and7 spec files require hunk-level
accounting or still-pending extraction;20 original SQLx deletions require
query/cache reconciliation. A differing blob is not evidence of lost work:
several files intentionally contain only one extracted responsibility or have
documented compatibility adaptations. An identical blob is not evidence that
the combined PRs build or preserve behavior. Keep the final ownership and
cross-PR integration audit open; do not convert these counts into completion
percentages. Merge-group watcher `90784` remains live with Flake Check pending.

Queue source identity confirmed: commit `35dc414fa53e43ded274dcb0252c11d44c5d0e74`
has tree `2546cacc3f025ef82c555b13f86126b459c6c34e`, identical to verified
`a2b1245`, with parent master `1b8fa4f`. Its Flake Check remains in progress;
the watcher is live, not a stopped build. Current CI uses pinned Hestia backed
by GitHub Actions cache; [GitHub's cache isolation rules](https://docs.github.com/en/actions/reference/workflows-and-actions/dependency-caching#restrictions-for-accessing-a-cache)
restrict PR-created caches to their merge ref. This is a possible contributor
to repeated queue build time, not a measured attribution or reason to weaken
isolation. No workflow/cache trust changes made. Crane dependency reuse is
verified locally; its eventual master-base CI timing remains unmeasured.

### Release profile merged; Crane verified on its final master base

Protected merge-group run37559430187 PASSED: Flake43m14s, Format53s.
Watcher `90784` is terminal. GitHub confirms #251 MERGED at2026-10-07T02:38:45Z,
commit `35dc414fa53e43ded274dcb0252c11d44c5d0e74`; fetched master matches it.
No rules were bypassed. GitHub automatically retargeted #252 to master.

Preserved current Crane head in `backup/crane-before-release-merge` at
`587119e5e91d31c615ee63064bced7e43e620112`, in addition to the older `774966e`
backup. Rebased only its five owned commits unsigned with `--no-update-refs`
onto actual master; both recovery refs remain unchanged. New head
`f6f09438be109b5dfc376f381f59892921fed736` has an exactly identical tree to
`587119e` and still changes only six build files (83 insertions,26 deletions).
Published successfully with an exact `587119e` lease; worktree clean.

Final-head local full Nix gate `94135` PASSED, reusing the same verified package,
tests, Clippy, SQLx, browser and VM derivations. This validates the rebased
source without claiming that cached checks ran again. Required master-base CI
run37563128636 is now active, watched by `97039` at120s intervals. PR description
updated; automatic protected merge requested with exact final-head matching
(`70881` succeeded). GraphQL confirms it enabled at2026-10-07T02:41:49Z,
stateOPEN, mergeQueueEntrynull and mergeCommitnull. #252 is not yet merged. Next require current
remote checks, confirm a genuine queue entry and its result, then actual merge
before resuming original extraction work. No extraction PR merge authorized.

### Supply the existing signed dependency cache to GitHub CI

Required Crane run37563128636 remained live beyond one hour on `f6f0943`;
Format44s passed. A normal enqueue attempt (`20445`) was rejected because
Flake Check was still in progress: no queue entry or bypass. Partial job-log
download returned BlobNotFound (`68665`), not a terminal build result.

Read-only diagnosis found local Nix already trusts `https://cache.numtide.com`
and its official public key, while the GitHub workflow did not configure it.
The key matches [Numtide's published cache](https://cache.numtide.com/index.html).
Public narinfo confirms both exact dependency outputs (`6264yrq` release,
`p0fga5r` development) and the previously verified `kqhd2w1` package are present
with that signing-key identity. This establishes available reusable artifacts,
not a complete attribution of the old run's duration. No global config changed.

Added eight lines to the two existing install-Nix steps: extra substituter and
extra trusted public key only. Default caches, signature verification, required
checks, job permissions and tokens remain unchanged. Formatting (`67880`) and
pinned-nixpkgs actionlint (`48939`) PASSED. Unsigned/pushed
`170e52175ad0be9b02159ed16f1ca16273af8b52`; PR now changes seven build/CI files.

Current full local gate `12076` is live; it restores the same release dependency
artifact and rebuilds the package because the source includes the workflow.
Current required CI run37568358785 is watched by `90257` at120s; Nixbot138 also
started. All current-head full-gate results are pending, not inherited passes.
Cancellation requested for only the verified superseded run37563128636
(`24771` accepted); watcher `97039` still lives until its terminal result arrives.
Do not restart or count that old run as passed. Collect its final log when
available. PR description distinguishes requested cancellation from completion.

Next collect current local/remote gates and old cancellation, inspect actual
cache reuse in current CI, then use the protected queue after required checks
pass. Do not retry enqueue while the same check is pending. #251 remains merged;
#252 and the original split goal remain unfinished.

Superseded watcher `97039` is now terminal (exit1): GitHub API confirms
run37563128636 completed with conclusion `cancelled`, exact head `f6f0943`.
Its Flake job duration1h11m23s includes cancellation processing and is not a
successful cold-build benchmark. The post-termination log request (`61362`)
still returned BlobNotFound; do not infer a specific compile/test failure or
claim full timing attribution. No forced cancellation or further restart.
PR description updated. Current live handles are local full gate `12076` and
required-CI watcher `90257` (run37568358785, Format50s passed), both on `170e521`.

### Priority build PRs merged; resume the original separation goal

Current-head local full Nix gate `12076` PASSED, including tests, SQLx, Clippy,
browser and both x86 VM checks. Required run37568358785 PASSED on `170e521`
(Flake19m59s, Format50s); watcher `90257` is terminal. Its complete log explicitly
copies both `6264yrq` release and `p0fga5r` development artifacts from
`https://cache.numtide.com` and restores them for the package/test/Clippy/SQLx
builds. Cache reuse in GitHub CI is now observed, not inferred from local runs.
Do not compare this successful duration with the canceled old run as a benchmark.

Final installed package `xm11dbjilz94xnfgpgivxni0vb17daq5` was compared with
previous verified `kqhd2w13ls03fp972728ldc36vvr7qsm`: server bytes and entire
public tree are identical; runtime store references contain only glibc.
Nixbot138 passed native package/tests/Clippy/SQLx/browser/format on both Linux
architectures and both x86 VMs. Both exact ARM VM raw logs (`drv/17/raw`) prove
missing KVM, TCG fallback and guest-shell startup timeout before application
assertions. No ARM VM deployment claim or disabled checks.

Automatic merge entered the normal protected queue without another mutation:
run37569967612 PASSED (Flake44s, Format43s). GitHub confirms #252 MERGED at
2026-10-07T04:09:07Z as `ed558f62ef03401864b0e0228b681cdb352edb62`.
Fetched master equals that commit; its tree exactly matches `170e521`.
PR description and final verification comment updated. #251 and #252 are both
integrated; their branches, worktrees and recovery refs remain preserved.
No live build/check handles remain from this priority work.

Original-goal prerequisite #216 revalidated MERGED at `02f7b58`. Original #212
is still `db3935d` with the same18 unpublished paths; #250 remains clean at
`c82a5b3`. No extraction PR was merged or closed. The original goal is active
and incomplete. Next resume #250's inherited browser-gate reconciliation against
the now-merged selector repair/build configuration, using a preserved isolated
prerequisite composition so its review diff stays scoped. Then continue the
inventoried reader/editor/consumer groups and final hunk-level ownership plus
cross-PR integration audit. Do not treat build optimization as completion of
permission separation, activation or Harvest parity.

### Editor API verification on the merged build base

Revalidated #216 as merged and #250 as open/draft at `c82a5b3` with a clean
worktree. Preserved that exact head as
`backup/permission-editor-api-before-build-base`. Created review-only base
`integration/permission-editor-build-prerequisites` at `0117991`, combining
master `ed558f6` and prerequisite #245 `14ad9ca` without conflicts. Relative to
#245 it changes only nine inherited build/CI/browser-helper files (116 additions,
39 deletions); no permission source was replaced. This branch is a verification
composition, not a delivery target or authorization to merge its prerequisites.

Rebased only the four editor API commits with `--no-update-refs`; new head is
`c727bc80608a61dc5e138c39a2fc389c452425b5`. All four entries in `git range-diff`
are equivalent. The review diff remains 24 files, 2359 additions and 16 deletions.
Published the prerequisite branch and the owned rebase with an exact old-head
lease, then retargeted #250; push/retarget handle `5584` completed successfully.
No extraction PR was merged, no original source was edited, and no policy was
activated.

Full current-head `nix flake check -L --cores 2 --max-jobs 1` is running in the
editor API worktree as session `46207`. Its result is pending: old-head passes
do not certify this composition, and the merged select-helper repair has not
yet been proven to close #250's browser failure. Next collect this gate and
reconcile the result, retaining the draft until verified; continue the existing
reader/editor/consumer inventory without expanding product scope.

### Scoped people-directory reader extracted independently of editor operations

Previous iteration was progress: #250's owned commits were preserved, rebased
without patch changes, published and submitted to a new exact-head gate. Its
session `46207` remains live; formatting and WASM package compilation passed,
but the full gate is still pending. No restart or inferred terminal state.

Created `feat/scoped-people-directory` in `.worktrees/scoped-people-directory`
from the existing review-only base `0117991`. Draft PR #253 contains original
reader commit `1eb13ec`'s production endpoint, DTO, storage reader, seven DB
tests and registered-session assertions. It does not require #250 editor
operations or #240 legacy projections. Shared dependencies are permission
storage/grants, `configure_administration`, person-management relationships,
profile test fixtures and the existing `PermissionRequester` value in the base.
No migrations, UI consumers, legacy `list_users` changes or activation are added.

Production reader/DTO and DB test files are byte-identical to `1eb13ec`.
Exact string comparison also passed for the complete session endpoint and HTTP
test body. Only the HTTP module/entry name changes: canonical `check_scoped`
becomes `scoped_directory::check`, registered in the existing HTTP suite. This
avoids copying the legacy assertions already owned by #240. The later
`ee16165` DTO changes are only an import spelling and removal of two web-only
lint expectations; those stay with its unextracted UI consumer. Specification
history/contracts remain owned by #248, not duplicated into this code PR.

Adversarial source review checked session-only actor/tenant derivation, explicit
policy-1 admission, active actor SHARE locking, fail-closed corrupt/missing
permission state, managed-person rather than project-manager scope, scope before
activity/cursor/limit, fixed identity-only projection, safe error translation,
bounded transaction settings and cancellation. The existing seven tests cover
revocation waits, relationship removal, direct deactivation, pool reuse, deleted
cursors, page boundaries and foreign/empty scopes; HTTP assertions cover forged
authority, unauthenticated/revoked callers and non-disclosing errors. These are
source-review observations, not yet a passing execution or cross-PR integration
claim. No new product semantics or relaxed assertions.

Initial unsigned head `269e58a` passed formatting (`68456`, zero changes).
Verification `14669` then FAILED offline compilation for two original test
queries whose descriptors came from earlier commits `03e90b1`/`8d49421`, not
the directory commit itself. Preserved and copied those exact source descriptors
(`0c02f33`, `f68ed67`) in unsigned/pushed `3a3753826a17923ecb1464aa9d911876e5a9ef86`.
The PR now adds 23 files/1177 lines, including 14 original SQLx descriptors;
two other descriptors added by `1eb13ec` already exist in the base. No base
descriptor was removed or modified.

After `14669` terminated, restarted tests/Clippy/SQLx verification on the changed
head as session `18891`, using isolated disposable PostgreSQL in Nix. Results
remain pending. Full browser/deployment gates and composition with #240/#250
remain required. Next collect both live gates, resolve only demonstrated
extraction failures, then continue the project-team reader/editor UI groups.
Original #212 still has its same 18 unpublished paths untouched; #208/#217 and
all original backups remain preserved. No extraction PR merge or closure.

### Project-team identity reader extracted on the shared cursor prerequisite

Previous iteration was progress: #253 was published with its source ownership,
preservation proof and a demonstrated cache-dependency fix. Current #253 head
`3a37538` now PASSES strict server/core Clippy and live SQLx cache checking in
session `18891`; 189 core tests passed, application tests are still compiling.
Session `46207` on #250 `c727bc8` remains live: package server/WASM and Clippy
passed, browser suite is progressing. Neither full gate has finished.

Revalidated #216 MERGED before creating `.worktrees/project-people-picker`,
branch `feat/project-people-picker`, from #253 `3a37538`. Unsigned/pushed head
`f498c3f93254cf87dc026b0e0a10c8bd4713d141` is draft PR #254, based on #253 for
the existing `PeopleCursor` type and inherited permission foundations. It does
not require editor API #250 or legacy projection #240. No new abstraction was
introduced just to remove that existing dependency.

The four complete original DTO/reader/DB-test/HTTP-test files are byte-identical
to source commit `6b5dbae`; exact comparison of its complete session endpoint
also passes. Adaptations are only module registration and test wiring beside
already extracted modules. Ten original SQLx descriptors are preserved unchanged.
The diff is 19 files/1258 additions, with no modifications to legacy operations,
no UI, no assignment writes, no migrations and no activation. The three DTO
web-only lint expectations removed by `2497dbe` stay until that original UI
consumer is extracted; no runtime-content difference is hidden there. Contracts
and historical specification changes remain owned by #248.

Adversarial source review checked separate create/edit grants, current managed
project designation, tenant-bound project existence, failure on missing/corrupt
authority before query validation, active-only candidate filtering, literal
substring search rather than wildcard expansion, 50-row pages/51-row lookahead,
100-character search and 500-ID resolution limits, ID/name-only output, session
requester binding, bounded transactions and cancellation. Original archived
project behavior is preserved, not reinterpreted. The nine unchanged database
tests exercise scope, page boundaries, foreign/deleted cursors, unconfigured
candidates, revocation/deactivation waits, authority held through materialization
and cancellation/pool-default restoration. Original HTTP tests retain forged
authority, unauthenticated/denied callers, safe errors and resolution checks.
Source review is not a substitute for execution or full integration.

Formatting `66398` PASSED with zero changes. A read-only inventory matched all
47 SQL macro invocations in the new reader/DB/HTTP files to exact cached query
strings, with zero missing descriptors; this is not a compile/schema test.
Runtime, server/WASM and full Nix gates for #254 have not started yet, while
the two existing verification sessions remain live. Next collect #250/#253,
then run #254's gates and cross-PR compositions before moving to the retained
time-reader/editor/UI blocks. No merge or closure was requested or performed.

### Cross-PR composition preserves legacy and canonical read coverage

Previous iteration was progress: draft #254 and its source ownership were
published. Created isolated `.worktrees/permission-readers-editor-check`, branch
`integration/permission-readers-editor-check`, from #254 `f498c3f`, then combined
#250 `c727bc8` and #240 `a19ea63` in unsigned local commits `a9f374e` and
`7a2d61c1fa6513a4c10056ef94f1f2cc6c2721c9`. This is a published verification
branch only, not another delivery PR, a GitHub PR merge or a master change.

Conflicts involved module/test registrations and imports, plus adjacent legacy
directory replacement context. Kept every module and HTTP test call from both
sides. Kept #240's reviewed `UserListItem` projection and removal of obsolete
`hide_rates` code/test, plus #253's full canonical endpoint. The resulting entire
`server_fns/users.rs` is byte-identical to original `1eb13ec`, not an invented
resolution. Dedicated reader/DTO/DB/HTTP files match #253/#254 unchanged; editor
files match #250 unchanged; legacy directory/session/approval-label HTTP files
match #240 unchanged. The three source heads are verified ancestors. Formatting
`94586` passed with zero changes, clean worktree; push `41158` completed.

#253 exact-head tests/Clippy/SQLx session `18891` is now terminal PASS. All seven
directory DB tests and the registered-session HTTP suite passed. Application
unit result: 976 passed, zero failed, 11 inherited ignored; core189 passed,
zero skipped. Additional test binaries passed (7, 5, 1, 44, 35, 36, 45, 11,
5, 1 and 8 tests). No tests were waived or modified in this extraction.
Standalone browser/deployment checks still remain; cross-PR evidence cannot
silently replace the exact standalone-head gate. PR description/comment updated.

With #253's compilation finished, started full Nix check for #254 `f498c3f`
as `81722`; it restored the exact package from Numtide cache and is executing
the remaining checks. After `18891` finished, started combined tests/Clippy/SQLx
on integration `7a2d61c` as `24884`. Both results remain pending. #250 full gate
`46207` is still live; its browser now passes the prior New Project per-person
budget selection and proceeds through the remaining responsive cases. This
does not yet certify the complete browser or full Nix suite.

Next collect `46207`, `81722` and `24884`, inspect any demonstrated failures,
then close the standalone #253 browser/deployment gap and continue the remaining
original time-reader/editor/UI extractions. Keep these PRs draft and no merges.

### Standalone directory full gate completed

Previous iteration was progress: published the isolated cross-PR composition and
completed the directory tests/cache/lints. Re-polled the same live handles this
iteration rather than restarting any build. #250 `46207` completed its full
browser derivation successfully, including the previously failing New Project
selection; SQLx passed and its test derivation is now running. Combined `24884`
passed Clippy and continues live SQLx checking. Picker full gate `81722` is still
live. These are partial results, not full-gate passes.

Started `nix flake check -L --cores 1 --max-jobs 1` on unchanged #253 `3a37538`
as `2803`; it completed with exit0, rebuilding only current formatting while
recognizing available cached check outputs. The subsequent read-only local
`nix path-info` request `66589` exited1 because three cached outputs were not
yet present locally; this was not a failing browser or deployment test.
Explicit `nix build` of browser/e2e/e2e-oidc then PASSED, materializing all three
exact outputs from the signed Numtide cache:

- `/nix/store/gayvrydx0rbm7pgq9b50fyhiggbj2w61-horae-browser-checks`
- `/nix/store/l4vka8grd4j3m54xr921q95v8vkfbivl-vm-test-run-horae-e2e`
- `/nix/store/asch5fnxcb7hr3k644akjnal97cd642m-vm-test-run-horae-e2e-oidc`

This closes the exact standalone-head browser/deployment gap by verified cache
reuse, not a claim of fresh local execution. Working tree is clean at the same
head. PR #253 description/comment and source ownership row updated. Cross-PR
integration remains pending, so draft status and prerequisite/retarget conditions
remain unchanged. Next collect `46207`, `81722`, `24884`, then complete combined
browser/deployment verification and continue the unextracted original groups.

### Delivery index reconciled with completed evidence

Previous iteration was progress: #253's full native gate and exact signed cached
browser/deployment outputs were verified and published. This iteration updated
the primary delivery table, which still showed #250's old base/failure and
omitted #253/#254/the combined check branch. It now records current heads and
bases, completed checks versus live gates, and the preserved failure history.
Also corrected the original `dcf4ac8` ownership row: #249's full Nix `72947`
passed earlier, as already recorded in its delivery row and completion entry;
no old check was rerun or reassigned to a different head.

#254 `81722` has now passed all nine project-people DB tests, the inherited
seven directory DB tests and the combined registered-session HTTP suite. Its
application unit result is 985 passed, zero failed, 11 inherited ignored;
additional test binaries passed. The full gate is still live in browser checks.
#250 `46207` and combined `24884` remain live in their test phases after completed
Clippy/SQLx; neither is restarted or counted as a full pass.

Read-only next-boundary inventory reconfirmed the original time-reader lineage:
`4ac30fa` adds the scoped reader/DTO/DB and HTTP tests; `60f60f9` adds subject
discovery; `5faed76` binds page reads to requester/subject. Later `e1ddd9a`,
`48a6533` and `02c4245` modify the same DTO for consumer/command contracts, and
`75f13a1` modifies the reader for report composition. Do not transplant the final
whole files as the initial-reader extraction. `c80233b`'s inserted legacy
invoice-identity HTTP block and model change already belong to #242 (`43337fc`)
and its independently registered `time_entry_payload` test; preserve its
canonical fixture remainder without duplicating that legacy test. No new
reader branch, runtime code, schema or product behavior was changed this turn.

Next collect the same three live handles, finish combined browser/deployment
verification after its tests, then extract the remaining original reader/editor
and consumer groups with this ownership boundary. Full original hunk accounting
and final cross-PR acceptance remain open; no original PR or extraction is merged.

### Initial scoped time reader extracted; two full gates completed

Previous iteration was progress: the delivery index was reconciled and the
time-reader lineage isolated without importing later consumer changes. Confirmed
#216 remains merged at `02f7b58` before editing. Read the goal, constitution and
Rust/async/testing/Ponytail guidance; no new product decision or abstraction.

#250 `46207` completed with exit0 on exact `c727bc8`: full native Nix, including
browser, deployment and OIDC, passed. Application unit result: 982 passed,
zero failed, 11 inherited ignored; core189 and additional binaries passed.
#254 `81722` also completed with exit0 on exact `f498c3f`: full native Nix passed,
including its complete browser run and exact cached outputs where available.
Application unit result: 985 passed, zero failed, 11 inherited ignored; all nine
picker DB tests, seven inherited directory DB tests and real-session HTTP
assertions passed. Updated both PR descriptions, preserving old failure history.

Combined tests/Clippy/SQLx `24884` completed with exit0 on unchanged `7a2d61c`.
Started its full `nix flake check -L --cores 2 --max-jobs 1`; process1004924 was
confirmed live in `.worktrees/permission-readers-editor-check`. The launch
response was truncated before retaining its session handle; do not restart or
interrupt it. Collect exact derivation/log evidence after that process ends.
This combination does not yet include #255.

Created `.worktrees/scoped-time-reader`, branch `feat/scoped-time-reader`, from
review base `0117991`, and published draft
[#255](https://github.com/numtide/horae/pull/255) at unsigned commit
`d93e1af639db34b3d709f7b6917d459f31dfb885`. Scope is the initial `4ac30fa` reader,
not the final mixed Timesheet/report file. Four whole files (DTO, transaction
reader, 576-line/eight-test DB suite and 155-line session HTTP suite) are
byte-identical to `4ac30fa`; the 31-line endpoint is copied unchanged. Only five
module/fixture composition points are adapted. No directory/editor/picker
dependency is needed. No migration, UI, command or activation is included.

Restored the original16 SQLx additions. A static48-macro inventory identified
seven older descriptors reused by these source files but absent from this base;
all seven were copied unchanged from the `4ac30fa` tree before compilation.
An initial scanner misparsed the quoted raw SQL literal; corrected the scanner
and discarded that false positive, with no source query edit. Final extraction
is32 files/1473 insertions, including23 SQLx descriptors and the nine Rust paths.
No original source or local unpublished work was removed.

Bounded adversarial review checked session-derived authority, tenant joins,
own/managed/all scope before filters/limits, explicit non-financial projection,
descending exclusive pagination, actor/policy gates, revoke/cancel transaction
tests and sanitized errors. No critical/high finding in this boundary. Preserve
the query/page web-only lint expectations until the real UI consumer arrives.
Later discovery `60f60f9`, context `5faed76`, consumer/command/report changes remain
unextracted; legacy invoice payload cleanup belongs to #242, not #255.

Formatting `53706` passed with zero changes, and committed-tree formatting passed
again in full native Nix `40092`, which remains live on exact `d93e1af`; it has
restored the merged Crane release dependency artifact and started app builds.
Compilation/test/browser/deployment success is not yet claimed for #255.

Next collect #255 `40092` and combined full-gate evidence, correct any demonstrated
failures, then compose #255 and continue original subject-discovery/context and
editor/UI extractions. Retargeted gates and final original-hunk accounting remain
open. No extraction PR or original PR was merged or closed.

### Timesheet person discovery extracted on its real prerequisites

Previous iteration was progress: published #255 and recorded the completed
#250/#254 gates. Re-read the objective, AGENTS, constitution and required skills;
confirmed #216 merged at `02f7b58`. Polled #255 `40092`, still live. The previous
combined full check process1004924 was also confirmed live; neither was restarted.

Created review-only branch/worktree `integration/scoped-time-people-prerequisites`
at unsigned merge `40102aed68dde211cc4f0c13b83e6dda04c78133`, composing exact #255
`d93e1af` and #253 `3a37538`. Both ancestor checks passed. The two conflicts were
only HTTP and DB module registrations; retained both readers and both suites.
This is not a GitHub PR merge or delivery destination. Published the branch for
the next scoped diff; no editor API, legacy identity projection or project-team
picker dependency was added.

Extracted original `60f60f9` into `.worktrees/timesheet-people-discovery`, branch
`feat/timesheet-people-discovery`, unsigned commit `1552fdb`, published as draft
[#256](https://github.com/numtide/horae/pull/256). It uses #255's admission
transaction and #253's existing `PeopleCursor`, avoiding a duplicate cursor or
unnecessary abstraction. Scope is identity discovery under time-read authority,
not a new directory grant, UI, write capability, page-context contract or policy
activation. The original active-participant/retained-history behavior is intact,
including managed-project members who have no hours in the selected period.

Three whole DTO/reader/711-line DB-test files match `60f60f9` exactly. Eight DB
tests, the 29-line endpoint and 135-line HTTP addition are unchanged. The combined
HTTP file differs from the original only by the36-line legacy invoice-identity
block already owned by #242; no assertion from `60f60f9` was removed. Five SQLx
descriptors are original. Total owned diff:11 paths,1105 additions/eight deletions;
deletions are the original admission-helper extraction, not removed checks.
Specifications remain in #248 and no duplicate specification package was created.

Bounded adversarial review checked tenant/activity fences, canonical grants
versus legacy roles and directory authority, minimal identity projection,
scope-before-search/filter/page, stable cursor behavior, malformed parents,
unrelated-hour exclusion, revocation waits, cancellation and error sanitization.
No critical/high finding in this boundary. Formatting `44568` passed with zero
changes. Static inventory matched all61 SQL macros in the reader, discovery
tests and HTTP suite to exact descriptors, with no missing query. This does not
replace database schema/type verification.

Started tests/Clippy/live SQLx on exact `1552fdb` as `61768`; Clippy is currently
running after restoring Crane check artifacts. Derivations are
`1ac19fafdp98kvyrmjqlfma56cbbz9j4-horae-tests-0.1.0`,
`gmqf2f49p879pssv08wqaxprnzj3c809-horae-clippy-0.1.0` and
`l3y7narrmiyf92z0fzczpm086fr4k66b-horae-sqlx-prepare-0.1.0`.
No runtime/full-gate pass is claimed yet. #255 `40092` remains live after its
successful client build; no failure or terminal result was observed.

Recovered exact prior combined `7a2d61c` derivation identifiers by evaluation:
browser `pypjghkk1hhc8raifxrlrhk0p45dvj6z`, e2e
`r8x7fg3dy674cjxzsjqz7yizqknaib1d`, OIDC
`222bzhk6jjk8cxijg1vd3j9cip16qk2r` (all `.drv`). Its three outputs were not yet
valid when queried (`nix path-info` exit1); that observation is pending build
evidence, not a failed check. Preserve the live process and recover its exact
logs/outputs before claiming completion. Native resources remain sufficient
(51GiB available RAM and108GiB free disk); no cleanup performed.

Next collect `40092` and `61768`, recover combined full-gate completion, run the
remaining standalone/full integration gates, then extract original `5faed76`
requester/subject context and the pending consumer/editor/UI groups. Continue
original-hunk accounting; no original PR or extraction was merged or closed.

### Requester-bound Timesheet context extracted

Previous iteration was progress: published #256, its explicit combined base and
source ownership. Re-read the objective and applicable skills, confirmed #216
remains merged at `02f7b58`, and checked the clean published source base before
creating the next worktree. Original source worktrees remain untouched.

Published draft [#257](https://github.com/numtide/horae/pull/257), branch
`feat/timesheet-page-context`, worktree `.worktrees/timesheet-page-context`, at
unsigned commit `8e09e6088eb68d253ec9bbe7cfe033b38e5e019c` on #256 `1552fdb`.
This extracts original `5faed76`: requester/policy-bound page loads resolve an
active selected person and their scoped rows under one transaction. Explicit
legacy-own policy is preserved; canonical denial never selects legacy mode.
There is no connected UI, write command, policy activation or data migration.

Three whole DTO/reader/354-line DB-test files match `5faed76` byte for byte.
All six DB tests and the33-line endpoint/109-line HTTP additions are unchanged;
only the new test-module registration is adapted. The HTTP suite still excludes
exactly the36-line legacy invoice-identity block independently owned by #242.
Total owned diff is six Rust paths,713 additions and33 deletions; deletions are
original helper extraction and transaction ownership changes, not removed tests.
No new SQL descriptor is needed: static inventory matched all37 macro query
strings in the reader, context DB tests and HTTP suite to existing descriptors.

Bounded adversarial review checked session-derived identity, expected requester
and policy mismatch, active-subject lock lifetime, per-page reauthorization,
scope narrowing, legacy-own restriction without canonical fallback, foreign and
inactive subjects, date/cursor validation, archive races, cancellation and error
sanitization. No critical/high finding in this boundary. Historical DTO web-only
lint expectations remain until the actual Timesheet UI consumer is extracted.
Formatting `17124` passed with zero changes and source/whitespace checks passed.

Started Nix tests/Clippy/live SQLx on exact `8e09e60` as `63162`, currently in
Clippy after restoring Crane check dependencies. Derivations are
`v02bmlf0gh6plfblz53vv1ca9m1h0z15-horae-clippy-0.1.0`,
`w2k6hrywi6rf5pfpi4kajgns6wy3mpxi-horae-tests-0.1.0` and
`xi4ji136irf6wp9rcfc1pcb35barknp9-horae-sqlx-prepare-0.1.0` (all `.drv`). No
runtime/full-gate pass is claimed for this new head.

Existing verification progressed without restart: #255 `40092` passed release
server/WASM and Clippy and is now running browser checks. #256 `61768` passed
Clippy and live SQLx and is compiling the application test suite. The earlier
combined `7a2d61c` full-check process1004924 remains live; its package build log
reached fixup and the exact browser log had no result yet. None of these live
full gates is counted as complete.

Next collect `40092`, `61768`, `63162` and combined exact-output evidence; finish
standalone and cross-PR full gates, then extract the original connected Timesheet
consumer (`e1ddd9a` plus its relevant later fixes) and editor/UI groups. Retain
the original14-line removal of two DTO lint expectations with their real consumer. Original-hunk
accounting remains incomplete; no extraction/original PR was merged or closed.

### Combined reader/editor verification closed and preservation audit refreshed

Previous iteration was progress: published #257 and preserved its original
contracts/tests. This iteration performed read-only verification and inventory;
no runtime code or original worktree changed. Revalidated all35 extraction PRs
#219–#257 (excluding #229/#230 and merged priority #251/#252): all remain open
and draft at their recorded heads. Original #212 still has the same18 unpublished
paths; original recovery refs remain intact.

#256 `61768` completed with exit0 on exact `1552fdb`: tests, Clippy and live SQLx
passed. Application unit result:992 passed, zero failed,11 inherited ignored;
all additional test binaries passed. Started full native Nix `37414` on the same
head; it remains pending. #257 `63162` passed Clippy/live SQLx and core189 tests
and continues the application suite. #255 `40092` remains live in browser checks.
These partial results do not certify either remaining full gate.

The original combined full-check process1004924 terminated. Its exact browser
output was then valid; on unchanged clean `7a2d61c`, full native Nix `16434`
completed with exit0, reporting zero rebuilds and all checks passed. Verified
the browser/e2e/OIDC output paths against their expected derivations and read
their terminal logs: browser finishes permission history, deployment script
finished in80.80s and OIDC in26.01s. This closes that combination's full-gate
evidence without claiming the cache-verification invocation reran those tests.
#250/#253/#254 descriptions now reflect the completed integration; retargeted
gates and later #255–#257 composition remain separate requirements.

Read-only conservation comparison used the35 live GitHub PR heads and merged
master `ed558f6`, against original `db3935d` from `9301112`. At the same path,
106 ordinary files,520 SQLx descriptors and48 specification files have exact
final original blobs in at least one extraction or master. The212 ordinary,
301 SQLx and7 specification files without exact matches still require original
hunk accounting or pending extraction; intentional adaptations are not losses.
Compared with the earlier extraction set, #253–#257 add eight exact final
source/test files and40 exact final SQLx descriptors. These are conservation
lower bounds, not completion percentages or integration/behavior proof.

The audit disabled rename detection to include both sides of replacements:
1,240 physical changed paths include46 removed SQLx paths. Confirmed Git's
rename-aware view still has1,214 changes, with26 SQLx rename pairs and20 unpaired
deletions, explaining the count difference exactly. Preserve old-query removal
and new-query adoption together during the final cache reconciliation; no cache
was deleted or rewritten by this inventory.

Timesheet history establishes the next delivery order: `48a6533` and `02c4245`
define/implement person-bound commands before the final navigation/action
consumer `a0632a8` and weekly submission `b8b1c60`. The initial read consumer
`e1ddd9a` and later focus/person-switch/date-offset fixes (`68bbaae`, `e29f4d8`,
`5f7895c`) belong with the corresponding connected UI, not the server command
PR. Browser fixture changes from `2497dbe` cross the project editor and must be
accounted separately. The preserved delegated-command branch `a0ea691` already
contains patch-equivalent command commits; do not extract a second copy from it.

Next collect `40092`, `37414` and `63162`, finish #257's full native gate, then
compose the later readers and extract original person-bound commands before
their final UI consumer. This refines the preceding next-action order based on
actual commit boundaries; it does not reduce scope. Full hunk accounting and
remaining original editor/report/project/task/UI groups remain open. No merge,
closure, activation or real-data change occurred.

### Timesheet reader gates closed; person-bound command extraction started

Previous iteration completed combined reader/editor verification and refreshed
preservation accounting. This iteration closed all three later standalone
reader gates without changing their heads: #255 `40092` on `d93e1af`, #256
`37414` on `1552fdb`, and #257 full native Nix `72055` on `8e09e60` each
terminated with exit0 and all checks passed. #257 tests/Clippy/live SQLx
`63162` also terminated with exit0:998 application tests passed, zero failed,
11 inherited ignored; core189 and all additional test binaries passed.
The full #257 invocation reused available outputs and rebuilt its format check;
it is not evidence of independently rerunning every cached test. Cross-PR
composition with the editor, project picker and legacy writers remains pending.

Created isolated `feat/timesheet-person-commands` at #257 `8e09e60`, worktree
`.worktrees/timesheet-person-commands`. Extracted the original67-line contracts
from `48a6533` together with the implemented35-line endpoints, complete command
module and four test files from `02c4245`. Kept existing UI-only lint expectations:
their original removals depend on the later connected UI, not these endpoints.
Registered the original real-session HTTP tests with the existing harness.
No UI consumer, policy activation, migration or new behavior is included.

Static review confirmed existing requester/policy binding, active same-tenant
subjects, own/person/project/all write scope, both ends of project moves,
atomic bulk sets, approval/billing conflict boundaries, task eligibility,
owner-only terminal timer recovery, revocation/cancellation fences and
post-commit effects. Approval-covered editing remains explicitly incomplete
in the original implementation and must stay declared, not silently enabled.
The new path uses the existing organization SHARE and subject write barriers;
#241's legacy-writer changes are a composition check, not a missing symbol.

Copied38 original command SQLx descriptors. Static inventory of84 macro calls
found one older reused descriptor absent from the extraction base:
`d0fb58934201c864ceeb74648fc7516252742592f57cee657d5c81ca56c67621`
(`SELECT minutes FROM time_entries WHERE id=$1`). Restored it byte-for-byte
from the same original tree; no query was changed. Whitespace check passed.
Initial staging orchestration `8929` timed out in automatic approval before
execution; the permitted retry succeeded. Formatting `62818` passed with zero
changes. All44 whole command/test/cache files match their original blobs.
Published unsigned `d2b45cadecae68bbcb28dfd4250462bcbd194e41` as draft #258,
stacked on #257;47 paths,2844 additions, including39 SQLx descriptor paths.
Nix tests/Clippy/live SQLx `64524` is running in Clippy. Exact derivations:
`jdf7ladl6xv4z3dx03rd3qc4gh7n8s5a-horae-clippy-0.1.0`,
`869wwixbiws3vpq7qisbs4b77iq0lnsx-horae-tests-0.1.0` and
`4ybli4yj1nq6m0zxb1pwsz268bqxp5zx-horae-sqlx-prepare-0.1.0` (all `.drv`).
No passing executable result for #258 is claimed yet. #255–#257 descriptions
now record their completed full gates while retaining draft/retargeting limits.

Next collect `64524` and run #258's full native gate, then verify composition
with #241 and the other reader/editor deliveries before continuing connected
UI extraction. Preserve the explicit approval-editing limitation. Original
hunk accounting remains incomplete. No extraction PR was merged or closed.

### Combined Timesheet and permission verification started

Previous turn made progress: published #258 and completed the three standalone
reader gates. Reconfirmed #216 merged at `02f7b58` before changes. The original
#212 worktree still has the same18 unpublished paths; none was edited.

Created verification-only branch `integration/timesheet-permission-check` in
`.worktrees/timesheet-permission-integration`, starting at #258 `d2b45ca`.
Composed prior reader/editor verification `7a2d61c` locally as `faec57b`, retaining
both sides of three registration conflicts: models, HTTP harness and profile
test modules. Then composed #241 `7820f8d` without conflict as
`6da9981ebe9680b5c0d66bc0e408f09d060d56a7`. This branch is published for
reproduction only; no delivery PR, GitHub merge or activation occurred.

The combination contains #240/#241/#250/#253–#258 and their inherited
foundations. All three source heads are ancestors. Twenty-one selected complete
reader/editor/command/legacy-writer source and test files match their owning
heads exactly; registrations keep every suite. Formatting `90772` passed with
zero changes and the worktree is clean. Nix tests/Clippy/live SQLx `25306` is
running on exact `6da9981`, currently in Clippy; full native gate remains pending.
This composition evidence does not certify the remaining original UI or task
lifecycle groups.

#258 `64524` remains live on unchanged `d2b45ca`: strict Clippy and live SQLx
passed, core compilation finished and the application test build is in progress.
Do not restart it merely because a poll is quiet. Its full native gate has not
yet been launched.

Read-only dependency audit clarified the next boundaries. `b8b1c60` changes
the existing `submit_week` signature to require `TimesheetWriteContext` and
updates its actual page caller. Extract it with the connected Timesheet UI,
its124-line submission tests and97-line session HTTP tests; do not ship an
incompatible standalone endpoint or invent an alternate compatibility API.
The original still restricts this submission route to legacy-own context.

The command implementation at final original `db3935d` differs from #258's
`02c4245` source by exactly the later `0591407` task-activity addition: one
`AND ($5 OR pt.active)` query condition and49 lines of archival test. These
belong with migration0048 and the project-task lifecycle extraction, alongside
the matching changed SQLx descriptor. Keep that remaining ownership explicit;
#258 does not claim the final task-archival behavior. The other command files
and DTO have no later differences in that comparison.

Next collect `64524` and `25306`, launch their full native gates after targeted
checks finish, and retain exact-head evidence. Then extract the coherent
Timesheet UI/weekly-submission group from `e1ddd9a`, `a0632a8`, `b8b1c60` and
later focus/person-switch/date-offset fixes, preserving browser coverage and
keeping task lifecycle changes separate. Original hunk accounting, editor,
report, project/task UI and unpublished client groups remain unfinished.

The connected UI extraction must also preserve current master navigation work:
its shared navigation script already handles client drafts, while original
`a0632a8` adds Timesheet on top of permission-editor guards not yet extracted.
Apply the owned Timesheet condition without overwriting client behavior; preserve
the permission-editor additions for their owner. The original editor-navigation
test file is absent from #258's base and needs explicit test ownership/wiring,
not a blind whole-file replacement. Timesheet page and sidebar themselves match
the pre-consumer source base; route/admin-shell files have other changes to keep.

### Cancellation-test failure preserved and synchronization corrected

#258 old-head `64524` terminated exit1 on `d2b45ca`:1010 application tests
passed, one failed,11 ignored. All13 new command tests passed. The failure was
`sheet_holds_selected_activity_through_entry_delivery_and_releases_on_cancel`,
at the five-second archive completion timeout (`timesheet_context.rs:343`),
not an authorization assertion. Strict Clippy and live SQLx passed; the overall
gate failed. No unchanged rerun was used to discard this failure.

Inspected the pinned SQLx0.8.6 source in the actual Nix vendor tree:
`sqlx-core/src/transaction.rs` Drop calls `start_rollback`;
`sqlx-postgres/src/transaction.rs` queues that rollback behind the current query;
pool return pings/flushes it asynchronously. Aborting the Rust task does not
immediately cancel the blocked PostgreSQL statement. The test retained its
artificial ACCESS EXCLUSIVE table barrier after confirmed task cancellation,
making queued cleanup compete with both the5000ms statement limit and the
five-second archive deadline. No production-code change was needed.

Owner #257 commit `b30e3cda8ca6f076294735eb9c8e9c03b17bd56e` adds four lines:
after confirmed cancellation, release that artificial barrier so SQLx rollback
can complete. Every original assertion remains, including the observed lock
dependency before cancellation, cancelled-task result, bounded archive
completion and subsequent access denial. No timeout was increased, test
removed/ignored or grant/runtime behavior changed. Formatting `7799` passed
with zero changes. This is an explicit test-harness adaptation, not a claim
that the database query itself now cancels immediately.

Propagated the owner fix without rewriting history: #258 is now
`b0eacfdcc08f2d3e226cf3a3e16a9fb25913cef8`; verification-only combination is
`015dcd15f97c299fef0799041849f9ffbf8696c0`. All three heads are published.
The old #257 passes at `8e09e60` remain historical, not current-head evidence.
Descriptions for #257/#258 now expose the failure, correction and pending gates.
Old combined `25306` remains live on `6da9981` in application tests; do not
mistake its result for verification of `015dcd1` or restart it while live.

Fresh #258 tests/Clippy/live SQLx `98625` runs on `b0eacfd`. Full native Nix
#257 `40402` runs on `b30e3cd`. After collecting old combined `25306`, start
fresh combined checks on `015dcd1`; #258 still also needs its full native gate.
No corrected-head passing executable result is claimed yet. Continue the
coherent UI extraction only with these failures/limits preserved in the ledger;
the goal is still incomplete and no PR was merged or closed.

### Selected-person Timesheet UI extracted; corrected command tests pass

Reconfirmed the user-priority optimization PRs #251/#252 are merged at
`35dc414` and `ed558f6`; extraction merge authority has not changed.
Old combined `25306` terminated successfully on `6da9981`:1037 application
tests passed, zero failed,11 inherited ignored. Current corrected combination
`015dcd1` has its own full native Nix gate `64407`, still running. Do not
transfer the historical pass to that head. Corrected #258 `b0eacfd` targeted
tests/Clippy/live SQLx `98625` now passed, including the previously failing
cancellation test:1011 application tests passed, zero failed,11 ignored.
Its remaining full native gate is `7633`. #257 `40402` remains running.

Created isolated `feat/timesheet-selected-person-ui` at #258 `b0eacfd`,
then published unsigned `0dfea8b791aebb39056d308be1c92db7fa1e25fa` as
draft #259. Git reports29 paths,2441 additions and381 deletions (30 physical
paths when the cache rename is counted separately). This coherent group
owns the original UI and the inseparable weekly-submission caller/contract.
No CSS, migration, policy activation, original worktree or real data changed.

Nine complete files match final original `db3935d`: Timesheet page, four
helper/test files, scoped-time DTO, own-submission HTTP fixture and both
Timesheet browser suites. The original submission patch and124-line race
tests are carried without changing assertions. Five original SQLx descriptors
replace the obsolete organization-config query descriptor; the removed
version remains recoverable in Git. Other source is deliberately not copied
wholesale: route/admin-shell retain current client/audit behavior, navigation
retains Clients/Projects/Invoices guards, modal fixture retains current
project-list endpoint, and New Project keeps its newer client/focus assertions.

The common navigation harness is extracted with all assertion bodies retained:
its permission-editor scenario is reserved for that future UI owner; this
branch tests the already-present client scenario instead, plus invoice,
project and Timesheet. Browser runner retains every current suite and adds
navigation, modals, Timesheet errors and selected-person permissions only.
Readiness changes touch only the Timesheet endpoint in shared layout/menu/
style fixtures. Canonical fixture activation occurs only in the runner's
disposable database, never an existing instance.

Bounded adversarial source review covered complete-page identity checks,
no partial totals, selected-person remount/history, dirty/pending navigation,
command context and owner-only recovery, lock states, submission policy/
activity fencing, SQLx ownership and unrelated shared-screen preservation.
The unchanged original still does not implement canonical submitted editing
or weekly submission, and task-archival condition `0591407` stays with its
lifecycle migration. Existing mouse-only calendar interactions, placeholder
notes labeling and inherited icon conventions are not certified as accessible
or redesigned in this split.

Rust/testing/async guidance and Impeccable's audit applied; the latter prompted
explicit shared-surface and inherited-accessibility limitations, not design
changes. Static UI detector returned an empty finding list; no visual pass is
claimed from that. Navigation script `65546` passed all10 tests; formatter
`44182` passed with zero changes and whitespace/provenance checks passed.
Full native gate `48062` runs on exact `0dfea8b`. Later combined verification
must include this head; `64407` does not yet contain #259.

Next collect the live standalone and combined gates without restarting them,
fix only demonstrated extraction/compatibility faults, then compose #259 with
the other extracted permission work and verify that new head. Continue
remaining permission-editor/People UI, reports, project/task lifecycle/UI,
unpublished Clients and original-hunk accounting. No extraction PR was merged
or closed, and the goal is not complete.

#258 full native Nix `7633` subsequently terminated exit0 with all checks
passed on exact `b0eacfd`. It reused available derivations and rebuilt the
format check; do not report a fresh execution of every cached test. The
targeted executable pass `98625` above remains its new-head test evidence.
PR258's description now records both. #257 `40402`, combined `64407`
and UI #259 `48062` remain live; #259's WASM client build has passed,
which is not its complete release/server/browser gate.

Read-only preparation for the next independent UI group found that final
`admin.rs` also contains later task-catalog mutations (`TaskRateEdit` and
the requester-bound `create_task` signature). Those belong to the unfinished
task owner, not a blind People/editor copy. `ee16165` ties CanonicalPeople,
requester-bound dialog selection and shell/sidebar access together; the
existing reader/editor combination `7a2d61c` already contains #250/#253.
Review the editor/recovery/template files fully before extracting that
consumer, preserve legacy task behavior, and reconcile its shared navigation
guard with #259 in a later integration check. No editor UI edits made yet.

### People/editor extraction and two Timesheet dependency corrections

Published draft #260 at `98857cfa0defe009447c4cb02cce4d4a54aa2d7a`,
based on already-verified `7a2d61c`. The23-path consumer group contains
4809 additions and126 deletions; its bulk includes the1988-line Rust UI
fixture and570-line real-browser recovery fixture, neither shortened. Fifteen
whole files match final original `db3935d` by Git blob hash: all eight
People/editor modules, both DTOs, storage JavaScript, Rust UI fixture, two
browser recovery/storage fixtures and browser.nix.

Historical `ee16165` supplies admin/shell/sidebar wiring without later
task-catalog changes. Timesheet route-user props are excluded from this
independent base, not removed from #259; later composition must retain them.
Only permission-editor endpoint exports are exposed here. Shared navigation
retains current Clients and adds permissions; its tests retain original
assertions with the independent branch's client scenario in place of the
not-yet-present Timesheet scenario. Combined verification must cover all five.
No CSS, new dependencies, migration, activation or real data changes in #260.

Bounded source review covered directory paging/requester identity, stale
response rejection, preview/confirmation, tab/session/org recovery binding,
storage before mutation, exact retries, self-demotion acknowledgement cleanup,
template limits and loss-of-authority recovery. Legacy People/tasks remain
when policy is inactive; canonical UI never substitutes for server checks.
JavaScript gate `31361` passed16 tests; formatter `27533` passed with zero
changes before the restored original Nix hunk; whitespace/provenance passed.
Impeccable detector returned no findings, not visual certification. Full native
gate `66677` runs on exact `98857cf`; no executable Rust/browser pass yet.

#259 initial full gate `48062` failed at `0dfea8b` before browser scenarios:
only tests/browser was copied into the Nix store, while navigation tests load
application JavaScript by relative path. Release server/WASM and strict Clippy
passed, not the overall gate. Commit `455c155` restores the exact original
`4294aa3` browser.nix hunk, copying the application subtree so assets remain
beside tests. #260 independently carries the same shared prerequisite.

Corrected-head gate `91457` then passed navigation10/10, modal, error and
initial selected-person scenarios, but failed the original calendar assertion:
moving an entry one hour stored start255 instead of240. Inspection identified
the omitted original `5f7895c` two-line CSS dependency: nested event text was
the mouse-coordinate target instead of the event box. Commit
`4ce091923c1ce89fcd478a50a5ea96c1358f8189` restores exactly that hunk.
Selectors occur only in Timesheet; current Clients styles and resize handle
remain untouched. No assertion or expected minute changed. Fresh full gate
`75088` runs on that head. Both failures remain evidence, not discarded reruns.

#257 current-head full native Nix `40402` terminated exit0 with all checks
passed at `b30e3cd`:998 application tests, zero failed,11 inherited ignored,
plus remaining suites, browser and NixOS/OIDC. Combined `64407` at `015dcd1`
has passed1037 application tests with zero failed/11 ignored and advanced to
deployment gates, but is not yet a complete pass. It excludes both UI PRs.

Next collect `75088`, `66677` and `64407` without duplicate runs. Fix only
demonstrated extraction faults. After standalone verification, compose #259
and #260 with the wider combination, preserving all navigation scenarios,
Timesheet user route props and every existing browser suite; verify that new
head. Continue remaining report/project/task/Clients groups and complete
original-hunk accounting. Optimization #251/#252 remain the only authorized
merged priority deliveries; no extraction was merged or closed.

### Report readers extracted; earlier combination verified

Previous goal turn made progress: #260 and original #259 dependency corrections
published. Current combined gate64407 terminated exit0, all checks passed,
on015dcd15f97c299fef0799041849f9ffbf8696c0:1037 application tests, remaining
binaries, browser and deployment/OIDC. It excludes the later UI/report PRs.
Original #212 retains the same18 dirty paths; #216 reconfirmed merged before
edits. No originals, #208, real data or activation state changed.

Draft #261 bf452ddc260ac5d1a2427b8f115d767eae6500b2 owns75f13a1/a23804f
detailed reporting plus final detailed active-project/billability refinements.
Base #257 supplies unchanged admission; only original helper visibility changes.
Final reader differs only by deferring grouped registration. All733 lines/
13 DB tests match final original;122-line HTTP fixture preserves75f13a1 reader
assertions and exact a23804f totals/pagination additions. Export/access/group
assertions remain with their owners. DTO runtime matches detailed original;
historical pre-consumer web expectations remain until UI wiring.

Static inventory matched38 macros to35 original descriptors;19 absent
descriptors copied exactly. Format45906 and whitespace passed unchanged.
Owned diff29 paths/1512 additions/one deletion. Full native Nix37229 runs
onbf452dd; live schema/executable acceptance remains pending.

Draft #26266dbf0bcefd1dee781e9daf9683b40d6235533b3 stacks on #261.
Final grouped reader153 lines and DB fixture722 lines/13 tests matchdb3935d;
171-line HTTP reader fixture matches41ff137. Endpoint33 lines original,
group DTO web expectations retained until consumer wiring. All32 macros
match27 original descriptors;10 absent descriptors restored. Format73230 and
whitespace passed unchanged. Owned diff18 paths/1385 additions. Full native
Nix79456 runs on66dbf0b; no executable pass claimed.

Bounded report review covered scope before filters/aggregation, qualified
historical parents, distinct entity IDs, requester/policy denial, active actor/
grant locks, revocation/cancellation, strict cursors/enums, exact64-bit totals,
effective/frozen rounding and billability, empty/exhausted pages and one-statement
snapshots. No new dependency/schema/CSS or legacy endpoint replacement.
No high/critical finding in the inspected boundary.

Scoped Spec Kit analysis used #248's spec/plan/tasks and original report
contract. Initial prerequisite invocation resolved stale metadata to016 and
failed for its absent plan. Inspected resolver, then explicitly selected015
with SPECIFY_FEATURE_DIRECTORY in paths-only mode and independently checked
all three required files. No feature.json persistence, extension hooks or spec
edits. This is a read-only invocation adaptation, not an initial-command pass.

FR-006/007/008/010/018 and report-relevant SC-002/003/006 map to T201–T202,
T213–T215, T219–T220 and reader portions of T222/T235: eight scoped requirements,
nine relevant tasks, all mapped. No new ambiguity, duplication or constitutional
conflict found in this boundary. Mapping is not implementation completion.
T203 remains open; full candidates, exports, consumer, financial families and
governance/activation remain separate. No spec remediation required/performed.

#25975088 has passed release/Clippy and the exact Calendar create/move/resize/
reorder/delete assertions after restoring original CSS. #26066677 has passed
release/Clippy and permission recovery/history/storage browser scenarios.
Their complete browser and remaining gates are still running; no full passes.

Next collect75088,66677,37229,79456 without duplicate runs. Compose #259/#260
with verified015dcd1 after standalone results, preserving all five navigation
guards/tests and route props, then verify that head. Compose report readers
and extract matching exports/access/consumer next. Original4294aa3's New Project
dev-login isolation hunk stays with project-editor fixture ownership alongside
later permission-label changes. Reports, project/task lifecycle/UI, unpublished
Clients and complete hunk reconciliation remain open. No extraction merge.

### Combined UI and report-reader verification

Reconfirmed #216 merged before changes. Priority #251/#252 are merged at
35dc414/ed558f6; no extraction PR was merged or closed. Original #212 remains
db3935d with the same18 dirty paths, preserved without edits.

Local verification branch `integration/timesheet-permission-check` now contains
#2594ce0919, #26098857cf and #261/#26266dbf0b on verified015dcd1. Integration
commits89d6377,be9e898,1a879615f35b285d29f3e7e9d60abc84e09c6fbc preserve
both parents; no original branch was rebased or rewritten. Composition proceeded
while remaining standalone gates ran, without treating those gates as passed.

Three UI conflicts were resolved by retaining all five navigation kinds and
both original scroll-coordinate scenarios. Tests now exercise13 navigation
cases plus6 recovery-storage cases: pinned Nix run17002 passed19/19. The
subsequent indentation-only adjustment is included in the fresh full gate.
The runner retains the union of all29 suites, no duplicate or missing names;
permission recovery remains before database-mutating legacy browser fixtures.
Timesheet `user` route props survive in shell, sidebar and shell-test stubs.
The report HTTP registration conflict retains every check call from both
parents; no assertion body was removed or weakened.

Owned Timesheet modules/DTO/browser fixtures, People editor modules/browser/
component tests, and report reader/DTO/DB/HTTP fixtures match their respective
extraction heads exactly. Only shared integration points changed. Format31039
passed with zero changes; whitespace passed. The targeted Impeccable shell
detector returned exit0 without diagnostics, not a visual or accessibility
certification. Full native Nix88592 runs on exact1a879615; not yet a pass.

Standalone #26066677 terminated exit0, all checks passed on98857cf: release,
strict core/server Clippy, complete browser, SQLx,1011 application tests (zero
failed,11 inherited ignored), remaining component suites and deployment/OIDC.
#25975088 passed release/Clippy/browser/SQLx,1032 application tests (zero failed,
11 ignored) and remaining suites; deployment checks now run. #26137229 passed
release/Clippy/browser/SQLx and starts application tests. #26279456 passed
release/Clippy and continues browser checks. Only #260 is a complete new pass.

Next collect75088,37229,79456 and88592 without duplicate builds; record
terminal results against these exact heads. Continue the original report export/
access/consumer extraction, then project/task and unpublished Clients ownership,
and finish hunk reconciliation. The combined branch is only a verification
artifact, not a proposed broad delivery PR or authorization to activate policy.

### Scoped spreadsheet extraction published

Previous turn was progress: published UI/report integration1a87961 and recorded
#260's completed gate. This turn #25975088 terminated exit0, all checks passed
on4ce0919 (1032 application tests, zero failed,11 inherited ignored, remaining
suites, browser and deployment/OIDC). #26137229 also terminated exit0 with all
checks passed onbf452dd. No repeated build or extraction merge.

Created prerequisite worktree/branch `integration/scoped-time-export-prerequisites`
at e8b95cc5365817d4998492b913ec6f5cf322d3e3 by composing #261 with #247.
One HTTP registration conflict retained every original module and check call.
This base has no independent acceptance claim; its combination is exercised by
the dependent full gate. It is not a delivery PR or merge target.

Draft #263 at8cc11c30757e177846530a0544f6808e337d792c owns cbc78a8's bounded
XLSX materialization and captured-scope release authorization. Original613-line/
11-test DB fixture is byte-identical. Original72-line HTTP addition coexists with
#261's totals/pagination assertions. Snapshot test adaptation exactly matches
cbc78a8: either a coherent earlier snapshot or a later size rejection is valid;
the size guarantee and subsequent rejection assertion remain enforced.

The231-line export reader differs from historical cbc78a8 only by the final
original active-project/billability SQL predicates and bindings. Its query DTO
already contains these fields; the legacy URL adapter initializes their existing
defaults. Full shared URL/requester/policy parsing remains with CSV/consumer work.
No UI, dependency, migration, real-data mutation or canonical-policy activation.

Review covered tenant/session binding, fail-closed policy/catalog facts, scope
before limits, exact minutes, private-field exclusion, source identity capture,
rendering without authority locks, fresh post-render revocation, empty exports,
actor commit/rollback races and cancellation. No new high/critical boundary
finding. SQL inventory:38 macros/26 unique queries;11 absent cache descriptors
copied byte-for-byte fromdb3935d. One obsolete original size-query descriptor
removed after checking no remaining Rust source contains its query; recoverable
from Git. Owned diff20 paths/1269 additions/103 deletions.

Formatter37857 changed only the combined HTTP module ordering. Initial source
commitcf7c07d omitted that tracked formatting delta; follow-up8cc11c3 includes
it. Active gate48482 evaluated the formatted worktree before the follow-up commit.
Clean-head evaluation67972 proved all ten native check derivations identical to
the active gate, including package, tests, browser, SQLx, Clippy, formatting and
both deployment checks. This is input-identity evidence, not a completed pass;
do not restart an identical build. Both branches and draft are published.

Next collect79456,88592 and48482. Continue09bd15f CSV delivery on the proper
#249 prerequisite, then shared filters, grouped exports and report consumer;
retain all deferred original assertions. Projects/tasks, unpublished Clients and
complete hunk accounting remain required. #212/#217/#208 remain untouched.

### Scoped CSV and shared download filters extracted

Previous turn progressed by publishing #263. This turn #262 gate79456 exited0
with all compatible native checks passed on66dbf0b:1024 application tests,
zero failures,11 inherited ignored, remaining suites, browser, SQLx, Clippy,
format and deployment/OIDC. Its PR body now records the completed evidence.
Wider combination88592 passed browser and live SQLx and is running application
tests. XLSX48482 passed release/Clippy and is running browser checks.

Created prerequisite branch/worktree `integration/scoped-time-csv-prerequisites`
at4e0ed43368fd428538b1f8ee3dc84e6c1c8020c1 from #2638cc11c3 and #24971232dc.
The sole limits-module conflict retained both time and project registrations;
automerged HTTP registry retains CSV routes and all reader/XLSX calls. No
independent base-gate claim. It is a review base, not a delivery merge target.

Draft #26455d362b8aa811a02a9e1f944e2625ab6b52fff0e owns09bd15f and93aaa68's
CSV delivery and shared URL filters. Source adaptation retains the final193-line
native cursor, including existing active-project/billability DTO predicates.
The URL adapter keeps their legacy false/Any defaults; later URL controls,
expected-policy binding, grouped exports and consumer tests retain other owners.
Existing #222 native stored-row decoding is reused, not duplicated.

Exact-source comparisons passed for the native cursor,222-line delivery module,
623-line/10-test DB fixture,149-line/five-test parser fixture and238-line
CSV/XLSX HTTP filter fixture. Historical HTTP context was adapted only to retain
#261 full-period totals; no existing pagination, XLSX or legacy assertion removed.
Shared XLSX authorization helpers match the original09bd15f refactor.

Review covered captured DECLARE authority versus current release authority,
strict native-state validation before empty sentinels, tenant-qualified parents,
source reassignment/deletion, exact effective minutes and billability, metadata
byte limits, pending-context revocation and lock-free capacity waits. Five
filter dimensions narrow rather than grant authority; malformed/ambiguous keys,
download cursors, incomplete bindings and switched identities remain denied.
No new high/critical boundary finding from source review; executable evidence
for this head remains pending. No UI/schema/dependency/real-data/activation change.

Scoped SQL inventory found47 macros/31 unique queries, with every macro parsed
and matched to its descriptor.17 absent descriptors were restored byte-for-byte
fromdb3935d; two obsolete native-cursor descriptors were removed only after
confirming their SQL no longer appears in remaining Rust sources, recoverable
from Git. An initial whole-tree heuristic did not handle concatenated macro
queries; it was not treated as authoritative or used for unrelated deletions.
The scoped inventory is complete; live SQLx remains the executable gate.

Formatter31607 and whitespace checks passed without edits after staging all
new files. Commit55d362b is clean and both branches are published. Full native
Nix gate40286 is running on that exact head; do not claim completion or restart
without changed evidence. Existing tasks T207–T212 trace this extraction;
T203, grouped delivery, pickers and financial reporting remain incomplete.

Next collect88592,48482 and40286 without duplicate builds. Extract grouped
XLSX/CSV and their preserved HTTP assertions, then the Reports consumer on the
actual reader/export foundations. Continue project/task/Clients extraction and
complete original-hunk ownership. No extraction merge or closure occurred.

### Report access prerequisite and grouped downloads published

Previous turn was progress: #264 and its conservation record were published.
This turn reconfirmed #216 merged at02f7b58 before edits and polled the same live
gates rather than restarting them. Full composition88592 on1a87961 has now
exited0 with all compatible native checks passed:1098 application tests,
zero failed,11 inherited ignored, remaining suites, browser, SQLx, Clippy,
format and deployment/OIDC. This combines Timesheet/People/readers but excludes
later exports and the future Reports UI; do not expand that acceptance claim.

Grouped source inspection exposed a real prerequisite: its strict transport
requires TimeReportPolicy and expected-policy download binding from7266abb.
Extracted this backend boundary first as draft #265 at
f436a29bdd5bfaacdd05a6b00b6f9e680699e9f3, based on #26455d362b.
Original access endpoint/helper/DTOs, source-policy admission and parser/HTTP
tests were preserved. The complete parser and registered-session fixtures
match7266abb exactly; query/page web lint expectations remain until their
actual consumer exists. Nine files,256 additions/eight deletions; no SQL change.
All12 inspected macros map to11 existing descriptors. Format45050 passed with
zero changes; full native Nix1825 is running on the clean published head.

Access review checked session-derived identity, optional requester mismatch,
fresh policy and grants, fail-closed corrupt/missing authority, information-safe
errors and stale links in both policy directions. Both formats reject mismatched
mode before source selection; inherited release checks reject later mode changes.
This is T216–T218's backend prerequisite, not UI/browser completion or T203.

Composed #265 with #262 in isolated prerequisite worktree/branch
integration/grouped-time-export-prerequisites atda493f6. The one HTTP-fixture
conflict retained all access/mode/export-filter assertions and the grouped
reader call; module registration was organized without dropping either side.
This is a review base with no independent gate claim, not a delivery merge target.

Draft #266 at0eec1a478e2d04068a923fb5a36d181266111638 extracts ca170c0/2b59b58's
grouped CSV/XLSX transports on that base. Eight whole-file comparisons passed:
handlers/renderer, final original XLSX/native cursor SQL, seven XLSX DB tests,
nine CSV DB tests, streaming coordinator, shared delivery authority and the
41ff137 multi-filter HTTP fixture. Both original production routes and their
HTTP harness registration are retained. Source SQL includes final original
active-project/billability predicates to match the inherited query DTO; the
later URL controls, extra fixtures and UI/browser links retain separate owners.

Grouped review checked distinct IDs despite identical labels, scope before
aggregation, exact integer totals, one-snapshot workbook payload/context bounds,
captured pairs after source edits/deletion, canonical-only admission and empty
sentinels. Streaming checks at most128 pairs at once under one group-wide
authority lifetime, reserves output before gates, and sends only after the last
context and successful gate release. Native metadata/Unicode byte weighting,
cancellation and more than10,000 groups/entries retain their original tests.
Reviewed against grouped contracts in csv-exports.md and time-reports.md.
No new high/critical source-boundary finding; runtime verification is pending.
No policy activation, schema/dependency/UI change or real-data mutation.

SQL inventory34 macros/28 unique queries;18 absent descriptors restored exactly
fromdb3935d. Formatter72873 passed with zero changes; whitespace/source checks
passed. Published owned diff34 files/2039 additions/13 deletions. Full native
Nix40587 runs on clean0eec1a4 and includes both prerequisite branches. It does
not certify consumer/browser integration or later filter transport. T221 remains
partially owned by the future UI; T203 remains open.

#26440286 passed its release build and entered Clippy. #26348482 passed release,
Clippy/browser/live SQLx and is running application tests. #2651825 and #26640587
are building their release applications. Original #212 still has the same18
unpublished paths; #212/#217/#208 were not modified or merged.

Next collect48482,40286,1825 and40587. Extract the remaining shared
active-project/billability URL controls and their original tests, then the Reports
consumer on the reader/export/access foundations. Fold these into the wider
composition only after preserving its existing fixtures. Continue project/task
and unpublished Clients boundaries and complete original-hunk accounting.

### Shared result-filter transport and fixtures published

Previous turn progressed by publishing #265/#266 and recording the successful
Timesheet/People/reader composition. This turn reconfirmed #216 merged and the
four pending gates live. #26348482 subsequently exited0 with all compatible native
checks passed on8cc11c3:1064 application tests, zero failed,11 inherited ignored,
remaining suites, browser, SQLx, Clippy, formatting and deployment/OIDC.
The previously recorded ten-derivation identity proof binds this run to that
clean published head; no identical build was restarted.

Created isolated worktree/branch feat/time-report-download-filters on #266.
Draft #267 at e029a892e228a076887d594750d48fff2fa60b9d owns the remaining
de8f9ad/db3935d URL-filter transport: optional strict active_projects_only and
the closed all/billable/non_billable selection, forwarded into already extracted
query fields and SQL. No production SQL, UI, CSS, schema or dependency change.
No activation or real-data mutation.

Preserved the original347-line/four-test active-project fixture and378-line/
three-test billability fixture byte-for-byte. They exercise detailed and grouped
CSV/XLSX, all four dimensions, empty-result authorization, archived client/task
independence, effective project/task billability, invoice-linked frozen zero
rounding, private-field exclusion and source snapshots after edits/archiving.
The final full HTTP filter fixture and grouped handler/parser file are exact;
the detailed parser fixture differs only by the two-line Project-export test
registration retained for that separate owner. No existing test weakened.

Source review checked strict default/duplicate/invalid query decoding, including
flattened grouped URLs, and traced the forwarded fields through each export's
existing predicates before aggregation and size limits. T222/T235 contracts
apply to this transport boundary; T236/UI resource keys and browser interactions
remain with the Reports consumer. No new high/critical source finding; executable
evidence for this head remains pending.

SQL inventory29 macros/26 unique queries;10 absent test descriptors restored
exactly fromdb3935d. Formatter23013 and whitespace checks passed with no edits.
Owned diff18 paths/1121 additions/two deletions, predominantly original fixtures.
Branch and draft are published; clean-head full native Nix51012 is running.

#26440286 has passed release/Clippy and is exercising browser checks;
#2651825 passed release and is running Clippy; #26640587 is building release.
Next collect these handles and51012 without duplicates. Extract the full
original Reports consumer, including its component/browser fixtures and filter/
group/expanded request keys, on the now available reader/export/access/filter
foundations. Then verify wider composition and continue project/task/Clients
ownership and the complete original-hunk audit. No extraction merge or closure.

### Reports consumer and wider composition published

Draft #268810ce57 preserves the remaining ordinary Reports consumer on #267:
three scoped modules, authenticated mode-gated route,15 original component
tests and the538-line disposable Chromium fixture. Eight paths,2566 additions/
43 deletions. No shared CSS, SQL, cache, schema, dependencies or activation.
The three modules, browser fixture and DTO are byte-identical todb3935d.
Route and test stub retain the existing no-argument project-tag API; only that
future Project-reader signature is deferred. All test assertions are preserved.
This assigns the remaining Reports UI/browser hunks from7266abb,41ff137,
e2e66fb,ca170c0,2b59b58,ecac66b,de8f9ad anddb3935d to #268; each already
extracted backend/specification owner remains unchanged.

Source review traced access-before-mount, pinned requester/mode, exact ready
resource keys including every filter/context, stale-response exclusion,
integer totals, escaped labels and full-period bound downloads without cursors.
Existing design contract explicitly requires incumbent shared components and
no shared-CSS changes, not copying the report-builder prototype. Keyboard,
responsive and theme assertions are preserved, not claimed executed yet.
T203 full candidate discovery and financial-family requirements remain open.
Formatter63610 passed without edits; whitespace and browser syntax52651 passed.
Clean-head full native gate50800 is running; the draft states pending acceptance.

Created isolated integration/reports-permission-check atb7836d7, combining
previously verified1a87961 with #268 and its complete export foundations.
Resolved only HTTP module and browser runner registration conflicts by retaining
both sides; all30 unique browser suites remain, with permission recovery and
Reports before legacy fixtures. Dedicated Reports and Timesheet/People source
and tests match their parents exactly. Formatter3606 and shell syntax passed.
Full native25958 is running; no identical check was restarted. This branch is a
verification artifact, not another delivery PR or authorization to merge.

Next collect40286/1825/40587/51012/50800/25958 without duplicate builds and
record actual outcomes. Continue Project reads/editor, task lifecycle/consumers,
unpublished Clients preservation and complete hunk accounting; the overall
separation goal is not complete. Ledger state through #267 was published
at18489b3 before this iteration. No extraction PR merged or closed.

### Conservation refresh and Project extraction boundaries

Previous iteration was progress: #268 and the wider composition were published.
Reconfirmed #216 merged at02f7b58; #212db3935d and #217dd141c5 remain open/draft,
and #20848a4156 remains open and untouched. Original tracked edits still compare
identically to preserved snapshotd364270; tar comparison of the six original
untracked files against the private backup also passes.

Read-only conservation audit used the live heads of46 open extraction PRs
(#219–#268 excluding #229/#230 and the merged build-only #251/#252), plus
mastered558f6. Compared Git blob IDs against the raw, no-rename9301112..db3935d
change set:373 non-SQLx paths and867 SQLx paths, including46 removed descriptors.
Exact final blobs occur in at least one candidate for160 ordinary/tooling files,
48 specs files and640 surviving SQLx descriptors. Compared with the earlier
35-PR audit this adds54 ordinary and120 descriptor matches. The158 ordinary,
seven specs and181 surviving descriptors without a whole-file match still need
hunk/equivalence ownership; they are not proven lost or absent. These counts
are conservation evidence, not test completion or a completion percentage.
Historical cache removals and integration-only branches are not credited as
delivered exact blobs by this audit.

Project boundary inspection traced2497dbe and2631186 against both the original
final tree and current extracted code, including the full project-read contract:

- Editor2497dbe: preserve its catalog/protected-field/save transaction, bound
  form and original tests as an editor responsibility. Existing #254 already
  carries the delegation/writer foundations; rate evaluator and project
  management modules compare exactly to the original final blobs even where
  commit ancestry differs. Do not duplicate those modules or infer a missing
  dependency solely from ancestry.
- Reader2631186: keep overview/details/tags/team/spend and configured-budget
  projections coherent with optional money and the summary/breakdown DTOs.
  Budget summary must be computed before private-detail filtering, and the
  trusted alert calculation must retain its unfiltered service semantics.
- Reader delivery: project CSV/XLSX and compatibility list/count/direct-ID
  consume the same row/field rules but retain their own bounded release checks.
  Consumer signature changes must include every caller and component stub,
  including the two Project-tag adaptations intentionally deferred in #268.
- Later task activity/canonical catalog work remains separate. The original
  Project-read fixture at2631186 does not yet register the later406-line task
  fixture; its final parent changes only that registration and helper visibility.
  Preserve these later hunks for their task owner rather than silently pulling
  task lifecycle into the project-reader extraction.

Concrete preservation hazard: replacing project_creation.rs wholesale with
db3935d would revert master's shared client-profile/default-rate validation to
the older inline checks. The2497dbe editor patch does not change that function;
extract its owned hunks and retain master's client validation. No runtime edit
was made during this inspection.

All six existing full-gate handles remain live. #26440286 has now passed
1095 application tests, zero failed/11 inherited ignored, and the remaining
application suites; deployment checks are still running. #2651825 and #26640587
are building/running tests; #26751012 passed release/Clippy and is in browser;
#26850800 and composition25958 are building. No restarted or duplicate builds.
Next extract the existing Project editor on the actual shared foundations,
collect these terminal outcomes, then finish Project reads/delivery, tasks,
unpublished Clients classification and complete hunk-level accounting.

During final collection40286 completed successfully on #26455d362b:
all compatible native checks, including browser, SQLx, Clippy, formatting,
server/WASM and deployment/OIDC. No pending local gate remains for that head;
retargeted remote acceptance and wider final composition are still separate.

#26850800 terminated after successful production server/WASM build: Clippy
found CurrentUser/UserListItem unavailable in its standalone component fixture.
Those DTOs belong to the identity/directory extractions, not Reports runtime.
The existing branch APIs both return User. Commit aef180f changes only those
two test-stub type references to the actual base API; all15 tests/assertions,
production modules and browser fixture are unchanged. Formatter26328 passed,
fix pushed, clean-head full64831 launched. Do not credit50800 as a pass.
The wider b7836d7 composition has the projected DTOs and continues under25958;
its old test fixture is not silently claimed identical to the corrected #268.
Reconcile this test-only difference in the next wider composition, preserving
the actual API types and every assertion. No duplicate live check restarted.

### Project editor extraction started; Reports access gate complete

Confirmed #216 remains merged at02f7b58. Created isolated
feat/scoped-project-editor from the fully verified shared-reader/editor
composition7a2d61c. Its editor source boundary is2497dbe, not the later final
tree containing task lifecycle changes. Preserve current client validation and
reuse the already-extracted rate/delegation implementations.

Full native1825 completed on #265f436a29 with exit0 and all compatible checks
passed, including deployment/OIDC. Updated its PR verification; no extraction
merge or closure. Other architectures were not executed. The current265 head
was reconfirmed before updating its body.

Next finish the Project editor extraction and its original regressions, collect
40587/51012/25958/64831, and continue the outstanding reader/task and hunk
accounting work. No check is restarted while its existing handle is live.

Published Project editor extraction #269 atd5dc1da on
feat/scoped-project-editor, based on7a2d61c. Its82 changed paths comprise
34 source/test/runner paths and48 original SQLx descriptors. The isolated
responsibility is bound editing, protected-field intent and atomic manager
selection; no Task lifecycle or policy activation is included. Original
2497dbe database/HTTP/component/browser regressions are retained, including
the4294aa3 single-DEV_LOGIN-administrator fixture correction. The scoped query
inventory parsed282/282 macros and restored only missing descriptors after
query/hash equality checks. No descriptor was deleted by a global heuristic.

Two master-preservation adaptations are explicit: retain the shared client
validation in project_creation.rs and the NewProjectForClient/client_context
flow (saved drafts retain precedence) in pages/new_project.rs. HTTP module and
browser registrations retain all existing suites. Detail-navigation fixture
exports retain the existing client models while adding the editor DTO imports.
Shared CSS, core rate/delegation code, schema and policy state are unchanged.

Formatter23849 passed with only module-order adjustments; whitespace, shell
and browser syntax checks passed. Commit/push60912 and draft creation5072
completed. Full native88044 runs on clean published d5dc1da. Runtime source
inspection is recorded, but complete adversarial fixture review and wider
composition are still pending; do not infer executable success from copying
the original regressions. The draft states these limitations.

Full native40587 also completed successfully on #2660eec1a4 with exit0 and
all compatible checks passed, including deployment/OIDC. Its PR body now
records that outcome after reconfirming its live head. Full51012 (#267),
25958 (b7836d7 composition),64831 (#268aef180f) and88044 (#269d5dc1da) remain
live; #268 production release and Clippy have passed. No duplicate check,
extraction merge or closure. Next review the retained editor fixtures, collect
these checks, then continue Project reads/delivery, tasks and hunk accounting.

### Project reader extraction and Reports composition verification

Full native51012 passed on #267e029a89; its PR body was updated after
reconfirming the live head. Full native25958 completed with exit0 and all
compatible checks passed on the wider Reports composition b7836d7. This
composition still uses the original #268 fixture types appropriate to its
identity foundation; reconcile the later standalone aef180f test-only
adaptation when composing again. Other architectures were not executed.
The #26864831 and #26988044 gates remain live; neither was restarted.

Completed scoped source review of #269's original canonical-field, manager,
catalog, concurrency, HTTP, component and browser fixture changes and runtime
boundary. No new critical or high finding was identified. Updated the draft
description, without claiming an independent review or full executable pass.
Its release and Clippy stages have passed; remaining checks are pending.

Created feat/scoped-project-reads in .worktrees/scoped-project-reads from the
shared reader/editor foundation7a2d61c. The original reader requires the
existing financial snapshot boundary #2320bb5721 for legacy fee reads.
Composed that prerequisite locally as c6e96f4 on
integration/project-read-prerequisites. The only manual merge resolution
united HTTP fixture registrations and calls, retaining both parents' suites.
No GitHub merge occurred; this new composition is not yet verified.

Extracted the2631186 ordinary reader, budget summaries, row projections,
requester-bound overview/detail UI and original database/component fixtures.
Preserved master's client-linked project route and passed initial_client into
the keyed overview child. Retained the shared is_admin helper because Clients
still consumes it. Adapted Client detail's existing spend caller and optional
amount display: withheld money remains unavailable, never fabricated zero.
Preserved all Client navigation fixtures and their deferred responses while
adapting the shared Project fixture; did not replace it with the older file.
The original new reader database fixtures and two component submodules were
read and retained without adding the later Task lifecycle fixture.

These reader changes are still local and uncommitted, not build-ready or
published. Whitespace checks pass, but HTTP/browser fixtures, SQLx inventory,
formatting, full native verification and remaining review are outstanding.
Project CSV/XLSX and Harvest-compatible list/count/direct-ID release boundaries
remain separate extractions. Next finish these reader regressions and caller
checks, publish the bounded draft, collect the two live gates, then continue
delivery, Tasks, unpublished Clients classification and full hunk accounting.

### Project reader draft published; editor fixture isolation corrected

Reconfirmed #216 merged at02f7b58. The preceding iteration was progress:
reader source, original fixtures and compatibility adaptations were preserved,
and the ledger was published as3d61f03. No unchanged check was restarted.

Full native64831 completed with exit0 and all compatible checks passed on
#268aef180f. Updated its PR body after confirming the live head. Other
architectures and retargeted GitHub acceptance remain separate.

Full native88044 on #269d5dc1da failed in the existing client-context browser
fixture after production builds, Clippy and earlier suites passed. Its global
draft oracle parsed multiple creators' rows as one JSON document. The retained
new-project-permissions fixture intentionally creates another user's draft.
Commit fdae71d scopes the oracle to the unique active DEV_LOGIN administrator
and organization. This matches the real login selection and the per-creator
draft constraint, retains every assertion and leaves other creators' drafts
untouched. No production code changed. Formatting traversal/whitespace passed;
the JavaScript file is not covered by treefmt, so traversal is not a JS syntax
proof. Published the fix and started full native87701 on the clean head;
updated #269 to record the failure and pending rerun, not a pass.

Published draft #270 at0723bb3, based on
integration/project-read-prerequisites c6e96f4. Its89 changed paths comprise
40 source/test/runner paths and49 SQLx descriptors. Retained the original
2631186 HTTP fixture and registered it alongside all existing checks.
Adapted the original browser fixtures to the bound overview and shared
task/team editor, retaining their failure/retry, rate, bulk-selection and
keyboard assertions. The new scoped Project-read fixture runs first; all23
previous suites remain. Modals and responsive fixtures preserve their actual
base's Timesheet and Client APIs rather than importing unrelated changes.

The scoped query inventory parsed259/259 macros after explicitly handling
Rust escaped line continuations. Restored49 missing descriptors only after
query/hash equality against2631186; no heuristic deletion. Core budget,
project DTO, backend reader and budget implementation match the original
source exactly. Formatting31397 passed with only component-fixture import
ordering; whitespace checks passed. Commit/push37975 and draft creation93585
completed; full native23221 runs on clean0723bb3. No executable acceptance
or final adversarial sign-off is claimed yet.

Project export links retain the original expected-requester parameters, but
CSV/XLSX server release guards and Harvest-compatible readers remain separate,
unextracted responsibilities. The draft explicitly records that limitation.
Do not activate policy or infer finished delivery enforcement from these links.

Next finish the scoped reader adversarial review, collect87701/23221, correct
any concrete failures, then extract project delivery/compatibility readers,
Task lifecycle/readers/UI, classify unpublished Clients and complete hunk-level
ownership and wider composition. No extraction PR was merged or closed.

### Project delivery extraction and concrete verification corrections

Full native23221 on #2700723bb3 failed Clippy after successful server/WASM
builds: the detail-navigation fixture re-exported an unused project_managers
module. Commit ed964fc removes only that unused module/import. Formatting88589
and whitespace passed; commit/push22660 completed. Full native56170 runs on the
published corrected head, and the draft now records both the failure and rerun.
Scoped reader review found no new critical/high issue in the extracted ordinary
read boundary: current organization/actor locks, strict stored policy/grants,
tenant joins, minimal labels, optional money, budget totals and original race
fixtures were inspected. This is not independent sign-off. Existing lifecycle
and fee operations still use their legacy role/snapshot gates; their canonical
authorization is explicitly outside this reader extraction and policy must
remain inactive. Wider UI/composition review and full acceptance remain pending.

Full native87701 on #269fdae71d passed the formerly failing client-context suite
and reached the history fixture. Its zero-receipt precondition failed because
the canonical editor fixture left four of its own receipts after deleting its
project. Commit8da2639 cleans only receipts joined by organization, actor and
request ID to that fixture project's edit requests, before cascade deletion.
It additionally checks that the original receipt snapshot is unchanged. The
history fixture and all prior assertions are retained. This cleanup runs only
inside the guarded disposable browser database, never a real database. Node
syntax93514 and whitespace passed; commit/push1458 completed. Full native89598
runs on the corrected head. The draft records this actual failure, not a pass.

Published draft #271b0cd080 on feat/scoped-project-exports, based on
integration/project-delivery-prerequisites abc7642. This base locally combines
#267e029a89 and #270ed964fc. Initial composition5f2eca0 resolved models, server
exports and two fixture registries by retaining both parents' registrations;
abc7642 then incorporates the reader test-import correction. No GitHub merge.
The base is an integration artifact, not an independent delivery PR.

The export extraction owns21 paths:16 Rust source/test paths and five original
SQLx descriptors. Source2631186 runtime and three new fixture modules are
preserved, with shared call sites receiving the monetary-context argument.
Requester binding rejects malformed/partial/mismatched identity without
selecting another actor. Canonical scope and money visibility are independent;
CSV source-state restoration applies to empty exports too. Workbook release
and CSV delivery after capacity waits recheck captured project/monetary access.
Retained nine canonical DB regressions, three parameter tests and legacy suites.
Scoped inventory parsed28/28 query macros and restored exact original cache
entries after query/hash equality; no heuristic cache deletion. Formatting26118
and whitespace passed; commit/push51395 and draft11523 completed. Full native82670
runs on the clean published head; live-schema acceptance remains pending.

Next collect56170/89598/82670 without duplicate runs, address concrete failures,
finish Project reader/delivery review, then extract the existing Harvest project
API list/count/direct-ID boundary. Created .worktrees/scoped-harvest-projects
on feat/scoped-harvest-projects from ed964fc for that bounded extraction; no
changes there yet. Tasks, unpublished Clients and complete hunk ownership remain
outstanding. No extraction PR was merged or closed, and no policy activated.

### CI repair priority and pending extraction verification

The user prioritized restoring Nixbot before further extraction work. Master
ed558f6 builds the application and passes its non-VM checks on both Linux
architectures, but Nixbot build138 fails both ARM VM checks before application
assertions: QEMU falls back to TCG and the test driver's fixed 300-second shell
connection timeout expires while the guest is still booting. The farm documents
TCG as intentional; no infrastructure changes or disabled checks are justified.

Draft #274, fix/nixbot-arm-vm-checks at cd26b56, adds a bounded serial-readiness
wait to the two existing VM tests. All application assertions remain unchanged.
Formatting and whitespace checks passed. Full native21618 and Nixbot build202
are pending; this is not yet a verified repair. No runner configuration, real
data or production application code was changed.

Before this priority change, draft #272 was published at bfa771d, based on
#270ed964fc. It extracts the original Harvest-compatible Project readers from
2631186: 16 paths, including ten original SQLx descriptors. The scoped query
inventory parsed21/21 macros and checked restored query/hash equality. Formatting
and whitespace passed. Full native46280 later failed the browser fixture
project-read-permissions.cjs at131: its errors array was nonempty. Diagnosis is
pending; compilation and Clippy success do not make this draft ready.

Draft #273 was published at571f3f5, based on #272bfa771d. It extracts original
task readers and their DB, HTTP and browser tests from f6e8bf1:18 paths, including
five original SQLx descriptors and the original three-line sidebar interaction
fix asserted by the browser fixture. Query inventory parsed58/58 macros, with
query/hash equality checked before restoring descriptors. Formatting, Node/shell
syntax and whitespace passed. Full native96635 remains pending. Canonical Time
commands are not in this base; activation still requires the wider composition.

Additional full native results are failures, not acceptance:

- #270ed964fc (56170) and #271b0cd080 (82670): new-project.cjs at1200 expected
  three options but received four. Fixture isolation needs investigation.
- #2698da2639 (89598): project-editor-permissions.cjs at145 expected two requests
  but observed one in the lost-acknowledgement retry scenario. The receipt cleanup
  has not received full-suite acceptance.

Do not rerun these unchanged heads or weaken their assertions. Next complete
#274's native and remote ARM validation, record its actual outcome, then return
to the concrete browser failures before further extraction. Existing original
branches/worktrees remain preserved; no extraction PR was merged or closed.

The initial #274 run exposed a second, test-driver-specific delay:
wait_for_console_text consumes one queued console line per one-second retry.
The guest had emitted readiness but the reader was still draining old boot
lines. Follow-up f869353 uses the existing bounded retry helper with
get_console_log instead; the readiness marker and900-second bound are unchanged.
Formatting and whitespace passed. Superseded native21618 was explicitly stopped,
not reported as success. Full native56305 and Nixbot build204 validate the new
head. The native OIDC VM completed its full assertions in21.47 seconds; the
remaining gates are pending. No application assertion or test was removed.

Full native56305 completed successfully on #274f869353: all native flake checks
passed, with the deployment/recovery VM completing in70.03 seconds. Nixbot204
is still pending and remains the required ARM evidence. Full native96635 on
#273571f3f5 failed in the existing new-project browser suite (expected three
project rows, received four); retain the draft and investigate isolation after
the priority repair. No unchanged extraction check is being restarted.

### CI repair follow-up and authorized branch refresh — 2026-10-07

Nixbot204 failed both ARM VM checks on f869353. The serial device dependency
timed out inside systemd after300 seconds, cancelling backdoor.service before
the driver's longer900-second readiness wait could help. Native checks passed;
this was not an accepted ARM fix.

A disposable local ARM VM using the exact cached guest kernel, initrd and
system closure reproduced the failure under TCG. /dev/hvc0 existed before
udev finished coldplug; systemd eventually marked it plugged after its job
had expired. Starting backdoor.service after coldplug immediately produced
the readiness marker. No real application database was accessed. The debug
VM was terminated after recording this evidence.

#274ccc2961 also sets DefaultDeviceTimeoutSec=900 inside the two test nodes.
Production service settings, assertions, supported systems and the overall
test deadline are unchanged. Full native97178 and Nixbot206 are running on
this correction; neither is yet reported as passed.

The user explicitly authorized merging #274 once the complete repair passes,
then rebasing the worked-on open PRs so their checks include the repaired CI.
No merge or rebase has occurred yet. Refresh stacks in dependency order,
preserve recovery refs and local edits, and use exact-head CI evidence after
each refresh. In particular #212 still has its18 original dirty paths; do
not overwrite them. This does not authorize merging extraction PRs or
continuing feature implementation ahead of the repair.

Full native97178 passed on ccc2961, including both VMs; deployment/recovery
finished in103.66 seconds. GitHub run37599025624 passed Flake Check and Format.
Nixbot206 failed before executing ARM checks: its scheduler reported no
connected, non-draining aarch64-linux worker after120 seconds. Both ARM VMs
and ARM formatting share this allocation failure; it is not evidence of a
test regression or a successful ARM correction. A single same-head Nixbot
rerun request through GitHub's documented check-run rerequest endpoint was
rejected with HTTP404 even though the same check is readable. It did not
schedule a new run.

One empty commit, #274a757709, triggers a fresh run without changing the
verified ccc2961 tree. Do not repeat empty commits if worker allocation fails
again; report the infrastructure blocker instead of bypassing ARM checks.

Nixbot reused failed build206 for the unchanged a757709 tree, so the empty
commit did not execute a new ARM run. The alternative check-suite rerequest
endpoint also returned HTTP404. GitHub Flake Check and Format passed on
a757709. A user-triggered Re-run on the
nixbot/nix-build check is now required to get fresh ARM evidence with the
available access. No CI result was overridden, no infrastructure configuration
was modified, and no PR was merged or rebased.

Read-only refresh audit: the52 extraction/ledger PRs #218–#273 all match their
published heads; only this ledger has current local edits. All other audited
extraction worktrees are clean. The read-only snapshot is saved locally in
`.scratch/pr-refresh-inventory-20261007.json`; re-fetch before any mutation.
Retain the57-open-PR inventory and dependency
base mapping when resuming; do not flatten shared integration bases into
unrelated feature diffs. Next action: validate the rerun of #274 on ARM,
merge only after complete acceptance, then perform the authorized refresh.

### PostgreSQL startup deadline in emulated VM checks — 2026-10-07

After the user requested another ARM diagnosis, direct Nixbot206 logs showed
new execution evidence despite GitHub still displaying the earlier failure.
ARM workers had become available:18 attributes succeeded and both VM checks
now reached the guest shell. The console device correction worked, but
PostgreSQL's120-second startup deadline expired during initdb. PostgreSQL
restarted successfully; Horae remained inactive because its dependency job
had already been cancelled. The worker-allocation blocker is superseded.

The user authorized fixing this. #2742d1486e sets only PostgreSQL
TimeoutStartSec=900 in both test nodes; TimeoutSec=120 still supplies the
unchanged shutdown deadline. Production modules, application assertions and
the overall VM-test deadline are unchanged. Source and whitespace review
passed. Full native43688 and current-head remote CI are running; no ARM
acceptance is claimed yet. PR metadata now describes the current failure,
not the obsolete allocation error. No merge or rebase has occurred.

Full native43688 passed on2d1486e, including both VM suites; the deployment
and import-recovery VM finished in129.29 seconds. The generated PostgreSQL
unit contains TimeoutSec=120 and TimeoutStartSec=900 as intended. Nixbot208
has completed18 of20 attributes and is executing the two ARM VM checks.
This is live validation, not the previous infrastructure blocker.

Nixbot208 succeeded on2d1486e for both Linux architectures, including both
ARM VM checks, after25m42s. GitHub Flake Check and Format also passed on the
same head. This supersedes the earlier blocked status: the test-only startup
correction is now fully verified. The user-authorized normal-queue merge of
#274 is being requested; confirm its actual merge before rebasing any stack.

#274 merged at11:43:02 UTC as8b3cc2577a3704cad28ee2e02a028fbf52668780.
Merge-group run37615829882 passed. The authorized branch refresh can now
proceed; this is not approval to merge any extraction PR.

### Post-repair stack refresh — 2026-10-07

Rebased all 52 extraction and ledger PRs in #218–#273 and their 18 shared
integration branches onto 8b3cc2577a3704cad28ee2e02a028fbf52668780. All 70
branches were published together with an atomic push and exact old-head
force-with-lease checks. No extraction PR was merged.

For every branch, the resulting tree exactly matches the original tree
combined with the repaired master. All 51 non-ledger PR diffs are byte-for-byte
unchanged at their respective merge bases, and every existing ancestor
relationship with a PR base is preserved. Historical integration conflicts
were resolved only after confirming equality with the original merge's
resolution. No application behavior was changed by this refresh.

Recovery refs remain under `backup/ci-refresh-20261007/`, with a verified
bundle in `.scratch/ci-refresh-before.bundle`. The local manifests
`.scratch/ci-refresh-plan.json`, `.scratch/ci-refresh-completed.log` and
`.scratch/ci-refresh-published.json` record original and published heads.
The original #208, #212 and #217 branches remain unchanged; #212 retains its 18
uncommitted paths. They are preservation sources, not refreshed extraction
branches. Root master and unrelated worktrees were not reset.

CI must validate the newly published heads. The successful #274 checks prove
the CI repair, not the correctness of every extraction. Existing browser
failures in #269–#273 remain unresolved until fresh evidence demonstrates
otherwise; those PRs remain drafts. Next action: inspect fresh-head results,
address extraction failures within their own scope, then continue the original
work-accounting and verification goal without adding features or merging
extractions.

### Project filter regression diagnosis after the refresh — 2026-10-07

Reconfirmed #216 merged at 02f7b58. The preceding iteration was progress:
#274 merged after passing both architectures and all extraction branches were
refreshed with verified tree and dependency preservation. No original branch
or unpublished Client work was modified.

On #270 at 18a2e4a, the existing native browser log identifies the fourth row
as the `Team recovery` project intentionally created by `action-errors.cjs`.
The `new-project.cjs` suite passes alone on the exact package
`8i4k07nmvmq6lx8xwxq2mp85lrl5zdkj-horae-0.1.0` (session 77388), establishing
that its fixed three-row assertion depends on suite ordering. This is not a
duplicate project produced by the lost-response creation retry.

Commit 2dbab02 in #270 changes only that test: capture the unfiltered project
identities and require both their exact count and identities after Reset
filters. Existing one-row tag filtering, empty search, disabled bulk actions,
creation replay, budget, report/export and invoice assertions remain intact.
The previously failing sequence `action-errors new-project` passes on a fresh
disposable database (81244). Node syntax and whitespace pass; treefmt does not
format this JavaScript file. Bounded adversarial review checked missing,
duplicated and substituted rows: the identity multiset comparison rejects
all three, unlike a count-only check. No production code, dependency or data
change was necessary. Full native gate 57812 is running on published 2dbab02;
neither complete acceptance nor downstream propagation is claimed yet.

The #269 canonical editor fixture passes in isolation on its exact package
`7r6v6zz9wq53cz7rmn8xmnycjx1fm6j2-horae-0.1.0` (1818), including the two
identical replay requests and unchanged revision. Its original failing log
still records one request instead of two. No speculative correction was made;
the intermittent failure remains unresolved pending stronger evidence.

Fresh remote evidence also prevents declaring ARM fully reliable across these
extractions. Nixbot build210 on #270's rebased 18a2e4a failed browser checks
and both ARM VMs. Its OIDC VM reached a listening Horae service around guest
second635 but never emitted the test-console connection marker within900
seconds. Its deployment VM did connect and reached the repeated-import
assertion, then exceeded wait_sql's90-second deadline waiting for `succeeded`.
These are distinct failures, not proof that PostgreSQL's startup correction
regressed. #272 build215 also reports failing ARM and browser checks; its
individual causes are not yet diagnosed. No limits or assertions were weakened.

Next inspect the live 57812 result, propagate the verified test correction
through its existing dependent stacks, and diagnose the two new ARM failures
before claiming reliable remote acceptance. Retain #269–#273 as drafts and
continue the original hunk-accounting goal; no extraction merge is authorized.

Full native 57812 failed in the project-only request audit, before the filter
suite: `directories` contained the legacy Timesheet's `list_clients` call from
the login landing page; `errors` was empty. The listener was installed before
login, and the old Timesheet explicitly mounts that directory resource. The
failure therefore depends on whether landing-page hydration starts before
the fixture navigates to Projects.

Commit 387f80f in #270 authenticates through the same dev-login endpoint using
the browser context's shared cookie store, with redirects disabled and the303
response and `/` destination asserted. No Timesheet is mounted; the directory
listener remains active before the first Projects navigation and none of its
assertions are removed or filtered. All project browser interactions, grant
revocations, inactive-account checks and requester-binding checks remain real.
Focused Chromium 8195 passes all three scenario groups; Node syntax and
whitespace pass. Full native 36963 is running on published 387f80f. Do not
restart the completed failing 57812 or count the focused pass as full acceptance.
Downstream propagation waits for this corrected head's full result.

### Detail browser contract and remaining blob inventory — 2026-10-07

The intervening estimate-only reply was no progress. Reconfirmed #216 merged
at 02f7b58 before resuming changes. Native gate36963 is terminal failed, not
still running: both earlier browser fixes passed, then
`new-project-permissions.cjs` attempted to replay `get_project_details`, which
the current detail UI no longer calls. The same fixture also assumed separate
assignment/task requests and an error inside a retained detail region.

Commit 4a5c7d9 in #270 adapts that browser matrix to the real
`get_project_detail_view` request and its nested project/label contract. It
asserts the exact requester, legacy-policy binding, edit affordance, project
identity, complete team/task labels and absence of financial fields even for
administrators. All six role/visibility cases, private-note redaction, foreign
organization denial, creation/draft ownership, exports, reports, revocation,
inactive users and anonymous creation requests remain exercised. Denied detail
must remove the details/team/tasks regions, and private notes must be absent
from the entire rendered body, not only one region. No production code changed.

The underlying legacy readers remain covered outside the browser through
`members_can_list_identities_but_not_assignment_rates`,
`managers_and_admins_can_read_assignment_rates`,
`revoked_membership_preserves_only_own_historical_tracking_identity`,
`project_reads_use_current_authority_and_reject_foreign_scope` and the registered
HTTP tests in `authorization_tests/project_reads.rs`; adapting the UI fixture
does not remove those tests or endpoints. Focused Chromium50335 passes every
matrix group on the exact unchanged production package from387f80f. Node syntax
and whitespace pass. Full browser sequence10797 is running on the same package
with the published fixture; complete fresh-head acceptance remains pending.

A read-only blob comparison is saved in the repository scratch artifact
`permission-blob-accounting.json` (generated by its sibling `.mjs` script).
Against original #212 db3935d and its9301112 base, 1,026 of1,214 changed paths
have an identical blob in at least one extraction:752 SQLx descriptors and274
other files. Another168 paths require adaptation/retained-work review
(69 SQLx,99 other files);20 original SQLx deletions need explicit query ownership
review. These numbers are not feature completion or final ownership: inherited
blobs may occur in multiple dependent PRs, and differing shared files can be
valid adaptations. #217 and the18 preserved dirty Client paths are separate
inventory obligations, not included in these counts. Nothing was deleted.

Next finish the browser sequence and fresh-head gate, propagate verified test
adaptations through #272/#273 and #271's shared base, continue diagnosing the
distinct ARM failures, and resolve the remaining hunk ownership. Task lifecycle
and catalogue UI work remains preserved in #212; it is not silently discarded
or counted as an accepted extraction. No merge or policy activation performed.
