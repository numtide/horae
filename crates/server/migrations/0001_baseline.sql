-- Schema baseline: replaces migrations 0001-0065, generated from their combined
-- result with pg_dump. Databases migrated before this baseline must be reset.

CREATE COLLATION public.permission_template_names (provider = icu, locale = 'und');

CREATE TYPE public.budget_kind AS ENUM (
    'none',
    'amount',
    'hours'
);

CREATE TYPE public.entry_state AS ENUM (
    'open',
    'submitted',
    'approved',
    'invoiced'
);

CREATE TYPE public.harvest_entity_type AS ENUM (
    'client',
    'project',
    'task',
    'time_entry',
    'person'
);

CREATE TYPE public.invoice_status AS ENUM (
    'draft',
    'sent',
    'paid',
    'void'
);

CREATE TYPE public.project_role AS ENUM (
    'lead',
    'freelancer',
    'admin'
);

CREATE TYPE public.project_type AS ENUM (
    'time_and_materials',
    'fixed_fee',
    'non_billable',
    'retainer'
);

CREATE TYPE public.round_dir AS ENUM (
    'nearest',
    'up',
    'down'
);

CREATE FUNCTION public.advance_invoice_edit_revision() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
  IF OLD IS DISTINCT FROM NEW THEN
    NEW.edit_revision := OLD.edit_revision + 1;
  END IF;
  RETURN NEW;
END;
$$;

CREATE FUNCTION public.advance_project_edit_revision() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
  IF OLD IS DISTINCT FROM NEW THEN
    NEW.edit_revision := OLD.edit_revision + 1;
  END IF;
  RETURN NEW;
END;
$$;

CREATE FUNCTION public.effective_minutes(minutes integer, frozen integer, inc smallint, dir public.round_dir) RETURNS integer
    LANGUAGE sql IMMUTABLE PARALLEL SAFE
    AS $$
  SELECT COALESCE(frozen, CASE
    WHEN inc <= 0 THEN minutes
    WHEN dir = 'down' THEN minutes - minutes % inc
    WHEN dir = 'up' THEN ((minutes::bigint + inc - 1) / inc * inc)::integer
    ELSE ((minutes::bigint / inc + CASE WHEN (minutes % inc) * 2 >= inc THEN 1 ELSE 0 END) * inc)::integer
  END)
$$;

CREATE FUNCTION public.fetch_csv_export_rows(max_rows integer) RETURNS SETOF record
    LANGUAGE plpgsql
    AS $$
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

CREATE FUNCTION public.harvest_id_assign() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
  IF NEW.harvest_id IS NULL THEN
    NEW.harvest_id := harvest_id_next(
      TG_RELID::regclass, TG_ARGV[0]::regclass, TG_ARGV[1]::harvest_entity_type, NEW.org_id
    );
  END IF;
  RETURN NEW;
END
$$;

CREATE FUNCTION public.harvest_id_next(p_table regclass, p_sequence regclass, p_type public.harvest_entity_type, p_org uuid) RETURNS bigint
    LANGUAGE plpgsql
    AS $_$
DECLARE
  candidate bigint;
  taken boolean;
BEGIN
  LOOP
    candidate := nextval(p_sequence);
    EXECUTE format('SELECT EXISTS (SELECT 1 FROM %s WHERE org_id = $1 AND harvest_id = $2)', p_table)
      INTO taken USING p_org, candidate;
    EXIT WHEN NOT taken AND NOT EXISTS (
      SELECT 1 FROM harvest_import_map
      WHERE org_id = p_org AND harvest_entity_type = p_type AND harvest_import_id = candidate
    );
  END LOOP;
  RETURN candidate;
END
$_$;

CREATE FUNCTION public.harvest_norm(t text) RETURNS text
    LANGUAGE sql IMMUTABLE
    AS $$
    SELECT translate(
      btrim(t, E' \t\n\x0B\f\r '),
      'ABCDEFGHIJKLMNOPQRSTUVWXYZ',
      'abcdefghijklmnopqrstuvwxyz'
    )
  $$;

CREATE FUNCTION public.invalidate_invoice_editor() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
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

CREATE FUNCTION public.invalidate_project_editor() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
  IF TG_OP = 'UPDATE' AND OLD IS NOT DISTINCT FROM NEW THEN
    RETURN NEW;
  END IF;
  IF TG_OP = 'DELETE' THEN
    UPDATE projects SET edit_revision = edit_revision + 1 WHERE id = OLD.project_id;
    RETURN OLD;
  END IF;
  IF TG_OP = 'UPDATE' AND OLD.project_id IS DISTINCT FROM NEW.project_id THEN
    UPDATE projects SET edit_revision = edit_revision + 1 WHERE id = OLD.project_id;
  END IF;
  UPDATE projects SET edit_revision = edit_revision + 1 WHERE id = NEW.project_id;
  RETURN NEW;
END;
$$;

CREATE FUNCTION public.line_amount_cents(rate_cents bigint, minutes integer) RETURNS bigint
    LANGUAGE sql IMMUTABLE STRICT PARALLEL SAFE
    AS $$
    SELECT div(rate_cents::numeric * minutes + 30, 60)::bigint
  $$;

CREATE FUNCTION public.resolve_project_rate(mode text, task_rate bigint, assignment_rate bigint, project_rate bigint, user_rate bigint, client_rate bigint) RETURNS bigint
    LANGUAGE sql IMMUTABLE PARALLEL SAFE
    AS $$
    SELECT CASE COALESCE(mode, 'legacy')
      WHEN 'legacy' THEN COALESCE(task_rate, assignment_rate, project_rate, user_rate)
      WHEN 'person' THEN COALESCE(assignment_rate, user_rate, client_rate)
      WHEN 'task' THEN COALESCE(task_rate, client_rate)
      WHEN 'project' THEN project_rate
      ELSE NULL
    END
  $$;

CREATE FUNCTION public.set_updated_at() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
  NEW.updated_at = now();
  RETURN NEW;
END
$$;

CREATE FUNCTION public.time_entries_minutes_daily_cap() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
  IF NEW.minutes > 14400
     AND (TG_OP = 'INSERT' OR NEW.minutes > OLD.minutes) THEN
    RAISE EXCEPTION 'Daily hours cannot exceed 240.'
      USING ERRCODE = 'check_violation';
  END IF;
  RETURN NEW;
END
$$;

CREATE TABLE public.approvals (
    id uuid NOT NULL,
    org_id uuid NOT NULL,
    user_id uuid NOT NULL,
    period_start date NOT NULL,
    period_end date NOT NULL,
    state public.entry_state DEFAULT 'submitted'::public.entry_state NOT NULL,
    submitted_at timestamp with time zone DEFAULT now() NOT NULL,
    approved_by uuid,
    approved_at timestamp with time zone
);

CREATE TABLE public.assignments (
    id uuid NOT NULL,
    project_id uuid NOT NULL,
    user_id uuid NOT NULL,
    role public.project_role DEFAULT 'freelancer'::public.project_role NOT NULL,
    rate_cents bigint,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT assignments_rate_cents_nonneg CHECK (((rate_cents IS NULL) OR (rate_cents >= 0)))
);

CREATE TABLE public.audit_log (
    id uuid NOT NULL,
    org_id uuid NOT NULL,
    actor_user_id uuid,
    action text NOT NULL,
    entity_type text NOT NULL,
    entity_id uuid,
    data jsonb,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);

CREATE TABLE public.clients (
    id uuid NOT NULL,
    org_id uuid NOT NULL,
    name text NOT NULL,
    currency character(3) NOT NULL,
    address text,
    tax_id text,
    active boolean DEFAULT true NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    default_rate_cents bigint,
    harvest_id bigint NOT NULL,
    CONSTRAINT clients_currency_iso3 CHECK ((char_length(currency) = 3)),
    CONSTRAINT clients_default_rate_cents_check CHECK (((default_rate_cents IS NULL) OR (default_rate_cents >= 0)))
);

COMMENT ON COLUMN public.clients.harvest_id IS 'The id in the Harvest-compatible API; Horae''s identity is id (UUID v7).';

CREATE SEQUENCE public.clients_harvest_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

CREATE TABLE public.harvest_account_bindings (
    org_id uuid NOT NULL,
    harvest_account_id text NOT NULL
);

CREATE TABLE public.harvest_connection_generations (
    org_id uuid NOT NULL,
    account_generation bigint DEFAULT 0 NOT NULL,
    connection_revision bigint DEFAULT 0 NOT NULL,
    CONSTRAINT harvest_connection_generations_account_generation_check CHECK ((account_generation >= 0)),
    CONSTRAINT harvest_connection_generations_connection_revision_check CHECK ((connection_revision >= 0))
);

CREATE TABLE public.harvest_credentials (
    id uuid NOT NULL,
    org_id uuid NOT NULL,
    harvest_account_id text NOT NULL,
    access_token_enc bytea NOT NULL,
    refresh_token_enc bytea NOT NULL,
    token_expires_at timestamp with time zone,
    scope text,
    synced_watermark jsonb DEFAULT '{}'::jsonb NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);

CREATE TABLE public.harvest_import_map (
    org_id uuid NOT NULL,
    harvest_entity_type public.harvest_entity_type NOT NULL,
    harvest_import_id bigint NOT NULL,
    horae_id uuid NOT NULL,
    harvest_updated_at timestamp with time zone,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    horae_updated_at timestamp with time zone
);

CREATE TABLE public.harvest_oauth_clients (
    id uuid NOT NULL,
    org_id uuid NOT NULL,
    client_id text NOT NULL,
    client_secret_enc bytea NOT NULL,
    redirect_url text,
    updated_by uuid,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT harvest_oauth_clients_client_id_check CHECK (((length(client_id) >= 1) AND (length(client_id) <= 255)))
);

CREATE TABLE public.horae_job_report_error_chunks (
    id uuid NOT NULL,
    job_id uuid NOT NULL,
    org_id uuid NOT NULL,
    sequence bigint NOT NULL,
    schema_version smallint DEFAULT 1 NOT NULL,
    body bytea NOT NULL,
    CONSTRAINT horae_job_report_error_chunks_body_check CHECK (((octet_length(body) >= 1) AND (octet_length(body) <= 65536))),
    CONSTRAINT horae_job_report_error_chunks_schema_version_check CHECK ((schema_version = 1)),
    CONSTRAINT horae_job_report_error_chunks_sequence_check CHECK ((sequence >= 0))
);

