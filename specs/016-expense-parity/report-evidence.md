# Detailed expense report evidence

Checked 2026-10-01 in the authenticated owner account. Only the isolated fixture
project and the week containing its single surviving expense were selected.
No saved/shared/recurring report, export, notification or payment was created.

## Filter, amount and receipt

The project-filtered report showed only the fixture: quantity 3 at rate 0.5,
amount USD 1.50, client subtotal USD 1.50 and report total USD 1.50. A receipt link
targets the internal expense receipt route. This proves single-currency display
reconciliation, not mixed-currency export precision or recipient access.

## Manual billing and selection

With no row checked, Actions was enabled and offered marking all expenses
invoiced or uninvoiced. The confirmation showed the date range and selected
project. Cancelling closed it without a billing change. Reopening and confirming
marked exactly one expense invoiced and displayed a lock. No invoice was created
or sent by this action.

Selecting that specific row and choosing uninvoiced produced a confirmation for
selected expenses. Its submitted ID list contained only the fixture. Confirming
reported one expense uninvoiced and removed the lock. Subsequent expense update
returned `is_billed=false`, `is_locked=false` and `invoice=null`.

After changing only that fixture to non-billable, the report still exposed the
invoiced action. Submitting the explicitly selected fixture returned a message
that nothing was marked because the expense was non-billable; no lock appeared.
Restoring billability returned the same unbilled/unlocked/null-invoice state and
USD 1.50. Thus the action's presence is not proof of entry eligibility; the
operation enforces the billable rule. No reimbursement state was changed.

### Initial pagination hypothesis and resolved target set

The [official guide](https://support.getharvest.com/hc/en-us/articles/360048687171-Detailed-time-and-detailed-expense-reports)
describes empty selection in terms of entries on the page. However, the actual
confirmation says all expenses within the displayed date/filter scope. Its form
contains the report filters and no expense IDs when nothing is selected.
The delivered report client also serializes explicit selection as `ids[]` but
omits IDs for empty selection and for its separate all-pages selection state.

Initially this left a target-set discrepancy: the single-row experiment could
not distinguish loaded rows from current matching records. The prior
unconditional current-page specification was too strong. Projects' independently
confirmed disabled-empty Actions remain unchanged.

Follow-up [target-set investigation](report-scope-evidence.md) tested 101 exact
fixtures. The detailed report rendered all on one page and ignored `page=2` /
`per_page=1` probes; ordinary expense-list pagination is a different surface.
Empty selection marked exactly those 101 while an out-of-date-filter baseline
stayed unbilled. No hypothetical report pagination behavior is certified by that
result, and the shared all-pages client code must not be treated as proof that
the expense report exposes the control.

The subsequent 1,001-fixture discriminator resolves the target set without
requiring pagination. A report and confirmation loaded with 1,000 rows also
marked the expense moved into its filter afterwards, for 1,001 changed entries;
the outside control remained unchanged. Explicitly clearing two IDs changed
only two. Thus no selection means current filter matches at execution, not the
loaded-row snapshot. See the before/after and project-period lock sequence in
[report-scope-evidence.md](report-scope-evidence.md). No unlimited-volume or
non-owner guarantee follows from this result.

## Export controls and delivery boundary

The UI exposes Excel, CSV, PDF and Custom. Custom offered CSV/Excel and ordered
columns: Date, Client, Project, Project Code, Category, Notes, Amount, Units,
Billable, Invoiced?, Approved?, First name, Last name, Employee Id, Roles, Teams,
Currency and Invoice ID. No changed custom configuration was saved.

Delivered export URLs retained the fixture project and dates. Spreadsheet
generation targets expense-report exports; PDF targets expense-receipt exports.
This one-receipt report was marked not too large, one chunk, 431 receipt bytes.
The client posts generation requests and supports either an immediate download
location or a message. Its large-PDF dialog explicitly describes email delivery;
the [export documentation](https://support.getharvest.com/hc/en-us/articles/31625325401229-Exporting-data)
also documents immediate or emailed delivery for exports.

Because the existing authorization excludes messages and no no-email guarantee
was established for this generation path, generation was **not submitted**.
This is a delivery-authority limitation, not a paid-plan export restriction and
not a passed CSV/XLSX/PDF content test. Do not connect integrations or send a
report merely to finish this check. Report printing/content checks that do not
send messages remain independent alternatives.

Private snapshots: `expense-report-no-selection-confirmation-20261001.md`,
`expense-report-manual-billed-20261001.md`,
`expense-report-manual-unbilled-20261001.md`. Browser source inspected:
`https://cache.harvestapp.com/static/reports/expenses-XPLYUEXQ.js`.
