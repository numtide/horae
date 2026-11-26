-- A project's money budget is an amount, so it is a whole number of its
-- currency's ISO 4217 minor unit: hundredths in multiples of 100 for a
-- zero-decimal currency (`currency_amount_step`, migration 0010). `/api/v1`
-- serves budgets in minor units and must never round one. The project form
-- refuses such a budget; this keeps any other writer out.
--
-- Existing rows are not rewritten: a budget that breaks the rule stops the
-- migration, naming the projects, so someone decides what each should be.
DO $$
DECLARE
    offending text;
BEGIN
    SELECT string_agg(id::text || ' (' || currency || ' ' || budget_amount_cents || ' hundredths)', ', ')
      INTO offending
      FROM projects
     WHERE budget_amount_cents % public.currency_amount_step(currency) <> 0;
    IF offending IS NOT NULL THEN
        RAISE EXCEPTION 'project budgets with a fraction of their currency''s unit: %', offending
            USING HINT = 'Set each budget to a whole amount of its currency, then rerun the migration.';
    END IF;
END
$$;

ALTER TABLE projects
    ADD CONSTRAINT projects_budget_amount_currency_step
    CHECK (budget_amount_cents % public.currency_amount_step(currency) = 0);
