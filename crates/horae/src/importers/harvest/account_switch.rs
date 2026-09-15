//! Organization-scoped identity boundaries for Harvest connections and work.

use horae_core::importers::harvest::types::ConnectionStatus;
use sqlx::{Acquire, PgPool, Postgres, Transaction};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Version {
    pub account_generation: i64,
    pub connection_revision: i64,
}

#[derive(Debug, thiserror::Error)]
pub enum ChangeError {
    #[error(
        "The Harvest connection changed. Reload the importer and review the current account before trying again."
    )]
    Stale,
    #[error(
        "This import belongs to an earlier Harvest connection. Start a new import with a new request ID."
    )]
    OldImport,
    #[error("Connect Harvest before starting an API import")]
    NotConnected,
    #[error("{0}")]
    Blocked(&'static str),
}

/// Short acceptance gate, never held over HTTP or an import. Queueing behind a
/// running import remains possible; changes also take the existing import lock.
pub async fn gate(tx: &mut Transaction<'_, Postgres>, org_id: Uuid) -> anyhow::Result<Version> {
    sqlx::query!(
        "INSERT INTO harvest_connection_generations (org_id) VALUES ($1) ON CONFLICT (org_id) DO NOTHING",
        org_id,
    )
    .execute(&mut **tx)
    .await?;
    let row = sqlx::query!(
        "SELECT account_generation, connection_revision FROM harvest_connection_generations WHERE org_id = $1 FOR UPDATE",
        org_id,
    )
    .fetch_one(&mut **tx)
    .await?;
    Ok(Version {
        account_generation: row.account_generation,
        connection_revision: row.connection_revision,
    })
}

pub async fn status<'e, E: sqlx::PgExecutor<'e>>(
    exec: E,
    org_id: Uuid,
    configured: bool,
) -> anyhow::Result<ConnectionStatus> {
    let row = sqlx::query!(
        r#"SELECT b.harvest_account_id AS "harvest_account_id?", c.org_id IS NOT NULL AS "connected!",
                  COALESCE(c.token_expires_at <= now(), false) AS "expired!",
                  COALESCE(g.account_generation, 0) AS "generation!",
                  COALESCE(g.connection_revision, 0) AS "revision!",
                  EXISTS(SELECT 1 FROM harvest_import_map WHERE org_id = o.id) AS "provenance!",
                  (SELECT count(*) FROM horae_jobs WHERE org_id = o.id
                    AND kind IN ('harvest_api_import', 'harvest_csv_import')
                    AND status IN ('queued', 'running')) AS "active!"
           FROM organizations o
           LEFT JOIN harvest_account_bindings b ON b.org_id = o.id
           LEFT JOIN harvest_credentials c ON c.org_id = o.id
           LEFT JOIN harvest_connection_generations g ON g.org_id = o.id
           WHERE o.id = $1"#,
        org_id,
    )
    .fetch_one(exec)
    .await?;
    Ok(ConnectionStatus {
        configured,
        connected: row.connected,
        account_id: row.harvest_account_id,
        token_expired: row.expired,
        account_generation: row.generation,
        connection_revision: row.revision,
        has_provenance: row.provenance,
        active_imports: row.active,
    })
}

