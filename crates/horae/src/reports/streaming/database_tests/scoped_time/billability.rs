use super::*;
use crate::models::permission_editor::PermissionRequester;
use crate::models::time_report::{TimeReportBillability, TimeReportGrouping};
use crate::reports::limits::time;

async fn fixture(pool: &PgPool) -> SeedIds {
    let ids = scoped(pool, OrgRole::Member, &[Permission::TimeReadOwn]).await;
    let nonbillable_project = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO projects (id,org_id,client_id,name,currency,project_type)
        VALUES ($1,$2,$3,'Nonbillable','EUR','non_billable')",
        nonbillable_project,
        ids.org_id,
        ids.client_id
    )
    .execute(pool)
    .await
    .unwrap();
    let tasks = [ids.task_id, Uuid::now_v7(), Uuid::now_v7(), Uuid::now_v7()];
    sqlx::query!(
        "INSERT INTO tasks (id,org_id,name,billable_default)
        VALUES ($1,$4,'Default off',false),($2,$4,'Override off',true),($3,$4,'Override on',false)",
        tasks[1],
        tasks[2],
        tasks[3],
        ids.org_id
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO project_tasks (project_id,task_id,billable)
        VALUES ($1,$2,false),($1,$3,true)",
        ids.project_id,
        tasks[2],
        tasks[3]
    )
    .execute(pool)
    .await
    .unwrap();
    let invoice = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO invoices (id,org_id,client_id,number,issued_on,due_on,currency,total_cents)
        VALUES ($1,$2,$3,'private-filter-invoice','2026-09-07','2026-10-07','EUR',12345)",
        invoice,
        ids.org_id,
        ids.client_id
    )
    .execute(pool)
    .await
    .unwrap();
    for (index, task) in [
        tasks[0], tasks[0], tasks[1], tasks[2], tasks[3], tasks[0], tasks[0],
    ]
    .into_iter()
    .enumerate()
    {
        let project = if index >= 5 {
            nonbillable_project
        } else {
            ids.project_id
        };
        let entry = time_entry(
            pool,
            &SeedIds {
                project_id: project,
                task_id: task,
                ..ids
            },
            EntryState::Open,
        )
        .await;
        sqlx::query!("UPDATE time_entries SET minutes=17,billable=$2,invoice_id=$3,rounded_minutes=$4 WHERE id=$1",
            entry, index != 1, (index == 6).then_some(invoice), (index == 6).then_some(0_i32))
            .execute(pool).await.unwrap();
    }
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
    sqlx::query!(
        "UPDATE organizations SET round_minutes=15,round_dir='up' WHERE id=$1",
        ids.org_id
    )
    .execute(pool)
    .await
    .unwrap();
    ids
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn billability_matches_effective_rows_totals_and_all_download_sources(pool: PgPool) {
    let ids = fixture(&pool).await;
    let requester = PermissionRequester {
        org_id: ids.org_id,
        user_id: ids.user_id,
    };
    for (billability, count, minutes, rounded, billable) in [
        (TimeReportBillability::All, 7, 119, 180, 60),
        (TimeReportBillability::Billable, 3, 51, 60, 60),
        (TimeReportBillability::NonBillable, 4, 68, 120, 0),
    ] {
        let options = || ExportParams {
            billability,
            ..params()
        };
        let query = options().time_query().unwrap();
        let workbook = time::entries(&pool, ids.org_id, ids.user_id, &query, None)
            .await
            .unwrap();
        assert_eq!(workbook.rows.len(), count);
        assert_eq!(
            workbook
                .rows
                .iter()
                .map(|row| i64::from(row.minutes))
                .sum::<i64>(),
            minutes
        );
        assert_eq!(
            workbook
                .rows
                .iter()
                .map(|row| i64::from(row.rounded_minutes.unwrap()))
                .sum::<i64>(),
            rounded
        );
        let csv = body(
            entries(pool.clone(), ids.org_id, ids.user_id, options())
                .await
                .unwrap(),
        )
        .await;
        let records = csv::Reader::from_reader(csv.as_slice())
            .records()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(records.len(), count);
        assert!(
            !String::from_utf8(csv)
                .unwrap()
                .contains("private-filter-invoice")
        );
        for dimension in [
            TimeReportGrouping::Client,
            TimeReportGrouping::Project,
            TimeReportGrouping::Task,
            TimeReportGrouping::Person,
        ] {
            let workbook = time::groups::read(&pool, requester, &query, dimension)
                .await
                .unwrap();
            assert_eq!(
                workbook
                    .rows
                    .iter()
                    .map(|g| g.totals.entry_count)
                    .sum::<i64>(),
                count as i64
            );
            assert_eq!(
                workbook
                    .rows
                    .iter()
                    .map(|g| g.totals.total_minutes)
                    .sum::<i64>(),
                minutes
            );
            assert_eq!(
                workbook
                    .rows
                    .iter()
                    .map(|g| g.totals.rounded_minutes)
                    .sum::<i64>(),
                rounded
            );
            assert_eq!(
                workbook
                    .rows
                    .iter()
                    .map(|g| g.totals.billable_minutes)
                    .sum::<i64>(),
                billable
            );
            let csv = body(
                groups::download(pool.clone(), ids.org_id, ids.user_id, options(), dimension)
                    .await
                    .unwrap(),
            )
            .await;
            let records = csv::Reader::from_reader(csv.as_slice())
                .records()
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
            assert_eq!(records.len(), workbook.rows.len());
            for (record, group) in records.iter().zip(&workbook.rows) {
                assert_eq!(&record[0], group.name);
                assert_eq!(
                    &record[1],
                    horae_core::duration::format_hours2(group.totals.rounded_minutes)
                );
                assert_eq!(
                    &record[2],
                    horae_core::duration::format_hours2(group.totals.billable_minutes)
                );
            }
        }
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn billability_csv_uses_the_captured_source_after_entry_changes(pool: PgPool) {
    for grouped in [false, true] {
        let ids = fixture(&pool).await;
        let query = ExportParams {
            billability: TimeReportBillability::Billable,
            ..params()
        }
        .time_query()
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
            "UPDATE time_entries SET billable=false WHERE org_id=$1",
            ids.org_id
        )
        .execute(&pool)
        .await
        .unwrap();
        if grouped {
            let rows = cursor::groups::fetch(&mut tx, 128)
                .await
                .unwrap()
                .into_iter()
                .filter_map(|row| row.into_fragment().unwrap())
                .collect::<Vec<_>>();
            assert_eq!(rows.len(), 2);
            assert!(
                rows.iter()
                    .all(|row| row.rounded_minutes == 60 && row.billable_minutes == 60)
            );
        } else {
            let rows = cursor::entries(&mut tx, 128)
                .await
                .unwrap()
                .into_iter()
                .filter_map(|row| row.into_entry(policy).unwrap())
                .collect::<Vec<_>>();
            assert_eq!(rows.len(), 3);
            assert!(rows.iter().all(|row| row.row.billable));
        }
        tx.rollback().await.unwrap();
        assert!(
            time::entries(&pool, ids.org_id, ids.user_id, &query, None)
                .await
                .unwrap()
                .rows
                .is_empty()
        );
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn billability_empty_exports_still_require_active_authority(pool: PgPool) {
    let ids = fixture(&pool).await;
    sqlx::query!(
        "UPDATE time_entries SET billable=false WHERE org_id=$1",
        ids.org_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let options = || ExportParams {
        billability: TimeReportBillability::Billable,
        ..params()
    };
    let query = options().time_query().unwrap();
    let requester = PermissionRequester {
        org_id: ids.org_id,
        user_id: ids.user_id,
    };
    assert!(
        time::entries(&pool, ids.org_id, ids.user_id, &query, None)
            .await
            .unwrap()
            .rows
            .is_empty()
    );
    assert!(
        time::groups::read(&pool, requester, &query, TimeReportGrouping::Client)
            .await
            .unwrap()
            .rows
            .is_empty()
    );
    for grouped in [false, true] {
        let response = if grouped {
            groups::download(
                pool.clone(),
                ids.org_id,
                ids.user_id,
                options(),
                TimeReportGrouping::Client,
            )
            .await
            .unwrap()
        } else {
            entries(pool.clone(), ids.org_id, ids.user_id, options())
                .await
                .unwrap()
        };
        let csv = body(response).await;
        assert_eq!(
            csv::Reader::from_reader(csv.as_slice()).records().count(),
            0
        );
    }
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
        time::groups::read(&pool, requester, &query, TimeReportGrouping::Client)
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
