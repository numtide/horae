use super::*;
use crate::server_fns::test_seed::time_entry;

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn hidden_spend_overflow_does_not_prevent_authorized_hour_totals(pool: PgPool) {
    let ids = tracked(&pool, OrgRole::Admin, &[Permission::ProjectReadAll]).await;
    sqlx::query!(
        "UPDATE projects SET rate_cents=$2 WHERE id=$1",
        ids.project_id,
        i64::MAX
    )
    .execute(&pool)
    .await
    .unwrap();
    time_entry(&pool, &ids, EntryState::Open).await;
    let rows = fetch_project_spend(&pool, ids.org_id, ids.user_id)
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!((rows[0].spent_minutes, rows[0].spent_cents), (120, None));
    replace_grants(
        &pool,
        &ids,
        &[Permission::ProjectReadAll, Permission::BillableRateReadAll],
    )
    .await;
    assert!(matches!(
        fetch_project_spend(&pool, ids.org_id, ids.user_id).await,
        Err(ServerFnError::ServerError {
            code: INTERNAL_ERROR,
            ..
        })
    ));
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn canonical_spend_excludes_entries_with_foreign_person_parents(pool: PgPool) {
    let ids = tracked(
        &pool,
        OrgRole::Admin,
        &[Permission::ProjectReadAll, Permission::BillableRateReadAll],
    )
    .await;
    let foreign = seed(&pool, OrgRole::Admin).await;
    sqlx::query!(
        "UPDATE time_entries SET user_id=$2 WHERE project_id=$1",
        ids.project_id,
        foreign.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let rows = fetch_project_spend(&pool, ids.org_id, ids.user_id)
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!((rows[0].spent_minutes, rows[0].spent_cents), (0, Some(0)));
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn spend_rechecks_financial_only_revocation_after_organization_wait(pool: PgPool) {
    let ids = tracked(
        &pool,
        OrgRole::Admin,
        &[Permission::ProjectReadAll, Permission::BillableRateReadAll],
    )
    .await;
    let mut hold = pool.begin().await.unwrap();
    sqlx::query!(
        "UPDATE organizations SET access_revision=access_revision+1 WHERE id=$1",
        ids.org_id
    )
    .execute(&mut *hold)
    .await
    .unwrap();
    let holder = sqlx::query_scalar!("SELECT pg_backend_pid()")
        .fetch_one(&mut *hold)
        .await
        .unwrap()
        .unwrap();
    let pending_pool = pool.clone();
    let pending =
        tokio::spawn(
            async move { fetch_project_spend(&pending_pool, ids.org_id, ids.user_id).await },
        );
    crate::server_fns::test_seed::wait_for_blocked(&pool, holder).await;
    let grants: Vec<String> =
        serde_json::from_value(serde_json::json!(PermissionSelection::new(&[
            Permission::ProjectReadAll
        ])))
        .unwrap();
    sqlx::query!(
        "UPDATE person_permission_states SET grants=$2 WHERE user_id=$1",
        ids.user_id,
        &grants
    )
    .execute(&mut *hold)
    .await
    .unwrap();
    hold.commit().await.unwrap();
    let rows = pending.await.unwrap().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!((rows[0].spent_minutes, rows[0].spent_cents), (60, None));
}

async fn tracked(pool: &PgPool, role: OrgRole, permissions: &[Permission]) -> SeedIds {
    let (ids, _) = fixture(pool, role, PermissionSelection::new(permissions)).await;
    sqlx::query!(
        "UPDATE projects SET rate_cents=12345 WHERE id=$1",
        ids.project_id
    )
    .execute(pool)
    .await
    .unwrap();
    time_entry(pool, &ids, EntryState::Open).await;
    ids
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn canonical_spend_reads_do_not_require_a_legacy_manager_role(pool: PgPool) {
    let ids = tracked(&pool, OrgRole::Member, &[Permission::ProjectReadAll]).await;
    let rows = fetch_project_spend(&pool, ids.org_id, ids.user_id)
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(
        (rows[0].project_id, rows[0].spent_minutes),
        (ids.project_id, 60)
    );
    assert!(
        serde_json::to_value(&rows[0])
            .unwrap()
            .get("spent_cents")
            .is_none()
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn canonical_spend_withholds_money_after_financial_revocation(pool: PgPool) {
    let ids = tracked(
        &pool,
        OrgRole::Admin,
        &[Permission::ProjectReadAll, Permission::BillableRateReadAll],
    )
    .await;
    let initial = fetch_project_spend(&pool, ids.org_id, ids.user_id)
        .await
        .unwrap();
    assert_eq!(
        serde_json::to_value(&initial[0]).unwrap()["spent_cents"],
        serde_json::json!(12345)
    );
    replace_grants(&pool, &ids, &[Permission::ProjectReadAll]).await;
    let rows = fetch_project_spend(&pool, ids.org_id, ids.user_id)
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].spent_minutes, 60);
    let payload = serde_json::to_value(&rows[0]).unwrap();
    assert!(
        payload.get("spent_cents").is_none(),
        "revoked amount: {payload}"
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn canonical_spend_managed_scope_requires_current_project_designation(pool: PgPool) {
    let ids = tracked(
        &pool,
        OrgRole::Manager,
        &[
            Permission::ProjectReadManaged,
            Permission::BillableRateReadManaged,
        ],
    )
    .await;
    assert!(
        fetch_project_spend(&pool, ids.org_id, ids.user_id)
            .await
            .unwrap()
            .is_empty()
    );
    designate(&pool, &ids).await;
    let rows = fetch_project_spend(&pool, ids.org_id, ids.user_id)
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(
        serde_json::to_value(&rows[0]).unwrap()["spent_cents"],
        serde_json::json!(12345)
    );
    sqlx::query!(
        "DELETE FROM project_management_assignments WHERE org_id=$1 AND manager_id=$2",
        ids.org_id,
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(
        fetch_project_spend(&pool, ids.org_id, ids.user_id)
            .await
            .unwrap()
            .is_empty()
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn canonical_spend_zero_and_withheld_money_remain_distinct_without_entries(pool: PgPool) {
    let (ids, _) = fixture(
        &pool,
        OrgRole::Admin,
        PermissionSelection::new(&[Permission::ProjectReadAll, Permission::BillableRateReadAll]),
    )
    .await;
    let rows = fetch_project_spend(&pool, ids.org_id, ids.user_id)
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].spent_minutes, 0);
    assert_eq!(
        serde_json::to_value(&rows[0]).unwrap()["spent_cents"],
        serde_json::json!(0)
    );
    replace_grants(&pool, &ids, &[Permission::ProjectReadAll]).await;
    let rows = fetch_project_spend(&pool, ids.org_id, ids.user_id)
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].spent_minutes, 0);
    assert!(
        serde_json::to_value(&rows[0])
            .unwrap()
            .get("spent_cents")
            .is_none()
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn canonical_shared_progress_and_report_permissions_do_not_open_ordinary_money(pool: PgPool) {
    let ids = tracked(
        &pool,
        OrgRole::Member,
        &[Permission::ReportProfitabilityRead],
    )
    .await;
    sqlx::query!(
        "INSERT INTO assignments (id,project_id,user_id,role) VALUES ($1,$2,$3,'freelancer')",
        Uuid::now_v7(),
        ids.project_id,
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let rows = fetch_project_spend(&pool, ids.org_id, ids.user_id)
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].spent_minutes, 60);
    let payload = serde_json::to_value(&rows[0]).unwrap();
    assert!(
        payload.get("spent_cents").is_none(),
        "report-only amount: {payload}"
    );
}

/// Exact (minutes, cents) per project for the shapes the grouped query has to
/// get right at once: an invoiced entry (its line amount wins over the live
/// rate), money the viewer may not see, a project without entries, and an entry
/// whose invoice is missing from the organization. The foreign key keeps
/// `invoice_id` pointing at a real invoice, so "missing" is an invoice of
/// another organization, which the org-scoped join cannot see.
#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn spend_totals_are_exact_for_invoiced_hidden_empty_and_missing_invoice_entries(
    pool: PgPool,
) {
    // One 60-minute entry at 12345 cents an hour on the managed project.
    let ids = tracked(
        &pool,
        OrgRole::Member,
        &[
            Permission::ProjectReadAll,
            Permission::BillableRateReadManaged,
        ],
    )
    .await;
    designate(&pool, &ids).await;
    let hidden = add_project(&pool, &ids).await;
    let empty = add_project(&pool, &ids).await;
    designate(
        &pool,
        &SeedIds {
            project_id: empty,
            ..ids
        },
    )
    .await;
    add_entry(&pool, &ids, hidden, 60, None).await;

    let invoice = add_invoice(&pool, ids.org_id, ids.client_id).await;
    let invoiced = add_entry(&pool, &ids, ids.project_id, 30, Some(invoice)).await;
    sqlx::query!(
        "INSERT INTO invoice_line_items (id, invoice_id, time_entry_id, description, minutes, rate_cents, amount_cents)
         VALUES ($1, $2, $3, 'Invoiced time', 30, 1998, 999)",
        Uuid::now_v7(),
        invoice,
        invoiced,
    )
    .execute(&pool)
    .await
    .unwrap();

    let foreign = seed(&pool, OrgRole::Admin).await;
    let missing = add_invoice(&pool, foreign.org_id, foreign.client_id).await;
    add_entry(&pool, &ids, ids.project_id, 45, Some(missing)).await;

    let totals: std::collections::BTreeMap<_, _> =
        fetch_project_spend(&pool, ids.org_id, ids.user_id)
            .await
            .unwrap()
            .into_iter()
            .map(|row| (row.project_id, (row.spent_minutes, row.spent_cents)))
            .collect();
    assert_eq!(
        totals,
        std::collections::BTreeMap::from([
            // 60 live + 30 invoiced; the missing-invoice entry counts neither
            // minutes nor money.
            (ids.project_id, (90, Some(12345 + 999))),
            (hidden, (60, None)),
            (empty, (0, Some(0))),
        ])
    );
}

async fn add_project(pool: &PgPool, ids: &SeedIds) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO projects (id, org_id, client_id, name, currency, rate_cents)
         VALUES ($1, $2, $3, $4, 'EUR', 12345)",
        id,
        ids.org_id,
        ids.client_id,
        format!("Project {id}"),
    )
    .execute(pool)
    .await
    .unwrap();
    id
}

async fn add_entry(
    pool: &PgPool,
    ids: &SeedIds,
    project_id: Uuid,
    minutes: i32,
    invoice_id: Option<Uuid>,
) -> Uuid {
    let id = Uuid::now_v7();
    let state = if invoice_id.is_some() {
        EntryState::Invoiced
    } else {
        EntryState::Open
    };
    sqlx::query!(
        "INSERT INTO time_entries
           (id, org_id, user_id, project_id, task_id, spent_date, minutes, billable, state, invoice_id)
         VALUES ($1, $2, $3, $4, $5, '2026-09-07', $6, true, $7, $8)",
        id,
        ids.org_id,
        ids.user_id,
        project_id,
        ids.task_id,
        minutes,
        state as EntryState,
        invoice_id,
    )
    .execute(pool)
    .await
    .unwrap();
    id
}

async fn add_invoice(pool: &PgPool, org_id: Uuid, client_id: Uuid) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO invoices (id, org_id, client_id, number, status, issued_on, due_on, currency, total_cents)
         VALUES ($1, $2, $3, $4, 'sent', '2026-09-07', '2026-10-07', 'EUR', 999)",
        id,
        org_id,
        client_id,
        format!("SPEND-{id}"),
    )
    .execute(pool)
    .await
    .unwrap();
    id
}
