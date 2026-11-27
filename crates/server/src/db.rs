use sqlx::{
    PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
};
use std::time::Duration;

/// Create the shared Postgres pool. The maximum connection count is read from
/// `HORAE_DB_MAX_CONNECTIONS` (defaults to 10 when unset or unparseable).
pub async fn create_pool(database_url: &str) -> anyhow::Result<PgPool> {
    let max_connections = std::env::var("HORAE_DB_MAX_CONNECTIONS")
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(10);

    // Force a fresh plan for every prepared statement instead of letting
    // Postgres settle on a generic one.
    //
    // The reports (`report_time`, `report_detailed`, the CSV/XLSX export) build
    // their optional filters as `($n IS NULL OR col = $n)`. A generic plan is
    // built with no parameter values, so that idiom cannot be folded away: the
    // equality stays behind a NullTest where it can never become an index
    // condition, and the planner charges a default selectivity for each
    // NullTest. Three of them multiply into a row estimate far below reality,
    // which picks the wrong join order. Postgres switches to the generic plan
    // after five executions and compares the two costs with no margin, so the
    // sixth report render of a session is the one that falls off the cliff.
    //
    // Measured against an 800k-row copy of this schema: `report_time` 31.9 ms
    // custom vs 81.4 ms generic, `report_detailed`/the CSV export 8.2 ms vs
    // 39.3 ms. The cost is roughly 21 microseconds of extra planning per
    // trivial statement — 5000 primary-key lookups took 1354 ms with the
    // default and 1457 ms forced (+7.5%), a mixed hot-query loop +2% — against
    // 30-50 ms bought back on every report render.
    //
    // Set as a connection startup option rather than with `ALTER DATABASE`: it
    // is scoped to this application's connections, needs no ownership
    // privileges on a database the deployment may share, and is visible here
    // rather than in out-of-band database state.
    let connect_options = database_url
        .parse::<PgConnectOptions>()?
        .options([("plan_cache_mode", "force_custom_plan")]);

    let pool = PgPoolOptions::new()
        .max_connections(max_connections)
        .acquire_timeout(Duration::from_secs(3))
        .connect_with(connect_options)
        .await?;
    Ok(pool)
}

/// The embedded migrations. `#[sqlx::test]` takes this as
/// `migrator = "crate::db::MIGRATOR"`: given a path instead, it would paste
/// every migration's SQL into each of the ~1,300 tests it expands.
pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

pub async fn run_migrations(pool: &PgPool) -> anyhow::Result<()> {
    MIGRATOR.run(pool).await?;
    Ok(())
}

#[derive(Clone, Copy)]
pub enum OrganizationLock {
    Shared,
    AccessChange,
}

/// Acquire the final gate mode before actor/resource locks, never by upgrading.
/// Access changes exclude SHARE readers but allow existing writers' FK checks.
pub async fn lock_organization(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    org_id: uuid::Uuid,
    mode: OrganizationLock,
) -> Result<(), sqlx::Error> {
    match mode {
        OrganizationLock::Shared => {
            sqlx::query!(
                "SELECT id FROM organizations WHERE id = $1 FOR SHARE",
                org_id
            )
            .fetch_one(&mut **tx)
            .await?;
        }
        OrganizationLock::AccessChange => {
            // A no-op update takes the same NO KEY UPDATE lock but also writes a
            // new row version. REPEATABLE READ readers that took their snapshot
            // before this change then fail their gate lock with 40001 and
            // retry, instead of authorizing from the older snapshot. The
            // revision value is deliberately unchanged.
            sqlx::query!(
                "UPDATE organizations SET access_revision = access_revision WHERE id = $1 RETURNING id",
                org_id
            )
            .fetch_one(&mut **tx)
            .await?;
        }
    }
    Ok(())
}

/// Lock `email` for creating an account, until commit, and report whether an
/// account already holds it in any letter case. The unique index on `email` is
/// case-sensitive, so every path that creates a user takes this lock and
/// refuses a held address. Callers lock the organization row first, so the
/// order is always organization, then email. The transaction must be READ
/// COMMITTED, so the check sees accounts committed while the lock was awaited.
pub async fn lock_new_user_email(
    conn: &mut sqlx::PgConnection,
    email: &str,
) -> Result<bool, sqlx::Error> {
    sqlx::query!(
        r#"SELECT pg_advisory_xact_lock(hashtextextended('horae.signup-email:' || lower($1), 0)) as "lock!: ()""#,
        email,
    )
    .execute(&mut *conn)
    .await?;
    sqlx::query_scalar!(
        r#"SELECT EXISTS(SELECT 1 FROM users WHERE lower(email) = lower($1)) as "held!""#,
        email,
    )
    .fetch_one(conn)
    .await
}

