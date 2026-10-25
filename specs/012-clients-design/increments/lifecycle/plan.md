# Implementation Plan: Client archive and restore

**Branch**: `fix/client-archive-lifecycle` | **Date**: 2026-10-25 | **Spec**: [spec.md](spec.md)

## Summary

Add the FR-036 archive prerequisite and authority recheck to the existing client
status command, require an active client when restoring projects, fail import
records that would create an active project under an archived client, and offer
Archive (confirmed) and Restore (direct) on the Clients list and Client Detail with
pending, error and focus handling.
The client row lock is the serialization point. No schema, dependency, policy or
new lock helper.

## Technical Context

- Rust 2024, Dioxus fullstack, Axum, sqlx/PostgreSQL (READ COMMITTED default),
  pinned by `flake.lock`/`Cargo.lock`.
- Tests: `#[sqlx::test]` on disposable databases, existing blocker/`wait_for_blocked`
  race seams, Nix browser checks under `crates/horae/tests/browser/`.
- Scope: `set_client_active`, project status writes, Harvest import project
  creation, Clients list labels/states, and the Archive/Restore action and Archived
  badge on Client Detail (archive confirmed, restore direct). Nothing else is
  planned to change.

## Constitution Check

| Principle | Result |
| --- | --- |
| Exactness | No duration/money change. PASS |
| Domain purity | The rule is one predicate evaluated under a row lock; there is no domain computation to put in `horae-core`. PASS |
| Single datastore | Existing PostgreSQL rows only; no migration. PASS |
| Mutations through server functions | Same `#[server]` functions; import keeps its job path. PASS |
| Reproducible builds | Plans `.sqlx/` regeneration, clippy, fmt and flake checks (T015, T018). PASS |

PASS means the design complies, not that any gate has run. Re-checked after
design: PASS, no exceptions.

## Design decisions (research)

