# Combined approval visibility

Status: user-approved Horae rule, 2026-10-02 (C06 visibility / FR-024).
Restricted-user Harvest enforcement remains unverified. No runtime acceptance
is claimed, and the remaining C06 lifecycle predicates are not resolved here.

## Operation boundary

- Construct the intended approval selection from the explicit person/project,
  client and date filters under FR-009. Do not redefine that selection by joining
  only records the caller can read and silently discarding inaccessible expenses.
- Require applicable approval authority and read access to every time and expense
  record affected by that selection. Matching profile names, time editing or
  a financial-report projection do not substitute for these record checks.
- If any affected record is unreadable or outside current approval authority,
  reject the entire command. No time, expense, coverage or successful-approval
  history mutation may survive a failed command. Independent locks remain intact.
- Do not add expense grants to a person's permissions or to the catalog's
  approval prerequisites. Read visibility is checked against actual affected
  records; an expense-free selection does not require an unused expense grant.
  Approval authority is not ordinary expense-write or receipt-download authority.
- Use current persisted authority and the actual affected set at commit under
  the reviewed transaction/revocation protocol. Concurrent new records and
  permission/assignment changes cannot evade the full-selection check or leave
  one domain approved while the other fails. A client preview is not authority.
- Denials explain that the complete selection cannot be approved without
  exposing inaccessible identities, notes, counts, amounts or receipt metadata.
  Successful scope/history remains limited to the authorized selected portion.

An explicitly selected project portion remains actionable under FR-009. This is
different from silently omitting expenses within that portion. Do not remove
expenses from a combined selection merely because the UI cannot display them.

## Acceptance and task traceability

T012 supplies failing policy/transaction cases; T013 implements FR-024 alongside
the reviewed approval coverage contract. Feature 016 supplies real expense
fixtures before combined acceptance. Mock-only or time-only checks cannot claim
that the combined flow works.

| Fixture | Required outcome |
| --- | --- |
| Approval grant, readable time, unreadable selected expense | Whole command denied; both domains and approval coverage unchanged; non-disclosing response |
| Approval grant, selected expense readable but selected time unreadable | Same atomic denial; no alternative record-reading path from approval authority |
| Approval grant and readable selected time/expenses, all other checks pass | Selected portion approved atomically; unrelated projects/dates unchanged |
| Both domains readable but no applicable approval authority | Denied; reading/editing does not imply approval |
| Valid expense-free selection | No denial solely for absent expense-read permission; other checks still apply |
| Visible preview followed by read/approval revocation or new inaccessible selected expense | No stale-authority commit or partial effect, through UI or direct request |
| Broader report permission, forged filters or cross-organization identifiers | No source-read bypass, silent selection narrowing or private error payload |

The approved decision concerns approval, not withdrawal, force submission,
self-approval eligibility, post-withdrawal state, empty-cell coverage algorithms
or receipt permissions. Their remaining evidence/design gates stay open; do not
extend this answer into an unapproved lifecycle rule.
