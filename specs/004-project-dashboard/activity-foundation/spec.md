# Feature specification: project activity foundation

**Branch**: `feat/project-activity-foundation` | **Created**: 2026-10-26

This bounded child increment of 004 recovers the independent reporting-period and
weekly-activity foundation (historical F001–F003) from #208. It reuses #208
FR-006 (all-time, month, quarter, year and custom intervals, inclusive by work
date) and FR-021 (exact integers, overflow is an error) for pure domain rules
only. It does not regenerate the dashboard specification, lift the merged
spec's D-001/D-005 deferrals, or make any chart, filter or report visible.

## User scenarios & testing

The consumer is a later reporting increment (server projection, chart, export)
that needs period and weekly totals it can trust without re-deriving calendar or
arithmetic rules.

### US1 — Choose an exact reporting period (P1)

Independent test: build each period kind for leap-year, month-end and calendar
extreme dates and compare the first and last included work dates.

1. Two absent bounds mean all time; any other incomplete, empty, malformed or
   reversed input is rejected and never broadens to all time.
1. Equal custom bounds are one included day.
1. Month, quarter and year periods are complete calendar periods containing the
   anchor date, including 29 February.

### US2 — Get exact weekly activity for a period (P1)

Independent test: aggregate known daily billable/non-billable minutes for each of
the seven week starts and reconcile every bucket and the cumulative total.

1. Buckets follow the supplied first weekday, are ordered and contiguous, and the
   first and last buckets are clipped to the period with explicit bounds.
1. Weeks without tracked time are present with zero minutes.
1. Only work dates inside the inclusive period count; duplicates are added and
   input order does not change the result.
1. Each bucket's cumulative value is billable plus non-billable minutes from the
   period start, not project inception, and the last equals the exact period total.

### Edge cases

- A period with more buckets than the caller's limit fails rather than truncating;
  a zero limit always fails.
- Negative included minutes, overflow within a bucket, across the billable split
  or across weeks fail with no partial series; the largest representable total
  succeeds without saturation.
- A first week preceding the representable calendar fails; a final week at the
  last representable date is clipped and succeeds.

## Requirements

- AFR-001: Parse an optional pair of ISO dates: both absent is all time; one
  absent is incomplete; malformed is invalid; start after end is reversed.
- AFR-002: Month/quarter/year periods are inclusive complete calendar periods,
  and every representable anchor date yields a period that contains it.
- AFR-003: Weekly buckets honour the configured week start, cover the whole
  period without gaps or overlap, keep empty weeks and expose clipped bounds.
- AFR-004: Minutes are actual tracked integer minutes split billable/non-billable;
  no invoice rounding, conversion to money or percentage is applied.
- AFR-005: Every arithmetic or date step is checked; overflow, negative input,
  unrepresentable dates and exceeding the caller's bucket limit are distinct,
  explicit errors. No result is truncated, saturated or partially returned.
- AFR-006: The rules are pure: no storage, network, clock, timezone conversion,
  permission or presentation dependency, and no new third-party dependency.

## Success criteria

- ASC-001: For full calendar years 2000, 2024 and 2026 and all seven week starts,
  bucket sums and the final cumulative value equal the input totals exactly.
- ASC-002: Every listed edge case produces its documented result in an automated
  test; the whole core suite and lints pass.

## Out of scope

Project breakdowns, budget or percentage rounding, database readers, server
functions, DTOs, permissions and member visibility (G01), percent rounding (O1),
UI, charts, routes, exports, CSS, migrations and generated query caches. No
policy activation or 015 waiver. Nothing here claims a visible dashboard, chart
or MVP completion; no Harvest or native UI behavior is newly verified.

## Assumptions

- The caller resolves the organization's week start and the bucket limit; the
  later server increment owns those values (historically 5,200 weeks). Weekly
  series need concrete bounds, so an all-time selection is resolved to dates
  by that caller; this increment does not define how.
- Work dates are calendar dates already in the organization's local sense.
