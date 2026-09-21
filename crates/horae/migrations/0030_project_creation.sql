-- New creation settings are opt-in. Existing and imported projects retain the
-- legacy billing cascade when no project_settings row exists.
ALTER TABLE users ADD CONSTRAINT users_id_org_unique UNIQUE (id, org_id);
ALTER TABLE projects ADD CONSTRAINT projects_id_org_unique UNIQUE (id, org_id);
ALTER TABLE tasks ADD CONSTRAINT tasks_id_org_unique UNIQUE (id, org_id);

ALTER TABLE clients ADD COLUMN default_rate_cents bigint
  CHECK (default_rate_cents IS NULL OR default_rate_cents >= 0);

CREATE TABLE project_drafts (
  id                   uuid PRIMARY KEY,
  org_id               uuid NOT NULL REFERENCES organizations(id),
  creator_id           uuid NOT NULL,
  revision             bigint NOT NULL DEFAULT 1 CHECK (revision > 0),
  payload_version      smallint NOT NULL DEFAULT 1 CHECK (payload_version = 1),
  payload              jsonb NOT NULL CHECK (
                         jsonb_typeof(payload) = 'object'
                         AND octet_length(payload::text) <= 262144),
  completed_project_id uuid,
  discarded_at         timestamptz,
  created_at           timestamptz NOT NULL DEFAULT now(),
  updated_at           timestamptz NOT NULL DEFAULT now(),
  FOREIGN KEY (creator_id, org_id) REFERENCES users(id, org_id),
  FOREIGN KEY (completed_project_id, org_id) REFERENCES projects(id, org_id),
  CHECK (completed_project_id IS NULL OR discarded_at IS NULL)
);

CREATE UNIQUE INDEX project_drafts_current_creator_idx
  ON project_drafts (org_id, creator_id)
  WHERE completed_project_id IS NULL AND discarded_at IS NULL;

CREATE TABLE project_settings (
  id                  uuid PRIMARY KEY,
  org_id              uuid NOT NULL REFERENCES organizations(id),
  project_id          uuid NOT NULL UNIQUE,
  creator_id          uuid NOT NULL,
  rate_mode           text NOT NULL CHECK (rate_mode IN ('person', 'task', 'project')),
  budget_scope        text NOT NULL DEFAULT 'project'
                      CHECK (budget_scope IN ('project', 'task', 'person')),
  monthly_reset       boolean NOT NULL DEFAULT false,
  include_nonbillable boolean NOT NULL DEFAULT false,
  alert_enabled       boolean NOT NULL DEFAULT false,
  alert_threshold     smallint NOT NULL DEFAULT 80 CHECK (alert_threshold BETWEEN 0 AND 100),
  report_visibility   text NOT NULL DEFAULT 'managers'
                      CHECK (report_visibility IN ('managers', 'project_members')),
  fee_mode            text CHECK (fee_mode IN ('single', 'milestones', 'monthly')),
  fee_amount_cents    bigint CHECK (fee_amount_cents >= 0),
  monthly_day         text CHECK (monthly_day IN ('first', 'fifteenth', 'last')),
  terms_days          smallint NOT NULL DEFAULT 30 CHECK (terms_days BETWEEN 0 AND 365),
  po_number           text NOT NULL DEFAULT '' CHECK (char_length(po_number) <= 200),
  discount_bps        smallint NOT NULL DEFAULT 0 CHECK (discount_bps BETWEEN 0 AND 10000),
  tax1_bps            smallint NOT NULL DEFAULT 0 CHECK (tax1_bps BETWEEN 0 AND 10000),
  tax2_name           text CHECK (char_length(btrim(tax2_name)) BETWEEN 1 AND 100),
  tax2_bps            smallint CHECK (tax2_bps BETWEEN 0 AND 10000),
  FOREIGN KEY (project_id, org_id) REFERENCES projects(id, org_id) ON DELETE CASCADE,
  FOREIGN KEY (creator_id, org_id) REFERENCES users(id, org_id),
  CHECK ((tax2_name IS NULL) = (tax2_bps IS NULL)),
  CHECK (CASE fee_mode
    WHEN 'single' THEN fee_amount_cents IS NOT NULL AND monthly_day IS NULL
    WHEN 'milestones' THEN fee_amount_cents IS NULL AND monthly_day IS NULL
    WHEN 'monthly' THEN fee_amount_cents IS NOT NULL AND monthly_day IS NOT NULL
    ELSE fee_amount_cents IS NULL AND monthly_day IS NULL
  END)
);

-- Private fields must not inherit table-level SELECT grants on projects or
-- assignments held by the plugin database role.
CREATE TABLE project_private_settings (
  id          uuid PRIMARY KEY,
  org_id      uuid NOT NULL REFERENCES organizations(id),
  project_id  uuid NOT NULL UNIQUE,
  admin_notes text NOT NULL DEFAULT '' CHECK (char_length(admin_notes) <= 10000),
  FOREIGN KEY (project_id, org_id) REFERENCES projects(id, org_id) ON DELETE CASCADE
);

