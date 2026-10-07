use super::*;
use crate::models::permission_editor::PermissionRequester;
use crate::models::time_report::TimeReportGrouping;

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn grouped_xlsx_rejects_partial_loss_of_a_multi_context_group(pool: PgPool) {
    let ids = scoped(&pool, OrgRole::Member, &[Permission::TimeReadManaged]).await;
    let owner = person(&pool, ids.org_id).await;
    let other = project(&pool, &ids).await;
    for project_id in [ids.project_id, other] {
        add_entries(
            &pool,
            &SeedIds {
                user_id: owner,
                project_id,
                ..ids
            },
            1,
        )
        .await;
        sqlx::query!("INSERT INTO project_management_assignments (id,org_id,manager_id,project_id) VALUES ($1,$2,$3,$4)", Uuid::now_v7(), ids.org_id, ids.user_id, project_id).execute(&pool).await.unwrap();
    }
    let requester = PermissionRequester {
        org_id: ids.org_id,
        user_id: ids.user_id,
    };
    for revoked in [ids.project_id, other] {
        let export = time::groups::read(
            &pool,
            requester,
            &params().time_query().unwrap(),
            TimeReportGrouping::Client,
        )
        .await
        .unwrap();
        assert_eq!(export.rows.len(), 1);
        assert_eq!(export.rows[0].totals.entry_count, 2);
        sqlx::query!(
            "DELETE FROM project_management_assignments WHERE manager_id=$1 AND project_id=$2",
            ids.user_id,
            revoked
        )
        .execute(&pool)
        .await
        .unwrap();
        assert!(matches!(
            export
                .scope
                .render(ExportPermit::acquire().unwrap(), &pool, || Ok(vec![1]))
                .await,
            Err(StatusCode::FORBIDDEN)
        ));
        sqlx::query!("INSERT INTO project_management_assignments (id,org_id,manager_id,project_id) VALUES ($1,$2,$3,$4)", Uuid::now_v7(), ids.org_id, ids.user_id, revoked).execute(&pool).await.unwrap();
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn grouped_xlsx_budget_includes_contexts_in_addition_to_names(pool: PgPool) {
    let ids = scoped(&pool, OrgRole::Member, &[Permission::TimeReadOwn]).await;
    let projects: Vec<Uuid> = (0..256).map(|_| Uuid::now_v7()).collect();
    sqlx::query!("INSERT INTO projects (id,org_id,client_id,name,currency) SELECT id,$2,$3,repeat('n',32736),'EUR' FROM unnest($1::uuid[]) id", &projects, ids.org_id, ids.client_id).execute(&pool).await.unwrap();
    sqlx::query!("INSERT INTO time_entries (id,org_id,user_id,project_id,task_id,spent_date,minutes,billable) SELECT id,$2,$3,id,$4,'2026-09-07',1,true FROM unnest($1::uuid[]) id", &projects, ids.org_id, ids.user_id, ids.task_id).execute(&pool).await.unwrap();
    assert_eq!((32736 + 32) * 256, 8 * 1024 * 1024);
    assert_eq!(groups(&pool, &ids).await.unwrap().rows.len(), 256);
    sqlx::query!(
        "UPDATE projects SET name=repeat('n',32737) WHERE id=ANY($1)",
        &projects
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(matches!(
        groups(&pool, &ids).await,
        Err(StatusCode::PAYLOAD_TOO_LARGE)
    ));
}

async fn groups(pool: &PgPool, ids: &SeedIds) -> Result<time::groups::GroupExport, StatusCode> {
    time::groups::read(
        pool,
        PermissionRequester {
            org_id: ids.org_id,
            user_id: ids.user_id,
        },
        &params().time_query().unwrap(),
        TimeReportGrouping::Project,
    )
    .await
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn grouped_xlsx_limits_groups_not_source_entries_and_preserves_scope(pool: PgPool) {
    let ids = scoped(&pool, OrgRole::Member, &[Permission::TimeReadOwn]).await;
    add_entries(&pool, &ids, 10_001).await;
    let outsider = person(&pool, ids.org_id).await;
    add_entries(
        &pool,
        &SeedIds {
            user_id: outsider,
            ..ids
        },
        1,
    )
    .await;
    let export = groups(&pool, &ids).await.unwrap();
    assert_eq!(export.rows.len(), 1);
    assert_eq!(export.rows[0].id, ids.project_id);
    assert_eq!(export.rows[0].totals.entry_count, 10_001);
    assert_eq!(export.rows[0].totals.rounded_minutes, 600_060);
    assert_eq!(export.rows[0].totals.billable_minutes, 600_060);
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn grouped_xlsx_rejects_wrong_identity_legacy_policy_and_page_cursor(pool: PgPool) {
    let ids = scoped(&pool, OrgRole::Admin, &[Permission::TimeReadOwn]).await;
    let requester = PermissionRequester {
        org_id: ids.org_id,
        user_id: ids.user_id,
    };
    let mut query = params().time_query().unwrap();
    query.expected_requester = Some(PermissionRequester {
        user_id: Uuid::now_v7(),
        ..requester
    });
    assert!(matches!(
        time::groups::read(&pool, requester, &query, TimeReportGrouping::Client).await,
        Err(StatusCode::FORBIDDEN)
    ));
    query.expected_requester = Some(requester);
    query.after = Some(crate::models::time_report::TimeReportCursor {
        spent_date: query.date_from,
        project_name: String::new(),
        task_name: String::new(),
        id: Uuid::now_v7(),
    });
    assert!(matches!(
        time::groups::read(&pool, requester, &query, TimeReportGrouping::Client).await,
        Err(StatusCode::BAD_REQUEST)
    ));
    query.after = None;
    sqlx::query!(
        "UPDATE organizations SET permission_policy_version=0 WHERE id=$1",
        ids.org_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(matches!(
        time::groups::read(&pool, requester, &query, TimeReportGrouping::Client).await,
        Err(StatusCode::FORBIDDEN)
    ));
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn grouped_xlsx_reauthorizes_original_pairs_after_reassignment(pool: PgPool) {
    let ids = scoped(&pool, OrgRole::Member, &[Permission::TimeReadManaged]).await;
    let owner = person(&pool, ids.org_id).await;
    let other = project(&pool, &ids).await;
    let entry = add_entries(
        &pool,
        &SeedIds {
            user_id: owner,
            ..ids
        },
        1,
    )
    .await[0];
    sqlx::query!("INSERT INTO project_management_assignments (id,org_id,manager_id,project_id) VALUES ($1,$2,$3,$4)", Uuid::now_v7(), ids.org_id, ids.user_id, ids.project_id).execute(&pool).await.unwrap();
    let export = groups(&pool, &ids).await.unwrap();
    sqlx::query!(
        "UPDATE time_entries SET project_id=$2 WHERE id=$1",
        entry,
        other
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE project_management_assignments SET project_id=$2 WHERE manager_id=$1",
        ids.user_id,
        other
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(matches!(
        export
            .scope
            .render(ExportPermit::acquire().unwrap(), &pool, || Ok(vec![1]))
            .await,
        Err(StatusCode::FORBIDDEN)
    ));
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn grouped_xlsx_caps_groups_and_names_after_filters(pool: PgPool) {
    let ids = scoped(&pool, OrgRole::Member, &[Permission::TimeReadOwn]).await;
    let projects: Vec<Uuid> = (0..10_001).map(|_| Uuid::now_v7()).collect();
    sqlx::query!("INSERT INTO projects (id,org_id,client_id,name,currency) SELECT id,$2,$3,'Same name','EUR' FROM unnest($1::uuid[]) id", &projects, ids.org_id, ids.client_id).execute(&pool).await.unwrap();
    sqlx::query!("INSERT INTO time_entries (id,org_id,user_id,project_id,task_id,spent_date,minutes,billable) SELECT id,$2,$3,id,$4,'2026-09-07',1,true FROM unnest($1::uuid[]) id", &projects, ids.org_id, ids.user_id, ids.task_id).execute(&pool).await.unwrap();
    assert!(matches!(
        groups(&pool, &ids).await,
        Err(StatusCode::PAYLOAD_TOO_LARGE)
    ));
    let requester = PermissionRequester {
        org_id: ids.org_id,
        user_id: ids.user_id,
    };
    let mut query = params().time_query().unwrap();
    query.project_ids = projects[..10_000].to_vec();
    let export = time::groups::read(&pool, requester, &query, TimeReportGrouping::Project)
        .await
        .unwrap();
    assert_eq!(
        export.rows.len(),
        10_000,
        "identical labels must not merge distinct projects"
    );
    query.project_ids = vec![projects[0]];
    sqlx::query!(
        "UPDATE projects SET name=repeat('界',10923) WHERE id=$1",
        projects[0]
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(matches!(
        time::groups::read(&pool, requester, &query, TimeReportGrouping::Project).await,
        Err(StatusCode::PAYLOAD_TOO_LARGE)
    ));
    query.project_ids = vec![projects[1]];
    assert_eq!(
        time::groups::read(&pool, requester, &query, TimeReportGrouping::Project)
            .await
            .unwrap()
            .rows
            .len(),
        1
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn grouped_xlsx_all_dimensions_render_exact_hours_without_private_scope(pool: PgPool) {
    let ids = scoped(&pool, OrgRole::Member, &[Permission::TimeReadOwn]).await;
    let entry = add_entries(&pool, &ids, 1).await[0];
    sqlx::query!(
        "UPDATE time_entries SET minutes=61,rounded_minutes=75 WHERE id=$1",
        entry
    )
    .execute(&pool)
    .await
    .unwrap();
    let requester = PermissionRequester {
        org_id: ids.org_id,
        user_id: ids.user_id,
    };
    for (dimension, id, title) in [
        (TimeReportGrouping::Client, ids.client_id, "Client"),
        (TimeReportGrouping::Project, ids.project_id, "Project"),
        (TimeReportGrouping::Task, ids.task_id, "Task"),
        (TimeReportGrouping::Person, ids.user_id, "Teammate"),
    ] {
        let export =
            time::groups::read(&pool, requester, &params().time_query().unwrap(), dimension)
                .await
                .unwrap();
        assert_eq!(export.rows[0].id, id);
        assert_eq!(export.rows[0].totals.total_minutes, 61);
        let body = export
            .scope
            .render(ExportPermit::acquire().unwrap(), &pool, move || {
                crate::reports::groups::workbook(&export.rows, dimension)
            })
            .await
            .unwrap();
        let bytes = axum::body::to_bytes(body, crate::reports::bounded::MAX_OUTPUT_BYTES)
            .await
            .unwrap();
        assert_text_cell(&bytes, "A1", title);
        assert_text_cell(&bytes, "B2", "1.25");
        assert_text_cell(&bytes, "C2", "1.25");
        assert_text_cell(&bytes, "D2", "0.00");
        let strings = xlsx_part(&bytes, "xl/sharedStrings.xml");
        for private in [
            ids.org_id,
            ids.user_id,
            ids.project_id,
            ids.task_id,
            ids.client_id,
        ] {
            assert!(!strings.contains(&private.to_string()));
        }
    }
}
