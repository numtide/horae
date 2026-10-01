use super::*;
use crate::server_fns::test_seed::{SeedIds, seed, time_entry};
use serial_test::serial;
use sqlx::PgPool;
use uuid::Uuid;

async fn read(pool: &PgPool, ids: &SeedIds) -> Result<ProjectInvoices, ServerFnError> {
    fetch_project_invoices(pool, ids.org_id, ids.user_id, ids.project_id).await
}

async fn invoice(pool: &PgPool, ids: &SeedIds, status: InvoiceStatus, currency: &str) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query!("INSERT INTO invoices (id,org_id,client_id,number,status,issued_on,due_on,currency,notes) VALUES ($1,$2,$3,$4,$5,'2026-09-01','2026-09-30',$6,'Private invoice note')",
        id, ids.org_id, ids.client_id, format!("INV-{id}"), status as InvoiceStatus, currency
    ).execute(pool).await.unwrap();
    id
}

async fn line(pool: &PgPool, invoice: Uuid, entry: Uuid, gross: i64, discount: i64) {
    sqlx::query!("INSERT INTO invoice_line_items (id,invoice_id,time_entry_id,description,minutes,rate_cents,amount_cents,allocated_discount_cents) VALUES ($1,$2,$3,'Private line description',60,$4,$4,$5)",
        Uuid::now_v7(),invoice,entry,gross,discount
    ).execute(pool).await.unwrap();
}

#[sqlx::test(migrations = "./migrations")]
#[serial]
async fn invoices_attribute_time_and_fees_after_discount_without_whole_invoice_tax(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Admin).await;
    let first = time_entry(&pool, &ids, EntryState::Open).await;
    let other_project = Uuid::now_v7();
    sqlx::query!("INSERT INTO projects (id,org_id,client_id,name,currency) VALUES ($1,$2,$3,'Other project','EUR')", other_project, ids.org_id, ids.client_id).execute(&pool).await.unwrap();
    let other = SeedIds {
        project_id: other_project,
        ..ids
    };
    let second = time_entry(&pool, &other, EntryState::Open).await;
    let inv = invoice(&pool, &ids, InvoiceStatus::Draft, "EUR").await;
    line(&pool, inv, first, 10000, 1000).await;
    line(&pool, inv, second, 20000, 2000).await;
    let fee = Uuid::now_v7();
    sqlx::query!("INSERT INTO project_fee_occurrences (id,org_id,project_id,period_key,due_on,description,amount_cents,currency) VALUES ($1,$2,$3,'single','2026-09-01','Private fee description',5000,'EUR')", fee, ids.org_id, ids.project_id).execute(&pool).await.unwrap();
    sqlx::query!("INSERT INTO invoice_line_items (id,invoice_id,fee_occurrence_id,description,amount_cents,allocated_discount_cents) VALUES ($1,$2,$3,'Private fee line',5000,500)", Uuid::now_v7(), inv, fee).execute(&pool).await.unwrap();
    sqlx::query!("UPDATE invoices SET discount_cents=3500,discount_bps=1000,tax1_bps=1000,tax1_cents=3150,total_cents=34650 WHERE id=$1",inv).execute(&pool).await.unwrap();
    let result = read(&pool, &ids).await.unwrap();
    assert_eq!(result.invoices.len(), 1);
    assert_eq!(result.invoices[0].id, inv);
    assert_eq!(result.invoices[0].net_before_tax_cents, 13500);
    assert_eq!(result.totals["EUR"].draft_cents, 13500);
    let counterpart = read(&pool, &other).await.unwrap();
    assert_eq!(counterpart.invoices[0].net_before_tax_cents, 18000);
    assert_eq!(
        result.invoices[0].net_before_tax_cents + counterpart.invoices[0].net_before_tax_cents,
        31500
    );
    assert!(!serde_json::to_string(&result).unwrap().contains("Private"));
    sqlx::query!("UPDATE time_entries SET minutes=1 WHERE id=$1", first)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query!(
        "UPDATE projects SET rate_cents=999999,active=false WHERE id=$1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(read(&pool, &ids).await.unwrap(), result);
}

