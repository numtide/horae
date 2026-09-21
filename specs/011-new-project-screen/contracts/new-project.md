# Contracts: New Project

## Routing and screen

- `/projects/new` is a static authenticated route inside AppLayout, before `/projects/:id`.
- Projects New project/empty-state actions navigate to it. Existing editing and bulk actions remain functional.
- Heading, label grid, type cards, tasks/team, invoice defaults and persistent footer match the handoff; no DevBar or second sidebar.
- Form controls have visible labels and stable IDs. Optional dates support clearing; tags use Enter/comma/remove and cannot trigger form submission.
- Cancel/back retains an acknowledged draft; explicit Discard draft confirms deletion. Save project is disabled with missing required fields or pending commit.
- Use explicit loading/error/empty states, preserve unsaved input, announce errors/save status and restore modal focus.

## Typed server functions

All use the existing authenticated Dioxus surface, named error status constants and server-side org/role checks. Request types contain no trusted org/user authority.

| Operation | Input | Result/behavior |
|---|---|---|
| Creation options | Search/page for client, task or person catalog | Bounded active same-org choices plus archived-client status; permitted rates only, admin costs only; supported currencies and mail availability. |
| Resolve selected client | Client ID | Minimal same-org client identity/currency, including archived status, for draft selections outside the catalog page; unavailable/foreign IDs return no result. |
| Resolve selected tasks/people | Task IDs and user IDs, at most 500 each | Active same-org catalog projections independent of pagination; duplicates collapse, unavailable/foreign IDs are omitted and manager responses exclude costs. Creation still revalidates all references. |
| Load current draft | None | Own current draft/revision/save time or no draft; read does not create a row. |
| Save draft | Draft ID or new request ID, expected revision, typed raw form values | Acknowledged ID/revision/time; initial save is idempotent; conflict never overwrites. |
| Discard draft | Draft ID, expected revision | Marks own incomplete draft discarded; stale/completed requests conflict. |
| Finalize draft | Draft ID, expected revision, latest form snapshot | Atomic validated creation, returns project ID; completed retry returns same ID; unauthorized/invalid request changes nothing. |
| Create client from dialog | Name, supported currency, optional default rate | Explicit authorized creation and selected client result; no prototype defaults. |
| Project progress | Project ID | Allowed hours/budget progress only, no rates/costs/notes; managers and member visibility handled explicitly. |
| Prepare project invoice | Project IDs and period | Eligible time/fee occurrences and defaults or explicit mixed-default/currency conflict. |
| Create/update invoice | Prepared selection plus explicit defaults/overrides | Draft invoice with frozen exact components; normal role/lock/void rules retained. |

New error handling distinguishes bad input (400), unauthorized (401), forbidden (403), missing/foreign resource (404) and revision/state conflict (409). Foreign draft IDs do not disclose ownership. Unexpected failures log sanitized context and return a safe message.

## Draft concurrency

Save requests serialize per page and carry the last acknowledged server revision. Edits during an in-flight save remain dirty and are coalesced into the next request. Acknowledging an older snapshot never marks newer input saved. Finalization prevents new autosave work and submits the latest snapshot after any pending save finishes. Server checks remain authoritative if clients bypass the UI.

Two-tab conflict provides reload-latest without silently discarding local input. Navigation with unacknowledged changes offers wait/retry or explicit discard; acknowledged Cancel/back preserves the draft. No fake timestamps or unconditional success toast.

## Authorization matrix

| Actor | Create | Own draft | Project progress | Rates | Admin notes/cost overrides |
|---|---|---|---|---|---|
| Organization admin | Yes | Yes | All org projects | Yes | Yes |
| Organization manager | Yes | Yes | Existing authorized org scope | Yes | No |
| Project lead, org member | No | No | Assigned project | No | No |
| Ordinary assigned member | No | No | Only if everyone visibility | No | No |
| Other org/member | No | No | No | No | No |

Personal timesheets remain readable by their owner. Project settings do not expose another user's time-entry details. Own running timer stop is safe after restriction revocation; starting/editing restricted work is refused. Historical archived-context edits keep existing permissions. Privileged approvals/imports retain their existing authority.

Project list, spend and team reads use `project_read_access` with the actor's current active database role and organization, not a role cached in a caller DTO. Missing settings preserve assigned legacy-member progress. Project leads/admins receive assigned progress without financial rates; an ordinary assigned member needs `project_members` visibility. Historical own entries grant identity lookup only, never team or progress access.

Timesheet and Timer use separate rate-free `list_tracking_projects`/`list_tracking_tasks` resources. They retain archived identities from the viewer's own history; the ordinary task catalog remains active-only. Unauthorized optional rates/budgets are absent in serialized responses and remain deserializable as absent values. Tracking identities are not a grant to start/edit work: the existing context and mutation guards stay authoritative.

