-- Canonical natural-key normalization for the Harvest importer (FR-012).
-- Additive: adds one IMMUTABLE function; alters no tables.
--
-- The importer matches records by a trimmed, case-folded natural key. The Rust
-- side (`horae_core::harvest_import::keys::normalize`) and this SQL function MUST
-- produce byte-identical output, or a re-import could fail to match and create a
-- duplicate. To keep them provably identical:
--   * trim exactly the same whitespace set (space, tab, LF, VT, FF, CR, NBSP) via
--     `btrim`'s character-set argument — not the default `trim`, which strips the
--     space character only;
--   * fold ONLY ASCII A-Z with `translate` — the exact behaviour of Rust's
--     `to_ascii_lowercase`, avoiding locale-dependent `lower()` on non-ASCII.
CREATE FUNCTION harvest_norm(t text) RETURNS text
  IMMUTABLE
  LANGUAGE sql
  AS $$
    SELECT translate(
      btrim(t, E' \t\n\x0B\f\r '),
      'ABCDEFGHIJKLMNOPQRSTUVWXYZ',
      'abcdefghijklmnopqrstuvwxyz'
    )
  $$;
