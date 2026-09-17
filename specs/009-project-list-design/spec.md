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

No scheduling, manager filter, bulk selection/actions, pinning, deletion, new
project metadata or client-detail links to the current placeholder. Keep the
working inline create/edit form; a modal conversion is a separate interaction
change. Retain app-shell navigation and actual roles. Add missing design tokens
and utilities without changing existing radii, typography or component defaults.
No fabricated demo data or changes to the live imported workspace.
