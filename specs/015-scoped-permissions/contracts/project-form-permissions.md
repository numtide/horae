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
| Fee budgets, fixed-fee schedules, taxes/discounts, client default rate | Do not silently classify these as ordinary project hourly rates; remaining operation/field rules need evidence |

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

Current authority must be loaded under the organization/actor gates before
protected effects, including replay. Preserve project revision checks and atomic
validation of the full requested association set. Identity choices are neither
scope nor reservations. Rate-mode/type switches, association removals and resets
must be included in the effect review; they can change money without posting a
numeric field. Do not publish a canonical form that only guards numeric inputs.

Review `project_edit_requests` replay payloads before changing serialization.
Keep original explicit intent distinct from its server-merged form: a retry must
not become different intent merely because protected current values were loaded.
Do not replay historical authority or reveal withheld values in mutation results.

## Required evidence and remaining entry gates

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

Before implementing the complete form cutover, settle initial designation scope,
create-time managed financial scope, and non-rate monetary/settings ownership.
Creation authority must remain useful without silently granting rates or forcing
an invented non-billable-only product. These open predicates do not reopen the
already-approved FR-021/022 rules or block independent existing-project tests.

Adversarial review rejects “redact to empty then save”, hidden defaults in picker
responses, role-name cost checks, mode switches bypassing write gates, financial
cascade deletion through team editing, and replaying a merged privileged form.
This inventory identifies required tests, not evidence that those defects are
currently exploitable under a deployed canonical policy or already repaired.
