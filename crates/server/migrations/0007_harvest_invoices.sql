-- Invoices imported from Harvest (spec 025). An imported invoice is a row of
-- `invoices` with source 'harvest', so every list, the entries' invoice_id and
-- the unique number keep working. Its Harvest-only values and its lines live in
-- their own tables, so the invariants of Horae's own lines stay as they are.

-- Written off: no longer due, not paid, its time stays invoiced.
ALTER TYPE invoice_status ADD VALUE 'closed';
ALTER TYPE harvest_entity_type ADD VALUE 'invoice';
CREATE TYPE invoice_source AS ENUM ('horae', 'harvest');

ALTER TABLE invoices ADD COLUMN source invoice_source NOT NULL DEFAULT 'horae';

-- Harvest taxes only the lines it flags and rounds its own way; an imported
-- invoice keeps the amounts Harvest billed, so Horae's rounding rules hold for
-- Horae's own invoices only.
ALTER TABLE invoices
    DROP CONSTRAINT invoice_discount_exact,
    DROP CONSTRAINT invoice_tax1_exact,
    DROP CONSTRAINT invoice_tax2_exact,
    ADD CONSTRAINT invoice_discount_exact CHECK (
        source = 'harvest'
        OR discount_cents::numeric = floor((subtotal_cents::numeric * discount_bps::numeric + 5000) / 10000)),
    ADD CONSTRAINT invoice_tax1_exact CHECK (
        source = 'harvest'
        OR tax1_cents::numeric = floor(((subtotal_cents::numeric - discount_cents::numeric) * tax1_bps::numeric + 5000) / 10000)),
    ADD CONSTRAINT invoice_tax2_exact CHECK (
        source = 'harvest'
        OR tax2_cents::numeric = floor(((subtotal_cents::numeric - discount_cents::numeric) * COALESCE(tax2_bps::integer, 0)::numeric + 5000) / 10000)),
    -- A value added to an enum cannot be used in the transaction that adds it,
    -- so the status compares as text.
    ADD CONSTRAINT invoices_closed_imported CHECK (status::text <> 'closed' OR source = 'harvest'),
    -- An imported invoice's rates are Harvest's, as written, in harvest_invoices.
    ADD CONSTRAINT invoices_harvest_no_rates CHECK (
        source = 'horae'
        OR (discount_bps = 0 AND tax1_bps = 0 AND tax2_bps IS NULL AND tax2_name IS NULL));

CREATE TABLE harvest_invoices (
    invoice_id uuid PRIMARY KEY,
    org_id uuid NOT NULL REFERENCES organizations (id),
    harvest_invoice_id bigint NOT NULL,
    -- What is still due; paid = total - due.
    due_cents bigint NOT NULL CHECK (due_cents >= 0),
    paid_on date,
    -- Percentages exactly as Harvest wrote them, for display only.
    discount_rate numeric,
    tax1_rate numeric,
    tax2_rate numeric,
    period_start date,
    period_end date,
    harvest_updated_at timestamptz NOT NULL,
    FOREIGN KEY (invoice_id, org_id) REFERENCES invoices (id, org_id) ON DELETE CASCADE,
    UNIQUE (org_id, harvest_invoice_id)
);

CREATE TABLE harvest_invoice_lines (
    id uuid PRIMARY KEY,
    org_id uuid NOT NULL REFERENCES organizations (id),
    invoice_id uuid NOT NULL,
    position integer NOT NULL CHECK (position >= 0),
    harvest_line_id bigint NOT NULL,
    kind text NOT NULL CHECK (char_length(kind) BETWEEN 1 AND 100),
    description text NOT NULL,
    -- Exact decimals as Harvest wrote them, for display only: a unit price
    -- may have more decimals than the currency, and nothing sums it.
    quantity numeric NOT NULL,
    unit_price numeric NOT NULL,
    amount_cents bigint NOT NULL,
    taxed boolean NOT NULL,
    taxed2 boolean NOT NULL,
    project_id uuid REFERENCES projects (id),
    FOREIGN KEY (invoice_id, org_id) REFERENCES invoices (id, org_id) ON DELETE CASCADE,
    UNIQUE (invoice_id, position)
);

CREATE INDEX harvest_invoice_lines_project_id_idx ON harvest_invoice_lines (project_id);