1. **Serialize on the client row, not the organization gate.** Status writes and
   imports all take the organization gate in SHARE mode, so the gate does not
   order them against each other (the 2026-10-22 note's "serialize under the org
   gate" was inaccurate). Every writer that can make a project active under a
   client already holds, or will hold, that client `FOR SHARE` in the same
   transaction. Archive takes it `FOR UPDATE` and checks projects in a later
   statement. Whichever commits first, the other then re-reads the latest client
   row (or sees the new project) and refuses. Rejected: an exclusive organization
   gate for client archive (blocks unrelated work, not needed), locking project rows
   on archive (adds a lock order with no benefit).
2. **Reuse the legacy status authority.** Make `projects::authorize_legacy_status_write`
   `pub(super)` with the policy-not-0 message as a parameter, and call it first in
   `set_client_active_record`. Rejected: extending `lock_client_manager` (no gate,
   no policy check; would duplicate the same query).
3. **Check project restore only on an actual transition.** Already-active projects
   stay no-ops even if legacy data has them under an archived client.
4. **Import check on the create path only**, by turning the existing currency
   lookup in `resolve_project` into `SELECT currency, active … FOR SHARE`. Existing
   matches return before it. Parent-failure caching already fails dependents
   (`parents.rs`, `apply.rs`); tests confirm it.
5. **READ COMMITTED is set explicitly in the archive transaction.** The proof in 1
   needs the post-lock project check to see writes that committed while archive
   waited. `db::lock_organization` sets no isolation and `pool.begin()` inherits
   the session default. Under REPEATABLE READ, a restoration or import that holds
   the client only `FOR SHARE` leaves the client row unchanged. Archive would then
   take its lock without error and its project check would read the snapshot from
   before the wait. So `set_client_active_record` starts with
   `SET TRANSACTION ISOLATION LEVEL READ COMMITTED, READ WRITE`, the inline
   first-statement pattern already used by `server_fns/invoices.rs`
   (`begin_invoice_write`) and others. No shared helper is added.
   The writer side is safe under any isolation. A writer's `FOR SHARE` on a client
   row that archive updated and committed after the writer's snapshot raises a
   serialization failure (40001) under REPEATABLE READ or SERIALIZABLE. That rolls
   the writer back, and an import reports it as that record's error. If archive
   committed before the snapshot, the writer reads the client as archived and
   refuses. Either way the writer never commits an active project under an
   archived client. Draft finalization also takes the organization gate in
   `AccessChange` mode, which conflicts with archive's SHARE gate. A test runs both
   commit orders with the database default set to REPEATABLE READ, so this does not
   rest on the server default.
6. **Archive confirmation on list and detail; Restore direct.** FR-036 removes the
   cascade and the affected-project count, not the archive confirmation (parent
   US4 scenario 3, the detail prototype's `ConfirmDialog` bound to `archive`). No
   approved source asks to confirm restore, so Restore calls the server directly
   with the same pending, error and focus handling. One small archive dialog,
   built on the existing `Modal` (native dialog, `busy`, `focus_fallback`) like the
   project-detail status dialog, serves both pages. Wording: "Archive {name}?" /
   "Only the client is archived. Its projects must already be archived. Tracked
   time, invoices and import history are kept." It never shows a project count or
   names. A refusal stays in the dialog with the server's message, a link to
   `Route::ProjectsForClient` and a safe retry. Cancel and Escape go through
   `Modal::on_dismiss`, which is ignored while busy. A restore failure uses the
   page's existing alert.
7. **Focus after refresh, for both actions.** After the reload, call the existing
   `restore_opener_focus(opener, fallback)` with `client-activity-{id}` /
   `clients-heading` (list) or `client-status` / `client-back` (detail). It acts
   only while focus is on the body or the fallback, so a user who moved focus keeps
   it. Today it does nothing when the opener is gone, which leaves focus on the
   body after a direct Restore removes the row from the Archived filter. Change it
   to focus the opener if present, otherwise the fallback (one expression:
   `(byId(opener) ?? byId(fallback))?.focus()`). For the existing editor callers,
   this changes only the case where the opener is gone (for example a renamed
   client no longer matches the search): focus moves to the heading instead of
   staying on the body. The browser check covers it. The archive dialog also
   passes the same fallback as `Modal`'s `focus_fallback`.

## Writer and lock scope

Order everywhere: organization gate → actor `users` row → project row → client row.
Archive locks no project row, so no cycle is introduced.

| Writer | Gate (existing) | Client lock | Check |
| --- | --- | --- | --- |
| `clients::set_client_active_record` | **add** `SET TRANSACTION ISOLATION LEVEL READ COMMITTED, READ WRITE` first, then `authorize_legacy_status_write` (org SHARE, actor `FOR SHARE OF u`, role, policy 0) | `lock_client` `FOR UPDATE` (existing) | **add** on archive transition: no linked `projects.active`, as a separate statement after the lock |
| `projects::set_project_active_in_transaction` (single + bulk) | existing `authorize_legacy_status_write` | **add** client `FOR SHARE` after `lock_project`, only when inactive → active | client active, else whole transaction fails |
| `project_creation::finalize_draft_record` | existing `AccessChange` | existing `lock_creation_client` (`AND active FOR SHARE`) | unchanged |
| `project_creation::editing::save` (reassignment) | existing gate | existing `FOR SHARE`, refuses a newly selected inactive client | unchanged |
| `importers::harvest::resolve::resolve_project` (create) | existing org SHARE in `begin_import_transaction` | **change** currency lookup to `FOR SHARE` | `!client.active && row.project_active` → `RowFailure` |

No other runtime writer sets `projects.active` or `projects.client_id`
(`rg "INSERT INTO projects|UPDATE projects SET"` outside tests and seeds).
An import batch holding a client `FOR SHARE` delays a concurrent archive of that
client until the batch commits; this is the same wait other client writers have.

## Error contract

Existing codes and helpers; status codes by name. Checks run in this order:
authority, then the client lookup (unknown and foreign look the same), then the
project state. A caller without current authority therefore learns nothing about
the client's projects.

| Case | Result |
| --- | --- |
| Unknown or foreign client/project | `NOT_FOUND`, existing messages (unchanged) |
| Actor inactive, not manager | `FORBIDDEN` "Manager access required" |
| Policy ≠ 0 | `FORBIDDEN` "Client status changes are unavailable" |
| Archive with an active linked project | `CONFLICT` "Archive this client's projects before archiving the client." |
| Restore project under archived client | `CONFLICT` "Restore the client before restoring this project." |
| Import record | `RowFailure` "client is archived; restore it before importing an active project" |

Server function signatures and plugin events stay as they are.

## Source changes

- `crates/horae/src/server_fns/clients.rs`: explicit isolation, actor id into `set_client_active_record`, authority, archive check.
- `crates/horae/src/server_fns/projects.rs`: `authorize_legacy_status_write` visibility/message; client check in `set_project_active_in_transaction`.
- `crates/horae/src/importers/harvest/resolve.rs`: create-path client check.
- `crates/horae/src/pages/clients/status.rs` (new, sibling-file module like `form.rs`): the archive confirmation on `Modal`.
- `crates/horae/src/pages/clients.rs`: Archive opens it, Restore calls directly; per-client pending guard; focus as in decision 7. `pages/clients/filters.rs`: "Archived clients". `pages/clients/detail.rs`: header `client-status` action, Archived badge, the same dialog, detail refresh. No other detail change.
- Tests: `server_fns/clients/tests.rs`, `server_fns/projects/bulk_tests.rs`, `importers/harvest/engine_tests.rs` and `sync_tests.rs`, browser `clients.cjs` (list and detail), `action-errors.cjs`, `clients-errors.cjs`, `clients-permissions.cjs`, and the `tests/detail_navigation.rs` stub (signature unchanged).
- `.sqlx/` regenerated.
- After the tests pass, one sentence in 015 `contracts/client-permissions.md` records the implemented, tested evidence (task T019).

## Verification

Run only after the coordinator hands over build resources; disposable `/tmp`
PostgreSQL only.

The exact commands are in tasks.md T004 and T015.

Browser and native checks (both themes, keyboard, 320/390/768/1440px, delayed and
failed responses) are run by the coordinator's browser.

## Complexity Tracking

None.
