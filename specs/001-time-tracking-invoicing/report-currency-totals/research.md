# Research: Currency-separated totals (R02)

## Evidence

| Source | Who opened it | What it establishes |
|--------|---------------|---------------------|
| UI parity runtime comparison of the legacy Reports page | Coordinator, 2026-10-21 | The legacy summary mixes EUR and USD rows, and the total billable amount is an unexplained em dash. |
| Native Chrome smoke test of the merged artifact on :8094, local seed, October 2026 | Coordinator, 2026-10-22, read-only | Loaded DOM: Acme billable `EUR 1,130.00`, TechStart billable `USD 1,365.00`, Billable total `—`, Cost total `EUR 0.00` (correct across both billing currencies). This is the before-state only, not acceptance of this change. |
| [Does Harvest support multiple currencies?](https://support.getharvest.com/hc/en-us/articles/360055384512-Does-Harvest-support-multiple-currencies) | Coordinator, via web | The Time report's top summaries break billable and uninvoiced amounts out per currency, with no conversion. Cost rates use the account's default currency. |
| [Time report](https://support.getharvest.com/hc/en-us/articles/360048181692-Time-report) | Coordinator, via web | Team rows that mix currencies are handled separately from the summary totals. That row-level withholding is out of scope here. |
| The same two URLs | This author, direct HTTP fetch, 2026-10-22 | Both returned HTTP 403, so they were not independently rechecked. The rows above rely on the coordinator's reading. |
| `design/project/app/07_Reports.dc.html` | This author | Shows only the empty report builder. It does not describe the existing Time table's total row, so it is not used as a design for it. |

## Decisions

### Sum in `i128` per currency, then check the result into `i64`

- **Chosen** because overflow is judged on the final sum, so the outcome cannot
  depend on row order. An `i128` cannot overflow from summing fewer than 2^64
  `i64` values.
- **Rejected:** `money::add`. It takes `[u8; 3]` currency codes and fails on
  the first running overflow, which would make the result depend on row order.
- **Rejected:** a generic money or totals framework. This needs one function.

### Withheld amounts are decided inside the helper

Any withheld amount returns `Withheld` before any total is reported. An earlier
row that already pushed its currency out of range therefore cannot pre-empt the
withheld check. This keeps the all-or-nothing rule in core, as the constitution
requires (Principle II).

### Currency identity is the exact stored code

The schema enforces only length three (migration `0010`: organization, client,
project, invoice). Only later columns such as `0031` and `0038` add an
uppercase pattern. Codes are therefore grouped and ordered by their exact text:

- no case folding, trimming or merging;
- ordered by byte order;
- no new validation.

Labels are a separate concern. In-range lines keep the existing
`money::format_cents`, which trims the code. A page test feeds the helper two
synthetic inputs, `" EU"` and `"EU "`, to make that split visible: they stay
separate groups, in byte order, while both display as `EU`. These are test
inputs only. Migration `0010` rejects a code like `"EU "` in `char(3)` columns,
because `char_length` ignores trailing blanks. The shared formatter does not
change.

### One line per currency, inside the existing cell

- Lines render as block `div`s inside `td.text-mono.text-right`. They inherit
  right alignment, the mono face and the total-row weight, so no CSS changes.
- With two or more currencies, the one-line cells in the total row centre
  vertically against the taller cells, because `tbody td` sets no
  `vertical-align`. That is acceptable table presentation, not a defect, so this
  increment does not change it.
- `.table-container` scrolls on its own, so the total lines are meant to stay
  inside the table scroller at narrow widths and with enlarged text. The suite
  checks this. It also records any page overflow and its source, separately,
  so that overflow from the existing shell is compared with the base build
  instead of being hidden.
- Enlarged text is tested by setting the root font size to 32px, which is the
  existing 200% text check. A narrow viewport alone approximates reflow; it is
  not text zoom.

### Browser proof boundaries

- **Seed dates:** the demo seed writes entries into the current ISO week
  (`seed.rs`, from `Utc::now()`). The suite reads the first and last seeded
  entry dates from the database and derives the expected totals from the row
  cells, with no hard-coded amounts.
- **Restricted:** proven in the browser with a real session. The approach
  follows `new-project-permissions.cjs`: sign in as the disposable admin, then
  switch that actor's `org_role` to `manager` in the runner's database. Add a
  real `project_member_costs` override for one seeded project and person in the
  period, creating the assignment only if it is missing. Record what the
  fixture touched, and restore the role and remove the fixture rows in
  `finally`. This is a small fixture, not an auth framework, and no real
  accounts are used. Core and page unit tests cover the same rule, and
  server-side withholding is also covered in `server_fns/reports/tests.rs`.
- **Overflow:** a group whose own sum exceeds bigint makes the query fail,
  because the `::bigint` cast raises an error. Building several in-range groups
  that large is disproportionate. It is proven in unit tests.
- **Exports:** they are per-entry (`crates/horae/src/reports.rs`) and carry no
  summary totals. They are unchanged.
- **Scoped reports:** the scoped (policy 1) reader exposes no money
  (`015 contracts/time-reports.md`).
