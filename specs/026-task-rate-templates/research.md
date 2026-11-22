# Research: Task rate templates

Each section gives the decision, why, and what else was considered. R1 to R3 follow the recommended
answers to the spec's open questions; if the maintainer answers otherwise, the tasks named there
change.

## R0. How Horae stores money today

**Finding**: every amount is an integer in hundredths of the major unit, whatever the currency.
`parse_cents` and `format_cents` always use two decimals, and `format_major_units` documents it:
"Horae stores every currency in hundredths, so a zero-decimal currency keeps its two decimals when
the stored amount has hundredths". The Harvest importer converts Harvest's decimals with the same
two-decimal rule. JPY 121 is stored as 12100; BHD 1.250 as 125.

**Consequence**: a template with a fixed scale of two decimals has the same integer as the rate it
becomes in any project currency. Applying a template copies the number. The migration rescales
nothing, including tasks whose stored currency is JPY or KWD (the handoff of PR 1 left that open).
The constitution's "minor units" means these hundredths in practice; the amendment draft says so in
one clause instead of redefining the unit.

**Alternatives**: storing ISO minor units per currency (JPY in units, KWD in thousandths) would make
the template convert by a power of ten, and would be a migration of every money column. Out of
scope; nothing in this feature needs it.

## R1. Rounding amounts to the currency's precision (FR-008, FR-009, FR-011)

**Decision**: one rule in `horae-core`, used by every amount computed from a rate and a duration:

```text
step   = 100 if the currency has 0 decimals, else 1        (in hundredths)
amount = floor((rate × minutes + 30 × step) ÷ (60 × step)) × step
```

That is one half-up rounding from the exact value `rate × minutes ÷ 60`, straight to the currency's
precision. Rounding to hundredths first and then to units would turn 120.496 into 120.50 and then
121; the single step gives 120. Three-decimal currencies use step 1 (hundredths), the finest unit
Horae stores, so their amounts stay exact.

Examples: 80.50 × 90 min in JPY = (724500 + 3000) ÷ 6000 = 121 → 12100 (JPY 121). 5.00 × 30 min in
JPY = (15000 + 3000) ÷ 6000 = 3 → 300 (JPY 3). The same in EUR: 12075 and 250, as today, because for
step 1 the formula is today's `line_amount_cents`.

The decimal table already exists as the private `currency_decimals` in `horae_core::money`; it
becomes public and the only list. The SQL twin `line_amount_cents` gains a currency argument and
calls a SQL `currency_amount_step(text)` that lists the same zero-decimal codes. A test runs both
over every code in the Rust table and over amounts near every half, so the two lists cannot drift.

**Scope (recommended answer to FR-011)**: every per-entry amount: invoice previews and lines
(`ops/invoices/entries.rs`), reports (`ops/reports.rs`), project figures (`ops/projects/reads.rs`)
and budgets (`ops/projects/budgets.rs`). Each already has the currency in the same row. Stored
invoice line amounts (`frozen_amount_cents`, `line.amount_cents`) win over the computed value, so
existing invoices do not change. If the maintainer limits the rule to invoices, T029 to T031 drop
and reports may differ from invoices by less than one unit per entry in zero-decimal currencies.

**Totals**: unchanged code. Subtotals and totals already add the stored line amounts
(`invoice_amounts`, `allocate_invoice_discount`), so FR-010 holds by construction; a property test
proves it for all three precisions.

**Alternatives**: Harvest's subtotal (round the unrounded sum) was rejected by the maintainer:
invoices must add up (spec 001 FR-023). Rounding only at display was rejected: stored amounts would
carry fractions the currency does not have, and the PDF would not add up.

## R2. Zero templates (FR-006)

**Decision (recommended)**: 0 is no rate. The editor stores no rate for 0, the migration clears the
stored zeros that PR 1 kept (entered in the editor, with a currency), and a CHECK makes the column
positive or null.

**Rationale**: Harvest's API sends 0 for a task without a default rate, PR 1 already treats Harvest's
0 as none, and a template of zero applied to a project would set a zero rate the manager did not
choose. A project that bills a task at zero can still set 0.00 as the project's own task rate.

**Alternative**: keep 0.00 as a valid template that applies a zero rate. Then T006 keeps the zeros
and the CHECK stays `>= 0`, T021 and T023 accept 0 as a template, and T003, T007, T020 and T026
change their zero cases.

## R3. Rates stored in a currency other than the workspace's (FR-012)

**Decision (recommended)**: keep the number. A task stored as 80 USD in an EUR workspace becomes the
template 80.00, which an EUR project later applies as EUR 80.00.

**Rationale**: that is what the template means, and what Harvest does with the same number. Only
future additions use the template; existing project rates are copies and stay as they are. The
upgrade notes in `quickstart.md` give a query that lists the affected tasks before upgrading, so an
administrator can review them.

**Alternative**: clear such rates. Safer against the 011 incident, but it throws away rates the CSV
importer read from real rows. Then T006 adds one `UPDATE`, T007 asserts it, and the notes say which tasks lost their
rate.

## R4. Names: keep `default_rate_cents`

**Decision**: the column `tasks.default_rate_cents`, the wire fields `default_rate_cents` and
`amount_cents`, and the plugin event field keep their names. `default_rate_currency` and every
`currency` beside a template go. A column comment and the type docs say that the value is a template
in hundredths with no currency.

