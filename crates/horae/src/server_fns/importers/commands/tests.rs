use super::*;
use crate::server_fns::test_seed::{SeedIds, seed, wait_for_blocked};
use std::time::Duration;

const CSV: &[u8] = b"Date,Client,Project,Task,Hours,Email\n";

#[derive(Clone, Copy, Debug)]
enum Operation {
    Api,
    Csv,
    Cancel,
    Retry,
    Status,
    History,
}
const OPERATIONS: [Operation; 6] = [
    Operation::Api,
    Operation::Csv,
    Operation::Cancel,
    Operation::Retry,
    Operation::Status,
    Operation::History,
];

fn payload() -> jobs::JobPayload {
    jobs::JobPayload::HarvestApi {
        mode: ImportMode::DryRun,
        sync: SyncScope::Full,
    }
}

async fn fixture(pool: &PgPool) -> (SeedIds, Uuid) {
    let ids = seed(pool, OrgRole::Admin).await;
    crate::importers::harvest::credentials::store(
        pool,
        ids.org_id,
        ids.user_id,
        &"11".repeat(32),
        "original",
        "access",
        "refresh",
        None,
        None,
    )
    .await
    .unwrap();
    let job = jobs::enqueue_csv(
        pool,
        ids.org_id,
        ImportMode::DryRun,
        CSV.to_vec(),
        "existing",
        JobPolicy::default(),
    )
    .await
    .unwrap();
    jobs::cancel(pool, ids.org_id, job).await.unwrap();
    (ids, job)
}

async fn run(
    pool: &PgPool,
    org: Uuid,
    actor: Uuid,
    job: Uuid,
    operation: Operation,
) -> Result<(), ServerFnError> {
    match operation {
        Operation::Api => start_api(pool, org, actor, &payload(), "new", JobPolicy::default(), 0)
            .await
            .map(|_| ()),
        Operation::Csv => start_csv(
            pool,
            org,
            actor,
            ImportMode::DryRun,
            CSV.to_vec(),
            "new",
            JobPolicy::default(),
        )
        .await
        .map(|_| ()),
        Operation::Cancel => cancel(pool, org, actor, job).await.map(|_| ()),
        Operation::Retry => retry(pool, org, actor, job).await.map(|_| ()),
        Operation::Status => status(pool, org, actor, job).await.map(|_| ()),
        Operation::History => history(pool, org, actor, 20, None).await.map(|_| ()),
    }
}

