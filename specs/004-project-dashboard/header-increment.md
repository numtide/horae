# Increment: Project Detail identity header and archive/reactivate

**Branch**: `feat/project-detail-header` | **Date**: 2026-10-22 | **Base**: `6f685d6c`

This is one bounded recovery increment of #208. It does not regenerate or replace
this feature's historical `plan.md` and `tasks.md`, which describe the original
2026-09-01 package and are not the current header contract. It does not complete
the Project Detail dashboard; every other dashboard slice remains open.

Implementation checkboxes record written code or tests, not acceptance. Runtime
verification and independent review remain pending until explicitly recorded.

## Requirement reconciliation

The current contract is the approved #208 specification, applied only where this
increment reaches. Requirement IDs below are #208's (`feat/project-dashboard`,
`specs/004-project-dashboard/spec.md` at `48a41560`); this branch's historical
`spec.md` reuses FR-014/015/017 for different requirements.

| #208 requirement | Status | This increment | Remaining |
|---|---|---|---|
| FR-001 identity | Partial | Client (link when the viewer may open it), `[code] name`, type, currency, Archived state, Back to Projects; saved dates, tags and administrator notes under "Project information". | Active Projects navigation highlighting is not verified here. |
| FR-014 edit | Partial | Edit project uses the shared `EditProject` route under the existing `can_edit` affordance, with a decorative pencil glyph. | H101 `return_to`; "Set up a budget". |
| FR-015 archive/reactivate | **Incomplete** | Shared Actions menu, native confirmation, pending/duplicate protection, retryable transport errors, success toast, persisted reload, focus return. | H102 Undo; H103 archived-edit lifecycle (P02). |
| FR-017 current authority | Partial | Affordances come from the current read; status writes recheck actor, role and policy zero in their own transaction; denied writes hide the loaded project behind the bound Retry. | Status changes under policy one need a reviewed canonical capability (not in scope). |
| FR-018 states | Implemented; verification pending | Loading, unavailable and Retry keep the existing stale-resource guards; Back is available in every state. | H009 and independent review. |
| FR-019 layout | Tests written; verification pending | 320/390/768/1440px, a 1440×360 viewport and 200% CSS zoom, in both themes; keyboard path to Actions and the dialog. | H009, visual review and actual 200% text enlargement; CSS zoom alone is not text zoom. |
| FR-020 design system | Implemented; verification pending | Utility classes, `Menu`, `Modal`, `Badge`, `Toast`, `NavIcon`; no inline styles, sample data or prototype handlers. | H009 and independent review. |
| FR-021 / FR-022 | Not affected | No quantities; the detail read gains no writes. | — |

Tasks and Team cards and fee balances are unchanged; later dashboard slices
replace them.

## Authority mapping

| Affordance | Server authority it mirrors | DTO field |
|---|---|---|
| Client link | `client_detail_view` admits `legacy OR ClientReadAll` (`ClientReadScope::can_read`) | `can_open_client` |
| Edit project | Existing editor affordance | `can_edit` (unchanged) |
| Archive / Reactivate | `set_project_active`: session `require_manager()`, bound requester, then in the write transaction under the shared organization gate: active actor, current Manager/Administrator role, `permission_policy_version = 0` | `can_change_status = legacy AND manager-or-above` (as the list's `can_change_legacy_status`) |

Identity fields `client_id`, `project_type` and `active` come from the existing
context query inside the detail read transaction. Overview readers already see
them, so no read widens.

## Server hardening

Before this increment, a status write checked only the session role and the
requester. A policy, role or account change after the session read did not
fail closed. `set_project_active` and `set_projects_active` now call
`authorize_legacy_status_write` inside their transaction, before any project
lock. It takes `lock_organization(Shared)` (so it orders after a concurrent
access change) and the actor/policy query used by `authorize_task_write`. Policy
one and unknown policies are rejected; no permission is added or activated.

## Gaps reported, not changed

- Harvest blocks editing while a project is archived and restores first
  ([Archiving, restoring and deleting projects](https://support.getharvest.com/hc/en-us/articles/360048686911-Archiving-restoring-and-deleting-projects),
  verified 2026-10-22). Horae's editor and `can_edit` still allow editing archived
  projects (P02). This increment claims no archived-edit parity.

## Plan

1. Reuse `ProjectDetailView` and its bound requester, policy and generation guards.
   Add identity fields in the existing authorized transaction; do not fetch a
   broader client catalog or expose private editor data.
2. Compose the header with existing tokens/utilities and shared controls. Keep
   Back available during loading and denial; preserve saved project information
   in a disclosure and current unrelated detail sections until their own slices.
3. Use the existing status endpoints, with a current policy-zero active-actor
   check inside their transaction before project locks. Reject denied mutations,
   suppress stale details and retain the original requester for Retry. Do not
   implement or activate policy-one lifecycle authority.
4. Regenerate SQLx in an isolated PostgreSQL sandbox, then compile/test offline,
   validate SSR/WASM, existing bulk callers, browser navigation and lifecycle,
   desktop/mobile themes, keyboard and enlarged text. Preserve shared CSS defaults.
5. Obtain independent technical and UX/regression reviews; resolve findings and
   verify required CI on the exact published head before the protected merge queue.

Constitution: no amount/duration arithmetic or domain-rule duplication, no new
datastore or schema, all mutations remain typed server functions, and Nix checks
remain mandatory. No constitutional exception is required.

## Tasks

- [x] H001 Server tests for identity and lifecycle affordances in
  `server_fns/projects/canonical_read_tests/detail.rs`.
- [x] H002 SSR tests for header identity, gating, archived state and Back in
  `tests/detail_navigation/detail.rs`; canonical and policy-zero fixtures separate.
- [x] H003 Native suite `tests/browser/project-detail-header.cjs` and its default
  entry in `run-design-checks.sh`.
- [x] H004 DTO fields in `models/project.rs`; projection in `detail_view.rs`.
- [x] H005 Header, Actions, confirmation, toast, focus and denial handling in
  `pages/projects.rs`; pencil glyph in `components/icons.rs`.
- [x] H006 Heading selectors in `new-project.cjs` and `invoice-preparation.cjs`.
- [x] H007 In-transaction status authority and tests for changed role, inactive
  actor, policy one/unknown, foreign actor, other-organization isolation and a
  policy change committed while the write waits (`bulk_tests.rs`); existing
  callers pass the actor.
- [x] H008 Regenerate `.sqlx/` (detail context query and new test queries).
- [ ] H009 Server/WASM Clippy, `cargo test -p horae --features server`
  (`detail_navigation`, `canonical_read_tests::detail`, `bulk_tests`,
  `mutation_tests`), `nix fmt`, and the browser runner including
  `project-detail-header`.
- [ ] H010 Resolve both independent reviews, inspect live desktop/mobile themes,
  keyboard, enlarged text and post-mutation focus; verify exact-head CI and merge
  through the protected queue. Do not infer this acceptance from written tests.

### Pending, outside this increment

- [ ] H101 FR-014: Edit project carries `return_to=/projects/<id>` and the editor
  returns there on save and cancel, as Harvest does; test with browser history.
- [ ] H102 FR-015: "Project archived" toast offers Undo that reactivates through
  the same authorized write, following the handoff's direct Undo action; test
  pending, failure and lost authority without a second confirmation dialog.
- [ ] H103 P02: implement archived-project editing restrictions (Harvest: restore
  first) across `can_edit`, the editor and its server write.