**Rationale**: import preview checkpoints restore tasks with
`jsonb_populate_recordset(NULL::tasks, …)`, which matches keys to column names; a rename would make
old checkpoints restore tasks without their rate. The plugin read role can read `tasks`, and plugins
receive `default_rate_cents` in `TaskCreated`; a rename would break both for no gain. Dropping the
currency column needs nothing for old checkpoints: `jsonb_populate_recordset` ignores keys that are
no longer columns (FR-018).

**Alternatives**: renaming to `default_rate_hundredths` reads better and was rejected for the two
compatibility costs above.

## R5. A core type for templates

**Decision**: `horae_core::money::RateTemplate`, a newtype over a positive `i64` of hundredths, with
`parse` (at most two decimals, through `parse_cents`), `hundredths`, `format` ("80.50") and
`apply(currency) -> i64`, which returns the project-task rate in the currency's hundredths (the same
number, per R0). The server and the UI use it where a template is read, written or applied, so the
rule "a template is not money until applied" has one home (Principle II).

**Alternatives**: plain `i64` everywhere, as today. Rejected because the template and money would
again be the same type, which is how the 011 bug happened.

## R6. Where templates are applied

The places that copy a task's default rate into `project_tasks.rate_cents`:

| Place | Today | After |
| --- | --- | --- |
| `ops/projects/creation/finalize.rs` (New Project) | copies only when the task's currency equals the project's, else refuses | copies `RateTemplate::apply(project currency)` |
| `ops/projects/editor/associations.rs` (project editor) | same, "Enter an explicit task rate in the project currency" | same as above |
| `pages/new_project/tasks.rs` (placeholder) | shows the default only when currencies match | shows "EUR 80.50 / h" from the template in the draft's currency |
| `ops/projects/creation/options.rs`, `editor/catalog.rs`, `editor/projection.rs` | read and redact the currency with the rate | read and redact the template only |
| Harvest importer (`resolve.rs`, `ensure_project_task`) | uses the CSV row's project-task rate, real money | unchanged |
| `seed.rs` | inserts project tasks with explicit rates | stops writing a task currency |

No other code path inserts into `project_tasks` (checked with `rg "INTO project_tasks"`). Reports and
invoices never read `tasks.default_rate_cents`; billing resolves the project's own task rate
(`resolve_project_rate(…, pt.rate_cents, …)`), so FR-013 holds without changes there.

## R7. Task editor and catalog wording

**Decision**: the editor's field is "Default hourly rate" without a currency, parsed with
`RateTemplate::parse`. `authorize_rate` in `ops/projects/task_writes.rs` keeps its permission check
(`BillableRateWriteAll`) and drops the workspace currency check and the "Task rate currency changed.
Reload" refusal. The Tasks screen's `rate_label` writes "80.50 / h" and a note that the rate takes
each project's currency; PR 1's `tasks-rate-currency-not-set` message is removed and
`tasks-rate-none` stays. All text goes in `crates/i18n/locales/en/horae.ftl`.

## R8. Interfaces

- Harvest-compatible API: `task_row_to_harvest` builds `Amount::new(cents, currency or "")`
  (`crates/harvest/src/v2/compat.rs`), written through `format_major_units`. Today a task stored in
  JPY is written with the JPY rule and an imported one with two decimals. After the change every
  task takes the no-currency path, so the value is the template at two decimals. Shape unchanged.
  `task_reads.rs` drops the column.
- `/api/v1` has no task rates (`rg rate crates/api/openapi/v1.json` lists project rates and budgets
  only). The snapshot test proves it unchanged.
- `TaskRateEdit::Set { amount_cents, currency }` becomes `Set { amount_cents }`, and
  `TaskCatalog::rate_currency` (the workspace currency the editor showed) goes. The enum already has
  `deny_unknown_fields`, so a request that still sends `currency` is refused and an old client cannot
  believe its currency was stored. The transport tests swap their cases: `{"action":"set", "amount_cents":8000}` is now valid, and the same with `"currency":"EUR"` is refused.

## R9. Migration

**Decision**: one migration at the next free number (0010 when this was written; PR 1 owns 0009):

1. Recommended R2: `UPDATE tasks SET default_rate_cents = NULL WHERE default_rate_cents = 0`.
1. Drop `tasks.default_rate_currency` (its CHECK goes with it).
1. Replace `tasks_default_rate_cents_nonneg` with `default_rate_cents IS NULL OR default_rate_cents > 0`.
1. Comment the column as a template.
1. Create `currency_amount_step(text)` and the three-argument `line_amount_cents`, drop the
   two-argument one, and update the plugin role's function allowlist
   (`crates/server/src/plugin/database.rs`) in the same PR.

The rows: a rate with the workspace currency, another currency, or none keeps its number (R0, R3).
Project tasks, clients, invoices and time entries are not touched.

## R10. Spec 011

`research.md` "Catalog denomination", `data-model.md` (the migration 0038 paragraph),
`quickstart.md` and `tasks.md` get a one-line note at the top of each affected passage: superseded by
spec 026. What stays: Horae never guesses a currency; a project-task rate is a copy that later
catalog edits do not reprice. What changes: a task's default rate has no currency, so there is
nothing to match, and every project currency can use it.
