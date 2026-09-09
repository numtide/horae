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