CREATE TABLE public.horae_job_uploads (
    job_id uuid NOT NULL,
    org_id uuid NOT NULL,
    filename text NOT NULL,
    content_type text NOT NULL,
    body bytea NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT horae_job_uploads_body_check CHECK ((octet_length(body) <= ((50 * 1024) * 1024)))
);

CREATE TABLE public.horae_jobs (
    id uuid NOT NULL,
    org_id uuid NOT NULL,
    kind text NOT NULL,
    payload jsonb NOT NULL,
    status text DEFAULT 'queued'::text NOT NULL,
    idempotency_key text,
    attempts integer DEFAULT 0 NOT NULL,
    max_attempts integer DEFAULT 5 NOT NULL,
    available_at timestamp with time zone DEFAULT now() NOT NULL,
    lease_until timestamp with time zone,
    worker_id text,
    phase text,
    processed_count bigint DEFAULT 0 NOT NULL,
    total_count bigint,
    checkpoint jsonb,
    report jsonb,
    last_error text,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    started_at timestamp with time zone,
    finished_at timestamp with time zone,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    claim_token uuid,
    cancellation_requested boolean DEFAULT false NOT NULL,
    account_generation bigint DEFAULT 0 NOT NULL,
    original_requester_id uuid,
    error_kind text,
    CONSTRAINT horae_jobs_account_generation_check CHECK ((account_generation >= 0)),
    CONSTRAINT horae_jobs_attempts_check CHECK ((attempts >= 0)),
    CONSTRAINT horae_jobs_checkpoint_report_budget CHECK ((octet_length(((checkpoint -> 'report'::text))::text) <= 16384)),
    CONSTRAINT horae_jobs_error_kind_check CHECK ((error_kind = ANY (ARRAY['reconnect_required'::text, 'forbidden'::text, 'unavailable'::text]))),
    CONSTRAINT horae_jobs_max_attempts_check CHECK ((max_attempts > 0)),
    CONSTRAINT horae_jobs_processed_count_check CHECK ((processed_count >= 0)),
    CONSTRAINT horae_jobs_report_budget CHECK ((octet_length((report)::text) <= 16384)),
    CONSTRAINT horae_jobs_status_check CHECK ((status = ANY (ARRAY['queued'::text, 'running'::text, 'succeeded'::text, 'failed'::text, 'cancelled'::text])))
);

CREATE TABLE public.horae_outbox (
    id uuid NOT NULL,
    org_id uuid NOT NULL,
    event_kind text NOT NULL,
    payload jsonb NOT NULL,
    attempts integer DEFAULT 0 NOT NULL,
    available_at timestamp with time zone DEFAULT now() NOT NULL,
    delivered_at timestamp with time zone,
    last_error text,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    claim_token uuid,
    failed_at timestamp with time zone,
    CONSTRAINT horae_outbox_attempts_check CHECK ((attempts >= 0)),
    CONSTRAINT outbox_terminal_state CHECK (((delivered_at IS NULL) OR (failed_at IS NULL)))
);

CREATE TABLE public.invoice_edit_requests (
    id uuid NOT NULL,
    org_id uuid NOT NULL,
    actor_id uuid NOT NULL,
    invoice_id uuid NOT NULL,
    completed_revision bigint NOT NULL,
    payload jsonb NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT invoice_edit_requests_completed_revision_check CHECK ((completed_revision > 0)),
    CONSTRAINT invoice_edit_requests_payload_check CHECK (((jsonb_typeof(payload) = 'object'::text) AND (octet_length((payload)::text) <= 262144)))
);

CREATE TABLE public.invoice_generation_requests (
    id uuid NOT NULL,
    org_id uuid NOT NULL,
    actor_id uuid NOT NULL,
    invoice_id uuid NOT NULL,
    payload jsonb NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT invoice_generation_requests_payload_check CHECK (((jsonb_typeof(payload) = 'object'::text) AND (octet_length((payload)::text) <= 262144)))
);

CREATE TABLE public.invoice_line_items (
    id uuid NOT NULL,
    invoice_id uuid NOT NULL,
    time_entry_id uuid,
    description text NOT NULL,
    minutes integer,
    rate_cents bigint,
    amount_cents bigint NOT NULL,
    fee_occurrence_id uuid,
    allocated_discount_cents bigint DEFAULT 0 NOT NULL,
    net_before_tax_cents bigint GENERATED ALWAYS AS ((amount_cents - allocated_discount_cents)) STORED,
    CONSTRAINT invoice_line_discount_bounds CHECK (((allocated_discount_cents >= 0) AND (allocated_discount_cents <= amount_cents))),
    CONSTRAINT invoice_line_items_amount_cents_nonneg CHECK ((amount_cents >= 0)),
    CONSTRAINT invoice_line_items_exclusive_source CHECK ((((time_entry_id IS NOT NULL) AND (fee_occurrence_id IS NULL) AND (minutes IS NOT NULL) AND (rate_cents IS NOT NULL)) OR ((time_entry_id IS NULL) AND (fee_occurrence_id IS NOT NULL) AND (minutes IS NULL) AND (rate_cents IS NULL)))),
    CONSTRAINT invoice_line_items_minutes_nonneg CHECK ((minutes >= 0)),
    CONSTRAINT invoice_line_items_rate_cents_nonneg CHECK ((rate_cents >= 0))
);

CREATE TABLE public.invoices (
    id uuid NOT NULL,
    org_id uuid NOT NULL,
    client_id uuid NOT NULL,
    number text NOT NULL,
    status public.invoice_status DEFAULT 'draft'::public.invoice_status NOT NULL,
    issued_on date NOT NULL,
    due_on date NOT NULL,
    currency character(3) NOT NULL,
    total_cents bigint DEFAULT 0 NOT NULL,
    notes text,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    po_number text DEFAULT ''::text NOT NULL,
    discount_bps smallint DEFAULT 0 NOT NULL,
    tax1_bps smallint DEFAULT 0 NOT NULL,
    tax2_name text,
    tax2_bps smallint,
    discount_cents bigint DEFAULT 0 NOT NULL,
    tax1_cents bigint DEFAULT 0 NOT NULL,
    tax2_cents bigint DEFAULT 0 NOT NULL,
    terms_days integer GENERATED ALWAYS AS ((due_on - issued_on)) STORED NOT NULL,
    subtotal_cents bigint GENERATED ALWAYS AS ((((((total_cents)::numeric + (discount_cents)::numeric) - (tax1_cents)::numeric) - (tax2_cents)::numeric))::bigint) STORED NOT NULL,
    edit_revision bigint DEFAULT 1 NOT NULL,
    CONSTRAINT invoice_discount_exact CHECK (((discount_cents)::numeric = floor(((((subtotal_cents)::numeric * (discount_bps)::numeric) + (5000)::numeric) / (10000)::numeric)))),
    CONSTRAINT invoice_second_tax_pair CHECK (((tax2_name IS NULL) = (tax2_bps IS NULL))),
    CONSTRAINT invoice_subtotal_nonnegative CHECK ((subtotal_cents >= 0)),
    CONSTRAINT invoice_tax1_exact CHECK (((tax1_cents)::numeric = floor((((((subtotal_cents)::numeric - (discount_cents)::numeric) * (tax1_bps)::numeric) + (5000)::numeric) / (10000)::numeric)))),
    CONSTRAINT invoice_tax2_exact CHECK (((tax2_cents)::numeric = floor((((((subtotal_cents)::numeric - (discount_cents)::numeric) * (COALESCE((tax2_bps)::integer, 0))::numeric) + (5000)::numeric) / (10000)::numeric)))),
    CONSTRAINT invoices_currency_iso3 CHECK ((char_length(currency) = 3)),
    CONSTRAINT invoices_discount_bps_check CHECK (((discount_bps >= 0) AND (discount_bps <= 10000))),
    CONSTRAINT invoices_discount_cents_check CHECK ((discount_cents >= 0)),
    CONSTRAINT invoices_edit_revision_check CHECK ((edit_revision > 0)),
    CONSTRAINT invoices_po_number_check CHECK ((char_length(po_number) <= 200)),
    CONSTRAINT invoices_tax1_bps_check CHECK (((tax1_bps >= 0) AND (tax1_bps <= 10000))),
    CONSTRAINT invoices_tax1_cents_check CHECK ((tax1_cents >= 0)),
    CONSTRAINT invoices_tax2_bps_check CHECK (((tax2_bps >= 0) AND (tax2_bps <= 10000))),
    CONSTRAINT invoices_tax2_cents_check CHECK ((tax2_cents >= 0)),
    CONSTRAINT invoices_tax2_name_check CHECK (((char_length(btrim(tax2_name)) >= 1) AND (char_length(btrim(tax2_name)) <= 100))),
    CONSTRAINT invoices_total_cents_nonneg CHECK ((total_cents >= 0))
);

CREATE TABLE public.organizations (
    id uuid NOT NULL,
    name text NOT NULL,
    default_currency character(3) DEFAULT 'EUR'::bpchar NOT NULL,
    week_start smallint DEFAULT 1 NOT NULL,
    round_minutes smallint DEFAULT 0 NOT NULL,
    round_dir public.round_dir DEFAULT 'nearest'::public.round_dir NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    provider_name text,
    provider_address text,
    provider_tax_id text,
    provider_email text,
    provider_phone text,
    bank_name text,
    bank_iban text,
    bank_bic text,
    bank_routing text,
    bank_account text,
    invoice_notes text,
    invoice_payment_terms text DEFAULT 'Net 30'::text,
    budget_alert_pcts integer[] DEFAULT '{80,100}'::integer[] NOT NULL,
    long_timer_minutes integer DEFAULT 480 NOT NULL,
    access_revision bigint DEFAULT 0 NOT NULL,
    timezone text DEFAULT 'UTC'::text NOT NULL,
    api_account_id bigint NOT NULL,
    CONSTRAINT organizations_access_revision_check CHECK ((access_revision >= 0)),
    CONSTRAINT organizations_default_currency_iso3 CHECK ((char_length(default_currency) = 3)),
    CONSTRAINT organizations_long_timer_minutes_nonneg CHECK ((long_timer_minutes >= 0)),
    CONSTRAINT organizations_round_minutes_nonneg CHECK ((round_minutes >= 0))
);

ALTER TABLE public.organizations ALTER COLUMN api_account_id ADD GENERATED ALWAYS AS IDENTITY (
    SEQUENCE NAME public.organizations_api_account_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1
);