CREATE TABLE project_member_costs (
  id              uuid PRIMARY KEY,
  org_id          uuid NOT NULL REFERENCES organizations(id),
  project_id      uuid NOT NULL,
  user_id         uuid NOT NULL,
  cost_rate_cents bigint NOT NULL CHECK (cost_rate_cents >= 0),
  UNIQUE (project_id, user_id),
  FOREIGN KEY (project_id, org_id) REFERENCES projects(id, org_id) ON DELETE CASCADE,
  FOREIGN KEY (user_id, org_id) REFERENCES users(id, org_id),
  FOREIGN KEY (project_id, user_id) REFERENCES assignments(project_id, user_id) ON DELETE CASCADE
);

CREATE TABLE project_tags (
  id      uuid PRIMARY KEY,
  org_id  uuid NOT NULL REFERENCES organizations(id),
  name    text NOT NULL CHECK (char_length(btrim(name)) BETWEEN 1 AND 50),
  UNIQUE (id, org_id)
);

CREATE UNIQUE INDEX project_tags_name_idx ON project_tags (org_id, lower(btrim(name)));

CREATE TABLE project_tag_links (
  id         uuid PRIMARY KEY,
  org_id     uuid NOT NULL REFERENCES organizations(id),
  project_id uuid NOT NULL,
  tag_id     uuid NOT NULL,
  UNIQUE (project_id, tag_id),
  FOREIGN KEY (project_id, org_id) REFERENCES projects(id, org_id) ON DELETE CASCADE,
  FOREIGN KEY (tag_id, org_id) REFERENCES project_tags(id, org_id)
);

CREATE TABLE project_task_settings (
  id           uuid PRIMARY KEY,
  org_id       uuid NOT NULL REFERENCES organizations(id),
  project_id   uuid NOT NULL,
  task_id      uuid NOT NULL,
  restricted   boolean NOT NULL DEFAULT false,
  budget_minutes bigint CHECK (budget_minutes >= 0),
  budget_cents bigint CHECK (budget_cents >= 0),
  UNIQUE (project_id, task_id),
  FOREIGN KEY (project_id, org_id) REFERENCES projects(id, org_id) ON DELETE CASCADE,
  FOREIGN KEY (task_id, org_id) REFERENCES tasks(id, org_id),
  FOREIGN KEY (project_id, task_id) REFERENCES project_tasks(project_id, task_id) ON DELETE CASCADE,
  CHECK (budget_minutes IS NULL OR budget_cents IS NULL)
);

CREATE TABLE project_task_members (
  id         uuid PRIMARY KEY,
  org_id     uuid NOT NULL REFERENCES organizations(id),
  project_id uuid NOT NULL,
  task_id    uuid NOT NULL,
  user_id    uuid NOT NULL,
  UNIQUE (project_id, task_id, user_id),
  FOREIGN KEY (project_id, org_id) REFERENCES projects(id, org_id) ON DELETE CASCADE,
  FOREIGN KEY (task_id, org_id) REFERENCES tasks(id, org_id),
  FOREIGN KEY (user_id, org_id) REFERENCES users(id, org_id),
  FOREIGN KEY (project_id, task_id) REFERENCES project_task_settings(project_id, task_id) ON DELETE CASCADE,
  FOREIGN KEY (project_id, user_id) REFERENCES assignments(project_id, user_id) ON DELETE CASCADE
);

CREATE TABLE project_member_budgets (
  id             uuid PRIMARY KEY,
  org_id         uuid NOT NULL REFERENCES organizations(id),
  project_id     uuid NOT NULL,
  user_id        uuid NOT NULL,
  budget_minutes bigint NOT NULL CHECK (budget_minutes >= 0),
  UNIQUE (project_id, user_id),
  FOREIGN KEY (project_id, org_id) REFERENCES projects(id, org_id) ON DELETE CASCADE,
  FOREIGN KEY (user_id, org_id) REFERENCES users(id, org_id),
  FOREIGN KEY (project_id, user_id) REFERENCES assignments(project_id, user_id) ON DELETE CASCADE
);

CREATE TABLE project_fee_milestones (
  id           uuid PRIMARY KEY,
  org_id       uuid NOT NULL REFERENCES organizations(id),
  project_id   uuid NOT NULL,
  name         text NOT NULL CHECK (char_length(btrim(name)) BETWEEN 1 AND 200),
  due_on       date NOT NULL,
  amount_cents bigint NOT NULL CHECK (amount_cents >= 0),
  position     smallint NOT NULL CHECK (position BETWEEN 0 AND 99),
  UNIQUE (project_id, position),
  UNIQUE (id, org_id),
  FOREIGN KEY (project_id, org_id) REFERENCES projects(id, org_id) ON DELETE CASCADE
);

REVOKE ALL ON project_drafts, project_settings, project_private_settings,
  project_member_costs, project_tags, project_tag_links, project_task_settings,
  project_task_members, project_member_budgets, project_fee_milestones FROM PUBLIC;
