use super::*;
use horae_core::permissions::catalog::BuiltInProfile;

async fn requester(pool: &PgPool, job: Uuid) -> Option<Uuid> {
    sqlx::query_scalar!(
        "SELECT original_requester_id FROM horae_jobs WHERE id = $1",
        job,
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn submit(
    pool: &PgPool,
    org: Uuid,
    actor: Uuid,
    operation: Operation,
    key: &str,
) -> JobStatus {
    match operation {
        Operation::Api => {
            start_api(pool, org, actor, &payload(), key, JobPolicy::default(), 0).await
        }
        Operation::Csv => {
            start_csv(
                pool,
                org,
                actor,
                ImportMode::DryRun,
                CSV.to_vec(),
                key,
                JobPolicy::default(),
            )
            .await
        }
        _ => unreachable!(),
    }
    .unwrap()
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn new_import_jobs_record_the_authorized_original_requester(pool: PgPool) {
    for operation in [Operation::Api, Operation::Csv] {
        let (ids, _) = fixture(&pool).await;
        let job = submit(&pool, ids.org_id, ids.user_id, operation, "attributed").await;
        assert_eq!(
            requester(&pool, job.id).await,
            Some(ids.user_id),
            "{operation:?}"
        );
        assert!(
            serde_json::to_value(job)
                .unwrap()
                .get("original_requester_id")
                .is_none()
        );
    }
}

async fn another_admin(pool: &PgPool, org: Uuid) -> Uuid {
    crate::ops::testing::add_person(pool, org, BuiltInProfile::Administrator).await
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn waiting_duplicate_keeps_the_first_committed_requester(pool: PgPool) {
    for operation in [Operation::Api, Operation::Csv] {
        let (ids, _) = fixture(&pool).await;
        let other = another_admin(&pool, ids.org_id).await;
        let mut first = begin_access(&pool, ids.org_id, ids.user_id).await.unwrap();
        let id = match operation {
            Operation::Api => {
                jobs::enqueue_api_in(
                    &mut first,
                    ids.org_id,
                    &payload(),
                    "concurrent",
                    JobPolicy::default(),
                    0,
                    ids.user_id,
                )
                .await
            }
            Operation::Csv => {
                jobs::enqueue_csv_in(
                    &mut first,
                    ids.org_id,
                    ImportMode::DryRun,
                    CSV.to_vec(),
                    "concurrent",
                    JobPolicy::default(),
                    ids.user_id,
                )
                .await
            }
            _ => unreachable!(),
        }
        .unwrap();
        let pid = sqlx::query_scalar!("SELECT pg_backend_pid()")
            .fetch_one(&mut *first)
            .await
            .unwrap()
            .unwrap();
        let db = pool.clone();
        let org = ids.org_id;
        let mut waiting = tokio::task::JoinSet::new();
        waiting.spawn(async move { submit(&db, org, other, operation, "concurrent").await });
        wait_for_blocked(&pool, pid).await;
        first.commit().await.unwrap();
        let duplicate = tokio::time::timeout(Duration::from_secs(5), waiting.join_next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(duplicate.id, id);
        assert_eq!(requester(&pool, id).await, Some(ids.user_id));
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn duplicates_and_job_lifecycle_never_replace_or_invent_the_original_requester(pool: PgPool) {
    for operation in [Operation::Api, Operation::Csv] {
        for known in [true, false] {
            let (ids, _) = fixture(&pool).await;
            let other = another_admin(&pool, ids.org_id).await;
            let id = if known {
                submit(&pool, ids.org_id, ids.user_id, operation, "original")
                    .await
                    .id
            } else {
                match operation {
                    Operation::Api => {
                        jobs::enqueue_api(
                            &pool,
                            ids.org_id,
                            &payload(),
                            "original",
                            JobPolicy::default(),
                            0,
                        )
                        .await
                    }
                    Operation::Csv => {
                        jobs::enqueue_csv(
                            &pool,
                            ids.org_id,
                            ImportMode::DryRun,
                            CSV.to_vec(),
                            "original",
                            JobPolicy::default(),
                        )
                        .await
                    }
                    _ => unreachable!(),
                }
                .unwrap()
            };
            let expected = known.then_some(ids.user_id);
            let repeated = submit(&pool, ids.org_id, other, operation, "original").await;
            assert_eq!(repeated.id, id);
            assert_eq!(requester(&pool, id).await, expected);
            sqlx::query!("UPDATE users SET active = false WHERE id = $1", ids.user_id)
                .execute(&pool)
                .await
                .unwrap();
            assert_eq!(
                cancel(&pool, ids.org_id, other, id).await.unwrap().status,
                "cancelled"
            );
            assert_eq!(
                retry(&pool, ids.org_id, other, id).await.unwrap().status,
                "queued"
            );
            assert_eq!(requester(&pool, id).await, expected);
            let (lease, stop) = jobs::claim_lease_for_test(&pool).await;
            assert_eq!(
                jobs::status(&pool, ids.org_id, id)
                    .await
                    .unwrap()
                    .unwrap()
                    .status,
                "running"
            );
            jobs::run_claimed(&pool, &lease, stop, async {
                Ok((repeated.report.unwrap(), 0))
            })
            .await
            .unwrap();
            assert_eq!(
                jobs::status(&pool, ids.org_id, id)
                    .await
                    .unwrap()
                    .unwrap()
                    .status,
                "succeeded"
            );
            assert_eq!(requester(&pool, id).await, expected);
        }
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn original_requester_foreign_key_preserves_tenant_and_known_identity(pool: PgPool) {
    let (ids, job) = fixture(&pool).await;
    let foreign = seed(&pool, BuiltInProfile::Administrator).await;
    for actor in [foreign.user_id, Uuid::now_v7()] {
        let error = sqlx::query!(
            "UPDATE horae_jobs SET original_requester_id = $1 WHERE id = $2",
            actor,
            job
        )
        .execute(&pool)
        .await
        .unwrap_err();
        assert_eq!(
            error.as_database_error().unwrap().constraint(),
            Some("horae_jobs_original_requester_fkey")
        );
        assert_eq!(requester(&pool, job).await, None);
    }
    // No stored permission state, so only the job's reference keeps the row.
    let original = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO users (id, org_id, email, name) VALUES ($1, $2, $3, 'Original requester')",
        original,
        ids.org_id,
        format!("{original}@test.com"),
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE horae_jobs SET original_requester_id = $1 WHERE id = $2",
        original,
        job
    )
    .execute(&pool)
    .await
    .unwrap();
    let before = snapshot(&pool).await;
    let error = sqlx::query!("DELETE FROM users WHERE id = $1", original)
        .execute(&pool)
        .await
        .unwrap_err();
    assert_eq!(
        error.as_database_error().unwrap().constraint(),
        Some("horae_jobs_original_requester_fkey")
    );
    assert_eq!(snapshot(&pool).await, before);
}