pub async fn change(
    pool: &PgPool,
    org_id: Uuid,
    account: &str,
    generation: i64,
    revision: i64,
) -> anyhow::Result<()> {
    let mut connection = super::lock_import(pool, org_id).await?;
    let result = async {
        let mut tx = connection.begin().await?;
        let current = gate(&mut tx, org_id).await?;
        let inspected = status(&mut *tx, org_id, true).await?;
        let expected = Version { account_generation: generation, connection_revision: revision };
        anyhow::ensure!(current == expected && inspected.account_id.as_deref() == Some(account), ChangeError::Stale);
        if let Some(reason) = inspected.change_account_blocker() {
            return Err(ChangeError::Blocked(reason).into());
        }
        sqlx::query!("DELETE FROM harvest_credentials WHERE org_id = $1", org_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query!("DELETE FROM harvest_account_bindings WHERE org_id = $1", org_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query!(
            "UPDATE harvest_connection_generations SET account_generation = account_generation + 1, connection_revision = connection_revision + 1 WHERE org_id = $1",
            org_id,
        )
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }.await;
    super::release_import(connection).await?;
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::JobPolicy,
        jobs::{self, JobPayload},
    };
    use horae_core::importers::harvest::types::{EntityType, ImportMode, SyncScope};

    async fn organization(pool: &sqlx::PgPool) -> Uuid {
        let id = Uuid::now_v7();
        sqlx::query!(
            "INSERT INTO organizations (id, name) VALUES ($1, 'Switch test')",
            id
        )
        .execute(pool)
        .await
        .unwrap();
        sqlx::query!(
            "INSERT INTO harvest_account_bindings (org_id, harvest_account_id) VALUES ($1, 'A')",
            id
        )
        .execute(pool)
        .await
        .unwrap();
        id
    }

    const KEY: &str = "1111111111111111111111111111111111111111111111111111111111111111";

    async fn connect(
        pool: &PgPool,
        org: Uuid,
        account: &str,
        version: Version,
    ) -> anyhow::Result<()> {
        super::super::credentials::store_for_attempt(
            pool, org, KEY, account, "access", "refresh", None, None, version,
        )
        .await
    }

    fn payload() -> JobPayload {
        JobPayload::HarvestApi {
            mode: ImportMode::DryRun,
            sync: SyncScope::Full,
        }
    }

    async fn business_snapshot(pool: &PgPool) -> serde_json::Value {
        sqlx::query_scalar!(r#"SELECT jsonb_agg(value ORDER BY value::text) AS "snapshot!" FROM (
            SELECT jsonb_build_array('organizations', to_jsonb(t)) AS value FROM organizations t
            UNION ALL SELECT jsonb_build_array('users', to_jsonb(t)) FROM users t
            UNION ALL SELECT jsonb_build_array('clients', to_jsonb(t)) FROM clients t
            UNION ALL SELECT jsonb_build_array('projects', to_jsonb(t)) FROM projects t
            UNION ALL SELECT jsonb_build_array('tasks', to_jsonb(t)) FROM tasks t
            UNION ALL SELECT jsonb_build_array('project_tasks', to_jsonb(t)) FROM project_tasks t
            UNION ALL SELECT jsonb_build_array('assignments', to_jsonb(t)) FROM assignments t
            UNION ALL SELECT jsonb_build_array('time_entries', to_jsonb(t)) FROM time_entries t
            UNION ALL SELECT jsonb_build_array('approvals', to_jsonb(t)) FROM approvals t
            UNION ALL SELECT jsonb_build_array('audit_log', to_jsonb(t)) FROM audit_log t
            UNION ALL SELECT jsonb_build_array('invoices', to_jsonb(t)) FROM invoices t
            UNION ALL SELECT jsonb_build_array('invoice_line_items', to_jsonb(t)) FROM invoice_line_items t
        ) records"#).fetch_one(pool).await.unwrap()
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn successful_change_preserves_business_rows_and_failure_rolls_back_secrets(
        pool: PgPool,
    ) {
        let seed =
            crate::server_fns::test_seed::seed(&pool, horae_core::types::OrgRole::Admin).await;
        let org = seed.org_id;
        let entry = Uuid::now_v7();
        sqlx::query!("INSERT INTO time_entries (id, org_id, user_id, project_id, task_id, spent_date, minutes, billable) VALUES ($1, $2, $3, $4, $5, CURRENT_DATE, 60, true)", entry, org, seed.user_id, seed.project_id, seed.task_id).execute(&pool).await.unwrap();
        let invoice = Uuid::now_v7();
        sqlx::query!("INSERT INTO invoices (id, org_id, client_id, number, issued_on, due_on, currency) VALUES ($1, $2, $3, 'SW-1', CURRENT_DATE, CURRENT_DATE, 'EUR')", invoice, org, seed.client_id).execute(&pool).await.unwrap();
        connect(
            &pool,
            org,
            "A",
            Version {
                account_generation: 0,
                connection_revision: 0,
            },
        )
        .await
        .unwrap();
        let before = business_snapshot(&pool).await;
        // Failure after DELETE must roll back credentials, binding and counters together.
        sqlx::query!(
            "UPDATE harvest_connection_generations SET connection_revision = $2 WHERE org_id = $1",
            org,
            i64::MAX
        )
        .execute(&pool)
        .await
        .unwrap();
        assert!(change(&pool, org, "A", 0, i64::MAX).await.is_err());
        let current = status(&pool, org, true).await.unwrap();
        assert!(current.connected);
        assert_eq!(current.account_id.as_deref(), Some("A"));
        assert_eq!(
            super::super::credentials::load(&pool, org, KEY)
                .await
                .unwrap()
                .unwrap()
                .access_token,
            "access"
        );
        sqlx::query!(
            "UPDATE harvest_connection_generations SET connection_revision = 1 WHERE org_id = $1",
            org
        )
        .execute(&pool)
        .await
        .unwrap();
        change(&pool, org, "A", 0, 1).await.unwrap();
        assert_eq!(business_snapshot(&pool).await, before);
        assert!(
            super::super::credentials::load(&pool, org, KEY)
                .await
                .unwrap()
                .is_none()
        );
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn running_jobs_block_change_and_old_execution_never_reaches_http(pool: PgPool) {
        use super::super::api_source::http::{
            ApiHttp,
            test_server::{Response, Server},
        };
        let org = organization(&pool).await;
        connect(
            &pool,
            org,
            "A",
            Version {
                account_generation: 0,
                connection_revision: 0,
            },
        )
        .await
        .unwrap();
        let job = jobs::enqueue_api(&pool, org, &payload(), "execution", JobPolicy::default(), 0)
            .await
            .unwrap();
        assert!(change(&pool, org, "A", 0, 1).await.is_err());
        let (lease, stop) = jobs::claim_lease_for_test(&pool).await;
        assert!(change(&pool, org, "A", 0, 1).await.is_err());
        jobs::cancel(&pool, org, job).await.unwrap();
        jobs::run_claimed(&pool, &lease, stop, lease.run(std::future::pending()))
            .await
            .unwrap();
        change(&pool, org, "A", 0, 1).await.unwrap();
        connect(
            &pool,
            org,
            "B",
            Version {
                account_generation: 1,
                connection_revision: 2,
            },
        )
        .await
        .unwrap();
        // Simulate an obsolete worker bypassing acceptance; execution still fences it.
        sqlx::query!(
            "UPDATE horae_jobs SET status = 'queued', attempts = 0, available_at = now(), cancellation_requested = false WHERE id = $1",
            job
        )
        .execute(&pool)
        .await
        .unwrap();
        let (lease, _stop) = jobs::claim_lease_for_test(&pool).await;
        let server = Server::start(|_| Response::json(serde_json::json!({})));
        let cfg = crate::config::HarvestConfig {
            client_id: "test".into(),
            client_secret: "test".into(),
            redirect_url: "http://localhost/callback".into(),
            encryption_key_hex: "invalid-key-must-not-be-read".into(),
        };
        let error = super::super::run_api_import_with_http(
            &pool,
            org,
            "EUR",
            &cfg,
            ImportMode::DryRun,
            SyncScope::Full,
            ApiHttp::local(server.base.clone()),
            Some(&lease),
        )
        .await
        .unwrap_err();
        assert!(
            error.to_string().contains("earlier Harvest connection"),
            "{error}"
        );
        assert_eq!(server.count.load(std::sync::atomic::Ordering::Acquire), 0);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn queue_acceptance_does_not_wait_for_the_import_session_lock(pool: PgPool) {
        let org = organization(&pool).await;
        connect(
            &pool,
            org,
            "A",
            Version {
                account_generation: 0,
                connection_revision: 0,
            },
        )
        .await
        .unwrap();
        let lock = super::super::lock_import(&pool, org).await.unwrap();
        let work = payload();
        tokio::time::timeout(
            std::time::Duration::from_secs(2),
            jobs::enqueue_api(&pool, org, &work, "behind-running", JobPolicy::default(), 0),
        )
        .await
        .unwrap()
        .unwrap();
        assert!(change(&pool, org, "A", 0, 1).await.is_err());
        super::super::release_import(lock).await.unwrap();
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn retry_and_change_are_serialized(pool: PgPool) {
        for _ in 0..8 {
            let org = organization(&pool).await;
            let job = jobs::enqueue(&pool, org, &payload(), "retry-race", JobPolicy::default())
                .await
                .unwrap();
            jobs::cancel(&pool, org, job).await.unwrap();
            let (changed, retried) =
                tokio::join!(change(&pool, org, "A", 0, 0), jobs::retry(&pool, org, job));
            assert_ne!(changed.is_ok(), retried.is_ok_and(|retried| retried));
        }
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn disconnect_and_reconnect_fence_callbacks_but_not_same_account_work(pool: PgPool) {
        let org = organization(&pool).await;
        let initial = Version {
            account_generation: 0,
            connection_revision: 0,
        };
        connect(&pool, org, "A", initial).await.unwrap();
        assert!(connect(&pool, org, "A", initial).await.is_err());
        let pending = Version {
            account_generation: 0,
            connection_revision: 1,
        };
        let job = jobs::enqueue_api(&pool, org, &payload(), "same", JobPolicy::default(), 0)
            .await
            .unwrap();
        jobs::cancel(&pool, org, job).await.unwrap();
        super::super::credentials::disconnect(&pool, org)
            .await
            .unwrap();
        assert!(connect(&pool, org, "A", pending).await.is_err());
        assert!(!status(&pool, org, true).await.unwrap().connected);
        connect(
            &pool,
            org,
            "A",
            Version {
                account_generation: 0,
                connection_revision: 2,
            },
        )
        .await
        .unwrap();
        assert!(jobs::retry(&pool, org, job).await.unwrap());
        assert_eq!(
            status(&pool, org, true).await.unwrap().account_generation,
            0
        );
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn a_b_a_keeps_old_callbacks_requests_and_retries_fenced(pool: PgPool) {
        let org = organization(&pool).await;
        connect(
            &pool,
            org,
            "A",
            Version {
                account_generation: 0,
                connection_revision: 0,
            },
        )
        .await
        .unwrap();
        let old = jobs::enqueue_api(&pool, org, &payload(), "lost-ack", JobPolicy::default(), 0)
            .await
            .unwrap();
        jobs::cancel(&pool, org, old).await.unwrap();
        let report = jobs::status(&pool, org, old).await.unwrap().unwrap().report;
        change(&pool, org, "A", 0, 1).await.unwrap();
        assert!(
            connect(
                &pool,
                org,
                "A",
                Version {
                    account_generation: 0,
                    connection_revision: 1
                }
            )
            .await
            .is_err()
        );
        connect(
            &pool,
            org,
            "B",
            Version {
                account_generation: 1,
                connection_revision: 2,
            },
        )
        .await
        .unwrap();
        assert!(
            jobs::enqueue_api(&pool, org, &payload(), "lost-ack", JobPolicy::default(), 1)
                .await
                .unwrap_err()
                .is::<jobs::RequestConflict>()
        );
        assert!(
            jobs::enqueue_api(
                &pool,
                org,
                &payload(),
                "never-accepted-old",
                JobPolicy::default(),
                0
            )
            .await
            .is_err()
        );
        assert!(
            jobs::retry(&pool, org, old)
                .await
                .unwrap_err()
                .is::<ChangeError>()
        );
        change(&pool, org, "B", 1, 3).await.unwrap();
        connect(
            &pool,
            org,
            "A",
            Version {
                account_generation: 2,
                connection_revision: 4,
            },
        )
        .await
        .unwrap();
        assert!(jobs::retry(&pool, org, old).await.is_err());
        assert!(
            jobs::enqueue_api(&pool, org, &payload(), "lost-ack", JobPolicy::default(), 2)
                .await
                .is_err()
        );
        assert_eq!(
            jobs::status(&pool, org, old).await.unwrap().unwrap().report,
            report
        );
        // A new pool has no process-local identity state to restore.
        let fresh_pool = PgPool::connect_with(pool.connect_options().as_ref().clone())
            .await
            .unwrap();
        assert_eq!(
            status(&fresh_pool, org, true)
                .await
                .unwrap()
                .account_generation,
            2
        );
        assert!(
            jobs::enqueue_api(&fresh_pool, org, &payload(), "new", JobPolicy::default(), 2)
                .await
                .is_ok()
        );
        fresh_pool.close().await;
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn competing_change_and_submission_cannot_both_succeed(pool: PgPool) {
        for _ in 0..8 {
            let org = organization(&pool).await;
            connect(
                &pool,
                org,
                "A",
                Version {
                    account_generation: 0,
                    connection_revision: 0,
                },
            )
            .await
            .unwrap();
            let work = payload();
            let (changed, queued) = tokio::join!(
                change(&pool, org, "A", 0, 1),
                jobs::enqueue_api(&pool, org, &work, "racing", JobPolicy::default(), 0)
            );
            assert_ne!(
                changed.is_ok(),
                queued.is_ok(),
                "exactly one operation must win"
            );
        }
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn competing_callbacks_and_confirmations_have_one_winner(pool: PgPool) {
        let org = organization(&pool).await;
        let initial = Version {
            account_generation: 0,
            connection_revision: 0,
        };
        let (first, second) = tokio::join!(
            connect(&pool, org, "A", initial),
            connect(&pool, org, "A", initial)
        );
        assert_ne!(first.is_ok(), second.is_ok());
        let (first, second) =
            tokio::join!(change(&pool, org, "A", 0, 1), change(&pool, org, "A", 0, 1));
        assert_ne!(first.is_ok(), second.is_ok());
        let final_status = status(&pool, org, true).await.unwrap();
        assert_eq!(final_status.account_generation, 1);
        assert_eq!(final_status.connection_revision, 2);

        for _ in 0..8 {
            let org = organization(&pool).await;
            connect(&pool, org, "A", initial).await.unwrap();
            let pending = Version {
                account_generation: 0,
                connection_revision: 1,
            };
            let (callback, changed) = tokio::join!(
                connect(&pool, org, "A", pending),
                change(&pool, org, "A", 0, 1),
            );
            assert_ne!(callback.is_ok(), changed.is_ok());
            let current = status(&pool, org, true).await.unwrap();
            assert_eq!(current.connection_revision, 2);
            assert_eq!(current.account_generation, i64::from(changed.is_ok()));
        }
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn change_preserves_terminal_history_and_rejects_stale_confirmation(pool: sqlx::PgPool) {
        let org = organization(&pool).await;
        let job = jobs::enqueue(
            &pool,
            org,
            &JobPayload::HarvestApi {
                mode: ImportMode::DryRun,
                sync: SyncScope::Full,
            },
            "old",
            JobPolicy::default(),
        )
        .await
        .unwrap();
        jobs::cancel(&pool, org, job).await.unwrap();
        let mut before = jobs::status(&pool, org, job).await.unwrap().unwrap();
        assert_eq!(
            before.retry_availability,
            crate::models::RetryAvailability::Available
        );
        let inspected = status(&pool, org, true).await.unwrap();
        change(
            &pool,
            org,
            "A",
            inspected.account_generation,
            inspected.connection_revision,
        )
        .await
        .unwrap();
        let after = status(&pool, org, true).await.unwrap();
        assert_eq!(after.account_id, None);
        assert_eq!(after.account_generation, 1);
        assert_eq!(after.connection_revision, 1);
        let retained = jobs::status(&pool, org, job).await.unwrap().unwrap();
        assert_eq!(
            retained.retry_availability,
            crate::models::RetryAvailability::PreviousAccount
        );
        // Only the advisory prerequisites change; every retained job/report field is preserved.
        before.retry_availability = crate::models::RetryAvailability::PreviousAccount;
        assert_eq!(before, retained);
        assert!(change(&pool, org, "A", 0, 0).await.is_err());
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn queued_work_and_provenance_block_without_changing_binding(pool: sqlx::PgPool) {
        let org = organization(&pool).await;
        let job = jobs::enqueue_csv(
            &pool,
            org,
            ImportMode::DryRun,
            b"csv".to_vec(),
            "active",
            JobPolicy::default(),
        )
        .await
        .unwrap();
        assert!(change(&pool, org, "A", 0, 0).await.is_err());
        let (lease, stop) = jobs::claim_lease_for_test(&pool).await;
        assert!(change(&pool, org, "A", 0, 0).await.is_err());
        jobs::cancel(&pool, org, job).await.unwrap();
        jobs::run_claimed(&pool, &lease, stop, lease.run(std::future::pending()))
            .await
            .unwrap();
        assert_eq!(status(&pool, org, true).await.unwrap().active_imports, 0);
        super::super::provenance::upsert(&pool, org, EntityType::Client, 1, Uuid::now_v7(), None)
            .await
            .unwrap();
        assert!(change(&pool, org, "A", 0, 0).await.is_err());
        let current = status(&pool, org, true).await.unwrap();
        assert_eq!(current.account_id.as_deref(), Some("A"));
        assert_eq!(current.account_generation, 0);
        assert!(current.has_provenance);
    }
}
