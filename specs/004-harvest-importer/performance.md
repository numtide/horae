# Import lookup measurements

## Repeated identities and project-task links

`RunCache` retains successful user resolutions and project-task pairs for one organization/import. Email and full-name keys are distinct and use the shared `harvest_norm` normalization. A match is promoted only after the containing row's savepoint commits. Missing/ambiguous users, database failures and rolled-back links are not cached. The next import starts empty and rechecks identities; within a run, the first successful identity match remains stable. Avoid editing user identities during an import.

Every source row still validates its rate, even if its project-task link is cached. Caching does not update existing link rates or solve historical per-entry monetary fidelity. Each time entry retains its savepoint, provenance and occurrence matching; previews still roll back the outer transaction.

The test `repeated_rows_resolve_each_user_and_project_task_once` captures SQLx statement events around the real import row pipeline. It imports 100 distinct time rows with one user and one client/project/task, then repeats the import. CSV email, CSV name-only and API-shaped rows exercise the same pipeline. Each scenario also runs a preview before committing. Normalized casing/whitespace alternates between rows.

Observed before caching (parent commit `ed6f727`) and after caching:

| Pipeline / operation | Logged statements before | After | User SELECTs before → after | Link INSERTs before → after |
|---|---:|---:|---:|---:|
| CSV creation or preview, email or name | 509 | 311 | 100 → 1 | 100 → 1 |
| CSV re-import, email or name | 405 | 207 | 100 → 1 | 100 → 1 |
| API-shaped creation or preview | 715 | 517 | 100 → 1 | 100 → 1 |
| API-shaped re-import | 505 | 307 | 100 → 1 | 100 → 1 |

All nine runs retain 100 successful `RELEASE SAVEPOINT` statements. This is a deterministic statement-count regression, **not** a wall-clock benchmark or a network-packet count. SQLx does not emit every transaction-opening command through its query logger. Connection setup, HTTP fetching and the API catalog stage are outside this row-pipeline comparison. Counts were gathered with the normal test profile; no Rust latency claim is inferred from them.

Run inside the Nix development shell with `DATABASE_URL` pointing to a migrated development PostgreSQL role with `CREATEDB`:

```sh
cargo test -p horae --features server lookup_cache --locked -- --nocapture
```

The tests create throwaway databases. Additional cases cover rollback before cache promotion, malformed rates after a cache hit, email/name namespace separation, distinct project-task pairs, new-run ambiguity detection and migration over existing normalized collisions. A separate counter regression verifies that a callsite first used on an untraced thread is still captured by the scoped measurement subscriber; the tests do not install a global logger.

## Cold lookup indexes

Caching still requires one SQL lookup for each distinct successful identity (and retries failures). Migration 22 adds non-unique `(org_id, harvest_norm(email))` and `(org_id, harvest_norm(name))` indexes. They preserve existing collisions so the resolver can report ambiguity; they do not impose a new identity policy or modify user records.

The reproducible [SQL probe](benchmarks/user-lookups.sql) copies the user schema into temporary tables, retains ordinary primary/unique/org indexes, and compares the actual `LIMIT 2` lookup with and without the expression indexes. It uses one organization, distinct identities, an exact match near the end of the table, `ANALYZE`, one warm-up and five measured samples per variant. It finishes with `ROLLBACK` and reads no real user data:

```sh
psql -X "$DATABASE_URL" -f specs/004-harvest-importer/benchmarks/user-lookups.sql
```

Local PostgreSQL 17.10, x86_64, 2026-09-08; median execution milliseconds from `EXPLAIN (ANALYZE, BUFFERS, FORMAT JSON)`:

| Users | Field | Without expression indexes | With expression indexes | Indexed variant's chosen plan |
|---|---|---:|---:|---|
| 100 | Email | 0.169 | 0.159 | Seq Scan |
| 100 | Name | 0.066 | 0.075 | Seq Scan |
| 1,000 | Email | 1.661 | 0.002 | Index Scan |
| 1,000 | Name | 0.699 | 0.002 | Index Scan |
| 50,000 | Email | 89.586 | 0.003 | Index Scan |
| 50,000 | Name | 42.471 | 0.002 | Index Scan |

