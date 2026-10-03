use super::*;
use crate::server_fns::test_seed::{seed, wait_for_blocked};
use sqlx::PgPool;
use std::time::Duration;

fn default_branding() -> OrgBranding {
    OrgBranding {
        provider_name: None,
        provider_address: None,
        provider_tax_id: None,
        provider_email: None,
        provider_phone: None,
        bank_name: None,
        bank_iban: None,
        bank_bic: None,
        bank_routing: None,
        bank_account: None,
        invoice_notes: None,
        invoice_payment_terms: Some("Net 30".into()),
    }
}

async fn row_version(pool: &PgPool, org_id: uuid::Uuid) -> Option<String> {
    sqlx::query_scalar!("SELECT xmin::text FROM organizations WHERE id = $1", org_id)
        .fetch_one(pool)
        .await
        .unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn unchanged_branding_preserves_the_row(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Admin).await;
    let branding = default_branding();
    let before = row_version(&pool, ids.org_id).await;
    let (returned, changed) = update_org_branding_record(&pool, ids.org_id, ids.user_id, &branding)
        .await
        .unwrap();
    assert_eq!(
        (changed, returned, row_version(&pool, ids.org_id).await),
        (false, branding, before)
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn each_branding_field_changes_once_including_null_and_empty(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Admin).await;
    let mut fields = serde_json::to_value(default_branding()).unwrap();
    for field in [
        "provider_name",
        "provider_address",
        "provider_tax_id",
        "provider_email",
        "provider_phone",
        "bank_name",
        "bank_iban",
        "bank_bic",
        "bank_routing",
        "bank_account",
        "invoice_notes",
        "invoice_payment_terms",
    ] {
        for value in [Some("Changed"), Some(""), None] {
            fields[field] = serde_json::to_value(value).unwrap();
            let requested: OrgBranding = serde_json::from_value(fields.clone()).unwrap();
            let (returned, changed) =
                update_org_branding_record(&pool, ids.org_id, ids.user_id, &requested)
                    .await
                    .unwrap();
            assert_eq!(
                (changed, &returned),
                (true, &requested),
                "{field}: {value:?}"
            );
            let before = row_version(&pool, ids.org_id).await;
            let (repeated, changed) =
                update_org_branding_record(&pool, ids.org_id, ids.user_id, &requested)
                    .await
                    .unwrap();
            assert_eq!(
                (changed, repeated, row_version(&pool, ids.org_id).await),
                (false, requested, before),
                "repeated {field}: {value:?}"
            );
        }
    }
}

async fn competing_edit(pool: &PgPool, requested_name: Option<&str>) -> (OrgBranding, bool) {
    let ids = seed(pool, OrgRole::Admin).await;
    let mut first = pool.begin().await.unwrap();
    let blocker = sqlx::query_scalar!("SELECT pg_backend_pid()")
        .fetch_one(&mut *first)
        .await
        .unwrap()
        .unwrap();
    sqlx::query!(
        "UPDATE organizations SET provider_name = 'Changed' WHERE id = $1",
        ids.org_id
    )
    .execute(&mut *first)
    .await
    .unwrap();
    let mut requested = default_branding();
    requested.provider_name = requested_name.map(str::to_owned);
    let run_pool = pool.clone();
    let mut run = tokio::task::JoinSet::new();
    run.spawn(async move {
        update_org_branding_record(&run_pool, ids.org_id, ids.user_id, &requested).await
    });
    wait_for_blocked(pool, blocker).await;
    first.commit().await.unwrap();
    tokio::time::timeout(Duration::from_secs(5), run.join_next())
        .await
        .unwrap()
        .unwrap()
        .unwrap()
        .unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn repeating_a_competing_branding_edit_does_not_report_a_change(pool: PgPool) {
    let (branding, changed) = competing_edit(&pool, Some("Changed")).await;
    assert_eq!(
        (changed, branding.provider_name.as_deref()),
        (false, Some("Changed"))
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn restoring_branding_after_a_competing_edit_reports_a_change(pool: PgPool) {
    let (branding, changed) = competing_edit(&pool, None).await;
    assert_eq!((changed, branding), (true, default_branding()));
}

#[sqlx::test(migrations = "./migrations")]
async fn missing_organization_is_not_an_unchanged_update(pool: PgPool) {
    let error = update_org_branding_record(
        &pool,
        uuid::Uuid::now_v7(),
        uuid::Uuid::now_v7(),
        &default_branding(),
    )
    .await
    .unwrap_err();
    assert!(matches!(
        error,
        ServerFnError::ServerError {
            code: NOT_FOUND,
            ..
        }
    ));
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn branding_requires_current_tenant_bound_authority_even_for_noops(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    let foreign = seed(&pool, OrgRole::Admin).await;
    let before = row_version(&pool, ids.org_id).await;
    for (actor, role, active) in [
        (ids.user_id, OrgRole::Member, true),
        (ids.user_id, OrgRole::Manager, false),
        (ids.user_id, OrgRole::Admin, false),
        (foreign.user_id, OrgRole::Admin, true),
        (uuid::Uuid::now_v7(), OrgRole::Admin, true),
    ] {
        sqlx::query!(
            "UPDATE users SET org_role = $2, active = $3 WHERE id = $1",
            ids.user_id,
            role as OrgRole,
            active,
        )
        .execute(&pool)
        .await
        .unwrap();
        for name in [None, Some("Unauthorized change")] {
            let requested = OrgBranding {
                provider_name: name.map(str::to_owned),
                ..default_branding()
            };
            let result = update_org_branding_record(&pool, ids.org_id, actor, &requested).await;
            assert!(
                matches!(
                    result,
                    Err(ServerFnError::ServerError {
                        code: FORBIDDEN,
                        ..
                    })
                ),
                "{actor} {role:?} active={active} name={name:?}: {result:?}"
            );
            assert_eq!(row_version(&pool, ids.org_id).await, before);
        }
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn branding_rechecks_authority_after_the_organization_wait(pool: PgPool) {
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
    for deactivate in [false, true] {
        for change in [false, true] {
            let ids = seed(&pool, OrgRole::Manager).await;
            let before = row_version(&pool, ids.org_id).await;
            let mut revoke = pool.begin().await.unwrap();
            let blocker = sqlx::query_scalar!("SELECT pg_backend_pid()")
                .fetch_one(&mut *revoke)
                .await
                .unwrap()
                .unwrap();
            sqlx::query!(
                "SELECT id FROM organizations WHERE id = $1 FOR UPDATE",
                ids.org_id
            )
            .fetch_one(&mut *revoke)
            .await
            .unwrap();
            let requested = OrgBranding {
                provider_name: change.then(|| "Unauthorized change".to_owned()),
                ..default_branding()
            };
            let run_pool = run_pool.clone();
            let mut run = tokio::task::JoinSet::new();
            run.spawn(async move {
                update_org_branding_record(&run_pool, ids.org_id, ids.user_id, &requested).await
            });
            wait_for_blocked(&pool, blocker).await;
            sqlx::query!(
                "UPDATE users SET org_role = $2, active = $3 WHERE id = $1",
                ids.user_id,
                if deactivate {
                    OrgRole::Manager
                } else {
                    OrgRole::Member
                } as OrgRole,
                !deactivate,
            )
            .execute(&mut *revoke)
            .await
            .unwrap();
            revoke.commit().await.unwrap();
            let result = tokio::time::timeout(Duration::from_secs(5), run.join_next())
                .await
                .unwrap()
                .unwrap()
                .unwrap();
            assert!(
                matches!(
                    result,
                    Err(ServerFnError::ServerError {
                        code: FORBIDDEN,
                        ..
                    })
                ),
                "deactivate={deactivate} change={change}: {result:?}"
            );
            assert_eq!(row_version(&pool, ids.org_id).await, before);
        }
    }
    run_pool.close().await;
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn branding_rechecks_authority_after_an_actor_only_wait(pool: PgPool) {
    for deactivate in [false, true] {
        let ids = seed(&pool, OrgRole::Manager).await;
        let before = row_version(&pool, ids.org_id).await;
        let mut revoke = pool.begin().await.unwrap();
        let blocker = sqlx::query_scalar!("SELECT pg_backend_pid()")
            .fetch_one(&mut *revoke)
            .await
            .unwrap()
            .unwrap();
        sqlx::query!(
            "UPDATE users SET org_role = $2, active = $3 WHERE id = $1",
            ids.user_id,
            if deactivate {
                OrgRole::Manager
            } else {
                OrgRole::Member
            } as OrgRole,
            !deactivate,
        )
        .execute(&mut *revoke)
        .await
        .unwrap();
        let db = pool.clone();
        let mut run = tokio::task::JoinSet::new();
        run.spawn(async move {
            update_org_branding_record(&db, ids.org_id, ids.user_id, &default_branding()).await
        });
        wait_for_blocked(&pool, blocker).await;
        revoke.commit().await.unwrap();
        let result = tokio::time::timeout(Duration::from_secs(5), run.join_next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert!(
            matches!(
                result,
                Err(ServerFnError::ServerError {
                    code: FORBIDDEN,
                    ..
                })
            ),
            "{result:?}"
        );
        assert_eq!(row_version(&pool, ids.org_id).await, before);
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn branding_holds_actor_authority_until_its_write_commits(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    let requested = OrgBranding {
        provider_name: Some("Authorized before revocation".into()),
        ..default_branding()
    };
    let mut barrier = pool.begin().await.unwrap();
    let blocker = sqlx::query_scalar!("SELECT pg_backend_pid()")
        .fetch_one(&mut *barrier)
        .await
        .unwrap()
        .unwrap();
    // SHARE permits the initial SELECT FOR UPDATE but blocks the later UPDATE.
    sqlx::query!("LOCK TABLE organizations IN SHARE MODE")
        .execute(&mut *barrier)
        .await
        .unwrap();
    let db = pool.clone();
    let submitted = requested.clone();
    let mut write = tokio::task::JoinSet::new();
    write.spawn(async move {
        update_org_branding_record(&db, ids.org_id, ids.user_id, &submitted).await
    });
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
        result = revoke.join_next() => panic!("revocation completed before branding committed: {result:?}"),
        () = wait_for_blocked(&pool, writer) => {},
    }
    barrier.commit().await.unwrap();
    let (returned, changed) = tokio::time::timeout(Duration::from_secs(5), write.join_next())
        .await
        .unwrap()
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!((returned, changed), (requested, true));
    tokio::time::timeout(Duration::from_secs(5), revoke.join_next())
        .await
        .unwrap()
        .unwrap()
        .unwrap()
        .unwrap();
    let result =
        update_org_branding_record(&pool, ids.org_id, ids.user_id, &default_branding()).await;
    assert!(
        matches!(
            result,
            Err(ServerFnError::ServerError {
                code: FORBIDDEN,
                ..
            })
        ),
        "{result:?}"
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn failed_branding_write_rolls_back_and_releases_authority(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    let before = row_version(&pool, ids.org_id).await;
    sqlx::query!(
        "ALTER TABLE organizations ADD CONSTRAINT reject_branding_write CHECK (false) NOT VALID"
    )
    .execute(&pool)
    .await
    .unwrap();
    let requested = OrgBranding {
        provider_name: Some("Must roll back".into()),
        ..default_branding()
    };
    let result = update_org_branding_record(&pool, ids.org_id, ids.user_id, &requested).await;
    assert!(result.is_err());
    assert_eq!(row_version(&pool, ids.org_id).await, before);
    // Transaction drop queues rollback; wait for completion, not a scheduler tick.
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
    .expect("failed branding update must release organization and actor locks");
    sqlx::query!("ALTER TABLE organizations DROP CONSTRAINT reject_branding_write")
        .execute(&pool)
        .await
        .unwrap();
    let (returned, changed) =
        update_org_branding_record(&pool, ids.org_id, ids.user_id, &requested)
            .await
            .unwrap();
    assert_eq!((returned, changed), (requested.clone(), true));
    let before = row_version(&pool, ids.org_id).await;
    let (returned, changed) =
        update_org_branding_record(&pool, ids.org_id, ids.user_id, &requested)
            .await
            .unwrap();
    assert_eq!(
        (returned, changed, row_version(&pool, ids.org_id).await),
        (requested, false, before)
    );
}
