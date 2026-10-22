# Implementation plan: editor reflow

## Context and constraints

Reuse the existing Rust/Dioxus form, native Modal, generated utility framework
and `np-*` structural CSS. No new dependency or abstraction. The parent feature's
data model and mutation contracts are unchanged; no new entity or endpoint.
Existing form and navigation tests remain authoritative regression coverage.

## Research and approach

Measured overflow comes from rem-sized preferred widths and flex children that
cannot shrink or wrap inside the remaining grid width. Discard actions share a
non-wrapping row. Confirm each cause against actual DOM geometry before fixing.
Prefer existing `min-w-0`, `max-w-full` and wrapping utilities at the owning
group. Use narrowly scoped structural CSS only where the utility layer cannot
express the relationship. Preserve useful desktop preferred widths.

Do not add `overflow:hidden`, remove controls, truncate action labels or shrink
text to conceal the issue. Do not change shared defaults to fix one caller.
Audit every New/Edit caller of a touched section and its conditional branches.

## Measured causes (R001)

Native Chrome on the `c408dcf7` production code (390px, 200% root text, seeded legacy
Acme project): `.np-scroll` client 380, scroll 543. At this size the content box is
about 300px. Pixel media queries already collapse the label grid; the overflow comes
from rem preferred widths that double with the text.

- Planning dates (`new_project/date_field.rs`): the `np-start`/`np-end` button
  and its `form-group` are 360px (`--width-planning-date`), right edge 400. The
  wrapper does not wrap, so a set date's clear control adds to the row. A button is
  not a compressible control, so `max-width: 100%` alone does not lower the group's
  minimum width. Candidate fix: a scoped `min(100%, token)` width, the idiom of
  `.np-tags .chip-input-field`, plus a wrapping wrapper.
- Task and person overrides (`new_project/tasks.rs`, `team.rs`): each prefix +
  `w-30` input + suffix label is a non-wrapping flex row. The input renders at
  240px because `max-w-full` resolves against a label sized to its own contents.
  Rate label right edge 539; bill/cost 528/543. The controls cell is about 148px.
  Candidate fix: wrapping labels first; add `min-w-0` on the inputs only if they
  still hold 240px.
- Discard confirmation (`new_project.rs`): the `.modal-actions` buttons are
  `flex: 1` in a non-wrapping row inside a roughly 230px body, so Keep editing ends
  at 470 against a 390 dialog. The same markup is used by the reload, New client
  and task-access dialogs. Candidate fix: the `flex-wrap` utility at those callers,
  as `projects.rs` already does; the shared `.modal-actions` default stays as is.
- Not measured as overflowing: `.np-code` (compressible input in a wrapping row)
  and the billing budget/fee wrappers, which don't render for legacy Acme. Change
  these only if the creation matrix reports them.

The existing 200% audit ran only at 1440px and measured `.np-page` and the dialog
element. Focus scrolls the inner `.np-scroll`/`.modal` scroller, which hid this
overflow. R002 checks those scrollports directly.

## Observed red (R002)

Tests committed in `c402bc6b`. A focused `project-edit` run against the cached
`cfa85271` server (session 72386, derivation `7li4ymlilpygvk9j115x07wcayfg59w6`)
exited 1. At 390px with 200% text, the form scroll width was 556 against a client
width of 390. The date group with a set date spans 40..474; its clear control
spans 408..474. The discard dialog's scroll width was 454 against a client width
of 324, with Keep editing at 272..454. The earlier normal-text Save/reload cases
at 390/768/1440 passed.

## Written correction (R003, not yet accepted)

- `new_project/date_field.rs`: the date wrapper wraps (`flex-wrap`), so a set date's
  clear control moves below the field. The button keeps its 180px preferred width
  but is capped at its group (`max-w-full`).
- `new_project/basics.rs` + `horae.css`: the dates row is marked `np-dates`;
  `.np-dates > .form-group { min-width: 0; }` lets each group shrink to the line.
  `FormGroup` takes no class, so this one scoped structural rule is necessary.
- `new_project/tasks.rs`, `team.rs`: task rate/budget and person bill/cost/budget
  labels add `flex-wrap min-w-0 max-w-full`. The label then narrows to its cell,
  the `w-30 max-w-full` input caps at the label, and the prefix and suffix wrap.
- `new_project.rs` (discard, reload), `basics.rs` (New client), `tasks.rs` (task
  access): `modal-actions flex-wrap`, as `projects.rs` already does. The shared
  `.modal-actions` default is unchanged.

Focused run 41504 on `4c552c1b` (package `aa6b6c2s5xivchzmj1ysbvvy4nr91n9r`)
exited 1. Dates and dialogs fit. At 390px with 200% text the form still scrolled
403 against 390, and the only offender was the task billable shortcut "None" at
289..403. Native DOM checks at 320px also showed `#np-code` overflowing (right edge
360 against a 310 client). Follow-up correction, also not yet accepted:

- `new_project/tasks.rs`: the Billable All/None shortcut group adds `flex-wrap`.
- `new_project/basics.rs`: `#np-code` adds `max-w-full`, capping its 160px
  preferred width at its field cell.

The checks now also cover 320px at 200% text (legacy Edit loop, creation matrix
and dialogs) and the reload confirmation at 390px (geometry, keyboard actions,
focus return, local input kept until an explicit reload). Each field cell
(`.np-row > .min-w-0`) and each `.np-row-controls` is measured as well as the
scrollport, so a control spilling into the gutter fails. Only control boxes are
compared, so text scrolling inside an input is never reported. The Edit suite
restores the 1440px viewport its later cases used before the enlarged-text loop.

At 100% text every affected row already fits, so preferred widths and layout are
unchanged. No text sizes, overflow clipping, truncation, tokens, utilities or
shared component defaults were changed.

## Implementation and validation

1. Extend the existing isolated browser checks with a failing enlarged-text
   scenario for the inner scroll area, affected controls and dialog actions.
   Include a legacy editor with real selected task/person rows and creation
   conditional panels, rather than only an empty form.
1. Make the smallest markup/utility changes in `pages/new_project.rs` and its
   affected section modules. If necessary, adjust only scoped `np-*` structural
   rules in `assets/css/horae.css`; generated utilities go through `build.rs`.
1. Preserve keyboard action/focus, values, pending controls and draft/edit guards.
   Run the existing conditional keyboard matrix and whole-browser regression;
   compare unrelated screens if any shared CSS/generated utility is touched.
1. Verify native Chrome in both themes and the specified widths/text sizes.
   Record actual geometry and screenshots, not just document-level overflow.
1. Run native/WASM builds, strict lint, tests, SQLx check and formatting. Obtain
   independent technical and visual/UX reviews, then exact-head CI and protected
   delivery. No real-account mutations or persistent-database migrations.

## Constitution and dependencies

All five principles remain satisfied: no money/time arithmetic, domain, datastore
or mutation-path changes; reproducible Nix checks remain mandatory. No exception.
The branch starts from editor-return commit `cfa85271`; update it after that
increment merges and repeat affected checks. At most two increments and one
heavy build at a time; the coordinator owns all browser/DB/build resources.

The parent data model, server contracts and quickstart are reused, not rewritten.
The acceptance checklist above is the bounded UI contract and validation guide.
Spec Kit has no installed agent-context update script or extension hooks here;
record this bounded context rather than claiming absent commands ran.
