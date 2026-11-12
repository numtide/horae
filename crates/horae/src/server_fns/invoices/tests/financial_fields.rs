//! Invoice payloads on stored grants: who may open them, and that a person who
//! manages invoices without rate or cost grants never receives a cost rate.

use super::*;
use crate::ops::testing::set_grants;
use horae_core::permissions::catalog::{BuiltInProfile, Permission};

const COST_RATE: i64 = 777_123;
const PROJECT_COST_RATE: i64 = 778_899;

/// A draft invoice for one hour of time whose author has a distinctive cost
/// rate, both on the person and as a project-specific override.
async fn costed_draft(pool: &PgPool) -> (crate::server_fns::test_seed::SeedIds, uuid::Uuid) {
    let ids = seed(pool, BuiltInProfile::Administrator).await;
    time_entry(pool, &ids, EntryState::Open).await;
    sqlx::query!(
        "UPDATE users SET billable_rate_cents = 3000, cost_rate_cents = $2 WHERE id = $1",
        ids.user_id,
        COST_RATE
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE projects SET rate_cents = 6000 WHERE id = $1",
        ids.project_id
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO assignments (id,project_id,user_id,role) VALUES ($1,$2,$3,'lead')",
        uuid::Uuid::now_v7(),
        ids.project_id,
        ids.user_id
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO project_member_costs (id,org_id,project_id,user_id,cost_rate_cents) VALUES ($1,$2,$3,$4,$5)",
        uuid::Uuid::now_v7(),
        ids.org_id,
        ids.project_id,
        ids.user_id,
        PROJECT_COST_RATE
    )
    .execute(pool)
    .await
    .unwrap();
    let day = "2026-09-07".parse().unwrap();
    let invoice = generate_invoice_for_period(pool, ids.org_id, ids.client_id, day, day)
        .await
        .unwrap();
    // Uninvoiced time, so the preview has a line priced from live rates.
    time_entry(pool, &ids, EntryState::Open).await;
    (ids, invoice.invoice.id)
}

fn assert_no_cost(payload: serde_json::Value) {
    let text = payload.to_string();
    for cost in [COST_RATE, PROJECT_COST_RATE] {
        assert!(!text.contains(&cost.to_string()), "{text}");
    }
}

fn forbidden<T: std::fmt::Debug>(result: Result<T, ServerFnError>) {
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

/// Issued invoices disclose the rates they recorded to their invoice grants
/// alone; preparing a new one prices work from current billable rates and
/// needs `BillableRateReadAll` too. No payload carries a cost rate.
#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn invoice_managers_without_rate_or_cost_grants_receive_no_cost_rates(pool: PgPool) {
    let (ids, id) = costed_draft(&pool).await;
    set_grants(
        &mut pool.acquire().await.unwrap(),
        &ids,
        &[Permission::InvoiceWriteAll],
    )
    .await
    .unwrap();
    let day = "2026-09-07".parse().unwrap();
    let (org, actor) = (ids.org_id, ids.user_id);
    assert_no_cost(json(fetch_list(&pool, org, actor, None).await.unwrap()));
    assert_no_cost(json(fetch_detail(&pool, org, actor, id).await.unwrap()));
    assert_no_cost(json(editing::load(&pool, org, actor, id).await.unwrap()));
    forbidden(preview::prepare(&pool, (org, actor), ids.client_id, (day, day), None, None).await);
    set_grants(
        &mut pool.acquire().await.unwrap(),
        &ids,
        &[Permission::InvoiceWriteAll, Permission::BillableRateReadAll],
    )
    .await
    .unwrap();
    assert_no_cost(json(
        preview::prepare(&pool, (org, actor), ids.client_id, (day, day), None, None)
            .await
            .unwrap(),
    ));
}

