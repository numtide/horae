-- Command history survives template deletion; snapshot IDs are not live FKs.
CREATE TABLE permission_change_receipts (
    id uuid PRIMARY KEY,
    org_id uuid NOT NULL REFERENCES organizations(id),
    actor_user_id uuid,
    operator_id text,
    operator_command text,
    request_id uuid NOT NULL,
    format_version integer NOT NULL CHECK (format_version > 0),
    intent jsonb NOT NULL CHECK (jsonb_typeof(intent) = 'object'),
    result jsonb NOT NULL CHECK (jsonb_typeof(result) = 'object'),
    audit jsonb NOT NULL CHECK (jsonb_typeof(audit) = 'object'),
    created_at timestamptz NOT NULL DEFAULT now(),
    FOREIGN KEY (org_id, actor_user_id) REFERENCES users(org_id, id),
    CHECK (
        (actor_user_id IS NOT NULL AND operator_id IS NULL AND operator_command IS NULL)
        OR (actor_user_id IS NULL AND operator_id IS NOT NULL AND operator_command IS NOT NULL
            AND btrim(operator_id) <> '' AND btrim(operator_command) <> '')
    )
);

CREATE UNIQUE INDEX permission_change_receipts_user_request_key
    ON permission_change_receipts (org_id, actor_user_id, request_id)
    WHERE actor_user_id IS NOT NULL;
CREATE UNIQUE INDEX permission_change_receipts_operator_request_key
    ON permission_change_receipts (org_id, operator_id, request_id)
    WHERE operator_id IS NOT NULL;
