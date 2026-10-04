use super::preflight::{self, PreflightCounts, PreflightError};
use crate::server_fns::test_seed::{seed, wait_for_blocked};
use horae_core::{
    permissions::catalog::BuiltInProfile,
    types::{EntryState, OrgRole},
};
use sqlx::PgPool;
use std::time::Duration;
use uuid::Uuid;

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn preflight_counts_cross_tenant_memberships_from_both_ends(pool: PgPool) {
    let a = seed(&pool, OrgRole::Admin).await;
    let b = seed(&pool, OrgRole::Admin).await;
    let unrelated = seed(&pool, OrgRole::Admin).await;
    assert_eq!(
        preflight::read(&pool, a.org_id, a.user_id).await.unwrap(),
        PreflightCounts::default()
    );
    sqlx::query!(
        "INSERT INTO assignments (id, project_id, user_id) VALUES ($1, $2, $3)",
        Uuid::now_v7(),
        a.project_id,
        b.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let before = history(&pool).await;
    for ids in [&a, &b] {
        let counts = preflight::read(&pool, ids.org_id, ids.user_id)
            .await
            .unwrap();
        assert_eq!(
            counts,
            PreflightCounts {
                cross_org_memberships: 1,
                ..Default::default()
            }
        );
    }
    assert_eq!(
        preflight::read(&pool, unrelated.org_id, unrelated.user_id)
            .await
            .unwrap(),
        PreflightCounts::default()
    );
    assert_eq!(history(&pool).await, before);
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn preflight_approval_diagnostics_preserve_stored_history(pool: PgPool) {
    let a = seed(&pool, OrgRole::Admin).await;
    let b = seed(&pool, OrgRole::Member).await;
    let c = seed(&pool, OrgRole::Admin).await;
    let mut offset = 0;
    for state in [
        EntryState::Open,
        EntryState::Submitted,
        EntryState::Approved,
        EntryState::Invoiced,
    ] {
        for actor in [None, Some(a.user_id), Some(b.user_id)] {
            for timestamp in [false, true] {
                sqlx::query!(
                    "INSERT INTO approvals (id, org_id, user_id, period_start, period_end, state, approved_by, approved_at)
                     VALUES ($1, $2, $3, DATE '2026-01-01' + $4::integer, DATE '2026-01-07' + $4::integer,
                             $5, $6, CASE WHEN $7 THEN now() ELSE NULL END)",
                    Uuid::now_v7(), a.org_id, a.user_id, offset, state as EntryState, actor, timestamp
                ).execute(&pool).await.unwrap();
                offset += 7;
            }
        }
    }
    // Foreign ownership and both foreign references still contribute one row per tenant.
    sqlx::query!(
        "INSERT INTO approvals (id, org_id, user_id, period_start, period_end, state, approved_by, approved_at)
         VALUES ($1, $2, $3, DATE '2028-01-01', DATE '2028-01-07', 'approved', $3, now())",
        Uuid::now_v7(), b.org_id, a.user_id
    ).execute(&pool).await.unwrap();
    let historic_approver = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO users (id, org_id, email, name, org_role, active)
         VALUES ($1, $2, 'historic@test.com', 'Historical approver', 'member', false)",
        historic_approver,
        a.org_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO approvals (id, org_id, user_id, period_start, period_end, state, approved_by, approved_at)
         VALUES ($1, $2, $3, DATE '2029-01-01', DATE '2029-01-07', 'approved', $4, now())",
        Uuid::now_v7(), a.org_id, a.user_id, historic_approver
    ).execute(&pool).await.unwrap();
    let before = history(&pool).await;
    assert_eq!(
        preflight::read(&pool, a.org_id, a.user_id).await.unwrap(),
        PreflightCounts {
            cross_org_approvals: 9,
            incomplete_approval_attribution: 4,
            unexpected_approval_attribution: 15,
            unexpected_approval_states: 12,
            ..Default::default()
        }
    );
    assert_eq!(
        preflight::read(&pool, c.org_id, c.user_id).await.unwrap(),
        PreflightCounts::default()
    );
    assert_eq!(history(&pool).await, before);
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn preflight_separates_unknown_import_requesters_without_changing_artifacts(pool: PgPool) {
    let a = seed(&pool, OrgRole::Admin).await;
    let unrelated = seed(&pool, OrgRole::Admin).await;
    for kind in [
        "harvest_api_import",
        "harvest_csv_import",
        "unrelated_service",
    ] {
        for status in ["queued", "running", "succeeded", "failed", "cancelled"] {
            for requester in [None, Some(a.user_id)] {
                let job = Uuid::now_v7();
                sqlx::query!(
                    "INSERT INTO horae_jobs (id, org_id, kind, status, payload, original_requester_id,
                       worker_id, lease_until, report, checkpoint)
                     VALUES ($1, $2, $3, $4, '{\"private\":\"source\"}', $5,
                       'retained-worker', now() + interval '1 hour', '{\"errors\":[]}', '{\"offset\":23}')",
                    job, a.org_id, kind, status, requester
                ).execute(&pool).await.unwrap();
                sqlx::query!(
                    "INSERT INTO horae_job_uploads (job_id, org_id, filename, content_type, body)
                     VALUES ($1, $2, 'private.csv', 'text/csv', $3)",
                    job,
                    a.org_id,
                    b"retained upload".as_slice()
                )
                .execute(&pool)
                .await
                .unwrap();
                sqlx::query!(
                    "INSERT INTO horae_job_report_error_chunks (id, job_id, org_id, sequence, body)
                     VALUES ($1, $2, $3, 0, $4)",
                    Uuid::now_v7(),
                    job,
                    a.org_id,
                    b"retained error".as_slice()
                )
                .execute(&pool)
                .await
                .unwrap();
            }
        }
    }
    let before = history(&pool).await;
    assert_eq!(
        preflight::read(&pool, a.org_id, a.user_id).await.unwrap(),
        PreflightCounts {
            pending_imports_without_requester: 4,
            terminal_imports_without_requester: 6,
            ..Default::default()
        }
    );
    assert_eq!(
        preflight::read(&pool, unrelated.org_id, unrelated.user_id)
            .await
            .unwrap(),
        PreflightCounts::default()
    );
    assert_eq!(history(&pool).await, before);
}

async fn history(pool: &PgPool) -> serde_json::Value {
    sqlx::query_scalar!(
        "SELECT jsonb_build_object(
          'orgs', (SELECT jsonb_agg(o ORDER BY id) FROM organizations o),
          'users', (SELECT jsonb_agg(u ORDER BY id) FROM users u),
          'projects', (SELECT jsonb_agg(p ORDER BY id) FROM projects p),
          'memberships', (SELECT jsonb_agg(a ORDER BY id) FROM assignments a),
          'approvals', (SELECT jsonb_agg(a ORDER BY id) FROM approvals a),
          'jobs', (SELECT jsonb_agg(j ORDER BY id) FROM horae_jobs j),
          'uploads', (SELECT jsonb_agg(u ORDER BY job_id) FROM horae_job_uploads u),
          'chunks', (SELECT jsonb_agg(c ORDER BY id) FROM horae_job_report_error_chunks c),
          'audit', (SELECT jsonb_agg(a ORDER BY id) FROM audit_log a),
          'receipts', (SELECT jsonb_agg(r ORDER BY id) FROM permission_change_receipts r))"
    )
    .fetch_one(pool)
    .await
    .unwrap()
    .unwrap()
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn preflight_requires_current_legacy_administrator_not_staged_grants(pool: PgPool) {
    let admin = seed(&pool, OrgRole::Admin).await;
    for role in [OrgRole::Manager, OrgRole::Member] {
        let ids = seed(&pool, role).await;
        let grants: Vec<String> = serde_json::from_value(
            serde_json::to_value(BuiltInProfile::Administrator.selection()).unwrap(),
        )
        .unwrap();
        sqlx::query!(
            "INSERT INTO person_permission_states (id, org_id, user_id, catalog_version, grants, is_administrator, source)
             VALUES ($1, $2, $3, 1, $4, true, 'individual')", Uuid::now_v7(), ids.org_id, ids.user_id, &grants
        ).execute(&pool).await.unwrap();
        assert!(matches!(
            preflight::read(&pool, ids.org_id, ids.user_id).await,
            Err(PreflightError::Forbidden)
        ));
    }
    for (org, user) in [
        (admin.org_id, Uuid::now_v7()),
        (Uuid::now_v7(), admin.user_id),
    ] {
        assert!(matches!(
            preflight::read(&pool, org, user).await,
            Err(PreflightError::Forbidden)
        ));
    }
    let foreign = seed(&pool, OrgRole::Admin).await;
    assert!(matches!(
        preflight::read(&pool, admin.org_id, foreign.user_id).await,
        Err(PreflightError::Forbidden)
    ));
    for version in [1, 99] {
        sqlx::query!(
            "UPDATE organizations SET permission_policy_version = $2 WHERE id = $1",
            admin.org_id,
            version
        )
        .execute(&pool)
        .await
        .unwrap();
        assert!(matches!(
            preflight::read(&pool, admin.org_id, admin.user_id).await,
            Err(PreflightError::Forbidden)
        ));
    }
    sqlx::query!(
        "UPDATE organizations SET permission_policy_version = 0 WHERE id = $1",
        admin.org_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE users SET active = false WHERE id = $1",
        admin.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(matches!(
        preflight::read(&pool, admin.org_id, admin.user_id).await,
        Err(PreflightError::Forbidden)
    ));
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn preflight_rechecks_revocation_after_organization_and_actor_waits(pool: PgPool) {
    let reader = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .after_connect(|connection, _| {
            Box::pin(async move {
                sqlx::query!("SET default_transaction_isolation = 'repeatable read'")
                    .execute(&mut *connection)
                    .await?;
                sqlx::query!("SET default_transaction_read_only = on")
                    .execute(&mut *connection)
                    .await?;
                Ok(())
            })
        })
        .connect_with(pool.connect_options().as_ref().clone())
        .await
        .unwrap();
    for organization_gate in [true, false] {
        let ids = seed(&pool, OrgRole::Admin).await;
        let mut blocker = pool.begin().await.unwrap();
        let pid = sqlx::query_scalar!("SELECT pg_backend_pid()")
            .fetch_one(&mut *blocker)
            .await
            .unwrap()
            .unwrap();
        if organization_gate {
            sqlx::query!(
                "SELECT id FROM organizations WHERE id = $1 FOR UPDATE",
                ids.org_id
            )
            .fetch_one(&mut *blocker)
            .await
            .unwrap();
        } else {
            sqlx::query!("SELECT id FROM users WHERE id = $1 FOR UPDATE", ids.user_id)
                .fetch_one(&mut *blocker)
                .await
                .unwrap();
        }
        let connection = reader.clone();
        let operation =
            tokio::spawn(
                async move { preflight::read(&connection, ids.org_id, ids.user_id).await },
            );
        wait_for_blocked(&pool, pid).await;
        sqlx::query!(
            "UPDATE users SET org_role = 'member' WHERE id = $1",
            ids.user_id
        )
        .execute(&mut *blocker)
        .await
        .unwrap();
        blocker.commit().await.unwrap();
        assert!(matches!(
            tokio::time::timeout(Duration::from_secs(5), operation)
                .await
                .unwrap()
                .unwrap(),
            Err(PreflightError::Forbidden)
        ));
    }
    let ids = seed(&pool, OrgRole::Admin).await;
    assert_eq!(
        preflight::read(&reader, ids.org_id, ids.user_id)
            .await
            .unwrap(),
        PreflightCounts::default()
    );
    let defaults = sqlx::query!("SELECT current_setting('default_transaction_isolation') AS isolation, current_setting('default_transaction_read_only') AS readonly")
        .fetch_one(&reader).await.unwrap();
    assert_eq!(defaults.isolation.as_deref(), Some("repeatable read"));
    assert_eq!(defaults.readonly.as_deref(), Some("on"));
    reader.close().await;
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn preflight_cancellation_releases_organization_and_single_connection(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Admin).await;
    let reader = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect_with(pool.connect_options().as_ref().clone())
        .await
        .unwrap();
    let mut blocker = pool.begin().await.unwrap();
    let pid = sqlx::query_scalar!("SELECT pg_backend_pid()")
        .fetch_one(&mut *blocker)
        .await
        .unwrap()
        .unwrap();
    sqlx::query!("SELECT id FROM users WHERE id = $1 FOR UPDATE", ids.user_id)
        .fetch_one(&mut *blocker)
        .await
        .unwrap();
    let connection = reader.clone();
    let operation =
        tokio::spawn(async move { preflight::read(&connection, ids.org_id, ids.user_id).await });
    wait_for_blocked(&pool, pid).await;
    operation.abort();
    assert!(operation.await.unwrap_err().is_cancelled());
    blocker.rollback().await.unwrap();
    tokio::time::timeout(Duration::from_secs(5), async {
        let mut connection = reader.acquire().await.unwrap();
        sqlx::query!("SELECT 1 AS drained")
            .fetch_one(&mut *connection)
            .await
            .unwrap();
    })
    .await
    .unwrap();
    let mut independent = pool.begin().await.unwrap();
    sqlx::query!(
        "SELECT id FROM organizations WHERE id = $1 FOR UPDATE NOWAIT",
        ids.org_id
    )
    .fetch_one(&mut *independent)
    .await
    .unwrap();
    independent.rollback().await.unwrap();
    assert_eq!(
        preflight::read(&reader, ids.org_id, ids.user_id)
            .await
            .unwrap(),
        PreflightCounts::default()
    );
    reader.close().await;
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn preflight_lock_failure_releases_organization_without_mutating_data(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Admin).await;
    let reader = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .after_connect(|connection, _| {
            Box::pin(async move {
                sqlx::query!("SET lock_timeout = '20ms'")
                    .execute(connection)
                    .await?;
                Ok(())
            })
        })
        .connect_with(pool.connect_options().as_ref().clone())
        .await
        .unwrap();
    let before = history(&pool).await;
    let mut blocker = pool.begin().await.unwrap();
    sqlx::query!("SELECT id FROM users WHERE id = $1 FOR UPDATE", ids.user_id)
        .fetch_one(&mut *blocker)
        .await
        .unwrap();
    let error = tokio::time::timeout(
        Duration::from_secs(5),
        preflight::read(&reader, ids.org_id, ids.user_id),
    )
    .await
    .unwrap()
    .unwrap_err();
    let PreflightError::Database(sqlx::Error::Database(error)) = error else {
        panic!("expected lock timeout: {error}")
    };
    assert_eq!(error.code().as_deref(), Some("55P03"));
    blocker.rollback().await.unwrap();
    sqlx::query!("SELECT 1 AS drained")
        .fetch_one(&reader)
        .await
        .unwrap();
    let mut independent = pool.begin().await.unwrap();
    sqlx::query!(
        "SELECT id FROM organizations WHERE id = $1 FOR UPDATE NOWAIT",
        ids.org_id
    )
    .fetch_one(&mut *independent)
    .await
    .unwrap();
    independent.rollback().await.unwrap();
    assert_eq!(history(&pool).await, before);
    assert_eq!(
        preflight::read(&reader, ids.org_id, ids.user_id)
            .await
            .unwrap(),
        PreflightCounts::default()
    );
    reader.close().await;
}