/// Generation rechecks the billable-rate grant inside its own transaction:
/// a review prepared with it cannot be generated once it is gone.
#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn generating_an_invoice_needs_everyones_billable_rates(pool: PgPool) {
    let (ids, _) = costed_draft(&pool).await;
    let day: chrono::NaiveDate = "2026-09-07".parse().unwrap();
    let (org, actor) = (ids.org_id, ids.user_id);
    let preparer = [Permission::InvoiceWriteAll, Permission::BillableRateReadAll];
    set_grants(&mut pool.acquire().await.unwrap(), &ids, &preparer)
        .await
        .unwrap();
    let request = InvoiceGenerationRequest {
        request_id: uuid::Uuid::now_v7(),
        review: preview::prepare(&pool, (org, actor), ids.client_id, (day, day), None, None)
            .await
            .unwrap(),
        confirmed_excess: vec![],
    };
    let generate = || {
        generate_invoice_with_request(
            &pool,
            org,
            ids.client_id,
            (day, day),
            None,
            None,
            Some((&request, actor)),
        )
    };
    for grants in [
        &[Permission::InvoiceWriteAll][..],
        &[Permission::BillableRateReadAll, Permission::InvoiceReadAll],
    ] {
        set_grants(&mut pool.acquire().await.unwrap(), &ids, grants)
            .await
            .unwrap();
        forbidden(generate().await);
    }
    assert_eq!(
        sqlx::query_scalar!(
            "SELECT count(*) FROM invoice_generation_requests WHERE org_id = $1",
            org
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        Some(0)
    );
    set_grants(&mut pool.acquire().await.unwrap(), &ids, &preparer)
        .await
        .unwrap();
    let (generated, created) = generate().await.unwrap();
    assert!(created);
    assert_no_cost(json(generated));
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn invoice_readers_open_invoices_but_not_the_writing_workflows(pool: PgPool) {
    let (ids, id) = costed_draft(&pool).await;
    set_grants(
        &mut pool.acquire().await.unwrap(),
        &ids,
        &[Permission::InvoiceReadAll],
    )
    .await
    .unwrap();
    let day = "2026-09-07".parse().unwrap();
    let (org, actor) = (ids.org_id, ids.user_id);
    assert_no_cost(json(fetch_list(&pool, org, actor, None).await.unwrap()));
    assert_no_cost(json(fetch_detail(&pool, org, actor, id).await.unwrap()));
    forbidden(editing::load(&pool, org, actor, id).await);
    forbidden(preview::prepare(&pool, (org, actor), ids.client_id, (day, day), None, None).await);
    forbidden(transition_invoice(&pool, org, id, InvoiceStatus::Sent, actor).await);
}

/// Profiles without an organization-wide invoice grant, including the managed
/// invoice grants whose scope is still undecided, open nothing.
#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn people_without_whole_invoice_grants_open_no_invoice(pool: PgPool) {
    let (ids, id) = costed_draft(&pool).await;
    let (org, actor) = (ids.org_id, ids.user_id);
    let selections: [&[Permission]; 4] = [
        &[],
        &[Permission::InvoiceReadManaged],
        &[
            Permission::InvoiceWriteManaged,
            Permission::ProjectWriteManaged,
        ],
        &[
            Permission::BillableRateWriteAll,
            Permission::CostRateWriteAll,
            Permission::ClientWriteAll,
        ],
    ];
    for grants in selections {
        set_grants(&mut pool.acquire().await.unwrap(), &ids, grants)
            .await
            .unwrap();
        forbidden(fetch_list(&pool, org, actor, None).await);
        forbidden(fetch_detail(&pool, org, actor, id).await);
        forbidden(editing::load(&pool, org, actor, id).await);
        forbidden(transition_invoice(&pool, org, id, InvoiceStatus::Void, actor).await);
    }
    // Being an administrator is not the grant either.
    crate::ops::testing::provision(&pool, &ids, &[], true).await;
    forbidden(fetch_list(&pool, org, actor, None).await);
}

/// The built-in profiles that hold the invoice grants manage invoices; the
/// others cannot even list them.
#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn built_in_profiles_follow_their_invoice_grants(pool: PgPool) {
    for profile in [
        BuiltInProfile::Member,
        BuiltInProfile::ProjectManager,
        BuiltInProfile::PeopleAdmin,
        BuiltInProfile::Accounting,
        BuiltInProfile::ExecutiveManager,
        BuiltInProfile::Administrator,
    ] {
        let (ids, id) = costed_draft(&pool).await;
        set_grants(
            &mut pool.acquire().await.unwrap(),
            &ids,
            profile.direct_permissions(),
        )
        .await
        .unwrap();
        let (org, actor) = (ids.org_id, ids.user_id);
        let manages = matches!(
            profile,
            BuiltInProfile::Accounting
                | BuiltInProfile::ExecutiveManager
                | BuiltInProfile::Administrator
        );
        assert_eq!(
            fetch_list(&pool, org, actor, None).await.is_ok(),
            manages,
            "{profile:?}"
        );
        let sent = transition_invoice(&pool, org, id, InvoiceStatus::Sent, actor).await;
        if manages {
            assert_eq!(sent.unwrap().status, InvoiceStatus::Sent, "{profile:?}");
        } else {
            forbidden(sent);
        }
    }
}

fn json(value: impl serde::Serialize) -> serde_json::Value {
    serde_json::to_value(value).unwrap()
}
