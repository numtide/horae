# Project form permission integration

Owner: OP10/11/12/15/16/17; FR-002/006/007/008/010/017/018/021/022/026.
Inspected source: `6b5dbae`, 2026-10-04. This is a field/effect integration
inventory, not a closed contract for every composite form operation or permission
activation. The identity-only picker in `project-people-picker.md` is complete;
the form must not infer financial or delegation authority from those choices.

## Reference and approved decisions

Reopened the current [permission reference](https://support.getharvest.com/hc/en-us/articles/44171549176077-Permissions),
[rate setup](https://support.getharvest.com/hc/en-us/articles/360048181492-Setting-billable-rates),
[rate editing](https://support.getharvest.com/hc/en-us/articles/12522267831181-Editing-billable-rates),
[cost-rate guide](https://support.getharvest.com/hc/en-us/articles/360048687391-Setting-and-editing-cost-rates)
and [project creation guide](https://support.getharvest.com/hc/en-us/articles/360048686831-Create-and-duplicate-projects).
The new-model guide separates project editing from billable/cost read and write.
The cost guide still contradicts itself about project overrides and uses older
Administrator-only wording. FR-021/022 already resolve those Horae predicates;
do not ask for them again or treat legacy wording as a new restriction.

Rate setup distinguishes project rates from person defaults/overrides and task
defaults copied into new projects. Rate editing distinguishes effective-dated
project/person changes from task changes; an authorization change is not permission
to reinterpret historical entries. This investigation adds no rate-history schema
or claim that current scalar storage implements all Harvest history behavior.

The creation guide includes initial manager selection, but its promotion prose
does not establish the custom-grant boundary. FR-026 explicitly leaves creation
separate. A product question is pending: allow `ProjectCreateAll` to select
initial active, already-compatible managers (including an explicitly selected
creator), or create without designations and delegate later with project-edit
authority. Neither option is accepted yet. No automatic creator designation or
global permission grant follows from either a picker response or draft ownership.

No new browser probe is claimed. The retained owner-only editor cannot distinguish
restricted financial access; the new-project control was plan-limited. Repeating
that observation would not answer the custom-grant question.

### Budget and note evidence, checked 2026-10-04

The [budget guide](https://support.getharvest.com/hc/en-us/articles/360048686811-How-to-set-project-budgets)
distinguishes hours from fee budgets. Its older Manager rules require additional
billable-amount permission for fee-budget visibility; sharing project reports
opens only limited hourly progress to ordinary teammates. An absent per-person
or per-task budget excludes that contribution, whereas zero is an actual limit.
Budget edits affect historical budget comparisons, unlike effective-dated rates.
Thus neither a universal monetary-field gate nor a universal preserve-history
rule correctly describes the form. The guide does not establish new custom-grant
write predicates for each budget field.

The creation guide and [invoice project context](https://support.getharvest.com/hc/en-us/articles/360048686671-Getting-project-context-when-invoicing-Fixed-Fee-projects)
describe a separate account preference for project-note visibility, even inside
an authorized invoice. Invoice access does not automatically reveal notes.
Horae's existing `admin_notes` is explicitly Administrator-only in feature 011;
it must not silently become Harvest's configurable shared project notes or become
writable through a cost grant. Keep that accepted boundary while reconciling the
missing note-preference behavior with account settings. No note preference or
new monetary permission is invented here.

## Current production path and required separation

Paths below are under `crates/horae/src/`.

| Path / value | Current behavior | Required integration boundary |
| --- | --- | --- |
| `models/project_creation.rs`: `ProjectForm`, task/member inputs | Financial edits are raw strings; empty and zero differ, but unavailable and untouched have no representation | Preserve raw authorized draft input; transport explicit unchanged intent separately from reset/inheritance and zero |
| `server_fns/project_creation/options.rs` | Search and selected-ID resolution include person/task/client defaults; role controls only costs | Identity selection must not expose default rates. Authorize each financial projection independently |
| `editing::load_project_form` | Returns project rate, fees, budgets, invoice defaults and catalog defaults; legacy Admin controls notes and costs | Project operation permission is only the first gate. Use approved rate/cost predicates and separately settle non-rate fields |
| `editing::load_members` / `load_tasks` | Optional stored overrides become empty strings; assigned archived identities remain visible | Distinguish missing override from withheld data; preserve archived saved rows without making them new picker candidates |
| `editing::save::save` | Compares and saves the complete form; validates and stores the submitted payload for replay | Authorize intent, merge unchanged protected fields inside the transaction and validate the resulting complete state before writes |
| `editing::associations::save_member` | Empty person rate can inherit; Admin-only cost writes delete an override on empty input | Explicit reset requires the corresponding write permission. A hidden field must never become an empty update |
| `editing::associations::save_tasks` | Empty changed task rate may copy the task default; mode and currency affect interpretation | Task/project overrides use project-owned authority; global defaults remain independently protected |
| `editing::associations::save` | Removing a member can delete linked settings; a private-cost guard currently depends on Admin role | Review all indirect financial deletions; do not remove protection merely by replacing the role name with project-write authority |
| `finalize::finalize_draft_record` | Rechecks legacy creator, creates related objects and applies rates in one transaction | Recheck creation plus every requested effect, including inline catalog writes and initial designations; draft save/ownership grants neither |

`design/project/app/13_New Project.dc.html` separates billable rates, budget,
team costs, manager checkboxes and invoice defaults. It is not an authorization
matrix: sample role labels, sample amounts and its general manager-visibility
hint do not override FR-008/021/022. No new CSS or form layout is required to
express the underlying permission separation.

## Settled financial dimensions

Reuse `horae_core::permissions::rates`, not another catalog or policy engine.
The server supplies trusted actor, organization, actual field owner and current
management relationships. Operation/state constraints remain additional gates.

| Field | Financial dimension |
| --- | --- |
| Existing project rate; its person/task overrides; authorized effective inherited project values | `BillableRateOwner::Project(project_id)`, separate Read/Write |
| General person default or unrelated history | `BillableRateOwner::Person(user_id)`; never infer access from managing one of their projects |
| Global task default | `BillableRateOwner::GlobalTask`, requiring all-rate authority |
| Person or project-specific cost rate | Explicit all-cost Read/Write; no managed-cost or Administrator-name shortcut |
| Existing `admin_notes` | Retain feature 011's Administrator-only field boundary, independently of cost grants; configurable Harvest project-note visibility is a separate reconciliation gap |
| Monetary budgets, fixed-fee schedules and project invoice defaults (terms, purchase order, taxes and discount) | User-confirmed option A, 2026-10-05 (FR-034): the corresponding project-scoped billable-rate Read/Write grant, in addition to project operation authority |
| Hour-only budgets | Ordinary project authority, without an added financial grant |
| Client default rate | Independent catalog ownership remains unresolved; FR-034 does not authorize client-default reads or writes |

An authorized effective inherited value is not permission to expose every default
in the catalog. Source resolution must match actual project billing mode and
currency; no unauthorized history query, fallback to zero or currency conversion.

## Protected-field transport and save invariant

The implementation must distinguish:

1. **Withheld read**: no sensitive value in the serialized response, errors or
   client state. It is not the same as a readable field with no override.
1. **Unchanged write intent**: retain the current server value without echoing it
   from the browser. This is usable by an otherwise authorized project editor
   without financial read/write permission.
1. **Explicit update/reset**: requires current field-write permission even when
   the requested value equals storage. Zero is an explicit amount; reset returns
   to the field's actual inheritance semantics, not a universal zero/null rule.

Use a narrowly scoped typed intent, not magic strings, `Option<bool>`, caller
authority flags or a general patch framework. Do not yet change the legacy wire
contract piecemeal: reader, editor state and save must agree on that distinction.
Existing raw draft validation remains necessary for incomplete authorized input.

Internal parsed intent is `horae_core::permissions::rates::RateEdit`:
`Unchanged` performs no field write, `Reset` removes the field's override, and
`Set` carries exact minor units in the already-validated field currency.
`authorize` accepts only a server-derived write decision; explicit edits require
it without comparing storage, so equal-value writes and zero are not exemptions.
This helper does not grant resource access, validate currency/amounts or encode a
public wire payload. It dispatches member-cost saves with explicit preservation,
reset and set outcomes. Canonical saves evaluate current field-specific grants;
legacy saves retain their existing role rules.

The internal existing-project transaction now transports a bounded, duplicate-free
`ProjectEditRequest.unchanged` list of `ProtectedProjectField` identities. A kept
field is restored from current storage; every other protected field is explicit
intent requiring current write permission, including an equal value or empty
reset. Row markers must refer to submitted task/person identities. A canonical
inline New task cannot alias a retained project task after authorization.
Legacy requests use an empty list and retain their original receipt shape.

Canonical receipt version 1 stores original form/normalized keep-list intent and
the required protected-write effects. Retries compare that original intent and
reauthorize recorded effects, including associations removed by the first save.
Never compare a receipt against a newly merged privileged form. Persistence must
honor keep flags directly where validation ignores inactive fields: project
rates, budget settings, fee/invoice defaults and stored milestones. These backend
changes do not yet close the actual form/catalog/manager integration below.

The canonical `pages/new_project.rs::EditProject` consumer now resolves the
project before its context-bound catalog reads; team search and Add everyone use
the separately authorized identity-only picker. Creation and legacy editors keep
their existing paths. A redacted editor response alone does not establish the
financial payload boundary: each refresh must retain the operation context.
The existing `pending_edit` snapshot retains the original request identity and
form across ambiguous failures; preserve that retry behavior for explicit field
intent instead of reconstructing an updated request from server-merged values.

The UI must preserve untouched editable fields too, not just fields it cannot
write. Otherwise a blank stored task override becomes an accidental reset/copy
on a name-only save. Track actual protected-control interactions separately from
value comparison: entering the same value, clearing and zero are explicit intent;
focus and picker search are not. Keep untouched None/hour budgets as well, so
unrelated edits preserve legacy inactive monetary values. An actual hour-budget
edit omits that marker without requiring a financial grant for the hour values.
When both the previous and requested budget modes are non-monetary, that edit
must also preserve any inactive parent monetary amount, including zero. It is
not intent to clear money that the form does not represent. A transition into
or out of a monetary mode still requires billable write authority on both save
and receipt replay and retains its authorized unit-conversion cleanup.
Keep original intent in the pending request; a retry must not reconstruct it.

Preserving Budget retains settings and each retained row's stored values, not a
deleted row's contribution to the active total. When removing a populated
per-task or per-person budget, persist the validated remaining aggregate in its
active unit only. Keep `None` distinct from an explicit zero. Monetary removals
still require billable write authority, including receipt replay; hours-only
removals do not clear unrelated inactive money. Removing an unbudgeted row or an
unrelated association must not silently repair a stale monetary parent total.
Retained member-hour budget rows are also covered by preservation when another
budget mode is active: a name or assignment-field edit must not delete/recreate
them. Skip only the budget write, not assignment or cost persistence. Explicit
budget edits retain their mode-change cleanup semantics, and authorized member
removal still removes that member's dependent budget through the existing FK.

Canonical edit requests must also carry `expected_requester`, copied from the
authorized editor read. Compare both organization and user with the authenticated
session before materializing protected fields or resolving replay. Missing or
different identity requires reload without writes or a consumed request ID, even
when the new session independently has project-edit authority. This binding is
not authority: current activity, scope and field grants are still checked under
the transaction gates. Preserve it in the pending request across ambiguous
failures; do not substitute a freshly observed identity into an old form. Legacy
policy retains omitted-identity compatibility, but cannot accept a supplied
identity that differs from the authenticated session.

Session-mismatch responses use conflict status with error details
`{"reason":"project_editor_session_changed"}`; the reason never contains the
previous or current requester identity. The UI uses the same reason for a
locally detected catalog/people response mismatch. Unauthorized/forbidden
responses and that explicit reason discard the editor, unlike ordinary
validation errors, revision conflicts or ambiguous network/server failures.

The loader is keyed by project identity. Invalidation cancels and clears its
cached resource and unmounts the form and its scoped pending work. A generation
argument rejects callbacks from an earlier instance; a local invalidation latch
rejects late results before applying them. Require explicit reload, with a fresh
form/request identity even if server revisions are unchanged. Navigation to a
different project starts its own loader through a keyed fragment; a key on a lone
component does not remount its state in the pinned Dioxus version. Real-link
component tests cover navigation after access loss and while a save response is
pending, including cancellation of the old response handler. Do not assert that an uncertain save
failed merely because its response handler was cancelled: the newly loaded
saved project is authoritative.

Current authority must be loaded under the organization/actor gates before
protected effects, including replay. Preserve project revision checks and atomic
validation of the full requested association set. Identity choices are neither
scope nor reservations. Rate-mode/type switches, association removals and resets
must be included in the effect review; they can change money without posting a
numeric field. Do not publish a canonical form that only guards numeric inputs.

Review `project_edit_requests` replay payloads before changing serialization.

### Existing-project manager selection

The canonical editor must return the complete `ProjectManagers` snapshot in its
access metadata under the same transaction as the form: current requester,
project ID, access revision and ID/name/activity for every retained designation.
Team manager checkboxes project that canonical set, not `assignments.role`.
People outside the tracking team and archived retained managers must not be lost.

Within the existing Team panel, retained responsibility-only identities remain
visible with the same manager checkbox and a tracking-membership distinction.
Keep the identities locally through uncheck/recheck and tracking removal so
the user can undo unsaved designation changes. This local list is display state,
not an eligibility or authority grant. Archived retained designations can be
removed or restored within the same unsaved edit; do not enable new archived
designations or unlock their tracking/rate/budget controls. Legacy archived rows
keep their previous read-only behavior. No additional manager page or modal is
introduced. The server remains authoritative for additions, scope and replay.

Canonical save intent includes a required manager selection with the original
access revision and complete manager IDs; legacy requests omit it. Reject
duplicates and disagreement between submitted team checkboxes and the independent
selection. Tracking removal alone must not imply designation removal. Canonical
membership writes preserve existing legacy roles and never create a Lead from a
manager checkbox; new tracking assignments use the ordinary membership role.

Reuse the delegation command in the same project transaction, after full-form
validation, with the existing AccessChange organization gate. Failure rolls back
project fields, relationships and both receipts. The form request ID also
identifies the delegation receipt, but an independently completed delegation
receipt must not be accepted as a fresh project save. Resolve original form replay
first under current authority; otherwise reject an already consumed delegation
ID. Save original manager intent with the form's protected-field intent; exact
replay never reapplies a subsequently changed manager set. Managed-only self-removal
may commit, but cannot subsequently reload or replay without current authority.
Keep original explicit intent distinct from its server-merged form: a retry must
not become different intent merely because protected current values were loaded.
Do not replay historical authority or reveal withheld values in mutation results.

## Required evidence and remaining entry gates

### Existing-editor catalog boundary

An existing editor uses `ProjectEditorContext` (project ID plus its captured
requester) for every client/task catalog search. The dedicated reader is
canonical-policy-only: current project-edit authority, active authenticated actor,
tenant/project existence and the matching requester are checked under the shared
organization/actor gates before any catalog is materialized. There is no legacy
role fallback. Context is binding, not authority, and every refresh reauthorizes.

Return bounded active client/task candidates in the same organization. Archived
retained identities still come from the saved editor, not a new-assignment search.
Preserve literal case-insensitive search and the existing client/task offset
bounds (50 candidates, an extra row only to establish continuation, offset at
most 10000). Do not return previous project codes or people in this response.
Person identity selection remains the separately reviewed `project_people` API.

Client choices carry name/currency/activity but never their default rate while
client-default ownership is unresolved. Task choices carry name/billable state;
their global default amount and currency both require the existing pure
`GlobalTask` billable Read decision. Project-managed rate Read alone is not that
authority. This reader grants no inline client/task mutation, designation,
financial write or final project-save authority.

The browser must load the project before requesting these catalogs, bind all
responses to the same context and discard prior requester state on mismatch.
The local editor now loads the project first and uses these catalogs for initial
options and client/task searches. People search and Add everyone use the
identity-only reader, check each response requester and collect all pages before
changing the team. New identity choices cannot populate financial defaults;
retained selected values come from the authorized editor projection. Creation
and legacy editors keep their own catalog path, with no error-triggered fallback
from the canonical path. Local integration now includes protected-control intent
and requester-state invalidation, with archived/outside-team manager controls.
Component and registered-session checks are recorded in `progress.md`. The
registered load/save endpoints authenticate the session and leave policy-aware
authorization to the transaction: legacy Members remain denied, while canonical
project editors are not rejected solely by their old role. Creation endpoints
remain separate. Headless Chromium now verifies withheld/read-only financial
fields, explicit zero/reset and exact retries, retained manager removal and
revocation/reload with a legacy Member holding explicit grants. The fixture is
disposable and passes twice with teardown between runs. Complete effect and
cross-surface/concurrency verification, default-suite and final Nix gates remain
open; this is not real policy activation.

Existing `project_creation/editing/tests.rs` supplies reusable legacy regression
cases, not canonical acceptance:

- `manager_edit_preserves_private_settings_and_cannot_remove_them_indirectly`
  covers ordinary edits with redacted private fields and atomic denial of a
  member removal that would cascade private costs. It deliberately still permits
  legacy Manager billable-rate edits, so its pass cannot establish FR-021.
- `editor_recovers_full_configuration_and_redacts_private_values_after_role_change`
  covers load-time redaction after demotion.
- `editor_recovers_each_budget_scope_without_turning_absence_into_zero` covers
  nullable budget recovery, while existing save/replay/revision tests protect
  the surrounding complete-form transaction.

Reuse these baselines rather than duplicate them. Add the following canonical
cross-product and effect tests when integrating the actual reader and writer:

T159 starts with the real-transaction reproductions in
`editing/tests/canonical_fields.rs`: absent-rate-read serialization, explicit
cost-read with a non-Administrator legacy role, and forbidden task-rate writes
under billable-read-only grants. Their configured fixture is built under legacy
policy before installing canonical permission state in its disposable database.
These are initial RED cases, not acceptance for unchanged intent, all field
owners, catalogs, replay or the complete form cutover.

- Existing-project read/write cross-product: project management versus person
  management; no grant, read-only and write; custom cost grants without Admin.
- Exact serialized payloads with populated hidden values; read-only and withheld
  saves change an ordinary project field without modifying rates/defaults/costs.
- Keep, reset and zero produce distinct intended outcomes; forged updates deny
  atomically, including updates equal to current storage and mode-switch effects.
- Concurrent rate/grant/designation/activity changes reauthorize and preserve
  revision/replay semantics; errors disclose no protected value or target state.
- Inherited person rates versus copied task defaults, incompatible currency,
  archived retained associations, mixed invalid batches and no history rewrite.
- Real form/HTTP/browser tests: absent controls cannot leak values, ordinary saves
  preserve withheld state, read-only displays cannot submit edits, and switching
  people or reloading cannot carry a former requester's financial state forward.

The user answered A on 2026-10-05: non-rate project monetary/settings ownership
is now settled by FR-034. Apply it to existing-project reads and all direct or
indirect save effects, without presenting it as verified Harvest enforcement.
Before the creation cutover, settle initial designation scope and create-time
managed financial scope separately.
Creation authority must remain useful without silently granting rates or forcing
an invented non-billable-only product. These open predicates do not reopen the
already-approved FR-021/022 rules or block independent existing-project tests.

Adversarial review rejects “redact to empty then save”, hidden defaults in picker
responses, role-name cost checks, mode switches bypassing write gates, financial
cascade deletion through team editing, and replaying a merged privileged form.
This inventory identifies required tests, not evidence that those defects are
currently exploitable under a deployed canonical policy or already repaired.
