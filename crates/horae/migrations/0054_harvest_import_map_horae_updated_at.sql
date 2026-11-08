-- The time entry's own `updated_at` as the importer last wrote it. A later
-- Harvest edit updates the Horae entry only while the two still match, so an
-- edit made in Horae after the import is never overwritten. NULL on mappings
-- written before this column: those fall back to the entry's `created_at`.
ALTER TABLE harvest_import_map ADD COLUMN horae_updated_at timestamptz;
