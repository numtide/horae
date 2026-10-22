# Implementation plan: undo project archive

Branch `feat/project-archive-undo`, originally based on `8b3eda6b` (#299). Scope:
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

Review additions (author tests, before correction): a pending Undo dismissed
before it fails must still show a visible, retryable failure; the duplicate
request count waits one render; and at 320px/200% the action toast's message
must stay at least one word wide, with a semibold (600) Undo as in the handoff.

Focused run 42166 failed on the expectation that Actions regain focus after an
Undo, not on the application. `.btn:disabled` and `.btn.is-disabled` set
`pointer-events: none`, so the test's forced clicks on the pending Undo and the
disabled Actions trigger fell through and focused `main#app-main`; native traces
confirmed plain keyboard Undo returns focus to Actions, while the forced sequence
leaves `MAIN` focused. `restore_detail_focus` deliberately overrides only the
body or Back, so it must not take `#app-main` (the skip-link target). The test
now keeps the keyboard Undo focus assertion with a keyboard duplicate, and moves
the forced pointer duplicates to a separate held cycle that asserts one request,
then that a deliberately focused sidebar Projects link keeps focus after the
reload. No product change and no retry.

Recovery feedback differs from the handoff by design of the existing flow: the
prototype's `undoArchive` simply hides the toast, while this view keeps the
existing "Project reactivated." success notice after an Undo succeeds, exactly as
after a confirmed Reactivate. No new product behavior is introduced for it.

Observed red after the review additions (`e2767535`, focused run compiled native
and WASM): at 320px/200% the action toast's message measured 108px against a
133px longest word, and the native package measured 101.7px; separately, a held
Undo, keyboard Dismiss and aborted request left zero toasts with the project
still archived. Correction: an Undo failure restores the archive notice with its
retry; the action uses `text-sm font-semibold`; and only an action toast wraps,
with its message `flex-none max-w-full` so it keeps its text width. Actionless
toast markup is unchanged. The pending label `Updating project…` may wrap inside
its button at 320px/200% rather than being shortened.

A DOM-only diagnostic of that correction found the longer failure text above the
viewport at 320×568/200%, and the pending action wider than its content column.
The bounded correction uses existing `px-1 max-w-full` on the action and the
truthful, shorter "Could not confirm restore. Retry Undo." message (a lost reply
does not establish whether the write committed). The test now requires the entire
pending and failure toast, including button contents, to fit that short viewport.
This diagnostic is not acceptance of the rebuilt artifact; those gates remain
pending below.

Known limits: a successful Undo is a real reactivation and emits the ordinary
`ProjectReactivated` event; leaving the project while Undo is pending may let the
write finish with no notice anywhere (the detail then shows the persisted
status); and a refresh that fails or loses access discards the transient Undo,
leaving the normal Reactivate action as the recovery path.

After a committed write whose reply is lost, the loaded header may still say
Archived until the successful retry refreshes it. The error does not assert that
the write failed. Dismissing a notice retains the shared component's existing
focus-to-body behavior; this increment does not change all toast consumers.
Clicking a pending, pointer-disabled action can instead focus `main#app-main`;
recovery deliberately preserves that target, also used by Skip to content.
The short failure message uses Harvest's "restore" wording while the existing
menu/success use Reactivate; this bounded copy difference is accepted to keep the
enlarged failure notice readable, not a new lifecycle state.

Both independent reviewers accepted the corrected `de09905d` source conditionally,
with no blocking finding. Actual rebuilt evidence is still required. In addition
to the automated gates, inspect wide-screen action alignment, icon wrapping in
the enlarged error, pending feedback, and actual keyboard access. Compare the
unchanged actionless gallery/importer consumers; source compatibility alone is
not a claim of completed browser verification.

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

## Native verification — 2026-10-22

Windows Chrome tested the actual `30023bed` package
`/nix/store/h21r4aw09lzzjy7gvcgghpvslibfbryd-horae-0.1.0` (production unchanged
since `de09905d`) against a fresh disposable database, with Instrument Sans
loaded. Forty-eight archive/pending/error cases covered both themes,
320/390/768/1440px and 100%/200% root text; the toast and button contents fit in
all cases, including 320×568. Held real requests were aborted and retried; no
response was fabricated. At the narrowest enlarged size the icon occupies its
own line and the disabled progress label wraps. The disabled label retains
the shared reduced opacity and is dim, especially in the dark theme. At
320×568/200% the notice is tall (about 369px pending, 418px failed) and covers
much of the underlying content until dismissed. The existing bottom-right stack
also overlays content on desktop; this increment does not reposition all toasts.

Actual Tab traversal reached Undo eleven stops after Actions in the seeded
detail, at desktop and 320px/200%; this is reachable, not a short keyboard path.
Keyboard Undo succeeded. Dismiss during a held Undo followed by an aborted
request restored the visible error and retry, and retry succeeded. Reactivated
feedback has no inverse action. Three actionless gallery notices retained their
original classes and 280×46.33px geometry in both themes. A real local importer
dry-run with one unmatched-user row produced the same actionless notice as the
baseline, also 280×46.33px in both themes; keyboard Dismiss worked. The synthetic
file was supplied through `DataTransfer` because the cross-OS file-path upload
was unavailable. No import was committed and no real-account data was touched.
These are emulated viewports/root text, not physical devices or native zoom.

Focused run 50307 passed every header and Undo scenario using tests `78e1c51c`
and the existing `h21r4` binary: production was unchanged, so no replacement
binary is claimed for this test-only revision. Both independent reviews accepted
the corrected source, split tests and native visuals with the explicit limits.
The complete combined checks below subsequently passed. Exact published-head CI
remains mandatory before protected merge.

## Combined acceptance and delivery

The full local run on `a30ffd5d2685437299b54fedd3c630b7c87891f5`, based on
editor-reflow head `7b9da1db`, passed the five Nix checks above and strict WASM
Clippy (`-D warnings -W clippy::perf`). The actual combined package was
`/nix/store/wadaabgmkd5imdqcbmiyxxsrpyn1y3yr-horae-0.1.0`. The browser suite,
SQLx cache verification, native Clippy and treefmt all passed. Tests passed:
191 core cases, 1,504 app cases and all integration binaries. The 11 existing
ignored app tests remain ignored; this increment did not disable any test.

An additional native Chrome confirmation on that combined package covered
archive, held Undo, aborted response, visible error and keyboard retry in both
themes at 1440×900/100% and 320×568/200% root text (12 captures). All toast and
button contents fit; successful reactivation persisted through reload. The
visual reviewer inspected the combined captures and source delta and found no
new regression. The previously documented limitations remain applicable.

Publication is stacked on the editor-reflow branch so its diff contains only
this increment. PR #300's Nixbot build 685 failed the ARM e2e setup while fetching
a remote store dependency, before that test ran. It must not be bypassed; a
retry authorization is pending. Neither successful local checks nor a dependent
PR replace the parent's required CI. After #300 merges, rebase only this
increment onto its actual squash commit, compare the source tree, and verify
the final published head before protected merge. Delivery remains open.

## Constitution check

Before and after design: no duration/money arithmetic or domain logic changes;
existing UUID/org-scoped data only; PostgreSQL remains the datastore; all writes
use the existing session-authenticated Dioxus function; Nix/format checks remain
mandatory. No new policy, SQL cache, migration or constitutional exception.

## Progress

Specify/clarify: reused H102, two user stories, five requirements, eight checklist
items passing; no product question. Plan setup executed for this bounded path.
T002 recorded above. T003/T006 baseline red observed: against the `8b3eda6b`
header and backend, the existing header, navigation and sixteen theme/size checks
passed and the first archive failed only on the missing Undo. T004/T005/T007 are
implemented and verified by the focused checks and native acceptance (optional
`Toast` action; one writer shared by confirmation and Undo, visible retry).
Spec Kit reanalysis covers all eight requirements/outcomes with ten tasks and no
unmapped task or constitutional conflict. The pending-dismissal exception was
clarified consistently with the existing retry requirement and test.
Independent reviews are resolved and the complete local checks passed. Delivery
remains pending. The combined run includes the independently reviewed editor
reflow; after its protected merge, preserve the verified source tree while
rebasing onto the actual master commit. Exact-head required CI is still mandatory.
