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

The two baseline widths measure different states. The native figure (543 against
380\) has no date set and a 10px classic scrollbar. The test-run figure (556
against 390) is headless with a date set, so the clear control adds width and no
scrollbar takes space.

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

Focused run 2237 on `420457fc` (package `ry3z6fvi42ji8xh5hm5fk98ncvv487f5`) exited
1 at 320px with 200% text, before reaching 390, so `420457fc` has no 390px/200%
result. Legacy Edit: form scroll 351 against 320; Add everyone 81..325; the
Visibility options are 263px wide against a 240px field cell; the task access
"Everyone" button is 164px wide against a 96px `.np-row-controls`; the discard
dialog scrolls 277 against 254 with no control outside its padding box (Keep
editing reaches into the body's padding). Short-height native check on the same
code: at 390x568 the form area was 66px tall beside a 353px footer, and at
320x568 it was 0px beside a 388px footer. The pinned shell clipped the form.

Creation run 18262 (after the calendar synchronization in `34948bc0`) exited 1.
All fourteen branches pass at 390/768/1440 with 100% text, and at 1440 with 200%
text the six hourly branches, Single fee and No budget pass. The first failure is
1440/200% Milestones: inner scroll 1386 against 1176, amount at 1002..1282, remove
at 1306..1386. The 900px viewport rule that stacks milestones does not apply
there, but the rem-sized columns no longer fit the narrower list.

Native checks on `420457fc` at 320px with 200% text: the set date button (230px)
wraps its text with no clipping (scroll width 229 equals client 229). The calendar
panel sits at 8..312 and scrolls vertically (picker 691px tall inside a 484px
panel), and Today is reachable, so that scrolling is intended.

Third correction, written and not yet accepted:

- `horae.css`, at 480px and below: the editor gutter becomes
  `min(var(--space-5), 6.25vw)`, still 20px at 100% text from 320px up and no longer
  doubled by enlarged text. Row overrides (`.np-row-controls`) take the full row
  under the person or task, so a single-word action such as Everyone keeps its
  full width.

- `horae.css` + `new_project.rs`: the pinned footer stays, with
  `.np-footer { max-height: 25%; }` and the existing `overflow-y-auto` utility.
  When the actions need more than a quarter of the editor, they scroll within the
  footer, so the fields always keep at least three quarters. (A first 50% cap
  left the fields too little room; see the footer-cap evidence below.) The shell, the
  navigation and the pinned layout are unchanged. This replaces an earlier
  short-phone rule that moved the editor into document scrolling. It was withdrawn
  because it also removed the pinned footer at 100% text and contradicted the
  existing 390x320 and 390x420 navigation assertions in `new-project.cjs`.

- `horae.css` + `new_project/billing.rs`: the milestone list is an inline-size
  container (`np-milestones`, the `client-detail` pattern). Below 30rem it uses
  the existing stacked layout. At 100% text the list is wider than 30rem wherever
  four columns show; at 200% text it stacks whenever the doubled columns would
  not fit.

- `new_project/team.rs`: Add everyone adds `flex-wrap justify-center max-w-full`,
  so its icon can sit above the label. The space is tight; acceptance requires
  the button-clipping check to pass, not an exemption for this control.

- Discard, reload, New client and task-access dialogs: `modal-body px-4 sm:px-6`,
  as an existing `projects.rs` dialog does locally. Only the horizontal padding
  changes below 640px; the vertical padding and desktop padding stay as they
  are. (`sm:p-6` is not a generated utility; `build.rs` only has responsive
  `px-6`.) The shared Modal styles are unchanged.

- `new_project/basics.rs`: the `fieldset` around Currency adds `min-w-0`, as the
  editor's other fieldsets already do. A native measurement at 390x900 with 200%
  text showed `#np-currency` at x 24..486 (461 wide) because the fieldset keeps
  its default `min-width: min-content`. That made it 461 wide inside a 331px field
  cell, with the form scrolling to 486. The shared select field and the other
  fieldsets are unchanged until the full creation matrix reports them.

Native diagnosis of the footer cap, not acceptance: with the shell at `100dvh`
and `.np-page` at 100% restored, adding `.np-footer { max-height: 50%; overflow-y: auto }` by hand in DevTools gave:

| Viewport | Text | Form | Footer | Document height |
|---|---|---|---|---|
| 320x568 | 200% | 192 | 192 | 568 |
| 390x568 | 200% | 210 | 210 | — |
| 390x568 | 100% | 388 | 105 | 568 |
| 390x320 | 100% | 140 | 105 | 320 |
| 844x390 | 200% | 195 | 195 | 390 |

Tab order through that layout has not been verified yet.

The checks now also measure each region's content box, including dialog bodies,
report a button whose content is clipped, check that the calendar fits the
viewport (and scrolls when tall) with Today reachable, and run the actual Tab
order at 320x568 and 390x568 (200% text), 844x390 (200%) and 390x568 (100%, a
regression guard). Every focused control must show inside its own visible
scrollport: form controls inside `.np-scroll` above the footer, footer actions
inside the footer's area, dialogs inside the viewport. Both suites collect
geometry problems, including the audit's vertical and horizontal focus checks,
and assert them once: after the enlarged-text sections in the creation suite and
after the data-integrity checks in the Edit suite. The creation suite prints the
collected list if a fatal assertion stops it first. Focus, names, Tab order and
data assertions stay fatal. Creation cases at 100% text keep empty dates again;
each 200% case sets a date first.

Preferred control widths remain unchanged. At 100% text, narrow assignment rows
now use the full row for overrides, and the pinned footer scrolls within itself
only when it would take more than a quarter of the editor. At 390x568 that is
about 123px against a 105px footer, so the cap does not apply; desktop is also
unaffected. At 390x320 the earlier measurement (a 105px footer in a 245px editor)
means the 25% cap does apply at 100% text: the footer stays pinned, shows about
61px and scrolls the rest. That needs checking against the existing 390x320 cases
before acceptance. These intentional accessibility changes still need
visual verification. No text
sizes, overflow clipping, truncation, tokens, utilities or shared component
defaults were changed.

Focused run 35160 on `f3c7d7b3` exited 1 after all 98 creation branches, 13
dialog audits and 4 short-screen audits, with 53 collected geometry problems.
This is not acceptance. Measured causes:

- At 390px and 200% text, the Billable rates fieldset kept its default
  `min-width: min-content` and was 371px wide. Its option labels spanned
  90..461, the project-rate input 171..411, and the fee and budget inputs reached
  406\.
- At 320px and 200% text: the same causes, plus the fee-mode segmented items
  (Single fee 65..325, Milestones 65..338), the second-tax name input and its
  remove button (to 340), and the task-access dialog labels (197 against a 190px
  body content box).
- The negative left edges reported for Back to Projects and other focused
  controls are what's left of the horizontal scroll these overflows cause.
  They are not patched separately.
- At 320px and 200% text, some non-billable branches scroll 325 against 320 with
  no control outside. The overflow check now names up to three innermost
  elements (text, legend or container) that spill, so the next run identifies
  them.
- Short screens: a radio card 272px tall is taller than the 192px form area left
  by the half-height footer cap. This led to the 25% cap below.

Footer-cap DOM diagnosis in native Chrome, with real Acme client and name values
in a temporary database and Save enabled. This is not acceptance; it waits for
the built artifact and a real Tab run. With a 25% cap, at 320x568 and 200% text
the form was 288.5px and the footer 96.2px. The whole 272px Non-billable radio
card was visible, and Save and Cancel (84px) were fully inside the footer when
actually focused. The same held at 390x568 and 844x390. A first attempt with
Save still disabled could not focus it and was discarded. The second confirmed
`activeElement === control` and `disabled === false` in all nine cases.

Fourth correction, written and not yet accepted:

- `new_project/billing.rs`: the Billable rates and Project fee fieldsets add
  `min-w-0`. The project-rate, budget-value and fee-amount groups add
  `flex-wrap min-w-0` beside their existing `max-w-full`, so the currency prefix,
  input and suffix wrap inside the option or panel.
- `new_project/invoice_defaults.rs`: the second-tax group adds `max-w-full`, so
  its `w-40 max-w-full` name input caps at the row.
- `new_project/tasks.rs`: the task-access fieldset adds `min-w-0`.
- The fee-mode segmented items have no `nowrap`; their text already wraps at
  spaces. "Milestones" is one word: radio, gap and `--space-4` padding on both
  sides make about 273px, while the fee panel leaves about 190px at 320px with
  200% text. The fieldset fix doesn't close that gap. It needs a separate,
  authorized local decision, such as reducing the panel or item padding at narrow
  widths in editor-scoped CSS. The shared `.segmented` styles stay unchanged.

Separate Edit run 29544 on `f3c7d7b3` exited 1 after 3 normal-text and 7 reflow
cases. The form scrolled 342 against 320 and 408 against 390 with no control
outside. At 320px the discard dialog body left 190px of content for a 200px
"Discard changes" button, so the dialog scrolled 261 against 254. There were no
footer or Tab failures in the legacy editor, and no regressions in normal-text
save or data. Candidate, pending the next run and not accepted: the four editor
dialogs use `modal-body px-3 sm:px-6`, as a `projects.rs` dialog already does.
At 200% text that is 24px a side instead of 32, so 206px of content for the
200px button and the 197px task-access labels. Vertical and desktop padding are
unchanged, and the buttons are not touched. (`sm:px-5` is not a generated
utility, and the generator stays unchanged.) The overflow check's fallback now
also measures text nodes directly, so text that spills without its own element
is named.

Baseline diagnosis, not acceptance: run 22143 used the `fa26b6ab` tests against
the `f3c7d7b3` binary and exited 1 (log derivation
`c361h6x0p88x1wsm6ia14517ap8j8y6n`). It identified the "hours" suffix at 275..342
and 341..408, already covered by the wrapping groups in `fa26b6ab`. In the person
identity cell, the "Project manager" role had a client width of 0 and a scroll
width of 106. The truncated "Admin User" name is intentional and is not treated
as overflow. In native New with the Development task and Admin User, the form
scrolled 320 against 310, with the role and name heavily compressed. At 200%
text the person grid spends 40 + 64px on its fixed columns, 64px on gaps and
80px on padding, so the identity (56px avatar plus a 32px gap) has no width left.

Fifth correction, written and not yet accepted:

- `build.rs`: a new opt-in `px-project-panel` utility beside `px-project-form`,
  `padding-inline: var(--project-panel-gutter)`. `horae-utils.css` was
  regenerated by compiling the existing generator, which has no dependencies
  beyond the standard library, with `rustc` and running it. Before the change it
  reproduced the committed file byte for byte. Afterwards the only difference is
  that one line. No other caller used the name. No defaults change.
- `horae.css`: `.np-page` defines `--project-panel-gutter: min(var(--space-5),
  6.25vw)`. That is 20px at 100% text from 320px up, and 20px instead of 40 at 200%
  text on a 320px screen. `.np-page .np-assignment-row` sets `column-gap:
  min(var(--space-4), var(--project-panel-gutter))`: 16px at 100% text, and at most
  the panel gutter when text is enlarged on narrow screens. The page prefix
  outranks the row's `gap-4` utility. Row gaps are unchanged.
- `new_project/tasks.rs`, `team.rs`: every `px-5` in the section headers, notes,
  rows, empty states, errors and add controls becomes `px-project-panel`.
  `billing.rs`: the billing panel's `p-5` becomes `py-5 px-project-panel`.
- `new_project/team.rs`: the person identity (avatar and text) wraps in both
  callers, the member row and the manager-only row, so the text can sit under the
  avatar. Its text block keeps `min-w-0`, and the name keeps its intentional
  truncation.
- `new_project/billing.rs`: the fee-mode segmented control and its items add
  `max-w-full`, and the items `flex-wrap`, so the radio and label wrap within the
  panel instead of setting a larger minimum. Font sizes are unchanged.

At 100% text the panel padding stays 20px and the row gap 16px, so normal and
desktop layouts are unchanged. Both overflow checks now also report assignment-row
text squeezed to zero width or wider than its own box, excluding intentionally
truncated names.

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
