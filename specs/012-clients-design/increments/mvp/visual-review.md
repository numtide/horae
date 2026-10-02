# Clients MVP visual review

## Direction contract

- **Thesis:** find a client, inspect authorized work and maintain its existing
  billing profile without leaving dead-end detail pages.
- **Own-world:** inherit `DESIGN.md`, the Clients and Client Detail handoffs,
  shared semantic components and generated utilities. No new visual identity.
- **Story:** search/filter the catalog, follow a real client link, inspect
  projects and billing, then edit or continue an existing creation workflow.
- **First viewport:** list title and real actions above filters and a scrolling
  table; detail identity above a primary projects panel and a narrower billing
  column. Narrow layouts stack panels; forms retain native protected focus.
- **Form:** user-pinned Operate surface, scoped extension of the existing app.
  No concept seed or generated comp applies. The `.dc.html` handoffs are the
  visual reference; the increment specification controls functional omissions.
- **Finish:** unreviewed and undocumented is unfinished; this build ends with
  the finish review, the verdict, DESIGN.md, and every shipping raster carrying
  its provenance. This extension preserves DESIGN.md; it ships no new rasters.

## First inspection round

Baseline: `1393796`. Captures are local, synthetic evidence under
`.scratch/clients-visual/round-one/`; `manifest.json` records the loaded fonts,
theme, root font size and geometry checks.

All 64 images inspected: list/detail/form at 320, 390, 768 and 1440 CSS pixels,
100% and 200% root text, both themes; plus 1440×360 short screens and scrolled
form actions. No image was blank or an unresolved loading state. Full-page
screenshots include content outside the viewport; the modal scrim intentionally
covers the viewport, not the offscreen document extension.

Material findings for one correction batch:

1. The nonexistent `md:w-form-select` utility left search full-width and forced
   it below desktop actions. Use the existing width utility with a max-width.
1. Table identity columns compressed names into fragments. Retain horizontal
   scrolling and the handoff's 260px identity minimum, expressed in rem.
1. Project counts followed the contextual link instead of the title. Restore
   title/count/link reading order.
1. At 200% text the fixed billing column crowded out the primary project panel.
   Stack based on available content width relative to text size.
1. Table labels, search placeholder and the rate hint were below 4.5:1 contrast.
   Reuse the secondary-text token locally, without changing the global palette.

Executable assertions reproduced the failures before the fix, using the stopped
disposable cluster `/tmp/horae-browser.Zu4AvW`. Browser measurements included
2.98:1 dark table labels, 3.50:1 light labels, 3.88:1 dark rate hint and 2.86:1
light hint. Keyboard open, native modality/background inertness, forward/reverse
focus traversal, Escape and trigger-focus return passed in all 18 configurations.
Chromium's observed BODY focus stop is permitted; focus on a background control
is not. No shared modal change was needed.

## Confirmation and handoffs

The fullstack rebuild passed. All 64 second-round captures were inspected under
`.scratch/clients-visual/round-two/`; each shows its named surface. Search aligns
in desktop layouts, names retain a readable column, project counts precede the
link, 200% text stacks the detail panels and the targeted small-text contrast
checks pass. Horizontal scrolling remains confined to the tables. The full
keyboard matrix passes. No further build-thread polishing pass is planned.

The detector was attempted once on the changed UI targets and failed with exit
127: engine 0.1.7 is absent and its cache directory is not writable. This is an
unavailable check, not a clean detector result. The fresh independent finish
review receives the screenshots, source handoffs and this limitation.

Pending: independent finish review and incumbent-system documentation check.
Automated geometry assertions do not replace these handoffs.
