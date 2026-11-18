-- Real update times on the catalogue, so lists can filter "updated since".
-- Existing rows take their creation time; tasks had none, so theirs is now.

ALTER TABLE projects ADD COLUMN updated_at timestamptz NOT NULL DEFAULT now();
ALTER TABLE clients ADD COLUMN updated_at timestamptz NOT NULL DEFAULT now();
ALTER TABLE users ADD COLUMN updated_at timestamptz NOT NULL DEFAULT now();
ALTER TABLE tasks
    ADD COLUMN created_at timestamptz NOT NULL DEFAULT now(),
    ADD COLUMN updated_at timestamptz NOT NULL DEFAULT now();

-- The backfill is not an edit: it must not advance the project's edit revision.
ALTER TABLE projects DISABLE TRIGGER projects_edit_revision;
UPDATE projects SET updated_at = created_at;
ALTER TABLE projects ENABLE TRIGGER projects_edit_revision;
UPDATE clients SET updated_at = created_at;
UPDATE users SET updated_at = created_at;

CREATE TRIGGER projects_set_updated_at BEFORE UPDATE ON projects
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
CREATE TRIGGER clients_set_updated_at BEFORE UPDATE ON clients
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
CREATE TRIGGER users_set_updated_at BEFORE UPDATE ON users
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
CREATE TRIGGER tasks_set_updated_at BEFORE UPDATE ON tasks
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
