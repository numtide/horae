use std::time::Duration;

use sqlx::postgres::PgPoolOptions;

/// A separate authenticated database identity for plugin queries. Never backed
/// by the application's writer pool or by SET ROLE on a writer connection.
#[derive(Clone)]
pub struct PluginDatabase {
    pool: sqlx::PgPool,
}

impl PluginDatabase {
    pub async fn connect(url: &str) -> anyhow::Result<Self> {
        Self::connect_with(url.parse()?).await
    }

    async fn connect_with(options: sqlx::postgres::PgConnectOptions) -> anyhow::Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(4)
            .acquire_timeout(Duration::from_secs(5))
            .connect_with(options)
            .await?;
        let mut connection = pool.acquire().await?;
        validate_connection(&mut connection).await?;
        drop(connection);
        Ok(Self { pool })
    }

    pub(super) fn pool(&self) -> &sqlx::PgPool {
        &self.pool
    }
}

pub(super) async fn validate_connection(connection: &mut sqlx::PgConnection) -> anyhow::Result<()> {
    let unsafe_identity = sqlx::query_scalar!(
        r#"SELECT (current_user <> session_user OR rolsuper OR rolcreatedb
            OR rolcreaterole OR rolreplication OR rolbypassrls
            OR EXISTS (SELECT 1 FROM pg_auth_members WHERE member = pg_roles.oid)
            OR has_database_privilege(current_database(), 'CREATE')
            OR EXISTS (SELECT 1 FROM pg_namespace WHERE has_schema_privilege(oid, 'CREATE'))
        ) AS "unsafe!" FROM pg_roles WHERE rolname = session_user"#
    )
    .fetch_one(&mut *connection)
    .await?;
    anyhow::ensure!(
        !unsafe_identity,
        "plugin database role must be an unprivileged login without role memberships or ownership"
    );

    // pg_settings is publicly updatable for session configuration. READ ONLY
    // blocks UPDATE and each query's session is discarded anyway.
    let unsafe_relations = sqlx::query_scalar!(
        r#"SELECT EXISTS (
            SELECT 1 FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace
            WHERE c.relowner = (SELECT oid FROM pg_roles WHERE rolname = current_user)
            OR CASE WHEN c.relkind = 'S' THEN
                has_sequence_privilege(c.oid, 'USAGE,UPDATE')
            WHEN c.relkind IN ('r','p','v','m','f') THEN
                ((has_table_privilege(c.oid, 'INSERT,UPDATE,DELETE,TRUNCATE,REFERENCES,TRIGGER')
                  OR has_any_column_privilege(c.oid, 'INSERT,UPDATE,REFERENCES'))
                 AND NOT (n.nspname = 'pg_catalog' AND c.relname = 'pg_settings'))
                OR (n.nspname NOT IN ('pg_catalog', 'information_schema')
                    AND NOT (n.nspname = 'public' AND c.relkind IN ('r','p') AND c.relname = ANY($1))
                    AND has_any_column_privilege(c.oid, 'SELECT'))
            ELSE false END
        ) OR has_column_privilege('public.users', 'oidc_subject', 'SELECT') AS "unsafe!""#,
        &[
            "organizations", "users", "clients", "projects", "tasks", "project_tasks",
            "assignments", "time_entries", "approvals", "invoices", "invoice_line_items"
        ] as &[&str],
    ).fetch_one(&mut *connection).await?;
    anyhow::ensure!(
        !unsafe_relations,
        "plugin database role has write privileges or can read non-business data"
    );

    let unsafe_functions = sqlx::query_scalar!(
        r#"SELECT EXISTS (
            SELECT 1 FROM pg_proc p JOIN pg_namespace n ON n.oid = p.pronamespace
            WHERE has_function_privilege(p.oid, 'EXECUTE') AND (
                p.prosecdef OR (n.nspname NOT IN ('pg_catalog','information_schema')
                    AND p.oid NOT IN (
                        'public.harvest_norm(text)'::regprocedure,
                        'public.line_amount_cents(bigint,integer)'::regprocedure,
                        'public.set_updated_at()'::regprocedure
                    )
                    -- The rounding helper may not be installed yet. A NULL in
                    -- NOT IN would also allow unrelated functions through.
                    AND p.oid IS DISTINCT FROM to_regprocedure(
                        'public.effective_minutes(integer,integer,smallint,public.round_dir)'
                    ))
            )
        ) AS "unsafe!""#
    )
    .fetch_one(&mut *connection)
    .await?;
    anyhow::ensure!(
        !unsafe_functions,
        "plugin database role can execute an unapproved database function"
    );
    Ok(())
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;

    #[sqlx::test(migrations = "./migrations")]
    async fn reader_accepts_optional_first_party_rounding_helper(pool: sqlx::PgPool) {
        // The validator must work both before and after the rounding migration.
        // Only its function identity matters here; arithmetic has its own tests.
        let present = sqlx::query_scalar!(
            r#"SELECT to_regprocedure('public.effective_minutes(integer,integer,smallint,public.round_dir)') IS NOT NULL as "present!""#,
        ).fetch_one(&pool).await.unwrap();
        if !present {
            sqlx::query!(
                "CREATE FUNCTION public.effective_minutes(integer, integer, smallint, public.round_dir)
                 RETURNS integer LANGUAGE sql IMMUTABLE AS $$ SELECT COALESCE($2, $1) $$",
            ).execute(&pool).await.unwrap();
        }
        let reader = Reader::new(&pool).await;
        let mut connection = reader.pool.acquire().await.unwrap();
        let result = validate_connection(&mut connection).await;
        drop(connection);
        reader.finish().await;
        result.unwrap();
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn reader_rejects_unapproved_invoker_functions(pool: sqlx::PgPool) {
        sqlx::query!(
            "CREATE FUNCTION public.plugin_test_unknown() RETURNS integer LANGUAGE sql AS $$ SELECT 1 $$",
        ).execute(&pool).await.unwrap();
        let reader = Reader::new(&pool).await;
        let mut connection = reader.pool.acquire().await.unwrap();
        let result = validate_connection(&mut connection).await;
        drop(connection);
        reader.finish().await;
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("unapproved database function")
        );
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn application_database_owner_is_not_a_plugin_identity(pool: sqlx::PgPool) {
        let mut connection = pool.acquire().await.unwrap();
        assert!(validate_connection(&mut connection).await.is_err());
    }

    pub struct Reader {
        pub pool: sqlx::PgPool,
        name: String,
        admin: sqlx::PgPool,
    }

    impl Reader {
        pub async fn database(&self) -> PluginDatabase {
            PluginDatabase::connect_with((*self.pool.connect_options()).clone())
                .await
                .unwrap()
        }
        pub async fn new(admin: &sqlx::PgPool) -> Self {
            let name = format!("horae_plugin_test_{}", uuid::Uuid::now_v7().simple());
            let password = uuid::Uuid::now_v7().simple().to_string();
            // Role identifiers cannot be bind parameters. This name contains
            // only a fixed prefix and a generated UUID, never external input.
            let sql = format!(
                "CREATE ROLE {name} LOGIN PASSWORD '{password}'; GRANT USAGE ON SCHEMA public TO {name}; GRANT SELECT ON public.time_entries TO {name}"
            );
            sqlx::raw_sql(&sql).execute(admin).await.unwrap();
            let options = (*admin.connect_options())
                .clone()
                .username(&name)
                .password(&password);
            let pool = PgPoolOptions::new()
                .max_connections(1)
                .connect_with(options)
                .await
                .unwrap();
            Self {
                pool,
                name,
                admin: admin.clone(),
            }
        }

        pub async fn finish(self) {
            self.pool.close().await;
            let sql = format!(
                "REVOKE ALL ON ALL TABLES IN SCHEMA public FROM {0}; REVOKE ALL ON ALL FUNCTIONS IN SCHEMA public FROM {0}; REVOKE USAGE ON SCHEMA public FROM {0}; DROP ROLE {0}",
                self.name
            );
            sqlx::raw_sql(&sql).execute(&self.admin).await.unwrap();
        }
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn reader_cannot_have_access_to_credentials(pool: sqlx::PgPool) {
        let reader = Reader::new(&pool).await;
        sqlx::raw_sql(&format!(
            "GRANT SELECT ON public.harvest_credentials TO {}",
            reader.name
        ))
        .execute(&pool)
        .await
        .unwrap();
        let mut connection = reader.pool.acquire().await.unwrap();
        let result = validate_connection(&mut connection).await;
        drop(connection);
        reader.finish().await;
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("non-business data")
        );
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn reader_cannot_switch_into_another_role(pool: sqlx::PgPool) {
        let reader = Reader::new(&pool).await;
        sqlx::raw_sql(&format!("GRANT pg_read_all_data TO {}", reader.name))
            .execute(&pool)
            .await
            .unwrap();
        let mut connection = reader.pool.acquire().await.unwrap();
        let result = validate_connection(&mut connection).await;
        drop(connection);
        reader.finish().await;
        assert!(result.unwrap_err().to_string().contains("role memberships"));
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn reader_cannot_execute_security_definer_functions(pool: sqlx::PgPool) {
        let reader = Reader::new(&pool).await;
        sqlx::query!("CREATE FUNCTION public.plugin_test_secret() RETURNS text LANGUAGE sql SECURITY DEFINER AS $$ SELECT 'secret'::text $$")
            .execute(&pool).await.unwrap();
        let mut connection = reader.pool.acquire().await.unwrap();
        let result = validate_connection(&mut connection).await;
        drop(connection);
        reader.finish().await;
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("unapproved database function")
        );
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn dedicated_reader_is_accepted_but_new_write_grants_are_rejected(pool: sqlx::PgPool) {
        let reader = Reader::new(&pool).await;
        let mut connection = reader.pool.acquire().await.unwrap();
        let allowed = validate_connection(&mut connection).await;
        let sql = format!("GRANT UPDATE ON public.time_entries TO {}", reader.name);
        sqlx::raw_sql(&sql).execute(&pool).await.unwrap();
        let forbidden = validate_connection(&mut connection).await;
        drop(connection);
        reader.finish().await;
        allowed.unwrap();
        assert!(
            forbidden
                .unwrap_err()
                .to_string()
                .contains("write privileges")
        );
    }
}