/// Join the user's time-entry write barrier before acquiring entry row locks.
/// Writers share this lock; submitting a week takes the same key exclusively.
/// The user-wide key also covers moves between weeks without a racy date lookup.
pub async fn lock_time_entry_write(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user_id: uuid::Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"SELECT pg_advisory_xact_lock_shared(hashtextextended('horae.timesheet:' || $1::uuid::text, 0)) as "lock!: ()""#,
        user_id,
    ).execute(&mut **tx).await?;
    Ok(())
}

/// `user_id`'s total minutes on each of `dates` (0 when empty), including this
/// transaction's own writes. The write lock above is shared, so it does not
/// order two writers for the same day; each (person, day) is locked here until
/// commit. Every web write that changes a day's minutes, decreases included,
/// takes it. Read before a write and again after it, the two totals bracket
/// exactly this transaction's change.
pub async fn daily_minutes_totals(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user_id: uuid::Uuid,
    dates: &[chrono::NaiveDate],
) -> Result<Vec<(chrono::NaiveDate, i64)>, sqlx::Error> {
    // One lock per statement, in ascending date order, so writers sharing days
    // always queue in the same order and cannot deadlock.
    let days: std::collections::BTreeSet<_> = dates.iter().collect();
    for day in days {
        sqlx::query!(
            r#"SELECT pg_advisory_xact_lock(hashtextextended('horae.day:' || $1::uuid::text || ':' || $2::date::text, 0)) as "lock!: ()""#,
            user_id,
            day as &chrono::NaiveDate,
        )
        .execute(&mut **tx)
        .await?;
    }
    let rows = sqlx::query!(
        r#"SELECT d as "day!: chrono::NaiveDate",
                  COALESCE((SELECT SUM(minutes) FROM time_entries
                            WHERE user_id = $1 AND spent_date = d), 0)::bigint as "total!"
           FROM (SELECT DISTINCT d FROM unnest($2::date[]) d) days ORDER BY d"#,
        user_id,
        dates as &[chrono::NaiveDate],
    )
    .fetch_all(&mut **tx)
    .await?;
    Ok(rows.into_iter().map(|row| (row.day, row.total)).collect())
}

/// Open only the submission barrier for tests that coordinate competing writers.
#[cfg(any(test, feature = "test-support"))]
pub async fn begin_time_entry_write(
    pool: &PgPool,
    user_id: uuid::Uuid,
) -> Result<sqlx::Transaction<'_, sqlx::Postgres>, sqlx::Error> {
    let mut tx = pool.begin().await?;
    lock_time_entry_write(&mut tx, user_id).await?;
    Ok(tx)
}

/// Drop everything this application owns and re-apply the migrations.
///
/// Destructive, and dev-only. Both schemas go: `public` holds the tables and
/// the migration ledger, `tower_sessions` the session store, which the session
/// layer recreates on the next start. Dropping the schemas rather than the
/// database is what lets this run over the pool that is already open — a
/// session cannot drop the database it is connected to.
pub async fn reset(pool: &PgPool) -> anyhow::Result<()> {
    sqlx::raw_sql(
        r#"DROP SCHEMA IF EXISTS tower_sessions CASCADE;
           DROP SCHEMA public CASCADE;
           CREATE SCHEMA public;"#,
    )
    .execute(pool)
    .await?;
    run_migrations(pool).await?;
    Ok(())
}

/// The org's rounding config, `(round_minutes, round_dir)`, ready for
/// `horae_core::rounding::round`. The executor generic lets callers pass the
/// pool or an open transaction. `round_minutes` is cast here: the column is
/// CHECK-constrained non-negative, so `as u32` cannot lose a
/// sign.
pub async fn org_rounding(
    ex: impl sqlx::PgExecutor<'_>,
    org_id: uuid::Uuid,
) -> Result<(u32, horae_core::types::RoundDir), sqlx::Error> {
    let row = sqlx::query!(
        r#"SELECT round_minutes, round_dir as "round_dir: horae_core::types::RoundDir"
           FROM organizations WHERE id = $1"#,
        org_id,
    )
    .fetch_one(ex)
    .await?;
    Ok((row.round_minutes as u32, row.round_dir))
}

