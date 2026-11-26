-- A task's default rate becomes a template: hundredths with no currency, which
-- take the currency of the project they are applied to (spec 026). Every number
-- is kept whatever currency it was stored with, because Horae stores every
-- currency in hundredths; a zero is no rate.
UPDATE tasks SET default_rate_cents = NULL WHERE default_rate_cents = 0;

ALTER TABLE tasks DROP COLUMN default_rate_currency;

ALTER TABLE tasks DROP CONSTRAINT tasks_default_rate_cents_nonneg;
ALTER TABLE tasks ADD CONSTRAINT tasks_default_rate_cents_positive
    CHECK (default_rate_cents IS NULL OR default_rate_cents > 0);

COMMENT ON COLUMN tasks.default_rate_cents IS
    'Rate template: hundredths with no currency; a project applies it in its own currency.';
