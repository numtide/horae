# Current-account permission investigation

Started 2026-10-02 for PR #212. Scope: gather the maximum evidence available with
the existing Harvest account. This is a research register, not the completed
operation-level authorization contract required by FR-002/020.

## Evidence rules and safety boundary

- Separate current browser observations, historical observations, delivered
  client configuration, public documentation and unverified server behavior.
  A visible control or a permission label is not a passing enforcement test.
- Use normal owner-visible navigation and read-only inspection of delivered
  configuration. Do not enable disabled controls, impersonate another person,
  probe guessed private endpoints, or treat owner success as non-owner evidence.
- Preserve other tabs. Keep raw account snapshots in the main checkout's ignored
  `.scratch/playwright-windows/`; commit only redacted findings and evidence paths.
- Do not buy seats, invite people, save profiles, modify assignments, submit or
  approve work, send email, alter subscriptions/modules, or change business data.
  A form labelled Save is not a read-only probe, even if its values look unchanged.
- A prerequisite dependency visible in a client asset establishes an editor rule,
  not the backend rule. Do not ship third-party implementation code in Horae.

## Read-only probe inventory

All rows initially await a fresh browser connection. Historical findings are
linked separately; do not mark a row complete from those alone.

| ID | Surface and evidence to capture | What the current owner can establish | What remains outside that observation |
| --- | --- | --- | --- |
| R01 | Team, active/archived people and available admission navigation | Current number/type of available test actors and any displayed seat gate | Signing in as another profile; purchasing capacity |
| R02 | Owner Permissions page, current public asset URL and configuration shape | Owner restriction, six profile choices, actual catalog/defaults and unknown IDs | Saved profile enforcement or permission revocation |
| R03 | Current permission catalog, including labels and keys | Exact time/expense/project/person/rate/invoice/estimate/report/withdrawal grants | Which lifecycle operations each grant authorizes on the server |
| R04 | Delivered editor prerequisite and classification rules | Required dependencies, floor, tie order and presentation of differences | Persisted prerequisite validation and backend classification |
| R05 | Owner-visible template controls and public editor lifecycle | Available create/apply/delete/rename controls and request shapes in delivered source | Save/delete persistence, uniqueness collisions and assignee propagation |
| R06 | Assigned projects and project editor, without toggling controls | Tracking membership, manager designation and rate controls are distinct | Non-owner assignment authority, promotion confirmation and concurrent revocation |
| R07 | Assigned people page, without changing relationships | Owner behavior and available controls/help | Who may manage another person's assignments under custom grants |
| R08 | Person Rates and project billing settings, read-only | Exact labels/fields for default, historical and project-specific rates | Managed-person versus managed-project rate access and financial redaction |
| R09 | Approval page, filters, unsubmitted/pending/approved views and lock banner | Enabled approval experience, displayed selection/date/status controls, company cutoff and empty states | Approval, overlap splitting, empty-cell/new-project locks, withdrawal persistence |
| R10 | Preferences/Modules, read-only | Availability, self-approval, deadline, company timezone, auto-lock and auto-submit configuration | Effects of changing modules/settings; custom-profile approval eligibility |
| R11 | Expenses and detailed expense report | Owner-visible lifecycle/download/billing actions and current lock explanations | Non-owner receipt/category/billing/correction authority |
| R12 | Invoice, estimate and retainer navigation/forms, without saving | Owner-visible lifecycle/payment actions, module restrictions and distinct entry points | Managed invoice draft versus finalization, mixed-scope invoices, non-owner retainer authority |
| R13 | Reports and saved reports navigation | Available report families and any owner-visible inactive-owner distinctions | Restricted rates, derived totals, download scope and inactive-owner authorization |
| R14 | Settings and own profile help/navigation | Owner versus ordinary-profile explanations and account-only controls | Non-owner profile/permission visibility, identity-provider policy and ownership transfer |
| R15 | Existing activity/history, if already available | Historical actor/date/scope and any explicit before/after states | Missing events, inferred withdrawal states or newly manufactured history |

## Fresh documentation findings

### D01 — Assignment read, assignment write and profile administration differ

