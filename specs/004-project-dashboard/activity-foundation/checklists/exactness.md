# Exactness and date-boundary requirements quality

Purpose: PR-review gate on the requirements in [spec.md](../spec.md) for exact
minutes and calendar boundaries. Created 2026-10-26; evaluated the same day.

## Completeness

- [x] CHK001 Are inclusive start/end semantics stated for custom and calendar periods? [Spec AFR-001, AFR-002]
- [x] CHK002 Is the meaning of a single-day period defined? [Spec US1]
- [x] CHK003 Is the cumulative basis (period start, billable plus non-billable) defined? [Spec US2, AFR-004]
- [x] CHK004 Is how an all-time selection obtains weekly bounds assigned to a caller? [Spec Assumptions]

## Clarity

- [x] CHK005 Are the distinct error outcomes enumerated rather than "invalid input"? [Spec AFR-001, AFR-005]
- [x] CHK006 Is "clipped" defined as bucket bounds limited to the selected period? [Spec AFR-003]
- [x] CHK007 Is the bucket limit's owner and failure mode (no truncation) explicit? [Spec Edge cases, Assumptions]

## Consistency

- [x] CHK008 Do exactness requirements align with Constitution I (integers, no float, sums equal parts)? [Spec AFR-004, AFR-005]
- [x] CHK009 Is the relationship to the merged spec's D-001/D-005 deferrals stated without contradiction? [Spec intro, Out of scope]

## Edge-case coverage

- [x] CHK010 Are leap day, month-end anchors, year boundaries and all seven week starts covered? [Spec US1, US2, ASC-001]
- [x] CHK011 Are calendar extremes (first/last representable date) specified? [Spec Edge cases]
- [x] CHK012 Are negative minutes, three overflow paths and the largest exact total specified? [Spec Edge cases]
- [x] CHK013 Are duplicates, input order and out-of-period dates specified? [Spec US2]

## Boundaries and assumptions

- [x] CHK014 Are G01, O1, policy 1 and 015 explicitly left unresolved? [Spec Out of scope]
- [x] CHK015 Is the "no timezone conversion" assumption documented? [Spec AFR-006, Assumptions]
