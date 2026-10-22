# Project editor enlarged-text reflow

This bounded correction reuses feature 011 US6/US7, FR-018/019/021 and
SC-006/007. It does not regenerate the completed creation/editing package or
change its functional contracts. Branch: `fix/project-editor-reflow`.

## Evidence and scope

On the merged header baseline, a 390px viewport with 200% root text produces
an inner editor scroll width of 543px versus 380px available. Planning-date
fields and task/person rate groups overflow. The discard dialog clips its
Keep editing action. The same defects reproduce before and after the separate
editor-return change. This is existing UI-parity inventory P02/X01.

The visual reference remains `design/project/app/13_New Project.dc.html`,
`DESIGN.md` and the existing component mapping in `../contracts/new-project.md`.
Harvest's functional workflow is unchanged: no new fields, actions or account
operations are proposed. Reference observations for editor navigation remain in
`../../004-project-dashboard/edit-return-increment.md`.

## Acceptance contract

- RF-001: New Project and Edit retain their designed hierarchy, shared controls
  and persistent actions at 390/768/1440px in both themes, at 100% and 200% text.
  Check 320px as a boundary regression; do not reduce font sizes to make it fit.
- RF-002: Visible field groups, dates, task/person overrides and conditional
  billing fields reflow within the editor's available width. Check the inner
  scroll area as well as the document; hiding overflow is not a solution.
- RF-003: Discard and reload confirmations keep both actions visible and
  keyboard reachable, with unchanged safe dismissal, focus return and semantics.
- RF-004: Actual Tab navigation reaches enabled controls without the footer
  obscuring them. Input, selected values, dirty state, validation and save/cancel
  behavior remain intact. Creation drafts and explicit project edits stay separate.
- RF-005: Default shared controls, unrelated screens, permissions and server
  contracts do not change. No SQL, schema, dependencies or new data are required.

No product clarification is outstanding: these are already-approved accessibility
requirements, not a new interpretation of Harvest. Raw error-copy cleanup, focus
token contrast, archived editing and dashboard metrics are separate increments.
