-- Invoice numbers are generated from a COUNT of existing invoices, which two
-- concurrent transactions can compute identically. Enforce uniqueness per org
-- at the database level so a race surfaces as a constraint violation instead
-- of two invoices sharing a number.

-- A database that already hit that race holds duplicate numbers, which would
-- make the ALTER below fail. Renumber the later invoices of each duplicate
-- group first, deterministically (ordered by created_at, then id): the first
-- invoice keeps its number, its twins get their group position appended —
-- 'INV-202607-001' stays, the duplicates become 'INV-202607-001-2',
-- 'INV-202607-001-3', and so on. Generated numbers always end in a
-- three-digit sequence, so the suffixed form cannot collide with a number
-- that already exists.
WITH ranked AS (
  SELECT id,
         row_number() OVER (PARTITION BY org_id, number ORDER BY created_at, id) AS rn
  FROM invoices
)
UPDATE invoices i
SET number = i.number || '-' || ranked.rn
FROM ranked
WHERE ranked.id = i.id
  AND ranked.rn > 1;

ALTER TABLE invoices
  ADD CONSTRAINT invoices_org_id_number_key UNIQUE (org_id, number);
