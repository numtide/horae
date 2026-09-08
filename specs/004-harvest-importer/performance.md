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

## Remaining scale limits

This change does not establish bounded-memory network streaming or validate a 100,000-entry end-to-end import. The adapters still materialize collections, the transaction spans the run, and cache size grows with distinct parents/users/project-task pairs and CSV occurrence keys. Those memory, latency and transaction measurements remain separate from the lookup improvements above.
