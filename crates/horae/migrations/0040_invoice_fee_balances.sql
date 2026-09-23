-- Invoice lines are the fee ledger; invoice status determines whether they count.
ALTER TABLE invoice_line_items
  ADD COLUMN allocated_discount_cents bigint NOT NULL DEFAULT 0,
  ADD COLUMN net_before_tax_cents bigint GENERATED ALWAYS AS
    (amount_cents - allocated_discount_cents) STORED,
  ADD CONSTRAINT invoice_line_discount_bounds CHECK
    (allocated_discount_cents BETWEEN 0 AND amount_cents);

-- Match core's largest-remainder allocation without rewriting gross amounts,
-- headers or source identities. Stable source order breaks equal remainders.
WITH source AS (
  SELECT l.id, l.invoice_id, l.time_entry_id, f.project_id, f.period_key,
         l.amount_cents::numeric AS gross, i.discount_cents::numeric AS discount,
         sum(l.amount_cents::numeric) OVER (PARTITION BY l.invoice_id) AS subtotal
  FROM invoice_line_items l
  JOIN invoices i ON i.id = l.invoice_id
  LEFT JOIN project_fee_occurrences f ON f.id = l.fee_occurrence_id
), shares AS (
  SELECT *, CASE WHEN subtotal = 0 THEN 0 ELSE floor(gross * discount / subtotal) END AS share,
            CASE WHEN subtotal = 0 THEN 0 ELSE mod(gross * discount, subtotal) END AS remainder
  FROM source
), ranked AS (
  SELECT *, discount - sum(share) OVER (PARTITION BY invoice_id) AS residual,
         row_number() OVER (PARTITION BY invoice_id ORDER BY remainder DESC,
           time_entry_id NULLS LAST, project_id, period_key COLLATE "C", id) AS position
  FROM shares
)
UPDATE invoice_line_items l
SET allocated_discount_cents = (r.share + CASE WHEN r.position <= r.residual THEN 1 ELSE 0 END)::bigint
FROM ranked r WHERE r.id = l.id;

CREATE INDEX invoice_lines_fee_balance ON invoice_line_items (fee_occurrence_id, invoice_id)
  WHERE fee_occurrence_id IS NOT NULL;
ALTER TABLE project_fee_occurrences DROP COLUMN invoice_id;

CREATE TABLE invoice_generation_requests (
  id uuid PRIMARY KEY,
  org_id uuid NOT NULL REFERENCES organizations(id),
  actor_id uuid NOT NULL,
  invoice_id uuid NOT NULL,
  payload jsonb NOT NULL CHECK (jsonb_typeof(payload) = 'object' AND octet_length(payload::text) <= 262144),
  created_at timestamptz NOT NULL DEFAULT now(),
  FOREIGN KEY (actor_id, org_id) REFERENCES users(id, org_id),
  FOREIGN KEY (invoice_id, org_id) REFERENCES invoices(id, org_id)
);
REVOKE ALL ON invoice_generation_requests FROM PUBLIC;
