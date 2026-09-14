# Durable import scale validation

These are local release-mode measurements, not production capacity guarantees.
T007 remains open until the durable/inline comparison and large-catalog cases
have completed. Run on an isolated PostgreSQL instance with a role that can
create databases, inside the Nix development shell.

## API

Run each scenario separately so its process high-water RSS starts fresh. Use
the full test name with `--exact`:

```sh
cargo test -p horae --features server --bin horae --release \
  importers::harvest::sync_tests::scale::api_100k_dry_run \
  -- --exact --ignored --nocapture --test-threads=1
```

Repeat for `api_durable_100k_dry_run`, `api_100k_commit_and_reimport` and
`api_durable_100k_commit_and_reimport` in the same module. The fixture produces
100,000 entries over 365 dates with unique notes, one client/project/task, one
matched user, and 100 entries with an unmatched user. It generates one page at
a time, retains only sixteen request headers, and preserves production pacing.
Each import must make exactly 1,004 HTTP requests and reconcile all outcomes.
Commit/reimport also check 99,900 stored entries, 5,994,000 integer minutes,
provenance counts and zero duplicate creations. Preview must persist no imported
parents, entries or provenance. Record errors prevent watermark advancement.

The durable scenarios use the claim/heartbeat wrapper. A test-only constraint
rejects final success so the EOF checkpoint remains available for measurement.
Removing that constraint and retrying must produce the same report without any
additional HTTP requests. Elapsed time includes this controlled finalization
failure and recovery; it is not a normal-success-only measurement.

Checkpoint output reports PostgreSQL's `pg_column_size(checkpoint)` and the
octet length of its JSON text. The former measures the datum's stored size,
including compression when applicable, not total table/index/WAL space. This
is an EOF observation, not a continuously sampled maximum checkpoint size.
RSS/HWM comes from `/proc/self/status` and includes the test process and HTTP
fixture, but excludes PostgreSQL and the kernel page cache. Reimport within a
test inherits the first import's high-water RSS.

### Observed API results

Local x86_64 release profile with the checked-in optimization/LTO settings,
2026-09-14, benchmark source at `66a1d0c`. No other local builds or import
benchmarks ran during this sample; lightweight read-only diagnostics did run.

| Scenario | Elapsed seconds | Process HWM, KiB | Result |
|---|---:|---:|---|
| Inline preview, 100,000 entries | 533.914 | 19,904 | All assertions passed |
| Durable preview, 100,000 entries plus EOF recovery | 1,451.289 | 53,636 | All assertions passed |
| Inline first commit, 100,000 entries | 543.282 | 19,908 | All assertions passed |
| Inline reimport in the same process | 160.710 | 19,996 | All assertions passed |

The durable preview's EOF checkpoint measured 2,856,318 stored bytes and
5,110,782 JSON-text bytes. Its first attempt reached EOF in 1,451.182 seconds;
retry added no HTTP requests and preserved the complete report. The approximately
2.72x elapsed-time increase is material: functional recovery passed, but this is
not evidence of throughput parity. Repeated restoration of growing preview
associations and checkpoint serialization need profiling before closing T007.
The failure-injection fixture now uses an unvalidated test-only constraint: it still rejects subsequent
completion transitions without rejecting jobs completed by an earlier scenario.
A small multi-run CSV regression exposed that fixture issue before CSV scale runs;
the API helper now also exercises preview, commit and reimport in its regular test.

The durable commit/reimport scenario completed at `fd36a53`, after the fixture
correction and master integration. Both imports passed their complete assertions,
including 99,900 stored entries, exact minutes/provenance and no duplicate creates.
No other local builds or import measurements ran during this sample; a read-only
job-progress query did run.

| Scenario | Elapsed seconds | Process HWM, KiB | EOF stored / JSON-text bytes |
|---|---:|---:|---:|
| Durable first commit plus EOF recovery | 242.264 | 21,064 | 1,930 / 15,033 |
| Durable reimport plus EOF recovery, same process | 160.742 | 21,088 | 1,932 / 15,033 |

The first attempts reached EOF at 242.226 and 160.713 seconds respectively.
Recovery preserved each report without additional HTTP requests. This sample
shows no commit/reimport slowdown against the earlier inline measurements;
it does not establish the same for previews or CSV, or guarantee production
throughput. The two source revisions and single-sample methodology are not a
controlled repeated-run speedup claim.

## CSV

The CSV comparison uses the application's production pool constructor, including
its custom-plan policy. Both paths receive the same lazily generated source.
The inline path consumes its streaming body; the durable path includes bounded
upload buffering, database upload persistence and the worker's database read.
It does not measure a browser or a TCP upload.

Run each full test name in `importers::harvest::engine_tests::csv_streaming`
with the release command above:

- `measure_csv_streaming_100k`
- `measure_csv_durable_100k`
- `measure_csv_streaming_large_catalog_100k`
- `measure_csv_durable_large_catalog_100k`

Each test runs preview, first commit and reimport sequentially. The basic source
has one client/project/task; the large-catalog source has 5,000 of each, with
matching project/task associations. Both have 100,000 entries, 100 invalid dates,
unique notes and one existing user. Assertions cover counts, exact minutes,
absence of CSV provenance, preview rollback and duplicate-free reimport.

Durable CSV also retains its last complete batch using the finalization-failure
constraint, measures that checkpoint and retries to the identical final report.
The input length is a multiple of the 500-record checkpoint interval. Elapsed
time includes this recovery, and process HWM is cumulative across all three
phases within each test.

### Observed CSV results

The one-parent-set inline scenario completed at `fd36a53` with all count,
error, minute, provenance, rollback and reimport assertions passing. Each phase
processed 100,000 records, including 100 invalid dates; committed state contains
99,900 entries and 5,994,000 minutes. No other local compilation or import
measurement ran during this scenario. Lightweight read-only diagnostics included
one representative `EXPLAIN (ANALYZE, BUFFERS)` query during reimport; it used
the organization/date index and filtered 273 candidate rows in about 1 ms.
That single query does not attribute the complete scenario's elapsed time.

| Scenario | Elapsed seconds | Process HWM, KiB |
|---|---:|---:|
| Inline CSV preview, one parent set | 140.992 | 34,996 |
| Inline CSV first commit, after preview | 650.712 | 41,508 |
| Inline CSV reimport, same process/database | 764.014 | 52,976 |

These phases share a database, so earlier rollback/commit activity and PostgreSQL
statistics can influence later query planning. They are not isolated fresh-database
samples. Durable CSV and both large-catalog comparisons remain pending.
