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

action-menu = Actions
action-menu-for = Actions for { $name }
action-edit = Edit
action-archive = Archive
action-restore = Restore
action-restoring = Restoring…

## Common words

common-session-ended = Your session has ended. Sign in again to continue.

## Navigation and shell

nav-my-settings = My settings
session-refresh-failed = Could not update your session
session-refresh-failed-hint = The page keeps what it showed before.
session-try-again = Try again

## Components

period-nav-previous-week = Previous week
period-nav-previous-day = Previous day
period-nav-next-week = Next week
period-nav-next-day = Next day
period-nav-today = Today
period-nav-this-week = This week
period-nav-week = Week

no-access-title = Access unavailable
no-access-body = Your account cannot open this. Ask an administrator of this workspace if you need it.
page-failed = Could not load this page
page-failed-hint = Reload the page. If it keeps failing, sign in again.
page-session-ended = Session ended
page-sign-in-again = Sign in again
confirm-dialog-type-word = Type <word>{ $word }</word> to confirm

## Settings

settings-group-settings = Settings
settings-group-account = Account
settings-profile-title = Profile
settings-profile-name-label = Name
settings-profile-email-label = Work email
settings-profile-email-hint = Managed by your sign-in provider.
settings-profile-sign-in-label = Signed in with
settings-profile-dev-sign-in = Development sign-in
settings-profile-sign-in-unavailable = Unavailable
settings-profile-sign-in-loading = Loading…
settings-profile-zone-label = Time zone
settings-profile-zone-hint = Decides which day is today on your timesheet and when you start a timer.
# Completes "Choose …" and "Search …" in the picker's accessible names.
settings-profile-zone-choice = time zone
settings-profile-zone-default = Workspace default ({ $zone })
settings-profile-saved = Profile saved.
settings-profile-save-failed = Could not save your profile. Try again.
settings-profile-cancel = Cancel
settings-profile-save = Save
settings-profile-saving = Saving…
settings-preferences-title = Preferences
settings-theme-label = Theme
settings-theme-hint = Choose how { -brand } looks on this device.

settings-access-title = Your access
settings-access-intro = What you can see and do in this workspace.
settings-access-history = See changes to your access
settings-access-failed = Could not load your access
settings-access-repair = Ask an administrator of this workspace to check your access.
settings-access-administrator = You can see and change everything here, including other people's access in <link>{ -people }</link>.
settings-access-change = To change your access, ask an administrator of this workspace.
settings-access-table-label = Your access by area
settings-access-column-area = Area
settings-access-column-view = View
settings-access-column-edit = Edit
settings-access-column-approve = Approve
settings-access-area-time = Time
settings-access-area-expenses = Expenses
settings-access-area-projects = Projects
settings-access-area-clients = Clients
settings-access-area-tasks = Tasks
settings-access-area-people = { -people }
settings-access-area-billable-rates = Billable rates
settings-access-area-cost-rates = Cost rates
settings-access-area-invoices = Invoices
settings-access-area-estimates = Estimates
settings-access-area-workspace-settings = Workspace settings
settings-access-area-account-billing = Account billing
settings-access-scope-all = All
settings-access-scope-managed = Managed
settings-access-scope-own = Own
settings-access-scope-none = No
settings-access-not-applicable-label = Does not apply
settings-access-legend = All: everyone's. Managed: the people and projects you manage. Own: only yours.
settings-access-other-title = You can also
settings-access-people-managed = People you manage
settings-access-projects-managed = Projects you manage
settings-access-administrator-badge = Administrator
settings-access-individual-badge = Individual configuration

## Workspace settings

workspace-plugins-title = Plugins
workspace-plugins-empty = No plugins installed
workspace-plugins-empty-hint = Plugins add features to { -brand }. The person who runs your { -brand } server installs them.
workspace-plugin-version = v{ $version }
workspace-plugin-hooks = Hooks: { $hooks }
workspace-access-failed = Could not verify workspace access
workspace-access-failed-hint = Try again, or sign in again if it keeps failing.

