use super::*;
use crate::models::client::ClientBillingSnapshot;
use crate::plugin::event::ActiveTransition;
use crate::server_fns::test_seed::{seed, wait_for_blocked};
use sqlx::PgPool;
use std::time::Duration;

fn profile() -> ClientProfile {
    ClientProfile {
        name: "  Acme  ".into(),
        currency: "EUR".into(),
        address: Some("Street\nFloor 2".into()),
        tax_id: Some("  ".into()),
    }
}

fn profile_edit(detail: &ClientDetails, rate_change: ClientRateChange) -> ClientProfileEdit {
    ClientProfileEdit {
        profile: ClientProfile {
            name: detail.client.name.clone(),
            currency: detail.client.currency.clone(),
            address: detail.client.address.clone(),
            tax_id: detail.client.tax_id.clone(),
        },
        rate_change,
        original: ClientBillingSnapshot {
            currency: detail.client.currency.clone(),
            default_rate_cents: detail.billing.as_ref().unwrap().default_rate_cents,
        },
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn profile_create_persists_normalized_identity_and_exact_optional_rate(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    for (input, expected) in [("", None), ("0", Some(0)), ("123.45", Some(12345))] {
        let created =
            create_client_profile_record(&pool, ids.org_id, ids.user_id, &profile(), input)
                .await
                .unwrap();
        assert_eq!(created.client.name, "Acme");
        assert_eq!(created.client.address.as_deref(), Some("Street\nFloor 2"));
        assert_eq!(created.client.tax_id, None);
        assert_eq!(created.client.id.get_version_num(), 7);
        let reloaded = client_details_for_viewer(&pool, ids.org_id, ids.user_id, created.client.id)
            .await
            .unwrap();
        assert_eq!(reloaded, created);
        assert_eq!(reloaded.billing.unwrap().default_rate_cents, expected);
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn profile_update_requires_explicit_rate_intent_for_currency_changes(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Admin).await;
    for input in ["0", "120.50"] {
        let before =
            create_client_profile_record(&pool, ids.org_id, ids.user_id, &profile(), input)
                .await
                .unwrap();
        let version = row_version(&pool, before.client.id).await;
        let mut edit = profile_edit(&before, ClientRateChange::Keep);
        edit.profile.currency = "USD".into();
        let error =
            update_client_profile_record(&pool, ids.org_id, ids.user_id, before.client.id, &edit)
                .await
                .unwrap_err();
        assert!(matches!(
            error,
            ServerFnError::ServerError { code: CONFLICT, .. }
        ));
        assert_eq!(row_version(&pool, before.client.id).await, version);
        edit.rate_change = ClientRateChange::Replace("99.99".into());
        let (updated, changed) =
            update_client_profile_record(&pool, ids.org_id, ids.user_id, before.client.id, &edit)
                .await
                .unwrap();
        assert!(changed);
        assert_eq!(updated.client.currency, "USD");
        assert_eq!(
            updated.billing.as_ref().unwrap().default_rate_cents,
            Some(9999)
        );
        let mut clear = profile_edit(&updated, ClientRateChange::Clear);
        clear.profile.currency = "GBP".into();
        let (cleared, changed) =
            update_client_profile_record(&pool, ids.org_id, ids.user_id, before.client.id, &clear)
                .await
                .unwrap();
        assert!(changed);
        assert_eq!(cleared.client.currency, "GBP");
        assert_eq!(cleared.billing.unwrap().default_rate_cents, None);
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn profile_noop_preserves_row_and_returns_no_update_event_signal(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    let before = create_client_profile_record(&pool, ids.org_id, ids.user_id, &profile(), "0")
        .await
        .unwrap();
    let version = row_version(&pool, before.client.id).await;
    for change in [
        ClientRateChange::Keep,
        ClientRateChange::Replace("0.00".into()),
    ] {
        let (after, changed) = update_client_profile_record(
            &pool,
            ids.org_id,
            ids.user_id,
            before.client.id,
            &profile_edit(&before, change),
        )
        .await
        .unwrap();
        assert!(!changed);
        assert_eq!(after, before);
        assert_eq!(row_version(&pool, before.client.id).await, version);
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn profile_saves_reject_invalid_input_without_partial_writes(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    let before = client_details_for_viewer(&pool, ids.org_id, ids.user_id, ids.client_id)
        .await
        .unwrap();
    let version = row_version(&pool, ids.client_id).await;
    for (name, currency, address, tax, rate) in [
        (" ", "EUR", None, None, "0"),
        ("Name", "JPY", None, None, "0"),
        ("Name", "EUR", Some("a\0b"), None, "0"),
        ("Name", "EUR", None, Some("a\0b"), "0"),
        ("Name", "EUR", None, None, "-1"),
        ("Name", "EUR", None, None, "1.001"),
    ] {
        let invalid = ClientProfile {
            name: name.into(),
            currency: currency.into(),
            address: address.map(str::to_owned),
            tax_id: tax.map(str::to_owned),
        };
        let error = create_client_profile_record(&pool, ids.org_id, ids.user_id, &invalid, rate)
            .await
            .unwrap_err();
        assert!(matches!(
            error,
            ServerFnError::ServerError {
                code: BAD_REQUEST,
                ..
            }
        ));
        let mut edit = profile_edit(&before, ClientRateChange::Replace(rate.into()));
        edit.profile = invalid;
        let error =
            update_client_profile_record(&pool, ids.org_id, ids.user_id, ids.client_id, &edit)
                .await
                .unwrap_err();
        assert!(matches!(
            error,
            ServerFnError::ServerError {
                code: BAD_REQUEST,
                ..
            }
        ));
    }
    let error = update_client_profile_record(
        &pool,
        ids.org_id,
        ids.user_id,
        ids.client_id,
        &profile_edit(&before, ClientRateChange::Replace(String::new())),
    )
    .await
    .unwrap_err();
    assert!(matches!(
        error,
        ServerFnError::ServerError {
            code: BAD_REQUEST,
            ..
        }
    ));
    assert_eq!(row_version(&pool, ids.client_id).await, version);
    assert_eq!(
        sqlx::query_scalar!("SELECT count(*) FROM clients WHERE org_id = $1", ids.org_id)
            .fetch_one(&pool)
            .await
            .unwrap(),
        Some(1)
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn profile_saves_recheck_manager_org_and_active_actor(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    let foreign = seed(&pool, OrgRole::Admin).await;
    let member = seed(&pool, OrgRole::Member).await;
    let before = client_details_for_viewer(&pool, ids.org_id, ids.user_id, ids.client_id)
        .await
        .unwrap();
    let edit = profile_edit(&before, ClientRateChange::Keep);
    for (org, actor) in [
        (ids.org_id, foreign.user_id),
        (member.org_id, member.user_id),
    ] {
        let error = create_client_profile_record(&pool, org, actor, &profile(), "")
            .await
            .unwrap_err();
        assert!(matches!(
            error,
            ServerFnError::ServerError {
                code: FORBIDDEN,
                ..
            }
        ));
        let error = update_client_profile_record(&pool, org, actor, ids.client_id, &edit)
            .await
            .unwrap_err();
        assert!(matches!(
            error,
            ServerFnError::ServerError {
                code: FORBIDDEN,
                ..
            }
        ));
    }
    for id in [foreign.client_id, uuid::Uuid::now_v7()] {
        let error = update_client_profile_record(&pool, ids.org_id, ids.user_id, id, &edit)
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
    sqlx::query!("UPDATE users SET active = false WHERE id = $1", ids.user_id)
        .execute(&pool)
        .await
        .unwrap();
    let error = create_client_profile_record(&pool, ids.org_id, ids.user_id, &profile(), "")
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        ServerFnError::ServerError {
            code: FORBIDDEN,
            ..
        }
    ));
    let error = update_client_profile_record(&pool, ids.org_id, ids.user_id, ids.client_id, &edit)
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        ServerFnError::ServerError {
            code: FORBIDDEN,
            ..
        }
    ));
}

#[sqlx::test(migrations = "./migrations")]
async fn profile_update_preserves_historical_rows_and_inactive_status(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    sqlx::query!(
        "UPDATE clients SET active = false WHERE id = $1",
        ids.client_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let invoice_id = uuid::Uuid::now_v7();
    sqlx::query!("INSERT INTO invoices (id, org_id, client_id, number, status, issued_on, due_on, currency, total_cents) VALUES ($1,$2,$3,'HISTORY','paid','2026-09-01','2026-09-22','GBP',12345)", invoice_id, ids.org_id, ids.client_id).execute(&pool).await.unwrap();
    let history = sqlx::query_scalar!(
        "SELECT to_jsonb(p) FROM projects p WHERE id = $1",
        ids.project_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let invoice = sqlx::query_scalar!(
        "SELECT to_jsonb(i) FROM invoices i WHERE id = $1",
        invoice_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let before = client_details_for_viewer(&pool, ids.org_id, ids.user_id, ids.client_id)
        .await
        .unwrap();
    let mut edit = profile_edit(&before, ClientRateChange::Replace("200".into()));
    edit.profile.name = "New name".into();
    edit.profile.currency = "USD".into();
    let (after, changed) =
        update_client_profile_record(&pool, ids.org_id, ids.user_id, ids.client_id, &edit)
            .await
            .unwrap();
    assert!(changed && !after.client.active);
    assert_eq!(
        sqlx::query_scalar!(
            "SELECT to_jsonb(p) FROM projects p WHERE id = $1",
            ids.project_id
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        history
    );
    assert_eq!(
        sqlx::query_scalar!(
            "SELECT to_jsonb(i) FROM invoices i WHERE id = $1",
            invoice_id
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        invoice
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn profile_update_rejects_stale_rate_even_when_the_target_currency_is_unchanged(
    pool: PgPool,
) {
    let ids = seed(&pool, OrgRole::Manager).await;
    let before = client_details_for_viewer(&pool, ids.org_id, ids.user_id, ids.client_id)
        .await
        .unwrap();
    sqlx::query!(
        "UPDATE clients SET default_rate_cents = $2 WHERE id = $1",
        ids.client_id,
        0_i64
    )
    .execute(&pool)
    .await
    .unwrap();
    for change in [
        ClientRateChange::Keep,
        ClientRateChange::Clear,
        ClientRateChange::Replace("100".into()),
    ] {
        let error = update_client_profile_record(
            &pool,
            ids.org_id,
            ids.user_id,
            ids.client_id,
            &profile_edit(&before, change),
        )
        .await
        .unwrap_err();
        assert!(matches!(
            error,
            ServerFnError::ServerError { code: CONFLICT, .. }
        ));
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn profile_currency_change_without_a_rate_keeps_it_unset(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    let before = client_details_for_viewer(&pool, ids.org_id, ids.user_id, ids.client_id)
        .await
        .unwrap();
    let mut edit = profile_edit(&before, ClientRateChange::Keep);
    edit.profile.currency = "CHF".into();
    let (after, changed) =
        update_client_profile_record(&pool, ids.org_id, ids.user_id, ids.client_id, &edit)
            .await
            .unwrap();
    assert!(changed);
    assert_eq!(after.client.currency, "CHF");
    assert_eq!(after.billing.unwrap().default_rate_cents, None);
}

#[sqlx::test(migrations = "./migrations")]
async fn profile_update_rejects_stale_currency_even_when_the_rate_is_still_unset(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    let before = client_details_for_viewer(&pool, ids.org_id, ids.user_id, ids.client_id)
        .await
        .unwrap();
    sqlx::query!(
        "UPDATE clients SET currency = 'USD' WHERE id = $1",
        ids.client_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let version = row_version(&pool, ids.client_id).await;
    let error = update_client_profile_record(
        &pool,
        ids.org_id,
        ids.user_id,
        ids.client_id,
        &profile_edit(&before, ClientRateChange::Keep),
    )
    .await
    .unwrap_err();
    assert!(matches!(
        error,
        ServerFnError::ServerError { code: CONFLICT, .. }
    ));
    assert_eq!(row_version(&pool, ids.client_id).await, version);
}

#[sqlx::test(migrations = "./migrations")]
async fn profile_update_rechecks_financial_snapshot_after_waiting_for_a_concurrent_edit(
    pool: PgPool,
) {
    let ids = seed(&pool, OrgRole::Manager).await;
    let before = client_details_for_viewer(&pool, ids.org_id, ids.user_id, ids.client_id)
        .await
        .unwrap();
    let edit = profile_edit(&before, ClientRateChange::Replace("100".into()));
    let mut first = pool.begin().await.unwrap();
    let blocker = sqlx::query_scalar!("SELECT pg_backend_pid()")
        .fetch_one(&mut *first)
        .await
        .unwrap()
        .unwrap();
    sqlx::query!(
        "UPDATE clients SET currency = 'USD', default_rate_cents = 0 WHERE id = $1",
        ids.client_id
    )
    .execute(&mut *first)
    .await
    .unwrap();
    let run_pool = pool.clone();
    let mut run = tokio::task::JoinSet::new();
    run.spawn(async move {
        update_client_profile_record(&run_pool, ids.org_id, ids.user_id, ids.client_id, &edit).await
    });
    wait_for_blocked(&pool, blocker).await;
    first.commit().await.unwrap();
    let error = tokio::time::timeout(Duration::from_secs(5), run.join_next())
        .await
        .unwrap()
        .unwrap()
        .unwrap()
        .unwrap_err();
    assert!(matches!(
        error,
        ServerFnError::ServerError { code: CONFLICT, .. }
    ));
    let after = client_details_for_viewer(&pool, ids.org_id, ids.user_id, ids.client_id)
        .await
        .unwrap();
    assert_eq!(after.client.currency, "USD");
    assert_eq!(after.billing.unwrap().default_rate_cents, Some(0));
}

#[sqlx::test(migrations = "./migrations")]
async fn profile_update_rechecks_authority_after_waiting_for_a_concurrent_demotion(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    let before = client_details_for_viewer(&pool, ids.org_id, ids.user_id, ids.client_id)
        .await
        .unwrap();
    let edit = profile_edit(&before, ClientRateChange::Replace("100".into()));
    let version = row_version(&pool, ids.client_id).await;
    let mut first = pool.begin().await.unwrap();
    let blocker = sqlx::query_scalar!("SELECT pg_backend_pid()")
        .fetch_one(&mut *first)
        .await
        .unwrap()
        .unwrap();
    sqlx::query!(
        "UPDATE users SET org_role = 'member' WHERE id = $1",
        ids.user_id
    )
    .execute(&mut *first)
    .await
    .unwrap();
    let run_pool = pool.clone();
    let mut run = tokio::task::JoinSet::new();
    run.spawn(async move {
        update_client_profile_record(&run_pool, ids.org_id, ids.user_id, ids.client_id, &edit).await
    });
    wait_for_blocked(&pool, blocker).await;
    first.commit().await.unwrap();
    let error = tokio::time::timeout(Duration::from_secs(5), run.join_next())
        .await
        .unwrap()
        .unwrap()
        .unwrap()
        .unwrap_err();
    assert!(matches!(
        error,
        ServerFnError::ServerError {
            code: FORBIDDEN,
            ..
        }
    ));
    assert_eq!(row_version(&pool, ids.client_id).await, version);
}

async fn client(pool: &PgPool) -> Client {
    let ids = seed(pool, OrgRole::Admin).await;
    sqlx::query_as!(
        Client,
        r#"SELECT id, org_id, name, currency, address, tax_id, active,
                  created_at as "created_at: chrono::DateTime<chrono::Utc>"
           FROM clients WHERE id = $1"#,
        ids.client_id,
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn row_version(pool: &PgPool, id: uuid::Uuid) -> Option<String> {
    sqlx::query_scalar!("SELECT xmin::text FROM clients WHERE id = $1", id)
        .fetch_one(pool)
        .await
        .unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn client_details_keep_catalog_identity_but_omit_member_billing(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Member).await;
    sqlx::query!(
        "UPDATE clients SET address = 'Test address', tax_id = 'TEST-VAT',
         default_rate_cents = 12345, active = false WHERE id = $1",
        ids.client_id,
    )
    .execute(&pool)
    .await
    .unwrap();

    let detail = client_details_for_viewer(&pool, ids.org_id, ids.user_id, ids.client_id)
        .await
        .unwrap();
    assert_eq!(detail.client.id, ids.client_id);
    assert_eq!(detail.client.address.as_deref(), Some("Test address"));
    assert_eq!(detail.client.tax_id.as_deref(), Some("TEST-VAT"));
    assert!(!detail.client.active);
    assert!(detail.billing.is_none());
    let payload = serde_json::to_string(&detail).unwrap();
    for restricted in ["billing", "default_rate_cents", "12345"] {
        assert!(
            !payload.contains(restricted),
            "unexpected {restricted}: {payload}"
        );
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn client_details_distinguish_unset_zero_and_positive_manager_rates(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    for rate in [None, Some(0), Some(12345)] {
        sqlx::query!(
            "UPDATE clients SET default_rate_cents = $2 WHERE id = $1",
            ids.client_id,
            rate
        )
        .execute(&pool)
        .await
        .unwrap();
        let detail = client_details_for_viewer(&pool, ids.org_id, ids.user_id, ids.client_id)
            .await
            .unwrap();
        assert_eq!(detail.billing.unwrap().default_rate_cents, rate);
    }
    sqlx::query!(
        "UPDATE users SET org_role = 'admin' WHERE id = $1",
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let detail = client_details_for_viewer(&pool, ids.org_id, ids.user_id, ids.client_id)
        .await
        .unwrap();
    assert_eq!(detail.billing.unwrap().default_rate_cents, Some(12345));
}

#[sqlx::test(migrations = "./migrations")]
async fn client_details_hide_foreign_missing_and_inactive_actor_records(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Admin).await;
    let foreign = seed(&pool, OrgRole::Admin).await;
    for (org, actor, client) in [
        (ids.org_id, ids.user_id, foreign.client_id),
        (ids.org_id, ids.user_id, uuid::Uuid::now_v7()),
        (ids.org_id, foreign.user_id, ids.client_id),
    ] {
        let error = client_details_for_viewer(&pool, org, actor, client)
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
    sqlx::query!("UPDATE users SET active = false WHERE id = $1", ids.user_id)
        .execute(&pool)
        .await
        .unwrap();
    let error = client_details_for_viewer(&pool, ids.org_id, ids.user_id, ids.client_id)
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
async fn client_invoices_require_manager_authority_even_without_invoices(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Member).await;
    let error = client_invoices_for_viewer(&pool, ids.org_id, ids.user_id, ids.client_id)
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        ServerFnError::ServerError {
            code: FORBIDDEN,
            ..
        }
    ));
    sqlx::query!(
        "UPDATE users SET org_role = 'manager' WHERE id = $1",
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(
        client_invoices_for_viewer(&pool, ids.org_id, ids.user_id, ids.client_id)
            .await
            .unwrap()
            .is_empty()
    );
    sqlx::query!("UPDATE users SET active = false WHERE id = $1", ids.user_id)
        .execute(&pool)
        .await
        .unwrap();
    let error = client_invoices_for_viewer(&pool, ids.org_id, ids.user_id, ids.client_id)
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
async fn client_invoices_reject_a_winning_concurrent_revocation(pool: PgPool) {
    for (revoked_role, active) in [(OrgRole::Member, true), (OrgRole::Manager, false)] {
        let ids = seed(&pool, OrgRole::Manager).await;
        sqlx::query!(
            "INSERT INTO invoices (id,org_id,client_id,number,issued_on,due_on,currency,total_cents)
             VALUES ($1,$2,$3,'REVOCATION','2026-09-01','2026-09-15','EUR',12345)",
            uuid::Uuid::now_v7(), ids.org_id, ids.client_id,
        )
        .execute(&pool)
        .await
        .unwrap();
        let mut revocation = pool.begin().await.unwrap();
        let blocker = sqlx::query_scalar!("SELECT pg_backend_pid()")
            .fetch_one(&mut *revocation)
            .await
            .unwrap()
            .unwrap();
        // Prevent invoice materialization until the pending revocation commits,
        // even if a reader checks authority without locking the user's row.
        sqlx::query!("LOCK TABLE invoices IN ACCESS EXCLUSIVE MODE")
            .execute(&mut *revocation)
            .await
            .unwrap();
        sqlx::query!(
            "UPDATE users SET org_role = $2, active = $3 WHERE id = $1",
            ids.user_id,
            revoked_role as OrgRole,
            active,
        )
        .execute(&mut *revocation)
        .await
        .unwrap();
        let run_pool = pool.clone();
        let mut run = tokio::task::JoinSet::new();
        run.spawn(async move {
            client_invoices_for_viewer(&run_pool, ids.org_id, ids.user_id, ids.client_id).await
        });
        wait_for_blocked(&pool, blocker).await;
        revocation.commit().await.unwrap();
        let error = tokio::time::timeout(Duration::from_secs(5), run.join_next())
            .await
            .unwrap()
            .unwrap()
            .unwrap()
            .unwrap_err();
        assert!(matches!(
            error,
            ServerFnError::ServerError {
                code: FORBIDDEN,
                ..
            }
        ));
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn client_invoices_hold_authority_until_the_read_finishes(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    let mut invoice_blocker = pool.begin().await.unwrap();
    let blocker = sqlx::query_scalar!("SELECT pg_backend_pid()")
        .fetch_one(&mut *invoice_blocker)
        .await
        .unwrap()
        .unwrap();
    sqlx::query!("LOCK TABLE invoices IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *invoice_blocker)
        .await
        .unwrap();
    let run_pool = pool.clone();
    let mut run = tokio::task::JoinSet::new();
    run.spawn(async move {
        client_invoices_for_viewer(&run_pool, ids.org_id, ids.user_id, ids.client_id).await
    });
    wait_for_blocked(&pool, blocker).await;

    let mut revocation = pool.begin().await.unwrap();
    sqlx::query!("SET LOCAL lock_timeout = '100ms'")
        .execute(&mut *revocation)
        .await
        .unwrap();
    let result = sqlx::query!(
        "UPDATE users SET org_role = 'member' WHERE id = $1",
        ids.user_id,
    )
    .execute(&mut *revocation)
    .await;
    revocation.rollback().await.unwrap();
    invoice_blocker.commit().await.unwrap();
    let invoices = tokio::time::timeout(Duration::from_secs(5), run.join_next())
        .await
        .unwrap()
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(invoices.is_empty());
    assert_eq!(
        result
            .unwrap_err()
            .as_database_error()
            .unwrap()
            .code()
            .as_deref(),
        Some("55P03"),
        "revocation must wait while the authorized reader is materializing invoices",
    );
    // A completed read must release its authority lock.
    sqlx::query!(
        "UPDATE users SET org_role = 'member' WHERE id = $1",
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
}

#[sqlx::test(migrations = "./migrations")]
async fn client_invoices_keep_exact_client_scope_statuses_and_currencies(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    let foreign = seed(&pool, OrgRole::Admin).await;
    let other_client = uuid::Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO clients (id, org_id, name, currency) VALUES ($1,$2,'Other','EUR')",
        other_client,
        ids.org_id
    )
    .execute(&pool)
    .await
    .unwrap();
    for (index, (org_id, client_id, status, currency)) in [
        (ids.org_id, ids.client_id, InvoiceStatus::Draft, "EUR"),
        (ids.org_id, ids.client_id, InvoiceStatus::Sent, "USD"),
        (ids.org_id, ids.client_id, InvoiceStatus::Paid, "GBP"),
        (ids.org_id, ids.client_id, InvoiceStatus::Void, "CHF"),
        (ids.org_id, other_client, InvoiceStatus::Draft, "EUR"),
        (
            foreign.org_id,
            foreign.client_id,
            InvoiceStatus::Draft,
            "EUR",
        ),
    ]
    .into_iter()
    .enumerate()
    {
        sqlx::query!(
            "INSERT INTO invoices (id,org_id,client_id,number,status,issued_on,due_on,currency,total_cents)
             VALUES ($1,$2,$3,$4,$5,'2026-09-01','2026-09-15',$6,12345)",
            uuid::Uuid::now_v7(), org_id, client_id, format!("TEST-{index}"),
            status as InvoiceStatus, currency,
        ).execute(&pool).await.unwrap();
    }
    sqlx::query!(
        "UPDATE clients SET active = false WHERE id = $1",
        ids.client_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let invoices = client_invoices_for_viewer(&pool, ids.org_id, ids.user_id, ids.client_id)
        .await
        .unwrap();
    assert_eq!(invoices.len(), 4);
    assert!(
        invoices
            .iter()
            .all(|invoice| invoice.client_id == ids.client_id
                && invoice.org_id == ids.org_id
                && invoice.total_cents == 12345)
    );
    let mut currencies: Vec<_> = invoices
        .iter()
        .map(|invoice| invoice.currency.as_str())
        .collect();
    currencies.sort_unstable();
    assert_eq!(currencies, ["CHF", "EUR", "GBP", "USD"]);
    let all = super::super::invoices::fetch_invoices(&pool, ids.org_id, None, None)
        .await
        .unwrap();
    assert_eq!(all.len(), 5);
    let drafts =
        super::super::invoices::fetch_invoices(&pool, ids.org_id, Some(InvoiceStatus::Draft), None)
            .await
            .unwrap();
    assert_eq!(drafts.len(), 2);
    assert!(
        drafts
            .iter()
            .all(|invoice| invoice.status == InvoiceStatus::Draft)
    );
    let paid = super::super::invoices::fetch_invoices(
        &pool,
        ids.org_id,
        Some(InvoiceStatus::Paid),
        Some(ids.client_id),
    )
    .await
    .unwrap();
    assert_eq!(paid.len(), 1);
    assert_eq!(paid[0].currency, "GBP");
    let error = client_invoices_for_viewer(&pool, ids.org_id, ids.user_id, foreign.client_id)
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
async fn client_summaries_exclude_foreign_clients_and_unreadable_project_currencies(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Member).await;
    let foreign = seed(&pool, OrgRole::Admin).await;
    sqlx::query!(
        "UPDATE projects SET currency = 'GBP' WHERE id = $1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE clients SET default_rate_cents = 12345 WHERE id = $1",
        ids.client_id
    )
    .execute(&pool)
    .await
    .unwrap();
    crate::server_fns::test_seed::time_entry(&pool, &ids, EntryState::Open).await;

    let summaries = client_summaries_for_viewer(&pool, ids.org_id, ids.user_id)
        .await
        .unwrap();
    assert_eq!(summaries.len(), 1);
    let summary = &summaries[0];
    assert_eq!(summary.client.id, ids.client_id);
    assert_eq!((summary.active_projects, summary.total_projects), (0, 0));
    assert!(summary.project_currencies.is_empty());
    let payload = serde_json::to_string(summary).unwrap();
    for restricted in [
        "default_rate_cents",
        "12345",
        "GBP",
        &foreign.client_id.to_string(),
    ] {
        assert!(
            !payload.contains(restricted),
            "unexpected value {restricted}: {payload}"
        );
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn client_summaries_follow_existing_progress_visibility_and_revoke_inactive_users(
    pool: PgPool,
) {
    let ids = seed(&pool, OrgRole::Member).await;
    sqlx::query!(
        "INSERT INTO assignments (id, project_id, user_id) VALUES ($1,$2,$3)",
        uuid::Uuid::now_v7(),
        ids.project_id,
        ids.user_id,
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE projects SET currency = 'GBP' WHERE id = $1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let summaries = client_summaries_for_viewer(&pool, ids.org_id, ids.user_id)
        .await
        .unwrap();
    assert_eq!(summaries[0].project_currencies, ["GBP"]);
    assert_eq!(
        (summaries[0].active_projects, summaries[0].total_projects),
        (1, 1)
    );

    sqlx::query!(
        "INSERT INTO project_settings (id, org_id, project_id, creator_id, rate_mode, report_visibility)
         VALUES ($1,$2,$3,$4,'project','managers')",
        uuid::Uuid::now_v7(), ids.org_id, ids.project_id, ids.user_id,
    ).execute(&pool).await.unwrap();
    let hidden = client_summaries_for_viewer(&pool, ids.org_id, ids.user_id)
        .await
        .unwrap();
    assert_eq!(
        (
            hidden[0].total_projects,
            hidden[0].project_currencies.as_slice()
        ),
        (0, &[][..])
    );

    sqlx::query!(
        "UPDATE assignments SET role = 'lead' WHERE user_id = $1",
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let lead = client_summaries_for_viewer(&pool, ids.org_id, ids.user_id)
        .await
        .unwrap();
    assert_eq!(lead[0].total_projects, 1);

    sqlx::query!("UPDATE users SET active = false WHERE id = $1", ids.user_id)
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        client_summaries_for_viewer(&pool, ids.org_id, ids.user_id)
            .await
            .unwrap()
            .is_empty()
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn client_summaries_keep_catalog_without_projects_and_count_archived_projects(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    let empty_id = uuid::Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO clients (id, org_id, name, currency, active) VALUES ($1,$2,'Empty','USD',false)",
        empty_id, ids.org_id,
    ).execute(&pool).await.unwrap();
    sqlx::query!(
        "UPDATE projects SET active = false WHERE id = $1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let summaries = client_summaries_for_viewer(&pool, ids.org_id, ids.user_id)
        .await
        .unwrap();
    assert_eq!(summaries.len(), 2);
    assert_eq!(
        (summaries[0].active_projects, summaries[0].total_projects),
        (0, 1)
    );
    assert_eq!(summaries[0].project_currencies, ["EUR"]);
    assert_eq!(summaries[1].client.id, empty_id);
    assert!(!summaries[1].client.active);
    assert_eq!(summaries[1].total_projects, 0);
}

#[sqlx::test(migrations = "./migrations")]
async fn client_summaries_reject_a_foreign_actor_even_when_client_org_is_known(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Admin).await;
    let other = seed(&pool, OrgRole::Admin).await;
    assert!(
        client_summaries_for_viewer(&pool, ids.org_id, other.user_id)
            .await
            .unwrap()
            .is_empty()
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn unchanged_edit_preserves_the_row(pool: PgPool) {
    let client = client(&pool).await;
    let before = row_version(&pool, client.id).await;
    let (returned, changed) =
        update_client_record(&pool, client.org_id, client.id, "Acme", "EUR", None, None)
            .await
            .unwrap();
    let after = row_version(&pool, client.id).await;
    assert_eq!((changed, returned, after), (false, client, before));
}

#[sqlx::test(migrations = "./migrations")]
async fn client_default_rate_prevents_currency_relabelling(pool: PgPool) {
    let client = client(&pool).await;
    for rate in [0_i64, 8000] {
        sqlx::query!(
            "UPDATE clients SET default_rate_cents = $2 WHERE id = $1",
            client.id,
            rate,
        )
        .execute(&pool)
        .await
        .unwrap();
        let before = row_version(&pool, client.id).await;
        let error = update_client_record(
            &pool,
            client.org_id,
            client.id,
            "Renamed",
            "USD",
            Some("Street"),
            Some("123"),
        )
        .await
        .unwrap_err();
        assert!(matches!(
            error,
            ServerFnError::ServerError { code: CONFLICT, .. }
        ));
        assert_eq!(row_version(&pool, client.id).await, before);
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn client_default_rate_allows_detail_edits_and_equivalent_currency(pool: PgPool) {
    let client = client(&pool).await;
    sqlx::query!(
        "UPDATE clients SET default_rate_cents = $2 WHERE id = $1",
        client.id,
        8000_i64,
    )
    .execute(&pool)
    .await
    .unwrap();
    for (name, expected_change) in [("Renamed", true), ("Renamed", false)] {
        let (updated, changed) = update_client_record(
            &pool,
            client.org_id,
            client.id,
            name,
            " eur ",
            Some("Street"),
            Some("123"),
        )
        .await
        .unwrap();
        assert_eq!(
            (changed, updated.currency.as_str()),
            (expected_change, "EUR")
        );
        assert_eq!(
            (
                updated.name.as_str(),
                updated.address.as_deref(),
                updated.tax_id.as_deref()
            ),
            (name, Some("Street"), Some("123"))
        );
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn currency_edit_rechecks_a_concurrently_added_client_default_rate(pool: PgPool) {
    let client = client(&pool).await;
    let mut first = pool.begin().await.unwrap();
    let blocker = sqlx::query_scalar!("SELECT pg_backend_pid()")
        .fetch_one(&mut *first)
        .await
        .unwrap()
        .unwrap();
    sqlx::query!(
        "UPDATE clients SET default_rate_cents = $2 WHERE id = $1",
        client.id,
        0_i64,
    )
    .execute(&mut *first)
    .await
    .unwrap();
    let run_pool = pool.clone();
    let mut run = tokio::task::JoinSet::new();
    run.spawn(async move {
        update_client_record(
            &run_pool,
            client.org_id,
            client.id,
            "Acme",
            "USD",
            None,
            None,
        )
        .await
    });
    wait_for_blocked(&pool, blocker).await;
    first.commit().await.unwrap();
    let error = tokio::time::timeout(Duration::from_secs(5), run.join_next())
        .await
        .unwrap()
        .unwrap()
        .unwrap()
        .unwrap_err();
    assert!(matches!(
        error,
        ServerFnError::ServerError { code: CONFLICT, .. }
    ));
}

#[sqlx::test(migrations = "./migrations")]
async fn unchanged_activation_preserves_the_row(pool: PgPool) {
    let client = client(&pool).await;
    let before = row_version(&pool, client.id).await;
    let (returned, transition) = set_client_active_record(&pool, client.org_id, client.id, true)
        .await
        .unwrap();
    let after = row_version(&pool, client.id).await;
    assert_eq!((transition, returned, after), (None, client, before));
}

async fn competing_edit(pool: &PgPool, requested_name: &'static str) -> (Client, bool) {
    let client = client(pool).await;
    let mut first = pool.begin().await.unwrap();
    let blocker = sqlx::query_scalar!("SELECT pg_backend_pid()")
        .fetch_one(&mut *first)
        .await
        .unwrap()
        .unwrap();
    sqlx::query!(
        "UPDATE clients SET name = 'Changed' WHERE id = $1",
        client.id
    )
    .execute(&mut *first)
    .await
    .unwrap();
    let run_pool = pool.clone();
    let mut run = tokio::task::JoinSet::new();
    run.spawn(async move {
        update_client_record(
            &run_pool,
            client.org_id,
            client.id,
            requested_name,
            "EUR",
            None,
            None,
        )
        .await
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
async fn repeating_a_competing_edit_does_not_report_a_change(pool: PgPool) {
    let (client, changed) = competing_edit(&pool, "Changed").await;
    assert_eq!((changed, client.name.as_str()), (false, "Changed"));
}

#[sqlx::test(migrations = "./migrations")]
async fn restoring_values_after_a_competing_edit_reports_a_change(pool: PgPool) {
    let (client, changed) = competing_edit(&pool, "Acme").await;
    assert_eq!((changed, client.name.as_str()), (true, "Acme"));
}

async fn competing_activation(pool: &PgPool, active: bool) -> (Client, Option<ActiveTransition>) {
    let client = client(pool).await;
    let mut first = pool.begin().await.unwrap();
    let blocker = sqlx::query_scalar!("SELECT pg_backend_pid()")
        .fetch_one(&mut *first)
        .await
        .unwrap()
        .unwrap();
    sqlx::query!("UPDATE clients SET active = false WHERE id = $1", client.id)
        .execute(&mut *first)
        .await
        .unwrap();
    let run_pool = pool.clone();
    let mut run = tokio::task::JoinSet::new();
    run.spawn(async move {
        set_client_active_record(&run_pool, client.org_id, client.id, active).await
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
async fn repeating_a_competing_deactivation_does_not_report_a_transition(pool: PgPool) {
    let (client, transition) = competing_activation(&pool, false).await;
    assert_eq!((transition, client.active), (None, false));
}

#[sqlx::test(migrations = "./migrations")]
async fn restoring_a_competing_deactivation_reports_reactivation(pool: PgPool) {
    let (client, transition) = competing_activation(&pool, true).await;
    assert_eq!(
        (transition, client.active),
        (Some(ActiveTransition::Reactivated), true)
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn each_field_changes_once_and_preserves_inactive_status(pool: PgPool) {
    let client = client(&pool).await;
    set_client_active_record(&pool, client.org_id, client.id, false)
        .await
        .unwrap();
    for (name, currency, address, tax_id) in [
        ("Renamed", "EUR", None, None),
        ("Renamed", "USD", None, None),
        ("Renamed", "USD", Some("Main Street"), None),
        ("Renamed", "USD", Some(""), None),
        ("Renamed", "USD", None, None),
        ("Renamed", "USD", None, Some("123")),
        ("Renamed", "USD", None, Some("")),
        ("Renamed", "USD", None, None),
    ] {
        let (updated, changed) = update_client_record(
            &pool,
            client.org_id,
            client.id,
            name,
            currency,
            address,
            tax_id,
        )
        .await
        .unwrap();
        assert_eq!(
            (
                changed,
                updated.name.as_str(),
                updated.currency.as_str(),
                updated.address.as_deref(),
                updated.tax_id.as_deref(),
                updated.active
            ),
            (true, name, currency, address, tax_id, false)
        );
        let before = row_version(&pool, client.id).await;
        let (repeated, changed) = update_client_record(
            &pool,
            client.org_id,
            client.id,
            name,
            currency,
            address,
            tax_id,
        )
        .await
        .unwrap();
        let after = row_version(&pool, client.id).await;
        assert_eq!((changed, repeated, after), (false, updated, before));
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn both_activation_transitions_change_once_and_preserve_details(pool: PgPool) {
    let mut expected = client(&pool).await;
    for (active, transition) in [
        (false, ActiveTransition::Deactivated),
        (true, ActiveTransition::Reactivated),
    ] {
        expected.active = active;
        let (updated, actual) =
            set_client_active_record(&pool, expected.org_id, expected.id, active)
                .await
                .unwrap();
        assert_eq!((actual, updated), (Some(transition), expected.clone()));
        let before = row_version(&pool, expected.id).await;
        let (repeated, actual) =
            set_client_active_record(&pool, expected.org_id, expected.id, active)
                .await
                .unwrap();
        let after = row_version(&pool, expected.id).await;
        assert_eq!((actual, repeated, after), (None, expected.clone(), before));
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn missing_and_foreign_clients_are_not_noops(pool: PgPool) {
    let client = client(&pool).await;
    let other = seed(&pool, OrgRole::Admin).await;
    let before = row_version(&pool, client.id).await;
    for (org_id, client_id) in [
        (client.org_id, uuid::Uuid::now_v7()),
        (other.org_id, client.id),
    ] {
        let error = update_client_record(&pool, org_id, client_id, "Acme", "EUR", None, None)
            .await
            .unwrap_err();
        assert!(matches!(
            error,
            ServerFnError::ServerError {
                code: NOT_FOUND,
                ..
            }
        ));
        let error = set_client_active_record(&pool, org_id, client_id, true)
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
    assert_eq!(row_version(&pool, client.id).await, before);
}

#[sqlx::test(migrations = "./migrations")]
async fn concurrent_deletion_is_not_an_unchanged_mutation(pool: PgPool) {
    for activation in [false, true] {
        let client = client(&pool).await;
        let mut first = pool.begin().await.unwrap();
        let blocker = sqlx::query_scalar!("SELECT pg_backend_pid()")
            .fetch_one(&mut *first)
            .await
            .unwrap()
            .unwrap();
        sqlx::query!("DELETE FROM projects WHERE client_id = $1", client.id)
            .execute(&mut *first)
            .await
            .unwrap();
        sqlx::query!("DELETE FROM clients WHERE id = $1", client.id)
            .execute(&mut *first)
            .await
            .unwrap();
        let run_pool = pool.clone();
        let mut run = tokio::task::JoinSet::new();
        run.spawn(async move {
            if activation {
                set_client_active_record(&run_pool, client.org_id, client.id, true)
                    .await
                    .map(|_| ())
            } else {
                update_client_record(
                    &run_pool,
                    client.org_id,
                    client.id,
                    "Acme",
                    "EUR",
                    None,
                    None,
                )
                .await
                .map(|_| ())
            }
        });
        wait_for_blocked(&pool, blocker).await;
        first.commit().await.unwrap();
        let error = tokio::time::timeout(Duration::from_secs(5), run.join_next())
            .await
            .unwrap()
            .unwrap()
            .unwrap()
            .unwrap_err();
        assert!(matches!(
            error,
            ServerFnError::ServerError {
                code: NOT_FOUND,
                ..
            }
        ));
    }
}
