use super::*;
use crate::server_fns::budgets::progress_for_viewer;

async fn configured(pool: &PgPool, ids: &SeedIds) {
    sqlx::query!(
        "UPDATE projects SET budget_kind='hours', budget_minutes=240, rate_cents=12345 WHERE id=$1",
        ids.project_id,
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO project_settings (id,org_id,project_id,creator_id,rate_mode,report_visibility)
         VALUES ($1,$2,$3,$4,'project','project_members')",
        Uuid::now_v7(),
        ids.org_id,
        ids.project_id,
        ids.user_id,
    )
    .execute(pool)
    .await
    .unwrap();
}

async fn read(pool: &PgPool, ids: &SeedIds) -> Vec<crate::models::ProjectBudgetOverview> {
    progress_for_viewer(pool, ids.org_id, ids.user_id, "2026-09-07".parse().unwrap())
        .await
        .unwrap()
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn canonical_budget_reader_does_not_require_legacy_management(pool: PgPool) {
    let (ids, _) = fixture(
        &pool,
        OrgRole::Member,
        PermissionSelection::new(&[Permission::ProjectReadAll]),
    )
    .await;
    configured(&pool, &ids).await;
    let rows = read(&pool, &ids).await;
    assert_eq!(rows.len(), 1, "authorized empty project budget is missing");
    assert_eq!((rows[0].budget, rows[0].consumed), (Some(240), Some(0)));
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn canonical_budget_managed_scope_requires_current_designation(pool: PgPool) {
    let (ids, _) = fixture(
        &pool,
        OrgRole::Admin,
        PermissionSelection::new(&[Permission::ProjectReadManaged]),
    )
    .await;
    configured(&pool, &ids).await;
    assert!(read(&pool, &ids).await.is_empty());
    designate(&pool, &ids).await;
    assert_eq!(read(&pool, &ids).await.len(), 1);
    sqlx::query!(
        "DELETE FROM project_management_assignments WHERE org_id=$1 AND manager_id=$2",
        ids.org_id,
        ids.user_id,
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(read(&pool, &ids).await.is_empty());
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn canonical_budget_financial_revocation_withholds_allowance_and_consumption(pool: PgPool) {
    let (ids, _) = fixture(
        &pool,
        OrgRole::Admin,
        PermissionSelection::new(&[Permission::ProjectReadAll, Permission::BillableRateReadAll]),
    )
    .await;
    configured(&pool, &ids).await;
    sqlx::query!(
        "UPDATE projects SET budget_kind='amount', budget_amount_cents=86753 WHERE id=$1",
        ids.project_id,
    )
    .execute(&pool)
    .await
    .unwrap();
    crate::server_fns::test_seed::time_entry(&pool, &ids, EntryState::Open).await;
    let rows = read(&pool, &ids).await;
    assert_eq!(
        (rows[0].budget, rows[0].consumed),
        (Some(86753), Some(12345))
    );
    replace_grants(&pool, &ids, &[Permission::ProjectReadAll]).await;
    let payload = serde_json::to_value(read(&pool, &ids).await).unwrap();
    assert_eq!(
        payload.as_array().unwrap().len(),
        1,
        "project context must survive financial-only revocation"
    );
    for row in payload.as_array().unwrap() {
        for field in ["budget", "consumed"] {
            assert!(
                row.get(field).is_none(),
                "withheld {field} disclosed: {row}"
            );
        }
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn canonical_shared_budget_does_not_disclose_other_person_allocations(pool: PgPool) {
    let (ids, _) = fixture(&pool, OrgRole::Member, PermissionSelection::new(&[])).await;
    configured(&pool, &ids).await;
    sqlx::query!(
        "UPDATE project_settings SET budget_scope='person' WHERE project_id=$1",
        ids.project_id,
    )
    .execute(&pool)
    .await
    .unwrap();
    let other = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO users (id,org_id,email,name) VALUES ($1,$2,$3,'Private teammate allocation')",
        other,
        ids.org_id,
        format!("{other}@test.com"),
    )
    .execute(&pool)
    .await
    .unwrap();
    for (person, budget) in [(ids.user_id, 60_i64), (other, 180_i64)] {
        sqlx::query!(
            "INSERT INTO assignments (id,project_id,user_id) VALUES ($1,$2,$3)",
            Uuid::now_v7(),
            ids.project_id,
            person,
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query!(
            "INSERT INTO project_member_budgets (id,org_id,project_id,user_id,budget_minutes)
             VALUES ($1,$2,$3,$4,$5)",
            Uuid::now_v7(),
            ids.org_id,
            ids.project_id,
            person,
            budget,
        )
        .execute(&pool)
        .await
        .unwrap();
    }
    crate::server_fns::test_seed::time_entry(&pool, &ids, EntryState::Open).await;
    let other_entry = crate::server_fns::test_seed::time_entry(&pool, &ids, EntryState::Open).await;
    sqlx::query!(
        "UPDATE time_entries SET user_id=$2 WHERE id=$1",
        other_entry,
        other
    )
    .execute(&pool)
    .await
    .unwrap();
    let overview = read(&pool, &ids).await;
    assert_eq!(overview.len(), 1);
    assert_eq!(
        (overview[0].budget, overview[0].consumed),
        (Some(240), Some(120))
    );
    let rows = &overview[0].breakdown;
    assert!(
        rows.iter()
            .any(|row| row.user_id == Some(ids.user_id) && row.budget == Some(60))
    );
    assert!(
        !rows.iter().any(|row| row.user_id == Some(other)),
        "other person's allocation: {rows:?}"
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn canonical_shared_budget_does_not_disclose_individual_task_allocations(pool: PgPool) {
    let (ids, _) = fixture(&pool, OrgRole::Member, PermissionSelection::new(&[])).await;
    configured(&pool, &ids).await;
    sqlx::query!(
        "INSERT INTO assignments (id,project_id,user_id) VALUES ($1,$2,$3)",
        Uuid::now_v7(),
        ids.project_id,
        ids.user_id,
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE project_settings SET budget_scope='task' WHERE project_id=$1",
        ids.project_id,
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO project_tasks (project_id,task_id,billable) VALUES ($1,$2,true)",
        ids.project_id,
        ids.task_id,
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO project_task_settings (id,org_id,project_id,task_id,budget_minutes)
         VALUES ($1,$2,$3,$4,240)",
        Uuid::now_v7(),
        ids.org_id,
        ids.project_id,
        ids.task_id,
    )
    .execute(&pool)
    .await
    .unwrap();
    let rows = read(&pool, &ids).await;
    assert!(
        !rows.is_empty(),
        "shared project summary must remain available"
    );
    assert_eq!((rows[0].budget, rows[0].consumed), (Some(240), Some(0)));
    assert!(
        rows.iter().all(|row| row.breakdown.is_empty()),
        "task allowances disclosed: {rows:?}"
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn canonical_budget_skips_hidden_monetary_overflow(pool: PgPool) {
    let (ids, _) = fixture(
        &pool,
        OrgRole::Admin,
        PermissionSelection::new(&[Permission::ProjectReadAll]),
    )
    .await;
    configured(&pool, &ids).await;
    sqlx::query!("UPDATE projects SET budget_kind='amount', budget_amount_cents=86753, rate_cents=$2 WHERE id=$1",
        ids.project_id, i64::MAX).execute(&pool).await.unwrap();
    let entry = crate::server_fns::test_seed::time_entry(&pool, &ids, EntryState::Open).await;
    sqlx::query!("UPDATE time_entries SET minutes=120 WHERE id=$1", entry)
        .execute(&pool)
        .await
        .unwrap();
    let rows = read(&pool, &ids).await;
    assert_eq!((rows[0].budget, rows[0].consumed), (None, None));
    assert!(rows[0].breakdown.is_empty());
    replace_grants(
        &pool,
        &ids,
        &[Permission::ProjectReadAll, Permission::BillableRateReadAll],
    )
    .await;
    assert!(matches!(
        progress_for_viewer(
            &pool,
            ids.org_id,
            ids.user_id,
            "2026-09-07".parse().unwrap()
        )
        .await,
        Err(ServerFnError::ServerError {
            code: INTERNAL_ERROR,
            ..
        })
    ));
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn canonical_budget_managed_financial_scope_is_independent_of_project_read_all(pool: PgPool) {
    let (ids, _) = fixture(
        &pool,
        OrgRole::Admin,
        PermissionSelection::new(&[
            Permission::ProjectReadAll,
            Permission::BillableRateReadManaged,
        ]),
    )
    .await;
    configured(&pool, &ids).await;
    sqlx::query!(
        "UPDATE projects SET budget_kind='amount', budget_amount_cents=86753 WHERE id=$1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let rows = read(&pool, &ids).await;
    assert_eq!((rows[0].budget, rows[0].consumed), (None, None));
    designate(&pool, &ids).await;
    let rows = read(&pool, &ids).await;
    assert_eq!((rows[0].budget, rows[0].consumed), (Some(86753), Some(0)));
    sqlx::query!(
        "DELETE FROM project_management_assignments WHERE org_id=$1 AND manager_id=$2",
        ids.org_id,
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let rows = read(&pool, &ids).await;
    assert_eq!((rows[0].budget, rows[0].consumed), (None, None));
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn canonical_budget_summaries_exclude_unallocated_hours_and_money_but_not_zero(pool: PgPool) {
    for monetary in [false, true] {
        let (ids, _) = fixture(
            &pool,
            OrgRole::Member,
            PermissionSelection::new(&[
                Permission::ProjectReadAll,
                Permission::BillableRateReadAll,
            ]),
        )
        .await;
        configured(&pool, &ids).await;
        sqlx::query!(
            "UPDATE projects SET budget_kind=$2, rate_cents=6000 WHERE id=$1",
            ids.project_id,
            if monetary {
                BudgetKind::Amount
            } else {
                BudgetKind::Hours
            } as BudgetKind
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query!(
            "UPDATE project_settings SET budget_scope='task' WHERE project_id=$1",
            ids.project_id
        )
        .execute(&pool)
        .await
        .unwrap();
        let other_task = Uuid::now_v7();
        sqlx::query!(
            "INSERT INTO tasks (id,org_id,name) VALUES ($1,$2,'Unallocated work')",
            other_task,
            ids.org_id
        )
        .execute(&pool)
        .await
        .unwrap();
        for (task, allowance) in [(ids.task_id, Some(60_i64)), (other_task, None)] {
            sqlx::query!(
                "INSERT INTO project_tasks (project_id,task_id,billable) VALUES ($1,$2,true)",
                ids.project_id,
                task
            )
            .execute(&pool)
            .await
            .unwrap();
            sqlx::query!("INSERT INTO project_task_settings (id,org_id,project_id,task_id,budget_minutes,budget_cents)
                VALUES ($1,$2,$3,$4,$5,$6)", Uuid::now_v7(), ids.org_id, ids.project_id, task,
                if monetary { None } else { allowance },
                if monetary { allowance.map(|minutes| minutes * 100) } else { None }).execute(&pool).await.unwrap();
            let entry =
                crate::server_fns::test_seed::time_entry(&pool, &ids, EntryState::Open).await;
            sqlx::query!(
                "UPDATE time_entries SET task_id=$2, minutes=$3 WHERE id=$1",
                entry,
                task,
                if allowance.is_some() { 30 } else { 120 }
            )
            .execute(&pool)
            .await
            .unwrap();
        }
        let unit = if monetary { 100 } else { 1 };
        let rows = read(&pool, &ids).await;
        assert_eq!(
            (rows[0].budget, rows[0].consumed),
            (Some(60 * unit), Some(30 * unit))
        );
        assert!(
            rows[0]
                .breakdown
                .iter()
                .any(|row| row.task_id == Some(other_task)
                    && row.budget.is_none()
                    && row.consumed == 120 * unit)
        );
        let alerts = crate::server_fns::budgets::configured_progress(
            &mut pool.acquire().await.unwrap(),
            ids.org_id,
            ids.project_id,
            "2026-09-07".parse().unwrap(),
        )
        .await
        .unwrap();
        assert_eq!(
            alerts.len(),
            1,
            "unallocated work must not acquire an alert allowance"
        );
        assert_eq!(
            (alerts[0].task_id, alerts[0].budget, alerts[0].consumed),
            (Some(ids.task_id), 60 * unit, 30 * unit)
        );
        sqlx::query!("UPDATE project_task_settings SET budget_minutes=$3, budget_cents=$4 WHERE project_id=$1 AND task_id=$2",
            ids.project_id, other_task, if monetary { None } else { Some(0_i64) },
            if monetary { Some(0_i64) } else { None }).execute(&pool).await.unwrap();
        let rows = read(&pool, &ids).await;
        assert_eq!(
            (rows[0].budget, rows[0].consumed),
            (Some(60 * unit), Some(150 * unit))
        );
    }
}
