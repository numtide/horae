# Increment: Project Detail identity header and archive/reactivate

**Branch**: `feat/project-detail-header` | **Date**: 2026-10-22 | **Base**: `6f685d6c`

This is one bounded recovery increment of #208. It does not regenerate or replace
this feature's historical `plan.md` and `tasks.md`, which describe the original
2026-09-01 package and are not the current header contract. It does not complete
the Project Detail dashboard; every other dashboard slice remains open.

Implementation checkboxes record written code or tests, not acceptance. The
verification record below distinguishes local acceptance from remote CI and
delivery; it does not certify the remaining dashboard slices.

## Requirement reconciliation

The current contract is the approved #208 specification, applied only where this
increment reaches. Requirement IDs below are #208's (`feat/project-dashboard`,
`specs/004-project-dashboard/spec.md` at `48a41560`); this branch's historical
`spec.md` reuses FR-014/015/017 for different requirements.

| #208 requirement | Status | This increment | Remaining |
|---|---|---|---|
| FR-001 identity | Partial | Client (link when the viewer may open it), `[code] name`, type, currency, Archived state, Back to Projects; saved dates, tags and administrator notes under "Project information". | Active Projects navigation highlighting is not verified here. |
| FR-014 edit | Partial | Edit project uses the shared `EditProject` route under the existing `can_edit` affordance, with a decorative pencil glyph; H101 contextual return delivered in #299. | "Set up a budget". |
| FR-015 archive/reactivate | **Incomplete** | Shared Actions menu, native confirmation, pending/duplicate protection, retryable transport errors, success toast, persisted reload, focus return. | H102 Undo; H103 archived-edit lifecycle (P02). |
| FR-017 current authority | Partial | Affordances come from the current read; status writes recheck actor, role and policy zero in their own transaction; denied writes hide the loaded project behind the bound Retry. | Status changes under policy one need a reviewed canonical capability (not in scope). |
| FR-018 states | Delivered | Loading, unavailable and Retry keep the existing stale-resource guards; Back is available in every state. Denied reloads clear success feedback and restore recovery focus. | — |
| FR-019 layout | Delivered | 320/390/768/1440px, a 1440×360 viewport, 200% CSS zoom and 200% root-font enlargement, in both themes; keyboard path to Actions and the dialog. | No claim of physical devices or native browser zoom. |
| FR-020 design system | Delivered | Utility classes, `Menu`, `Modal`, `Badge`, `Toast`, `NavIcon`; no inline styles, sample data or prototype handlers. Shared menu/toast fixes have cross-screen regression coverage. | — |
| FR-021 / FR-022 | Not affected | No quantities; the detail read gains no writes. | — |

Tasks and Team cards and fee-balance behavior are unchanged; later dashboard
slices replace them. The fee date controls gain local minimum-width utilities
because their intrinsic width overflowed the page with enlarged text.

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

