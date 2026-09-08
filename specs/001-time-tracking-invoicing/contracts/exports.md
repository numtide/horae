# Export execution and resource limits

The Axum handlers in `crates/horae/src/reports.rs` serve downloadable files.
Report and invoice exports require an active manager/admin session. Project
exports require an active session. Every dataset is scoped to that user's
organization; these access rules are unchanged.

## Buffered formats: XLSX and invoice PDF

| Resource | Limit |
|---|---|
| Concurrent exports per process | 2, shared by all three XLSX routes and invoice PDF |
| Queued exports | None; excess requests receive `503 Service Unavailable` |
| XLSX dataset | 10,000 rows and 8 MiB of source text |
| Invoice PDF dataset | 1,000 lines and 1 MiB of source text, including invoice/client/provider data |
| Individual text field | 32,767 UTF-8 bytes |
| Output file | 32 MiB |
| Each export SQL statement | 5 seconds |
| Idle read transaction | 10 seconds |
| Render wait | 30 seconds, including execution and time queued in Tokio's blocking pool |

Limits are inclusive. Oversized datasets/files return `413 Payload Too Large`;
they are not silently truncated. Reduce the date range or select narrower
filters for a timesheet export. An invoice exceeding the PDF limit can still
use XLSX if it fits the XLSX limits. A database statement timeout or exceeded
render wait returns `504 Gateway Timeout`. Other query/render failures return
`500`; detailed errors are logged on the server.

Admission is reserved **before loading** a dataset. PostgreSQL checks row and
text sizes before sending full text to the application. The check and payload
queries run in one read-only, repeatable-read transaction, so edits cannot grow
the payload between those queries. Transactions finish before rendering starts.
Settings are transaction-local and do not change the application's normal pool
configuration. The row-count probe reads at most the limit plus one matching
row; it does not require counting the entire organization.

Project scope (`active`, `budgeted`, or `archived`) is applied in SQL for both
CSV and XLSX. Unknown scopes retain the existing active-project fallback.
Ordering ends with the project UUID. Timesheet client/project/user/date filters
and effective billing-minute semantics are shared with the detailed report.

XLSX construction, ZIP serialization, Typst compilation, and PDF serialization
run on blocking workers, not the async request threads. The admission permit
stays with the worker and then the response body. Dropping a request or timing
out does **not** release capacity while its renderer is still running. Tokio
cannot forcibly abort a blocking closure once it starts; only queued work can
be aborted. See the [Tokio execution contract](https://docs.rs/tokio/1.53.1/tokio/task/fn.spawn_blocking.html).

XLSX's output writer rejects writes/seeks beyond the output limit before its
destination grows. Typst returns a completed byte vector, so the PDF output
limit is checked afterward. PDF rendering uses the fixed embedded template and
embedded fonts only, without enumerating host font directories.

These are input/output and concurrency bounds, **not a hard per-process RSS or
CPU sandbox**. The libraries allocate working memory beyond the source text;
the native renderer may outlive the request's wait deadline. External process
isolation would be needed for enforceable memory/CPU termination. The current
template treats customer/provider text as data, not executable Typst code.

## Progressive CSV downloads

Timesheet, project, and invoice CSV routes stream the existing SQL queries row
by row. The CSV header and first row are sent before reading the remaining
rows; an empty result sends just its header (plus the stored total for an empty
invoice). Column order, quoting, filenames, effective minutes, and exact integer
money formatting are unchanged. Invoice metadata, lines, and total come from
one read-only, repeatable-read transaction.

CSV has four admission slots per process, separate from XLSX/PDF. Admission is
reserved before acquiring a database connection; excess requests receive `503`.
One bounded channel holds one output chunk. After the initial chunk, the writer
flushes around 64 KiB, always at a record boundary. A slow reader stops the
producer from reading more rows once the queue and in-progress chunk fill.
The slot is retained by both producer and response until each is finished or
dropped, including a completed producer whose body has not yet been consumed.

There is no CSV row-count or total-file-size cap. Memory is proportional to the
largest row and a small number of chunks, **not a hard 64 KiB limit**: a single
large text field, CSV quoting, SQLx/network buffering, and HTTP transport buffers
can exceed that threshold. PostgreSQL may also sort/materialize query results
on its side; streaming does not bound database working memory.

Each statement has a 5-second timeout and an idle transaction has a 10-second
timeout. The producer has a 60-second wall-clock wait limit, including pool
acquisition and waits for a slow consumer. This is not a hard CPU deadline for
encoding an exceptionally large row. Dropping the handler or body aborts the
async producer. Its connection is closed instead of being returned with an
unfinished query; SQLx retains the pool slot during graceful closure. See
[SQLx connection cleanup](https://docs.rs/sqlx/0.8.6/sqlx/pool/struct.PoolConnection.html#method.close_on_drop).

Invalid dates receive `400`; authorization failures retain `401`/`403`.
Producer failures before the first chunk return an HTTP error (`404` for a
missing or foreign invoice, `504` for a deadline, otherwise `500`). Once headers have been
sent, a query error, panic, or timeout interrupts the response body; it does
**not** turn into normal end-of-file or append an error record to the CSV.
Clients must treat an interrupted download as incomplete and retry it. A proxy
that buffers responses can delay when the client observes the first bytes.

## Other remaining limits

The SPA's detailed-report and invoice-detail server functions retain their
existing uncapped collection contract. CSV import streaming is separate from
these export routes.

Invoice PDFs read current client/provider details in their export snapshot;
this is not a historical branding snapshot captured when the invoice was sent.
Invoice line quantities/rates/amounts remain the stored invoice snapshot.

## Verification and measurement

Tests exercise the production loaders, dataset/field limits, filtered selection,
tenant isolation, repeatable-read consistency, read-only enforcement and SQL
timeouts. Worker tests cover thread isolation, overload, cancellation, timeout,
panic/error cleanup, response-body lifetime, and output writer boundaries.
Workbook tests read the generated ZIP/XML, including frozen zero rounding;
PDF tests compare bytes from repeated renders.

CSV tests cover first-chunk delivery before source completion, backpressure,
overload, startup/body cancellation, database connection closure, actual SQL
errors, producer panic/timeout, quoting and exact cents, tenant/filter selection,
concurrent invoice edits, and downloads exceeding the XLSX row-count limit.

For a manual release measurement against an auxiliary PostgreSQL database with
the current migrations, run inside the Nix dev shell:

```sh
cargo test -p horae --features server --release measure_bounded_export_limits -- --ignored --nocapture
```

The test uses its own disposable database. It renders 10,000 timesheet rows and
1,000 invoice lines, prints loading/total durations, output sizes, and Linux
process high-water RSS. Set `HORAE_EXPORT_PROBE_DIR` to an existing temporary
directory to retain the XLSX/PDF artifacts. This is a sample dataset measurement,
not a worst-case capacity guarantee. No new query indexes are introduced.

For a progressive 100,000-row CSV measurement, without collecting the output:

```sh
cargo test -p horae --features server --release measure_streaming_csv_export -- --ignored --nocapture
```

This also uses a disposable database and reports first-chunk latency, total
time, bytes, chunk count, and process high-water RSS (including fixture setup).