The pair of indexes occupied 32,768 / 163,840 / 5,931,008 bytes at those respective sizes. Small tables still favored a sequential scan; the indexes target cold matches in larger organizations, not repeated cache hits. These are synthetic warm-cache samples, not production distributions or end-to-end speedup estimates. The script prints min/median/max and index size to make repeated local comparisons possible.

Deployment uses ordinary transactional `CREATE INDEX`, consistent with the existing migrations: user writes can wait while indexes are built. Schedule migration appropriately for large user tables. Index maintenance adds write/storage cost; this probe does not measure write throughput.

## API pages

The API importer now processes time-entry pages through a capacity-one channel. Blocking HTTP can retain the page being downloaded and one queued page while async SQL consumes the current page. Catalog metadata is indexed once; each `SourceRow` is assembled only when needed. The importer no longer holds all time-entry JSON objects, all typed entries and a second full vector of assembled rows.

Requests use 100 records per page. Responses are limited to 2,000 records and 10 MiB of decompressed JSON; these are input limits, not a promise that JSON parsing uses only 10 MiB of resident memory. Pagination follows the provider's `links.next`, including cursor URLs with a null `next_page`. Links must retain the original origin and collection path, contain no user information or fragment, and fit in 8 KiB. Redirects and cyclic pagination fail the run. Missing pagination/record fields and invalid JSON fail visibly instead of silently truncating the import. These choices follow Harvest's [pagination contract](https://help.getharvest.com/api-v2/introduction/overview/pagination/).

The client spaces requests by at least 160 ms, including retries across collections, and honors `Retry-After` in full for waits up to five minutes. Longer waits reject the run for a later retry; they are never shortened into an early request. There are at most six attempts per page. Concurrent traffic from other applications can still consume the account's quota and trigger 429 responses; see Harvest's [rate limits](https://help.getharvest.com/api-v2/introduction/overview/general/). Data-page connections have a 10-second connect limit and a 30-second request deadline. Backoff checks cancellation every 100 ms; an already-started blocking request can continue until it completes or times out. The existing ureq client's system DNS lookup is not interruptible and can exceed that deadline; the session lock remains owned until the worker actually exits.

One outer transaction still preserves preview rollback, per-row savepoints and the atomic data/provenance/watermark update. A later download failure rolls everything back. Row-level validation failures continue to later pages but suppress watermark advancement. Cancellation before commit drops the SQL transaction and closes the page receiver; the HTTP worker retains ownership of the close-on-drop database session until it really exits, so the organization advisory lock cannot be released early. This requires only one pool connection.

### Plans while tables grow

A prepared lookup planned against a nearly empty provenance table can retain an unsuitable generic plan as that table grows inside the import transaction. The [plan-cache probe](benchmarks/import-plan-cache.sql) reproduces this using a temporary copy with both original indexes. On local PostgreSQL 17.10, the cached plan used the reverse `(org_id, entity_type, horae_id)` index, filtered out 100,000 records on a miss, and took 8.342 ms. A custom plan used all three primary-key fields and took 0.022 ms (0.131 ms planning). These are single illustrative samples; the index predicates and filtered-row counts establish the extra work more directly than those timings.

The API data transaction sets `SET LOCAL plan_cache_mode = force_custom_plan`. This incurs planning work but avoids retaining the initial tiny-table plan throughout a growing import; PostgreSQL documents the [generic/custom plan tradeoff](https://www.postgresql.org/docs/17/runtime-config-query.html#GUC-PLAN-CACHE-MODE). It does not change database-wide settings, CSV behavior or the statement-count comparison above. The setting ends with the transaction.

```sh
psql -X "$DATABASE_URL" -f specs/004-harvest-importer/benchmarks/import-plan-cache.sql
```

### Reproducing the scale checks

The ignored scale tests generate 100,000 entries over 365 dates, with unique notes, one client/project/task, one known user and 100 entries referencing an unknown user. The first entry is invalid. A loopback HTTP fixture generates one page at a time, retains only the first 16 request headers, and uses the production request pacing. Each run verifies 1,004 HTTP requests, reconciled counts, 99,900 valid entries, 5,994,000 minutes and exact provenance counts. Preview verifies that no parents, time entries or provenance persist; reimport verifies zero creations. The invalid entries leave the watermark unchanged.

Run each scenario in a separate process so Linux's process high-water RSS is not inherited from another scenario:

```sh
cargo test -p horae --features server --release api_100k_commit_and_reimport --locked -- --ignored --nocapture
cargo test -p horae --features server --release api_100k_dry_run --locked -- --ignored --nocapture
```

Use the normal Nix development shell and a migrated disposable development database role with `CREATEDB`. SQLx creates a separate test database. The smaller `http_pages_preview_commit_and_reimport` test exercises the same HTTP path in the ordinary test suite.

For the pre-streaming reference, use a separate worktree at `cdba75d37d7ba35616d6c5e2254bd2f2039454e6`, apply [scale-baseline.patch](benchmarks/scale-baseline.patch), and run the same commands. That reference constructs the fetched JSON collection in-process and measures conversion, assembly and SQL application; it excludes HTTP. It is not an HTTP latency comparison.

RSS/HWM readings come from `/proc/self/status` and include the Rust test process and loopback fixture, but not PostgreSQL's process, database storage or the kernel's page cache. Release uses the checked-in optimization/LTO profile. These are local synthetic checks, not production capacity guarantees.

Reference measurement on 2026-09-08, before page streaming or transaction-local custom plans:

| Phase | Elapsed seconds | Reported process high-water RSS, KiB |
|---|---:|---:|
| Before constructing input | — | 9,136 |
| Fetched JSON collection constructed | — | 307,756 |
| JSON construction and typed conversion | 0.338 | 329,000 |
| First 100,000-entry application | 1,953.051 | 372,532 |
| Reimport in the same process | 91.033 | 372,532 |

The reference passed all row, minute, error, provenance and reimport assertions. The reimport inherits the process high-water mark; it is not an independent peak-memory measurement. An additional `/proc` sample near the end of the first application reported 373,076 KiB, consistent with approximately 364 MiB rather than an exact allocation count. The old preview scenario was not measured separately.

With page buffering and transaction-local custom plans, including loopback HTTP and production pacing:

| Phase | Elapsed seconds | Reported process high-water RSS, KiB |
|---|---:|---:|
| Before starting HTTP | — | 9,172 |
| First 100,000-entry import | 522.387 | 11,924 |
| Reimport in the same process | 160.717 | 11,952 |
| Before independent preview | — | 8,788 |
| 100,000-entry preview | 513.223 | 11,320 |

All three scenarios passed the row, error, minute and provenance checks; preview left no imported data persisted. The plan change and page buffering were applied together; the time improvement cannot be attributed to streaming alone, and the reference excludes HTTP. The HTTP reimport is paced over 1,004 requests, explaining its higher elapsed time than the reference's SQL-only reimport. Custom plans do not guarantee that the planner always selects the cheapest actual execution plan; the separate SQL probe illustrates one reproduced failure mode.

## Remaining scale limits

Memory still grows with catalog metadata, distinct cached parents/users/project-task pairs and accumulated row errors. The CSV adapter still retains uploaded bytes and parsed rows, plus occurrence keys. The API transaction remains open during time-entry downloading and application; this change does not batch commits or limit total database transaction/WAL size. The import remains request-scoped, not a durable background job; these measurements do not cover browser or reverse-proxy timeouts. Those limits are distinct from bounded time-entry page buffering.
