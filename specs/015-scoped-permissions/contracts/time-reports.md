# Ordinary detailed time reports

Owner: OP25/OP31, FR-006/007/008/010/018. This is the nonfinancial time-report
boundary, not financial-family C02 authority or complete Reports acceptance.

## Reference and scope

The [detailed report guide](https://support.getharvest.com/hc/en-us/articles/360048687171-Detailed-time-and-detailed-expense-reports)
and [Member reports](https://support.getharvest.com/hc/en-us/articles/360048181592-Members-Reports),
rechecked 2026-10-04, document own reporting, person/project scope, multiple
selections and retained archived records. Apply the already confirmed canonical
`TimeReadOwn`, `TimeReadManaged` (person OR project) and `TimeReadAll` predicates;
neither `PeopleRead*` nor a financial report grant substitutes for time authority.
The guides' legacy role wording is not a new custom-profile enforcement test.

Filter candidate discovery remains in `people-directory.md`: report rows do not
define the full candidate universe. Its unresolved zero-record, historical-person
and narrowing questions do not prevent implementing authorized result reads.

## Reader

- Require policy 1, an active session actor and valid current permission state.
  Reuse the time-reader's bounded READ COMMITTED organization/actor SHARE fence.
  Denied/unsupported canonical state must not fall back to legacy roles.
- Qualify entry, person, project, task and client by organization. Preserve
  archived people/projects/tasks/clients in results; current tracking membership is not
  a historical read requirement. Grants combine without duplicate rows.
- Inclusive ordered dates and optional client/project/person/task/tag ID lists
  narrow this authorized set. An empty list means no additional restriction;
  duplicate IDs cannot multiply rows. Match ANY ID within a dimension (including
  tags), and AND across dimensions. Foreign/missing/invisible filter IDs reveal
  no identity and contribute no matches; mixing them with valid IDs does not
  invalidate the valid matches. Tag matches use EXISTS with the tag
  and link both qualified by organization.
- Return entry ID, date, project/task/person names, integer actual and effective
  rounded minutes, billable status and notes. Reuse the current report SQL's
  `effective_minutes` and project/task/invoice billable eligibility. Do not reuse
  the Timesheet's raw billable/rounded projection as report accounting facts.
  Do not select rates, costs, currency, invoice IDs or account metadata.
- Pages contain at most 500 entries, ordered by date, project name, task name,
  entry ID. Use C collation for portable label order and an exclusive cursor.
  Reject a cursor date outside the requested range or NUL in its names; never
  look up its ID. Apply C collation to comparison as well as ordering.
  This is transport pagination, not a period or
  user-visible report-size limit; no full-period totals may be claimed from one
  page. Each page reauthorizes, without promising a cross-page snapshot.
- Return requester identity; an optional expected requester rejects account
  changes before reading rows. It binds the consumer, never supplies authority.

### Full-period totals for the paged consumer

Return `totals` alongside each page: `entry_count`, `total_minutes`,
`rounded_minutes` and `billable_minutes`, all integer 64-bit values. Billable
minutes sum effective rounded minutes of report-billable entries, matching the
existing ordinary time accounting convention. These are time facts, not money.
Compute totals over the entire authorized, date/filter-matched set before the
cursor and page limit. An empty matched set returns zeros; an exhausted cursor
may return no entries with nonzero full-period totals.

Use one SQL statement/snapshot for the totals and bounded page, and one shared
scoped/filter relation, so concurrent entry changes cannot mix snapshots within
a response. Keep the current authority fences, deadlines and strict decoding.
Do not materialize all report notes into application memory to compute totals.
Each subsequent page still refreshes authority and data; the totals are not a
cross-request snapshot or a directory of selectable candidates.

This is an implementation refinement of exact scoped aggregates (FR-008/018),
not a new Harvest permission rule. The Member report guide, reopened 2026-10-05,
confirms period-wide time reporting and rounding; it does not settle restricted
picker eligibility. Verify empty/exhausted pages, more than 500 entries, scope
unions, filters, tenant parents, frozen rounding, large sums and revocation via
the actual session endpoint as well as the reader. T203 remains open until the
ordinary UI, complete pickers and matching delivery paths are connected.

## Required follow-through

### Ordinary consumer integration

Resolve a single authenticated report-access response before mounting any report
or catalog resource. Reuse the export authority gate to read policy, active
identity and current ordinary time authority under the same transaction fences.
Return only requester and a supported legacy/scoped mode. Only explicit policy
0 with eligible Manager/Admin authority mounts legacy catalogs or reports.
Errors, unknown catalogs and canonical denial never fall back to legacy roles.
Canonical ordinary time access requires a supported stored catalog and at least one
`TimeReadOwn`, `TimeReadManaged` or `TimeReadAll` grant. Display gates do not
replace independent server authorization, including the remaining C02 work.

Connect the existing bounded ordinary reader with inclusive dates, cursor
navigation and full-period totals. Keep the requester's identity in the route
component across child remounts and permission retries, pinned by that first
accepted access response. Also retain its policy; changing mode requires an
explicit page reload, not a retry that mounts a different consumer. Canonical reads and
CSV/XLSX links carry that binding; refresh cannot silently adopt another account.
Only a successful, ready response for the exact current date/cursor request may
display rows, totals or downloads. Hide stale data while pending, after denial,
on invalid/reversed dates and on identity mismatch. Retry preserves the binding;
changing dates resets pagination, not identity. Downloads omit the page cursor.
Legacy report/catalog APIs still have their existing independent role checks and
do not yet accept this requester binding; this gate does not claim to repair all
legacy account-switch races or complete their canonical cutover.

Use the incumbent detailed table, native labelled dates and shared controls;
do not change shared CSS or copy the custom report-builder prototype into this
different surface. The first connected result view does not define candidate
eligibility: do not mount legacy catalogs, derive choices from result pages or
fake unresolved pickers. Full multi-selection discovery, grouping and financial
families remain required; this increment does not complete T203 or activate policy.

Verify the actual component with controlled pending/error/success responses,
canonical and legacy mount isolation, date/cursor changes, full-period totals,
escaped labels and identity changes. Then exercise actual server delivery in
disposable Chromium with desktop/mobile captures and legacy regression coverage.

The new reader does not replace the legacy Reports component, grouped monetary
report or downloads. Separate canonical and policy-0 consumers before cutover:
the current page mounts legacy catalogs and financial summary reads before its
role notice. Do not connect that page merely by swapping its detail resource.
Preserve the complete picker and financial-family requirements as open work.

CSV and XLSX must eventually use the same scoped report projection and filters.
Preserve bounded transport and source snapshots, then reauthorize the complete
captured person/project scope before release. As an implementation inference from
`permission-state.md`, `csv-exports.md` and `project-exports.md`, source reassignment
does not replace a captured row's scope: current access to its new project alone
cannot disclose old-project bytes. Check current grants against recorded scope,
not a later lookup that silently changes it. Test both reassignment directions.
Do not reuse a manager-only release check for canonical downloads. Moving XLSX
to READ COMMITTED also requires a single-statement size/payload snapshot, as in
`limits::project::projects`, rather than retaining separate count/read queries.

### XLSX delivery

Use the existing XLSX route, format, renderer admission and output limits. Policy
0 retains Manager/Admin access; policy 1 requires valid ordinary time-read
grants, regardless of the legacy role. Capture the policy version with the
requester and private person/project pairs. Never serialize these scope facts.
Reject a policy change at release rather than falling back between policies.

Hold the organization and active actor SHARE gates for source authorization and
one bounded size/payload statement. Scope and all filters apply before the
10,001-row size probe; oversized results return 413 without materializing their
text in the client. Preserve the 10,000-row, 8 MiB text and 32,767-byte field
limits. Reject reversed dates and page cursors: a download represents the full
selected period, not the current page. The scalar legacy URL filters remain
supported alongside the multi-ID transport defined below. The Reports consumer
and candidate discovery retain their separate T203 requirements.

Commit before rendering. Immediately before releasing the rendered body, load
current policy, active requester and strict grants under fresh gates, and require
every captured person/project pair to be currently readable. Source entry
deletion or reassignment does not replace those pairs. Empty files still require
current time-read authority. Denial, cancellation or failed final authorization
must drop the body and release admission; no partial XLSX is a successful result.

CSV uses the boundary detailed in `csv-exports.md` (T207–T209). Its cursor must be declared
outside the authorization savepoint, with current source authority loaded in
the cursor's own snapshot and validated even when the source is empty. Do not
copy the XLSX transaction lifetime across client-paced CSV backpressure.
PostgreSQL closes cursors created inside a rolled-back savepoint
([ROLLBACK TO](https://www.postgresql.org/docs/17/sql-rollback-to.html)); changing
to `WITH HOLD` instead would materialize their source
([DECLARE](https://www.postgresql.org/docs/17/sql-declare.html)). Both references
were checked on 2026-10-04; neither is a reason to replace bounded streaming.

## Download query transport

The existing CSV and XLSX GET routes accept the same `from`/`to` dates and
optional comma-separated `client_ids`, `project_ids`, `user_ids`, `task_ids` and
`tag_ids`. Parse every UUID strictly; only an entirely empty string means no
additional restriction. Reject malformed IDs, empty interior elements and
whitespace rather than dropping them. Sort/deduplicate IDs before reading.
URL-encoded commas have the same meaning as literal commas. Repeated known
query keys are invalid, not first/last-wins.

Keep the existing scalar `client_id`, `project_id`, `user_id` and `tag_id` links.
Reject scalar/plural conflicts for the same dimension by presence, including an
empty plural value. Neither union nor silent precedence can change the user's
selection. Multiple IDs are OR within a dimension and dimensions remain AND;
foreign/unauthorized IDs cannot grant access. This is a wire-format refinement
of the existing report contract, not a new Harvest product rule.

Optional `expected_org_id` and `expected_user_id` must both be present and valid
or both absent. Map them only to the expected-requester comparison; authority
still comes from the session. Wrong complete binding returns 403; malformed or
partial binding returns 400. Unrelated forged `org_id`/`actor_id` parameters
remain ignored. Reject every `after` parameter, including a bare or empty one;
downloads cover the selected period, never a paged suffix. Reject reversed or
invalid dates. Preserve filenames, media types and bounded export behavior.

Optional `expected_policy=legacy|scoped` binds the mode of the issuing screen.
Reject unknown, empty or repeated values; reject a mismatch before selecting any
export rows under the locked source authority. The canonical consumer always
sends `scoped`. Omitting it preserves existing direct/legacy links. This closes
the interval between a scoped screen load and a later download after policy
changes; the existing source/release policy checks still protect changes during
the download. Test actual CSV and XLSX links across both transition directions.

Verify parsing and both registered HTTP routes, including mixed allowed/foreign
filters, task/tag narrowing, unchanged scalar links, account changes and complete
CSV/XLSX contents. No candidate-discovery assumption or UI acceptance follows
from these transport checks.

## Acceptance

Cover profile/custom scope unions, historical labels, malformed tenant parents,
date and multi-ID filters, duplicate tags, exact rounding/billable semantics,
absence of populated sensitive fields, empty/foreign results, keyset boundaries,
requester mismatch, invalid policy/state, revocation and actor deactivation.
Exercise the registered session endpoint, not only the storage helper. Complete
SQLx preparation, native/WASM checks and adversarial review. Consumer/browser and
equivalent CSV/XLSX delivery remain mandatory before OP25/OP31 acceptance.
