# Tasks: editor reflow

Scope is this directory's spec/plan, extending parent feature 011 US6/US7.
The parent requirements checklist is complete; historical completed tasks do not
certify this new regression. No backend or product decision is being reopened.

## Setup and regression

- [x] R001 Confirm measured causes and source mapping in `plan.md`; preserve the
  isolated branch/worktree and existing guards.
- [x] R002 [US6] Add failing inner-scroll, control and dialog geometry/keyboard
  assertions in `crates/horae/tests/browser/new-project-keyboard.cjs` and/or
  `project-edit.cjs`. Use their disposable fixtures and preserve all old cases.
  Red observed on `cfa85271` (`c402bc6b`, session 72386): form 556/390 and
  discard dialog 454/324 at 390px with 200% text; see `plan.md`.

## Implementation

- [ ] R003 [US6] Apply minimal wrapping/shrink constraints in
  `crates/horae/src/pages/new_project.rs` and its affected section modules;
  use `assets/css/horae.css` only for necessary scoped structural rules and
  generate the opt-in panel utility through `crates/horae/build.rs` into
  `assets/css/horae-utils.css`.
  Written through `3f954e5a`, not accepted: awaiting green actual-package
  regression checks. Measured causes and prior failures are summarized in
  `plan.md`; shared defaults and the persistent-actions contract are unchanged.
- [ ] R004 [US7] Verify configured/legacy Edit, creation conditional panels,
  discard/reload focus and unchanged saved/draft state with the existing browser
  suites; record evidence in this directory's `plan.md`.

## Delivery

- [ ] R005 Run full tests/browser/native/WASM/lint/format checks and native
  390/768/1440 themes/text verification; record explicit limits in `plan.md`.
- [ ] R006 Resolve two independent reviews, update the dependent base, publish
  the bounded PR and deliver only with mandatory exact-head CI green; record PR
  and acceptance in `plan.md`.

Dependencies: R001 → R002 (observed failure) → R003 → R004 → R005 → R006.
RF-001/002 map to R002–005; RF-003/004 to R002–005; RF-005 to R001/003–006.
No parallel edits inside this increment. Read-only review can accompany the
coordinator's serialized checks; it never substitutes for executed acceptance.
