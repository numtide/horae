# Harvest behavior reference

Inspected 2026-09-29 using the existing Playwright MCP bridge to Windows Chrome
and Harvest's official help center. This is behavioral evidence, not permission
to erase existing development or Harvest data.

## Observed in the authenticated browser

- The project detail Actions menu contains Pin, Duplicate, Archive and Delete.
- Delete opens a confirmation dialog requiring the word `DELETE`. Its warning
  explicitly includes project time, expenses and linked retainers, while keeping
  invoices and removing their project links. The destructive confirmation was
  never submitted; leaving the page dismissed the dialog.
- Duplicate opens the New Project form populated from the source: the inspected
  name has a `COPY` suffix, the project code is blank, and the original client,
  fixed-fee type and fee amount are selected. Save project is a separate action;
  no duplicate was saved. Empty source fields do not establish a general rule
  about whether all dates, notes or tags are copied.
- The Invoices tab shows a linked draft in its pre-tax subtotal and exposes
  Unlink. Link invoice is disabled with an explanation when the client has no
  unlinked invoices. Unlink was not invoked, so its persistence/confirmation
  behavior is not claimed as browser-tested.
- The real fixed-fee summary permits negative uninvoiced value when linked
  invoice value exceeds the agreed fee, including a draft contribution.
- Tasks/Team/Invoices, progress/hours charts, zero-time task rows, report links
  and an incomplete-cost warning are present. The inspected export menu offers
  CSV and Excel; the Horae handoff's PDF summary is not proven by this account.

Snapshots are retained locally in `.scratch/playwright-windows/output/`:

- `page-2026-09-29T18-49-42-269Z.yml`: project Actions menu.
- `page-2026-09-29T18-49-57-870Z.yml`: delete confirmation, not submitted.
- `page-2026-09-29T18-50-35-735Z.yml`: invoice tab and disabled Link explanation.
- `page-2026-09-29T18-51-14-570Z.yml`: unsaved duplicate form.

The bridge is connected and navigation/snapshots work. Some normal clicks timed
out while the tab reported itself hidden; subsequent detail-menu and delete-
dialog clicks succeeded. An attempted DOM menu click is not keyboard/accessibility
acceptance. No project, invoice, tracked time, pin or account setting was saved,
deleted or changed. An attempt to leave the unsaved duplicate form through direct
navigation was aborted; its own Cancel action was then invoked without saving.

## Official behavioral contracts

Read-only MCP recheck on 2026-09-30 confirms that the inspected fixed-fee
project's Invoiced amount matches its one draft invoice's pre-tax amount. Its
Uninvoiced amount is negative and reconciles to total project fees minus that
invoice contribution. Switching to Invoices leaves the summary unchanged. The
native click timed out waiting for tab stability; a DOM click opened the tab.
This confirms displayed behavior, not native pointer acceptance, and no billing
record or setting was changed. It does not establish the handling of every
invoice status, taxes, mixed-project lines or recurring fees.

- **Pin:** moves a project to the top of the list, up to 100 pinned projects.
  The article does not establish the ownership/persistence implementation.
  [Projects overview](https://support.getharvest.com/hc/en-us/articles/360048181432-Projects-overview).
- **Duplicate:** copies project settings for a new project and works for archived
  projects too. Project/custom-person historical and future billable-rate schedules
  are not copied as project settings; person defaults remain profile-owned.
  [Create and duplicate projects](https://support.getharvest.com/hc/en-us/articles/360048686831-Create-and-duplicate-projects).
- **Archive versus Delete:** archive preserves time and invoicing/reporting but
  prevents tracking and editing until restored. Delete permanently removes project
  time/expenses/linked retainers and unlinks, but preserves, invoices. It requires
  typed confirmation and has no ordinary undo.
  [Project lifecycle](https://support.getharvest.com/hc/en-us/articles/360048686911-Archiving-restoring-and-deleting-projects).
- **Link/Unlink:** linkage is at invoice-line level. Linking from a project applies
  all invoice lines; a mixed-project invoice shows only that project's attributed
  pre-tax value. Reassigning linkage does not move the original time/expenses.
  For time-and-materials, only actually invoiced source entries reduce uninvoiced
  time; manually linking a line does not. A line linked to a fixed-fee project
  reduces that project's uninvoiced amount. Retainer invoices have special limits.
  [Invoice-project linking](https://support.getharvest.com/hc/en-us/articles/360048686631-Linking-invoices-to-projects).

## Implications for Horae planning

1. Duplicate should reuse the existing creation draft/editor, with a fresh identity
   and no copied time, invoices or consumed fee balances. Opening it must not
   silently create a finalized project.
1. Pin needs actual persisted ordering; it is not a decorative icon. Determine
   whether it is user-specific rather than claiming this from a single account.
1. Delete is materially different from Archive. Its invoice preservation, audit
   retention, import reconciliation and foreign-key behavior require explicit
   tests and policy; do not implement it as an unchecked cascade.
1. Invoice display attribution and original billing sources are different concepts.
   A cosmetic association table would not reproduce Harvest's fixed-fee behavior.
   Any implementation must reconcile attribution with Horae's per-occurrence
   balances, discount allocation, excess confirmation and locked entry history.
1. The user's request to inspect Harvest supplies reference evidence. It does not
   make untested edge cases verified or authorize destructive browser testing.

## Clients, personal settings and workspace follow-up

Additional read-only MCP navigation on 2026-09-29 confirmed:

- Clients exposes New client, Actions, Import/Export and a client/contact search.
  The list's records did not render in the captured session. This is incomplete
  browser evidence, not proof that the account has no clients or that a paid plan
  is required. Bulk selection, contacts and list filtering remain unverified.
- The existing project's client link opens an Edit client form with name, address
  and preferred currency, including an account-default option. No fields were
  changed or saved. This observed route does not establish a client analytics
  dashboard equivalent to Horae's Client Detail handoff.
- My profile offers Basic info, Rates, Assigned projects, Assigned people,
  Permissions, Notifications and Security. The inspected email field is disabled
  and points to Harvest ID for changes. Horae should respect its OIDC ownership,
  not invent a local password/email management flow by copying the form.
- Workspace Preferences separately exposes timezone, week start, time rounding,
  date/time display, default currency and project-note visibility, among other
  account settings. These were inspected, not edited.
- The workspace navigation includes Import/Export. Automatic backups and audit
  retention from Horae's own handoff were not verified in Harvest and must not be
  presented as Harvest-confirmed behavior.

Evidence remains local, without committing account data:

- `.scratch/playwright-windows/harvest-clients-20260929.yml`.
- `output/page-2026-09-29T18-59-40-065Z.yml`: workspace Preferences.
- `output/page-2026-09-29T18-59-50-452Z.yml`: personal Basic info/navigation.
- `output/page-2026-09-29T19-00-12-856Z.yml`: existing client edit form.

Official documentation supplies the untested lifecycle distinction: a client can
be archived only after all its projects are archived; deletion additionally
requires removing its projects and invoices. Contacts are separate records, and
adding one does not send mail. These are reference requirements, not completed
Horae tests. See [Clients and contacts](https://support.getharvest.com/hc/en-us/articles/360048181312-Create-and-edit-clients-and-client-contacts).

Account-wide preferences are administrator-only; personal settings are a separate
surface. See [Account preferences](https://support.getharvest.com/hc/en-us/articles/360048179912-Customizing-account-preferences)
and [Profile and Harvest ID](https://support.getharvest.com/hc/en-us/articles/360048687071-Members-My-profile-and-Harvest-ID).
