-- Every row has a `harvest_id` after 0061; new rows get one from the trigger.
-- NOT VALID skips the scan, so the ACCESS EXCLUSIVE lock is held only for the
-- catalogue change. 0063 validates and sets NOT NULL in a later transaction.
ALTER TABLE clients ADD CONSTRAINT clients_harvest_id_present
  CHECK (harvest_id IS NOT NULL) NOT VALID;
ALTER TABLE projects ADD CONSTRAINT projects_harvest_id_present
  CHECK (harvest_id IS NOT NULL) NOT VALID;
ALTER TABLE tasks ADD CONSTRAINT tasks_harvest_id_present
  CHECK (harvest_id IS NOT NULL) NOT VALID;
ALTER TABLE time_entries ADD CONSTRAINT time_entries_harvest_id_present
  CHECK (harvest_id IS NOT NULL) NOT VALID;
ALTER TABLE users ADD CONSTRAINT users_harvest_id_present
  CHECK (harvest_id IS NOT NULL) NOT VALID;