## API tokens

api-tokens-title = API tokens
api-tokens-intro = A token lets a script or another app use { -brand } as you, with your access. Treat it like a password. The API is described in <link>its OpenAPI document</link>.
api-tokens-list-label = Your tokens
api-tokens-empty = No tokens yet
api-tokens-empty-hint = Create a token to let a script or another app use { -brand } on your behalf.
api-tokens-revoke-title = Revoke “{ $name }”?
api-tokens-revoke-body = Anything that uses this token loses access at once. A revoked token cannot be restored.

## Tasks

tasks-load-failed = Could not load tasks
tasks-archive-title = Archive this task?
tasks-archive-body = It is archived in every project. Existing time is kept. Stop running timers using this task first.
tasks-restore-title = Restore this task?
tasks-restore-body = It returns to the catalog. To use it again in an existing project, restore it separately in that project's editor.
tasks-archive-confirm = Confirm archive
tasks-restore-confirm = Confirm restore
tasks-activity-keep = Keep current status
tasks-archived-notice = Task archived.
tasks-restored-notice = Task restored to the catalog. Restore it in each project where needed.
tasks-activity-failed = Could not confirm the change. Refresh tasks to check whether it was applied before retrying.
tasks-access-changed = Your session or task permissions changed. Check your access before retrying.
tasks-status-active = Active
tasks-status-archived = Archived
tasks-rate-none = No rate
# A default rate stored without a currency, such as one imported from Harvest.
tasks-rate-currency-not-set = { $amount } / h · currency not set

## Clients

clients-edit = Edit client
clients-archive = Archive client
clients-restore = Restore client
clients-restoring = Restoring client…
clients-archived-toast = { $name } archived
clients-restored-toast = { $name } restored

## Invoices

invoices-new = New invoice
invoices-new-title = New invoice
invoices-back = Back to Invoices
invoices-permissions-failed = Could not check your invoice access
invoices-edit-values = Edit invoice values
invoices-void-ellipsis = Void…
invoices-void-title = Void invoice { $number }?
invoices-void-body = The invoice stays on record but can no longer be sent or paid, and its time becomes editable again. This cannot be undone.
invoices-void-confirm = Void invoice
invoices-status-draft = Draft
invoices-status-sent = Sent
invoices-status-paid = Paid
invoices-status-void = Void

## Importers: connection

harvest-client-remove-title = Remove the OAuth app?
harvest-client-remove-confirm = Remove and disconnect
harvest-client-change-title = Change the client ID?
harvest-client-change-confirm = Save and disconnect

## Importers: runs

## People and permissions

people-title = { -people }
people-loading = Loading { -people(case: "lower") }…
# What LoadError names in "Could not load …".
people-load-subject = { -people(case: "lower") }
people-session-changed = Your session changed. Reload this page before selecting a person.
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
people-actions-for = Actions for { $name }
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
people-profile-administrator = Administrator
# A person whose grants were chosen one by one rather than from a profile.
people-profile-individual = Custom

permissions-discard-title = Discard unsaved permission changes?
permissions-discard-body = The changes in this form are lost. Saved permissions stay as they are.
permissions-profile-delete-title = Delete this custom profile?
permissions-profile-delete-pending = Deleting a custom profile. Confirm or go back in the dialog.
permissions-profile-delete-hint = This does not save the person's draft. Reloading discards unsaved person changes.

## Audit log

