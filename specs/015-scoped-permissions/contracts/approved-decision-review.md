# Approved decisions: Harvest parity review

Checked: 2026-10-20. User approval establishes Horae's intended behavior, not
Harvest parity. The user requested revalidation of previously approved decisions.
This register covers all 21 Q/A entries in `spec.md`; it does not certify all
36 requirements, 238 tasks, or runtime enforcement.

Status meanings:

- **Documented**: the cited Harvest guide supports the stated behavior; this is
  not a restricted-user persistence test.
- **Partial**: evidence supports part of the rule, but not the complete predicate.
- **Unverified**: no discriminating reference outcome establishes the decision.
- **Conflict**: reference evidence contradicts at least one accepted outcome.
- **Product scope**: an explicit scope boundary, not an external behavior claim.

## Decision inventory

| Decision in spec clarification order | Reference status | Remaining comparison |
| --- | --- | --- |
| Six built-ins, reusable custom profiles, individual adjustments | Documented: [permissions guide][permissions]; existing editor observations | Custom save/reload and restricted-user operation matrix |
| Partial project/person approval within selected filters | Documented: [flexible approval][approval] | Whole-submission promotion, future projects, withdrawal persistence |
| Complete web scope; no native apps, Forecast or new integrations | Product scope | Preserve this boundary; existing integrations remain in scope |
| Any existing compatible person-scoped grant permits new assignments, FR-028 | **Conflict AP-01** below | Replace the insufficient eligibility oracle only after resolving the actual manager-level predicate |
| Only Administrators write person-management assignments, FR-027 | Documented: [assignment guide][assignments] and [permissions guide][permissions] | New-model custom-profile enforcement and transactional cases |
| Project editors delegate to already-compatible people, FR-026 | Partial: [person profiles][profiles] describes project assignments and legacy promotion | New-model custom editor authority; separate assignment from privilege changes |
| Retain project designation with read access; confirmed removal on read loss, FR-025 | Partial: [Users API][users] documents removal on loss of project access | Exact read-only threshold, confirmation and saved relationship effects |
| Deny combined approval if any selected time/expense is unreadable, FR-024 | Partial: [flexible approval][approval] combines time and expenses | Arbitrary custom grants, whole-selection denial and hidden expense cases |
| New-model cost read/write grants and built-in defaults, FR-022 | Documented: [permissions guide][permissions]; retained editor catalog | Older [person-profile guide][profiles] still says Administrator-only; restricted-user enforcement not established |
| Deleting a template preserves assignee permissions, FR-015 | Conflicting documentation: web [permissions guide][permissions] versus [Custom Roles API][custom-roles] | AP-02 below; web persistence untested, no deletion authorized |
| Financial report permission permits its defined financial fields without ordinary rate grants, FR-008 | Partial: [reference profiles](reference-profiles.md) records report grants without rate prerequisite edges | Server projections, exports and unrelated source-rate denial |
| Person-default versus project-owned billable-rate scope, FR-021 | Partial/conflicting: [rate evidence](rate-scope-evidence.md) | New-model field-by-field enforcement; managed labels differ between guide and editor |
| Trimmed, case-insensitive custom-profile creation uniqueness, FR-032 | API-documented case-insensitive creation/update uniqueness: [Custom Roles API][custom-roles]; retained editor trims names | Actual web persistence and exact Unicode equivalence remain untested; no longer only lookup evidence |
| Reject person-management self-links, FR-031 | Unverified | Explicit self-link acceptance/denial; do not confuse with self-approval |
| Confirmed relationship removal on loss of last compatible grant, FR-029 | Unverified; depends on AP-01 | Correct eligibility threshold and persistence/confirmation behavior |
| Owner-only terminal stop after tracking-eligibility loss, FR-033 | Partial: [timer guide][timers] allows ordinary delegated stopping | The exceptional lost-eligibility case, not ordinary stopping |
| Include active project participants with no time in Timesheet selection | Partial: [teammate Timesheet guide][timesheet] documents selection and delegated tracking | Zero-entry membership and custom-scope candidate outcomes |
| Monetary project settings require billable-rate authority, FR-034 | Unverified as a complete field rule | Budgets, fees, payment terms, purchase order, taxes and discounts separately |
| Client-default rate requires global rate plus client authority, FR-035 | Unverified Horae extension | No equivalent global client-default-rate contract established in the consulted Harvest guides |
| Client archive requires archived projects; client restore does not restore projects, FR-036 | Documented archive precondition: [clients guide][clients]; separate restoration: [unlocking guide][unlocking] | Custom authority and concurrent creation/restoration; no live lifecycle mutation performed |
| Missing local scheduled time executes at first valid instant, FR-023 | Unverified Horae rule | Harvest server DST-gap execution; monthly clamping evidence does not settle DST |

