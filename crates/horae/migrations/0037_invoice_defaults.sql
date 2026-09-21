ALTER TABLE invoices
  ADD COLUMN po_number text NOT NULL DEFAULT '' CHECK (char_length(po_number) <= 200),
  ADD COLUMN discount_bps smallint NOT NULL DEFAULT 0 CHECK (discount_bps BETWEEN 0 AND 10000),
  ADD COLUMN tax1_bps smallint NOT NULL DEFAULT 0 CHECK (tax1_bps BETWEEN 0 AND 10000),
  ADD COLUMN tax2_name text CHECK (char_length(btrim(tax2_name)) BETWEEN 1 AND 100),
  ADD COLUMN tax2_bps smallint CHECK (tax2_bps BETWEEN 0 AND 10000),
  ADD COLUMN discount_cents bigint NOT NULL DEFAULT 0 CHECK (discount_cents >= 0),
  ADD COLUMN tax1_cents bigint NOT NULL DEFAULT 0 CHECK (tax1_cents >= 0),
  ADD COLUMN tax2_cents bigint NOT NULL DEFAULT 0 CHECK (tax2_cents >= 0),
  ADD CONSTRAINT invoice_second_tax_pair CHECK ((tax2_name IS NULL) = (tax2_bps IS NULL));

-- Derive these snapshots only from invoice-owned values. Legacy writers keep
-- their existing dates/total and automatically receive a zero-adjustment subtotal.
ALTER TABLE invoices
  ADD COLUMN terms_days integer GENERATED ALWAYS AS (due_on - issued_on) STORED NOT NULL,
  ADD COLUMN subtotal_cents bigint GENERATED ALWAYS AS
    ((total_cents::numeric + discount_cents - tax1_cents - tax2_cents)::bigint) STORED NOT NULL;

ALTER TABLE invoices
  ADD CONSTRAINT invoice_subtotal_nonnegative CHECK (subtotal_cents >= 0),
  ADD CONSTRAINT invoice_discount_exact CHECK
    (discount_cents = floor((subtotal_cents::numeric * discount_bps + 5000) / 10000)),
  ADD CONSTRAINT invoice_tax1_exact CHECK
    (tax1_cents = floor(((subtotal_cents::numeric - discount_cents) * tax1_bps + 5000) / 10000)),
  ADD CONSTRAINT invoice_tax2_exact CHECK
    (tax2_cents = floor(((subtotal_cents::numeric - discount_cents) * COALESCE(tax2_bps, 0) + 5000) / 10000));
