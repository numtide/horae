# Currency history and precision evidence

Checked 2026-10-01 using the authorized disposable client/project and a new
ordinary expense. The previous source-deletion fixture was not recreated under
its old identity. No existing business record or workspace currency was changed.

## Historical labels and inheritance

The new expense was saved as USD 1.24 (`1.235` input, HTTP 201) while its project
overrode its EUR client. Project/client currency changes were then saved through
their guarded browser editors. The same existing expense was reopened after
each change, without editing it between these changes:

| Change | Existing expense row |
| --- | --- |
| Project USD → JPY | ¥1; weekly total also ¥1 |
| Project JPY → BHD, after a separate JPY save of -1.24 | -د.ب1.24 |
| Project BHD → inherit client EUR, after restoring amount 1.24 | €1.24 |
| Client EUR → GBP while project inherits | £1.24 |
| Restore project USD override and client EUR | $1.24 |

Changing the project/client currency affects historical expense labels and
grouping; it is not a creation-time currency snapshot. No exchange-rate
conversion was observed. The JPY-to-BHD transition exposes the fractional
numeric value hidden by the JPY list format. Project and client were restored
to USD override / EUR after the tests.

## Server amount versus list format

The observed expense form endpoint accepted these updates to the same fixture.
These are server acceptance cases, not proof that the browser form allows every
input. Its session token remained inside the page.

| Project currency | Submitted amount | HTTP | Response amount |
| --- | --- | --- | --- |
| JPY | 1.235 | 200 | 1.24 |
| JPY | 1.005 | 200 | 1.01 |
| JPY | -1.235 | 200 | -1.24 |
| BHD | 1.235 | 200 | 1.24 |
| BHD | 1.005 | 200 | 1.01 |
| BHD | -1.235 | 200 | -1.24 |
| BHD | 1.24 | 200 | 1.24 |

The numeric response uses two-decimal amounts in these cases, as in the prior
USD tests; JPY list display uses zero decimal places and BHD displayed two.
Do not infer stored precision from a symbol/format or truncate JPY fractions
because they are hidden in that list. Conversely, do not infer three-decimal
expense amount acceptance from a currency's usual denomination.

The technical plan must reconcile this exact fractional representation and
historical currency attribution with Horae's integer-money constitution. No
floating-point implementation or implicit lossy conversion is authorized by
these reference observations. Export precision, mixed-currency summation and
invoiced historical currency changes remain separate cases.

## Evidence and limitations

Private scratch evidence includes `expense-currency-results-20261001.md`,
`expense-currency-jpy-history-20261001.md`,
`expense-currency-inherited-gbp-history-20261001.md`, the project/client save
snapshots and `expense-currency-restored-readback-20261001.md`.
Exact fixture IDs and cleanup state are in the local ledger.

The raw endpoint response establishes accepted value and current project
currency, not Harvest's database column types. The JPY initial historical read
did not expose its fractional amount until a later save; do not claim a direct
pre-save numeric read where only a formatted row was inspected. No non-owner,
foreign-exchange, receipt-currency or cross-currency invoice test is implied.
