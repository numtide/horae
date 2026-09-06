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

pub async fn run_migrations(pool: &PgPool) -> anyhow::Result<()> {
    sqlx::migrate!("./migrations").run(pool).await?;
    Ok(())
}

/// The org's rounding config, `(round_minutes, round_dir)`, ready for
/// `horae_core::rounding::round`. The executor generic lets callers pass the
/// pool or an open transaction. `round_minutes` is cast here: the column is
/// CHECK-constrained non-negative (migration 0010), so `as u32` cannot lose a
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
    use super::create_pool;

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
}
