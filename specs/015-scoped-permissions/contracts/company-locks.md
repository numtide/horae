# Company lock and submission scheduling

Status: C05's deadline dependency resolved from current official documentation,
2026-10-02. This specifies the newer web behavior requested for parity, not the
rollout state or experimentally verified enforcement of the owner's account.
FR-019/023 apply. No live settings or data were changed.

## Documented behavior

[Understanding Timesheet Lock](https://support.getharvest.com/hc/en-us/articles/46563687732877-Understanding-Timesheet-Lock)
defines deadline-based locking and independent weekly, monthly or 1–90-day
rolling schedules. Only deadline mode needs a submission deadline. Automatic
submission is optional with automatic locking; both need the approval module.
There is one organization cutoff: automatic runs advance it, manual changes can
move it backward without cancelling future runs. Manual future dates are invalid.
Disabling scheduling may retain or clear the cutoff; disabling approvals retains
existing protection. Affected running timers stop.

The [lock Q&A](https://support.getharvest.com/hc/en-us/articles/46563806657421-Timesheet-auto-lock-and-manual-lock-Q-A)
separates deadlines from edit prevention, locking from approval/invoicing, and
company locks from project-scoped approval. Lock-triggered submission covers time
and expenses without manager notifications. Lock changes require Administrator
identity and company-write authority. Its administrator editing exception is more
specific than the overview's blanket prohibition; ordinary users remain blocked.
Manual locking has its own optional submission checkbox even when recurring
auto-lock is disabled; do not apply the recurring toggle's prerequisite to it.

The [current general approval guide](https://support.getharvest.com/hc/en-us/articles/360048181832-Submitting-and-approving-timesheets)
also explicitly describes independent schedules and the auto-submit dependency,
while warning about incremental rollout. The retained deadline-disabled owner
controls do not override this documented newer behavior.

## Horae integration obligations

Rechecked both articles through the public Help Center JSON API on 2026-10-04
after the web reader could not retrieve their HTML. The overview's `updated_at`
is `2026-07-02T21:00:35Z`; the Q&A's is `2026-06-11T10:49:18Z`.
The overview explicitly binds weekly/monthly cutoff calculation to the end of
the preceding week/month and manual-date validation to company-local today.
It does not specify what a monthly day 29–31 does in shorter months, DST gap/fold
resolution, or overdue execution after downtime. The Q&A does not resolve those
cases either. Keep T043 open; neither another product's scheduler nor an older
deadline-only preferences article supplies Harvest evidence for these rules.
No authenticated browser observation or account change was made during that
2026-10-04 check.

Read-only reference refresh, 2026-10-13: both public Help Center JSON articles
still report the same update timestamps and leave those calendar/execution
boundaries unspecified. An authenticated owner session's Preferences editor
still disables automatic locking and submission without a configured deadline.
The editor was exited with Cancel, without changing fields or saving. This
confirms the account limitation, not enforcement of the newer independent
calendars, behavior for a restricted actor, or an approved change to FR-023.
That observation alone leaves monthly missing-date, daylight-saving and downtime
behavior unresolved. The following shipped-client evidence subsequently supplies
a reference-derived monthly rule, pending user confirmation, without claiming
server execution.

### Monthly boundary evidence — 2026-10-20

The authenticated Preferences editor loads
[`preferences-I5UQWPUO.js`](https://cache.harvestapp.com/static/company/preferences-I5UQWPUO.js).
The downloaded artifact is 1,223,819 bytes with SHA-256
`3a4bdec9357cec5d3bc84be57ceee819b26332ed61339a313b8fd69f7f8e4748`.
Its monthly preview contains an explicit user-facing explanation that schedules
for days absent from a month run on that month's last day. Its next-monthly-run
helper independently clamps the requested day to that month's length.

Reproducible locations in those exact bytes (zero-based, end-exclusive):

- Monthly next-run helper `o9`: UTF-8 bytes `[1192678, 1192879)`.
- Monthly preview branch calling that helper and emitting the explanation:
  UTF-8 bytes `[1195430, 1195950)`.

These are static shipped-client observations, not a completed runtime trace or
server-scheduler test. The ordinary editor still presents the deadline-dependent
controls; no feature flag was changed, hidden component activated, field edited,
or preference saved. The complete static analyzer reached its 45-second limit;
its retained partial evidence is
`ev_b0b64643f707e33de17160cbca4adbee099b9b34c2184641e9ead87bf9db5205`.
The source locations above were inspected directly, not inferred from that
incomplete graph. Raw downloaded code and the partial evidence export remain
local investigation artifacts, not application dependencies.

Proposed target, pending user confirmation: a monthly schedule would retain
its configured day and use the last available day in a shorter month, rather
than skip the month. A clamped February occurrence would not change a configured
31 into 28 or 29 for subsequent months. The cutoff would remain the end of the
month preceding the occurrence, as documented above. This answers the
last-day-versus-skip question from reference evidence only; the user has not
answered it and no user answer is recorded. **Status: pending the user's
confirmation**; the rule is the reference-derived target, not an accepted
decision, until confirmed. The DST gap rule is resolved separately below; DST fold and
missed-run recovery remain unresolved. A client
date-library implementation does not establish the server's timezone policy.

### Daylight-saving gap decision — 2026-10-20

The user selected execution at the first valid instant after a daylight-saving
gap. Resolve the occurrence in the configured organization timezone: if the
chosen local date/time does not exist, use the transition's first valid instant,
not the same minute shifted by the gap's duration. Preserve the configured time
for subsequent occurrences and do not silently skip the occurrence.

Acceptance example for T043: a daily 02:30 schedule in `Europe/Madrid` runs at
2027-03-28 03:00 CEST (01:00 UTC), not 03:30, because 02:30 does not exist that
day. Its next day's occurrence is 2027-03-29 02:30 CEST (00:30 UTC).
Preview and worker must agree. Inject the clock and use the same timezone rules
in both; a different viewer timezone must not change the organization schedule.
Other cutoff and independent-protection rules still apply.

This is an explicit Horae decision, not Harvest enforcement evidence. Searches
of the public Help Center for auto-lock/daylight and the exact phrase
"daylight saving", plus the lock overview, Q&A and account-preferences guide,
did not establish a scheduler rule. The absence is limited to those sources.
Repeated local times during the autumn transition and delayed execution after
an outage remain separate unresolved cases.

These are implementation safeguards from FR-006/007/010/013/017/019, not claims
about Harvest's internal design:

- Keep schedule configuration, company cutoff, submission coverage, approval
  coverage and invoice protection distinct. Clearing one protection cannot clear
  another. Ordinary write permission cannot bypass a lock. Explicit privileged
  correction is a separate operation: preserve the lock facts rather than
  denying every correction merely because approval or invoicing is present.
  Apply the field/surface limits and unresolved boundaries below.
- Save configuration through authenticated server functions with current
  authority and revision checks. Keep audit facts for configuration changes,
  manual cutoff changes and effective automated execution; do not manufacture
  a human actor for system execution.
- Persist effective organization timezone and week-start inputs. Compute the
  displayed next run and applied cutoff through one shared calculation, with
  an injected clock for tests. Use the approved DST gap contract above and
  monthly clamping once the user confirms it; resolve DST folds and missed-run behavior before implementation. Shipped preview
  code does not establish those server algorithms.
- Serialize manual changes, due execution, module changes and entry mutations.
  A worker rechecks current enabled/configuration state, not stale admission
  state. Duplicate execution/retry cannot double-add minutes or submissions.
  Use the existing scheduler/jobs infrastructure where appropriate; no separate
  queue framework is implied.
- Do not recalculate or mutate unrelated financial/history records. Required
  timer finalization and explicitly authorized corrections are distinct lifecycle
  operations, not permission migration side effects. Do not mutate an existing
  database to demonstrate this contract. Expense integration depends on feature
  016; absence of its runtime is not proof that combined behavior passes.

## Acceptance and remaining verification

The dedicated [time editing guide](https://support.getharvest.com/hc/en-us/articles/26885299659405-Editing-and-deleting-time-entries),
checked 2026-10-03, documents Administrator correction of notes, duration or
start/end time and confirmed deletion of approved, invoiced or archived time
from Day view. It excludes project/task reassignment and locked Week-view
editing/deletion. The [expense editing guide](https://support.getharvest.com/hc/en-us/articles/4406054281101-How-to-edit-expenses)
documents Administrator notes/amount correction and deletion, not project/category
changes. These specific exceptions refine the general unlocking guide; correction
and unlocking must not be treated as the same command.

Checked 2026-10-04: the more specific [Calendar guide](https://support.getharvest.com/hc/en-us/articles/39977971409549-Track-and-edit-time-in-the-Calendar-view)
also permits Administrator notes/start/end correction from the locked entry's
Calendar form, while explicitly forbidding locked drag/resize and project/task
changes. Do not implement the general article's Day-only wording as a prohibition
of this documented Calendar exception. Calendar deletion of locked records is
not explicitly settled by its separate generic deletion instructions; retain
that gap rather than extrapolating the correction exception.

These are documentation findings, not restricted-user browser tests. The newer
permission catalog does not establish a custom-grant equivalent to these
Administrator exceptions. Nor do these articles settle company-lock combinations,
date/billability/receipt changes, interval effects after deletion or invoice-source
snapshot effects. Keep those cases open; do not implement a generic admin bypass
or silently change ordinary time/expense-write grants. T006 and T043/T044 must
bind the resolved cases to their final command and preservation tests before
runtime integration. Feature 016 owns the expense implementation.

T043 needs the complete local calendar/execution and correction contract before
pure calculation tests/code; unrelated matrix rows are not a prerequisite.
T044–T046 additionally need reviewed storage/command contracts and T042 before
integration. T006–T009 still gate policy cutover and full-feature acceptance.
Use disposable fixtures and production helpers with an injected clock.

| Case | Required check |
| --- | --- |
| Deadline absent | Independent modes remain configurable; deadline mode is rejected |
| Cutoff evolution | Automatic execution cannot undo a later manual cutoff; manual changes preserve scheduling |
| Separate protections | Clear company cutoff with approved/invoiced controls still locked; ordinary writes denied |
| Identity/action | Non-admin company-write alone cannot change locks; correction never grants unrelated actions |
| Timers/submission | Count minutes once; paired submission does not approve or notify managers |
| Manual submission | Manual lock can submit with recurring scheduling off; unchecked/cancel does not submit |
| Concurrent changes | Disable/revise configuration while a run waits; no stale job applies the old rule |
| Calendar boundaries | Test company-local dates, week start, month end and daylight-saving behavior against the finalized algorithm |

Monthly acceptance examples for T043 use company timezone UTC and a scheduled
time of 12:00, with the clock before that month's occurrence. They specify
Horae's expected results from the reference rule, pending the user's
confirmation above, not completed Harvest tests:

| Configured day | Occurrence month | Expected occurrence | Inclusive cutoff |
| --- | --- | --- | --- |
| 29, 30 or 31 | February 2027 | 2027-02-28 12:00 | 2027-01-31 |
| 29, 30 or 31 | February 2028 | 2028-02-29 12:00 | 2028-01-31 |
| 31 | April 2027 | 2027-04-30 12:00 | 2027-03-31 |
| 31, unchanged after February's occurrence | March 2027 | 2027-03-31 12:00 | 2027-02-28 |
| 28 | February 2028 | 2028-02-28 12:00 | 2028-01-31 |

For each case, preview and execution must agree and retries preserve the
nondecreasing cutoff and single-effect invariants above. The concrete runtime
tests are still pending; these examples do not close the other T043 cases.

No runtime or browser acceptance has passed for this new contract. Date-boundary
algorithms outside the monthly cases and the ordinary/privileged correction matrix still require explicit
implementation cases; C06's custom combined-approval authority remains separate.