CREATE TABLE public.permission_change_receipts (
    id uuid NOT NULL,
    org_id uuid NOT NULL,
    actor_user_id uuid,
    operator_id text,
    operator_command text,
    request_id uuid NOT NULL,
    format_version integer NOT NULL,
    intent jsonb NOT NULL,
    result jsonb NOT NULL,
    audit jsonb NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT permission_change_receipts_audit_check CHECK ((jsonb_typeof(audit) = 'object'::text)),
    CONSTRAINT permission_change_receipts_check CHECK ((((actor_user_id IS NOT NULL) AND (operator_id IS NULL) AND (operator_command IS NULL)) OR ((actor_user_id IS NULL) AND (operator_id IS NOT NULL) AND (operator_command IS NOT NULL) AND (btrim(operator_id) <> ''::text) AND (btrim(operator_command) <> ''::text)))),
    CONSTRAINT permission_change_receipts_format_version_check CHECK ((format_version > 0)),
    CONSTRAINT permission_change_receipts_intent_check CHECK ((jsonb_typeof(intent) = 'object'::text)),
    CONSTRAINT permission_change_receipts_result_check CHECK ((jsonb_typeof(result) = 'object'::text))
);

CREATE TABLE public.permission_templates (
    id uuid NOT NULL,
    org_id uuid NOT NULL,
    name text NOT NULL,
    catalog_version integer NOT NULL,
    grants text[] NOT NULL,
    revision bigint DEFAULT 0 NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT permission_templates_catalog_version_check CHECK ((catalog_version > 0)),
    CONSTRAINT permission_templates_grants_check CHECK ((array_position(grants, NULL::text) IS NULL)),
    CONSTRAINT permission_templates_name_check CHECK (((char_length(name) >= 1) AND (char_length(name) <= 100))),
    CONSTRAINT permission_templates_revision_check CHECK ((revision >= 0))
);

CREATE TABLE public.person_management_assignments (
    id uuid NOT NULL,
    org_id uuid NOT NULL,
    manager_id uuid NOT NULL,
    managed_user_id uuid NOT NULL,
    revision bigint DEFAULT 0 NOT NULL,
    CONSTRAINT person_management_assignments_check CHECK ((manager_id <> managed_user_id)),
    CONSTRAINT person_management_assignments_revision_check CHECK ((revision >= 0))
);

CREATE TABLE public.person_permission_states (
    id uuid NOT NULL,
    org_id uuid NOT NULL,
    user_id uuid NOT NULL,
    catalog_version integer NOT NULL,
    grants text[] NOT NULL,
    is_administrator boolean NOT NULL,
    source text NOT NULL,
    built_in_profile text,
    template_id uuid,
    applied_template_revision bigint,
    revision bigint DEFAULT 0 NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT person_permission_states_applied_template_revision_check CHECK ((applied_template_revision >= 0)),
    CONSTRAINT person_permission_states_catalog_version_check CHECK ((catalog_version > 0)),
    CONSTRAINT person_permission_states_check CHECK ((((source = 'built_in'::text) AND (built_in_profile IS NOT NULL) AND (built_in_profile = ANY (ARRAY['member'::text, 'project_manager'::text, 'people_admin'::text, 'accounting'::text, 'executive_manager'::text, 'administrator'::text])) AND (template_id IS NULL) AND (applied_template_revision IS NULL)) OR ((source = 'template'::text) AND (built_in_profile IS NULL) AND (template_id IS NOT NULL) AND (applied_template_revision IS NOT NULL)) OR ((source = 'individual'::text) AND (built_in_profile IS NULL) AND (template_id IS NULL) AND (applied_template_revision IS NULL)))),
    CONSTRAINT person_permission_states_grants_check CHECK ((array_position(grants, NULL::text) IS NULL)),
    CONSTRAINT person_permission_states_revision_check CHECK ((revision >= 0))
);

CREATE TABLE public.project_budget_notifications (
    id uuid NOT NULL,
    org_id uuid NOT NULL,
    project_id uuid NOT NULL,
    task_id uuid,
    user_id uuid,
    recipient_id uuid NOT NULL,
    period_key text NOT NULL,
    threshold smallint NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT project_budget_notifications_check CHECK (((task_id IS NULL) OR (user_id IS NULL))),
    CONSTRAINT project_budget_notifications_period_key_check CHECK (((period_key = 'lifetime'::text) OR (period_key ~ '^[0-9]{4}-[0-9]{2}$'::text))),
    CONSTRAINT project_budget_notifications_threshold_check CHECK (((threshold >= 0) AND (threshold <= 100)))
);

CREATE TABLE public.project_drafts (
    id uuid NOT NULL,
    org_id uuid NOT NULL,
    creator_id uuid NOT NULL,
    revision bigint DEFAULT 1 NOT NULL,
    payload_version smallint DEFAULT 1 NOT NULL,
    payload jsonb NOT NULL,
    completed_project_id uuid,
    discarded_at timestamp with time zone,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT project_drafts_check CHECK (((completed_project_id IS NULL) OR (discarded_at IS NULL))),
    CONSTRAINT project_drafts_payload_check CHECK (((jsonb_typeof(payload) = 'object'::text) AND (octet_length((payload)::text) <= 262144))),
    CONSTRAINT project_drafts_payload_version_check CHECK ((payload_version = 1)),
    CONSTRAINT project_drafts_revision_check CHECK ((revision > 0))
);

CREATE TABLE public.project_edit_requests (
    id uuid NOT NULL,
    org_id uuid NOT NULL,
    project_id uuid NOT NULL,
    actor_id uuid NOT NULL,
    expected_revision bigint NOT NULL,
    completed_revision bigint NOT NULL,
    payload jsonb NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT project_edit_requests_completed_revision_check CHECK ((completed_revision > 0)),
    CONSTRAINT project_edit_requests_expected_revision_check CHECK ((expected_revision > 0)),
    CONSTRAINT project_edit_requests_payload_check CHECK (((jsonb_typeof(payload) = 'object'::text) AND (octet_length((payload)::text) <= 262144)))
);

CREATE TABLE public.project_fee_milestones (
    id uuid NOT NULL,
    org_id uuid NOT NULL,
    project_id uuid NOT NULL,
    name text NOT NULL,
    due_on date NOT NULL,
    amount_cents bigint NOT NULL,
    "position" smallint NOT NULL,
    CONSTRAINT project_fee_milestones_amount_cents_check CHECK ((amount_cents >= 0)),
    CONSTRAINT project_fee_milestones_name_check CHECK (((char_length(btrim(name)) >= 1) AND (char_length(btrim(name)) <= 200))),
    CONSTRAINT project_fee_milestones_position_check CHECK ((("position" >= 0) AND ("position" <= 99)))
);

CREATE TABLE public.project_fee_occurrences (
    id uuid NOT NULL,
    org_id uuid NOT NULL,
    project_id uuid NOT NULL,
    milestone_id uuid,
    period_key text NOT NULL,
    due_on date NOT NULL,
    description text NOT NULL,
    amount_cents bigint NOT NULL,
    currency character(3) NOT NULL,
    CONSTRAINT project_fee_occurrences_amount_cents_check CHECK ((amount_cents >= 0)),
    CONSTRAINT project_fee_occurrences_currency_check CHECK ((currency ~ '^[A-Z]{3}$'::text)),
    CONSTRAINT project_fee_occurrences_period_key_check CHECK (((char_length(period_key) >= 1) AND (char_length(period_key) <= 100)))
);

CREATE TABLE public.project_management_assignments (
    id uuid NOT NULL,
    org_id uuid NOT NULL,
    manager_id uuid NOT NULL,
    project_id uuid NOT NULL,
    revision bigint DEFAULT 0 NOT NULL,
    CONSTRAINT project_management_assignments_revision_check CHECK ((revision >= 0))
);

CREATE TABLE public.project_member_budgets (
    id uuid NOT NULL,
    org_id uuid NOT NULL,
    project_id uuid NOT NULL,
    user_id uuid NOT NULL,
    budget_minutes bigint NOT NULL,
    CONSTRAINT project_member_budgets_budget_minutes_check CHECK ((budget_minutes >= 0))
);

CREATE TABLE public.project_member_costs (
    id uuid NOT NULL,
    org_id uuid NOT NULL,
    project_id uuid NOT NULL,
    user_id uuid NOT NULL,
    cost_rate_cents bigint NOT NULL,
    CONSTRAINT project_member_costs_cost_rate_cents_check CHECK ((cost_rate_cents >= 0))
);

CREATE TABLE public.project_private_settings (
    id uuid NOT NULL,
    org_id uuid NOT NULL,
    project_id uuid NOT NULL,
    admin_notes text DEFAULT ''::text NOT NULL,
    CONSTRAINT project_private_settings_admin_notes_check CHECK ((char_length(admin_notes) <= 10000))
);

