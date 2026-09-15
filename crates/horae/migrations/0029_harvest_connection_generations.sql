-- This boundary must outlive credentials, bindings and retained job reports.
CREATE TABLE harvest_connection_generations (
    org_id uuid PRIMARY KEY REFERENCES organizations(id),
    account_generation bigint NOT NULL DEFAULT 0 CHECK (account_generation >= 0),
    connection_revision bigint NOT NULL DEFAULT 0 CHECK (connection_revision >= 0)
);

INSERT INTO harvest_connection_generations (org_id) SELECT id FROM organizations;

ALTER TABLE horae_jobs ADD COLUMN account_generation bigint NOT NULL DEFAULT 0
    CHECK (account_generation >= 0);
