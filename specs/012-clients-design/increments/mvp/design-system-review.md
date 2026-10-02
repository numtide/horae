# Clients MVP design-system review

Outcome: the finished Clients MVP is an ordinary extension of the incumbent
system. No replacement identity or shared-component change is required. This
documentation check preserves `DESIGN.md`; it does not regenerate system files.

## Evidence and boundary

Compared `DESIGN.md`, the direction contract in `visual-review.md` and the MVP
specification against `crates/horae/src/pages/clients.rs`, `clients/detail.rs`,
`clients/form.rs`, the client rules and token definitions in `horae.css`, generated
utilities, and the shared form, badge and modal implementations. Checked the
Clients and Client Detail handoff source for typography and panel proportions.

Read the independent `finish-review.md` disposition (`ship`) and sampled these
existing captures from `.scratch/clients-visual/round-two/`:

- `dark-1440x900-100-list.png`
- `light-1440x900-100-detail.png`
- `dark-390x900-100-form.png`

Those captures corroborate the source's type hierarchy, inherited themes,
panel treatment and shared editor. The manifest records loaded fonts and
viewport/text-size variants; the finish review owns inspection of all 64
required images. This handoff did not rerun browser, permission or persistence
tests. The unavailable context loader and detector were not retried and do not
count as passed checks.

## System comparison

| Area | Evidence and result |
|---|---|
| Typography | Page titles use the existing 34px display scale and tight tracking; panel titles inherit Newsreader. Controls use Instrument Sans; amounts, rates and counts use IBM Plex Mono. No font or type-scale replacement. |
| Color and depth | Both themes inherit the existing surface, border, accent and semantic tokens. Tables and panels use secondary surfaces, fine borders and the existing 16px grouped-list radius. No new palette, shadow or decorative asset. |
| Controls | The list reuses `Menu`, `Badge` and standard button classes. The editor reuses `Modal`, `FormGroup`, `Input`, `Select` and `Textarea`, including native modal focus protection and busy-state controls. |
| Local layout | The editor retains `--width-client-editor: 35rem`; search and the detail billing column reuse `--width-form-select: 20rem`. The client-only `--width-client-name: 16.25rem` preserves readable identity columns inside scrolling tables. A named container query restores two detail columns at 56rem of available content width. These are surface dimensions, not new global layout rules. |
| Local readability | Client table headings, the search placeholder and rate hint opt into the existing secondary-text color. Shared table/field defaults and both palettes remain unchanged. |
| Scope | Missing contacts, bulk/destructive actions, payment terms and aggregate financial panels are explicit MVP exclusions. The surface uses real states and links; the documentation does not promote those exclusions into system-wide rules. |

## Existing documentation differences

These differences predate this finish pass, as confirmed against the checked-in
CSS, and were not repaired:

- `DESIGN.md` describes semantic backgrounds as solid tints, while the existing
  information background uses rgba. Its solid-button description names
  strong-text ink, while the implementation uses `--color-on-pine`.
- The document primarily enumerates the dark palette; the existing stylesheet
  also defines the light palette. Its broad accessibility statement does not
  establish contrast for every shared label or placeholder. The Clients pass
  improves only its scoped uses, as recorded in `visual-review.md`.
- `DESIGN.md` has no token-bearing frontmatter and this worktree has no
  `.impeccable/design.json`. These are existing format/tooling gaps, not authority
  to migrate the design system. `PRODUCT.md` is absent and was not invented.

The scoped diff leaves `DESIGN.md`, shared components, generated utilities and
the utility generator untouched. No `.impeccable` files were created. Only this
review report was written by the documentation handoff. Later UI changes need
a corresponding evidence recheck; this is neither full feature 012 acceptance
nor whole-application accessibility or security certification.
