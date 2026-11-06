-- Harvest people are imported as accounts (spec 019). Their provenance links a
-- Harvest user id to the Horae account it matched or created.
ALTER TYPE harvest_entity_type ADD VALUE 'person';
