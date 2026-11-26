-- Round an invoice's discount and taxes to its currency's step (whole units
-- for a zero-decimal currency, hundredths otherwise) by a method the
-- organization chooses. Each invoice keeps the method it was computed with,
-- so changing the setting never changes an existing invoice.

CREATE TYPE public.rounding_method AS ENUM ('half_up', 'down', 'up');

ALTER TABLE public.organizations
    ADD COLUMN invoice_rounding public.rounding_method NOT NULL DEFAULT 'half_up';
ALTER TABLE public.invoices
    ADD COLUMN rounding public.rounding_method NOT NULL DEFAULT 'half_up';

-- `percentage` basis points of `amount` hundredths, rounded to the step by
-- magnitude: `down` toward zero, `up` away from zero, `half_up` half a step
-- away from zero. The SQL twin of `horae_core::invoice::Rounding::portion`,
-- which a parity test compares. `div` truncates exactly; numeric `/` would
-- round past 16 digits.
CREATE FUNCTION public.invoice_portion_cents(
    amount numeric, basis_points integer, currency text, method public.rounding_method)
    RETURNS numeric
    LANGUAGE sql IMMUTABLE PARALLEL SAFE
    AS $$
    SELECT sign(amount) * step * div(abs(amount) * basis_points + CASE method
        WHEN 'down' THEN 0
        WHEN 'up' THEN 10000 * step - 1
        ELSE 5000 * step
    END, 10000 * step)
    FROM (SELECT public.currency_amount_step(currency)::numeric AS step) currency_step
  $$;

-- Stop on invoices the new rules would reject, rather than rewrite issued
-- amounts: every existing invoice was computed half up to hundredths, so only
-- a zero-decimal invoice with a fractional discount or tax can fail here.
DO $$
DECLARE
    inexact text;
BEGIN
    SELECT string_agg(number, ', ' ORDER BY number) INTO inexact
    FROM public.invoices
    WHERE source = 'horae'
      AND public.currency_amount_step(currency) <> 1
      AND (discount_cents % public.currency_amount_step(currency) <> 0
           OR tax1_cents % public.currency_amount_step(currency) <> 0
           OR tax2_cents % public.currency_amount_step(currency) <> 0);
    IF inexact IS NOT NULL THEN
        RAISE EXCEPTION 'Invoices with fractional amounts in a zero-decimal currency: %', inexact
            USING HINT = 'Correct or delete these invoices, then run the migration again.';
    END IF;
END
$$;

-- An imported invoice keeps the amounts Harvest billed (FR-004); the importer
-- already refuses amounts finer than the currency, so it never trips these.
-- A discount never exceeds the subtotal, which a subtotal off the step could
-- otherwise round it past.
ALTER TABLE public.invoices
    DROP CONSTRAINT invoice_discount_exact,
    DROP CONSTRAINT invoice_tax1_exact,
    DROP CONSTRAINT invoice_tax2_exact,
    ADD CONSTRAINT invoice_discount_exact CHECK (
        source = 'harvest'
        OR discount_cents::numeric = LEAST(subtotal_cents::numeric, public.invoice_portion_cents(
            subtotal_cents::numeric, discount_bps, currency, rounding))),
    ADD CONSTRAINT invoice_tax1_exact CHECK (
        source = 'harvest'
        OR tax1_cents::numeric = public.invoice_portion_cents(
            subtotal_cents::numeric - discount_cents::numeric, tax1_bps, currency, rounding)),
    ADD CONSTRAINT invoice_tax2_exact CHECK (
        source = 'harvest'
        OR tax2_cents::numeric = public.invoice_portion_cents(
            subtotal_cents::numeric - discount_cents::numeric, COALESCE(tax2_bps::integer, 0),
            currency, rounding));
