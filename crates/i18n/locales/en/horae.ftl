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
-people = { $case ->
    [lower] people
   *[upper] People
}

## Shared actions

## Common words

## Navigation and shell

## Components

period-nav-previous-week = Previous week
period-nav-previous-day = Previous day
period-nav-next-week = Next week
period-nav-next-day = Next day
period-nav-today = Today
period-nav-this-week = This week
period-nav-week = Week

## Settings

## API tokens

## Tasks

## Clients

## Invoices

## Importers: connection

## Importers: runs

## People and permissions

people-title = { -people }
people-loading = Loading { -people(case: "lower") }…
# What LoadError names in "Could not load …".
people-load-subject = { -people(case: "lower") }
people-session-changed = Your session changed. Reload this page before selecting a person.
people-access-unavailable = Access unavailable
people-access-unavailable-hint = Your account cannot view { -people(case: "lower") }. Ask an Administrator if you need it.
people-access-check-failed = Could not verify your access
people-access-check-failed-hint = Try again, or sign in again if it keeps failing.
people-add = Add person
people-go-this-week = This week
people-summary-label = Week summary
people-total-hours = Total hours
people-billable = Billable
people-non-billable = Non-billable
people-billable-share = Billable { $percent }%
people-non-billable-share = Non-billable { $percent }%
people-share = { $percent }%
people-any-profile = Any profile
people-status-active = Active ({ $count })
people-status-archived = Archived ({ $count })
people-status-all = All ({ $count })
people-search-label = Search by name or email
people-none-archived = No archived { -people(case: "lower") }
people-none-match-filters = No { -people(case: "lower") } match these filters
people-none-match-filters-hint = Change the filters to see more { -people(case: "lower") }.
people-clear-filters = Clear filters
people-none-match-search = No one matches “{ $query }”
people-search-hint-active = Search looks at names and emails among active { -people(case: "lower") }.
people-search-hint-archived = Search looks at names and emails among archived { -people(case: "lower") }.
people-search-hint-all = Search looks at names and emails among everyone.
people-clear-search = Clear search
people-column-person = Person
people-column-hours = Hours
people-column-billable-share = Billable %
people-actions = Actions
people-archived = Archived
people-hours-unavailable-label = Hours not available
people-billable-unavailable-label = Billable hours not available
people-share-unavailable-label = Billable share not available
people-view-timesheet = View timesheet
people-permissions = Permissions…
people-rates = Rates…
people-permission-history = Permission history
people-archive = Archive
people-restore = Restore
people-undo = Undo
people-added = Person added
people-archived-toast = Person archived
people-restored = Person restored
people-permissions-saved = Permissions saved
people-permissions-unchanged = Permissions are unchanged
people-add-title = Add person
people-adding = Adding person…
people-add-name = Name
people-add-email = Email
people-add-email-hint = They sign in with this address.
people-add-profile = Profile
people-profile-member = Member
people-profile-member-hint = Tracks their own time.
people-profile-project-manager = Project Manager
people-profile-project-manager-hint = Also manages the projects you assign.
people-profile-accounting = Accounting
people-profile-accounting-hint = Also sees everyone's time, rates and invoices.
people-profile-people-admin = People Admin
people-profile-people-admin-hint = Also manages { -people(case: "lower") } and everyone's time.
people-profile-executive-manager = Executive Manager
people-profile-executive-manager-hint = Also manages projects, { -people(case: "lower") } and invoicing.

## Audit log

## Reports

## New project

## Approvals

## Projects

## Timesheet
