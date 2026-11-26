-- The authority an operation's actor is loaded with, in one round trip.
--
-- `Actor::load` used to send three statements: lock the organization, lock
-- the person, read the permission state. This function runs the same three,
-- in that order, inside one call. PL/pgSQL keeps what separate statements
-- did:
--
-- * Each statement takes its own snapshot at READ COMMITTED (the function is
--   VOLATILE). When the organization lock waits for an access change, the
--   person and the permission state are then read as that change committed
--   them. A single joined statement would read them from the snapshot taken
--   before the wait. At REPEATABLE READ every statement uses the
--   transaction's snapshot, and a lock on a row changed after it fails with
--   40001, as before.
-- * Each statement is prepared when it is reached, so its table locks are
--   taken then too: nothing touches the permission states before the
--   organization and the person are locked.
--
-- Locks are taken organization first, then person, the order every access
-- change uses. `with_person` is false for the channels with no person (the
-- operator, the system), which lock only the organization.
--
-- No row means no such organization. Otherwise the row carries the
-- organization's settings, which many operations need and which its lock
-- holds still, whether the person is an active member, and their stored
-- permission state, if any.
CREATE FUNCTION horae_actor_authority(org uuid, person uuid, with_person boolean)
RETURNS TABLE (
    person_found boolean,
    timezone text,
    week_start smallint,
    default_currency character(3),
    access_revision bigint,
    round_minutes smallint,
    round_dir round_dir,
    catalog_version integer,
    grants text[],
    is_administrator boolean,
    source text,
    built_in_profile text,
    template_id uuid,
    applied_template_revision bigint,
    revision bigint
)
LANGUAGE plpgsql VOLATILE AS $$
#variable_conflict use_column
DECLARE
    found_person boolean := false;
BEGIN
    PERFORM 1 FROM organizations WHERE id = org FOR SHARE;
    IF NOT FOUND THEN
        RETURN;
    END IF;
    IF with_person THEN
        PERFORM 1 FROM users
         WHERE org_id = org AND id = person AND active
           FOR SHARE;
        found_person := FOUND;
    END IF;
    IF found_person THEN
        RETURN QUERY
            SELECT true,
                   o.timezone, o.week_start, o.default_currency, o.access_revision,
                   o.round_minutes, o.round_dir,
                   s.catalog_version, s.grants, s.is_administrator, s.source,
                   s.built_in_profile, s.template_id, s.applied_template_revision, s.revision
              FROM organizations o
              LEFT JOIN person_permission_states s
                ON s.org_id = o.id AND s.user_id = person
             WHERE o.id = org;
    ELSE
        RETURN QUERY
            SELECT false,
                   o.timezone, o.week_start, o.default_currency, o.access_revision,
                   o.round_minutes, o.round_dir,
                   NULL::integer, NULL::text[], NULL::boolean, NULL::text,
                   NULL::text, NULL::uuid, NULL::bigint, NULL::bigint
              FROM organizations o
             WHERE o.id = org;
    END IF;
END;
$$;

-- Only the application calls it. The plugin database role must reach no
-- function beyond those it was approved for, and this one takes locks.
REVOKE EXECUTE ON FUNCTION horae_actor_authority(uuid, uuid, boolean) FROM PUBLIC;
