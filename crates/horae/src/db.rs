use sqlx::{PgPool, postgres::PgPoolOptions};
use std::time::Duration;

/// Create the shared Postgres pool. The maximum connection count is read from
/// `HORAE_DB_MAX_CONNECTIONS` (defaults to 10 when unset or unparseable).
pub async fn create_pool(database_url: &str) -> anyhow::Result<PgPool> {
    let max_connections = std::env::var("HORAE_DB_MAX_CONNECTIONS")
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(10);
    let pool = PgPoolOptions::new()
        .max_connections(max_connections)
        .acquire_timeout(Duration::from_secs(3))
        .connect(database_url)
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
