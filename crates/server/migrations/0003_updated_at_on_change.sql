-- An UPDATE that rewrites a row with the same values (a re-import, a no-op
-- save) keeps its updated_at, so "updated since" lists only real changes.
-- In a BEFORE trigger the WHEN sees NEW after earlier triggers, so on projects
-- it also sees projects_edit_revision's bump, which only happens on a change.

DROP TRIGGER projects_set_updated_at ON projects;
CREATE TRIGGER projects_set_updated_at BEFORE UPDATE ON projects
    FOR EACH ROW WHEN (OLD.* IS DISTINCT FROM NEW.*) EXECUTE FUNCTION set_updated_at();

DROP TRIGGER clients_set_updated_at ON clients;
CREATE TRIGGER clients_set_updated_at BEFORE UPDATE ON clients
    FOR EACH ROW WHEN (OLD.* IS DISTINCT FROM NEW.*) EXECUTE FUNCTION set_updated_at();

DROP TRIGGER users_set_updated_at ON users;
CREATE TRIGGER users_set_updated_at BEFORE UPDATE ON users
    FOR EACH ROW WHEN (OLD.* IS DISTINCT FROM NEW.*) EXECUTE FUNCTION set_updated_at();

DROP TRIGGER tasks_set_updated_at ON tasks;
CREATE TRIGGER tasks_set_updated_at BEFORE UPDATE ON tasks
    FOR EACH ROW WHEN (OLD.* IS DISTINCT FROM NEW.*) EXECUTE FUNCTION set_updated_at();

DROP TRIGGER time_entries_set_updated_at ON time_entries;
CREATE TRIGGER time_entries_set_updated_at BEFORE UPDATE ON time_entries
    FOR EACH ROW WHEN (OLD.* IS DISTINCT FROM NEW.*) EXECUTE FUNCTION set_updated_at();
