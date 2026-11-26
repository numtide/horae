use std::sync::Arc;

use horae_core::{permissions::catalog::BuiltInProfile, types::EntryState};
use sqlx::PgPool;

use super::{claim_long_timers, ensure_all_schedules, ensure_schedules};
use crate::{
    plugin::PluginRegistry,
    state::AppState,
    testing::{seed, time_entry},
};

async fn start_timer(pool: &PgPool, entry: uuid::Uuid, started: &str) {
    sqlx::query!(
        "UPDATE time_entries SET is_running = true, started_at = now() - $2::text::interval
          WHERE id = $1",
        entry,
        started,
    )
    .execute(pool)
    .await
    .unwrap();
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_long_timer_is_announced_at_most_once_and_only_by_its_organizations_sweep(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Member).await;
    let entry = time_entry(&pool, &ids, EntryState::Open).await;
    let other = seed(&pool, BuiltInProfile::Member).await;
    start_timer(&pool, entry, "9 hours").await;

    assert!(
        claim_long_timers(&pool, other.org_id)
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(claim_long_timers(&pool, ids.org_id).await.unwrap().len(), 1);
    assert!(
        claim_long_timers(&pool, ids.org_id)
            .await
            .unwrap()
            .is_empty()
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_timer_under_the_limit_is_not_announced(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Member).await;
    let entry = time_entry(&pool, &ids, EntryState::Open).await;
    start_timer(&pool, entry, "7 hours").await;
    assert!(
        claim_long_timers(&pool, ids.org_id)
            .await
            .unwrap()
            .is_empty()
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn each_organization_gets_the_three_sweeps_every_minute_once(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Member).await;
    ensure_all_schedules(&pool).await.unwrap();
    let schedules = || {
        sqlx::query!(
            r#"SELECT name, kind, every_secs, concurrency_key, priority, max_attempts,
                      next_due_at AS "next_due_at!"
                 FROM horae_job_schedules WHERE org_id = $1 ORDER BY name"#,
            ids.org_id,
        )
        .fetch_all(&pool)
    };
    let first = schedules().await.unwrap();
    let summary: Vec<_> = first
        .iter()
        .map(|s| {
            (
                s.name.as_str(),
                s.kind.as_str(),
                s.every_secs,
                s.concurrency_key.as_deref(),
                s.priority,
                s.max_attempts,
            )
        })
        .collect();
    assert_eq!(
        summary,
        [
            (
                "budget_alert_sweep",
                "budget_alert_sweep",
                Some(60),
                Some("sweep:budget"),
                0,
                1
            ),
            (
                "long_timer_sweep",
                "long_timer_sweep",
                Some(60),
                Some("sweep:long_timer"),
                0,
                1
            ),
            (
                "oauth_cleanup",
                "oauth_cleanup",
                Some(60),
                Some("sweep:oauth"),
                -1,
                1
            ),
        ]
    );

    // A restart (or `init`) stores them again without moving their next firing.
    let mut connection = pool.acquire().await.unwrap();
    ensure_schedules(&mut connection, ids.org_id).await.unwrap();
    let again = schedules().await.unwrap();
    assert_eq!(
        first.iter().map(|s| s.next_due_at).collect::<Vec<_>>(),
        again.iter().map(|s| s.next_due_at).collect::<Vec<_>>()
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_scheduled_long_timer_sweep_runs_on_the_worker(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Member).await;
    let entry = time_entry(&pool, &ids, EntryState::Open).await;
    start_timer(&pool, entry, "9 hours").await;
    ensure_all_schedules(&pool).await.unwrap();

    let zones = crate::timezone::Zones::new(pool.clone());
    let now = chrono::Utc::now();
    assert_eq!(horae_jobs::fire_due(&pool, &zones, now).await.unwrap(), 3);

    let state = AppState::new(pool.clone(), Arc::new(PluginRegistry::empty()));
    let registry = super::registry(&state);
    while let Some(job) = horae_jobs::claim_registered(&pool, &registry, "test-worker")
        .await
        .unwrap()
    {
        horae_jobs::execute(&pool, &registry, "test-worker", job)
            .await
            .unwrap();
    }
    let statuses = sqlx::query_scalar!(
        r#"SELECT status AS "status!" FROM horae_jobs WHERE org_id = $1"#,
        ids.org_id
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(statuses, ["succeeded"; 3]);
    let notified = sqlx::query_scalar!(
        "SELECT notified_long_running_at IS NOT NULL FROM time_entries WHERE id = $1",
        entry
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(notified, Some(true));
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn sweeps_run_while_an_import_holds_the_import_worker(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Member).await;
    let import = uuid::Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO horae_jobs (id, org_id, kind, payload, idempotency_key, max_attempts)
         VALUES ($1, $2, 'harvest_api_import', '{}', 'long-import', 1)",
        import,
        ids.org_id,
    )
    .execute(&pool)
    .await
    .unwrap();
    let state = AppState::new(pool.clone(), Arc::new(PluginRegistry::empty()));
    // The import worker is busy with the import for as long as it runs.
    let running = horae_jobs::claim_registered(&pool, &super::super::registry(&state), "busy")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(running.id, import);

    ensure_all_schedules(&pool).await.unwrap();
    let zones = crate::timezone::Zones::new(pool.clone());
    horae_jobs::fire_due(&pool, &zones, chrono::Utc::now())
        .await
        .unwrap();
    let sweeps = super::super::spawn_sweeps(&state);
    tokio::time::timeout(std::time::Duration::from_secs(20), async {
        loop {
            let done = sqlx::query_scalar!(
                r#"SELECT count(*) AS "n!" FROM horae_jobs
                    WHERE org_id = $1 AND kind <> 'harvest_api_import' AND status = 'succeeded'"#,
                ids.org_id
            )
            .fetch_one(&pool)
            .await
            .unwrap();
            if done == 3 {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("the three sweeps succeed while the import runs");
    sweeps
        .shutdown(std::time::Duration::from_secs(5))
        .await
        .unwrap();
    let status = sqlx::query_scalar!("SELECT status FROM horae_jobs WHERE id = $1", import)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(status, "running");
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn failed_sweeps_stay_in_the_job_list(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Member).await;
    for status in ["succeeded", "failed"] {
        sqlx::query!(
            "INSERT INTO horae_jobs (id, org_id, kind, payload, status, max_attempts, finished_at)
             VALUES ($1, $2, 'budget_alert_sweep', '{}', $3, 1, now())",
            uuid::Uuid::now_v7(),
            ids.org_id,
            status,
        )
        .execute(&pool)
        .await
        .unwrap();
    }
    let listed = crate::jobs::list(&pool, ids.org_id, 100, None)
        .await
        .unwrap();
    let statuses: Vec<_> = listed.iter().map(|job| job.status.as_str()).collect();
    assert_eq!(statuses, ["failed"]);
}