#[cfg(test)]
mod tests {
    use super::{create_pool, reset};

    /// The startup option only reaches the server if it survives parsing the
    /// URL and the pool's own connect path, so assert it on a pooled
    /// connection rather than trusting the builder.
    #[tokio::test]
    async fn pool_connections_force_custom_plans() {
        let url = std::env::var("DATABASE_URL").expect("DATABASE_URL");
        let pool = create_pool(&url).await.unwrap();
        let mode: String = sqlx::query_scalar("SHOW plan_cache_mode")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(mode, "force_custom_plan");
    }

    /// `migrate reset --confirm` promises a reset, so assert data is actually
    /// gone afterwards and the schema is back.
    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    async fn reset_empties_the_database_and_reapplies_migrations(pool: sqlx::PgPool) {
        sqlx::query!(
            "INSERT INTO organizations (id, name) VALUES ($1, 'Doomed')",
            uuid::Uuid::now_v7(),
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::raw_sql("CREATE SCHEMA tower_sessions")
            .execute(&pool)
            .await
            .unwrap();

        reset(&pool).await.unwrap();

        let orgs: i64 = sqlx::query_scalar!("SELECT COUNT(*) FROM organizations")
            .fetch_one(&pool)
            .await
            .unwrap()
            .unwrap_or(-1);
        assert_eq!(orgs, 0);

        let sessions: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM pg_namespace WHERE nspname = 'tower_sessions')",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(!sessions, "the session schema should be gone too");
    }

    /// `catalogue_updated_at` backfills existing rows from their creation time,
    /// without counting as a project edit, and a later edit moves it.
    #[sqlx::test(migrations = false)]
    async fn catalogue_update_times_are_backfilled_and_follow_updates(pool: sqlx::PgPool) {
        // Only the migrations before it: later ones may rely on its columns.
        let at = super::MIGRATOR
            .iter()
            .position(|migration| migration.description == "catalogue updated at")
            .expect("the catalogue_updated_at migration");
        let (before, catalogue) = super::MIGRATOR.migrations.split_at(at);
        for migration in before {
            sqlx::raw_sql(&migration.sql).execute(&pool).await.unwrap();
        }
        let ids = crate::testing::seed_tenant(&pool).await;
        let created = "2024-01-02T03:04:05Z";
        sqlx::raw_sql(&format!(
            "UPDATE projects SET created_at = '{created}';
             UPDATE clients SET created_at = '{created}';
             UPDATE users SET created_at = '{created}';"
        ))
        .execute(&pool)
        .await
        .unwrap();
        let revision = || async {
            sqlx::query_scalar!(
                "SELECT edit_revision FROM projects WHERE id = $1",
                ids.project_id
            )
            .fetch_one(&pool)
            .await
            .unwrap()
        };
        let revision_before = revision().await;

        sqlx::raw_sql(&catalogue[0].sql)
            .execute(&pool)
            .await
            .unwrap();

        let times = || async {
            sqlx::query!(
                r#"SELECT
                     (SELECT updated_at FROM projects WHERE id = $1) AS "project!: chrono::DateTime<chrono::Utc>",
                     (SELECT updated_at FROM clients WHERE id = $2) AS "client!: chrono::DateTime<chrono::Utc>",
                     (SELECT updated_at FROM users WHERE id = $3) AS "user!: chrono::DateTime<chrono::Utc>",
                     (SELECT created_at FROM tasks WHERE id = $4) AS "task_created!: chrono::DateTime<chrono::Utc>",
                     (SELECT updated_at FROM tasks WHERE id = $4) AS "task!: chrono::DateTime<chrono::Utc>""#,
                ids.project_id,
                ids.client_id,
                ids.user_id,
                ids.task_id,
            )
            .fetch_one(&pool)
            .await
            .unwrap()
        };
        let backfilled = times().await;
        let created: chrono::DateTime<chrono::Utc> = created.parse().unwrap();
        assert_eq!(
            (backfilled.project, backfilled.client, backfilled.user),
            (created, created, created)
        );
        assert_eq!(backfilled.task, backfilled.task_created);
        assert_eq!(revision().await, revision_before, "the backfill is no edit");

        sqlx::raw_sql(
            "UPDATE projects SET name = name || '!';
             UPDATE clients SET name = name || '!';
             UPDATE users SET name = name || '!';
             UPDATE tasks SET name = name || '!';",
        )
        .execute(&pool)
        .await
        .unwrap();
        let updated = times().await;
        assert!(updated.project > backfilled.project);
        assert!(updated.client > backfilled.client);
        assert!(updated.user > backfilled.user);
        assert!(updated.task > backfilled.task);
        assert_eq!(updated.task_created, backfilled.task_created);
    }

