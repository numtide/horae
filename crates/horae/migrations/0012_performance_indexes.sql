-- Performance indexes (additive: no table/column/data changes).
--
-- Fills the index gaps found by auditing every table's foreign-key and
-- org-scoping columns against the existing PKs, unique constraints, and indexes.
-- Only columns that are NOT already the leading column of an existing
-- index/unique/PK are covered here, so nothing added below is redundant.
-- Uses plain CREATE INDEX (sqlx runs migrations in a transaction, which forbids
-- CREATE INDEX CONCURRENTLY).

-- ── time_entries (hot, append-heavy) ─────────────────────────────────────────

-- Every query is org-scoped and reports filter `WHERE spent_date BETWEEN …`,
-- but no existing index leads with org_id (the existing composites lead with
-- user_id / project_id). Serves org-wide date-range reports and exports.
CREATE INDEX ON time_entries (org_id, spent_date);

-- task_id is a foreign key with no index (Postgres does not auto-index FKs);
-- task joins and task deletes seq-scan the whole table without it.
CREATE INDEX ON time_entries (task_id);

-- Block-range index on the time-ordered spent_date: tiny to store and maintain,
-- ideal for the date-range scans on this large append-heavy table.
CREATE INDEX ON time_entries USING brin (spent_date);

-- ── org-scoping / FK indexes on the other tables ─────────────────────────────

-- users are listed and looked up per organization; org_id is an unindexed FK.
CREATE INDEX ON users (org_id);

-- clients are listed per organization; org_id is an unindexed FK.
CREATE INDEX ON clients (org_id);

-- projects are listed per organization and per client; both are unindexed FKs.
CREATE INDEX ON projects (org_id);
CREATE INDEX ON projects (client_id);

-- the task catalog is scoped per organization; org_id is an unindexed FK.
CREATE INDEX ON tasks (org_id);

-- project_tasks.project_id is already the leading PK column; task_id is the
-- trailing FK with no index (needed to resolve "which projects enable this task"
-- and to check the FK on task deletes).
CREATE INDEX ON project_tasks (task_id);

-- assignments.project_id is already covered by UNIQUE (project_id, user_id);
-- user_id is the unindexed FK that backs "my assignments / my projects".
CREATE INDEX ON assignments (user_id);

-- approvals.user_id is already covered by UNIQUE (user_id, period_start); the
-- approvals queue is read per organization, and org_id is an unindexed FK.
CREATE INDEX ON approvals (org_id);

-- invoices are listed per organization and per client; both are unindexed FKs.
CREATE INDEX ON invoices (org_id);
CREATE INDEX ON invoices (client_id);

-- invoice_line_items.invoice_id is already the leading column of
-- UNIQUE (invoice_id, time_entry_id); time_entry_id is the trailing FK with no
-- index, needed to check references when a time entry is modified or deleted.
CREATE INDEX ON invoice_line_items (time_entry_id);
