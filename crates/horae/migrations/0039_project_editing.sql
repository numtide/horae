-- Imports and detail actions must invalidate open editors too.
ALTER TABLE projects ADD COLUMN edit_revision bigint NOT NULL DEFAULT 1 CHECK (edit_revision > 0);

CREATE FUNCTION advance_project_edit_revision() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
  IF OLD IS DISTINCT FROM NEW THEN
    NEW.edit_revision := OLD.edit_revision + 1;
  END IF;
  RETURN NEW;
END;
$$;
REVOKE ALL ON FUNCTION advance_project_edit_revision() FROM PUBLIC;
CREATE TRIGGER projects_edit_revision BEFORE UPDATE ON projects
FOR EACH ROW EXECUTE FUNCTION advance_project_edit_revision();

CREATE FUNCTION invalidate_project_editor() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
  IF TG_OP = 'UPDATE' AND OLD IS NOT DISTINCT FROM NEW THEN
    RETURN NEW;
  END IF;
  IF TG_OP = 'DELETE' THEN
    UPDATE projects SET edit_revision = edit_revision + 1 WHERE id = OLD.project_id;
    RETURN OLD;
  END IF;
  IF TG_OP = 'UPDATE' AND OLD.project_id IS DISTINCT FROM NEW.project_id THEN
    UPDATE projects SET edit_revision = edit_revision + 1 WHERE id = OLD.project_id;
  END IF;
  UPDATE projects SET edit_revision = edit_revision + 1 WHERE id = NEW.project_id;
  RETURN NEW;
END;
$$;
REVOKE ALL ON FUNCTION invalidate_project_editor() FROM PUBLIC;

DO $$
DECLARE relation_name text;
BEGIN
  FOREACH relation_name IN ARRAY ARRAY[
    'project_settings', 'project_private_settings', 'project_tasks', 'assignments',
    'project_member_costs', 'project_member_budgets', 'project_task_settings',
    'project_task_members', 'project_tag_links', 'project_fee_milestones',
    'project_fee_occurrences'
  ] LOOP
    EXECUTE format(
      'CREATE TRIGGER project_editor_changed AFTER INSERT OR UPDATE OR DELETE ON %I FOR EACH ROW EXECUTE FUNCTION invalidate_project_editor()',
      relation_name
    );
  END LOOP;
END;
$$;

-- Request payloads can contain private notes/costs: no plugin reader grants.
CREATE TABLE project_edit_requests (
  id uuid PRIMARY KEY,
  org_id uuid NOT NULL REFERENCES organizations(id),
  project_id uuid NOT NULL,
  actor_id uuid NOT NULL,
  expected_revision bigint NOT NULL CHECK (expected_revision > 0),
  completed_revision bigint NOT NULL CHECK (completed_revision > 0),
  payload jsonb NOT NULL CHECK (jsonb_typeof(payload) = 'object' AND octet_length(payload::text) <= 262144),
  created_at timestamptz NOT NULL DEFAULT now(),
  FOREIGN KEY (project_id, org_id) REFERENCES projects(id, org_id) ON DELETE CASCADE,
  FOREIGN KEY (actor_id, org_id) REFERENCES users(id, org_id)
);
REVOKE ALL ON project_edit_requests FROM PUBLIC;

-- Reordering must retain milestone identities already used by invoice sources.
ALTER TABLE project_fee_milestones DROP CONSTRAINT project_fee_milestones_project_id_position_key;
ALTER TABLE project_fee_milestones ADD CONSTRAINT project_fee_milestones_project_id_position_key
  UNIQUE (project_id, position) DEFERRABLE INITIALLY DEFERRED;
