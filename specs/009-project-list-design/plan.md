# Implementation plan

1. Reuse the existing ProjectList, Menu, Combobox, NavIcon and empty-state styles.
1. Align header actions, wrap filters, replace the empty message with actionable
   empty states and remove unsupported numeric columns.
1. Replace the inline-width budget bar with native progress styled by tokens.
   Keep data loading/failure distinct from a genuine zero-spend project.
1. Add browser coverage for permissions, empty states, filtering, progress,
   keyboard access and responsive geometry. Update the renamed action in the
   existing action-error regression harness.
1. Run server Clippy, WASM check, formatting and isolated browser checks; publish
   one review PR. CI's full Nix gate remains required before merge.
1. Refine visual fidelity with additive utilities, existing chips and shared
   content-sized table tracks. Allow Menu/Combobox trigger utilities without
   changing compact defaults; cover both defaults and opt-in classes in tests.
1. Compare computed shared styles and dimensions on other routes before/after;
   exercise responsive layouts, menus and navigation in the isolated preview.

## Constitution check

No schema, queries, authorization, domain arithmetic, dependencies or mutation
paths change. All existing actions continue through their server functions.
Validation uses the Nix toolchain. No principle exceptions are required.

## Client-detail navigation (2026-10-25)

Implements the spec's "Client-detail navigation" amendment. `setup-plan.sh`
kept this plan; no template sections are regenerated for one navigation change.

### Technical context

- Change site: `pages::projects::ProjectList`, where bands are built from the
  already loaded `ProjectOverview`. Destination: the existing
  `Route::ClientDetail { id }` (`pages::clients::detail`).
- Source access: `project_rows` joins clients only within the viewer's
  organization and, under the legacy policy, requires the client to exist.
  `ProjectOverviewClient` is documented as context, not directory access, so a
  link must rest on the destination's rule, not on the row alone.
- Destination access: `client_detail_view` → `ClientReadScope::can_read` is
  `legacy || ClientReadAll`, filtered by organization. Project Detail mirrors
  this as `can_open_client`. Under the legacy policy every active member of the
  organization passes, as `policy_zero_keeps_legacy_catalog_billing_and_manager_affordance`
  asserts for admin, manager and member; foreign and missing clients fail closed
  (`foreign_missing_and_rebound_requests_fail_closed`).
- Unknowns: none remaining. Scoped-policy rows lack the client-read grant, so
  links are limited to `!overview.canonical_permissions` (spec Clarifications).

### Decisions

- Link when the overview is legacy-policy and the row has client details;
  otherwise render the label as text. Alternative rejected: adding a list-level
  grant flag (a new response field, outside this slice).
- Group and key bands by `client_id`. Sort by lower-cased client name, then
  `client_id`, then lower-cased project name, so equal names stay contiguous
  per client and the visible order is otherwise unchanged.
- The link is a Dioxus `Link` inside the existing `.proj-group` band; it takes
  the global link color, which equals the band's current `text-primary`.
  Plain labels keep the band's existing color; the handoff only shows linked
  bands. No CSS, token or shared default changes.
- No research.md, data-model.md or contracts: no entity, response or endpoint
  changes. Validation steps live in tasks.md and acceptance.md.

### Files

- `crates/horae/src/pages/projects.rs` — band grouping and rendering only.
- `crates/horae/tests/detail_navigation/overview.rs` — SSR tests for hrefs,
  same-name and case-only grouping, unknown-client and policy-one fallbacks, no
  additional reads, and navigation to the client route and back.
- `crates/horae/tests/browser/client-context.cjs` — keyboard navigation from a
  band to Client Detail as a real legacy manager and member in the disposable
  runner database.
- `specs/009-project-list-design/{spec,plan,tasks,acceptance}.md`, checklist.

### Constitution check

No schema, query, migration, mutation, authorization, dependency or domain
arithmetic change; the destination's server check is untouched. Builds and
tests run in the Nix shell against disposable databases. No exceptions needed.
Re-checked after design: unchanged.
