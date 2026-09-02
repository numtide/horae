-- Maintain `updated_at` in the database so no writer can forget it.
-- A single trigger function stamps NEW.updated_at = now() on every UPDATE; it is
-- attached to each table that carries an updated_at column. The application no
-- longer sets updated_at by hand.

CREATE FUNCTION set_updated_at() RETURNS trigger
  LANGUAGE plpgsql
  AS $$
BEGIN
  NEW.updated_at = now();
  RETURN NEW;
END
$$;

CREATE TRIGGER time_entries_set_updated_at
  BEFORE UPDATE ON time_entries
  FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE TRIGGER harvest_credentials_set_updated_at
  BEFORE UPDATE ON harvest_credentials
  FOR EACH ROW EXECUTE FUNCTION set_updated_at();