Count, pagination, export and detail queries use the same authorization projection. Do not add private fields to plugin payloads/granted tables. Existing organization-report manager gate remains.

Project CSV and XLSX exports require `can_view_progress` just like Projects. XLSX size/count checks apply that filter before limits and share their existing read-only repeatable-read transaction with the payload query. Harvest project counts, pages and detail use the same view; unavailable detail is 404. Own time-entry project identities remain readable without granting project-wide budget access.

UI task reads and Harvest task counts/pages/detail share `task_read_access`: active organization managers/admins see the catalog; members see assigned-project tasks and their own historical task identities. Task identity does not grant tracking access. Member task rates are omitted. Harvest time-entry queries revalidate the current active actor, even if the supplied role is stale. Entries omit `budgeted` without progress access, omit member financial rates, and expose project cost overrides only to administrators. Managers retain legacy profile costs only when no project override exists; an inaccessible override is not replaced with a fabricated fallback.

Grouped reports resolve organization and manager/admin authority from the current active viewer ID. A manager's group containing any project cost override omits `cost_cents` altogether; it does not expose the override, substitute a profile rate or return a partial cost. Other groups retain their legacy costs, while administrators see exact effective totals. The UI labels missing group/grand-total costs `Restricted`; billing amounts and worked hours remain available. Detailed time CSV/XLSX exports contain no cost fields.

## Billing and downstream invariants

Missing settings preserves all legacy rates/amounts. New selected modes have parity across Rust/SQL, project spend, reports, invoice preparation and compatibility exports. Zero is not absent; currency mismatch is not conversion.

Configured fixed fees use fee occurrences, not synthetic time. Non-billable projects never supply invoice lines. Monthly budgets use the selected month; lifetime tracked totals remain available separately. Per-task/person budgets are keyed to real selected entities.

Invoice defaults are snapshots, editable on drafts; conflicting defaults require explicit resolution. CSV/XLSX/PDF and displayed totals include every adjustment component and identify currency. A new project does not issue invoices or send client email.

## Optional email deployment

Administrator configuration supplies absolute sendmail-compatible executable and sender; no shell command text. Availability is returned truthfully to the UI. Reject header/recipient injection, bound subprocess time/output, use direct stdin message and fixed arguments. Tests use an isolated stub, never the user's mail configuration.

Outbox claims are event-kind scoped and lease-fenced. Logical uniqueness prevents repeated enqueue for the same project/scope/period/threshold/recipient. Message-ID remains stable; retries after unacknowledged acceptance may duplicate receipt. Permanent failure is observable and never displayed as sent.

## Reference component mapping

- Heading: Newsreader display family and existing `text-4xl` (34 px); body/control copy uses Instrument Sans and `text-sm`; numeric values use the existing mono family. Do not change shared heading sizes.
- Label rows: 220 px label column plus flexible content, 24 px gap and 20 px row spacing. Use a prefixed structural grid only for the column relationship; compose `gap-6`/spacing utilities for the rest. Collapse rows and three type cards according to the target's narrow-layout rules.
- Main panel: 1,180 px maximum reference width, 24 px top and 40 px horizontal padding; reserve sufficient bottom space for the sticky action footer. Add a token/utility for missing dimensions, not inline pixel rules or copies of shared controls.
- Inputs: background/border/text map to `--color-bg`, `--color-border-input`, `--color-text` and `--radius-btn`; retain shared focus treatment. Required labels and optional clearing must be real controls, not clickable unlabelled spans.
- DatePicker: source reviewed in full; single-day selection, month navigation and Today use the real clock. Start/end inputs remain independently clearable. Reuse the existing picker; never import the prototype's fixed July 2026 clock.
- Dropdown/SegmentedControl: reuse existing shared controls with opt-in accessibility/search additions where needed; selected values come from the form, not handoff defaults.
- Toast: source reviewed in full; shared success/error notification with dismiss button after a real acknowledgement. Draft status remains on the page and cannot be inferred from a transient toast.
- Rail and DevBar: reviewed as reference only. Keep the existing application shell and Projects active navigation; do not recreate the timer, user menu, demo identities or developer state controls.

Catalog pages contain 50 rows plus explicit `more_*` flags; offsets above 10,000 require narrowing the search. Selected values on draft recovery must still resolve even when outside the first page; the UI integration must cover that case rather than substitute a different item. Email availability reflects the validated deployment configuration; absent configuration keeps the existing disabled control and explanation. Neither availability nor a queued job means a message was delivered.
