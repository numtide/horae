# Implementation Plan: Scoped Roles and Permissions

**Branch**: `feat/scoped-permissions` | **Date**: 2026-09-30 | **Spec**: [spec.md](spec.md)

## Summary

Deliver Harvest parity: six built-in profiles, reusable custom profiles, per-person adjustments and scoped approvals. No fixed-role-only or whole-week-only substitute is accepted.

This is an incremental plan. The independent record-scope foundation is executable; the full feature's research and policy design are not complete. Implementing that foundation does not authorize replacing runtime role checks. FR-002's complete operation matrix, adoption of the included governance amendment, dependent-spec reconciliation and reviewed migration remain prerequisites to the policy cutover.

## Technical Context

- Rust edition 2024, existing `horae-core` and Dioxus/Axum application crates.
- Existing UUID, serde and chrono dependencies; no new crate, dependency or policy engine.
- Pure scope evaluation has no I/O or persistence. Later persistence uses PostgreSQL, organization foreign keys and UUID v7 primary keys.
- Foundation validation: exhaustive unit tests, core regression suite, Clippy and Nix formatting.
- Runtime validation: SQLx integration tests, cross-surface authorization tests and browser comparisons with Harvest using disposable fixtures.
- Scope evaluation borrows assignments and allocates nothing. No new performance SLA is invented.

## Constitution Check

| Gate | Foundation | Runtime cutover |
| --- | --- | --- |
| Exact integer time/money | No arithmetic changes | Preserve totals and historical values |
| Pure domain rules | Scope evaluator in `horae-core`, no I/O | Shared decisions, trusted facts loaded by server |
| PostgreSQL, org isolation, UUID v7 | No schema changes; isolation tested | Required in migrations and every assignment |
| Server-function mutations | No new mutation surface | All access mutations session-authenticated |
| Reproducible validation | Nix core tests, Clippy and formatting | Full flake and integration checks before merge |
| Authorization target and transition | Runtime roles unchanged | Constitution 1.1.0 amendment included; matrix, migration and cross-surface acceptance still required before cutover |

Foundation design passes these gates. The full-feature post-design check is still open, not waived. No complexity exception is requested.

## Project Structure

```text
specs/015-scoped-permissions/
  spec.md
  plan.md
  research.md
  data-model.md
  contracts/record-scope.md
  quickstart.md
  tasks.md
crates/core/src/
  permissions.rs
  permissions/tests.rs
crates/horae/src/
  server_fns.rs
  server_fns/
  models/
  pages/
crates/horae/tests/integration.rs
```

## Execution and Dependencies

The 2026-10-02 implementation request permits advancing the confirmed catalog and
profile/dependency model with local tests while reference conflicts remain isolated.
Follow [grant-catalog.md](contracts/grant-catalog.md) and T027–T029 before runtime
integration. This does not reduce the requested feature, resolve C01–C07 by guess,
approve legacy mappings or waive the runtime cutover checks below.

1. Implement the independent FR-006 scope predicate with failing tests first: own, managed people, managed projects and organization; union without privilege inference; fail closed for inactive or mismatched identities/organizations.
1. Finish the Harvest reference matrix, custom prerequisite graph, approval/withdrawal semantics and access-path inventory. Inspect current runtime checks and review migration differences. Review the included constitution amendment and reconcile dependent feature contracts before cutover.
1. Finalize capability/profile and assignment persistence contracts. Add transactional authorization, stale-edit rejection, audit and concurrent last-administrator protection.
1. Integrate current permissions across server functions, lists/aggregates, exports/downloads, compatibility API, CLI, jobs and plugins. Do not activate a partially migrated policy.
1. Replace whole-week-only approval storage with verified date/project coverage, including empty-cell locks, submission editing and independent locks.
1. Implement Settings/Workspace permissions with shared descriptions, then verify full parity and migration fixtures. Keep the PR draft until all acceptance gates pass.

Steps 2–6 need detailed contracts before coding. Step 1 neither chooses role grants nor changes approval behavior. It does not satisfy any full user story by itself.

The 2026-10-02 persistence proposal in [data-model.md](data-model.md) and
[permission-state.md](contracts/permission-state.md) details canonical saved
grants, explicit administrative identity, composite tenant references, revisions,
atomic audit/outcome storage and one organization-first lock protocol. T033/T034
review these mechanics; T035–T041 refine future tests and integration. This is
partial planning while Phase 0 conflicts remain, not completed Spec Kit Phase 1
or permission to create schema early. Full post-design constitution approval is
still open; no policy, migration or cross-surface gate is waived.

Storage stays in the existing app (`server_fns/permissions.rs` plus
`permissions/`); shared DTOs live in `models/permissions.rs`. No new crate,
repository framework or policy engine is proposed. Reuse tenant keys and
transaction patterns after reconciling lock order across all writers. Saved
template behavior is not selected by the storage representation.

The [migration contract](contracts/migration.md) now defines the reviewable access
diff, stale-preview/atomic-cutover acceptance and historical provenance gaps for
step 2. Mappings and compatibility policy remain unapproved; the artifact's
existence does not complete T007 or authorize policy activation.

### Independent tenant-isolation repair

The access inventory found missing organization filters in the existing approval/reopen mutations. Correct these without introducing new roles or changing same-organization weekly semantics. Extract the existing transactions into pool-injected helpers so database tests exercise the production SQL; retain plugin dispatch after commit in the server wrappers. Test Manager and Admin callers against foreign pending/approved weeks, mixed-ID bulk requests, same-org success and invoice-lock preservation. Regenerate the SQLx cache and run approval regression tests. This closes an existing invariant violation; it does not bypass the matrix/governance gates for the new policy.

The same inventory identifies unscoped legacy assignment creation/removal. Preserve their existing administrator-only policy, but validate both project and person organization and reload active administrator authority under a transaction lock. Tests must call the production helpers, cover both foreign-ID directions, unknown IDs, duplicate creation, repeat removal, and completed/concurrent revocation. Return no foreign assignment details and emit events only for committed changes. This requires no schema or role migration and does not settle the future assignment-authority matrix.

User creation, role changes and activation changes also need transactional
reauthorization. Share the existing organization lock for last-administrator
protection, then reload and lock the active same-organization actor through commit.
Exercise completed revocation, waits on both organization and actor locks,
foreign/unknown actors, creation rollback and the existing concurrent last-admin
cases against the actual helpers. This closes the current FR-010/011 boundary;
it does not replace profile persistence, revisions or durable audit.

## Workflow Notes

The checked-in `setup-plan.sh --json` was executed. This repository does not contain `update-agent-context.sh`; no agent-context generation is claimed. Requirements checklist remains 12/16; the user requested continuation despite the remaining full-feature gaps.
