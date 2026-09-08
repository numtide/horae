-- Preserve the per-line formula and function identity from migration 0016,
-- but allow a large intermediate product when the final bigint still fits.
-- numeric is exact decimal arithmetic, not floating point. div truncates
-- toward zero like Rust's integer division; a numeric-to-bigint cast alone
-- would round again. The final cast raises 22003 if the amount cannot fit.
-- For non-negative stored rates/minutes the existing +30 rounds half up.
CREATE OR REPLACE FUNCTION line_amount_cents(rate_cents bigint, minutes integer) RETURNS bigint
  IMMUTABLE
  STRICT
  PARALLEL SAFE
  LANGUAGE sql
  AS $$
    SELECT div(rate_cents::numeric * minutes + 30, 60)::bigint
  $$;
