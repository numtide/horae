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

    /// `zero_task_rates` clears a zero rate without a currency, the Harvest
    /// importer's "no rate", and keeps every other rate.
    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    async fn zero_task_rates_without_currency_become_no_rate(pool: sqlx::PgPool) {
        let ids = crate::testing::seed_tenant(&pool).await;
        let rates = [
            ("Imported zero", Some(0), None),
            ("Zero in EUR", Some(0), Some("EUR")),
            ("Imported rate", Some(8000), None),
            ("No rate", None, None),
        ];
        for (name, cents, currency) in rates {
            sqlx::query!(
                "INSERT INTO tasks (id, org_id, name, default_rate_cents, default_rate_currency)
                 VALUES ($1, $2, $3, $4, $5)",
                uuid::Uuid::now_v7(),
                ids.org_id,
                name,
                cents,
                currency,
            )
            .execute(&pool)
            .await
            .unwrap();
        }
        let repair = super::MIGRATOR
            .iter()
            .find(|migration| migration.description == "zero task rates")
            .expect("the zero_task_rates migration");
        sqlx::raw_sql(&repair.sql).execute(&pool).await.unwrap();

        let stored = sqlx::query!(
            "SELECT name, default_rate_cents, default_rate_currency FROM tasks
              WHERE org_id = $1 AND name <> 'Dev' ORDER BY id",
            ids.org_id,
        )
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(
            stored
                .iter()
                .map(|task| (
                    task.name.as_str(),
                    task.default_rate_cents,
                    task.default_rate_currency.as_deref()
                ))
                .collect::<Vec<_>>(),
            [
                ("Imported zero", None, None),
                ("Zero in EUR", Some(0), Some("EUR")),
                ("Imported rate", Some(8000), None),
                ("No rate", None, None),
            ]
        );
    }
}
