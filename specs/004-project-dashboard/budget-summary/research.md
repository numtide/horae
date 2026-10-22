# Research: project budget remaining tile

Read-only, 2026-10-22, at `0243a386`. Reuses the unpublished
`feat/project-summary-tiles` research (`f5784b47`) for monthly scope, budget
boundaries and formatting. Its G01 split question is not part of this tile.
**Evidence** is what a source or the code states; **inference** is our reading.

## R1 — Headline period

Decision: monthly-reset budgets use the calendar month of the server's UTC
date; all other budgets are lifetime. No later period control changes it.

Evidence: [Harvest project analysis](https://support.getharvest.com/hc/en-us/articles/360048181412-Project-analysis)
scopes hours and remaining budget to the month when the budget resets monthly.
A read-only Harvest check (coordinator notes) found the five-tile summary
unchanged when the lower table switched to Month, on a non-monthly project.
Code: `budgets::fetch_progress` filters `spent_date` to
`date_trunc('month', $date)` when `monthly_reset`; `list_project_budget_progress`
passes `Utc::now().date_naive()`. There is no organization time-zone column.

Inference: reusing the list's server UTC date keeps list and detail equal. The
non-UTC midnight limitation of feature 011 remains; no time-zone setting.

## R2 — Budget rules and legacy fallback

Decision: reuse `fetch_progress` and `progress_for_viewer`'s summary rules for
configured projects; fall back to the list's legacy rule otherwise.

Evidence (code):

- `fetch_progress` reads only projects with a `project_settings` row. It
  applies scope, effective (rounded) minutes, `include_nonbillable`, invoice
  line amounts and fee redaction (`can_view_money`).
- `progress_for_viewer` summarizes project scope directly and task/person
  scopes with `horae_core::budget::allocated_totals` (blank excluded, zero
  counts, overflow → `None`). Whole-project totals are computed before private
  breakdown filtering; that is the list's existing contract.
- The list (`pages/projects.rs`) uses the configured summary when its kind is
  not `none`, else `row_spend`: legacy hour budgets against all raw tracked
  minutes, fee budgets against `fetch_project_spend`'s `spent_cents`, lifetime.
- Configured and legacy rows share `projects.budget_kind`, so kind `none`
  means no budget in both paths.
- The list redacts `budget_amount_cents` with the same authority as money
  (`a.can_view_progress` under policy 0; `BillableRateRead*` under policy 1).

[Harvest budgets](https://support.getharvest.com/hc/en-us/articles/360048686811-How-to-set-project-budgets)
distinguishes blank from zero allowances; [Harvest rounding](https://support.getharvest.com/hc/en-us/articles/360053116772-How-does-time-rounding-work)
applies rounding per entry to budgets. Both match the configured path.

Rejected: new budget SQL for the detail page (a second source of truth);
calling the two list readers (separate transactions, R3).

### R2a — The list's `Option` pair cannot carry the tile's states

Evidence (coordinator plan review, confirmed in code):

- `allocated_totals` returns `(None, Some(_))` both when every allowance is
  blank and when the allowance sum overflows but usage fits. Its own test
  asserts `[(Some(i64::MAX), 0), (Some(1), 0), (Some(0), 0)] → (None, Some(0))`,
  the same pair as `[(None, 120)]`.
- `progress_for_viewer` writes withheld fees as `(None, None)`. Consumption
  overflow alongside an allowance overflow gives the same pair.
- A visible all-blank fee budget is `(None, Some(0))`, which is not withheld.
- Allowances and tracked minutes are non-negative (`0010_invariant_checks.sql`,
  `0030_project_creation.sql`), so remaining cannot overflow from stored data.
  The core function still uses `checked_sub`.

Decision: add a small exact projection rather than inferring states from the
pair. Do not change `ProjectBudgetOverview`, `progress_for_viewer` or
`allocated_totals`; the list API stays as is.

- A pure `horae_core::budget::headline(&[(Option<i64>, i64)]) -> BudgetHeadline`
  makes **one** call to `allocated_totals` plus an explicit has-allocation
  fact (`any(allowance.is_some())`). It returns `Unallocated` without
  allocation, `Exact { budget, spent, remaining }` when both totals and
  `checked_sub` succeed, and `Unavailable` otherwise. There is no second
  summation.
- When a fee budget's money is not visible, the server returns `Unavailable`
  without calling `headline`. Withheld and overflow render identically and
  expose no figures, so the wire does not separate them (coordinator review).
  Authorization tests read one project as authorized and unauthorized viewers.

Rejected: changing `allocated_totals`' return type or the list DTO for this
tile; a boolean flag next to the pair (still ambiguous for consumption
overflow); a nested `Withheld`/`Computed` wrapper (wire detail no viewer can
use); a parallel summation.

## R6 — Budgeted presentation (official Harvest screenshots)

Sources: the current [Project analysis guide](https://support.getharvest.com/hc/en-us/articles/360048181412-Project-analysis),
read on 2026-10-22, and its screenshots
[24294245057933](https://support.getharvest.com/hc/article_attachments/24294245057933)
(monthly card) and
[24283243575565](https://support.getharvest.com/hc/article_attachments/24283243575565)
(overrun card with breakdown), both viewed for this research.

Evidence:

- **Monthly card:** a header band "February 2024"; "Budget remaining (29%)";
  `$2,300.00`; a "?" control; a row "Monthly budget $8,000.00"; a progress bar
  filled to roughly 71 %. 2,300/8,000 = 28.75 %, shown as 29 %, so the
  percentage is remaining over budget and the bar is spent share.
- **Overrun card:** "Budget remaining (-12%)" and `-$5,764.50` in red. The
  open "?" breakdown lists Total budget $50,000.00 and Budget spent $55,764.50,
  split into Time $55,675.50 and Expenses $89.00. −5,764.50/50,000 = −11.53 %,
  shown as −12 %.
- **Other tiles:** in the monthly screenshot, the Internal costs and
  Uninvoiced tiles carry "All time" bands; Total hours carries "February
  2024".
- **Approved 004 (#208 at `48a41560`, US1.4):** "consumed allowance and signed
  remaining allowance. An overrun remains visible when the bar is full."
- **Older `../spec.md`:** the bar is spent ÷ budget, clamped to 100 %.
- **Existing core and list:** `horae_core::budget::used_percent(consumed, budget)` rounds half up, clamps to 0..=100 and returns `None` for budgets
  ≤ 0. The list renders it in `<progress class="proj-bar">`. Its "(NN%)" label
  is `100 - used`, which is clamped and cannot show −12 %.

Inferences and decisions:

- Reuse `used_percent` for the bar. Add a signed remaining percentage in core
  (`remaining * 100 / budget`, rounded to nearest, `None` when budget ≤ 0),
  because no existing function produces a negative value.
- The screenshots fit round-to-nearest but include no tie; O1 asks for the
  tie rule. Excluded by the evidence: truncation (28/−11), floor (28) and
  ceiling (−11).
- Lifetime row label "Total budget" comes from the breakdown popover
  (inference for the card). A human month caption comes from the monthly band
  (evidence); omitting a band on lifetime cards follows the handoff
  (inference).
- Not adopted: the "?" breakdown (O2; its expense split needs feature 016) and
  Harvest's grey band styling (the Horae design governs).

## R3 — One consistent, authorized read

Decision: add `ReadAccess::begin_snapshot` beside `begin`. Same authority
loader; REPEATABLE READ, READ WRITE (authority locks need write mode); the
existing `configure_transaction_limits`; up to three fresh transactions on
SQLSTATE 40001, then the existing `conflict("Access changed while reading. Please try again.")`. The detail view uses it for every policy.

Evidence (code): `ReadAccess::begin` is READ COMMITTED with
`organizations … FOR SHARE` and `users … FOR SHARE`, so each later statement
sees a new snapshot. `manager` in `crates/horae/src/server_fns/snapshot.rs`
already uses REPEATABLE READ with a
40001 restart, but authorizes only legacy managers and is used by fee
balances; it is not reused as an authorization path. `progress_for_viewer` and
`fetch_project_spend` each begin and commit their own `ReadAccess`.

Inference: in REPEATABLE READ the snapshot is taken by the first query (the
limits statement), before the locks. A writer that commits an organization or
actor change after that raises 40001 on our `FOR SHARE`, so a retry sees the
new authority; reader-first locks make writers wait. Plain reads after the
locks cannot raise 40001. So retrying only the begin is sufficient **if** the
whole detail read graph after it has no locking read or write. Checked today:
`project_details_in_transaction`, the context query, `assignment_rows`, the
team/task queries, `fetch_progress`, the spend query and
`load_person_permissions` contain no `FOR SHARE`/`FOR UPDATE` and no write.
T016 re-proves this in code and with a test. A retry loop inside
`begin_snapshot` is the whole mechanism; there is no generic retry
abstraction.

## R4 — Display

Decision: make `format_hours2` in `crates/core/src/duration.rs` integer-only
and use it for hour values, with a visible `h` unit and "hours" in the
accessible name (proposal P2); use `money::format_cents` for fees. The tile
appends the unit; `format_hours2` output stays unit-free for its report and
export callers.

Evidence: `format_hours2` is `format!("{:.2}", minutes as f64 / 60.0)`, used by
reports, exports and report pages. `format_decimal` (trimmed, clamps negatives)
is a separate contract and stays unchanged.

Inference: `minutes * 100 / 60` never ends in exactly .5 for an integer
(10·m ≠ 6k+3), so integer half-up rounding has no tie cases. Equality with
today's float output is claimed **only for the tested ordinary range**
(−100 000..=100 000). An exactly converted integer still does not make division
by 60 exact. Counterexample (coordinator, 2026-10-22): `m = 9 007 199 254 740 991`
(< 2^53) gives integer `150119987579016.52`, but float division gives
`150119987579016.53`. The huge-value float output is wrong and is not
preserved. Proposed body: `hundredths = (|m| as u128 * 100 + 30) / 60`, sign
prefix, `{}.{:02}`. Regressions: equality over the ordinary range; exact
strings for this counterexample and for `i64::MIN`/`i64::MAX`.

## R5 — Layout

Decision: a container with the `grid gap-4` utilities plus one structural
`pd-summary` rule for `repeat(auto-fill, minmax(min(15rem, 100%), 1fr))`
columns, holding one card
(`card rounded-xl p-5`): label `text-sm text-secondary`; value
`font-mono text-3xl font-semibold text-strong`; rows `text-sm` label/value;
no-budget copy `text-sm text-subtle`; link `text-sm font-semibold`.

Evidence: handoff tiles use `repeat(auto-fit, minmax(240px,1fr))`, 16px gap,
16px radius, 20px padding, 30px mono value; tokens `--radius-xl` 16px,
`--space-5` 20px, `--font-size-3xl` 1.875rem, `--color-text-subtle` #8F8676.
No grid `minmax` utility exists.

Inference: with all five tiles, `auto-fill` and `auto-fit` render alike; with
one tile, `auto-fill` keeps tile width instead of a full-width card. `min(…, 100%)` avoids overflow at 320px with 200% text, which a 240px floor would not.
