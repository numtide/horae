-- Round per-entry amounts to the currency's precision (spec 026, research R1).
-- Money stays in hundredths for every currency; a zero-decimal currency's
-- amounts become multiples of 100. These are the SQL twins of
-- `horae_core::money::currency_decimals` and
-- `horae_core::invoice::line_amount_cents`, which a parity test compares.

-- The step, in hundredths, of a currency's amounts: 100 for the ISO 4217
-- zero-decimal codes of `ZERO_DECIMAL_CURRENCIES`, else 1. Trimmed and
-- case-sensitive, as Rust matches it.
CREATE FUNCTION public.currency_amount_step(currency text) RETURNS integer
    LANGUAGE sql IMMUTABLE STRICT PARALLEL SAFE
    AS $$
    SELECT CASE WHEN btrim(currency) IN (
        'BIF', 'CLP', 'DJF', 'GNF', 'ISK', 'JPY', 'KMF', 'KRW', 'PYG', 'RWF', 'UGX', 'VND', 'VUV',
        'XAF', 'XOF', 'XPF'
    ) THEN 100 ELSE 1 END
  $$;

-- rate × minutes ÷ 60 rounded once, half up, to the currency's step. `div`
-- truncates toward zero as Rust's integer division does; do not use floor.
CREATE FUNCTION public.line_amount_cents(rate_cents bigint, minutes integer, currency text)
    RETURNS bigint
    LANGUAGE sql IMMUTABLE STRICT PARALLEL SAFE
    AS $$
    SELECT (div(rate_cents::numeric * minutes + 30 * public.currency_amount_step(currency),
                60 * public.currency_amount_step(currency))
            * public.currency_amount_step(currency))::bigint
  $$;

DROP FUNCTION public.line_amount_cents(bigint, integer);
