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
  an injected clock for tests. Resolve month-end and daylight-saving behavior
  before implementation; the public guides do not define those algorithms.
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

No runtime or browser acceptance has passed for this new contract. Date-boundary
algorithms and the ordinary/privileged correction matrix still require explicit
implementation cases; C06's custom combined-approval authority remains separate.