CREATE TABLE public.project_settings (
    id uuid NOT NULL,
    org_id uuid NOT NULL,
    project_id uuid NOT NULL,
    creator_id uuid NOT NULL,
    rate_mode text NOT NULL,
    budget_scope text DEFAULT 'project'::text NOT NULL,
    monthly_reset boolean DEFAULT false NOT NULL,
    include_nonbillable boolean DEFAULT false NOT NULL,
    alert_enabled boolean DEFAULT false NOT NULL,
    alert_threshold smallint DEFAULT 80 NOT NULL,
    report_visibility text DEFAULT 'managers'::text NOT NULL,
    fee_mode text,
    fee_amount_cents bigint,
    monthly_day text,
    terms_days smallint DEFAULT 30 NOT NULL,
    po_number text DEFAULT ''::text NOT NULL,
    discount_bps smallint DEFAULT 0 NOT NULL,
    tax1_bps smallint DEFAULT 0 NOT NULL,
    tax2_name text,
    tax2_bps smallint,
    CONSTRAINT project_settings_alert_threshold_check CHECK (((alert_threshold >= 0) AND (alert_threshold <= 100))),
    CONSTRAINT project_settings_budget_scope_check CHECK ((budget_scope = ANY (ARRAY['project'::text, 'task'::text, 'person'::text]))),
    CONSTRAINT project_settings_check CHECK (((tax2_name IS NULL) = (tax2_bps IS NULL))),
    CONSTRAINT project_settings_check1 CHECK (
CASE fee_mode
    WHEN 'single'::text THEN ((fee_amount_cents IS NOT NULL) AND (monthly_day IS NULL))
    WHEN 'milestones'::text THEN ((fee_amount_cents IS NULL) AND (monthly_day IS NULL))
    WHEN 'monthly'::text THEN ((fee_amount_cents IS NOT NULL) AND (monthly_day IS NOT NULL))
    ELSE ((fee_amount_cents IS NULL) AND (monthly_day IS NULL))
END),
    CONSTRAINT project_settings_discount_bps_check CHECK (((discount_bps >= 0) AND (discount_bps <= 10000))),
    CONSTRAINT project_settings_fee_amount_cents_check CHECK ((fee_amount_cents >= 0)),
    CONSTRAINT project_settings_fee_mode_check CHECK ((fee_mode = ANY (ARRAY['single'::text, 'milestones'::text, 'monthly'::text]))),
    CONSTRAINT project_settings_monthly_day_check CHECK ((monthly_day = ANY (ARRAY['first'::text, 'fifteenth'::text, 'last'::text]))),
    CONSTRAINT project_settings_po_number_check CHECK ((char_length(po_number) <= 200)),
    CONSTRAINT project_settings_rate_mode_check CHECK ((rate_mode = ANY (ARRAY['person'::text, 'task'::text, 'project'::text]))),
    CONSTRAINT project_settings_report_visibility_check CHECK ((report_visibility = ANY (ARRAY['managers'::text, 'project_members'::text]))),
    CONSTRAINT project_settings_tax1_bps_check CHECK (((tax1_bps >= 0) AND (tax1_bps <= 10000))),
    CONSTRAINT project_settings_tax2_bps_check CHECK (((tax2_bps >= 0) AND (tax2_bps <= 10000))),
    CONSTRAINT project_settings_tax2_name_check CHECK (((char_length(btrim(tax2_name)) >= 1) AND (char_length(btrim(tax2_name)) <= 100))),
    CONSTRAINT project_settings_terms_days_check CHECK (((terms_days >= 0) AND (terms_days <= 365)))
);

CREATE TABLE public.project_tag_links (
    id uuid NOT NULL,
    org_id uuid NOT NULL,
    project_id uuid NOT NULL,
    tag_id uuid NOT NULL
);

CREATE TABLE public.project_tags (
    id uuid NOT NULL,
    org_id uuid NOT NULL,
    name text NOT NULL,
    CONSTRAINT project_tags_name_check CHECK (((char_length(btrim(name)) >= 1) AND (char_length(btrim(name)) <= 50)))
);

CREATE TABLE public.project_task_members (
    id uuid NOT NULL,
    org_id uuid NOT NULL,
    project_id uuid NOT NULL,
    task_id uuid NOT NULL,
    user_id uuid NOT NULL
);

CREATE TABLE public.project_task_settings (
    id uuid NOT NULL,
    org_id uuid NOT NULL,
    project_id uuid NOT NULL,
    task_id uuid NOT NULL,
    restricted boolean DEFAULT false NOT NULL,
    budget_minutes bigint,
    budget_cents bigint,
    CONSTRAINT project_task_settings_budget_cents_check CHECK ((budget_cents >= 0)),
    CONSTRAINT project_task_settings_budget_minutes_check CHECK ((budget_minutes >= 0)),
    CONSTRAINT project_task_settings_check CHECK (((budget_minutes IS NULL) OR (budget_cents IS NULL)))
);

CREATE TABLE public.project_tasks (
    project_id uuid NOT NULL,
    task_id uuid NOT NULL,
    billable boolean NOT NULL,
    rate_cents bigint,
    active boolean DEFAULT true NOT NULL,
    CONSTRAINT project_tasks_rate_cents_nonneg CHECK (((rate_cents IS NULL) OR (rate_cents >= 0)))
);

CREATE TABLE public.projects (
    id uuid NOT NULL,
    org_id uuid NOT NULL,
    client_id uuid NOT NULL,
    code text,
    name text NOT NULL,
    project_type public.project_type DEFAULT 'time_and_materials'::public.project_type NOT NULL,
    currency character(3) NOT NULL,
    starts_on date,
    ends_on date,
    budget_kind public.budget_kind DEFAULT 'none'::public.budget_kind NOT NULL,
    budget_amount_cents bigint,
    budget_minutes bigint,
    active boolean DEFAULT true NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    last_budget_alert_pct integer,
    rate_cents bigint,
    edit_revision bigint DEFAULT 1 NOT NULL,
    harvest_id bigint NOT NULL,
    CONSTRAINT projects_budget_amount_cents_nonneg CHECK (((budget_amount_cents IS NULL) OR (budget_amount_cents >= 0))),
    CONSTRAINT projects_budget_minutes_nonneg CHECK (((budget_minutes IS NULL) OR (budget_minutes >= 0))),
    CONSTRAINT projects_currency_iso3 CHECK ((char_length(currency) = 3)),
    CONSTRAINT projects_edit_revision_check CHECK ((edit_revision > 0)),
    CONSTRAINT projects_rate_cents_check CHECK (((rate_cents IS NULL) OR (rate_cents >= 0)))
);

COMMENT ON COLUMN public.projects.harvest_id IS 'The id in the Harvest-compatible API; Horae''s identity is id (UUID v7).';

CREATE SEQUENCE public.projects_harvest_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

CREATE TABLE public.tasks (
    id uuid NOT NULL,
    org_id uuid NOT NULL,
    name text NOT NULL,
    billable_default boolean DEFAULT true NOT NULL,
    default_rate_cents bigint,
    active boolean DEFAULT true NOT NULL,
    default_rate_currency text,
    is_default boolean DEFAULT false NOT NULL,
    harvest_id bigint NOT NULL,
    CONSTRAINT tasks_default_rate_cents_nonneg CHECK (((default_rate_cents IS NULL) OR (default_rate_cents >= 0))),
    CONSTRAINT tasks_default_rate_currency_check CHECK (((default_rate_currency IS NULL) OR (default_rate_currency ~ '^[A-Z]{3}$'::text)))
);

COMMENT ON COLUMN public.tasks.harvest_id IS 'The id in the Harvest-compatible API; Horae''s identity is id (UUID v7).';

CREATE SEQUENCE public.tasks_harvest_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

CREATE TABLE public.time_entries (
    id uuid NOT NULL,
    org_id uuid NOT NULL,
    user_id uuid NOT NULL,
    project_id uuid NOT NULL,
    task_id uuid NOT NULL,
    spent_date date NOT NULL,
    minutes integer DEFAULT 0 NOT NULL,
    rounded_minutes integer,
    notes text,
    billable boolean NOT NULL,
    is_running boolean DEFAULT false NOT NULL,
    started_at timestamp with time zone,
    state public.entry_state DEFAULT 'open'::public.entry_state NOT NULL,
    invoice_id uuid,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    notified_long_running_at timestamp with time zone,
    start_minute integer,
    sort_order integer DEFAULT 0 NOT NULL,
    harvest_id bigint NOT NULL,
    CONSTRAINT time_entries_minutes_nonneg CHECK ((minutes >= 0)),
    CONSTRAINT time_entries_rounded_minutes_nonneg CHECK (((rounded_minutes IS NULL) OR (rounded_minutes >= 0))),
    CONSTRAINT time_entries_start_minute_range CHECK (((start_minute IS NULL) OR ((start_minute >= 0) AND (start_minute <= 1439)))),
    CONSTRAINT time_entries_within_day CHECK (((start_minute IS NULL) OR ((start_minute + minutes) <= 1440)))
);

COMMENT ON COLUMN public.time_entries.harvest_id IS 'The id in the Harvest-compatible API; Horae''s identity is id (UUID v7).';

CREATE SEQUENCE public.time_entries_harvest_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

CREATE TABLE public.users (
    id uuid NOT NULL,
    org_id uuid NOT NULL,
    email text NOT NULL,
    name text NOT NULL,
    oidc_subject text,
    cost_rate_cents bigint,
    billable_rate_cents bigint,
    active boolean DEFAULT true NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    oidc_issuer text,
    timezone text,
    harvest_id bigint NOT NULL,
    CONSTRAINT users_billable_rate_cents_nonneg CHECK (((billable_rate_cents IS NULL) OR (billable_rate_cents >= 0))),
    CONSTRAINT users_cost_rate_cents_nonneg CHECK (((cost_rate_cents IS NULL) OR (cost_rate_cents >= 0))),
    CONSTRAINT users_oidc_issuer_requires_subject CHECK (((oidc_issuer IS NULL) OR (oidc_subject IS NOT NULL)))
);

COMMENT ON COLUMN public.users.harvest_id IS 'The id in the Harvest-compatible API; Horae''s identity is id (UUID v7).';

CREATE VIEW public.time_entry_contexts AS
 SELECT u.id AS user_id,
    u.org_id,
    p.id AS project_id,
    t.id AS task_id,
    ((p.project_type <> 'non_billable'::public.project_type) AND pt.billable) AS billable
   FROM ((((((public.users u
     JOIN public.projects p ON ((p.org_id = u.org_id)))
     JOIN public.clients c ON (((c.id = p.client_id) AND (c.org_id = u.org_id))))
     JOIN public.project_tasks pt ON ((pt.project_id = p.id)))
     JOIN public.tasks t ON (((t.id = pt.task_id) AND (t.org_id = u.org_id))))
     LEFT JOIN public.assignments a ON (((a.project_id = p.id) AND (a.user_id = u.id))))
     LEFT JOIN public.project_task_settings s ON (((s.project_id = p.id) AND (s.task_id = t.id))))
  WHERE (u.active AND c.active AND p.active AND t.active AND pt.active AND ((a.id IS NOT NULL) OR (EXISTS ( SELECT 1
           FROM public.person_permission_states state
          WHERE ((state.org_id = u.org_id) AND (state.user_id = u.id) AND state.is_administrator)))) AND ((NOT COALESCE(s.restricted, false)) OR (EXISTS ( SELECT 1
           FROM public.project_task_members m
          WHERE ((m.project_id = p.id) AND (m.task_id = t.id) AND (m.user_id = u.id))))));

CREATE SEQUENCE public.users_harvest_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

ALTER TABLE ONLY public.approvals
    ADD CONSTRAINT approvals_pkey PRIMARY KEY (id);

ALTER TABLE ONLY public.approvals
    ADD CONSTRAINT approvals_user_id_period_start_key UNIQUE (user_id, period_start);

ALTER TABLE ONLY public.assignments
    ADD CONSTRAINT assignments_pkey PRIMARY KEY (id);

