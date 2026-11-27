-- A fixed fee is money, so it is a whole number of its currency's step
-- (`currency_amount_step`, migration 0010): whole units for a zero-decimal
-- currency. A fractional fee makes an invoice whose total cannot be paid
-- exactly. The project form and the draft editor refuse one; this keeps any
-- other writer out.
--
-- Existing rows are not rewritten: a fee that breaks the rule stops the
-- migration, naming the projects, so someone decides what each should be.
DO $$
DECLARE
    offending text;
BEGIN
    SELECT string_agg(DISTINCT p.id::text || ' (' || p.currency || ')', ', ')
      INTO offending
      FROM public.projects p
     WHERE EXISTS (SELECT 1 FROM public.project_settings s
                    WHERE s.project_id = p.id
                      AND s.fee_amount_cents % public.currency_amount_step(p.currency) <> 0)
        OR EXISTS (SELECT 1 FROM public.project_fee_milestones m
                    WHERE m.project_id = p.id
                      AND m.amount_cents % public.currency_amount_step(p.currency) <> 0)
        OR EXISTS (SELECT 1 FROM public.project_fee_occurrences f
                    WHERE f.project_id = p.id
                      AND f.amount_cents % public.currency_amount_step(f.currency) <> 0);
    IF offending IS NOT NULL THEN
        RAISE EXCEPTION 'project fees with a fraction of their currency''s unit: %', offending
            USING HINT = 'Set each fee to a whole amount of its currency, then rerun the migration.';
    END IF;
END
$$;

-- An occurrence carries its own currency.
ALTER TABLE public.project_fee_occurrences
    ADD CONSTRAINT project_fee_occurrences_amount_currency_step
    CHECK (amount_cents % public.currency_amount_step(currency) = 0);

-- A fee schedule's currency is its project's, which a CHECK cannot read, so
-- constraint triggers check the project's whole schedule against its current
-- currency when a fee or the currency is written. The app never changes a
-- project's currency, so they are deferred to the commit only for a data
-- migration, which may then rewrite the currency and the fees in either order.
-- A failure is a check violation named after the rule.
CREATE FUNCTION public.check_fee_amount_currency_step() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
DECLARE
    project uuid;
BEGIN
    -- Separate statements: `NEW.project_id` does not exist on a projects row.
    IF TG_TABLE_NAME = 'projects' THEN
        project := NEW.id;
    ELSE
        project := NEW.project_id;
    END IF;
    IF EXISTS (
        SELECT 1 FROM public.projects p
         WHERE p.id = project
           AND (EXISTS (SELECT 1 FROM public.project_settings s
                         WHERE s.project_id = p.id
                           AND s.fee_amount_cents % public.currency_amount_step(p.currency) <> 0)
                OR EXISTS (SELECT 1 FROM public.project_fee_milestones m
                            WHERE m.project_id = p.id
                              AND m.amount_cents % public.currency_amount_step(p.currency) <> 0))
    ) THEN
        RAISE EXCEPTION 'project % has fees with a fraction of its currency''s unit', project
            USING ERRCODE = 'check_violation', CONSTRAINT = 'project_fee_amount_currency_step';
    END IF;
    RETURN NULL;
END
$$;

CREATE CONSTRAINT TRIGGER project_settings_fee_amount_currency_step
    AFTER INSERT OR UPDATE OF fee_amount_cents, project_id ON public.project_settings
    DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW WHEN (NEW.fee_amount_cents IS NOT NULL)
    EXECUTE FUNCTION public.check_fee_amount_currency_step();
CREATE CONSTRAINT TRIGGER project_fee_milestones_amount_currency_step
    AFTER INSERT OR UPDATE OF amount_cents, project_id ON public.project_fee_milestones
    DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION public.check_fee_amount_currency_step();
CREATE CONSTRAINT TRIGGER projects_fee_amount_currency_step
    AFTER UPDATE OF currency ON public.projects
    DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW WHEN (OLD.currency IS DISTINCT FROM NEW.currency)
    EXECUTE FUNCTION public.check_fee_amount_currency_step();
