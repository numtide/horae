-- SQL twin of horae_core::rounding::effective_minutes. Apply rounding per
-- entry, never after summing; frozen values (including zero) always win.
-- Not STRICT: an absent frozen value means use the current organization rule.
CREATE FUNCTION effective_minutes(minutes integer, frozen integer, inc smallint, dir round_dir)
RETURNS integer
IMMUTABLE
PARALLEL SAFE
LANGUAGE sql
AS $$
  SELECT COALESCE(frozen, CASE
    WHEN inc <= 0 THEN minutes
    WHEN dir = 'down' THEN minutes - minutes % inc
    WHEN dir = 'up' THEN ((minutes::bigint + inc - 1) / inc * inc)::integer
    ELSE ((minutes::bigint / inc + CASE WHEN (minutes % inc) * 2 >= inc THEN 1 ELSE 0 END) * inc)::integer
  END)
$$;

-- Older invoices did not persist the billed duration on their entries. Their
-- line snapshot, not today's organization setting, is the historical value.
-- Never replace an already frozen value or rewrite an invoice line/amount.
UPDATE time_entries te
SET rounded_minutes = line.minutes
FROM invoice_line_items line
WHERE te.invoice_id = line.invoice_id
  AND te.id = line.time_entry_id
  AND te.rounded_minutes IS NULL;
