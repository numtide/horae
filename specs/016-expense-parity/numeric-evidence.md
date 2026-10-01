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
Maximum bounds remain untested.

## Historical rate behavior

The expense was saved with quantity 2 and total USD 0.25 at rate 0.125. Changing
the category rate to 0.5 left its reloaded row at USD 0.25. Opening Edit and
saving without changing any fields then changed the row to USD 1.00. This
confirms resave repricing, not background repricing. Archived-category resave and
invoiced-source correction are separate, still-open cases.

Local evidence includes `expense-first-response-20261001.json`,
`expense-negative-response-20261001.json`,
`expense-category-rate-precision-20261001.json`,
`expense-numeric-results-20261001.md`, and
`expense-before-reprice-20261001.md` / `expense-after-reprice-20261001.md`.

## Remaining limits

Input locale, true upper bounds and exports still need evidence. Subsequent
[currency/history](currency-evidence.md), [category archive](category-evidence.md)
and [draft billing](billing-evidence.md) checks narrow their respective gates;
they do not settle all currencies, permission combinations or issued artifacts.
The inconclusive earlier unsaved experiments are superseded only for the cases
above. Endpoint acceptance is not UI acceptance or proof of non-owner authority.
