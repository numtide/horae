# Increment: return from editing to the current project

**Branch**: `feat/project-edit-return` | **Date**: 2026-10-22

This implements H101 from [the header increment](header-increment.md), not the
remaining dashboard. It reuses feature 011 US7 and its explicit-save editor.
The original dashboard plan and completed editor tasks remain historical records.

## Contract and evidence

Harvest's live project detail Edit link includes
`return_to=/projects/<the current project>` (read-only observation, 2026-10-22).
The editor page returned for that link (authenticated GET, HTTP 200) shows a
"Back to Project" link and a Cancel link, both pointing at the same project's
detail, so the destination and Back copy were observed directly rather than
inferred from the link.
The handoff supplies the shared editor and project navigation. No Harvest write
is required or claimed. Horae already saves to the same project's detail; the
gap is Cancel/Back/discard returning to the list when entering from detail.

- ER-001: Detail → Edit carries the current project destination through reload.
  Clean Cancel/Back and confirmed discard return there without saving changes.
- ER-002: Only the exact same-project detail path is an accepted return target,
  and it must be the whole query. Missing, foreign-project, absolute,
  protocol-relative, malformed, non-canonical or extended targets, and any
  extra or repeated query parameter, fall back to the existing Projects list.
  Never navigate a raw string. The destination must not change when the
  router writes the URL back and parses it again.
- ER-003: List-origin editing keeps its existing destinations: Cancel/Back to
  Projects, successful Save to the saved project's detail. Plain existing edit
  URLs continue to parse. A router-generated empty query delimiter is not a
  behavioral change; do not add duplicate route variants to avoid it.
- ER-004: Dirty/pending navigation, conflict/retry handling, authorized loading
  and saving, same project identity and creation-draft isolation are unchanged.
  Save remains explicit and returns to the saved project's detail.
- ER-005: Back copy describes its destination; shared design controls, themes,
  keyboard access and 390/768/1440 layouts remain intact. No CSS changes.

## Plan and constraints

Use the existing Dioxus route query convention with a typed, defaultable query
field. Map its decoded value to a typed internal route through a small pure
function; do not decode twice, access global browser location, or add storage.
Thread the resolved destination into the keyed editor loader, error states and
explicit leave/discard paths. Reuse the navigation guard and shared form.

Only the detail-origin Edit link carries a nonempty return target. Update route
constructors and SSR test mirrors as required; assertions must keep checking the
right project and destination rather than merely accepting any query string.
Keep sidebar highlighting, Undo, archived-edit restrictions and dashboard data
outside this increment. No server/API/SQL/schema/dependency changes are planned.

Constitution: no financial or duration arithmetic, datastore or authority change;
all existing writes stay on their authenticated server-function path. No exception.

## Tasks

- [x] E001 [US7] Add route/parser and unsafe-target tests in
  `crates/horae/src/route.rs`, plus SSR link assertions in
  `crates/horae/tests/detail_navigation/detail.rs`; observe failure first.
- [x] E002 [US7] Implement query and typed destination mapping in
  `crates/horae/src/route.rs`, updating required route constructors and test mirrors.
- [x] E003 [US7] Thread destination and truthful Back copy through the shared
  loader/editor in `crates/horae/src/pages/new_project.rs`; add detail-origin
  context in `crates/horae/src/pages/projects.rs` without changing list behavior.
- [x] E004 [US7] Extend `crates/horae/tests/browser/project-detail-header.cjs`
  for Cancel/Back, discard/stay, Save persistence, reload, dirty browser history
  and forged targets. Preserve list-origin and creation regression assertions.
- [x] E005 Run server/WASM build, strict lint, formatting, route/SSR tests and
  complete browser regression checks; record results in this document.
- [ ] E006 Obtain independent technical and visual/UX reviews, resolve findings,
  verify exact-head mandatory CI and deliver through the protected merge queue.

Dependencies: E001 → E002 → E003 → E004 → E005 → E006. One author owns this
worktree; reviewers are read-only and heavy checks are coordinator-serialized.
ER-001/003 map to E001–004, ER-002 to E001/002/004, ER-004/005 to E003–005.
Acceptance is a real detail → editor → unchanged or saved detail flow, plus
unchanged list-origin exits; written tests alone do not complete this increment.

## Workflow record

Spec Kit setup-plan, setup-tasks and implementation prerequisites ran on this
worktree with feature 004 selected explicitly. Existing files were preserved;
the historical requirements checklist is 16/16. There are no extension hooks.
The temporary selector change was restored. Clarification adds no new product
decision: this is the existing H101 contract and observed Harvest destination.

