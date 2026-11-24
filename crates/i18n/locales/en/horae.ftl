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

## Leaving an editor

leave-discard-title = { $kind ->
    [client] Discard unsaved client changes?
    [permissions] Discard unsaved permission changes?
    [invoice] Discard unsaved invoice changes?
    [timesheet] Discard unsaved timesheet changes?
   *[project] Discard unsaved project changes?
}
leave-discard-body = Your changes on this page are not saved. Leaving the page discards them.
leave-discard = Discard changes
leave-keep-editing = Keep editing
leave-pending-title = Wait before leaving
leave-pending-body = { $kind ->
    [client] A client save is pending. Wait for it to finish before leaving.
    [permissions] A permission request is unresolved. Wait for it to finish, or retry the same request before leaving.
    [invoice] An invoice request is unresolved. Wait for it to finish, or recover the request before leaving.
    [timesheet] A timesheet change is still saving. Wait for it to finish before leaving.
   *[project] A project save is unresolved. Wait for it to finish, or retry the request before leaving.
}
leave-pending-stay = Stay on this page

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
bulk-bar-label = Actions for the selected rows
bulk-bar-selected = { $count } selected
bulk-bar-clear = Clear

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

workspace-settings-title = Settings
workspace-settings-intro = Defaults for everyone in this workspace.
workspace-settings-form-label = Workspace settings
workspace-zone-label = Time zone
workspace-zone-hint = Used for today, timers and monthly budgets when a person has not chosen their own time zone.
# Completes "Choose …" and "Search …" in the picker's accessible names.
workspace-zone-choice = workspace time zone
workspace-save = Save changes
workspace-saving = Saving…
workspace-discard = Discard
workspace-save-failed = Could not save the settings. Try again.
workspace-plugins-title = Plugins
workspace-plugins-intro = What the plugins installed on this server add to { -brand }.
workspace-plugins-table-label = Installed plugins
workspace-plugins-loading = Loading plugins…
workspace-plugins-what = plugins
workspace-plugins-column-name = Name
workspace-plugins-column-version = Version
workspace-plugins-column-status = Status
workspace-plugins-column-hooks = Hooks
workspace-plugins-status-loaded = Loaded
workspace-plugins-empty = No plugins installed
workspace-plugins-empty-hint = Plugins add features to { -brand }. The person who runs your { -brand } server installs them.
workspace-plugin-version = v{ $version }
workspace-access-failed = Could not verify workspace access
workspace-access-failed-hint = Try again, or sign in again if it keeps failing.

## API tokens

api-tokens-title = API tokens
api-tokens-intro = A token lets a script or another app use { -brand } as you, with your access. Treat it like a password. The API is described in <link>its OpenAPI document</link>.
api-tokens-list-label = Your tokens
api-tokens-form-label = New token
api-tokens-name-hint = Name it after what will use it, such as “Calendar sync”.
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
tasks-rate-per-hour = { $amount } / h
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