ALTER TABLE ONLY public.assignments
    ADD CONSTRAINT assignments_project_id_user_id_key UNIQUE (project_id, user_id);

ALTER TABLE ONLY public.audit_log
    ADD CONSTRAINT audit_log_pkey PRIMARY KEY (id);

ALTER TABLE ONLY public.clients
    ADD CONSTRAINT clients_pkey PRIMARY KEY (id);

ALTER TABLE ONLY public.harvest_account_bindings
    ADD CONSTRAINT harvest_account_bindings_pkey PRIMARY KEY (org_id);

ALTER TABLE ONLY public.harvest_connection_generations
    ADD CONSTRAINT harvest_connection_generations_pkey PRIMARY KEY (org_id);

ALTER TABLE ONLY public.harvest_credentials
    ADD CONSTRAINT harvest_credentials_org_id_key UNIQUE (org_id);

ALTER TABLE ONLY public.harvest_credentials
    ADD CONSTRAINT harvest_credentials_pkey PRIMARY KEY (id);

ALTER TABLE ONLY public.harvest_import_map
    ADD CONSTRAINT harvest_import_map_pkey PRIMARY KEY (org_id, harvest_entity_type, harvest_import_id);

ALTER TABLE ONLY public.harvest_oauth_clients
    ADD CONSTRAINT harvest_oauth_clients_org_id_key UNIQUE (org_id);

ALTER TABLE ONLY public.harvest_oauth_clients
    ADD CONSTRAINT harvest_oauth_clients_pkey PRIMARY KEY (id);

ALTER TABLE ONLY public.horae_job_report_error_chunks
    ADD CONSTRAINT horae_job_report_error_chunks_job_id_sequence_key UNIQUE (job_id, sequence);

ALTER TABLE ONLY public.horae_job_report_error_chunks
    ADD CONSTRAINT horae_job_report_error_chunks_pkey PRIMARY KEY (id);

ALTER TABLE ONLY public.horae_job_uploads
    ADD CONSTRAINT horae_job_uploads_pkey PRIMARY KEY (job_id);

ALTER TABLE ONLY public.horae_jobs
    ADD CONSTRAINT horae_jobs_id_org_unique UNIQUE (id, org_id);

ALTER TABLE ONLY public.horae_jobs
    ADD CONSTRAINT horae_jobs_org_id_kind_idempotency_key_key UNIQUE (org_id, kind, idempotency_key);

ALTER TABLE ONLY public.horae_jobs
    ADD CONSTRAINT horae_jobs_pkey PRIMARY KEY (id);

ALTER TABLE ONLY public.horae_outbox
    ADD CONSTRAINT horae_outbox_pkey PRIMARY KEY (id);

ALTER TABLE ONLY public.invoice_edit_requests
    ADD CONSTRAINT invoice_edit_requests_pkey PRIMARY KEY (id);

ALTER TABLE ONLY public.invoice_generation_requests
    ADD CONSTRAINT invoice_generation_requests_pkey PRIMARY KEY (id);

ALTER TABLE ONLY public.invoice_line_items
    ADD CONSTRAINT invoice_line_items_fee_unique UNIQUE (invoice_id, fee_occurrence_id);

ALTER TABLE ONLY public.invoice_line_items
    ADD CONSTRAINT invoice_line_items_invoice_id_time_entry_id_key UNIQUE (invoice_id, time_entry_id);

ALTER TABLE ONLY public.invoice_line_items
    ADD CONSTRAINT invoice_line_items_pkey PRIMARY KEY (id);

ALTER TABLE ONLY public.invoices
    ADD CONSTRAINT invoices_id_org_unique UNIQUE (id, org_id);

ALTER TABLE ONLY public.invoices
    ADD CONSTRAINT invoices_org_id_number_key UNIQUE (org_id, number);

ALTER TABLE ONLY public.invoices
    ADD CONSTRAINT invoices_pkey PRIMARY KEY (id);

ALTER TABLE ONLY public.project_fee_milestones
    ADD CONSTRAINT milestones_project_org_unique UNIQUE (id, project_id, org_id);

ALTER TABLE ONLY public.organizations
    ADD CONSTRAINT organizations_api_account_id_key UNIQUE (api_account_id);

ALTER TABLE ONLY public.organizations
    ADD CONSTRAINT organizations_pkey PRIMARY KEY (id);

ALTER TABLE ONLY public.permission_change_receipts
    ADD CONSTRAINT permission_change_receipts_pkey PRIMARY KEY (id);

ALTER TABLE ONLY public.permission_templates
    ADD CONSTRAINT permission_templates_org_id_id_key UNIQUE (org_id, id);

ALTER TABLE ONLY public.permission_templates
    ADD CONSTRAINT permission_templates_pkey PRIMARY KEY (id);

ALTER TABLE ONLY public.person_management_assignments
    ADD CONSTRAINT person_management_assignments_org_id_manager_id_managed_use_key UNIQUE (org_id, manager_id, managed_user_id);

ALTER TABLE ONLY public.person_management_assignments
    ADD CONSTRAINT person_management_assignments_pkey PRIMARY KEY (id);

ALTER TABLE ONLY public.person_permission_states
    ADD CONSTRAINT person_permission_states_org_id_user_id_key UNIQUE (org_id, user_id);

ALTER TABLE ONLY public.person_permission_states
    ADD CONSTRAINT person_permission_states_pkey PRIMARY KEY (id);

ALTER TABLE ONLY public.project_budget_notifications
    ADD CONSTRAINT project_budget_notifications_pkey PRIMARY KEY (id);

ALTER TABLE ONLY public.project_budget_notifications
    ADD CONSTRAINT project_budget_notifications_project_id_task_id_user_id_per_key UNIQUE NULLS NOT DISTINCT (project_id, task_id, user_id, period_key, threshold, recipient_id);

ALTER TABLE ONLY public.project_drafts
    ADD CONSTRAINT project_drafts_pkey PRIMARY KEY (id);

ALTER TABLE ONLY public.project_edit_requests
    ADD CONSTRAINT project_edit_requests_pkey PRIMARY KEY (id);

ALTER TABLE ONLY public.project_fee_milestones
    ADD CONSTRAINT project_fee_milestones_id_org_id_key UNIQUE (id, org_id);

ALTER TABLE ONLY public.project_fee_milestones
    ADD CONSTRAINT project_fee_milestones_pkey PRIMARY KEY (id);

ALTER TABLE ONLY public.project_fee_milestones
    ADD CONSTRAINT project_fee_milestones_project_id_position_key UNIQUE (project_id, "position") DEFERRABLE INITIALLY DEFERRED;

ALTER TABLE ONLY public.project_fee_occurrences
    ADD CONSTRAINT project_fee_occurrences_pkey PRIMARY KEY (id);

ALTER TABLE ONLY public.project_fee_occurrences
    ADD CONSTRAINT project_fee_occurrences_project_id_period_key_key UNIQUE (project_id, period_key);

ALTER TABLE ONLY public.project_management_assignments
    ADD CONSTRAINT project_management_assignments_org_id_manager_id_project_id_key UNIQUE (org_id, manager_id, project_id);

ALTER TABLE ONLY public.project_management_assignments
    ADD CONSTRAINT project_management_assignments_pkey PRIMARY KEY (id);

ALTER TABLE ONLY public.project_member_budgets
    ADD CONSTRAINT project_member_budgets_pkey PRIMARY KEY (id);

ALTER TABLE ONLY public.project_member_budgets
    ADD CONSTRAINT project_member_budgets_project_id_user_id_key UNIQUE (project_id, user_id);

ALTER TABLE ONLY public.project_member_costs
    ADD CONSTRAINT project_member_costs_pkey PRIMARY KEY (id);

ALTER TABLE ONLY public.project_member_costs
    ADD CONSTRAINT project_member_costs_project_id_user_id_key UNIQUE (project_id, user_id);

ALTER TABLE ONLY public.project_private_settings
    ADD CONSTRAINT project_private_settings_pkey PRIMARY KEY (id);

ALTER TABLE ONLY public.project_private_settings
    ADD CONSTRAINT project_private_settings_project_id_key UNIQUE (project_id);

ALTER TABLE ONLY public.project_settings
    ADD CONSTRAINT project_settings_pkey PRIMARY KEY (id);

ALTER TABLE ONLY public.project_settings
    ADD CONSTRAINT project_settings_project_id_key UNIQUE (project_id);

ALTER TABLE ONLY public.project_tag_links
    ADD CONSTRAINT project_tag_links_pkey PRIMARY KEY (id);

ALTER TABLE ONLY public.project_tag_links
    ADD CONSTRAINT project_tag_links_project_id_tag_id_key UNIQUE (project_id, tag_id);

ALTER TABLE ONLY public.project_tags
    ADD CONSTRAINT project_tags_id_org_id_key UNIQUE (id, org_id);

ALTER TABLE ONLY public.project_tags
    ADD CONSTRAINT project_tags_pkey PRIMARY KEY (id);

ALTER TABLE ONLY public.project_task_members
    ADD CONSTRAINT project_task_members_pkey PRIMARY KEY (id);

ALTER TABLE ONLY public.project_task_members
    ADD CONSTRAINT project_task_members_project_id_task_id_user_id_key UNIQUE (project_id, task_id, user_id);

ALTER TABLE ONLY public.project_task_settings
    ADD CONSTRAINT project_task_settings_pkey PRIMARY KEY (id);

ALTER TABLE ONLY public.project_task_settings
    ADD CONSTRAINT project_task_settings_project_id_task_id_key UNIQUE (project_id, task_id);

ALTER TABLE ONLY public.project_tasks
    ADD CONSTRAINT project_tasks_pkey PRIMARY KEY (project_id, task_id);

ALTER TABLE ONLY public.projects
    ADD CONSTRAINT projects_id_org_unique UNIQUE (id, org_id);

ALTER TABLE ONLY public.projects
    ADD CONSTRAINT projects_pkey PRIMARY KEY (id);

ALTER TABLE ONLY public.tasks
    ADD CONSTRAINT tasks_id_org_unique UNIQUE (id, org_id);

ALTER TABLE ONLY public.tasks
    ADD CONSTRAINT tasks_pkey PRIMARY KEY (id);

ALTER TABLE ONLY public.time_entries
    ADD CONSTRAINT time_entries_pkey PRIMARY KEY (id);

ALTER TABLE ONLY public.users
    ADD CONSTRAINT users_email_key UNIQUE (email);