The [people-assignment article](https://support.getharvest.com/hc/en-us/articles/4422314817677-Making-people-assignments-for-Managers)
reserves editing managed-person relationships to administrators. The
[teammates API](https://help.getharvest.com/api-v2/users-api/users/teammates/)
also documents administrator-only access, a Manager subject requirement and
replacement of the complete assignment set on update. Neither source explains
the six-profile/custom-grant mapping. Keep read versus write and relationship
replacement as separate cases in FR-005; do not infer a web grant from this API.

### D02 — Profile visibility and assignment promotion need explicit resolution

The [person-profile article](https://support.getharvest.com/hc/en-us/articles/360048687291-Person-profiles)
describes non-admin access to their own or managed people's permissions and an
explicit confirmation before promoting a Member through a project-manager
assignment. It also describes immediately persisted manager-checkbox changes.
Therefore R06 must not toggle a checkbox as a supposedly harmless preview.
The [new permissions reference](https://support.getharvest.com/hc/en-us/articles/44171549176077-Permissions)
instead says role visibility/change is administrator-only. Treat this as a
reference conflict, not evidence to remove Horae's required own-access explanation
(FR-012) or to grant non-admin access to another person's configuration.

### D03 — Retainer funding is not an ordinary invoice operation

The [retainer guide](https://support.getharvest.com/hc/en-us/articles/360048180992-How-to-create-and-draw-from-a-retainer)
documents administrator-only creation/editing. It separates adding funds from
drawing funds through eligible work invoices; even saving an unchanged eligible
invoice may apply newly available funds. R12 therefore inspects forms only.
The [legacy Manager exclusions](https://support.getharvest.com/hc/en-us/articles/30704133376269-What-can-people-with-Manager-permissions-never-access-in-Harvest)
also distinguish retainer administration from drawing on one. These descriptions
do not establish the newer Accounting/Executive Manager or custom invoice-grant
boundaries. Keep retainer create/fund/draw/archive/delete and payment recording
as separate operation-matrix rows, not one blanket invoice-write grant.

### D04 — Public API cannot substitute for custom-template interaction

The [Users API](https://help.getharvest.com/api-v2/users-api/users/users/#custom-profiles)
explicitly does not offer custom-profile creation or editing. Its documented
profile application behavior supplements the editor evidence but is not an
alternative way to test template persistence with an immutable owner. A failed
or refused owner mutation would not establish normal non-owner behavior either.

### D05 — Company cutoff is distinct from scoped approval coverage

[Timesheet Lock](https://support.getharvest.com/hc/en-us/articles/46563687732877-Understanding-Timesheet-Lock)
documents a single company cutoff shared by manual and scheduled locking.
Automatic execution cannot move it backward; a manual change can, without
cancelling the schedule. Manual dates cannot exceed today in company timezone.
Scheduled modes follow a deadline, the previous week/month or a 1–90-day window.
Approval-module removal stops scheduling without clearing the existing cutoff.
Auto-submit accompanies auto-lock; affected running timers are stopped.

The [lock Q&A](https://support.getharvest.com/hc/en-us/articles/46563806657421-Timesheet-auto-lock-and-manual-lock-Q-A)
reserves cutoff changes to administrators with company-write authority. Locking
does not itself approve or invoice entries; lock-triggered submission does not
send manager notifications. A submission deadline alone is not an editing lock.
The guide's blanket editing prohibition and the Q&A's administrator exception
leave privileged correction unresolved. Do not infer custom-grant behavior.

FR-019 already requires independent administrative locks. Record this newly
documented mechanism separately from scoped approval, invoice, archive and
project-period restrictions. In particular, clearing the company cutoff must
not be interpreted as withdrawing all other kinds of protection. R09/R10/R15
will check rollout and existing evidence without applying or removing a lock.
This discovery does not authorize implementing a scheduler in this research PR.

Direct opening of these two sources failed during this session; following their
links from the [general approval article](https://support.getharvest.com/hc/en-us/articles/360048181832-Submitting-and-approving-timesheets)
returned their content successfully. The same general article mixes older
whole-week statements with newer features; it cannot override the current
flexible-approval contract.

### D06 — Approval unknowns remain concrete, not discretionary simplifications

A fresh reading of [flexible approval](https://support.getharvest.com/hc/en-us/articles/39974542812429-Flexible-timesheet-approval)
still does not settle post-withdrawal submission state, future project coverage
after whole-submission approval, or overlapping-interval removal. It describes
force submission and combined time/expense workflows without supplying the
arbitrary-custom-grant predicate. Its current text does explicitly describe
configurable Project Manager self-approval, disabled by default. Keep that as
documentation evidence until the account preference and enforcement are tested.

The read-only owner investigation may establish available controls or existing
history, not every transition. A second actor and authorized disposable mutation
fixtures remain necessary where no existing evidence distinguishes the outcomes.

## Current session status

- Current account state is not yet revalidated: Playwright MCP handshake succeeded,
  but both browser-list calls on the same client timed out after 240 seconds each.
  The client remains available; no successful browser connection or fresh read-only
  probe is claimed. Further browser retries await new connection evidence.
- The historical public asset `permissions-MQ3TAOLU.js` returned HTTP 404 on
  2026-10-02. Its previously recorded conclusions retain their historical date;
  the current editor must supply a replacement asset URL.
- No account writes have been attempted. No claim is made that the account still
  has exactly one person until R01 is rechecked.
- Next: connect the existing browser session, complete R01–R15 where accessible,
  record dated evidence and explicitly classify inaccessible versus untested rows.

## Acceptance boundary

This research increment is complete only when every read-only probe has an
evidence-backed outcome or an exhausted access limitation, new findings are
reconciled with the existing contracts, and an adversarial evidence review finds
no unjustified promotion of observations into enforcement claims. That does not
complete T006, the full spec, implementation, or PR acceptance: non-owner and
state-changing discrimination cases may still require separate reference access.
