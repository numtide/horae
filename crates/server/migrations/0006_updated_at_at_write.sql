-- Stamp update times when a row is written, not when its transaction began.
-- The Harvest API's lists serve only rows stamped before the oldest
-- transaction still writing began (horae-harvest-api's `pagination::horizon`).
-- That holds only if a row's stamp is never earlier than the moment its own
-- write happened: with now(), a transaction that began before the horizon but
-- writes after it would stamp a time a client has already passed.

CREATE OR REPLACE FUNCTION set_updated_at() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
  NEW.updated_at = clock_timestamp();
  RETURN NEW;
END
$$;

ALTER TABLE clients ALTER COLUMN updated_at SET DEFAULT clock_timestamp();
ALTER TABLE projects ALTER COLUMN updated_at SET DEFAULT clock_timestamp();
ALTER TABLE tasks ALTER COLUMN updated_at SET DEFAULT clock_timestamp();
ALTER TABLE users ALTER COLUMN updated_at SET DEFAULT clock_timestamp();
ALTER TABLE person_permission_states ALTER COLUMN updated_at SET DEFAULT clock_timestamp();
ALTER TABLE time_entries ALTER COLUMN updated_at SET DEFAULT clock_timestamp();