#[sqlx::test(migrations = "./migrations")]
#[serial]
async fn invoices_preserve_void_history_and_separate_historical_currencies(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Admin).await;
    let entry = time_entry(&pool, &ids, EntryState::Open).await;
    let old = invoice(&pool, &ids, InvoiceStatus::Void, "EUR").await;
    line(&pool, old, entry, 10000, 1000).await;
    let new = invoice(&pool, &ids, InvoiceStatus::Paid, "USD").await;
    line(&pool, new, entry, 12000, 0).await;
    sqlx::query!(
        "UPDATE time_entries SET state='invoiced',invoice_id=$2 WHERE id=$1",
        entry,
        new
    )
    .execute(&pool)
    .await
    .unwrap();
    let result = read(&pool, &ids).await.unwrap();
    assert_eq!(
        result.invoices.iter().map(|row| row.id).collect::<Vec<_>>(),
        [new, old]
    );
    assert_eq!(result.totals["EUR"].void_cents, 9000);
    assert_eq!(result.totals["EUR"].non_void_cents, 0);
    assert_eq!(result.totals["USD"].paid_cents, 12000);
    assert_eq!(result.totals["USD"].non_void_cents, 12000);
    assert_eq!(result.totals.len(), 2);
}

#[sqlx::test(migrations = "./migrations")]
#[serial]
async fn invoices_require_current_financial_authority_and_same_organization(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Admin).await;
    let foreign = seed(&pool, OrgRole::Admin).await;
    assert!(read(&pool, &ids).await.unwrap().invoices.is_empty());
    let inv = invoice(&pool, &foreign, InvoiceStatus::Paid, "EUR").await;
    let foreign_entry = time_entry(&pool, &foreign, EntryState::Open).await;
    line(&pool, inv, foreign_entry, 900, 0).await;
    assert!(read(&pool, &ids).await.unwrap().totals.is_empty());
    assert!(
        fetch_project_invoices(&pool, ids.org_id, foreign.user_id, ids.project_id)
            .await
            .is_err()
    );
    assert!(
        fetch_project_invoices(&pool, ids.org_id, ids.user_id, foreign.project_id)
            .await
            .is_err()
    );
    sqlx::query!(
        "INSERT INTO assignments (id,project_id,user_id,role) VALUES ($1,$2,$3,'admin')",
        Uuid::now_v7(),
        ids.project_id,
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE users SET org_role='member' WHERE id=$1",
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(matches!(
        read(&pool, &ids).await,
        Err(ServerFnError::ServerError {
            code: NOT_FOUND,
            ..
        })
    ));
    sqlx::query!(
        "UPDATE users SET org_role='manager' WHERE id=$1",
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(read(&pool, &ids).await.is_ok());
    sqlx::query!("UPDATE users SET active=false WHERE id=$1", ids.user_id)
        .execute(&pool)
        .await
        .unwrap();
    assert!(read(&pool, &ids).await.is_err());
}

#[sqlx::test(migrations = "./migrations")]
#[serial]
async fn invoices_reject_excessive_history_without_partial_totals(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Admin).await;
    let entry = time_entry(&pool, &ids, EntryState::Open).await;
    let invoices: Vec<_> = (0..5001).map(|_| Uuid::now_v7()).collect();
    let lines: Vec<_> = (0..5001).map(|_| Uuid::now_v7()).collect();
    sqlx::query!("INSERT INTO invoices (id,org_id,client_id,number,status,issued_on,due_on,currency) SELECT id,$1,$2,id::text,'void','2026-09-01','2026-09-30','EUR' FROM unnest($3::uuid[]) id", ids.org_id,ids.client_id,&invoices).execute(&pool).await.unwrap();
    sqlx::query!("INSERT INTO invoice_line_items (id,invoice_id,time_entry_id,description,minutes,rate_cents,amount_cents) SELECT id,invoice_id,$1,'Bounded invoice',60,1,1 FROM unnest($2::uuid[],$3::uuid[]) AS source(id,invoice_id)",entry,&lines,&invoices).execute(&pool).await.unwrap();
    assert!(matches!(
        read(&pool, &ids).await,
        Err(ServerFnError::ServerError {
            code: BAD_REQUEST,
            ..
        })
    ));
}