    /// An UPDATE that writes the same values keeps `updated_at` and the
    /// project's edit revision; one that changes a value moves both.
    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    async fn updated_at_moves_only_when_a_row_changes(pool: sqlx::PgPool) {
        let ids = crate::testing::seed_tenant(&pool).await;
        let entry =
            crate::testing::time_entry(&pool, &ids, horae_core::types::EntryState::Open).await;
        let stamps = || async {
            sqlx::query!(
                r#"SELECT
                     (SELECT updated_at FROM projects WHERE id = $1) AS "project!: chrono::DateTime<chrono::Utc>",
                     (SELECT edit_revision FROM projects WHERE id = $1) AS "revision!",
                     (SELECT updated_at FROM clients WHERE id = $2) AS "client!: chrono::DateTime<chrono::Utc>",
                     (SELECT updated_at FROM users WHERE id = $3) AS "user!: chrono::DateTime<chrono::Utc>",
                     (SELECT updated_at FROM tasks WHERE id = $4) AS "task!: chrono::DateTime<chrono::Utc>",
                     (SELECT updated_at FROM time_entries WHERE id = $5) AS "entry!: chrono::DateTime<chrono::Utc>""#,
                ids.project_id,
                ids.client_id,
                ids.user_id,
                ids.task_id,
                entry,
            )
            .fetch_one(&pool)
            .await
            .unwrap()
        };
        let before = stamps().await;

        sqlx::raw_sql(
            "UPDATE projects SET name = name;
             UPDATE clients SET name = name;
             UPDATE users SET name = name;
             UPDATE tasks SET name = name;
             UPDATE time_entries SET minutes = minutes, notes = notes;",
        )
        .execute(&pool)
        .await
        .unwrap();
        let unchanged = stamps().await;
        assert_eq!(
            (
                unchanged.project,
                unchanged.revision,
                unchanged.client,
                unchanged.user,
                unchanged.task,
                unchanged.entry
            ),
            (
                before.project,
                before.revision,
                before.client,
                before.user,
                before.task,
                before.entry
            ),
            "a write of the same values is no change"
        );

        sqlx::raw_sql(
            "UPDATE projects SET name = name || '!';
             UPDATE clients SET name = name || '!';
             UPDATE users SET name = name || '!';
             UPDATE tasks SET name = name || '!';
             UPDATE time_entries SET minutes = minutes + 1;",
        )
        .execute(&pool)
        .await
        .unwrap();
        let changed = stamps().await;
        assert!(changed.project > before.project);
        assert_eq!(changed.revision, before.revision + 1);
        assert!(changed.client > before.client);
        assert!(changed.user > before.user);
        assert!(changed.task > before.task);
        assert!(changed.entry > before.entry);
    }

