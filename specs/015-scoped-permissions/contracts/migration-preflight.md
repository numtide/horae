# Legacy permission migration preflight

Owner: US5, FR-006/010/014/017/018; migration cases M01/M07/M08.
This is an internal read-only diagnostic, not the complete migration preview,
an activation token, an approved mapping or permission to repair records.

## Boundary

`permissions::preflight::read` accepts server-authenticated organization/person
IDs. Before any diagnostic query, use explicit READ COMMITTED, organization
SHARE and then actor SHARE. Require policy version zero and a current active
same-organization legacy Administrator. A staged canonical Administrator cannot
substitute for the active policy. Deny other/unknown policy versions, missing
organizations, foreign/inactive/non-Administrator actors without diagnostics.
Recheck actor eligibility after its lock wait. Use one connection and release
all locks before returning. No DML, audit write, migration or external call occurs.
Bound statements to five seconds and idle transactions to ten seconds; database
errors remain typed internally. There is no public error serialization here.

Collect all counts in one SQL statement after authorization, so the report's
categories share a statement snapshot. This is evidence at inspection time,
not a fence over business records or authorization for future activation.
The eventual reviewed activation must inspect/fence its inputs again.

## Diagnostic projection

Return only integer counts, without resource IDs, names, rates, payloads,
filenames, credentials or foreign organization details:

- Cross-organization tracking memberships touching either a project or person
  belonging to the inspected organization. No child settings are required to
  detect the invalid parent pair. A wholly unrelated pair is not included.
- Cross-organization approvals touching the organization through ownership,
  subject or approver. Count an affected row once even if both references are
  foreign. Include inbound references: their unique subject/period pair may
  interfere with the subject organization's submissions.
- Organization approvals in approved state missing an approver or timestamp;
  non-approved rows with any approval attribution; and unexpected open/invoiced
  approval states, as three independent counts. Existing writers only create
  submitted rows, approve them or delete on reopening. Report the stored facts,
  never invent attribution or historical coverage. Counts may overlap and must
  not be summed as a unique number of affected records.
- Import jobs with NULL original requester, separately counting queued/running
  and terminal states. Include both existing Harvest API and CSV job kinds;
  unrelated kinds are not implicitly classified as user-initiated imports.

Membership/approval anomalies are source-integrity findings. Unknown historical
requesters are a distinct transition-policy dependency, not proof of corruption
or trusted service authority. Zero findings do not establish migration readiness:
coverage conversion, role deltas and other M01–M08 checks remain outstanding.

## Acceptance

- Clean and populated legacy fixtures; both cross-tenant membership directions,
  wholly unrelated rows and overlapping approval anomalies have exact counts.
- Approval fixtures distinguish ordinary submitted rows from missing approved
  provenance; complete actor/time evidence is not treated as missing.
- Both import kinds, all five statuses, known requesters and unrelated jobs;
  preserve job payload, lease, generation and report artifacts unchanged.
- Reject missing/foreign/inactive/non-admin identities and nonzero policy modes;
  staged canonical privileges do not confer legacy access.
- A revocation winning the organization or actor wait denies disclosure under
  inherited REPEATABLE READ. A new call observes newly committed findings.
- Compare source row identities and full stored values before/after inspection;
  failure/cancellation releases the connection and locks. No repairs or events.

Tests run only on disposable PostgreSQL. No public endpoint, UI control or
real-data inspection is introduced by this increment.