invoices-title = Invoices
invoices-invoice = Invoice
invoices-new = New invoice
invoices-new-for = New invoice for { $client }
invoices-new-title = New invoice
invoices-back = Back to Invoices
invoices-back-to-invoice = Back to invoice
invoices-more = More
invoices-settings = Invoice settings
invoices-permissions-failed = Could not check your invoice access
invoices-void-ellipsis = Void…
invoices-void-title = Void invoice { $number }?
invoices-void-body = The invoice stays on record but can no longer be sent or paid, and its time becomes editable again. This cannot be undone.
invoices-void-confirm = Void invoice
invoices-voided = Invoice voided
invoices-status-draft = Draft
invoices-status-sent = Sent
invoices-status-overdue = Overdue
invoices-status-paid = Paid
invoices-status-void = Void
invoices-status-closed = Written off
invoices-status-any = Any status
invoices-client-any = Any client
invoices-period-any = Any time
invoices-period-this-month = This month
invoices-period-last-month = Last month
invoices-period-this-year = This year
invoices-period-last-year = Last year
invoices-period-up-to-today = Everything up to today
invoices-period-custom = Custom dates
invoices-period-up-to = Everything up to { $date }
invoices-period-range = { $from } – { $to }
invoices-period = Period
invoices-period-from = From
invoices-period-to = To
invoices-period-invalid = Choose a period that ends on or after its start.
invoices-pick-date = Pick a date
invoices-search-label = Search by number or client
invoices-clear-filters = Clear filters
invoices-none-match = No invoices match these filters
invoices-empty-title = No invoices yet
invoices-empty-body = An invoice is made from a client's unbilled time and project fees.
invoices-loading = Loading invoices…
invoices-what = invoices
invoices-view-invoices = Invoices
invoices-view-uninvoiced = Uninvoiced
invoices-summary-label = Summary
invoices-summary-outstanding = Outstanding
invoices-summary-overdue = Overdue
invoices-summary-ready = Ready to invoice
invoices-summary-count = { $count ->
    [one] 1 invoice
   *[other] { $count } invoices
}
invoices-summary-ready-clients = { $count ->
    [one] 1 client with unbilled work
   *[other] { $count } clients with unbilled work
}
invoices-column-number = Number
invoices-column-client = Client
invoices-column-issued = Issued
invoices-column-due = Due
invoices-column-amount = Amount
invoices-column-status = Status
invoices-column-project = Project
invoices-column-description = Description
invoices-column-hours = Hours
invoices-column-rate = Rate
invoices-column-unbilled-time = Unbilled time
invoices-column-unbilled-amount = Unbilled amount
invoices-column-fees-due = Fees due
invoices-uninvoiced-loading = Loading unbilled work…
invoices-uninvoiced-what = unbilled work
invoices-uninvoiced-empty-title = Nothing left to invoice
invoices-uninvoiced-empty-body = Every client's billable time and due fees are on an invoice.
invoices-detail-title = Invoice { $number }
invoices-detail-loading = Loading the invoice…
invoices-detail-what = this invoice
invoices-download-pdf = Download PDF
invoices-mark-sent = Mark as sent
invoices-mark-paid = Mark as paid
invoices-marked-sent = Invoice marked as sent
invoices-marked-paid = Invoice marked as paid
invoices-write-off = Write off
invoices-written-off = Invoice written off
invoices-imported = From Harvest
invoices-imported-note = This invoice was imported from Harvest. It can be marked as paid, written off or voided here, but not edited.
invoices-paid = Paid
invoices-amount-due = Amount due
invoices-paid-on = Paid on
invoices-column-quantity = Quantity
invoices-column-unit-price = Unit price
invoices-edit-draft = Edit draft
invoices-duplicate = Duplicate
invoices-terms = Payment terms
invoices-terms-days = { $days ->
    [0] Due on receipt
    [one] 1 day
   *[other] { $days } days
}
invoices-terms-on-receipt = Due on receipt
invoices-terms-net = Net { $days }
invoices-terms-custom = Custom days
invoices-terms-days-label = Days until payment is due
invoices-terms-days-hint = From 0 to 365 days after the invoice date.
invoices-po-number = PO number
invoices-optional = Optional
invoices-tax = Tax (%)
invoices-tax-hint = Optional, up to two decimal places
invoices-second-tax-name = Second tax name
invoices-second-tax = Second tax (%)
invoices-second-tax-add = Add a second tax
invoices-second-tax-remove = Remove second tax
invoices-discount = Discount (%)
invoices-discount-hint = Applied before tax. Both taxes use the discounted subtotal, without compounding.
invoices-subtotal = Subtotal
invoices-total = Total
invoices-subject = Subject
invoices-subject-hint = Optional. One line saying what the invoice is for, shown on the invoice.
invoices-subject-too-long = The subject can have at most 200 characters.
invoices-notes = Notes
invoices-step = Step { $step } of { $steps }
invoices-step-client-title = Who is it for?
invoices-step-client-hint = Next you choose the period and the projects. Lines come from the client's unbilled time and project fees.
invoices-continue = Continue
invoices-clients-loading = Loading clients…
invoices-clients-what = clients for invoicing
invoices-client-unavailable = Client unavailable
invoices-client-unavailable-body = This client cannot be invoiced. It may be archived, or the link may be wrong.
invoices-choose-client = Choose a client
invoices-change-client = Change client
invoices-preparing = Loading the client's projects…
invoices-duplicating = Starting from invoice { $number }: its values, subject and notes are copied, and its projects are selected. Time already on an invoice is never copied.
invoices-projects = Projects
invoices-all-projects = All projects
invoices-project-archived = { $name } (archived)
invoices-projects-none = Select at least one project.
invoices-awaiting-approval = { $duration } of billable time in this period is waiting for approval. It stays out of this invoice until it is approved. <link>Open Approvals</link>
invoices-defaults-conflict = These projects have different invoice values. Use one project's values, or enter the values below.
invoices-use-defaults-from = Use the values of { $project }
invoices-values = Invoice values
invoices-preview = Preview
invoices-lines-label = Invoice lines
invoices-preview-loading = Loading the preview…
invoices-preview-updating = Updating the preview…
invoices-preview-dates = { $count ->
    [one] Issued { $issued } · due { $due } · 1 line
   *[other] Issued { $issued } · due { $due } · { $count } lines
}
invoices-fee-agreed = Agreed: { $amount }
invoices-fee-invoiced = Invoiced (including drafts): { $amount }
invoices-fee-available = Available: { $amount }
invoices-fee-over = Over-invoiced: { $amount }
invoices-fee-net = Proposed net before tax: { $amount }
invoices-refresh-fees = Reload fee sources
invoices-preview-retry = Try again
invoices-create-draft = Create draft
invoices-retry-create = Retry creating the draft
invoices-request-not-sent = { $reason } No invoice request was sent.
invoices-recovery-not-cleared = The invoice is saved, but its recovery record could not be cleared: { $reason }
invoices-generation-uncertain = The draft may have been created. Retry the same request to recover it without creating another.
invoices-excess-title = Confirm over-invoicing
invoices-excess-cancel = Cancel over-invoicing
invoices-excess-create = Confirm excess and create
invoices-excess-create-body = Creating this draft exceeds the agreed fees by the amounts below, before tax. Cancelling creates nothing.
invoices-excess-save = Confirm excess and save
invoices-excess-save-body = Saving exceeds the agreed fees by the amounts below. Cancelling changes nothing.
invoices-excess-line = { $source } · excess { $amount }
invoices-edit-title = Edit invoice { $number }
invoices-editor-loading = Loading the draft…
invoices-editor-what = this draft
invoices-details = Details
invoices-fee-lines = Fee lines
invoices-fee-lines-hint = Changes apply to this draft only. Agreed fees, time entries and other invoices stay as they are.
invoices-fee-description = Fee description
invoices-fee-amount = Fee amount ({ $currency })
invoices-fee-description-invalid = Fee descriptions need 1 to 1,000 characters.
invoices-fee-amount-invalid = Enter an exact fee amount with at most two decimal places.
invoices-fee-amount-negative = Fee amounts cannot be negative.
invoices-fee-edit-balance = Agreed { $agreed } · on other invoices { $other } · left after this one { $remaining }
invoices-over-invoiced = Over-invoiced
invoices-totals = Totals
invoices-save-draft = Save draft
invoices-retry-save = Retry saving
invoices-discard-changes = Discard changes
invoices-draft-saved = Draft saved
invoices-save-uncertain = The draft may have been saved. Retry the same changes to recover the result safely.
invoices-column-last-invoice = Last invoice
invoices-last-invoice-date = on { $date }
invoices-never-invoiced = Never invoiced
invoices-period-since-last = Since the last invoice
invoices-period-since = Since { $number }, from { $date }
invoices-preview-empty = No billable time or fees in this period
invoices-preview-empty-body = Choose another period or other projects. Time waiting for approval joins an invoice once it is approved.
invoices-terms-invalid = Payment terms must be between 0 and 365 days.
invoices-po-number-invalid = Purchase order must contain at most 200 characters.
invoices-percentage-invalid = Percentages must be between 0 and 100, with at most two decimal places.
invoices-second-tax-name-invalid = The second tax needs a name of 1 to 100 characters.
invoices-fees-to-invoice = Fees to invoice
invoices-fees-to-invoice-hint = Amounts and descriptions change this invoice only. Balances include drafts and are before tax. Settled fees are not selected for you, and reloading keeps your edits and leaves new fees unselected.
invoices-fee-group = Fee: { $label } ({ $currency })
invoices-fee-unavailable = This fee is no longer available in this currency. Your edits are kept below; remove this row before the invoice can be created.
invoices-fee-remove-unavailable = Remove unavailable fee edits
invoices-fee-unavailable-edits = Remove unavailable fee edits before creating the invoice.
invoices-fee-include = Include fee: { $label }
invoices-fee-amount-selected = Selected fee amounts must be positive; unselected amounts cannot be negative.
invoices-fee-last-review = Last review · Agreed: { $agreed } · Invoiced (including drafts): { $invoiced } · Remaining: { $remaining }
invoices-recovery-encode = Cannot encode the invoice recovery record.
invoices-recovery-too-large = The invoice recovery record exceeds 512 KiB.
invoices-recovery-unavailable = Browser session storage is unavailable.
invoices-recovery-invalid = The invoice recovery record is invalid. Keep this tab and check the saved invoices before clearing browser data.
invoices-access-required = Invoices require access to all invoices.
invoices-recover-title = Recover invoice request
invoices-recover-body = A previous invoice request may have completed. Recover the exact request that was sent before starting another. Retrying an acknowledged request creates no new invoice.
invoices-recover = Recover invoice request
invoices-recover-rejected = The server rejected this request. Check the saved invoices before discarding this tab's recovery record; discarding it never deletes an invoice.
invoices-recover-discard = Discard rejected recovery request
invoices-open-failed = Could not open invoices
invoices-settings-loading = Loading invoice settings…
invoices-settings-what = invoice settings
invoices-settings-company = Company information
invoices-settings-company-hint = Your details as the issuer. Every invoice PDF shows the current values, including invoices already sent.
invoices-settings-payment = Payment details
invoices-settings-payment-hint = How clients pay you. Every invoice PDF shows the current values, including invoices already sent.
invoices-settings-defaults = Default values
invoices-settings-defaults-hint = Terms print on every invoice PDF. Notes are copied into each new invoice; existing invoices keep theirs.
invoices-settings-save-company = Save company information
invoices-settings-save-payment = Save payment details
invoices-settings-save-defaults = Save default values
invoices-settings-saving = Saving…
invoices-settings-saved = Saved.
invoices-settings-name = Company name
invoices-settings-address = Address
invoices-settings-tax-id = Tax ID
invoices-settings-email = Email
invoices-settings-phone = Phone
invoices-settings-bank = Bank name
invoices-settings-iban = IBAN
invoices-settings-bic = BIC / SWIFT
invoices-settings-routing = Routing number
invoices-settings-account = Account number
invoices-settings-notes = Invoice notes
invoices-settings-too-long = Too long to print on an invoice: use at most 32,767 bytes.
invoices-settings-nul = Remove the NUL character.
invoices-settings-preview-hint = How these details read on an invoice, including unsaved edits.
invoices-settings-preview-no-name = No company name
invoices-settings-preview-tax-id = Tax ID: { $id }
invoices-settings-preview-bank = Bank
invoices-settings-preview-bic = BIC
invoices-settings-preview-routing = Routing
invoices-settings-preview-account = Account
invoices-settings-preview-line = { $label }: { $value }
invoices-settings-preview-terms = Terms

