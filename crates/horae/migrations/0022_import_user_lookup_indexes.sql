-- Cold lookups still scan all users once per distinct imported identity.
-- Keep these non-unique: existing normalized collisions must remain visible
-- to the importer's ambiguity check, not block deployment or select a user.
CREATE INDEX users_import_email_idx ON users (org_id, harvest_norm(email));
CREATE INDEX users_import_name_idx ON users (org_id, harvest_norm(name));
