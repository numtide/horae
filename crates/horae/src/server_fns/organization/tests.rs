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
    let (returned, changed) = update_org_branding_record(&pool, ids.org_id, &branding)
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
            let (returned, changed) = update_org_branding_record(&pool, ids.org_id, &requested)
                .await
                .unwrap();
            assert_eq!(
                (changed, &returned),
                (true, &requested),
                "{field}: {value:?}"
            );
            let before = row_version(&pool, ids.org_id).await;
            let (repeated, changed) = update_org_branding_record(&pool, ids.org_id, &requested)
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
    run.spawn(async move { update_org_branding_record(&run_pool, ids.org_id, &requested).await });
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
    let error = update_org_branding_record(&pool, uuid::Uuid::now_v7(), &default_branding())
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
