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