E001 failure observed in the browser only: the focused `project-detail-header`
run on the cached header server failed at the first detail-origin Cancel,
expecting `/projects/<id>` and receiving `/projects`. The route unit tests were
compile-red by construction (missing field and function) and were not executed
separately; the SSR link assertions were not run before implementation.

E002–E004 were implemented in `f785f187` and passed the full check, but
runtime review then found that repeated parsing changed the destination.
The router decodes the query once when parsing but serialized a `String`
value without re-escaping `%`. Both server memory history and hydration
reparse serialized routes, so
`return_to=%252Fprojects%252F<id>` was decoded a second time to
`/projects/<id>`: the editor showed "Back to project" and Cancel opened the
detail. The defect was the changed destination, not the address bar's spelling.

The extended tests reproduced this first: the focused
`project-detail-header` run exited 1 because the double-encoded target showed
"Back to project" instead of "Back to Projects".

The fix makes the whole query the typed `EditReturn` (`?:..return_to`) in
`pages/new_project/edit_return.rs`. The router hands it the query decoded
once; it keeps only a project UUID, and only when the query is exactly
`return_to=/projects/<canonical UUID>`. Serializing the route, as links,
navigation and the server's history do, emits only that canonical query or an
empty one, so parsing a serialized route cannot reach a different destination.
The address bar is separate: a direct load or reload keeps the typed URL, which
is parsed again with the same single decode and reaches the same destination.
A canonical target naming another project is a valid query and serializes
unchanged; the editor still exits to the Projects list, because only the
edited project's own detail is honoured. Non-canonical targets are dropped.
The type is its own file, using only `std` and `uuid` with no UI or router
imports, so both SSR harnesses include the real type rather than a copy.

The corrected implementation passed E005 on `cfa85271`; delivery still requires
exact-head mandatory CI and the protected merge queue (E006).

The mapping `edit_return_route` lives beside the
editor in `pages/new_project.rs`, not in `route.rs`, because the SSR harnesses
compile that page against their own route enums; one implementation is then
exercised by the route, SSR and browser tests. Plain list-origin navigation
now ends in the router's empty `?`. Existing browser URL assertions accept
exactly the same origin and project with or without that empty delimiter;
assertions that were already regex-only keep an anchored regex. The router
decodes the query once, so a standard percent-encoded same-project target is
honoured and a double-encoded one is not.

## Verification — 2026-10-22

The corrected code and tests passed the complete five-check command with
Nix-isolated PostgreSQL fixtures:

```sh
nix build .#checks.x86_64-linux.tests .#checks.x86_64-linux.browser \
  .#checks.x86_64-linux.clippy .#checks.x86_64-linux.sqlx-prepare \
  .#checks.x86_64-linux.treefmt --no-link --max-jobs 1 --cores 8 -L
```

- 191 core tests, 1,504 server tests and 352 tests across 15 external binaries
  passed. The 11 pre-existing manual server tests remain ignored.
- The complete browser runner passed, including detail-origin Back/Cancel,
  discard/stay, actual Save persistence, dirty history, reload, malformed and
  multiply encoded targets, and the existing list-origin/creation suites.
- Strict core/server all-target Clippy, SQLx cache verification and formatting
  passed; the application package built both SSR and WASM. Separate strict WASM
  application-binary Clippy also passed (`--features web --target wasm32-unknown-unknown --bin horae -- -D warnings -W clippy::perf`).
- Native Windows Chrome verified valid single-encoded and invalid double-encoded
  targets before/after reload, extra parameters, foreign targets, list Actions →
  Edit → Cancel, keyboard Back/Cancel, dirty discard and load-error recovery.
  The changed Back control was inspected at 390/768/1440px in both themes, with
  actual keyboard focus and activation. These checks used disposable local data.
- Independent technical and visual/UX reviews found no unresolved blocking issue.
  Review caught the repeated-decoding defect before publication; its failing
  browser regression was observed before the typed-query correction. Final
  review also checked the SSR harness assertions and settled theme captures.

The editor's existing inner overflow and clipped discard actions at 390px with
200% root text reproduce on the merged baseline, before this change. They remain
open for a separate feature 011 editor-reflow follow-up, outside this PR; this
record does not claim full enlarged-text acceptance for the whole editor.
Existing raw server-error copy and unmeasured shared focus-ring contrast also
remain outside this increment. Native browser zoom and physical-device coverage
are not claimed.