async fn snapshot(pool: &PgPool) -> serde_json::Value {
    sqlx::query_scalar!(
        r#"SELECT jsonb_build_array(
        (SELECT jsonb_agg(to_jsonb(j) ORDER BY id) FROM horae_jobs j),
        (SELECT jsonb_agg(to_jsonb(u) ORDER BY job_id) FROM horae_job_uploads u),
        (SELECT jsonb_agg(to_jsonb(g) ORDER BY org_id) FROM harvest_connection_generations g)
    ) AS "snapshot!""#
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

fn assert_denied(result: Result<(), ServerFnError>, operation: Operation) {
    assert!(
        matches!(
            result,
            Err(ServerFnError::ServerError {
                code: FORBIDDEN,
                ..
            })
        ),
        "{operation:?} must deny stale authority: {result:?}"
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn commands_and_status_require_current_active_tenant_administrator(pool: PgPool) {
    for operation in OPERATIONS {
        let (ids, job) = fixture(&pool).await;
        let foreign = seed(&pool, OrgRole::Admin).await;
        let before = snapshot(&pool).await;
        for (role, active) in [
            (OrgRole::Member, true),
            (OrgRole::Manager, true),
            (OrgRole::Admin, false),
        ] {
            sqlx::query!(
                "UPDATE users SET org_role = $2, active = $3 WHERE id = $1",
                ids.user_id,
                role as OrgRole,
                active
            )
            .execute(&pool)
            .await
            .unwrap();
            assert_denied(
                run(&pool, ids.org_id, ids.user_id, job, operation).await,
                operation,
            );
            assert_eq!(snapshot(&pool).await, before);
        }
        for actor in [foreign.user_id, Uuid::now_v7()] {
            assert_denied(
                run(&pool, ids.org_id, actor, job, operation).await,
                operation,
            );
            assert_eq!(snapshot(&pool).await, before);
        }
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn duplicate_submission_and_noop_cancellation_do_not_bypass_revocation(pool: PgPool) {
    let (ids, job) = fixture(&pool).await;
    sqlx::query!(
        "UPDATE users SET org_role = 'member' WHERE id = $1",
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let before = snapshot(&pool).await;
    assert_denied(
        start_csv(
            &pool,
            ids.org_id,
            ids.user_id,
            ImportMode::DryRun,
            CSV.to_vec(),
            "existing",
            JobPolicy::default(),
        )
        .await
        .map(|_| ()),
        Operation::Csv,
    );
    for key in ["existing", "missing"] {
        assert_denied(
            start_csv(
                &pool,
                ids.org_id,
                ids.user_id,
                ImportMode::DryRun,
                b"invalid headers".to_vec(),
                key,
                JobPolicy::default(),
            )
            .await
            .map(|_| ()),
            Operation::Csv,
        );
    }
    assert_denied(
        cancel(&pool, ids.org_id, ids.user_id, job)
            .await
            .map(|_| ()),
        Operation::Cancel,
    );
    assert_eq!(snapshot(&pool).await, before);
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn import_access_rechecks_revocation_after_organization_and_actor_waits(pool: PgPool) {
    let run_pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .after_connect(|connection, _| {
            Box::pin(async move {
                sqlx::query!(
                    "SET SESSION CHARACTERISTICS AS TRANSACTION ISOLATION LEVEL REPEATABLE READ"
                )
                .execute(connection)
                .await?;
                Ok(())
            })
        })
        .connect_with((*pool.connect_options()).clone())
        .await
        .unwrap();
    for operation in OPERATIONS {
        for actor_only in [false, true] {
            for deactivate in [false, true] {
                let (ids, job) = fixture(&pool).await;
                let before = snapshot(&pool).await;
                let mut revoke = pool.begin().await.unwrap();
                let blocker = sqlx::query_scalar!("SELECT pg_backend_pid()")
                    .fetch_one(&mut *revoke)
                    .await
                    .unwrap()
                    .unwrap();
                if !actor_only {
                    sqlx::query!(
                        "SELECT id FROM organizations WHERE id = $1 FOR UPDATE",
                        ids.org_id
                    )
                    .fetch_one(&mut *revoke)
                    .await
                    .unwrap();
                }
                sqlx::query!(
                    "UPDATE users SET org_role = $2, active = $3 WHERE id = $1",
                    ids.user_id,
                    if deactivate {
                        OrgRole::Admin
                    } else {
                        OrgRole::Member
                    } as OrgRole,
                    !deactivate
                )
                .execute(&mut *revoke)
                .await
                .unwrap();
                let db = run_pool.clone();
                let mut pending = tokio::task::JoinSet::new();
                pending
                    .spawn(async move { run(&db, ids.org_id, ids.user_id, job, operation).await });
                tokio::select! {
                    result = pending.join_next() => panic!("{operation:?} bypassed revocation wait: {result:?}"),
                    () = wait_for_blocked(&pool, blocker) => {},
                }
                revoke.commit().await.unwrap();
                assert_denied(
                    tokio::time::timeout(Duration::from_secs(5), pending.join_next())
                        .await
                        .unwrap()
                        .unwrap()
                        .unwrap(),
                    operation,
                );
                assert_eq!(snapshot(&pool).await, before);
            }
        }
    }
    run_pool.close().await;
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn import_commands_and_results_hold_actor_authority_until_commit(pool: PgPool) {
    for operation in OPERATIONS {
        let (ids, job) = fixture(&pool).await;
        let mut barrier = pool.begin().await.unwrap();
        let blocker = sqlx::query_scalar!("SELECT pg_backend_pid()")
            .fetch_one(&mut *barrier)
            .await
            .unwrap()
            .unwrap();
        // All command/status paths reach the job table after authorization.
        sqlx::query!("LOCK TABLE horae_jobs IN ACCESS EXCLUSIVE MODE")
            .execute(&mut *barrier)
            .await
            .unwrap();
        let db = pool.clone();
        let mut command = tokio::task::JoinSet::new();
        command.spawn(async move { run(&db, ids.org_id, ids.user_id, job, operation).await });
        wait_for_blocked(&pool, blocker).await;
        let writer = sqlx::query_scalar!(
            "SELECT pid AS \"pid!\" FROM pg_stat_activity WHERE datname=current_database() AND $1=ANY(pg_blocking_pids(pid))",
            blocker,
        ).fetch_one(&pool).await.unwrap();
        let db = pool.clone();
        let mut revoke = tokio::task::JoinSet::new();
        revoke.spawn(async move {
            sqlx::query!(
                "UPDATE users SET org_role = 'member' WHERE id = $1",
                ids.user_id
            )
            .execute(&db)
            .await
        });
        tokio::select! {
            result = revoke.join_next() => panic!("{operation:?} released authority early: {result:?}"),
            () = wait_for_blocked(&pool, writer) => {},
        }
        barrier.commit().await.unwrap();
        tokio::time::timeout(Duration::from_secs(5), command.join_next())
            .await
            .unwrap()
            .unwrap()
            .unwrap()
            .unwrap();
        tokio::time::timeout(Duration::from_secs(5), revoke.join_next())
            .await
            .unwrap()
            .unwrap()
            .unwrap()
            .unwrap();
        let before = snapshot(&pool).await;
        assert_denied(
            run(&pool, ids.org_id, ids.user_id, job, operation).await,
            operation,
        );
        assert_eq!(snapshot(&pool).await, before);
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn foreign_and_missing_jobs_never_reveal_or_change_another_tenant(pool: PgPool) {
    let (owner, job) = fixture(&pool).await;
    let foreign = seed(&pool, OrgRole::Admin).await;
    let before = snapshot(&pool).await;
    for id in [job, Uuid::now_v7()] {
        assert!(
            status(&pool, foreign.org_id, foreign.user_id, id)
                .await
                .unwrap()
                .is_none()
        );
        assert!(
            history(&pool, foreign.org_id, foreign.user_id, 20, Some(id))
                .await
                .unwrap()
                .is_empty()
        );
        for result in [
            cancel(&pool, foreign.org_id, foreign.user_id, id).await,
            retry(&pool, foreign.org_id, foreign.user_id, id).await,
        ] {
            assert!(
                matches!(
                    result,
                    Err(ServerFnError::ServerError {
                        code: NOT_FOUND,
                        ..
                    })
                ),
                "{result:?}"
            );
        }
    }
    for operation in OPERATIONS {
        assert_denied(
            run(&pool, Uuid::now_v7(), owner.user_id, job, operation).await,
            operation,
        );
    }
    assert_eq!(snapshot(&pool).await, before);
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn authorized_job_lifecycle_and_duplicate_retention_work_with_one_connection(pool: PgPool) {
    let (ids, existing) = fixture(&pool).await;
    let single = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect_with((*pool.connect_options()).clone())
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(5), async {
        let api = start_api(
            &single,
            ids.org_id,
            ids.user_id,
            &payload(),
            "api",
            JobPolicy::default(),
            0,
        )
        .await
        .unwrap();
        assert_eq!(api.status, "queued");
        let duplicate = start_api(
            &single,
            ids.org_id,
            ids.user_id,
            &payload(),
            "api",
            JobPolicy::default(),
            0,
        )
        .await
        .unwrap();
        assert_eq!(duplicate.id, api.id);
        let csv = start_csv(
            &single,
            ids.org_id,
            ids.user_id,
            ImportMode::DryRun,
            CSV.to_vec(),
            "csv",
            JobPolicy::default(),
        )
        .await
        .unwrap();
        assert_eq!(csv.status, "queued");
        assert_eq!(
            cancel(&single, ids.org_id, ids.user_id, csv.id)
                .await
                .unwrap()
                .status,
            "cancelled"
        );
        assert_eq!(
            cancel(&single, ids.org_id, ids.user_id, csv.id)
                .await
                .unwrap()
                .status,
            "cancelled"
        );
        assert_eq!(
            retry(&single, ids.org_id, ids.user_id, csv.id)
                .await
                .unwrap()
                .status,
            "queued"
        );
        assert!(matches!(
            retry(&single, ids.org_id, ids.user_id, csv.id).await,
            Err(ServerFnError::ServerError {
                code: NOT_FOUND,
                ..
            })
        ));
        assert_eq!(
            status(&single, ids.org_id, ids.user_id, csv.id)
                .await
                .unwrap()
                .unwrap()
                .status,
            "queued"
        );
        assert_eq!(
            history(&single, ids.org_id, ids.user_id, 100, None)
                .await
                .unwrap()
                .len(),
            3
        );
        sqlx::query!("DELETE FROM horae_job_uploads WHERE job_id = $1", existing)
            .execute(&single)
            .await
            .unwrap();
        let replay = start_csv(
            &single,
            ids.org_id,
            ids.user_id,
            ImportMode::DryRun,
            CSV.to_vec(),
            "existing",
            JobPolicy::default(),
        )
        .await
        .unwrap();
        assert_eq!(replay.id, existing);
        assert!(matches!(
            replay.retry_availability,
            crate::models::RetryAvailability::MissingUpload
        ));
        assert!(matches!(
            retry(&single, ids.org_id, ids.user_id, existing).await,
            Err(ServerFnError::ServerError {
                code: NOT_FOUND,
                ..
            })
        ));
    })
    .await
    .expect("authorized commands must not acquire a second connection");
    single.close().await;
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn failed_upload_insert_rolls_back_the_job_and_releases_authority(pool: PgPool) {
    let (ids, _) = fixture(&pool).await;
    let before = snapshot(&pool).await;
    sqlx::query!(
        "ALTER TABLE horae_job_uploads ADD CONSTRAINT reject_command_upload CHECK (false) NOT VALID"
    )
    .execute(&pool)
    .await
    .unwrap();
    let failed = start_csv(
        &pool,
        ids.org_id,
        ids.user_id,
        ImportMode::DryRun,
        CSV.to_vec(),
        "failed",
        JobPolicy::default(),
    )
    .await;
    assert!(
        matches!(
            failed,
            Err(ServerFnError::ServerError {
                code: INTERNAL_ERROR,
                ..
            })
        ),
        "{failed:?}"
    );
    assert_eq!(snapshot(&pool).await, before);
    tokio::time::timeout(Duration::from_secs(5), async {
        let mut probe = pool.begin().await.unwrap();
        sqlx::query!(
            "SELECT id FROM organizations WHERE id = $1 FOR UPDATE",
            ids.org_id
        )
        .fetch_one(&mut *probe)
        .await
        .unwrap();
        sqlx::query!("SELECT id FROM users WHERE id = $1 FOR UPDATE", ids.user_id)
            .fetch_one(&mut *probe)
            .await
            .unwrap();
        probe.rollback().await.unwrap();
    })
    .await
    .expect("rollback must release authority locks");
    sqlx::query!("ALTER TABLE horae_job_uploads DROP CONSTRAINT reject_command_upload")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        start_csv(
            &pool,
            ids.org_id,
            ids.user_id,
            ImportMode::DryRun,
            CSV.to_vec(),
            "failed",
            JobPolicy::default()
        )
        .await
        .unwrap()
        .status,
        "queued"
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn conflicting_and_stale_commands_preserve_payload_policy_and_retained_state(pool: PgPool) {
    let (ids, csv) = fixture(&pool).await;
    let api = start_api(
        &pool,
        ids.org_id,
        ids.user_id,
        &payload(),
        "api",
        JobPolicy::default(),
        0,
    )
    .await
    .unwrap();
    cancel(&pool, ids.org_id, ids.user_id, api.id)
        .await
        .unwrap();
    let before = snapshot(&pool).await;
    let conflicting = jobs::JobPayload::HarvestApi {
        mode: ImportMode::Commit,
        sync: SyncScope::Full,
    };
    for result in [
        start_api(
            &pool,
            ids.org_id,
            ids.user_id,
            &conflicting,
            "api",
            JobPolicy::default(),
            0,
        )
        .await,
        start_csv(
            &pool,
            ids.org_id,
            ids.user_id,
            ImportMode::Commit,
            CSV.to_vec(),
            "existing",
            JobPolicy::default(),
        )
        .await,
    ] {
        assert!(
            matches!(
                result,
                Err(ServerFnError::ServerError { code: CONFLICT, .. })
            ),
            "{result:?}"
        );
    }
    assert_eq!(snapshot(&pool).await, before);
    sqlx::query!("UPDATE harvest_connection_generations SET account_generation = account_generation + 1 WHERE org_id = $1", ids.org_id)
        .execute(&pool).await.unwrap();
    let before = snapshot(&pool).await;
    for result in [
        start_api(
            &pool,
            ids.org_id,
            ids.user_id,
            &payload(),
            "api",
            JobPolicy::default(),
            0,
        )
        .await,
        retry(&pool, ids.org_id, ids.user_id, api.id).await,
    ] {
        assert!(
            matches!(
                result,
                Err(ServerFnError::ServerError { code: CONFLICT, .. })
            ),
            "{result:?}"
        );
    }
    assert_eq!(snapshot(&pool).await, before);
    assert_eq!(
        retry(&pool, ids.org_id, ids.user_id, csv)
            .await
            .unwrap()
            .status,
        "queued"
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn authorized_submissions_do_not_wait_for_the_running_import_reservation(pool: PgPool) {
    let (ids, _) = fixture(&pool).await;
    let mut reservation = pool.acquire().await.unwrap();
    reservation.close_on_drop();
    assert!(
        sqlx::query_scalar!(
            r#"SELECT pg_try_advisory_lock(hashtextextended($1, 0)) as "acquired!""#,
            format!("horae:harvest-import:{}", ids.org_id),
        )
        .fetch_one(&mut *reservation)
        .await
        .unwrap()
    );
    tokio::time::timeout(Duration::from_secs(5), async {
        for operation in [Operation::Api, Operation::Csv] {
            run(&pool, ids.org_id, ids.user_id, Uuid::now_v7(), operation)
                .await
                .unwrap();
        }
    })
    .await
    .expect("job acceptance must not acquire the worker reservation");
    sqlx::query!("SELECT pg_advisory_unlock_all()")
        .execute(&mut *reservation)
        .await
        .unwrap();
    reservation.close().await.unwrap();
}
