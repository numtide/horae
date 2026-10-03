use super::*;

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

#[sqlx::test(migrations = "./migrations")]
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
    let id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO users (id, org_id, email, name, org_role) VALUES ($1, $2, $3, 'Other Admin', 'admin')",
        id, org, format!("{id}@test.com"),
    ).execute(pool).await.unwrap();
    id
}

#[sqlx::test(migrations = "./migrations")]
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

#[sqlx::test(migrations = "./migrations")]
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
            sqlx::query!(
                "UPDATE users SET active = false, org_role = 'member' WHERE id = $1",
                ids.user_id
            )
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

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn original_requester_foreign_key_preserves_tenant_and_known_identity(pool: PgPool) {
    let (ids, job) = fixture(&pool).await;
    let foreign = seed(&pool, OrgRole::Admin).await;
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
    let original = another_admin(&pool, ids.org_id).await;
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

async fn legacy_content(pool: &PgPool) -> serde_json::Value {
    sqlx::query_scalar!(
        r#"SELECT jsonb_build_array(
            (SELECT jsonb_agg(to_jsonb(j) - 'original_requester_id' ORDER BY id) FROM horae_jobs j),
            (SELECT jsonb_agg(to_jsonb(u) ORDER BY job_id) FROM horae_job_uploads u),
            (SELECT jsonb_agg(to_jsonb(c) ORDER BY id) FROM horae_job_report_error_chunks c),
            (SELECT jsonb_agg(to_jsonb(u) ORDER BY id) FROM users u)
        ) AS "content!""#
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

#[sqlx::test(migrations = false)]
#[serial_test::serial]
async fn requester_migration_preserves_unknown_authors_in_all_job_states(pool: PgPool) {
    let mut previous = sqlx::migrate!("./migrations");
    previous.migrations = std::borrow::Cow::Owned(
        previous
            .iter()
            .filter(|m| m.version < 45)
            .cloned()
            .collect(),
    );
    previous.run(&pool).await.unwrap();
    let ids = seed(&pool, OrgRole::Admin).await;
    let mut jobs = Vec::new();
    for status in ["queued", "running", "succeeded", "failed", "cancelled"] {
        let id = Uuid::now_v7();
        let token = Uuid::now_v7();
        sqlx::query!(
            "INSERT INTO horae_jobs (id, org_id, kind, payload, status, checkpoint, report, claim_token, lease_until, account_generation)
             VALUES ($1, $2, 'harvest_csv_import', '{\"legacy\":true}', $3, '{\"cursor\":500}', '{\"retained\":true}', $4, now() + interval '1 hour', 7)",
            id, ids.org_id, status, token,
        ).execute(&pool).await.unwrap();
        sqlx::query!(
            "INSERT INTO horae_job_uploads (job_id, org_id, filename, content_type, body) VALUES ($1, $2, 'retained.csv', 'text/csv', $3)",
            id, ids.org_id, CSV,
        ).execute(&pool).await.unwrap();
        let chunk = Uuid::now_v7();
        sqlx::query!(
            "INSERT INTO horae_job_report_error_chunks (id, job_id, org_id, sequence, body) VALUES ($1, $2, $3, 0, $4)",
            chunk, id, ids.org_id, b"retained error\n".as_slice(),
        ).execute(&pool).await.unwrap();
        jobs.push(id);
    }
    let before = legacy_content(&pool).await;
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    assert_eq!(legacy_content(&pool).await, before);
    for id in jobs {
        assert_eq!(requester(&pool, id).await, None);
    }
}
