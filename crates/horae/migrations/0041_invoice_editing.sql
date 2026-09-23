ALTER TABLE invoices ADD COLUMN edit_revision bigint NOT NULL DEFAULT 1 CHECK (edit_revision > 0);

CREATE FUNCTION advance_invoice_edit_revision() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
  IF OLD IS DISTINCT FROM NEW THEN
    NEW.edit_revision := OLD.edit_revision + 1;
  END IF;
  RETURN NEW;
END;
$$;
REVOKE ALL ON FUNCTION advance_invoice_edit_revision() FROM PUBLIC;
CREATE TRIGGER invoices_edit_revision BEFORE UPDATE ON invoices
FOR EACH ROW EXECUTE FUNCTION advance_invoice_edit_revision();

CREATE FUNCTION invalidate_invoice_editor() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
  IF TG_OP = 'UPDATE' AND OLD IS NOT DISTINCT FROM NEW THEN
    RETURN NEW;
  END IF;
  IF TG_OP = 'DELETE' THEN
    UPDATE invoices SET edit_revision = edit_revision + 1 WHERE id = OLD.invoice_id;
    RETURN OLD;
  END IF;
  IF TG_OP = 'UPDATE' AND OLD.invoice_id IS DISTINCT FROM NEW.invoice_id THEN
    UPDATE invoices SET edit_revision = edit_revision + 1 WHERE id = OLD.invoice_id;
  END IF;
  UPDATE invoices SET edit_revision = edit_revision + 1 WHERE id = NEW.invoice_id;
  RETURN NEW;
END;
$$;
REVOKE ALL ON FUNCTION invalidate_invoice_editor() FROM PUBLIC;
CREATE TRIGGER invoice_editor_changed AFTER INSERT OR UPDATE OR DELETE ON invoice_line_items
FOR EACH ROW EXECUTE FUNCTION invalidate_invoice_editor();

CREATE TABLE invoice_edit_requests (
  id uuid PRIMARY KEY,
  org_id uuid NOT NULL REFERENCES organizations(id),
  actor_id uuid NOT NULL,
  invoice_id uuid NOT NULL,
  completed_revision bigint NOT NULL CHECK (completed_revision > 0),
  payload jsonb NOT NULL CHECK (jsonb_typeof(payload) = 'object' AND octet_length(payload::text) <= 262144),
  created_at timestamptz NOT NULL DEFAULT now(),
  FOREIGN KEY (actor_id, org_id) REFERENCES users(id, org_id),
  FOREIGN KEY (invoice_id, org_id) REFERENCES invoices(id, org_id)
);
REVOKE ALL ON invoice_edit_requests FROM PUBLIC;
