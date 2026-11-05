-- Harvest refuses a person's day above 240 hours ("Daily hours cannot exceed
-- 240."). The server enforces that day total in each write transaction; this
-- trigger bounds a single entry at one full day, 14,400 minutes, so no newly
-- written value rounds past `integer` (frozen rounding adds at most one
-- smallint increment).
--
-- A trigger rather than a CHECK: a timer left running for days may already
-- have stored more, and a CHECK, even NOT VALID, tests every new row version,
-- so submitting, approving, invoicing or sweeping such a row would fail. The
-- trigger refuses only a new entry over the limit or raising one over it; an
-- entry already over may still be lowered. It fires only on writes that set
-- `minutes`, so state, invoice and sweep updates skip it.

CREATE FUNCTION time_entries_minutes_daily_cap() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
  IF NEW.minutes > 14400
     AND (TG_OP = 'INSERT' OR NEW.minutes > OLD.minutes) THEN
    RAISE EXCEPTION 'Daily hours cannot exceed 240.'
      USING ERRCODE = 'check_violation';
  END IF;
  RETURN NEW;
END
$$;
REVOKE ALL ON FUNCTION time_entries_minutes_daily_cap() FROM PUBLIC;

CREATE TRIGGER time_entries_minutes_daily_cap
  BEFORE INSERT OR UPDATE OF minutes ON time_entries
  FOR EACH ROW EXECUTE FUNCTION time_entries_minutes_daily_cap();