    /// `task_rate_templates` keeps every task rate's number whatever its
    /// currency, clears zeros, drops the currency column and leaves project
    /// task rates alone.
    #[sqlx::test(migrations = false)]
    async fn task_rate_templates_keep_numbers_and_clear_zeros(pool: sqlx::PgPool) {
        let at = super::MIGRATOR
            .iter()
            .position(|migration| migration.description == "task rate templates")
            .expect("the task_rate_templates migration");
        let (before, templates) = super::MIGRATOR.migrations.split_at(at);
        for migration in before {
            sqlx::raw_sql(&migration.sql).execute(&pool).await.unwrap();
        }
        let ids = crate::testing::seed_tenant(&pool).await;
        let rates = [
            ("Workspace currency", Some(8050), Some("EUR")),
            ("Other currency", Some(12000), Some("USD")),
            ("Zero-decimal currency", Some(500), Some("JPY")),
            ("No currency", Some(7500), None),
            ("Zero in EUR", Some(0), Some("EUR")),
            ("No rate", None, None),
        ];
        // The column is gone after the migration, so these are not checked macros.
        for (name, cents, currency) in rates {
            sqlx::query(
                "INSERT INTO tasks (id, org_id, name, default_rate_cents, default_rate_currency)
                 VALUES ($1, $2, $3, $4, $5)",
            )
            .bind(uuid::Uuid::now_v7())
            .bind(ids.org_id)
            .bind(name)
            .bind(cents)
            .bind(currency)
            .execute(&pool)
            .await
            .unwrap();
        }
        sqlx::query!(
            "INSERT INTO project_tasks (project_id, task_id, billable, rate_cents)
             VALUES ($1, $2, true, 9000)",
            ids.project_id,
            ids.task_id,
        )
        .execute(&pool)
        .await
        .unwrap();
        let project_tasks = || async {
            sqlx::query_scalar!(
                r#"SELECT to_jsonb(pt)::text AS "row!" FROM project_tasks pt
                    ORDER BY pt.project_id, pt.task_id"#
            )
            .fetch_all(&pool)
            .await
            .unwrap()
        };
        let project_tasks_before = project_tasks().await;
        assert!(!project_tasks_before.is_empty());

        sqlx::raw_sql(&templates[0].sql)
            .execute(&pool)
            .await
            .unwrap();

        let stored = sqlx::query!(
            "SELECT name, default_rate_cents FROM tasks
              WHERE org_id = $1 AND name <> 'Dev' ORDER BY id",
            ids.org_id,
        )
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(
            stored
                .iter()
                .map(|task| (task.name.as_str(), task.default_rate_cents))
                .collect::<Vec<_>>(),
            [
                ("Workspace currency", Some(8050)),
                ("Other currency", Some(12000)),
                ("Zero-decimal currency", Some(500)),
                ("No currency", Some(7500)),
                ("Zero in EUR", None),
                ("No rate", None),
            ]
        );
        let currency_column = sqlx::query_scalar!(
            r#"SELECT EXISTS (SELECT 1 FROM information_schema.columns
                WHERE table_name = 'tasks' AND column_name = 'default_rate_currency') AS "exists!""#
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(!currency_column);
        assert_eq!(project_tasks().await, project_tasks_before);
        let zero = sqlx::query!(
            "UPDATE tasks SET default_rate_cents = 0 WHERE org_id = $1",
            ids.org_id
        )
        .execute(&pool)
        .await;
        assert!(zero.is_err(), "a zero template is refused");
    }

    /// `invoice_rounding` stops on a Horae invoice with fractional yen rather
    /// than rewrite it, and passes once it is gone; two-decimal invoices and
    /// imported ones with whole amounts carry over unchanged, half up.
    #[sqlx::test(migrations = false)]
    async fn invoice_rounding_stops_on_fractional_zero_decimal_invoices(pool: sqlx::PgPool) {
        let at = super::MIGRATOR
            .iter()
            .position(|migration| migration.description == "invoice rounding")
            .expect("the invoice_rounding migration");
        let (before, rounding) = super::MIGRATOR.migrations.split_at(at);
        for migration in before {
            sqlx::raw_sql(&migration.sql).execute(&pool).await.unwrap();
        }
        let ids = crate::testing::seed_tenant(&pool).await;
        // (number, currency, subtotal, tax, source): 8% of 123.45 and of JPY 12,345.
        let invoices = [
            ("EUR-1", "EUR", 12_345_i64, 988_i64, "horae"),
            ("JPY-1", "JPY", 1_234_500, 98_760, "horae"),
            ("JPY-2", "JPY", 1_000_000, 80_000, "horae"),
            ("H-1", "JPY", 1_234_500, 0, "harvest"),
        ];
        for (number, currency, subtotal, tax, source) in invoices {
            // The `rounding` column does not exist yet, so these are not checked macros.
            sqlx::query(
                "INSERT INTO invoices (id, org_id, client_id, number, issued_on, due_on, currency,
                                       total_cents, tax1_bps, tax1_cents, source)
                 VALUES ($1, $2, $3, $4, current_date, current_date, $5, $6,
                         CASE WHEN $8 = 'horae' THEN 800 ELSE 0 END, $7, $8::invoice_source)",
            )
            .bind(uuid::Uuid::now_v7())
            .bind(ids.org_id)
            .bind(ids.client_id)
            .bind(number)
            .bind(currency)
            .bind(subtotal + tax)
            .bind(tax)
            .bind(source)
            .execute(&pool)
            .await
            .unwrap();
        }
        let snapshot = || async {
            sqlx::query_scalar!(
                r#"SELECT (to_jsonb(i) - 'rounding')::text AS "row!" FROM invoices i ORDER BY number"#
            )
            .fetch_all(&pool)
            .await
            .unwrap()
        };
        let carried = snapshot().await;

