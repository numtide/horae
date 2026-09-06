-- Indexes for three time_entries scans that 0012 did not cover (additive: no
-- table/column/data changes).
--
-- 0012 filled the structural gaps (unindexed FKs, org scoping). These three come
-- from measuring the queries that actually run, against a 510k-row copy of this
-- schema. Like 0012, plain CREATE INDEX: sqlx runs migrations in a transaction,
-- which forbids CREATE INDEX CONCURRENTLY.

-- Voiding an invoice clears every entry it billed (`WHERE invoice_id = $1`), and
-- invoice_id is a foreign key with no index. Partial because the overwhelming
-- majority of entries are never invoiced, which keeps the index to the rows the
-- query can actually match. 42.1 ms sequential scan -> 0.07 ms index scan.
CREATE INDEX ON time_entries (invoice_id) WHERE invoice_id IS NOT NULL;

-- The Harvest v2 API's `updated_since` filter is how clients sync incrementally,
-- so it runs on every poll and reads a small tail of a large table. The 0012
-- (org_id, spent_date) index does not serve it — spent_date is when the work
-- happened, updated_at is when the row last changed. 22.7 ms -> 0.67 ms.
CREATE INDEX ON time_entries (org_id, updated_at);

-- `check_project_budget` re-sums a project's entire history after every entry
-- create, update, delete, and timer stop. INCLUDE carries the two summed columns
-- in the index so the sum is an index-only scan that never touches the heap:
-- 3.81 ms -> 0.21 ms.
--
-- The tradeoff is size: ~24 MB at 510k rows against ~8 MB for a plain
-- (project_id) index. The payload is why — it makes every entry distinct, so the
-- btree cannot deduplicate the repeated project_ids the way a plain index does,
-- and it is written on every entry write. If that ever outweighs the budget
-- check, dropping just the INCLUDE clause keeps the index and most of the win.
CREATE INDEX ON time_entries (project_id) INCLUDE (minutes, rounded_minutes);