- Harvest's documentation says to restore an archived project before editing
  ([Archiving, restoring and deleting projects](https://support.getharvest.com/hc/en-us/articles/360048686911-Archiving-restoring-and-deleting-projects),
  read 2026-10-22). A read-only browser check on an existing archived project
  showed no header Edit action, but its budget/rate links opened an enabled,
  populated editor. No save was attempted. Thus the archived write behavior is
  unverified and the documentation alone does not establish a route prohibition.
  Horae's editor and `can_edit` still allow archived projects (P02). H103 must
  reconcile the evidence before imposing a new restriction; this increment
  claims no archived-edit parity.

## Plan

1. Reuse `ProjectDetailView` and its bound requester, policy and generation guards.
   Add identity fields in the existing authorized transaction; do not fetch a
   broader client catalog or expose private editor data.
1. Compose the header with existing tokens/utilities and shared controls. Keep
   Back available during loading and denial; preserve saved project information
   in a disclosure and current unrelated detail sections until their own slices.
1. Use the existing status endpoints, with a current policy-zero active-actor
   check inside their transaction before project locks. Reject denied mutations,
   suppress stale details and retain the original requester for Retry. Do not
   implement or activate policy-one lifecycle authority.
1. Regenerate SQLx in an isolated PostgreSQL sandbox, then compile/test offline,
   validate SSR/WASM, existing bulk callers, browser navigation and lifecycle,
   desktop/mobile themes, keyboard and enlarged text. Preserve shared CSS defaults.
1. Obtain independent technical and UX/regression reviews; resolve findings and
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
- [x] H008a Fix the shared popover coordinate mismatch exposed by the header's
  200% CSS-zoom test; verify normal/scaled menus, selectors and calendars, plus
  their existing keyboard, scrolling and dismissal checks. CSS zoom is distinct
  from native browser zoom; 200% root-font tests also enlarge rem-based spacing.
- [x] H008b Verify enlarged-text date controls, wrapping confirmation actions,
  viewport-constrained shared toasts and focus after a denied refresh. Cover
  other shared-toast consumers and retain normal desktop sizing.
- [x] H009 Server all-target/WASM binary Clippy, `cargo test -p horae --features server`
  (`detail_navigation`, `canonical_read_tests::detail`, `bulk_tests`,
  `mutation_tests`), `nix fmt`, and the browser runner including
  `project-detail-header`.
- [x] H010 Resolve both independent reviews, inspect live desktop/mobile themes,
  keyboard, enlarged text and post-mutation focus; verify exact-head CI and merge
  through the protected queue. Do not infer this acceptance from written tests.

## Verification record — 2026-10-22

Implementation commit `3a360f5e23f1c4c049ba3634c0befeb69b5ad16a` passed the
following complete local command, using Nix-isolated PostgreSQL fixtures:

```sh
nix build .#checks.x86_64-linux.tests .#checks.x86_64-linux.browser \
  .#checks.x86_64-linux.clippy .#checks.x86_64-linux.sqlx-prepare \
  .#checks.x86_64-linux.treefmt --no-link --max-jobs 1 --cores 8 -L
```

- 191 core tests, 1,498 server tests and 349 tests across 15 external binaries
  passed; 11 pre-existing manual server tests remain ignored. No assertion,
  ignored-test policy or CI configuration was weakened.
- The complete browser runner passed, including header lifecycle, binding and
  denial recovery, shared menus/selectors/calendars, responsive toasts, bulk
  actions, creation/editing, Clients and invoice-client labels.
- Strict core/server all-target Clippy, SQLx cache verification and formatting
  passed. The package built both SSR and WASM and supplied the browser binary.
- A supplementary WASM **all-target** lint invocation failed on existing native
  test modules requiring Tokio/SSR. Those files are unchanged from the base;
  their native tests passed above. This is not a passing check or a reason to
  alter their coverage. Separate strict WASM application-binary Clippy passed:
  `cargo clippy -p horae --features web --target wasm32-unknown-unknown --bin horae -- -D warnings -W clippy::perf`.

Native Windows Chrome also exercised the built package on a disposable local
database: settled desktop/mobile captures in both themes, actual archive and
reactivation, keyboard cancellation, enlarged text and a denied reload. Desktop
toast width is 280px; at 320px with 200% root font the toast and page remain inside
the viewport, and Tab scrolls each dialog action fully into view. Status reloads
return focus to Actions; denied refresh returns focus to Back with no old toast.
CSS zoom scales the whole page; root-font enlargement also scales rem spacing.
These checks do not claim physical-device or native browser-zoom coverage.

Independent technical and visual/UX reviews found no unresolved blocking issues
after remediation and runtime inspection. Resolved findings included action
height, stale feedback, scaled menu coordinates, missing responsive padding and
cyclic toast sizing. A pre-existing successful Refresh/Retry focus gap remains
outside this increment; denied and post-status reload recovery are covered.

H010 completed: [PR #298](https://github.com/numtide/horae/pull/298) merged through
the protected queue on 2026-10-22 at 02:53 UTC, as `5e555351`. Both mandatory
Nixbot checks passed on published head `029b0d79`; build 683 completed all 22
attributes, including the ARM browser and NixOS checks.

### Follow-up increments

- [x] H101 FR-014: Edit project carries `return_to=/projects/<id>` and the editor
  returns there on save and cancel, as Harvest does; test with browser history.
  [PR #299](https://github.com/numtide/horae/pull/299) merged through the protected
  queue at 05:12 UTC on 2026-10-22 as `8b3eda6b`, after both reviews and required
  checks on head `25390256` (Nixbot build 684). Local verification is recorded in
  [the return increment](edit-return-increment.md).
- [ ] H102 FR-015: "Project archived" toast offers Undo that reactivates through
  the same authorized write, following the handoff's direct Undo action; test
  pending, failure and lost authority without a second confirmation dialog.
  Implementation and review corrections are in [the bounded Undo package](archive-undo/plan.md);
  final runtime checks, reviews and protected delivery remain pending.
- [ ] H103 P02: resolve the archived-editor documentation/runtime conflict above
  before deciding whether restrictions belong in `can_edit`, the editor and its
  server write. Enabled GET controls are not proof that an archived save succeeds.
