# Observed Harvest profile configuration

Evidence date: 2026-09-30. Read-only inspection of `permissions-config-data-island` on the connected account's Permissions page. This is the configuration delivered by Harvest to its own editor, not an inferred hierarchy and not proof of server-side enforcement. No profile was applied or saved.

Revalidated 2026-10-02: [current-account investigation](current-account-investigation.md)
retains the current catalog, asset hashes and direct-default sizes. Its C01–C07
register supersedes any assumption that template deletion, report-derived access,
managed rates or custom approval enforcement has been settled by this source.

The table records direct defaults for the supported catalog, with the documented immutable own-time floor made explicit. `O` = own, `M` = managed, `A` = organization, `—` = absent from that profile. Project `M` is managed projects; time `M` covers managed people/projects. Financial scope semantics must not be inferred from time scope. Profile order is not a privilege ladder.

| Operation | Member | Project Manager | People Admin | Accounting | Executive Manager | Administrator |
| --- | --- | --- | --- | --- | --- | --- |
| Read time | O | M + O | A | A | A | A |
| Write time | O | M + O | A | O | A | A |
| Approve time | — | M | A | — | A | A |
| Read projects | — | M | A | A | A | A |
| Edit projects | — | M | — | — | A | A |
| Create projects | — | A | — | — | A | A |
| Read clients | — | A | — | A | A | A |
| Write clients | — | A | — | A | A | A |
| Read tasks | — | A | — | — | A | A |
| Write tasks | — | A | — | — | A | A |
| Read people directory | — | — | A | — | A | A |
| Manage people | — | — | A | — | A | A |
| Read billable rates | — | — | — | A | A | A |
| Edit billable rates | — | — | — | — | — | A |
| Read cost rates | — | — | — | A | A | A |
| Edit cost rates | — | — | — | — | — | A |
| Read invoices | — | — | — | A | A | A |
| Manage invoices | — | — | — | A | A | A |
| Withdraw approvals | — | — | M | — | M | A (administrator access) |
| Read account settings | — | — | — | — | — | A |
| Manage account settings | — | — | — | — | — | A |

Absence of general project/client/task/directory access is not denial of the minimal identities required in an authorized person's tracking picker. That separate read contract must be retained without leaking detailed records.

## Reports and catalog conflicts

The live editor differs from the general help article:

- Separate `timers:approve:managed` and `timers:approve:all` grants exist. Approval is not inferred from time editing or mere visibility.
- People Admin includes `reports:read:contractor`; Accounting does not. Accounting includes profitability and invoicing report grants. Executive Manager and Administrator include all three.
- `saved_reports:read:inactive` and `saved_reports:write:inactive` concern deactivated owners' reports, not ordinary personal saved reports. Accounting, Executive Manager and Administrator include both.
- The managed billable-rate labels refer to managed **people**, while the help article describes managed **projects**. Record this conflict; verify actual rate visibility and edit boundaries before implementing it.
- Two Administrator IDs (59 and 60) have no entry in this account's displayed permission catalog. They must not be assigned invented meanings or imported as trusted Horae grants.

Expenses, estimates, Forecast, subscription billing and currently absent report products do not become working Horae features merely because the reference supports them. FR-003 requires explicit product-surface mapping. The general time report must derive its row scope and sensitive fields from applicable implemented capabilities.

The confirmed web scope includes expenses. The 2026-10-01
[expense permission evidence](expense-permissions-evidence.md) records its six
read/write grants and built-in defaults from the current editor configuration.
Category, receipt, billing, approval and privileged lifecycle mapping remains
open; ordinary expense writing must not stand in for all of these operations.

## Customizable dimensions present in the reference

In addition to the defaults above, the catalog includes managed-person read/write, managed billable-rate read/write, managed-project invoice read, managed-project draft editing and managed-project invoice management. These must not be lost by hard-coding only built-in profile defaults.

## Editor prerequisite and classification evidence

The page loaded this [public permission-editor asset](https://cache.harvestapp.com/static/people/permissions-MQ3TAOLU.js). Source inspection was limited to the permission editor; it was not executed as replacement application code. These are observed client-side rules, still requiring persistence/interaction verification:

- Adding a write/create/approve permission includes the matching read permission when available. Managed-draft invoice editing uses managed invoice read.
- Organization-wide reads include managed reads; organization-wide action grants include the corresponding managed action when available. Managed reads include own reads; managed write/create includes the corresponding own action when available.
- Managed invoice management also includes managed draft editing. Dependencies are traversed transitively; removal traverses dependants. Member-floor grants cannot be removed.
- Customization excludes Administrator from automatic best-fit classification. Among non-admin profiles fully contained in the selected grants, the editor chooses the largest permission set; ties preserve source order. If none is contained, it chooses the profile with the fewest missing grants, also preserving order. This classification is descriptive, not a grant of missing permissions.
- Template matching also considers the Member floor and selected permission set. Inspection showed create/save-as-new and delete behavior, not a verified in-place template update/rename contract. Do not invent propagation to assignees.

The delivered source role order is Administrator, Executive Manager, Project Manager, Accounting, People Admin, Member. This is evidence for resolving ties, not authorization precedence.

The replacement asset observed on 2026-10-02 additionally distinguishes initial
template/profile selection (built-in wins a size tie) from automatic non-admin
classification. Report grants describe access to displayed underlying data but
do not add cross-resource rate grants in the client dependency traversal. Time
approval adds time reads, not time writing or expense grants; withdrawal has no
corresponding dependency edge. These are editor observations only. The deletion
fallback warning conflicts with the help article's preservation promise; no
deletion or permission-loss preview request was executed.

## API application evidence

The [Users API](https://help.getharvest.com/api-v2/users-api/users/users/#custom-profiles), checked 2026-09-30, documents:

- Repeating `permissions_profile` preserves individual grants; switching profiles replaces them.
- Returned `access_roles` omits custom identity; writing it back can remove custom grants.
- Profile lookup ignores case/surrounding whitespace. Unknown or ambiguous names fail with 422. Sending both fields is rejected.
- Losing project access removes project-manager designations.
- Descriptive `roles` do not grant permissions.

These are documented API semantics, not observed UI persistence. Lookup normalization does not prove template-creation uniqueness rules. Browser acceptance must distinguish an unchanged save from explicit profile replacement. Horae's read-only compatibility API does not acquire write endpoints through this research.

## Remaining acceptance gaps

The full Horae operation matrix still needs lifecycle mapping, managed-rate conflict resolution, custom save/delete persistence checks, assignment-authority checks, exact approval/withdrawal enforcement and migration review. UI configuration/source evidence alone does not pass those tests. Keep the full-feature gate open while using these findings to replace earlier guesses.

The 2026-10-01 [rate-scope investigation](rate-scope-evidence.md) confirms that
public permission/API documentation does not settle the people-versus-project
conflict. It defines a discriminating reference fixture and records which legacy
API statements cannot be used as new-model enforcement proof.
