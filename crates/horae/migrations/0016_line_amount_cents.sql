-- Money arithmetic the aggregating reports can run in SQL (FR-024).
-- Additive: adds one IMMUTABLE function; alters no tables.
--
-- The per-project spend total and the grouped time report used to ship one row
-- per time entry to the server just so the money could go through
-- `horae_core::invoice::line_amount_cents`. With this function they group in
-- Postgres and ship one row per group instead. The rate cascade needs no twin:
-- `resolve_rate` is exactly `COALESCE(pt.rate_cents, a.rate_cents,
-- u.billable_rate_cents)`, which the queries write out inline.
--
-- This function and the Rust one MUST stay bit-identical, or a report would
-- disagree with the invoice generated from the same entries.
-- `integration.rs::line_amount_cents_sql_matches_rust` pins that: it drives both
-- from one table of adversarial inputs and asserts they agree. The rounding term
-- is per row, so `SUM(f(rate, minutes)) <> f(rate, SUM(minutes))` — a caller
-- either aggregates entirely here or entirely in Rust, never half in each.
--
-- What makes the two provably identical:
--   * both operands are integers and stay integers — bigint * integer is bigint,
--     matching `i64 * i64`, and no float ever enters the expression;
--   * integer division truncates toward zero in both PostgreSQL and Rust, and
--     migration 0010's CHECK constraints keep rates and minutes non-negative
--     anyway, so the two agree on the sign convention regardless;
--   * STRICT because the Rust signature takes plain integers, not options: an
--     absent rate has no amount, so NULL in means NULL out. The body is strict
--     in both arguments, so this still inlines.
--   * PARALLEL SAFE because it is pure arithmetic, and an unsafe function in
--     the target list would bar a parallel plan for the whole aggregate.
CREATE FUNCTION line_amount_cents(rate_cents bigint, minutes integer) RETURNS bigint
  IMMUTABLE
  STRICT
  PARALLEL SAFE
  LANGUAGE sql
  AS $$
    SELECT (rate_cents * minutes + 30) / 60
  $$;
