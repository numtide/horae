use super::*;
use crate::plugin::event::ActiveTransition;
use crate::server_fns::test_seed::{seed, wait_for_blocked};
use sqlx::PgPool;
use std::time::Duration;

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
