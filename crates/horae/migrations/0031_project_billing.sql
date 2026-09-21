-- NULL settings preserve the legacy/imported precedence. Callers only supply
-- inherited rates whose denomination matches the configured project currency.
-- Keep this pure function in parity with horae_core::invoice::resolve_project_rate.
CREATE FUNCTION resolve_project_rate(
  mode text,
  task_rate bigint,
  assignment_rate bigint,
  project_rate bigint,
  user_rate bigint,
  client_rate bigint
) RETURNS bigint
  LANGUAGE sql
  IMMUTABLE
  PARALLEL SAFE
  AS $$
    SELECT CASE COALESCE(mode, 'legacy')
      WHEN 'legacy' THEN COALESCE(task_rate, assignment_rate, project_rate, user_rate)
      WHEN 'person' THEN COALESCE(assignment_rate, user_rate, client_rate)
      WHEN 'task' THEN COALESCE(task_rate, client_rate)
      WHEN 'project' THEN project_rate
      ELSE NULL
    END
  $$;

-- Application queries run as the migration owner. Plugins do not need this
-- function or access to private project configuration; do not widen their grants.
REVOKE ALL ON FUNCTION resolve_project_rate(text, bigint, bigint, bigint, bigint, bigint) FROM PUBLIC;