ALTER TABLE ONLY public.users
    ADD CONSTRAINT users_id_org_unique UNIQUE (id, org_id);

ALTER TABLE ONLY public.users
    ADD CONSTRAINT users_oidc_subject_key UNIQUE (oidc_subject);

ALTER TABLE ONLY public.users
    ADD CONSTRAINT users_org_id_id_key UNIQUE (org_id, id);

ALTER TABLE ONLY public.users
    ADD CONSTRAINT users_pkey PRIMARY KEY (id);

CREATE INDEX approvals_org_id_idx ON public.approvals USING btree (org_id);

CREATE INDEX assignments_user_id_idx ON public.assignments USING btree (user_id);

CREATE UNIQUE INDEX clients_harvest_id ON public.clients USING btree (org_id, harvest_id);

CREATE INDEX clients_org_id_idx ON public.clients USING btree (org_id);

CREATE INDEX harvest_import_map_horae_id ON public.harvest_import_map USING btree (org_id, harvest_entity_type, horae_id);

CREATE INDEX horae_jobs_claim_idx ON public.horae_jobs USING btree (available_at, created_at) WHERE (status = ANY (ARRAY['queued'::text, 'running'::text]));

CREATE INDEX horae_outbox_delivery_idx ON public.horae_outbox USING btree (available_at, created_at) WHERE (delivered_at IS NULL);

CREATE INDEX horae_outbox_pending_kind_idx ON public.horae_outbox USING btree (event_kind, available_at, created_at) WHERE ((delivered_at IS NULL) AND (failed_at IS NULL));

CREATE INDEX invoice_line_items_time_entry_id_idx ON public.invoice_line_items USING btree (time_entry_id);

CREATE INDEX invoice_lines_fee_balance ON public.invoice_line_items USING btree (fee_occurrence_id, invoice_id) WHERE (fee_occurrence_id IS NOT NULL);

CREATE INDEX invoices_client_id_idx ON public.invoices USING btree (client_id);

CREATE INDEX invoices_org_id_idx ON public.invoices USING btree (org_id);

CREATE UNIQUE INDEX one_running_timer_per_user ON public.time_entries USING btree (user_id) WHERE is_running;

CREATE UNIQUE INDEX permission_change_receipts_operator_request_key ON public.permission_change_receipts USING btree (org_id, operator_id, request_id) WHERE (operator_id IS NOT NULL);

CREATE UNIQUE INDEX permission_change_receipts_user_request_key ON public.permission_change_receipts USING btree (org_id, actor_user_id, request_id) WHERE (actor_user_id IS NOT NULL);

CREATE UNIQUE INDEX permission_templates_org_name_key ON public.permission_templates USING btree (org_id, lower((name COLLATE public.permission_template_names)));

CREATE INDEX person_permission_states_template_idx ON public.person_permission_states USING btree (org_id, template_id) WHERE (template_id IS NOT NULL);

CREATE UNIQUE INDEX project_drafts_current_creator_idx ON public.project_drafts USING btree (org_id, creator_id) WHERE ((completed_project_id IS NULL) AND (discarded_at IS NULL));

CREATE UNIQUE INDEX project_tags_name_idx ON public.project_tags USING btree (org_id, lower(btrim(name)));

CREATE INDEX project_tasks_task_id_idx ON public.project_tasks USING btree (task_id);

CREATE INDEX projects_client_id_idx ON public.projects USING btree (client_id);

CREATE UNIQUE INDEX projects_harvest_id ON public.projects USING btree (org_id, harvest_id);

CREATE INDEX projects_org_id_idx ON public.projects USING btree (org_id);

CREATE UNIQUE INDEX tasks_harvest_id ON public.tasks USING btree (org_id, harvest_id);

CREATE INDEX tasks_org_id_idx ON public.tasks USING btree (org_id);

CREATE UNIQUE INDEX time_entries_harvest_id ON public.time_entries USING btree (org_id, harvest_id);

CREATE INDEX time_entries_invoice_id_idx ON public.time_entries USING btree (invoice_id) WHERE (invoice_id IS NOT NULL);

CREATE INDEX time_entries_org_id_spent_date_idx ON public.time_entries USING btree (org_id, spent_date);

CREATE INDEX time_entries_org_id_updated_at_idx ON public.time_entries USING btree (org_id, updated_at);

CREATE INDEX time_entries_org_id_user_id_project_id_idx ON public.time_entries USING btree (org_id, user_id, project_id);

CREATE INDEX time_entries_org_id_user_id_task_id_idx ON public.time_entries USING btree (org_id, user_id, task_id);

CREATE INDEX time_entries_project_id_minutes_rounded_minutes_idx ON public.time_entries USING btree (project_id) INCLUDE (minutes, rounded_minutes);

CREATE INDEX time_entries_project_id_spent_date_idx ON public.time_entries USING btree (project_id, spent_date);

CREATE INDEX time_entries_spent_date_idx ON public.time_entries USING brin (spent_date) WITH (autosummarize='on');

CREATE INDEX time_entries_task_id_idx ON public.time_entries USING btree (task_id);

CREATE INDEX time_entries_user_id_spent_date_idx ON public.time_entries USING btree (user_id, spent_date);

CREATE UNIQUE INDEX users_harvest_id ON public.users USING btree (org_id, harvest_id);

CREATE INDEX users_import_email_idx ON public.users USING btree (org_id, public.harvest_norm(email));

CREATE INDEX users_import_name_idx ON public.users USING btree (org_id, public.harvest_norm(name));

CREATE INDEX users_org_id_idx ON public.users USING btree (org_id);

CREATE TRIGGER clients_harvest_id BEFORE INSERT ON public.clients FOR EACH ROW EXECUTE FUNCTION public.harvest_id_assign('clients_harvest_id_seq', 'client');

CREATE TRIGGER harvest_credentials_set_updated_at BEFORE UPDATE ON public.harvest_credentials FOR EACH ROW EXECUTE FUNCTION public.set_updated_at();

CREATE TRIGGER harvest_oauth_clients_set_updated_at BEFORE UPDATE ON public.harvest_oauth_clients FOR EACH ROW EXECUTE FUNCTION public.set_updated_at();

CREATE TRIGGER invoice_editor_changed AFTER INSERT OR DELETE OR UPDATE ON public.invoice_line_items FOR EACH ROW EXECUTE FUNCTION public.invalidate_invoice_editor();

CREATE TRIGGER invoices_edit_revision BEFORE UPDATE ON public.invoices FOR EACH ROW EXECUTE FUNCTION public.advance_invoice_edit_revision();

CREATE TRIGGER person_permission_states_set_updated_at BEFORE UPDATE ON public.person_permission_states FOR EACH ROW EXECUTE FUNCTION public.set_updated_at();

CREATE TRIGGER project_editor_changed AFTER INSERT OR DELETE OR UPDATE ON public.assignments FOR EACH ROW EXECUTE FUNCTION public.invalidate_project_editor();

CREATE TRIGGER project_editor_changed AFTER INSERT OR DELETE OR UPDATE ON public.project_fee_milestones FOR EACH ROW EXECUTE FUNCTION public.invalidate_project_editor();

CREATE TRIGGER project_editor_changed AFTER INSERT OR DELETE OR UPDATE ON public.project_fee_occurrences FOR EACH ROW EXECUTE FUNCTION public.invalidate_project_editor();

CREATE TRIGGER project_editor_changed AFTER INSERT OR DELETE OR UPDATE ON public.project_member_budgets FOR EACH ROW EXECUTE FUNCTION public.invalidate_project_editor();

CREATE TRIGGER project_editor_changed AFTER INSERT OR DELETE OR UPDATE ON public.project_member_costs FOR EACH ROW EXECUTE FUNCTION public.invalidate_project_editor();

CREATE TRIGGER project_editor_changed AFTER INSERT OR DELETE OR UPDATE ON public.project_private_settings FOR EACH ROW EXECUTE FUNCTION public.invalidate_project_editor();

CREATE TRIGGER project_editor_changed AFTER INSERT OR DELETE OR UPDATE ON public.project_settings FOR EACH ROW EXECUTE FUNCTION public.invalidate_project_editor();

CREATE TRIGGER project_editor_changed AFTER INSERT OR DELETE OR UPDATE ON public.project_tag_links FOR EACH ROW EXECUTE FUNCTION public.invalidate_project_editor();

CREATE TRIGGER project_editor_changed AFTER INSERT OR DELETE OR UPDATE ON public.project_task_members FOR EACH ROW EXECUTE FUNCTION public.invalidate_project_editor();

CREATE TRIGGER project_editor_changed AFTER INSERT OR DELETE OR UPDATE ON public.project_task_settings FOR EACH ROW EXECUTE FUNCTION public.invalidate_project_editor();

CREATE TRIGGER project_editor_changed AFTER INSERT OR DELETE OR UPDATE ON public.project_tasks FOR EACH ROW EXECUTE FUNCTION public.invalidate_project_editor();

CREATE TRIGGER projects_edit_revision BEFORE UPDATE ON public.projects FOR EACH ROW EXECUTE FUNCTION public.advance_project_edit_revision();

CREATE TRIGGER projects_harvest_id BEFORE INSERT ON public.projects FOR EACH ROW EXECUTE FUNCTION public.harvest_id_assign('projects_harvest_id_seq', 'project');

CREATE TRIGGER tasks_harvest_id BEFORE INSERT ON public.tasks FOR EACH ROW EXECUTE FUNCTION public.harvest_id_assign('tasks_harvest_id_seq', 'task');

CREATE TRIGGER time_entries_harvest_id BEFORE INSERT ON public.time_entries FOR EACH ROW EXECUTE FUNCTION public.harvest_id_assign('time_entries_harvest_id_seq', 'time_entry');

CREATE TRIGGER time_entries_minutes_daily_cap BEFORE INSERT OR UPDATE OF minutes ON public.time_entries FOR EACH ROW EXECUTE FUNCTION public.time_entries_minutes_daily_cap();

CREATE TRIGGER time_entries_set_updated_at BEFORE UPDATE ON public.time_entries FOR EACH ROW EXECUTE FUNCTION public.set_updated_at();

CREATE TRIGGER users_harvest_id BEFORE INSERT ON public.users FOR EACH ROW EXECUTE FUNCTION public.harvest_id_assign('users_harvest_id_seq', 'person');

ALTER TABLE ONLY public.approvals
    ADD CONSTRAINT approvals_approved_by_fkey FOREIGN KEY (approved_by) REFERENCES public.users(id);

