ALTER TABLE horae_outbox ADD COLUMN failed_at timestamptz;
ALTER TABLE horae_outbox ADD CONSTRAINT outbox_terminal_state
  CHECK (delivered_at IS NULL OR failed_at IS NULL);

CREATE INDEX horae_outbox_pending_kind_idx
  ON horae_outbox (event_kind, available_at, created_at)
  WHERE delivered_at IS NULL AND failed_at IS NULL;
