use super::claim_budget_band;
use crate::server_fns::test_seed::seed;
use horae_core::types::OrgRole;
use sqlx::PgPool;

#[sqlx::test(migrations = "./migrations")]
async fn concurrent_budget_checks_have_one_band_claimant(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    sqlx::query!(
        "UPDATE projects SET last_budget_alert_pct = NULL WHERE id = $1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();

    let barrier = std::sync::Arc::new(tokio::sync::Barrier::new(2));
    let first_pool = pool.clone();
    let first_barrier = std::sync::Arc::clone(&barrier);
    let first = tokio::spawn(async move {
        first_barrier.wait().await;
        claim_budget_band(&first_pool, ids.project_id, 0, 80).await
    });
    let second_pool = pool.clone();
    let second_barrier = std::sync::Arc::clone(&barrier);
    let second = tokio::spawn(async move {
        second_barrier.wait().await;
        claim_budget_band(&second_pool, ids.project_id, 0, 80).await
    });

    let (first, second) = tokio::join!(first, second);
    let claimed = [first.unwrap().unwrap(), second.unwrap().unwrap()];
    assert_eq!(claimed.iter().filter(|&&won| won).count(), 1);
    assert_eq!(
        sqlx::query_scalar!(
            "SELECT last_budget_alert_pct FROM projects WHERE id = $1",
            ids.project_id
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        Some(80)
    );
}
