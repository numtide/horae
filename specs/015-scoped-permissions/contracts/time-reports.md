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

## Required follow-through

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

## Acceptance

Cover profile/custom scope unions, historical labels, malformed tenant parents,
date and multi-ID filters, duplicate tags, exact rounding/billable semantics,
absence of populated sensitive fields, empty/foreign results, keyset boundaries,
requester mismatch, invalid policy/state, revocation and actor deactivation.
Exercise the registered session endpoint, not only the storage helper. Complete
SQLx preparation, native/WASM checks and adversarial review. Consumer/browser and
equivalent CSV/XLSX delivery remain mandatory before OP25/OP31 acceptance.
