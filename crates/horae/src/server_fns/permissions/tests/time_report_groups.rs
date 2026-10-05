use super::*;
use crate::models::time_report::{TimeReportGroupCursor, TimeReportGroupQuery, TimeReportGrouping};
use crate::server_fns::permissions::time_entries::TimeReadError;
use crate::server_fns::permissions::time_reports::groups::read;
use crate::server_fns::test_seed::time_entry;
use horae_core::types::EntryState;

fn query(group_by: TimeReportGrouping) -> TimeReportGroupQuery {
    TimeReportGroupQuery {
        date_from: "2026-09-01".parse().unwrap(),
        date_to: "2026-09-30".parse().unwrap(),
        client_ids: vec![],
        project_ids: vec![],
        user_ids: vec![],
        task_ids: vec![],
        tag_ids: vec![],
        group_by,
        after: None,
        expected_requester: None,
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn grouped_reports_preserve_dimensions_own_scope_and_rounding(pool: PgPool) {
    let ids = fixture(&pool).await;
    let actor = person(&pool, ids.org_id, false, BuiltInProfile::Member).await;
    let own = time_entry(
        &pool,
        &SeedIds {
            user_id: actor,
            ..ids
        },
        EntryState::Open,
    )
    .await;
    time_entry(&pool, &ids, EntryState::Open).await;
    sqlx::query!("UPDATE time_entries SET minutes=17 WHERE id=$1", own)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query!(
        "UPDATE organizations SET round_minutes=15,round_dir='up' WHERE id=$1",
        ids.org_id
    )
    .execute(&pool)
    .await
    .unwrap();
    for (dimension, id) in [
        (TimeReportGrouping::Client, ids.client_id),
        (TimeReportGrouping::Project, ids.project_id),
        (TimeReportGrouping::Task, ids.task_id),
        (TimeReportGrouping::Person, actor),
    ] {
        let result = read(&pool, ids.org_id, actor, &query(dimension))
            .await
            .unwrap();
        assert_eq!(result.groups.len(), 1);
        assert_eq!(result.groups[0].id, id);
        assert_eq!(result.requester.user_id, actor);
        assert_eq!(result.groups[0].totals, result.totals);
        assert_eq!(
            serde_json::to_value(&result.totals).unwrap(),
            serde_json::json!({
                "entry_count":1,"total_minutes":17,"rounded_minutes":30,"billable_minutes":30
            })
        );
        let value = serde_json::to_value(&result.groups[0]).unwrap();
        assert_eq!(value.as_object().unwrap().len(), 3);
        assert_eq!(value["totals"].as_object().unwrap().len(), 4);
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn grouped_reports_keep_duplicate_names_distinct_across_pages(pool: PgPool) {
    let ids = fixture(&pool).await;
    let mut project_ids: Vec<_> = (0..503).map(|_| Uuid::now_v7()).collect();
    project_ids.sort_unstable();
    for project_id in &project_ids {
        sqlx::query!("INSERT INTO projects (id,org_id,client_id,name,currency) VALUES ($1,$2,$3,'Same label','EUR')", project_id, ids.org_id, ids.client_id).execute(&pool).await.unwrap();
        time_entry(
            &pool,
            &SeedIds {
                project_id: *project_id,
                ..ids
            },
            EntryState::Open,
        )
        .await;
    }
    let extra = time_entry(
        &pool,
        &SeedIds {
            project_id: project_ids[0],
            ..ids
        },
        EntryState::Open,
    )
    .await;
    sqlx::query!(
        "UPDATE time_entries SET minutes=0 WHERE id=$1 OR project_id=$2",
        extra,
        project_ids[502]
    )
    .execute(&pool)
    .await
    .unwrap();
    let mut filter = query(TimeReportGrouping::Project);
    let first = read(&pool, ids.org_id, ids.user_id, &filter).await.unwrap();
    assert_eq!(first.groups.len(), 500);
    assert_eq!(first.totals.entry_count, 504);
    assert_eq!(first.totals.total_minutes, 502 * 60);
    assert_eq!(first.groups[0].totals.entry_count, 2);
    assert_eq!(
        first.groups.iter().map(|row| row.id).collect::<Vec<_>>(),
        project_ids[..500]
    );
    filter.after = first.next_after;
    let last = read(&pool, ids.org_id, ids.user_id, &filter).await.unwrap();
    assert_eq!(
        last.groups.iter().map(|row| row.id).collect::<Vec<_>>(),
        project_ids[500..]
    );
    assert_eq!(last.totals, first.totals);
    assert!(last.next_after.is_none());
    assert_eq!(last.groups[2].totals.total_minutes, 0);
    filter.after = Some(TimeReportGroupCursor {
        group_by: filter.group_by,
        name: "Same label".into(),
        id: project_ids[502],
    });
    let exhausted = read(&pool, ids.org_id, ids.user_id, &filter).await.unwrap();
    assert!(exhausted.groups.is_empty());
    assert_eq!(exhausted.totals, first.totals);
}

async fn set_grants(pool: &PgPool, actor: Uuid, selection: PermissionSelection) {
    let values: Vec<String> =
        serde_json::from_value(serde_json::to_value(selection).unwrap()).unwrap();
    sqlx::query!(
        "UPDATE person_permission_states SET grants=$2,is_administrator=false WHERE user_id=$1",
        actor,
        &values
    )
    .execute(pool)
    .await
    .unwrap();
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn grouped_reports_apply_scope_union_before_every_group(pool: PgPool) {
    let ids = fixture(&pool).await;
    let target = person(&pool, ids.org_id, false, BuiltInProfile::Member).await;
    let outsider = person(&pool, ids.org_id, false, BuiltInProfile::Member).await;
    let other = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO projects (id,org_id,client_id,name,currency) VALUES ($1,$2,$3,'Other','EUR')",
        other,
        ids.org_id,
        ids.client_id
    )
    .execute(&pool)
    .await
    .unwrap();
    for user_id in [ids.user_id, target, outsider] {
        for project_id in [ids.project_id, other] {
            time_entry(
                &pool,
                &SeedIds {
                    user_id,
                    project_id,
                    ..ids
                },
                EntryState::Open,
            )
            .await;
        }
    }
    sqlx::query!("INSERT INTO person_management_assignments (id,org_id,manager_id,managed_user_id) VALUES ($1,$2,$3,$4)", Uuid::now_v7(), ids.org_id, ids.user_id, target).execute(&pool).await.unwrap();
    sqlx::query!("INSERT INTO project_management_assignments (id,org_id,manager_id,project_id) VALUES ($1,$2,$3,$4)", Uuid::now_v7(), ids.org_id, ids.user_id, ids.project_id).execute(&pool).await.unwrap();
    for selection in BuiltInProfile::ALL
        .iter()
        .map(|profile| profile.selection())
        .chain([
            PermissionSelection::new(&[Permission::TimeReadManaged]),
            PermissionSelection::new(&[
                Permission::PeopleReadAll,
                Permission::ReportProfitabilityRead,
            ]),
        ])
    {
        let expected = if selection.contains(Permission::TimeReadAll) {
            6
        } else if selection.contains(Permission::TimeReadManaged) {
            5
        } else {
            2
        };
        set_grants(&pool, ids.user_id, selection).await;
        for dimension in [
            TimeReportGrouping::Client,
            TimeReportGrouping::Project,
            TimeReportGrouping::Task,
            TimeReportGrouping::Person,
        ] {
            let page = read(&pool, ids.org_id, ids.user_id, &query(dimension))
                .await
                .unwrap();
            assert_eq!(page.totals.entry_count, expected);
            assert_eq!(page.totals.total_minutes, expected * 60);
            assert_eq!(
                page.groups
                    .iter()
                    .map(|group| group.totals.entry_count)
                    .sum::<i64>(),
                expected
            );
            assert_eq!(
                page.groups
                    .iter()
                    .map(|group| group.totals.rounded_minutes)
                    .sum::<i64>(),
                expected * 60
            );
        }
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn grouped_reports_reject_wrong_requesters_cursors_and_policy(pool: PgPool) {
    use crate::models::permission_editor::PermissionRequester;
    let ids = fixture(&pool).await;
    let mut filter = query(TimeReportGrouping::Project);
    filter.expected_requester = Some(PermissionRequester {
        org_id: ids.org_id,
        user_id: Uuid::now_v7(),
    });
    assert!(matches!(
        read(&pool, ids.org_id, ids.user_id, &filter).await,
        Err(TimeReadError::Forbidden)
    ));
    filter.expected_requester = None;
    for (dimension, name) in [
        (TimeReportGrouping::Task, "name"),
        (TimeReportGrouping::Project, "bad\0name"),
    ] {
        filter.after = Some(TimeReportGroupCursor {
            group_by: dimension,
            name: name.into(),
            id: Uuid::now_v7(),
        });
        assert!(matches!(
            read(&pool, ids.org_id, ids.user_id, &filter).await,
            Err(TimeReadError::InvalidQuery)
        ));
    }
    filter.after = None;
    filter.date_to = "2026-08-31".parse().unwrap();
    assert!(matches!(
        read(&pool, ids.org_id, ids.user_id, &filter).await,
        Err(TimeReadError::InvalidQuery)
    ));
    filter.date_to = "2026-09-30".parse().unwrap();
    for mode in [0, 99] {
        sqlx::query!(
            "UPDATE organizations SET permission_policy_version=$2 WHERE id=$1",
            ids.org_id,
            mode
        )
        .execute(&pool)
        .await
        .unwrap();
        assert!(read(&pool, ids.org_id, ids.user_id, &filter).await.is_err());
    }
    let mut wire = serde_json::to_value(&filter).unwrap();
    wire["group_by"] = serde_json::json!("profitability");
    assert!(serde_json::from_value::<TimeReportGroupQuery>(wire).is_err());
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn grouped_reports_keep_history_but_exclude_foreign_parent_records(pool: PgPool) {
    let ids = fixture(&pool).await;
    let target = person(&pool, ids.org_id, false, BuiltInProfile::Member).await;
    let foreign = seed(&pool, OrgRole::Admin).await;
    let historical = time_entry(
        &pool,
        &SeedIds {
            user_id: target,
            ..ids
        },
        EntryState::Open,
    )
    .await;
    for malformed in [
        SeedIds {
            user_id: foreign.user_id,
            ..ids
        },
        SeedIds {
            task_id: foreign.task_id,
            ..ids
        },
        SeedIds {
            project_id: foreign.project_id,
            ..ids
        },
    ] {
        time_entry(&pool, &malformed, EntryState::Open).await;
    }
    sqlx::query!(
        "UPDATE time_entries SET minutes=17,rounded_minutes=0 WHERE id=$1",
        historical
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!("UPDATE users SET active=false WHERE id=$1", target)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query!(
        "UPDATE projects SET active=false,project_type='non_billable' WHERE id=$1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!("UPDATE tasks SET active=false WHERE id=$1", ids.task_id)
        .execute(&pool)
        .await
        .unwrap();
    let page = read(
        &pool,
        ids.org_id,
        ids.user_id,
        &query(TimeReportGrouping::Person),
    )
    .await
    .unwrap();
    assert_eq!(page.groups.len(), 1);
    assert_eq!(page.groups[0].id, target);
    assert_eq!(page.totals.entry_count, 1);
    assert_eq!(page.totals.total_minutes, 17);
    assert_eq!(page.totals.rounded_minutes, 0);
    assert_eq!(page.totals.billable_minutes, 0);
    sqlx::query!(
        "UPDATE projects SET client_id=$2 WHERE id=$1",
        ids.project_id,
        foreign.client_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let empty = read(
        &pool,
        ids.org_id,
        ids.user_id,
        &query(TimeReportGrouping::Client),
    )
    .await
    .unwrap();
    assert!(empty.groups.is_empty());
    assert_eq!(empty.totals.entry_count, 0);
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn grouped_reports_filter_before_aggregation_and_keep_empty_totals(pool: PgPool) {
    let ids = fixture(&pool).await;
    let other = person(&pool, ids.org_id, false, BuiltInProfile::Member).await;
    time_entry(&pool, &ids, EntryState::Open).await;
    time_entry(
        &pool,
        &SeedIds {
            user_id: other,
            ..ids
        },
        EntryState::Open,
    )
    .await;
    let foreign = seed(&pool, OrgRole::Admin).await;
    time_entry(&pool, &foreign, EntryState::Open).await;
    let mut filter = query(TimeReportGrouping::Client);
    filter.user_ids = vec![other, other, foreign.user_id];
    let result = read(&pool, ids.org_id, ids.user_id, &filter).await.unwrap();
    assert_eq!(result.groups.len(), 1);
    assert_eq!(result.totals.entry_count, 1);
    assert_eq!(result.totals.total_minutes, 60);
    filter.project_ids = vec![foreign.project_id];
    let empty = read(&pool, ids.org_id, ids.user_id, &filter).await.unwrap();
    assert!(empty.groups.is_empty());
    assert_eq!(empty.totals.entry_count, 0);
    assert_eq!(empty.totals.total_minutes, 0);
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn grouped_reports_observe_revocation_after_wait_and_release_on_cancel(pool: PgPool) {
    let ids = fixture(&pool).await;
    let target = person(&pool, ids.org_id, false, BuiltInProfile::Member).await;
    time_entry(
        &pool,
        &SeedIds {
            user_id: target,
            ..ids
        },
        EntryState::Open,
    )
    .await;
    let reader_pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect_with(pool.connect_options().as_ref().clone())
        .await
        .unwrap();
    for cancel in [false, true] {
        let mut revoke = pool.begin().await.unwrap();
        sqlx::query!(
            "SELECT id FROM organizations WHERE id=$1 FOR UPDATE",
            ids.org_id
        )
        .fetch_one(&mut *revoke)
        .await
        .unwrap();
        let pid = sqlx::query_scalar!("SELECT pg_backend_pid()")
            .fetch_one(&mut *revoke)
            .await
            .unwrap()
            .unwrap();
        let db = reader_pool.clone();
        let org = ids.org_id;
        let actor = ids.user_id;
        let reader =
            tokio::spawn(
                async move { read(&db, org, actor, &query(TimeReportGrouping::Person)).await },
            );
        wait_for_blocked(&pool, pid).await;
        if cancel {
            reader.abort();
            assert!(reader.await.unwrap_err().is_cancelled());
            revoke.rollback().await.unwrap();
        } else {
            let values: Vec<String> = serde_json::from_value(
                serde_json::to_value(BuiltInProfile::Member.selection()).unwrap(),
            )
            .unwrap();
            sqlx::query!(
                "UPDATE person_permission_states SET grants=$2 WHERE user_id=$1",
                ids.user_id,
                &values
            )
            .execute(&mut *revoke)
            .await
            .unwrap();
            revoke.commit().await.unwrap();
            let page = reader.await.unwrap().unwrap();
            assert!(page.groups.is_empty());
            assert_eq!(
                serde_json::to_value(page.totals).unwrap(),
                serde_json::json!({
                    "entry_count":0,"total_minutes":0,"rounded_minutes":0,"billable_minutes":0
                })
            );
        }
    }
    let mut tx = tokio::time::timeout(Duration::from_secs(5), reader_pool.begin())
        .await
        .unwrap()
        .unwrap();
    sqlx::query!(
        "SELECT id FROM organizations WHERE id=$1 FOR UPDATE NOWAIT",
        ids.org_id
    )
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    tx.rollback().await.unwrap();
    reader_pool.close().await;
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn grouped_reports_recheck_actor_deactivation_after_wait(pool: PgPool) {
    let ids = fixture(&pool).await;
    time_entry(&pool, &ids, EntryState::Open).await;
    let mut deactivate = pool.begin().await.unwrap();
    sqlx::query!("UPDATE users SET active=false WHERE id=$1", ids.user_id)
        .execute(&mut *deactivate)
        .await
        .unwrap();
    let pid = sqlx::query_scalar!("SELECT pg_backend_pid()")
        .fetch_one(&mut *deactivate)
        .await
        .unwrap()
        .unwrap();
    let db = pool.clone();
    let reader = tokio::spawn(async move {
        read(
            &db,
            ids.org_id,
            ids.user_id,
            &query(TimeReportGrouping::Project),
        )
        .await
    });
    wait_for_blocked(&pool, pid).await;
    deactivate.commit().await.unwrap();
    assert!(matches!(
        reader.await.unwrap(),
        Err(TimeReadError::Forbidden)
    ));
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn grouped_reports_sum_frozen_minutes_beyond_integer_column_range(pool: PgPool) {
    let ids = fixture(&pool).await;
    let billed = time_entry(&pool, &ids, EntryState::Open).await;
    let unbilled = time_entry(&pool, &ids, EntryState::Open).await;
    sqlx::query!(
        "UPDATE time_entries SET minutes=$2,rounded_minutes=$2,billable=(id=$3) WHERE id=ANY($1)",
        &[billed, unbilled],
        i32::MAX,
        billed
    )
    .execute(&pool)
    .await
    .unwrap();
    let max = i64::from(i32::MAX);
    for dimension in [
        TimeReportGrouping::Client,
        TimeReportGrouping::Project,
        TimeReportGrouping::Task,
        TimeReportGrouping::Person,
    ] {
        let page = read(&pool, ids.org_id, ids.user_id, &query(dimension))
            .await
            .unwrap();
        assert_eq!(page.groups.len(), 1);
        assert_eq!(page.groups[0].totals, page.totals);
        assert_eq!(
            serde_json::to_value(page.totals).unwrap(),
            serde_json::json!({
                "entry_count":2,"total_minutes":max*2,"rounded_minutes":max*2,"billable_minutes":max
            })
        );
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn grouped_reports_round_each_entry_before_summing_a_group(pool: PgPool) {
    let ids = fixture(&pool).await;
    for _ in 0..2 {
        let entry = time_entry(&pool, &ids, EntryState::Open).await;
        sqlx::query!("UPDATE time_entries SET minutes=17 WHERE id=$1", entry)
            .execute(&pool)
            .await
            .unwrap();
    }
    sqlx::query!(
        "UPDATE organizations SET round_minutes=15,round_dir='up' WHERE id=$1",
        ids.org_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let page = read(
        &pool,
        ids.org_id,
        ids.user_id,
        &query(TimeReportGrouping::Project),
    )
    .await
    .unwrap();
    assert_eq!(page.groups.len(), 1);
    assert_eq!(page.groups[0].totals, page.totals);
    assert_eq!(
        serde_json::to_value(page.totals).unwrap(),
        serde_json::json!({
            "entry_count":2,"total_minutes":34,"rounded_minutes":60,"billable_minutes":60
        })
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn grouped_reports_use_the_same_c_order_for_labels_and_cursor(pool: PgPool) {
    let ids = fixture(&pool).await;
    let names = ["Z", "a", "z", "Á", "é"];
    let mut projects = Vec::new();
    for name in names.iter().rev() {
        let project = Uuid::now_v7();
        sqlx::query!(
            "INSERT INTO projects (id,org_id,client_id,name,currency) VALUES ($1,$2,$3,$4,'EUR')",
            project,
            ids.org_id,
            ids.client_id,
            name
        )
        .execute(&pool)
        .await
        .unwrap();
        time_entry(
            &pool,
            &SeedIds {
                project_id: project,
                ..ids
            },
            EntryState::Open,
        )
        .await;
        projects.push(project);
    }
    projects.reverse();
    let mut filter = query(TimeReportGrouping::Project);
    for offset in 0..=names.len() {
        let page = read(&pool, ids.org_id, ids.user_id, &filter).await.unwrap();
        assert_eq!(
            page.groups
                .iter()
                .map(|group| group.name.as_str())
                .collect::<Vec<_>>(),
            names[offset..]
        );
        assert_eq!(page.totals.entry_count, 5);
        assert_eq!(page.totals.total_minutes, 300);
        if offset < names.len() {
            filter.after = Some(TimeReportGroupCursor {
                group_by: filter.group_by,
                name: names[offset].into(),
                id: projects[offset],
            });
        }
    }
}
