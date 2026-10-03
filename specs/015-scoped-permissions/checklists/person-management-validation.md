# Local readiness: pure person-management prerequisites

This checklist permits T050–T052 only. It is not runtime acceptance or completion
of the full feature's requirements checklist.

- [x] FR-028/029/031 are explicitly approved; do not ask those choices again.
- [x] Existing catalog and strict restoration are complete (T027–T029, T047–T049).
- [x] [Contract](../contracts/person-management-validation.md) maps exact current grants and self-link cases without guessing operation predicates.
- [x] Tests, implementation paths and regression/review tasks are defined in T050–T052.
- [x] Compatibility and self-link validation cannot confer writer authority; FR-027 and tenant/revision/audit checks remain server obligations.
- [x] No new schema, runtime consumer, real data mutation, activation or migration mapping is included.
- [x] Local tests need no Harvest account, PostgreSQL, saved-profile classification or approval-execution decision.

Implementation results remain pending: T050–T052 must record red/green tests,
core regression and review evidence before being checked off. A passing local
increment does not complete US2, T006–T009 or T020.
