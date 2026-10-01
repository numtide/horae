# Persisted expense numeric evidence

Checked 2026-10-01 in authenticated Harvest using authorized disposable fixtures.
This is reference evidence, not a Horae test result. Exact fixture IDs, response
bodies and snapshots stay in the main checkout's private `.scratch/` ledger.
No existing business record was changed.

## Method and currency

A new EUR client and a T&M project with a USD override were saved through the
browser UI. Project-editor read-back retained that pair. A new ordinary expense
entered as `1.235` formatted to `1.24` on blur, saved with HTTP 201, and displayed
USD 1.24. Editing to `-1.235` formatted and persisted as USD -1.24 (HTTP 200).
The expense response's embedded client currency was USD although the actual
client retained EUR: that response field is not proof of the client's stored
currency. Project precedence is verified for this persisted case; subsequent
[currency evidence](currency-evidence.md) adds historical relabeling and JPY/BHD
precision without implying coverage of every supported currency.

Additional cases used the observed web form PUT endpoint from the same browser,
with its session token kept inside the page. These are **server acceptance
tests**, not proof that the UI permits every input. No write API is proposed for
Horae. Every success was checked against the same fixture identity and project;
the final valid value was reopened in the UI.

## Ordinary amounts

| Submitted amount | Response | Saved USD amount |
| --- | --- | --- |
| 0 | 200 | 0 |
| 0.004 | 200 | 0 |
| 0.005 | 200 | 0.01 |
| 1.234 | 200 | 1.23 |
| 1.235 | 200 | 1.24 |
| -1.235 | 200 | -1.24 |
| abc | 422 | Rejected: not a number |
| 99999999.99 | 200 | 99999999.99 |
| 100000000 | 200 | 100000000 |
| 999999999999.99 | 200 | 999999999999.99 |
| 1000000000000.00 | 200 | 1000000000000 |

These establish two-decimal rounding with ties away from zero for the tested
USD cases, not a maximum. Zero has a genuine UI/server difference: entering `0`
in the UI clears the input on blur and required-field validation blocks save,
whereas the form endpoint accepts zero. Do not turn the UI observation into a
domain prohibition on zero, or silently claim UI zero support.

## Unit quantities and rate precision

A category created through the UI retained rate `0.125` (HTTP 201). Its category
list showed the account's EUR symbol; the USD project expense applied the same
numeric rate without conversion. At that rate, the expense endpoint returned:

| Submitted units | Response | Saved units | Saved USD total |
| --- | --- | --- | --- |
| 1 | 200 | 1 | 0.13 |
| 2 | 200 | 2 | 0.25 |
| 1.234 | 200 | 1.23 | 0.15 |
| 1.235 | 200 | 1.24 | 0.16 |
| 1.23456 | 200 | 1.23 | 0.15 |
| -1.235 | 200 | -1.24 | -0.16 |
| 0 | 200 | 0 | 0 |
| 0.04 | 200 | 0.04 | 0.01 |
| 0.12 | 200 | 0.12 | 0.02 |
| abc | 422 | Rejected: not a number | Unchanged |

The `1.235` case distinguishes rounding order: rounding raw quantity times rate
would give 0.15, but rounding quantity to 1.24 first yields 0.155, then 0.16.
The negative counterpart gives -1.24 and -0.16. Preserve fractional quantities;
the API documentation's integer label does not describe this web behavior.

UI category editing `0.123456789` saved `0.123`. Further tests against the observed
category form endpoint established:

| Submitted rate | Response | Saved rate / error |
| --- | --- | --- |
| 0.1234 | 200 | 0.123 |
| 0.12345 | 200 | 0.123 |
| 0.1235 | 200 | 0.124 |
| -0.125 | 422 | Must be greater than zero |
| 0 | 422 | Must be greater than zero |
| abc | 422 | Not a number |

Three-decimal rates must not be truncated to monetary cents. The technical plan
must explicitly reconcile exact fractional rates with the constitution's
integer-minor-unit amounts; floating-point is not acceptable. Tiny positive
rates that round to zero were tested subsequently: `0.0004` is rejected and
`0.0005` persists as `0.001`, documented in [category evidence](category-evidence.md).
The later bounded-magnitude section records rate/quantity limits and the
ordinary-amount boundary conflict.

## Historical rate behavior

The expense was saved with quantity 2 and total USD 0.25 at rate 0.125. Changing
the category rate to 0.5 left its reloaded row at USD 0.25. Opening Edit and
saving without changing any fields then changed the row to USD 1.00. This
confirms resave repricing, not background repricing. Subsequent archived-category
and invoiced-source correction results are recorded separately in category and
billing evidence; they must not be inferred from this active-category case.

