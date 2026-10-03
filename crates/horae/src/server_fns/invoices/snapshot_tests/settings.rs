use super::*;

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn financial_snapshots_override_isolation_but_restore_connection_settings(pool: PgPool) {
    let ids = fee_tests::single_fee(&pool).await;
    let reader_pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect_with((*pool.connect_options()).clone())
        .await
        .unwrap();
    for isolation in ["read committed", "repeatable read", "serializable"] {
        for timeout in ["0", "250ms", "10s"] {
            sqlx::query!("SELECT set_config('default_transaction_isolation', $1, false) AS isolation, set_config('default_transaction_read_only', 'on', false) AS read_only, set_config('lock_timeout', $2, false) AS timeout", isolation, timeout)
                .fetch_one(&reader_pool).await.unwrap();
            let mut tx =
                crate::server_fns::snapshot::manager(&reader_pool, ids.org_id, ids.user_id)
                    .await
                    .unwrap();
            let settings = sqlx::query!("SELECT current_setting('transaction_isolation') AS \"isolation!\", current_setting('transaction_read_only') AS \"read_only!\", current_setting('lock_timeout') AS \"timeout!\"")
                .fetch_one(&mut *tx).await.unwrap();
            assert_eq!(
                (
                    settings.isolation.as_str(),
                    settings.read_only.as_str(),
                    settings.timeout.as_str()
                ),
                ("repeatable read", "off", timeout)
            );
            tx.commit().await.unwrap();
            for reader in [Reader::Invoice, Reader::Fees] {
                assert_denied(read(&reader_pool, (ids.org_id, Uuid::now_v7()), &ids, reader).await);
                assert_eq!(
                    read(&reader_pool, (ids.org_id, ids.user_id), &ids, reader)
                        .await
                        .unwrap(),
                    12500
                );
            }
            let settings = sqlx::query!("SELECT current_setting('transaction_isolation') AS \"isolation!\", current_setting('transaction_read_only') AS \"read_only!\", current_setting('lock_timeout') AS \"timeout!\"")
                .fetch_one(&reader_pool).await.unwrap();
            assert_eq!(
                (
                    settings.isolation.as_str(),
                    settings.read_only.as_str(),
                    settings.timeout.as_str()
                ),
                (isolation, "on", timeout)
            );
        }
    }
    reader_pool.close().await;
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn financial_snapshots_honor_stricter_lock_timeout_during_authorization(pool: PgPool) {
    let ids = fee_tests::single_fee(&pool).await;
    let reader_pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .after_connect(|connection, _| {
            Box::pin(async move {
                sqlx::query!("SET lock_timeout='250ms'")
                    .execute(connection)
                    .await?;
                Ok(())
            })
        })
        .connect_with((*pool.connect_options()).clone())
        .await
        .unwrap();
    let mut writer = pool.begin().await.unwrap();
    crate::db::lock_organization(
        &mut writer,
        ids.org_id,
        crate::db::OrganizationLock::AccessChange,
    )
    .await
    .unwrap();
    for reader in [Reader::Invoice, Reader::Fees] {
        let error = tokio::time::timeout(
            std::time::Duration::from_secs(2),
            read(&reader_pool, (ids.org_id, ids.user_id), &ids, reader),
        )
        .await
        .expect("stricter timeout was replaced by the five-second ceiling")
        .unwrap_err();
        assert!(
            matches!(
                error,
                ServerFnError::ServerError {
                    code: INTERNAL_ERROR,
                    ..
                }
            ),
            "{error:?}"
        );
        assert!(!error.to_string().contains("lock timeout"));
    }
    writer.rollback().await.unwrap();
    for reader in [Reader::Invoice, Reader::Fees] {
        assert_eq!(
            read(&reader_pool, (ids.org_id, ids.user_id), &ids, reader)
                .await
                .unwrap(),
            12500
        );
    }
    reader_pool.close().await;
}
#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn financial_snapshots_retry_only_serialization_failures_and_stop_after_three(pool: PgPool) {
    let ids = fee_tests::single_fee(&pool).await;
    sqlx::query!("CREATE SCHEMA snapshot_fault")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query!("CREATE SEQUENCE snapshot_fault.attempts")
        .execute(&pool)
        .await
        .unwrap();
    // The sequence survives rollback and counts real prelude executions without
    // introducing a production test hook or another user/role.
    sqlx::query!("CREATE FUNCTION snapshot_fault.fail() RETURNS boolean LANGUAGE plpgsql AS $$ BEGIN PERFORM nextval('snapshot_fault.attempts'); RAISE EXCEPTION 'private_snapshot_failure' USING ERRCODE=current_setting('horae.snapshot_error'); END $$")
        .execute(&pool).await.unwrap();
    sqlx::query!("CREATE VIEW snapshot_fault.organizations AS SELECT id FROM public.organizations WHERE snapshot_fault.fail()")
        .execute(&pool).await.unwrap();
    let reader_pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .after_connect(|connection, _| {
            Box::pin(async move {
                sqlx::query!("SET search_path = snapshot_fault, public")
                    .execute(connection)
                    .await?;
                Ok(())
            })
        })
        .connect_with((*pool.connect_options()).clone())
        .await
        .unwrap();
    let mut attempts = 0;
    for reader in [Reader::Invoice, Reader::Fees] {
        for (code, expected_status, count) in [("40001", CONFLICT, 3), ("22000", INTERNAL_ERROR, 1)]
        {
            sqlx::query!("SELECT set_config('horae.snapshot_error', $1, false)", code)
                .fetch_one(&reader_pool)
                .await
                .unwrap();
            let error = read(&reader_pool, (ids.org_id, ids.user_id), &ids, reader)
                .await
                .unwrap_err();
            assert!(
                matches!(error, ServerFnError::ServerError { code, .. } if code == expected_status),
                "{error:?}"
            );
            assert!(!error.to_string().contains("private_snapshot_failure"));
            attempts += count;
            assert_eq!(sqlx::query_scalar!("SELECT last_value FROM pg_sequences WHERE schemaname='snapshot_fault' AND sequencename='attempts'").fetch_one(&pool).await.unwrap(), Some(attempts));
        }
    }
    reader_pool.close().await;
}
