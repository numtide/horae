-- A task default rate of zero with no currency is how the Harvest importer
-- stored a Harvest task without a default rate. Zero is no rate; the importer
-- no longer stores it, and this clears the rows it already wrote. A zero rate
-- with a currency was entered on purpose and stays.
UPDATE tasks SET default_rate_cents = NULL
 WHERE default_rate_cents = 0 AND default_rate_currency IS NULL;
