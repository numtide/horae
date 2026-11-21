### Horae's interface text in English, the source catalog. Every other language
### translates this file, and the build checks each translation against it.
###
### Ids: `<area>-<what>`, lower case, words joined by `-`. The area is the screen
### or the component (`settings`, `api-tokens`, `tasks`, `clients`, `invoices`,
### `harvest`, `people`, `permissions`, `audit`, `reports`, `new-project`,
### `approvals`, `projects`, `timesheet`, `nav`, `date-picker`, `timer`, …). The
### suffix says what the text is: `-title` (a heading), `-hint`, `-label` (an
### accessible name), `-placeholder`, `-empty`, `-loading`, `-failed`, `-confirm`.
###
### Shared ids: `action-*` for a button or menu item, `common-*` for a word or a
### status, only when two or more areas use the same English text with the same
### meaning. The same English with two meanings ("Open" the verb and "Open" the
### status) gets two ids, because other languages may need two words.
###
### Terms: the brand (`-brand`) and each glossary word of DESIGN.md are terms,
### added the first time a screen uses them, with a `case` variant where English
### needs both "People" and "people". Messages refer to the term instead of
### repeating the word, so a rename happens in one place.
###
### Counts: one message per text with a count, the count as a variable and the
### plural forms in the message. No screen picks between two messages by `n == 1`
### or builds "item(s)".
###
### Markup: a sentence with a link or emphasis inside is one message with
### <tag>…</tag> around that part; the screen splits it with `split_tag`. No
### sentence is assembled from fragments.
###
### Whitespace, braces and numbers: Fluent trims leading and trailing spaces, so
### a space between a message and the markup around it stays in the `rsx!`.
### A literal brace is written {"{"} or {"}"}. Counts that select a plural pass as
### integers; money, durations and dates pass already formatted, as strings.
###
### A message arrives with the code that uses it: a test fails on any message no
### code calls. Each area adds its messages under its own section below.

## Glossary

-brand = Horae

## Shared actions

## Common words

## Navigation and shell

## Components

confirm-dialog-type-word = Type <word>{ $word }</word> to confirm

## Settings

## API tokens

api-tokens-revoke-title = Revoke “{ $name }”?
api-tokens-revoke-body = Anything that uses this token loses access at once. A revoked token cannot be restored.

## Tasks

tasks-archive-title = Archive this task?
tasks-archive-body = It is archived in every project. Existing time is kept. Stop running timers using this task first.
tasks-restore-title = Restore this task?
tasks-restore-body = It returns to the catalog. To use it again in an existing project, restore it separately in that project's editor.

## Clients

## Invoices

## Importers: connection

harvest-client-remove-title = Remove the OAuth app?
harvest-client-remove-confirm = Remove and disconnect
harvest-client-change-title = Change the client ID?
harvest-client-change-confirm = Save and disconnect

## Importers: runs

## People and permissions

permissions-discard-title = Discard unsaved permission changes?
permissions-discard-body = The changes in this form are lost. Saved permissions stay as they are.
permissions-profile-delete-title = Delete this custom profile?
permissions-profile-delete-pending = Deleting a custom profile. Confirm or go back in the dialog.
permissions-profile-delete-hint = This does not save the person's draft. Reloading discards unsaved person changes.

## Audit log

## Reports

## New project

## Approvals

## Projects

projects-archive-failed = Could not archive the project. Try again, or refresh the page.

## Timesheet

timesheet-delete-entry-title = Delete this time entry?
timesheet-delete-entry-confirm = Delete entry
timesheet-delete-entry-body = The entry is removed from this timesheet and from reports. This cannot be undone.