        let error = sqlx::raw_sql(&rounding[0].sql)
            .execute(&pool)
            .await
            .unwrap_err()
            .to_string();
        assert!(error.contains("JPY-1"), "{error}");
        assert!(
            !error.contains("JPY-2") && !error.contains("H-1"),
            "{error}"
        );
        assert_eq!(snapshot().await, carried, "nothing is rewritten");

        sqlx::query!("DELETE FROM invoices WHERE number = 'JPY-1'")
            .execute(&pool)
            .await
            .unwrap();
        let carried = snapshot().await;
        sqlx::raw_sql(&rounding[0].sql)
            .execute(&pool)
            .await
            .unwrap();
        assert_eq!(snapshot().await, carried);
        let methods = sqlx::query_scalar!(r#"SELECT rounding::text AS "m!" FROM invoices"#)
            .fetch_all(&pool)
            .await
            .unwrap();
        assert!(methods.iter().all(|method| method == "half_up"));
    }

    /// `fee_amount_steps` stops on a zero-decimal project with a fractional
    /// fee, naming it, and rewrites nothing; once the fee is whole it passes,
    /// and hundredths stay fine in a two-decimal currency.
    #[sqlx::test(migrations = false)]
    async fn fee_amount_steps_stop_on_fractional_zero_decimal_fees(pool: sqlx::PgPool) {
        let at = super::MIGRATOR
            .iter()
            .position(|migration| migration.description == "fee amount steps")
            .expect("the fee_amount_steps migration");
        let (before, steps) = super::MIGRATOR.migrations.split_at(at);
        for migration in before {
            sqlx::raw_sql(&migration.sql).execute(&pool).await.unwrap();
        }
        let ids = crate::testing::seed_tenant(&pool).await;
        let euro = crate::testing::seed_tenant(&pool).await;
        for (ids, currency) in [(&ids, "JPY"), (&euro, "EUR")] {
            sqlx::query!(
                "UPDATE projects SET currency = $2, project_type = 'fixed_fee' WHERE id = $1",
                ids.project_id,
                currency
            )
            .execute(&pool)
            .await
            .unwrap();
            sqlx::query!(
                "INSERT INTO project_settings (id, org_id, project_id, creator_id, rate_mode, fee_mode, fee_amount_cents)
                 VALUES ($1, $2, $3, $4, 'person', 'single', 10050)",
                uuid::Uuid::now_v7(),
                ids.org_id,
                ids.project_id,
                ids.user_id
            )
            .execute(&pool)
            .await
            .unwrap();
        }
        let error = sqlx::raw_sql(&steps[0].sql)
            .execute(&pool)
            .await
            .unwrap_err()
            .to_string();
        assert!(error.contains(&ids.project_id.to_string()), "{error}");
        assert!(!error.contains(&euro.project_id.to_string()), "{error}");
        let fee = || async {
            sqlx::query_scalar!(
                "SELECT fee_amount_cents FROM project_settings WHERE project_id = $1",
                ids.project_id
            )
            .fetch_one(&pool)
            .await
            .unwrap()
        };
        assert_eq!(fee().await, Some(10050), "nothing is rewritten");

        sqlx::query!(
            "UPDATE project_settings SET fee_amount_cents = 10000 WHERE project_id = $1",
            ids.project_id
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::raw_sql(&steps[0].sql).execute(&pool).await.unwrap();
        assert_eq!(fee().await, Some(10000));
    }
}
