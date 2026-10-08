//! Time-entry billing identities stay private across session-authenticated reads.

use super::*;
use horae_core::types::EntryState;

pub(super) async fn check(pool: &PgPool, api: &Api) {
    let ids = crate::server_fns::test_seed::seed(pool, OrgRole::Member).await;
    let foreign = crate::server_fns::test_seed::seed(pool, OrgRole::Admin).await;
    let target = user(pool, ids.org_id, OrgRole::Member).await;
    let target_ids = crate::server_fns::test_seed::SeedIds {
        user_id: target,
        ..ids
    };
    let entry = crate::server_fns::test_seed::time_entry(pool, &target_ids, EntryState::Open).await;
    let invoice = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO invoices (id,org_id,client_id,number,issued_on,due_on,currency,total_cents)
        VALUES ($1,$2,$3,'PRIVATE-INVOICE','2026-09-07','2026-10-07','EUR',45000)",
        invoice,
        ids.org_id,
        ids.client_id
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE time_entries SET notes='Visible work notes', rounded_minutes=60, invoice_id=$2,
        created_at='2026-09-07 12:00:00+00' WHERE id=$1",
        entry,
        invoice
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query!("UPDATE users SET oidc_subject=id::text, cost_rate_cents=6000, billable_rate_cents=10000 WHERE org_id=$1", ids.org_id).execute(pool).await.unwrap();
    for role in [OrgRole::Member, OrgRole::Manager, OrgRole::Admin] {
        sqlx::query!(
            "UPDATE users SET org_role = $2 WHERE id = $1",
            target,
            role as OrgRole
        )
        .execute(pool)
        .await
        .unwrap();
        let cookie = api.cookie(target).await;
        let response = api
            .json(
                "list_time_entries",
                json!({
                    "_user_id":foreign.user_id,"project_id":null,
                    "date_from":"2026-09-01","date_to":"2026-09-30","limit":null
                }),
                &cookie,
            )
            .await;
        for row in response.as_array().unwrap() {
            assert!(
                row.get("invoice_id").is_none(),
                "time read disclosed invoice identity: {row}"
            );
            assert_eq!(row["user_id"], json!(target));
        }
        assert_eq!(response.as_array().unwrap().len(), 1);
        assert_eq!(response[0]["id"], json!(entry));
        let stored =
            sqlx::query_scalar!("SELECT invoice_id FROM time_entries WHERE id = $1", entry)
                .fetch_one(pool)
                .await
                .unwrap();
        assert_eq!(stored, Some(invoice));
    }
}
