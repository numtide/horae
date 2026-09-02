-- Enforce the SPEC §1 domain invariants at the database level.
-- Additive and non-destructive: adds CHECK constraints only, no data changes.
--   * durations are stored as non-negative integer minutes;
--   * money is a non-negative integer count of minor units (cents);
--   * currency columns hold a 3-letter ISO 4217 code.
-- Nullable columns are written so NULL passes (the invariant applies only when a
-- value is present). Columns that can legitimately be negative (e.g. sort_order)
-- are intentionally left unconstrained.

-- ── Durations (minutes) ──────────────────────────────────────────────────────

ALTER TABLE time_entries
  ADD CONSTRAINT time_entries_minutes_nonneg
    CHECK (minutes >= 0),
  ADD CONSTRAINT time_entries_rounded_minutes_nonneg
    CHECK (rounded_minutes IS NULL OR rounded_minutes >= 0);

ALTER TABLE invoice_line_items
  ADD CONSTRAINT invoice_line_items_minutes_nonneg
    CHECK (minutes >= 0);

ALTER TABLE projects
  ADD CONSTRAINT projects_budget_minutes_nonneg
    CHECK (budget_minutes IS NULL OR budget_minutes >= 0);

ALTER TABLE organizations
  ADD CONSTRAINT organizations_round_minutes_nonneg
    CHECK (round_minutes >= 0),
  ADD CONSTRAINT organizations_long_timer_minutes_nonneg
    CHECK (long_timer_minutes >= 0);

-- ── Money (integer cents) ────────────────────────────────────────────────────

ALTER TABLE users
  ADD CONSTRAINT users_cost_rate_cents_nonneg
    CHECK (cost_rate_cents IS NULL OR cost_rate_cents >= 0),
  ADD CONSTRAINT users_billable_rate_cents_nonneg
    CHECK (billable_rate_cents IS NULL OR billable_rate_cents >= 0);

ALTER TABLE tasks
  ADD CONSTRAINT tasks_default_rate_cents_nonneg
    CHECK (default_rate_cents IS NULL OR default_rate_cents >= 0);

ALTER TABLE project_tasks
  ADD CONSTRAINT project_tasks_rate_cents_nonneg
    CHECK (rate_cents IS NULL OR rate_cents >= 0);

ALTER TABLE assignments
  ADD CONSTRAINT assignments_rate_cents_nonneg
    CHECK (rate_cents IS NULL OR rate_cents >= 0);

ALTER TABLE projects
  ADD CONSTRAINT projects_budget_amount_cents_nonneg
    CHECK (budget_amount_cents IS NULL OR budget_amount_cents >= 0);

ALTER TABLE invoices
  ADD CONSTRAINT invoices_total_cents_nonneg
    CHECK (total_cents >= 0);

ALTER TABLE invoice_line_items
  ADD CONSTRAINT invoice_line_items_rate_cents_nonneg
    CHECK (rate_cents >= 0),
  ADD CONSTRAINT invoice_line_items_amount_cents_nonneg
    CHECK (amount_cents >= 0);

-- ── Currency codes (ISO 4217, exactly 3 letters) ─────────────────────────────
-- char(3) values are blank-padded; char_length ignores trailing blanks, so this
-- rejects a short code such as 'US' stored as 'US '.

ALTER TABLE organizations
  ADD CONSTRAINT organizations_default_currency_iso3
    CHECK (char_length(default_currency) = 3);

ALTER TABLE clients
  ADD CONSTRAINT clients_currency_iso3
    CHECK (char_length(currency) = 3);

ALTER TABLE projects
  ADD CONSTRAINT projects_currency_iso3
    CHECK (char_length(currency) = 3);

ALTER TABLE invoices
  ADD CONSTRAINT invoices_currency_iso3
    CHECK (char_length(currency) = 3);
