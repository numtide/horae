-- Harvest refuses a person's day above 240 hours ("Daily hours cannot exceed
-- 240."). The server enforces that day total in each write transaction; these
-- checks bound a single entry, so no stored value rounds past `integer`:
--   * minutes: at most one full capped day, 14,400;
--   * rounded_minutes: frozen at approval or invoicing by rounding minutes by
--     the organization's increment, a smallint, so at most 14,400 + 32,767.
-- NOT VALID: new writes are checked, existing rows are not. A timer left
-- running for days may already have stored more, so a later migration
-- validates these once production has been checked.

ALTER TABLE time_entries
  ADD CONSTRAINT time_entries_minutes_daily_cap
    CHECK (minutes <= 14400) NOT VALID,
  ADD CONSTRAINT time_entries_rounded_minutes_cap
    CHECK (rounded_minutes IS NULL OR rounded_minutes <= 14400 + 32767) NOT VALID;
