-- Round per-entry amounts to the currency's precision (spec 026, research R1).
-- Money stays in hundredths for every currency; a zero-decimal currency's
-- amounts become multiples of 100. These are the SQL twins of
-- `horae_core::money::currency_decimals` and
-- `horae_core::invoice::line_amount_cents`, which a parity test compares.
--
-- Both functions must stay inlinable, because reports, project figures and
-- budgets call them once per time entry: no STRICT (Postgres inlines a STRICT
-- SQL function only when it can prove the body is NULL for NULL input), a
-- single expression, and each argument the callers pass as an expensive
-- expression (the rate cascade, `effective_minutes`) referenced once. Called
-- opaquely they made a 500,000-row sum about ten times slower.

-- The step, in hundredths, of a currency's amounts: 100 for the ISO 4217
-- zero-decimal codes of `ZERO_DECIMAL_CURRENCIES`, else 1. Trimmed and
-- case-sensitive, as Rust matches it. A NULL currency gets step 1; every
-- currency column the callers read is NOT NULL.
CREATE FUNCTION public.currency_amount_step(currency text) RETURNS integer
    LANGUAGE sql IMMUTABLE PARALLEL SAFE
    AS $$
    SELECT CASE WHEN btrim(currency) IN (
        'BIF', 'CLP', 'DJF', 'GNF', 'ISK', 'JPY', 'KMF', 'KRW', 'PYG', 'RWF', 'UGX', 'VND', 'VUV',
        'XAF', 'XOF', 'XPF'
    ) THEN 100 ELSE 1 END
  $$;

-- rate × minutes ÷ 60 rounded once, half up, to the currency's step. `div`
-- truncates toward zero as Rust's integer division does; do not use floor.
-- A NULL rate or duration gives NULL, as the STRICT function it replaces did.
CREATE FUNCTION public.line_amount_cents(rate_cents bigint, minutes integer, currency text)
    RETURNS bigint
    LANGUAGE sql IMMUTABLE PARALLEL SAFE
    AS $$
    SELECT (div(rate_cents::numeric * minutes + 30 * public.currency_amount_step(currency),
                60 * public.currency_amount_step(currency))
            * public.currency_amount_step(currency))::bigint
  $$;

-- `line_amount_cents(bigint, integer)` stays until a later migration: during a
-- rolling update the previous release keeps serving reports, project figures
-- and budget checks with it after this migration has run.
