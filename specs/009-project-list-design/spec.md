# Project list design alignment

## Scope

Align the existing Projects list with `design/project/app/05_Projects.dc.html`.
This is a presentation slice of feature 001, not the project dashboard (004).
Preserve existing server functions, permissions, billing calculations and exports.

## Acceptance requirements

- Managers can open the existing create/edit form from a prominent New project
  action. Administrators additionally have a working Import link. Members have
  neither management nor import actions.
- Search, scope and client filters remain functional and fit narrow viewports.
- Client-grouped rows show project identity, billing type, budget, spent and
  remaining values. Omit unsupported Scheduled/Delta columns. Never substitute
  zero for spend data that is loading or failed.
- Budget consumption uses a labelled native progress element, without inline
  styles. Existing money/hour formatting and calculation semantics stay intact.
- An empty project collection explains how to get started, with role-appropriate
  actions. A filtered empty result offers a reset to all clients, empty search
  and active projects; it does not claim that the workspace has no projects.
- CSV/XLSX export, edit and archive/unarchive remain available as before.
- At 320, 768 and 1440 CSS pixels, controls remain reachable and rows scroll
  inside their container, not the document. Keyboard users can reach the scroller
  and open row menus; long project names stay in the name column.
- Match the handoff's 34px heading, 14px row text, inline billing-type chips,
  accented client bands and 16px list-card radius through tokens and utilities.
  Full currency codes, large amounts and negative remaining budgets must not
  overlap adjacent columns; header and row tracks remain aligned.
- Preserve shared component defaults and all existing utility definitions.
  Verify other screens and the component gallery at mobile, tablet and desktop
  widths before accepting page-level design refinements.

## Deliberate handoff deviations

Historical record of the feature 009 delivery; later features superseded some
items (feature 010 added bulk selection/actions).

No scheduling, manager filter, bulk selection/actions, pinning, deletion or new
project metadata. Client-detail links were deferred while Client Detail was a
placeholder; feature 012 replaced it, so they are now in scope (see below). Keep
the working inline create/edit form; a modal conversion is a separate interaction
change. Retain app-shell navigation and actual roles. Add missing design tokens
and utilities without changing existing radii, typography or component defaults.
No fabricated demo data or changes to the live imported workspace.

## Client-detail navigation (amendment 2026-10-25)

The handoff's client bands link to Client Detail. Feature 012 replaced the
placeholder this deviation depended on, and Project Detail already links its
client under the legacy policy, so the Projects list now follows the handoff.
This changes navigation only; no data, permission or product policy changes,
and it neither activates nor completes the scoped-permissions policy (policy one).

### Clarifications

#### Session 2026-10-25

Resolved from checked-in contracts; none needed new product input.

- Q: Who gets a link? → A: Under the legacy policy, every signed-in viewer of
  the list, because Client Detail admits every active user to clients of their
  own organization. Under scoped permissions (policy one) the band keeps its
  current plain-label behavior: the list does not carry the client-read grant,
  so it offers no link rather than guessing.
- Q: What identifies a group? → A: The client's identifier. Two clients with
  the same name form two bands, each linking to its own record.
- Q: What does a project without client details show? → A: The existing
  "Unknown client" label, as plain non-interactive text in the band's current
  color.

### User story — open a project's client (P1)

A user scanning projects by client follows the client name to that client's
detail page, by pointer or keyboard, and returns with the browser's Back.

**Independent test**: list projects of two same-named clients plus a project
without client details, as a manager and as a member, and follow each heading.

**Acceptance scenarios**:

1. **Given** a legacy-policy manager or member viewing projects of a known
   client, **When** they activate its heading, **Then** the detail page for
   that client identifier opens.
1. **Given** two clients sharing a display name, or names differing only in
   case, **When** the list renders, **Then** each has its own band and link,
   and no project appears under the other client.
1. **Given** a project whose client details are absent, or a scoped-policy
   reader, **When** the list renders, **Then** the band shows its label as
   text with no link.
1. **Given** any filter, scope, tag, search, selection or export state, **When**
   headings become links, **Then** those behaviours, ordering by client then
   project name, budgets and loading/error states are unchanged.

### Requirements

- **FR-N1**: A known client heading is a standard link to that client's detail
  page, reachable with Tab, activated with Enter and named by the client name.
- **FR-N2**: Links appear only for legacy-policy lists; under policy one the
  band stays a plain label, as today (see Clarifications). The destination's
  own authorization is unchanged.
- **FR-N3**: Projects are grouped by client identifier; display names only
  label and order the bands. Clients with equal names never merge.
- **FR-N4**: Missing client details keep the "Unknown client" label, as text.
- **FR-N5**: No new data request, field, permission, style token, dependency
  or migration, and no additional reads when the list loads or renders.
  Existing band styling, including the plain-label color, is reused.

### Edge cases

- Same-named clients, including names differing only in case, stay separate
  and keep a stable order.
- A client that becomes unreadable after the list loaded shows Client Detail's
  existing not-found state; the list does not re-check per click.
- Other organizations' clients never appear: the list joins clients only within
  the viewer's organization, and Client Detail filters by it again.

### Success criteria

- **SC-N1**: Every known client band in a legacy-policy list opens the matching
  client, for both a manager and a member.
- **SC-N2**: With two same-named clients and a case-only variant, the list
  shows one band per client, each containing only its own projects.
- **SC-N3**: Unknown-client and policy-one bands contain no link and keep their
  current color; rendering issues no client-catalog or extra overview reads.
- **SC-N4**: Existing Projects design, bulk-action and client-context checks
  still pass.

### Assumptions and limits

- No live Harvest session was available. This follows the checked-in handoff
  (`05_Projects.dc.html`), the implemented Client Detail route and its existing
  authorization tests, not a fresh Harvest comparison.
- Scoped-policy readers who hold the client-read grant get no link here yet;
  offering one needs that grant on the list and belongs to the scoped
  permissions work.
