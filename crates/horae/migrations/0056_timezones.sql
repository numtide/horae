-- IANA zone names decide what "today" is for timers and the timesheet. The
-- application validates them against its zone database, which changes over
-- time, so there is no CHECK here.
ALTER TABLE organizations ADD COLUMN timezone text NOT NULL DEFAULT 'UTC';

-- NULL follows the organization's zone.
ALTER TABLE users ADD COLUMN timezone text;