ALTER TABLE ONLY public.approvals
    ADD CONSTRAINT approvals_org_id_fkey FOREIGN KEY (org_id) REFERENCES public.organizations(id);

ALTER TABLE ONLY public.approvals
    ADD CONSTRAINT approvals_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id);

ALTER TABLE ONLY public.assignments
    ADD CONSTRAINT assignments_project_id_fkey FOREIGN KEY (project_id) REFERENCES public.projects(id);

ALTER TABLE ONLY public.assignments
    ADD CONSTRAINT assignments_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id);

ALTER TABLE ONLY public.clients
    ADD CONSTRAINT clients_org_id_fkey FOREIGN KEY (org_id) REFERENCES public.organizations(id);

ALTER TABLE ONLY public.time_entries
    ADD CONSTRAINT fk_time_entries_invoice FOREIGN KEY (invoice_id) REFERENCES public.invoices(id);

ALTER TABLE ONLY public.harvest_account_bindings
    ADD CONSTRAINT harvest_account_bindings_org_id_fkey FOREIGN KEY (org_id) REFERENCES public.organizations(id);

ALTER TABLE ONLY public.harvest_connection_generations
    ADD CONSTRAINT harvest_connection_generations_org_id_fkey FOREIGN KEY (org_id) REFERENCES public.organizations(id);

ALTER TABLE ONLY public.harvest_credentials
    ADD CONSTRAINT harvest_credentials_org_id_fkey FOREIGN KEY (org_id) REFERENCES public.organizations(id);

ALTER TABLE ONLY public.harvest_import_map
    ADD CONSTRAINT harvest_import_map_org_id_fkey FOREIGN KEY (org_id) REFERENCES public.organizations(id);

ALTER TABLE ONLY public.harvest_oauth_clients
    ADD CONSTRAINT harvest_oauth_clients_org_id_fkey FOREIGN KEY (org_id) REFERENCES public.organizations(id);

ALTER TABLE ONLY public.harvest_oauth_clients
    ADD CONSTRAINT harvest_oauth_clients_updated_by_fkey FOREIGN KEY (updated_by) REFERENCES public.users(id);

ALTER TABLE ONLY public.horae_job_report_error_chunks
    ADD CONSTRAINT horae_job_report_error_chunks_job_id_org_id_fkey FOREIGN KEY (job_id, org_id) REFERENCES public.horae_jobs(id, org_id) ON DELETE CASCADE;

ALTER TABLE ONLY public.horae_job_report_error_chunks
    ADD CONSTRAINT horae_job_report_error_chunks_org_id_fkey FOREIGN KEY (org_id) REFERENCES public.organizations(id);

ALTER TABLE ONLY public.horae_job_uploads
    ADD CONSTRAINT horae_job_uploads_job_org_fkey FOREIGN KEY (job_id, org_id) REFERENCES public.horae_jobs(id, org_id) ON DELETE CASCADE;

ALTER TABLE ONLY public.horae_job_uploads
    ADD CONSTRAINT horae_job_uploads_org_id_fkey FOREIGN KEY (org_id) REFERENCES public.organizations(id);

ALTER TABLE ONLY public.horae_jobs
    ADD CONSTRAINT horae_jobs_org_id_fkey FOREIGN KEY (org_id) REFERENCES public.organizations(id);

ALTER TABLE ONLY public.horae_jobs
    ADD CONSTRAINT horae_jobs_original_requester_fkey FOREIGN KEY (org_id, original_requester_id) REFERENCES public.users(org_id, id);

ALTER TABLE ONLY public.horae_outbox
    ADD CONSTRAINT horae_outbox_org_id_fkey FOREIGN KEY (org_id) REFERENCES public.organizations(id);

ALTER TABLE ONLY public.invoice_edit_requests
    ADD CONSTRAINT invoice_edit_requests_actor_id_org_id_fkey FOREIGN KEY (actor_id, org_id) REFERENCES public.users(id, org_id);

ALTER TABLE ONLY public.invoice_edit_requests
    ADD CONSTRAINT invoice_edit_requests_invoice_id_org_id_fkey FOREIGN KEY (invoice_id, org_id) REFERENCES public.invoices(id, org_id);

ALTER TABLE ONLY public.invoice_edit_requests
    ADD CONSTRAINT invoice_edit_requests_org_id_fkey FOREIGN KEY (org_id) REFERENCES public.organizations(id);

ALTER TABLE ONLY public.invoice_generation_requests
    ADD CONSTRAINT invoice_generation_requests_actor_id_org_id_fkey FOREIGN KEY (actor_id, org_id) REFERENCES public.users(id, org_id);

ALTER TABLE ONLY public.invoice_generation_requests
    ADD CONSTRAINT invoice_generation_requests_invoice_id_org_id_fkey FOREIGN KEY (invoice_id, org_id) REFERENCES public.invoices(id, org_id);

ALTER TABLE ONLY public.invoice_generation_requests
    ADD CONSTRAINT invoice_generation_requests_org_id_fkey FOREIGN KEY (org_id) REFERENCES public.organizations(id);

ALTER TABLE ONLY public.invoice_line_items
    ADD CONSTRAINT invoice_line_items_fee_occurrence_id_fkey FOREIGN KEY (fee_occurrence_id) REFERENCES public.project_fee_occurrences(id);

ALTER TABLE ONLY public.invoice_line_items
    ADD CONSTRAINT invoice_line_items_invoice_id_fkey FOREIGN KEY (invoice_id) REFERENCES public.invoices(id) ON DELETE CASCADE;

ALTER TABLE ONLY public.invoice_line_items
    ADD CONSTRAINT invoice_line_items_time_entry_id_fkey FOREIGN KEY (time_entry_id) REFERENCES public.time_entries(id);

ALTER TABLE ONLY public.invoices
    ADD CONSTRAINT invoices_client_id_fkey FOREIGN KEY (client_id) REFERENCES public.clients(id);

ALTER TABLE ONLY public.invoices
    ADD CONSTRAINT invoices_org_id_fkey FOREIGN KEY (org_id) REFERENCES public.organizations(id);

ALTER TABLE ONLY public.permission_change_receipts
    ADD CONSTRAINT permission_change_receipts_org_id_actor_user_id_fkey FOREIGN KEY (org_id, actor_user_id) REFERENCES public.users(org_id, id);

ALTER TABLE ONLY public.permission_change_receipts
    ADD CONSTRAINT permission_change_receipts_org_id_fkey FOREIGN KEY (org_id) REFERENCES public.organizations(id);

ALTER TABLE ONLY public.permission_templates
    ADD CONSTRAINT permission_templates_org_id_fkey FOREIGN KEY (org_id) REFERENCES public.organizations(id);

ALTER TABLE ONLY public.person_management_assignments
    ADD CONSTRAINT person_management_assignments_managed_user_id_org_id_fkey FOREIGN KEY (managed_user_id, org_id) REFERENCES public.users(id, org_id);

ALTER TABLE ONLY public.person_management_assignments
    ADD CONSTRAINT person_management_assignments_manager_id_org_id_fkey FOREIGN KEY (manager_id, org_id) REFERENCES public.users(id, org_id);

ALTER TABLE ONLY public.person_management_assignments
    ADD CONSTRAINT person_management_assignments_org_id_fkey FOREIGN KEY (org_id) REFERENCES public.organizations(id);

ALTER TABLE ONLY public.person_permission_states
    ADD CONSTRAINT person_permission_states_org_id_fkey FOREIGN KEY (org_id) REFERENCES public.organizations(id);

ALTER TABLE ONLY public.person_permission_states
    ADD CONSTRAINT person_permission_states_org_id_template_id_fkey FOREIGN KEY (org_id, template_id) REFERENCES public.permission_templates(org_id, id) ON DELETE RESTRICT;

ALTER TABLE ONLY public.person_permission_states
    ADD CONSTRAINT person_permission_states_org_id_user_id_fkey FOREIGN KEY (org_id, user_id) REFERENCES public.users(org_id, id);

ALTER TABLE ONLY public.project_budget_notifications
    ADD CONSTRAINT project_budget_notifications_org_id_fkey FOREIGN KEY (org_id) REFERENCES public.organizations(id);

ALTER TABLE ONLY public.project_budget_notifications
    ADD CONSTRAINT project_budget_notifications_project_id_org_id_fkey FOREIGN KEY (project_id, org_id) REFERENCES public.projects(id, org_id);

ALTER TABLE ONLY public.project_budget_notifications
    ADD CONSTRAINT project_budget_notifications_recipient_id_org_id_fkey FOREIGN KEY (recipient_id, org_id) REFERENCES public.users(id, org_id);

ALTER TABLE ONLY public.project_budget_notifications
    ADD CONSTRAINT project_budget_notifications_task_id_org_id_fkey FOREIGN KEY (task_id, org_id) REFERENCES public.tasks(id, org_id);

ALTER TABLE ONLY public.project_budget_notifications
    ADD CONSTRAINT project_budget_notifications_user_id_org_id_fkey FOREIGN KEY (user_id, org_id) REFERENCES public.users(id, org_id);

ALTER TABLE ONLY public.project_drafts
    ADD CONSTRAINT project_drafts_completed_project_id_org_id_fkey FOREIGN KEY (completed_project_id, org_id) REFERENCES public.projects(id, org_id);

ALTER TABLE ONLY public.project_drafts
    ADD CONSTRAINT project_drafts_creator_id_org_id_fkey FOREIGN KEY (creator_id, org_id) REFERENCES public.users(id, org_id);

ALTER TABLE ONLY public.project_drafts
    ADD CONSTRAINT project_drafts_org_id_fkey FOREIGN KEY (org_id) REFERENCES public.organizations(id);

ALTER TABLE ONLY public.project_edit_requests
    ADD CONSTRAINT project_edit_requests_actor_id_org_id_fkey FOREIGN KEY (actor_id, org_id) REFERENCES public.users(id, org_id);

ALTER TABLE ONLY public.project_edit_requests
    ADD CONSTRAINT project_edit_requests_org_id_fkey FOREIGN KEY (org_id) REFERENCES public.organizations(id);

ALTER TABLE ONLY public.project_edit_requests
    ADD CONSTRAINT project_edit_requests_project_id_org_id_fkey FOREIGN KEY (project_id, org_id) REFERENCES public.projects(id, org_id) ON DELETE CASCADE;

ALTER TABLE ONLY public.project_fee_milestones
    ADD CONSTRAINT project_fee_milestones_org_id_fkey FOREIGN KEY (org_id) REFERENCES public.organizations(id);

