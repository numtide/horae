-- `harvest_import_id` is the id a record had in Harvest (specs/021-harvest-api-compat).
--
-- Renamed so it does not read as the `harvest_id` the numbered tables gain
-- next, which is the id in the Harvest-compatible API. A rename only changes
-- the catalogue; the brief ACCESS EXCLUSIVE lock is released at commit.
ALTER TABLE harvest_import_map RENAME COLUMN harvest_id TO harvest_import_id;