The DST-fold question remains unanswered and is not a 22nd approved decision.
No decision is silently reversed by this review. A known conflict is nevertheless
an open parity issue, not an accepted deviation merely because the older decision
was approved.

## AP-01 — Person-assignment eligibility (high)

The current [permissions guide][permissions], under “Assigning people in the new
permissions framework,” requires manager-level access. It explicitly rejects
standalone managed-person view/manage permissions on a Member-based custom role
as sufficient for receiving assignments. Administrator assignment authority
remains a separate rule.

FR-028 and [the pure validation contract](person-management-validation.md) instead
accept `PeopleReadManaged` or `PeopleWriteManaged` plus their prerequisite closure
and Member floor. This supplies a concrete contradictory expected result, not
just a disagreement over profile labels.

### Fresh browser and artifact evidence

Read-only navigation to the connected account's Permissions editor succeeded on
2026-10-20. The owner restriction and `disabled: true` remain present. No control
was enabled, profile applied, preview endpoint called or relationship changed.
The account still shows one owner and an invitation link to purchase a second
seat. These observations cannot establish non-owner server enforcement.

The page loaded
[`permissions-5IFM33Q5.js`](https://cache.harvestapp.com/static/people/permissions-5IFM33Q5.js).
SHA-256:
`8095b9d5488de8a371ce95e6dedb0955e9c9cc5a1a90ad55455f1bd1b738b6e9`.
Direct inspection treated the downloaded file as inert text.

| Source location, UTF-8 byte offsets, zero-based/end-exclusive | Observation |
| --- | --- |
| `[999579, 999971)` (`kP`) | Best-fit classification selects the largest contained built-in permission set, otherwise the set with fewest missing grants |
| `[1006721, 1006788)` (`It`) | Automatic classification excludes Administrator |
| `[1008620, 1009032)` (`bn`) | Grant edits recompute the base role from effective grants; the original selected profile is not an immutable classification |
| `[1000808, 1001482)` (`iK`) | Managed people-write requires matching people-read; it does not add Project Manager's time/project grants |

The current DOM configuration gives Member the IDs `24, 25, 38, 44`,
managed people-read `17`, and managed people-write `18`. No other non-admin
built-in is contained in that six-ID combination. Therefore the inspected
classification algorithm would keep this combination as Member. This is a
static inference using the delivered configuration, not an executed form edit.
It corroborates the documented counterexample without demonstrating the private
server's assignment predicate.

REA retained static application evidence
`ev_2cdc13d998cc503abb6479a05635920c1659f69c0f5fffd1cd4ecfa9169e9d54`.
The analysis completed with partial semantic coverage under the selected
1,000-node/200-unknown limits. It does not prove enforcement or complete control
flow; the specific findings above come from bounded source inspection.
Raw artifacts remain outside Git in the primary checkout:

- `.scratch/playwright-windows/output/page-2026-10-20T09-44-42-314Z.yml`
- `.scratch/harvest-rea-20261020/permissions/permissions-5IFM33Q5.js`
- `.scratch/harvest-rea-20261020/permissions-evidence.json`

### Correction boundary and next discriminator

Do not fix this by comparing the displayed profile name, by requiring the entire
Project Manager permission set for every assignment, or by preserving an original
template label as authority. The guide and classification code do not establish
any of those as the complete server rule.

Revalidate the Member-only, Member plus managed people-read/write, Project Manager,
People Admin, Accounting and Executive Manager cases, including customized
reductions and subsequent loss of eligibility. Determine which current capability
or classification actually makes a person assignable, separately from who can
write the assignment. Keep unresolved combinations explicit.

Affected artifacts: FR-028/029 and US2 acceptance; the validation contract and
its T050–T052 oracle; T056–T058 permission-change effects; T012/T013 runtime
assignments; OP47; and the plan's claim of a closed local eligibility contract.
Existing historical task checkmarks mean tests passed against the previous
Horae rule, not that the rule now meets the Harvest parity target.

Until this discriminator is resolved, do not treat the person-assignment
contract as parity-ready or enable it through the deferred client-reader work.
No runtime code, historical task results, migration mappings or approved product
rules are changed by this evidence register.

Follow-up: the current [teammates API][teammates] requires a Manager subject but
does not map that term to all six profiles. The owner-visible Assigned people
page provides an informational message instead of an assignment form, explaining
that Administrator access already covers everyone. Snapshot:
`.scratch/playwright-windows/output/page-2026-10-20T09-51-11-092Z.yml`.
This observation supplies no counterfactual result for custom profiles or an
Administrator's direct assignment request. Do not repeat owner navigation as
a substitute for those missing cases.

The [PM01–PM08 discriminator matrix](person-management-validation.md#harvest-eligibility-discriminators--2026-10-20)
now separates documented negative/positive cases from the unresolved
customization, other-profile and retention outcomes. The Users API also documents
direct-permission updates leaving access roles unchanged. This makes separate
recording of grants, access role and profile necessary when comparing supported
edit paths; it does not prove which fact the web assignment server authorizes.
T006 owns this reference verification. No probe or historical test is marked
passed by defining these cases.

## AP-02 — Custom-role API and web lifecycle conflict (high)

The [Custom Roles API][custom-roles], discovered through the current Users API,
now documents create, update and delete operations. This supersedes the older
claim that no custom-profile mutation API exists; it does not add a Horae API
mutation surface or authorize calls against the connected account.

Its documented deletion resets assignees to Member and clears direct grants,
contradicting the web guide's preservation promise and Horae's approved C01.
It documents update propagation to assignees, while rename preserves permissions.
It also describes authorization through all-user management, unlike the web
guide's Administrator-only template administration.

These are surface-specific reference conflicts, not proof that the web UI uses
the same behavior. Preserve the approved rule until corrected explicitly; do not
silently downgrade users or broaden template-write authority. Revisit FR-011/015,
T006 and the template command/editor contracts before claiming lifecycle parity.
Existing create/delete tests alone do not verify these reference differences.

Creation and update names are explicitly case-insensitive in this API contract.
FR-032's choice now has documentary support beyond lookup normalization. Exact
Unicode behavior and web persistence remain unverified.

Retained public HTML in the primary checkout:
`.scratch/harvest-reference-20261020/custom-roles.html`, SHA-256
`c6f1d6d865d939570d4f2dafea1e166a36c85bb383a47060164b33ce09d63405`.
The [permission catalog API][permission-api] describes account-visible claims,
not manager eligibility. Neither API was invoked; only public documentation was
read. The catalog copy is retained beside it as `permissions.html`, SHA-256
`0f1583d5a459819f025501ff03261111042a6fd7f1c2a593ff017ea73e0855e2`.

[approval]: https://support.getharvest.com/hc/en-us/articles/39974542812429-Flexible-timesheet-approval
[assignments]: https://support.getharvest.com/hc/en-us/articles/4422314817677-Making-people-assignments-for-Managers
[clients]: https://support.getharvest.com/hc/en-us/articles/360048181312-Create-and-edit-clients-and-client-contacts
[custom-roles]: https://help.getharvest.com/api-v2/roles-api/roles/custom-roles/
[permission-api]: https://help.getharvest.com/api-v2/roles-api/roles/permissions/
[permissions]: https://support.getharvest.com/hc/en-us/articles/44171549176077-Permissions
[profiles]: https://support.getharvest.com/hc/en-us/articles/360048687291-Person-profiles
[teammates]: https://help.getharvest.com/api-v2/users-api/users/teammates/
[timers]: https://support.getharvest.com/hc/en-us/articles/360048687651-Why-can-t-I-invoice-approve-or-archive-items-when-a-timer-is-running
[timesheet]: https://support.getharvest.com/hc/en-us/articles/360048687591-How-to-view-edit-and-submit-another-person-s-timesheet
[unlocking]: https://support.getharvest.com/hc/en-us/articles/4408222060301-Unlocking-time-and-expenses-if-the-project-task-or-person-is-archived
[users]: https://help.getharvest.com/api-v2/users-api/users/users/
