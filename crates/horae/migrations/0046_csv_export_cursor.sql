-- Native, bounded transport for the connection-local CSV source snapshot.
-- The application applies migrations as its runtime role. Separate migration
-- owners must grant EXECUTE to their runtime role explicitly.
CREATE FUNCTION fetch_csv_export_rows(max_rows integer)
RETURNS SETOF record LANGUAGE plpgsql SECURITY INVOKER AS $$
DECLARE
    source refcursor := 'horae_csv';
    item record;
    payload_bytes bigint := 0;
BEGIN
    IF max_rows IS NULL OR max_rows < 1 OR max_rows > 128 THEN
        RAISE EXCEPTION 'Invalid CSV batch size';
    END IF;
    FOR position IN 1..max_rows LOOP
        FETCH NEXT FROM source INTO item;
        EXIT WHEN NOT FOUND;
        IF item.export_bytes IS NULL OR item.export_bytes < 0 THEN
            RAISE EXCEPTION 'Invalid CSV payload weight';
        END IF;
        payload_bytes := payload_bytes + item.export_bytes;
        RETURN NEXT item;
        -- Include the crossing row: never lose a record already advanced past.
        EXIT WHEN payload_bytes >= 65536;
    END LOOP;
END;
$$;
REVOKE ALL ON FUNCTION fetch_csv_export_rows(integer) FROM PUBLIC;
