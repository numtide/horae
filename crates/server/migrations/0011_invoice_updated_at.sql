-- A real update time on invoices, so `/api/v1/invoices` can filter "updated
-- since" (spec 028). Existing rows take their creation time. Like the
-- catalogue's (0003, 0006), it is stamped when a row is written, and only
-- when the invoice changes, so a sync sees status changes and edits, not
-- no-op saves.
--
-- A BEFORE trigger's WHEN cannot read the generated columns (terms_days,
-- subtotal_cents), so it compares the stored ones by name; a new column of
-- invoices belongs in this list. edit_revision is left out: its own trigger
-- advances it on every update, changed or not.

ALTER TABLE invoices ADD COLUMN updated_at timestamptz NOT NULL DEFAULT clock_timestamp();
UPDATE invoices SET updated_at = created_at;

CREATE TRIGGER invoices_set_updated_at BEFORE UPDATE ON invoices
    FOR EACH ROW WHEN (
        (OLD.client_id, OLD.number, OLD.status, OLD.issued_on, OLD.due_on, OLD.currency,
         OLD.total_cents, OLD.notes, OLD.po_number, OLD.discount_bps, OLD.tax1_bps,
         OLD.tax2_name, OLD.tax2_bps, OLD.discount_cents, OLD.tax1_cents, OLD.tax2_cents,
         OLD.subject, OLD.source)
        IS DISTINCT FROM
        (NEW.client_id, NEW.number, NEW.status, NEW.issued_on, NEW.due_on, NEW.currency,
         NEW.total_cents, NEW.notes, NEW.po_number, NEW.discount_bps, NEW.tax1_bps,
         NEW.tax2_name, NEW.tax2_bps, NEW.discount_cents, NEW.tax1_cents, NEW.tax2_cents,
         NEW.subject, NEW.source)
    ) EXECUTE FUNCTION set_updated_at();

CREATE INDEX invoices_org_id_updated_at_idx ON invoices (org_id, updated_at);
