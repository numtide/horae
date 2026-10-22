# Implementation plan: undo project archive

Branch `feat/project-archive-undo`, based on `8b3eda6b` (#299). Scope:
[spec.md](spec.md), existing H102 only. No broader dashboard completion.

## Technical context and approach

Reuse Rust/Dioxus SSR+WASM, `ProjectDetailContent`, shared `Toast` and the existing
`set_project_active` server function. PostgreSQL, schemas, SQL queries and
dependencies are unchanged. One transient action per detail view; no queue,
receipt, generic command framework or new durable model is needed.

1. Add an optional action to `crates/horae/src/components/toast.rs` using existing
   button/spacing utilities. Actionless consumers must render as before. Do not
   globally relocate the toast stack or change its shared default sizing.
1. In `crates/horae/src/pages/projects.rs`, distinguish archived-success feedback
   from reactivated-success feedback without comparing translated display text.
   Render Undo only for this view's successful archive and current loaded
   authority. Use the existing bound requester and absolute active=true write.
1. Share the existing pending guard between confirmation and Undo. Direct Undo
   opens no confirmation. On success refresh persisted details and recover
   focus; retryable errors stay visible outside the now-closed archive modal.
   On denial clear stale feedback and follow existing access-loss recovery.
1. Keep notice dismissal and navigation safe while asynchronous work completes.
   Reusing the keyed project content prevents state leaking between projects.
   Retry is idempotent at the existing server: setting an already-active project
   emits no second transition. This is not a history rollback or new CAS policy.

## Research and existing contracts

The functional and design sources, read-only observation limits and H103 conflict
are recorded in [spec.md](spec.md). Reuse the approved lifecycle/authority mapping
in `../header-increment.md`; do not infer the archived editor accepts writes.

Server inspection: `set_project_active` requires a manager, validates the bound
requester, then `authorize_legacy_status_write` checks active actor/current role
and policy zero under the organization gate. `set_project_active_in_transaction`
locks the project and updates only when active differs. Existing tests already
cover policy/role changes and organization isolation. Undo uses that contract,
never a new endpoint or cached permission decision.

## Validation and execution

Extend `crates/horae/tests/browser/project-detail-header.cjs` using its existing
disposable fixture and request interception. First observe missing Undo against
the baseline, then implement. Cover actual keyboard Undo without a second dialog,
pending duplicate suppression, persisted reload/data retention, retryable failure,
lost response after commit, lost authority, dismissal and project navigation.
Existing SSR/detail, bulk, and shared-toast consumers remain regression gates.

Shared Toast scope (T002): the Rust `Toast` has three callers, all actionless and
all to stay so: the Project Detail status result in `pages/projects.rs` (the only
Undo consumer), the importer success in `pages/importers.rs`, and three gallery
samples in `pages/gallery.rs`. The project-list bulk results do not use `Toast`.
The handoff has eleven `Toast` imports across Sign In, Timesheet, Workspace,
Importers, Project Detail, Clients, Client Detail, New Project and the component
sheet; only Project Detail's archived toast sets `action-label="Undo"`, and the
component sheet draws that same toast with the action between message and close.
Shared `.toast`/`.toast-container` rules in `assets/css/horae.css` stay unchanged.

Red tests (T003/T006) are additive to the existing flow: the first archive and
the 320px/200% archived state require reachable Undo and Dismiss in both themes;
a normal reactivation offers no Undo. A new block then covers keyboard Undo with
one held request, no dialog, a disabled `Updating project…` action and a disabled
Actions menu; persisted reload and unchanged non-status columns; keyboard
Dismiss without a write; aborted and lost-after-commit responses that stay
retryable outside the dialog and converge without rewriting the row (`xmin`);
changed account, policy and role denying Undo with the existing recovery; and
leaving during a pending Undo for another project without carrying its notice.

Run focused browser checks before the complete flake checks:

```sh
nix build .#checks.x86_64-linux.tests .#checks.x86_64-linux.browser \
  .#checks.x86_64-linux.clippy .#checks.x86_64-linux.sqlx-prepare \
  .#checks.x86_64-linux.treefmt --no-link --max-jobs 1 --cores 8 -L
```

Also run strict WASM application Clippy, `nix fmt`, and native Chrome acceptance
at 320/390/768/1440px, both themes and enlarged text, using only a disposable
local database. Inspect shared actionless toasts. No physical-device claim.
Two independent reviews, exact published-head required CI and protected merge
are mandatory. Only one heavy build at once; the editor reflow is the other
increment. No edits to its worktree, root or reference branches #208/#212.

## Constitution check

Before and after design: no duration/money arithmetic or domain logic changes;
existing UUID/org-scoped data only; PostgreSQL remains the datastore; all writes
use the existing session-authenticated Dioxus function; Nix/format checks remain
mandatory. No new policy, SQL cache, migration or constitutional exception.

## Progress

Specify/clarify: reused H102, two user stories, five requirements, eight checklist
items passing; no product question. Plan setup executed for this bounded path.
T002 recorded above; T003/T006 browser tests written, baseline red observation
pending. Implementation, independent reviews and delivery are pending.