Local evidence includes `expense-first-response-20261001.json`,
`expense-negative-response-20261001.json`,
`expense-category-rate-precision-20261001.json`,
`expense-numeric-results-20261001.md`, and
`expense-before-reprice-20261001.md` / `expense-after-reprice-20261001.md`.

## Current-locale input and larger values

The report's delivered configuration explicitly identifies `.` as decimal symbol
and `,` as thousands separator. No account preference was changed. Entering
`1,25` in the existing unit expense and saving through the UI persisted quantity
125 and USD 62.50 at rate 0.5. Its input still contained `1,25` immediately before
submission: this experiment proves saved interpretation, not blur formatting.

Separate ordinary-amount endpoint cases returned:

| Input | HTTP | Exact lexical response amount |
| --- | --- | --- |
| 1,25 | 200 | 125.0 |
| 1,234.56 | 200 | 1234.56 |
| 1.234,56 | 422 | Not a number |
| 1 234.56 | 422 | Not a number |
| 1e3 | 200 | 1000.0 |
| 9999999999999.99 | 200 | 9999999999999.99 |
| 10000000000000 | 200 | 10000000000000.0 |
| 100000000000000 | 200 | 100000000000000.0 |

Raw numeric substrings were retained rather than trusting JavaScript arithmetic
at large scales. The last value is an accepted sample, **not the maximum**.
The fixture was restored to quantity 3 / USD 1.50 and its original category/note.
Private results: `expense-independent-results-20261001.md`; UI snapshot:
`expense-locale-comma-persisted-20261001.md`.

## Bounded magnitude follow-up

Additional fixed-size probes found explicit server validation limits. They used
short decimal strings, not load testing or unbounded fuzzing, and restored the
fixture after each batch.

| Quantity input | Result |
| --- | --- |
| 999999999.99 | 200, same quantity |
| 999999999.994 | 200, quantity 999999999.99 |
| 999999999.995 | 422, maximum 999999999.99 |
| -100000 | 200, same quantity |
| -100000.01 | 422, minimum -100000.0 |

Rate 10000000 is accepted; 10000000.0004 rounds to that value and is accepted;
10000000.0005 fails the maximum-10000000 validation. Together with the tiny-rate
cases this supports positivity and maximum validation after three-decimal
rounding. At rate 0.5 the maximum accepted quantity returned total 500000000.0.

Ordinary-amount limits have a **reference discrepancy**, not a clean proven
range. The upper error names 999999999999999.99, but inputs ending in .50, .90,
.93, .94 or .99 at that magnitude were rejected. Input 999999999999999.00 was
accepted. Input 999999999999998.99 returned 999999999999999.0, losing its cent in
the lexical response. The lower error names -10000000000000.0, yet both that
value and -10000000000000.01 were accepted; -1000000000000000 was rejected.
These results do not establish whether every discrepancy occurs in parsing,
validation, storage or response serialization. No database representation was
inspected. Public documentation searches did not settle these boundaries.

Do not label an error message as a verified inclusive bound or reproduce lost
cents in Horae. The constitution requires exact arithmetic and reconciliation.
The final amount-bound contract needs an explicit resolution of this conflict;
the independent investigation is recorded, not silently converted into an
invented limit. No larger-magnitude probing is needed merely to repeat this
known contradiction. Private cases: `expense-lifecycle-results-20261001.md`.

## Remaining limits

Current-locale UI follow-up used the disposable GBP reconciliation expense.
Entering `1e3` in the quantity field and leaving it changed the visible value to
`13.00`, not `1000.00`; Update expense saved 13 units at rate 0.5 / GBP 6.50.
Thus endpoint exponent acceptance is not evidence that the UI supports scientific
notation. This is an observed normalization distinction, not an exponent parser
to infer from the endpoint or a financial arithmetic rule.
Changing that same disposable expense to an ordinary category and repeating the
UI input likewise normalized `1e3` to `13.00`; saving displayed GBP 13.00.
The reference therefore does not interpret scientific notation in either tested
web field. The ordinary endpoint's 1000 response remains separate evidence.

Other locale configurations and exports still need evidence; ordinary amount
bounds need the conflict resolution above. Subsequent
[currency/history](currency-evidence.md), [category archive](category-evidence.md)
and [draft billing](billing-evidence.md) checks narrow their respective gates;
they do not settle all currencies, permission combinations or issued artifacts.
The inconclusive earlier unsaved experiments are superseded only for the cases
above. Endpoint acceptance is not UI acceptance or proof of non-owner authority.