ALTER TABLE ONLY public.project_fee_milestones
    ADD CONSTRAINT project_fee_milestones_project_id_org_id_fkey FOREIGN KEY (project_id, org_id) REFERENCES public.projects(id, org_id) ON DELETE CASCADE;

ALTER TABLE ONLY public.project_fee_occurrences
    ADD CONSTRAINT project_fee_occurrences_milestone_id_project_id_org_id_fkey FOREIGN KEY (milestone_id, project_id, org_id) REFERENCES public.project_fee_milestones(id, project_id, org_id);

ALTER TABLE ONLY public.project_fee_occurrences
    ADD CONSTRAINT project_fee_occurrences_org_id_fkey FOREIGN KEY (org_id) REFERENCES public.organizations(id);

ALTER TABLE ONLY public.project_fee_occurrences
    ADD CONSTRAINT project_fee_occurrences_project_id_org_id_fkey FOREIGN KEY (project_id, org_id) REFERENCES public.projects(id, org_id);

ALTER TABLE ONLY public.project_management_assignments
    ADD CONSTRAINT project_management_assignments_manager_id_org_id_fkey FOREIGN KEY (manager_id, org_id) REFERENCES public.users(id, org_id);

ALTER TABLE ONLY public.project_management_assignments
    ADD CONSTRAINT project_management_assignments_org_id_fkey FOREIGN KEY (org_id) REFERENCES public.organizations(id);

ALTER TABLE ONLY public.project_management_assignments
    ADD CONSTRAINT project_management_assignments_project_id_org_id_fkey FOREIGN KEY (project_id, org_id) REFERENCES public.projects(id, org_id);

ALTER TABLE ONLY public.project_member_budgets
    ADD CONSTRAINT project_member_budgets_org_id_fkey FOREIGN KEY (org_id) REFERENCES public.organizations(id);

ALTER TABLE ONLY public.project_member_budgets
    ADD CONSTRAINT project_member_budgets_project_id_org_id_fkey FOREIGN KEY (project_id, org_id) REFERENCES public.projects(id, org_id) ON DELETE CASCADE;

ALTER TABLE ONLY public.project_member_budgets
    ADD CONSTRAINT project_member_budgets_project_id_user_id_fkey FOREIGN KEY (project_id, user_id) REFERENCES public.assignments(project_id, user_id) ON DELETE CASCADE;

ALTER TABLE ONLY public.project_member_budgets
    ADD CONSTRAINT project_member_budgets_user_id_org_id_fkey FOREIGN KEY (user_id, org_id) REFERENCES public.users(id, org_id);

ALTER TABLE ONLY public.project_member_costs
    ADD CONSTRAINT project_member_costs_org_id_fkey FOREIGN KEY (org_id) REFERENCES public.organizations(id);

ALTER TABLE ONLY public.project_member_costs
    ADD CONSTRAINT project_member_costs_project_id_org_id_fkey FOREIGN KEY (project_id, org_id) REFERENCES public.projects(id, org_id) ON DELETE CASCADE;

ALTER TABLE ONLY public.project_member_costs
    ADD CONSTRAINT project_member_costs_project_id_user_id_fkey FOREIGN KEY (project_id, user_id) REFERENCES public.assignments(project_id, user_id) ON DELETE CASCADE;

ALTER TABLE ONLY public.project_member_costs
    ADD CONSTRAINT project_member_costs_user_id_org_id_fkey FOREIGN KEY (user_id, org_id) REFERENCES public.users(id, org_id);

ALTER TABLE ONLY public.project_private_settings
    ADD CONSTRAINT project_private_settings_org_id_fkey FOREIGN KEY (org_id) REFERENCES public.organizations(id);

ALTER TABLE ONLY public.project_private_settings
    ADD CONSTRAINT project_private_settings_project_id_org_id_fkey FOREIGN KEY (project_id, org_id) REFERENCES public.projects(id, org_id) ON DELETE CASCADE;

ALTER TABLE ONLY public.project_settings
    ADD CONSTRAINT project_settings_creator_id_org_id_fkey FOREIGN KEY (creator_id, org_id) REFERENCES public.users(id, org_id);

ALTER TABLE ONLY public.project_settings
    ADD CONSTRAINT project_settings_org_id_fkey FOREIGN KEY (org_id) REFERENCES public.organizations(id);

ALTER TABLE ONLY public.project_settings
    ADD CONSTRAINT project_settings_project_id_org_id_fkey FOREIGN KEY (project_id, org_id) REFERENCES public.projects(id, org_id) ON DELETE CASCADE;

ALTER TABLE ONLY public.project_tag_links
    ADD CONSTRAINT project_tag_links_org_id_fkey FOREIGN KEY (org_id) REFERENCES public.organizations(id);

ALTER TABLE ONLY public.project_tag_links
    ADD CONSTRAINT project_tag_links_project_id_org_id_fkey FOREIGN KEY (project_id, org_id) REFERENCES public.projects(id, org_id) ON DELETE CASCADE;

ALTER TABLE ONLY public.project_tag_links
    ADD CONSTRAINT project_tag_links_tag_id_org_id_fkey FOREIGN KEY (tag_id, org_id) REFERENCES public.project_tags(id, org_id);

ALTER TABLE ONLY public.project_tags
    ADD CONSTRAINT project_tags_org_id_fkey FOREIGN KEY (org_id) REFERENCES public.organizations(id);

ALTER TABLE ONLY public.project_task_members
    ADD CONSTRAINT project_task_members_org_id_fkey FOREIGN KEY (org_id) REFERENCES public.organizations(id);

ALTER TABLE ONLY public.project_task_members
    ADD CONSTRAINT project_task_members_project_id_org_id_fkey FOREIGN KEY (project_id, org_id) REFERENCES public.projects(id, org_id) ON DELETE CASCADE;

ALTER TABLE ONLY public.project_task_members
    ADD CONSTRAINT project_task_members_project_id_task_id_fkey FOREIGN KEY (project_id, task_id) REFERENCES public.project_task_settings(project_id, task_id) ON DELETE CASCADE;

ALTER TABLE ONLY public.project_task_members
    ADD CONSTRAINT project_task_members_project_id_user_id_fkey FOREIGN KEY (project_id, user_id) REFERENCES public.assignments(project_id, user_id) ON DELETE CASCADE;

ALTER TABLE ONLY public.project_task_members
    ADD CONSTRAINT project_task_members_task_id_org_id_fkey FOREIGN KEY (task_id, org_id) REFERENCES public.tasks(id, org_id);

ALTER TABLE ONLY public.project_task_members
    ADD CONSTRAINT project_task_members_user_id_org_id_fkey FOREIGN KEY (user_id, org_id) REFERENCES public.users(id, org_id);

ALTER TABLE ONLY public.project_task_settings
    ADD CONSTRAINT project_task_settings_org_id_fkey FOREIGN KEY (org_id) REFERENCES public.organizations(id);

ALTER TABLE ONLY public.project_task_settings
    ADD CONSTRAINT project_task_settings_project_id_org_id_fkey FOREIGN KEY (project_id, org_id) REFERENCES public.projects(id, org_id) ON DELETE CASCADE;

ALTER TABLE ONLY public.project_task_settings
    ADD CONSTRAINT project_task_settings_project_id_task_id_fkey FOREIGN KEY (project_id, task_id) REFERENCES public.project_tasks(project_id, task_id) ON DELETE CASCADE;

ALTER TABLE ONLY public.project_task_settings
    ADD CONSTRAINT project_task_settings_task_id_org_id_fkey FOREIGN KEY (task_id, org_id) REFERENCES public.tasks(id, org_id);

ALTER TABLE ONLY public.project_tasks
    ADD CONSTRAINT project_tasks_project_id_fkey FOREIGN KEY (project_id) REFERENCES public.projects(id);

ALTER TABLE ONLY public.project_tasks
    ADD CONSTRAINT project_tasks_task_id_fkey FOREIGN KEY (task_id) REFERENCES public.tasks(id);

ALTER TABLE ONLY public.projects
    ADD CONSTRAINT projects_client_id_fkey FOREIGN KEY (client_id) REFERENCES public.clients(id);

ALTER TABLE ONLY public.projects
    ADD CONSTRAINT projects_org_id_fkey FOREIGN KEY (org_id) REFERENCES public.organizations(id);

ALTER TABLE ONLY public.tasks
    ADD CONSTRAINT tasks_org_id_fkey FOREIGN KEY (org_id) REFERENCES public.organizations(id);

ALTER TABLE ONLY public.time_entries
    ADD CONSTRAINT time_entries_org_id_fkey FOREIGN KEY (org_id) REFERENCES public.organizations(id);

ALTER TABLE ONLY public.time_entries
    ADD CONSTRAINT time_entries_project_id_fkey FOREIGN KEY (project_id) REFERENCES public.projects(id);

ALTER TABLE ONLY public.time_entries
    ADD CONSTRAINT time_entries_task_id_fkey FOREIGN KEY (task_id) REFERENCES public.tasks(id);

ALTER TABLE ONLY public.time_entries
    ADD CONSTRAINT time_entries_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id);

ALTER TABLE ONLY public.users
    ADD CONSTRAINT users_org_id_fkey FOREIGN KEY (org_id) REFERENCES public.organizations(id);

REVOKE ALL ON FUNCTION public.advance_invoice_edit_revision() FROM PUBLIC;

REVOKE ALL ON FUNCTION public.advance_project_edit_revision() FROM PUBLIC;

REVOKE ALL ON FUNCTION public.fetch_csv_export_rows(max_rows integer) FROM PUBLIC;

REVOKE ALL ON FUNCTION public.harvest_id_assign() FROM PUBLIC;

REVOKE ALL ON FUNCTION public.harvest_id_next(p_table regclass, p_sequence regclass, p_type public.harvest_entity_type, p_org uuid) FROM PUBLIC;

REVOKE ALL ON FUNCTION public.invalidate_invoice_editor() FROM PUBLIC;

REVOKE ALL ON FUNCTION public.invalidate_project_editor() FROM PUBLIC;

REVOKE ALL ON FUNCTION public.resolve_project_rate(mode text, task_rate bigint, assignment_rate bigint, project_rate bigint, user_rate bigint, client_rate bigint) FROM PUBLIC;

REVOKE ALL ON FUNCTION public.time_entries_minutes_daily_cap() FROM PUBLIC;
