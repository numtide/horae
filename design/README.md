# Horae design handoff

HTML/CSS/JS prototypes for Horae's screens and shared visual components. These
files are design references, not production application code.

## Source

- Archive: `Horae (3).zip`, exported and imported on 2026-09-21.
- SHA-256: `4be94f9599b304924570504d9aeef111efe36d29a3c710a808b25033e5bff2ec`.
- The archive root maps directly to `design/project/`; file contents and names
  are preserved, including the two screens numbered `12_`.
- All 37 archive files are present locally. The existing `.gitignore` excludes
  `.thumbnail`, so 36 prototype/support files are versioned. The archive does
  not contain a README; this guide is maintained by the repository.

## Contents

- [Design system](project/Design%20System.dc.html): palette, typography and tokens.
- `project/app/01_*.dc.html` through `13_*.dc.html`: 14 screen prototypes,
  including both the client list and client detail, plus New Project.
- `project/app/`: shared rail, user menu, admin navigation, dropdown, dialog,
  date picker, period navigation, empty/error states, skeleton loading, toast,
  toggle and segmented control;
  `DevBar` is a prototype-only state selector.
- `project/app/format.js`: prototype display-format examples.
- [Components](project/foundations/Components.dc.html) and
  [Logo](project/foundations/Logo.dc.html): foundation references.
- The three `support.js` files support the exported prototypes.
- [Implementation gaps](IMPLEMENTATION-GAPS.md): historical source-based comparison
  from 2026-09-16, not a current implementation audit or live PR status.

## Latest export changes

- Adds [New Project](project/app/13_New%20Project.dc.html),
  [ErrorState](project/app/ErrorState.dc.html) and
  [Skeleton](project/app/Skeleton.dc.html).
- Updates 14 existing prototype files, including the design system, Projects,
  Project Detail, shared Dropdown and foundation component examples.
- Projects now links to the dedicated creation screen and includes loading/error
  examples. Project Detail revises its charts and presentation.
- This import only updates design references. It does not implement these flows
  or expand existing feature specifications. Reconcile the historical gap inventory
  against current application code before using it to plan further work.

## Using the handoff

Read the relevant screen's source in full and follow its component imports before
implementing that screen. Do not infer priority from an export's selected page.
Do not render or screenshot prototypes unless explicitly requested.

Implement approved flows using Horae's existing Dioxus components and tokens,
following [DESIGN.md](../DESIGN.md). Do not copy demo records, simulated timers,
no-op actions, floating-point monetary calculations or prototype-only controls
into the application. Keep exported source unchanged so future updates remain
comparable.

Requirements and implementation plans belong under `specs/<NNN-feature>/`;
the [constitution](../.specify/memory/constitution.md) governs product invariants.
The prototype is not authority to add authentication methods, permissions,
destructive operations or features without specifying their behavior.

## Known handoff limitations

The export contains a relative link from `project/foundations/Components.dc.html`
to `07_Reports.dc.html`, but the report screen is under `project/app/`. This
upstream link is preserved rather than patched in the imported source. Static
local-link/component-name checks covered 141 local URLs and 114 component imports
and found no other missing targets. All 37 extracted files match the imported
bundle byte-for-byte; this is not a browser or runtime validation of the export.

The implementation inventory records conflicting or unsupported mockup behavior,
including currency aggregation, account switching, authentication and roles.

Last reviewed: 2026-09-21.