audit-title = Audit log
audit-intro = Every change to who can do what, newest first.
audit-time-zone = Times in your time zone ({ $zone }).
audit-loading = Loading permission changes…
audit-list-label = Permission changes
audit-failed = Could not load permission changes
audit-load-more = Load more
audit-kind-all = All changes
audit-kind-person = Person
audit-kind-profile = Profile
audit-kind-managers = Managers
audit-filter-kind-person = Changes to a person
audit-filter-kind-profile = Changes to a profile
audit-filter-kind-managers = Changes to project managers
audit-filter-anyone = Anyone
audit-filter-someone = One person
audit-period-all = All time
audit-period-days = { $days ->
    [one] Last day
   *[other] Last { $days } days
}
audit-today = Today
audit-yesterday = Yesterday
audit-unknown-person = A removed person
audit-unknown-project = A deleted project
audit-unknown-profile = a deleted profile
audit-operator = An operator ({ $command })
audit-summary-label = { $added } added, { $removed } removed
audit-added = +{ $count }
audit-removed = −{ $count }
audit-scope-change = { $before } → { $after }
audit-event-person-changed = <actor>{ $actor }</actor> changed the permissions of <person>{ $person }</person>
audit-event-person-moved = <actor>{ $actor }</actor> moved <person>{ $person }</person> to the { $profile } profile
audit-event-person-individual = <actor>{ $actor }</actor> gave <person>{ $person }</person> their own permissions
audit-event-person-admin-granted = <actor>{ $actor }</actor> made <person>{ $person }</person> an administrator
audit-event-person-admin-revoked = <actor>{ $actor }</actor> removed <person>{ $person }</person> as an administrator
audit-event-person-unchanged = <actor>{ $actor }</actor> saved the permissions of <person>{ $person }</person> without changes
audit-event-profile-created = <actor>{ $actor }</actor> created the profile { $profile } with { $count ->
    [one] one permission
   *[other] { $count } permissions
}
audit-event-profile-deleted = <actor>{ $actor }</actor> deleted the profile { $profile }
audit-event-managers-added = <actor>{ $actor }</actor> made <person>{ $person }</person> a manager of <project>{ $project }</project>
audit-event-managers-removed = <actor>{ $actor }</actor> removed <person>{ $person }</person> as a manager of <project>{ $project }</project>
audit-event-managers-changed = <actor>{ $actor }</actor> changed the managers of <project>{ $project }</project>
audit-event-managers-unchanged = <actor>{ $actor }</actor> saved the managers of <project>{ $project }</project> without changes
audit-details-label = Details
audit-detail-label = What changed
audit-detail-areas = Only the areas that changed.
audit-detail-added = Now: { $permission }
audit-detail-removed = No longer: { $permission }
audit-detail-profile = Profile: { $before } → { $after }
audit-detail-no-longer-manages = No longer manages { $name }
audit-detail-detached = { $name } keeps these permissions as their own
audit-detail-manager-added = { $name } became a manager
audit-detail-manager-removed = { $name } is no longer a manager
audit-receipt = Receipt { $id }
audit-copy = Copy
audit-copied = Copied
audit-empty-title = No permission changes yet
audit-empty-body = When someone changes a person's permissions, a profile or a project's managers, it appears here with who did it and when.
audit-empty-action = Go to { -people }
audit-filtered-empty-title = No changes match these filters
audit-filtered-empty-body = Try a longer period or another person.
audit-clear-filters = Clear filters

## Reports

## New project

## Approvals

approvals-approve-all = Approve all
approvals-awaiting = { $count ->
    [one] { $count } timesheet awaiting your approval
   *[other] { $count } timesheets awaiting your approval
}
approvals-approved-notice = { $requested ->
    [one] Approved { $approved } of { $requested } timesheet
   *[other] Approved { $approved } of { $requested } timesheets
}
approvals-approve-for = Approve { $name }'s week
approvals-reopen-for = Reopen { $name }'s week

## Projects

projects-title = Projects
projects-load-failed = Could not load projects
projects-archive-failed = Could not archive the project. Try again, or refresh the page.

## Timesheet

timesheet-submit-week = Submit week
timesheet-submitted = Submitted
timesheet-add-entry = Add entry
timesheet-edit-entry = Edit { $project }, { $task }

timesheet-delete-entry-title = Delete this time entry?
timesheet-delete-entry-confirm = Delete entry
timesheet-delete-entry-body = The entry is removed from this timesheet and from reports. This cannot be undone.
