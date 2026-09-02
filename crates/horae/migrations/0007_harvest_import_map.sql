-- Provenance mapping for the Harvest importer (data-model.md, FR-012/FR-026).
-- Additive: introduces a new enum and a new table; alters no existing columns.
--
-- Maps a stable Harvest record id to the Horae record it was imported into, so a
-- re-sync matches by Harvest identity — exact and edit-robust — ahead of the
-- composite natural key. Written only on a committing run, never in a dry-run.

CREATE TYPE harvest_entity_type AS ENUM ('client', 'project', 'task', 'time_entry');

CREATE TABLE harvest_import_map (
  org_id              uuid                NOT NULL REFERENCES organizations(id),
  harvest_entity_type harvest_entity_type NOT NULL,
  harvest_id          bigint              NOT NULL,
  horae_id            uuid                NOT NULL,
  harvest_updated_at  timestamptz,
  created_at          timestamptz         NOT NULL DEFAULT now(),
  PRIMARY KEY (org_id, harvest_entity_type, harvest_id)
);

-- Reverse lookup (does this Horae row already have a mapping?) and cascade-free
-- housekeeping by Horae id.
CREATE INDEX harvest_import_map_horae_id ON harvest_import_map (org_id, harvest_entity_type, horae_id);
