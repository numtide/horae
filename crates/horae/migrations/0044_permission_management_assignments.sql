-- Management scope is independent of tracking membership and its business children.
-- No legacy backfill, cascading deletion or policy activation.
CREATE TABLE project_management_assignments (
    id uuid PRIMARY KEY,
    org_id uuid NOT NULL REFERENCES organizations(id),
    manager_id uuid NOT NULL,
    project_id uuid NOT NULL,
    revision bigint NOT NULL DEFAULT 0 CHECK (revision >= 0),
    UNIQUE (org_id, manager_id, project_id),
    FOREIGN KEY (manager_id, org_id) REFERENCES users(id, org_id),
    FOREIGN KEY (project_id, org_id) REFERENCES projects(id, org_id)
);

CREATE TABLE person_management_assignments (
    id uuid PRIMARY KEY,
    org_id uuid NOT NULL REFERENCES organizations(id),
    manager_id uuid NOT NULL,
    managed_user_id uuid NOT NULL,
    revision bigint NOT NULL DEFAULT 0 CHECK (revision >= 0),
    CHECK (manager_id <> managed_user_id),
    UNIQUE (org_id, manager_id, managed_user_id),
    FOREIGN KEY (manager_id, org_id) REFERENCES users(id, org_id),
    FOREIGN KEY (managed_user_id, org_id) REFERENCES users(id, org_id)
);
