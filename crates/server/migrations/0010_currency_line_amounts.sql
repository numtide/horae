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
-- expression (the rate cascade, `effective_minutes`) referenced once; only
-- the cheap currency argument is repeated. Called opaquely they made a
-- 500,000-row sum several times slower.

-- The step, in hundredths, of a currency's amounts: 100 for the ISO 4217
-- zero-decimal codes of `ZERO_DECIMAL_CURRENCIES`, else 1. Trimmed and
-- case-sensitive, as Rust matches it. A NULL currency gets step 1; every
-- currency column the callers read is NOT NULL.
CREATE FUNCTION public.currency_amount_step(currency text) RETURNS integer
    LANGUAGE sql IMMUTABLE PARALLEL SAFE
    AS $$
    SELECT CASE WHEN btrim(currency) IN (
        'BIF', 'BYR', 'CLP', 'DJF', 'GNF', 'ISK', 'JPY', 'KMF', 'KRW', 'PYG', 'RWF', 'UGX', 'UYI',
        'VND', 'VUV', 'XAF', 'XOF', 'XPF'
    ) THEN 100 ELSE 1 END
  $$;

-- rate × minutes ÷ 60 rounded once, half up, to the currency's step. `div`
-- truncates toward zero as Rust's integer division does, and is exact: do not
-- use floor, or numeric `/`, which rounds once the quotient passes 16 digits.
-- For a step of 100, truncating the hundredths again to whole units equals
-- dividing by 6000 at once, since truncating divisions compose. The currency
-- is tested twice and the rate and minutes are used once each.
-- A NULL rate or duration gives NULL, as the STRICT function it replaces did.
CREATE FUNCTION public.line_amount_cents(rate_cents bigint, minutes integer, currency text)
    RETURNS bigint
    LANGUAGE sql IMMUTABLE PARALLEL SAFE
    AS $$
    SELECT trunc(div(rate_cents::numeric * minutes + 30 * public.currency_amount_step(currency), 60),
                 CASE public.currency_amount_step(currency) WHEN 100 THEN -2 ELSE 0 END)::bigint
  $$;

DROP FUNCTION public.line_amount_cents(bigint, integer);
