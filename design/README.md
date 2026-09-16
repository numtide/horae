# Horae design handoff

HTML/CSS/JS prototypes for Horae's screens and shared visual components. These
files are design references, not production application code.

## Source

- Archive: `Horae (1).zip`, exported on 2026-09-15 and imported on 2026-09-16.
- SHA-256: `9e5190e786b55cc728706413bbbf95bd4ef4912f6dddcb06b2123fd6f0b1277d`.
- The archive root maps directly to `design/project/`; file contents and names
  are preserved, including the two screens numbered `12_`.
- All 34 archive files are present locally. The existing `.gitignore` excludes
  `.thumbnail`, so 33 prototype/support files are versioned. The archive does
  not contain a README; this guide is maintained by the repository.

## Contents

- [Design system](project/Design%20System.dc.html): palette, typography and tokens.
- `project/app/01_*.dc.html` through `12_*.dc.html`: 13 screen prototypes,
  including both the client list and client detail.
- `project/app/`: shared rail, user menu, admin navigation, dropdown, dialog,
  date picker, period navigation, empty state, toast, toggle and segmented control;
  `DevBar` is a prototype-only state selector.
- `project/app/format.js`: prototype display-format examples.
- [Components](project/foundations/Components.dc.html) and
  [Logo](project/foundations/Logo.dc.html): foundation references.
- The three `support.js` files support the exported prototypes.
- [Implementation gaps](IMPLEMENTATION-GAPS.md): source-based comparison with
  the application and existing feature specifications, including pending PRs.

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
local-link/component-name checks found no other missing targets; this is not a
browser or runtime validation of the export.

The implementation inventory records conflicting or unsupported mockup behavior,
including currency aggregation, account switching, authentication and roles.

Last reviewed: 2026-09-16.
