use super::*;
use crate::models::permission_editor::PermissionRequester;
use crate::reports::limits::time;

async fn fixture(pool: &PgPool) -> SeedIds {
    let ids = scoped(pool, OrgRole::Member, &[Permission::TimeReadOwn]).await;
    add_entries(pool, &ids, 1).await;
    let projects = project_entries(pool, &ids, ids.user_id, 3).await;
    sqlx::query!(
        "UPDATE projects SET active=false WHERE id=ANY($1)",
        &projects[..2]
    )
    .execute(pool)
    .await
    .unwrap();
    let other = person(pool, ids.org_id).await;
    add_entries(
        pool,
        &SeedIds {
            user_id: other,
            ..ids
        },
        1,
    )
    .await;
    // Project activity is independent of archived clients/tasks and row authority.
    sqlx::query!("UPDATE clients SET active=false WHERE id=$1", ids.client_id)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query!("UPDATE tasks SET active=false WHERE id=$1", ids.task_id)
        .execute(pool)
        .await
        .unwrap();
    ids
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn active_projects_only_matches_detailed_and_grouped_downloads(pool: PgPool) {
    let ids = fixture(&pool).await;
    for (active_projects_only, count) in [(false, 4), (true, 2)] {
        let options = || ExportParams {
            active_projects_only,
            ..params()
        };
        let query = options().time_query().unwrap();
        let workbook = time::entries(&pool, ids.org_id, ids.user_id, &query, None)
            .await
            .unwrap();
        assert_eq!(workbook.rows.len(), count);
        let csv = body(
            entries(pool.clone(), ids.org_id, ids.user_id, options())
                .await
                .unwrap(),
        )
        .await;
        assert_eq!(
            csv::Reader::from_reader(csv.as_slice()).records().count(),
            count
        );
        for dimension in [
            TimeReportGrouping::Client,
            TimeReportGrouping::Project,
            TimeReportGrouping::Task,
            TimeReportGrouping::Person,
        ] {
            let workbook = time::groups::read(
                &pool,
                PermissionRequester {
                    org_id: ids.org_id,
                    user_id: ids.user_id,
                },
                &query,
                dimension,
            )
            .await
            .unwrap();
            assert_eq!(
                workbook
                    .rows
                    .iter()
                    .map(|g| g.totals.rounded_minutes)
                    .sum::<i64>(),
                count as i64 * 60
            );
            let csv = body(
                groups::download(pool.clone(), ids.org_id, ids.user_id, options(), dimension)
                    .await
                    .unwrap(),
            )
            .await;
            let rows = csv::Reader::from_reader(csv.as_slice())
                .records()
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
            assert_eq!(
                rows.len(),
                if dimension == TimeReportGrouping::Project {
                    count
                } else {
                    1
                }
            );
            if dimension != TimeReportGrouping::Project {
                assert_eq!(&rows[0][1], format!("{count}.00"));
            }
        }
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn active_projects_only_empty_downloads_still_require_current_authority(pool: PgPool) {
    let ids = fixture(&pool).await;
    sqlx::query!(
        "UPDATE projects SET active=false WHERE org_id=$1",
        ids.org_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let options = || ExportParams {
        active_projects_only: true,
        ..params()
    };
    let query = options().time_query().unwrap();
    assert!(
        time::entries(&pool, ids.org_id, ids.user_id, &query, None)
            .await
            .unwrap()
            .rows
            .is_empty()
    );
    let csv = body(
        entries(pool.clone(), ids.org_id, ids.user_id, options())
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(
        csv::Reader::from_reader(csv.as_slice()).records().count(),
        0
    );
    let requester = PermissionRequester {
        org_id: ids.org_id,
        user_id: ids.user_id,
    };
    assert!(
        time::groups::read(&pool, requester, &query, TimeReportGrouping::Client)
            .await
            .unwrap()
            .rows
            .is_empty()
    );
    let csv = body(
        groups::download(
            pool.clone(),
            ids.org_id,
            ids.user_id,
            options(),
            TimeReportGrouping::Client,
        )
        .await
        .unwrap(),
    )
    .await;
    assert_eq!(
        csv::Reader::from_reader(csv.as_slice()).records().count(),
        0
    );
    sqlx::query!("UPDATE users SET active=false WHERE id=$1", ids.user_id)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        time::entries(&pool, ids.org_id, ids.user_id, &query, None)
            .await
            .err(),
        Some(StatusCode::FORBIDDEN)
    );
    assert_eq!(
        entries(pool.clone(), ids.org_id, ids.user_id, options())
            .await
            .err(),
        Some(StatusCode::FORBIDDEN)
    );
    assert_eq!(
        time::groups::read(&pool, requester, &query, TimeReportGrouping::Client)
            .await
            .err(),
        Some(StatusCode::FORBIDDEN)
    );
    assert_eq!(
        groups::download(
            pool,
            ids.org_id,
            ids.user_id,
            options(),
            TimeReportGrouping::Client
        )
        .await
        .err(),
        Some(StatusCode::FORBIDDEN)
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn active_projects_only_csv_captures_activity_at_source_snapshot(pool: PgPool) {
    let ids = fixture(&pool).await;
    let query = ExportParams {
        active_projects_only: true,
        ..params()
    }
    .time_query()
    .unwrap();
    for grouped in [false, true] {
        sqlx::query!(
            "UPDATE projects SET active=(id=$1) WHERE org_id=$2",
            ids.project_id,
            ids.org_id
        )
        .execute(&pool)
        .await
        .unwrap();
        let mut tx = pool.begin().await.unwrap();
        let (_, policy) = Authority::time(&mut tx, ids.org_id, ids.user_id)
            .await
            .unwrap();
        if grouped {
            cursor::groups::declare(
                &mut tx,
                ids.org_id,
                ids.user_id,
                &query,
                TimeReportGrouping::Client,
            )
            .await
            .unwrap();
        } else {
            cursor::declare_entries(&mut tx, ids.org_id, ids.user_id, &query)
                .await
                .unwrap();
        }
        sqlx::query!(
            "UPDATE projects SET active=NOT active WHERE org_id=$1",
            ids.org_id
        )
        .execute(&pool)
        .await
        .unwrap();
        if grouped {
            let fragments = cursor::groups::fetch(&mut tx, 128)
                .await
                .unwrap()
                .into_iter()
                .filter_map(|row| row.into_fragment().unwrap())
                .collect::<Vec<_>>();
            assert_eq!(fragments.len(), 1);
            assert_eq!(fragments[0].context, (ids.user_id, ids.project_id));
            assert_eq!(fragments[0].rounded_minutes, 60);
        } else {
            let rows = cursor::entries(&mut tx, 128)
                .await
                .unwrap()
                .into_iter()
                .filter_map(|row| row.into_entry(policy).unwrap())
                .collect::<Vec<_>>();
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0].context, (ids.user_id, ids.project_id));
        }
        tx.rollback().await.unwrap();
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn active_projects_only_workbooks_retain_source_after_archiving(pool: PgPool) {
    let ids = fixture(&pool).await;
    let query = ExportParams {
        active_projects_only: true,
        ..params()
    }
    .time_query()
    .unwrap();
    let detailed = time::entries(&pool, ids.org_id, ids.user_id, &query, None)
        .await
        .unwrap();
    let grouped = time::groups::read(
        &pool,
        PermissionRequester {
            org_id: ids.org_id,
            user_id: ids.user_id,
        },
        &query,
        TimeReportGrouping::Client,
    )
    .await
    .unwrap();
    assert_eq!(detailed.rows.len(), 2);
    assert_eq!(grouped.rows[0].totals.rounded_minutes, 120);
    sqlx::query!(
        "UPDATE projects SET active=false WHERE org_id=$1",
        ids.org_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let body = detailed
        .scope
        .render(
            crate::reports::bounded::ExportPermit::acquire().unwrap(),
            &pool,
            move || crate::reports::entries_xlsx(&detailed.rows),
        )
        .await
        .unwrap();
    assert!(
        axum::body::to_bytes(body, crate::reports::bounded::MAX_OUTPUT_BYTES)
            .await
            .unwrap()
            .starts_with(b"PK")
    );
    let body = grouped
        .scope
        .render(
            crate::reports::bounded::ExportPermit::acquire().unwrap(),
            &pool,
            move || crate::reports::groups::workbook(&grouped.rows, TimeReportGrouping::Client),
        )
        .await
        .unwrap();
    assert!(
        axum::body::to_bytes(body, crate::reports::bounded::MAX_OUTPUT_BYTES)
            .await
            .unwrap()
            .starts_with(b"PK")
    );
    assert!(
        time::entries(&pool, ids.org_id, ids.user_id, &query, None)
            .await
            .unwrap()
            .rows
            .is_empty()
    );
}
