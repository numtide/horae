-- Additive storage only: no legacy role mapping or policy activation.
ALTER TABLE organizations
    ADD COLUMN permission_policy_version integer NOT NULL DEFAULT 0
        CHECK (permission_policy_version >= 0),
    ADD COLUMN access_revision bigint NOT NULL DEFAULT 0 CHECK (access_revision >= 0);

ALTER TABLE users ADD CONSTRAINT users_org_id_id_key UNIQUE (org_id, id);

CREATE TABLE permission_templates (
    id uuid PRIMARY KEY,
    org_id uuid NOT NULL REFERENCES organizations(id),
    name text NOT NULL CHECK (char_length(name) BETWEEN 1 AND 100),
    catalog_version integer NOT NULL CHECK (catalog_version > 0),
    grants text[] NOT NULL CHECK (array_position(grants, NULL) IS NULL),
    revision bigint NOT NULL DEFAULT 0 CHECK (revision >= 0),
    created_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (org_id, id)
);

-- The database is the sole case-comparison authority, including concurrent inserts.
CREATE UNIQUE INDEX permission_templates_org_name_key
    ON permission_templates (org_id, lower(name));

CREATE TABLE person_permission_states (
    id uuid PRIMARY KEY,
    org_id uuid NOT NULL REFERENCES organizations(id),
    user_id uuid NOT NULL,
    catalog_version integer NOT NULL CHECK (catalog_version > 0),
    grants text[] NOT NULL CHECK (array_position(grants, NULL) IS NULL),
    is_administrator boolean NOT NULL,
    source text NOT NULL,
    built_in_profile text,
    template_id uuid,
    applied_template_revision bigint CHECK (applied_template_revision >= 0),
    revision bigint NOT NULL DEFAULT 0 CHECK (revision >= 0),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (org_id, user_id),
    FOREIGN KEY (org_id, user_id) REFERENCES users(org_id, id),
    FOREIGN KEY (org_id, template_id) REFERENCES permission_templates(org_id, id)
        ON DELETE RESTRICT,
    CHECK (
        (source = 'built_in' AND built_in_profile IS NOT NULL
            AND built_in_profile IN ('member', 'project_manager', 'people_admin',
                'accounting', 'executive_manager', 'administrator')
            AND template_id IS NULL AND applied_template_revision IS NULL)
        OR (source = 'template' AND built_in_profile IS NULL
            AND template_id IS NOT NULL AND applied_template_revision IS NOT NULL)
        OR (source = 'individual' AND built_in_profile IS NULL
            AND template_id IS NULL AND applied_template_revision IS NULL)
    )
);

CREATE INDEX person_permission_states_template_idx
    ON person_permission_states (org_id, template_id) WHERE template_id IS NOT NULL;

CREATE TRIGGER person_permission_states_set_updated_at
    BEFORE UPDATE ON person_permission_states
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
