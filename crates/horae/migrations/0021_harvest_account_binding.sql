-- Provenance IDs belong to one Harvest account, even after OAuth disconnect.
CREATE TABLE harvest_account_bindings (
  org_id             uuid PRIMARY KEY REFERENCES organizations(id),
  harvest_account_id text NOT NULL
);

INSERT INTO harvest_account_bindings (org_id, harvest_account_id)
SELECT org_id, harvest_account_id FROM harvest_credentials;

-- Legacy provenance without credentials has no recoverable account identity.
-- Connecting such an organization requires operator verification, not guessing.