## Importers: connection

harvest-client-remove-title = Remove the OAuth app?
harvest-client-remove-confirm = Remove and disconnect
harvest-client-change-title = Change the client ID?
harvest-client-change-confirm = Save and disconnect

## Importers: runs

harvest-run-invoices-forbidden = The Harvest user who connected Horae can't read invoices, which the import needs.
harvest-run-invoices-forbidden-hint = Reconnect as a Harvest administrator, or ask one to let this user create and edit invoices.
harvest-billing-title = Billed time
harvest-billing-preview-title = Billed time it will mark
harvest-billing-linked = Linked to an imported invoice
harvest-billing-unlinked = No longer billed, open again
harvest-billing-without-invoice = Billed with no invoice in Horae
harvest-billing-without-invoice-hint = Harvest billed this time on an invoice that was not imported. It stays out of new invoices.
harvest-billing-conflicts = Already billed or submitted in Horae
harvest-billing-drafts = Draft invoices left out: { $count }
harvest-billing-no-longer-listed = Imported invoices Harvest no longer lists: { $count }
harvest-billing-voided-drafts = Imported invoices Harvest turned back into drafts, now void: { $count }
harvest-billing-removed = { $count ->
    [one] Removed imported invoice { $numbers }: Harvest deleted it, and a new invoice uses its number.
   *[other] Removed imported invoices { $numbers }: Harvest deleted them, and new invoices use their numbers.
}
harvest-billing-full-scan = Every time entry was read again, to match it with Harvest's invoices.
harvest-billing-watermarks-held = Some records could not be imported, so the next run reads again everything since the last run without errors. Fix the errors below to let runs read only what changed.
harvest-billing-conflict-billed = { $entry } is on Horae invoice { $number }
harvest-billing-conflict-submitted = { $entry } is submitted for approval in Horae
harvest-billing-conflicts-more = { $count ->
    [one] and one more
   *[other] and { $count } more
}

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

reports-time-label = Time report
reports-detailed-label = Detailed time report

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
