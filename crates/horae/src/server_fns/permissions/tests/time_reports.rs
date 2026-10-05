use super::*;
use crate::models::time_report::{TimeReportCursor, TimeReportQuery};
use crate::server_fns::permissions::time_entries::TimeReadError;
use crate::server_fns::permissions::time_reports::read;
use crate::server_fns::test_seed::time_entry;
use horae_core::types::EntryState;

fn query() -> TimeReportQuery {
    TimeReportQuery {
        date_from: "2026-09-01".parse().unwrap(),
        date_to: "2026-09-30".parse().unwrap(),
        client_ids: vec![],
        project_ids: vec![],
        user_ids: vec![],
        task_ids: vec![],
        tag_ids: vec![],
        after: None,
        expected_requester: None,
    }
}

fn assert_totals(
    page: &crate::models::time_report::TimeReportPage,
    count: i64,
    actual: i64,
    rounded: i64,
    billable: i64,
) {
    assert_eq!(
        serde_json::to_value(page).unwrap()["totals"],
        serde_json::json!({
            "entry_count":count, "total_minutes":actual,
            "rounded_minutes":rounded, "billable_minutes":billable
        })
    );
}

async fn grants(pool: &PgPool, actor: Uuid, selection: PermissionSelection) {
    let values: Vec<String> =
        serde_json::from_value(serde_json::to_value(selection).unwrap()).unwrap();
    sqlx::query!(
        "UPDATE person_permission_states SET grants=$2, is_administrator=false WHERE user_id=$1",
        actor,
        &values
    )
    .execute(pool)
    .await
    .unwrap();
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn detailed_report_uses_own_scope_without_directory_or_financial_grants(pool: PgPool) {
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
    let page = read(&pool, ids.org_id, actor, &query()).await.unwrap();
    assert_eq!(
        page.entries
            .iter()
            .map(|entry| entry.id)
            .collect::<Vec<_>>(),
        [own]
    );
    assert_eq!(page.requester.user_id, actor);
    assert!(page.next_after.is_none());
    assert_totals(&page, 1, 60, 60, 60);
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn detailed_report_keeps_archived_history_and_report_rounding(pool: PgPool) {
    let ids = fixture(&pool).await;
    let target = person(&pool, ids.org_id, false, BuiltInProfile::Member).await;
    let entry = time_entry(
        &pool,
        &SeedIds {
            user_id: target,
            ..ids
        },
        EntryState::Open,
    )
    .await;
    sqlx::query!("UPDATE time_entries SET minutes=17, rounded_minutes=0, notes='Retained history' WHERE id=$1", entry).execute(&pool).await.unwrap();
    sqlx::query!("UPDATE users SET active=false, billable_rate_cents=123456, cost_rate_cents=987654 WHERE id=$1", target).execute(&pool).await.unwrap();
    sqlx::query!(
        "UPDATE projects SET active=false, project_type='non_billable' WHERE id=$1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE organizations SET round_minutes=15, round_dir='up' WHERE id=$1",
        ids.org_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let page = read(&pool, ids.org_id, ids.user_id, &query())
        .await
        .unwrap();
    assert_eq!(page.entries.len(), 1);
    assert_eq!(page.entries[0].id, entry);
    assert_eq!(page.entries[0].minutes, 17);
    assert_eq!(page.entries[0].rounded_minutes, 0);
    assert!(!page.entries[0].billable);
    assert_totals(&page, 1, 17, 0, 0);
    let value = serde_json::to_value(&page.entries[0]).unwrap();
    assert_eq!(value.as_object().unwrap().len(), 9);
    for forbidden in [
        "billable_rate_cents",
        "cost_rate_cents",
        "invoice_id",
        "currency",
        "email",
    ] {
        assert!(value.get(forbidden).is_none());
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn detailed_report_profiles_and_custom_scope_union_do_not_duplicate_rows(pool: PgPool) {
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
    let mut all_rows = Vec::new();
    for user_id in [ids.user_id, target, outsider] {
        for project_id in [ids.project_id, other] {
            all_rows.push(
                time_entry(
                    &pool,
                    &SeedIds {
                        user_id,
                        project_id,
                        ..ids
                    },
                    EntryState::Open,
                )
                .await,
            );
        }
    }
    sqlx::query!("INSERT INTO person_management_assignments (id,org_id,manager_id,managed_user_id) VALUES ($1,$2,$3,$4)", Uuid::now_v7(), ids.org_id, ids.user_id, target).execute(&pool).await.unwrap();
    sqlx::query!("INSERT INTO project_management_assignments (id,org_id,manager_id,project_id) VALUES ($1,$2,$3,$4)", Uuid::now_v7(), ids.org_id, ids.user_id, ids.project_id).execute(&pool).await.unwrap();
    let selections = BuiltInProfile::ALL
        .iter()
        .map(|profile| profile.selection())
        .chain([
            PermissionSelection::new(&[Permission::TimeReadManaged]),
            PermissionSelection::new(&[
                Permission::PeopleReadAll,
                Permission::ReportProfitabilityRead,
            ]),
        ]);
    for selection in selections {
        let count = if selection.contains(Permission::TimeReadAll) {
            6
        } else if selection.contains(Permission::TimeReadManaged) {
            5
        } else {
            2
        };
        grants(&pool, ids.user_id, selection).await;
        let page = read(&pool, ids.org_id, ids.user_id, &query())
            .await
            .unwrap();
        let actual: std::collections::BTreeSet<_> =
            page.entries.iter().map(|entry| entry.id).collect();
        assert_eq!(page.entries.len(), count);
        assert_eq!(actual, all_rows[..count].iter().copied().collect());
        assert_totals(
            &page,
            count as i64,
            count as i64 * 60,
            count as i64 * 60,
            count as i64 * 60,
        );
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn detailed_report_filters_are_any_within_and_between_dimensions(pool: PgPool) {
    let ids = fixture(&pool).await;
    let foreign = seed(&pool, OrgRole::Admin).await;
    let entry = time_entry(&pool, &ids, EntryState::Open).await;
    time_entry(&pool, &foreign, EntryState::Open).await;
    let tags = [Uuid::now_v7(), Uuid::now_v7()];
    for tag in tags {
        sqlx::query!(
            "INSERT INTO project_tags (id,org_id,name) VALUES ($1,$2,$3)",
            tag,
            ids.org_id,
            tag.to_string()
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query!(
            "INSERT INTO project_tag_links (id,org_id,project_id,tag_id) VALUES ($1,$2,$3,$4)",
            Uuid::now_v7(),
            ids.org_id,
            ids.project_id,
            tag
        )
        .execute(&pool)
        .await
        .unwrap();
    }
    let mut filter = query();
    filter.client_ids = vec![ids.client_id, foreign.client_id];
    filter.project_ids = vec![ids.project_id, ids.project_id];
    filter.user_ids = vec![ids.user_id, foreign.user_id];
    filter.task_ids = vec![ids.task_id];
    filter.tag_ids = vec![tags[0], tags[0], tags[1], Uuid::now_v7()];
    let page = read(&pool, ids.org_id, ids.user_id, &filter).await.unwrap();
    assert_eq!(
        page.entries.iter().map(|row| row.id).collect::<Vec<_>>(),
        [entry]
    );
    filter.task_ids = vec![foreign.task_id];
    assert_totals(&page, 1, 60, 60, 60);
    let empty = read(&pool, ids.org_id, ids.user_id, &filter).await.unwrap();
    assert!(empty.entries.is_empty());
    assert_totals(&empty, 0, 0, 0, 0);
    filter.task_ids.clear();
    filter.date_from = "2026-09-08".parse().unwrap();
    let empty = read(&pool, ids.org_id, ids.user_id, &filter).await.unwrap();
    assert!(empty.entries.is_empty());
    assert_totals(&empty, 0, 0, 0, 0);
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn detailed_report_keyset_exhausts_ties_without_repeating_rows(pool: PgPool) {
    let ids = fixture(&pool).await;
    let mut expected = Vec::new();
    for _ in 0..503 {
        expected.push(time_entry(&pool, &ids, EntryState::Open).await);
    }
    expected.sort_unstable();
    let mut filter = query();
    let first = read(&pool, ids.org_id, ids.user_id, &filter).await.unwrap();
    let totals = serde_json::json!({
        "entry_count":503,"total_minutes":30180,"rounded_minutes":30180,
        "billable_minutes":30180
    });
    assert_eq!(serde_json::to_value(&first).unwrap()["totals"], totals);
    assert_eq!(
        first
            .entries
            .iter()
            .map(|entry| entry.id)
            .collect::<Vec<_>>(),
        expected[..500]
    );
    filter.after = first.next_after;
    let last = read(&pool, ids.org_id, ids.user_id, &filter).await.unwrap();
    assert_eq!(serde_json::to_value(&last).unwrap()["totals"], totals);
    assert_eq!(
        last.entries
            .iter()
            .map(|entry| entry.id)
            .collect::<Vec<_>>(),
        expected[500..]
    );
    assert!(last.next_after.is_none());
    let final_entry = last.entries.last().unwrap();
    let mut exhausted = filter.clone();
    exhausted.after = Some(TimeReportCursor {
        spent_date: final_entry.spent_date,
        project_name: final_entry.project_name.clone(),
        task_name: final_entry.task_name.clone(),
        id: final_entry.id,
    });
    let empty = read(&pool, ids.org_id, ids.user_id, &exhausted)
        .await
        .unwrap();
    assert!(empty.entries.is_empty());
    assert!(empty.next_after.is_none());
    assert_eq!(serde_json::to_value(&empty).unwrap()["totals"], totals);
    filter.after.as_mut().unwrap().id = Uuid::nil();
    assert_eq!(
        read(&pool, ids.org_id, ids.user_id, &filter)
            .await
            .unwrap()
            .entries
            .len(),
        500
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn detailed_report_rejects_invalid_queries_requesters_and_policy_without_fallback(
    pool: PgPool,
) {
    let ids = fixture(&pool).await;
    let mut filter = query();
    filter.date_to = "2026-08-31".parse().unwrap();
    assert!(matches!(
        read(&pool, ids.org_id, ids.user_id, &filter).await,
        Err(TimeReadError::InvalidQuery)
    ));
    filter = query();
    filter.after = Some(TimeReportCursor {
        spent_date: filter.date_from,
        project_name: "bad\0name".into(),
        task_name: "Dev".into(),
        id: Uuid::now_v7(),
    });
    assert!(matches!(
        read(&pool, ids.org_id, ids.user_id, &filter).await,
        Err(TimeReadError::InvalidQuery)
    ));
    filter.after.as_mut().unwrap().project_name.clear();
    filter.after.as_mut().unwrap().spent_date = "2026-10-01".parse().unwrap();
    assert!(matches!(
        read(&pool, ids.org_id, ids.user_id, &filter).await,
        Err(TimeReadError::InvalidQuery)
    ));
    filter = query();
    filter.expected_requester = Some(crate::models::permission_editor::PermissionRequester {
        org_id: ids.org_id,
        user_id: Uuid::now_v7(),
    });
    assert!(matches!(
        read(&pool, ids.org_id, ids.user_id, &filter).await,
        Err(TimeReadError::Forbidden)
    ));
    for policy in [0, 2] {
        sqlx::query!(
            "UPDATE organizations SET permission_policy_version=$2 WHERE id=$1",
            ids.org_id,
            policy
        )
        .execute(&pool)
        .await
        .unwrap();
        let result = read(&pool, ids.org_id, ids.user_id, &query()).await;
        assert!(matches!(
            result,
            Err(TimeReadError::Forbidden | TimeReadError::Unavailable)
        ));
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn detailed_report_observes_revocation_after_wait_and_releases_on_cancel(pool: PgPool) {
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
        let reader = tokio::spawn(async move { read(&db, org, actor, &query()).await });
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
            assert!(page.entries.is_empty());
            assert_totals(&page, 0, 0, 0, 0);
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
async fn detailed_report_excludes_malformed_tenant_parents(pool: PgPool) {
    let ids = fixture(&pool).await;
    let foreign = seed(&pool, OrgRole::Admin).await;
    let valid = time_entry(&pool, &ids, EntryState::Open).await;
    for malformed in [
        SeedIds {
            user_id: foreign.user_id,
            ..ids
        },
        SeedIds {
            project_id: foreign.project_id,
            ..ids
        },
        SeedIds {
            task_id: foreign.task_id,
            ..ids
        },
    ] {
        time_entry(&pool, &malformed, EntryState::Open).await;
    }
    let page = read(&pool, ids.org_id, ids.user_id, &query())
        .await
        .unwrap();
    assert_eq!(
        page.entries.iter().map(|row| row.id).collect::<Vec<_>>(),
        [valid]
    );
    sqlx::query!(
        "UPDATE projects SET client_id=$2 WHERE id=$1",
        ids.project_id,
        foreign.client_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert_totals(&page, 1, 60, 60, 60);
    let empty = read(&pool, ids.org_id, ids.user_id, &query())
        .await
        .unwrap();
    assert!(empty.entries.is_empty());
    assert_totals(&empty, 0, 0, 0, 0);
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn detailed_report_preserves_billed_status_and_frozen_minutes_after_configuration_changes(
    pool: PgPool,
) {
    let ids = fixture(&pool).await;
    let billed = time_entry(&pool, &ids, EntryState::Invoiced).await;
    let unbillable = time_entry(&pool, &ids, EntryState::Invoiced).await;
    let open = time_entry(&pool, &ids, EntryState::Open).await;
    let invoice = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO invoices (id,org_id,client_id,number,issued_on,due_on,currency,total_cents)
        VALUES ($1,$2,$3,'report-private','2026-09-07','2026-10-07','EUR',42000)",
        invoice,
        ids.org_id,
        ids.client_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE time_entries SET invoice_id=$2,minutes=17,rounded_minutes=0 WHERE id=ANY($1)",
        &[billed, unbillable],
        invoice
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE time_entries SET billable=false WHERE id=$1",
        unbillable
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE time_entries SET minutes=17,rounded_minutes=NULL WHERE id=$1",
        open
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE projects SET project_type='non_billable' WHERE id=$1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE tasks SET billable_default=false WHERE id=$1",
        ids.task_id
    )
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
    let page = read(&pool, ids.org_id, ids.user_id, &query())
        .await
        .unwrap();
    assert_totals(&page, 3, 51, 30, 0);
    let rows = page.entries;
    let billed_row = rows.iter().find(|row| row.id == billed).unwrap();
    assert!(billed_row.billable);
    assert_eq!(billed_row.rounded_minutes, 0);
    assert!(
        !rows
            .iter()
            .find(|row| row.id == unbillable)
            .unwrap()
            .billable
    );
    let open_row = rows.iter().find(|row| row.id == open).unwrap();
    assert!(!open_row.billable);
    assert_eq!(open_row.rounded_minutes, 30);
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn detailed_report_totals_round_each_entry_before_summing_billable_time(pool: PgPool) {
    let ids = fixture(&pool).await;
    let live = time_entry(&pool, &ids, EntryState::Open).await;
    let frozen = time_entry(&pool, &ids, EntryState::Open).await;
    let unbillable = time_entry(&pool, &ids, EntryState::Open).await;
    sqlx::query!(
        "UPDATE time_entries SET minutes=17,rounded_minutes=CASE WHEN id=$2 THEN 0 ELSE NULL END,
         billable=(id<>$3) WHERE id=ANY($1)",
        &[live, frozen, unbillable],
        frozen,
        unbillable
    )
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
    let page = read(&pool, ids.org_id, ids.user_id, &query())
        .await
        .unwrap();
    assert_totals(&page, 3, 51, 60, 30);
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn detailed_report_totals_use_bigint_and_count_zero_duration_entries(pool: PgPool) {
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
    let page = read(&pool, ids.org_id, ids.user_id, &query())
        .await
        .unwrap();
    let max = i64::from(i32::MAX);
    assert_totals(&page, 2, max * 2, max * 2, max);

    sqlx::query!(
        "UPDATE time_entries SET minutes=0,rounded_minutes=0 WHERE org_id=$1",
        ids.org_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let page = read(&pool, ids.org_id, ids.user_id, &query())
        .await
        .unwrap();
    assert_eq!(page.entries.len(), 2);
    assert_totals(&page, 2, 0, 0, 0);
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn detailed_report_rechecks_direct_actor_deactivation_after_wait(pool: PgPool) {
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
    let reader = tokio::spawn(async move { read(&db, ids.org_id, ids.user_id, &query()).await });
    wait_for_blocked(&pool, pid).await;
    deactivate.commit().await.unwrap();
    assert!(matches!(
        reader.await.unwrap(),
        Err(TimeReadError::Forbidden)
    ));
}
