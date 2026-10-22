use super::*;
use crate::server_fns::test_seed::SeedIds;
use sqlx::PgPool;
use uuid::Uuid;

async fn invoice_fixture(pool: &PgPool) -> (SeedIds, InvoiceWithLines) {
    let ids = fee_tests::single_fee(pool).await;
    let mut invoice = generate_invoice_for_period(
        pool,
        ids.org_id,
        ids.client_id,
        "2026-09-01".parse().unwrap(),
        "2026-09-30".parse().unwrap(),
    )
    .await
    .unwrap();
    // Compare persisted values, including the database-generated creation time.
    invoice.invoice = crate::reports::fetch_invoice_metadata(pool, invoice.invoice.id, ids.org_id)
        .await
        .unwrap()
        .unwrap();
    (ids, invoice)
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn invoice_labels_preserve_names_and_history_for_inactive_clients(pool: PgPool) {
    let (ids, original) = invoice_fixture(&pool).await;
    let foreign = invoice_fixture(&pool).await;
    let other_client = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO clients (id, org_id, name, currency) VALUES ($1, $2, 'Acme', 'EUR')",
        other_client,
        ids.org_id,
    )
    .execute(&pool)
    .await
    .unwrap();
    let other_invoice = Uuid::now_v7();
    sqlx::query!("INSERT INTO invoices (id,org_id,client_id,number,issued_on,due_on,currency,total_cents) VALUES ($1,$2,$3,'PRIVATE-1','2026-09-07','2026-10-07','EUR',1200)", other_invoice, ids.org_id, other_client).execute(&pool).await.unwrap();
    let name = "Agencia <histórica> & 東京";
    for active in [true, false] {
        sqlx::query!(
            "UPDATE clients SET active=$2, name=$3, address='Private address', tax_id='Private tax' WHERE id=$1",
            ids.client_id, active, name,
        ).execute(&pool).await.unwrap();
        let list = fetch_list(&pool, ids.org_id, ids.user_id, None)
            .await
            .unwrap();
        let detail = fetch_detail(&pool, ids.org_id, ids.user_id, original.invoice.id)
            .await
            .unwrap();
        assert_eq!(list.len(), 2);
        let historical = list
            .iter()
            .find(|item| item.invoice.id == original.invoice.id)
            .unwrap();
        let other = list
            .iter()
            .find(|item| item.invoice.id == other_invoice)
            .unwrap();
        assert_eq!(historical.client_name, name);
        assert_eq!(historical.invoice, original.invoice);
        assert_eq!(other.client_name, "Acme");
        assert_eq!(other.invoice.client_id, other_client);
        let list = serde_json::to_value(&list).unwrap();
        let detail = serde_json::to_value(detail).unwrap();
        assert_eq!(detail["client_name"], name);
        assert_eq!(
            detail["invoice"],
            serde_json::to_value(&original.invoice).unwrap()
        );
        assert_eq!(
            detail["lines"],
            serde_json::to_value(&original.lines).unwrap()
        );
        for response in [&list, &detail] {
            let serialized = response.to_string();
            assert!(!serialized.contains("Private address"));
            assert!(!serialized.contains("Private tax"));
            assert!(!serialized.contains(&foreign.0.client_id.to_string()));
        }
        assert!(
            fetch_list(&pool, ids.org_id, ids.user_id, Some(InvoiceStatus::Sent))
                .await
                .unwrap()
                .is_empty()
        );
    }
}
