//! `list` and `get`, the entry reads `/api/v1` serves: the Timesheet's scope,
//! every filter, and a walk in `id` order.

use std::collections::BTreeSet;

use super::*;
use crate::ops::time_reads::{EntryFilter, TimeEntryQuery};
use crate::ops::{self, Caller};
use crate::testing::time_entry;
use chrono::{DateTime, Utc};
use horae_core::types::EntryState;

async fn list_as(
    pool: &PgPool,
    org: Uuid,
    actor: Uuid,
    filter: &EntryFilter,
    after: Option<Uuid>,
    limit: i64,
) -> Result<Vec<Uuid>, OpError> {
    let mut read = ops::begin_read(pool, Caller::session(org, actor)).await?;
    let result = list(&mut read.tx, &read.actor, filter, after, limit).await;
    let entries = read.finish(result).await?;
    Ok(entries.into_iter().map(|entry| entry.id).collect())
}

async fn get_as(pool: &PgPool, org: Uuid, actor: Uuid, id: Uuid) -> Result<Uuid, OpError> {
    let mut read = ops::begin_read(pool, Caller::session(org, actor)).await?;
    let result = get(&mut read.tx, &read.actor, id).await;
    Ok(read.finish(result).await?.id)
}

/// What the Timesheet's entry reader shows the actor over the fixture's month.
async fn timesheet_ids(pool: &PgPool, org: Uuid, actor: Uuid) -> BTreeSet<Uuid> {
    let mut read = ops::begin_read(pool, Caller::session(org, actor))
        .await
        .unwrap();
    let query = TimeEntryQuery {
        date_from: "2026-01-01".parse().unwrap(),
        date_to: "2026-12-31".parse().unwrap(),
        user_id: None,
        project_id: None,
        after: None,
    };
    let result = super::entries(&mut read.tx, &read.actor, &query).await;
    let page = read.finish(result).await.unwrap();
    page.entries.into_iter().map(|entry| entry.id).collect()
}

async fn grant(pool: &PgPool, org: Uuid, actor: Uuid, grants: &[Permission]) {
    save_state(pool, org, actor, false, &PermissionSelection::new(grants)).await;
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn each_scope_lists_what_the_timesheet_shows(pool: PgPool) {
    let ids = fixture(&pool).await;
    let managed = person(&pool, ids.org_id, false, BuiltInProfile::Member).await;
    let outsider = person(&pool, ids.org_id, false, BuiltInProfile::Member).await;
    let other_project = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO projects (id,org_id,client_id,name,currency) VALUES ($1,$2,$3,'Other','EUR')",
        other_project,
        ids.org_id,
        ids.client_id
    )
    .execute(&pool)
    .await
    .unwrap();
    for user_id in [ids.user_id, managed, outsider] {
        for project_id in [ids.project_id, other_project] {
            let owner = SeedIds {
                user_id,
                project_id,
                ..ids
            };
            time_entry(&pool, &owner, EntryState::Open).await;
        }
    }
    sqlx::query!("INSERT INTO person_management_assignments (id,org_id,manager_id,managed_user_id) VALUES ($1,$2,$3,$4)", Uuid::now_v7(), ids.org_id, ids.user_id, managed).execute(&pool).await.unwrap();
    sqlx::query!("INSERT INTO project_management_assignments (id,org_id,manager_id,project_id) VALUES ($1,$2,$3,$4)", Uuid::now_v7(), ids.org_id, ids.user_id, ids.project_id).execute(&pool).await.unwrap();

    for (grants, expected) in [
        (vec![Permission::TimeReadOwn], 2),
        (vec![Permission::TimeReadManaged], 5),
        (vec![Permission::TimeReadAll], 6),
    ] {
        grant(&pool, ids.org_id, ids.user_id, &grants).await;
        let listed = list_as(
            &pool,
            ids.org_id,
            ids.user_id,
            &EntryFilter::default(),
            None,
            100,
        )
        .await
        .unwrap();
        assert_eq!(listed.len(), expected, "{grants:?}");
        assert!(listed.is_sorted(), "id order");
        assert_eq!(
            listed.iter().copied().collect::<BTreeSet<_>>(),
            timesheet_ids(&pool, ids.org_id, ids.user_id).await,
            "{grants:?}"
        );
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn another_persons_entry_outside_the_scope_is_not_found(pool: PgPool) {
    let ids = fixture(&pool).await;
    let member = person(&pool, ids.org_id, false, BuiltInProfile::Member).await;
    let admins = time_entry(&pool, &ids, EntryState::Open).await;
    let owner = SeedIds {
        user_id: member,
        ..ids
    };
    let theirs = time_entry(&pool, &owner, EntryState::Open).await;

    assert_eq!(get_as(&pool, ids.org_id, member, theirs).await, Ok(theirs));
    assert_eq!(
        get_as(&pool, ids.org_id, ids.user_id, theirs).await,
        Ok(theirs)
    );
    let hidden = get_as(&pool, ids.org_id, member, admins).await;
    assert!(matches!(hidden, Err(OpError::NotFound(_))), "{hidden:?}");
    let missing = get_as(&pool, ids.org_id, member, Uuid::now_v7()).await;
    assert!(matches!(missing, Err(OpError::NotFound(_))), "{missing:?}");

    let foreign = fixture(&pool).await;
    let elsewhere = get_as(&pool, foreign.org_id, foreign.user_id, theirs).await;
    assert!(
        matches!(elsewhere, Err(OpError::NotFound(_))),
        "{elsewhere:?}"
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn every_filter_narrows_the_list(pool: PgPool) {
    let ids = fixture(&pool).await;
    let other = person(&pool, ids.org_id, false, BuiltInProfile::Member).await;
    let (client, project, task) = (Uuid::now_v7(), Uuid::now_v7(), Uuid::now_v7());
    sqlx::query!(
        "INSERT INTO clients (id,org_id,name,currency) VALUES ($1,$2,'Beta','EUR')",
        client,
        ids.org_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO projects (id,org_id,client_id,name,currency) VALUES ($1,$2,$3,'Beta site','EUR')",
        project,
        ids.org_id,
        client
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO tasks (id,org_id,name) VALUES ($1,$2,'Design')",
        task,
        ids.org_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let mine = time_entry(&pool, &ids, EntryState::Open).await;
    let theirs = time_entry(
        &pool,
        &SeedIds {
            user_id: other,
            ..ids
        },
        EntryState::Open,
    )
    .await;
    let beta = time_entry(
        &pool,
        &SeedIds {
            project_id: project,
            task_id: task,
            ..ids
        },
        EntryState::Open,
    )
    .await;
    sqlx::query!(
        "UPDATE time_entries SET spent_date='2026-09-20' WHERE id=$1",
        beta
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE time_entries SET is_running=true, started_at=now() WHERE id=$1",
        theirs
    )
    .execute(&pool)
    .await
    .unwrap();
    let before_edit: DateTime<Utc> =
        sqlx::query_scalar!(r#"SELECT clock_timestamp() AS "now!: DateTime<Utc>""#)
            .fetch_one(&pool)
            .await
            .unwrap();
    sqlx::query!("UPDATE time_entries SET notes='edited' WHERE id=$1", mine)
        .execute(&pool)
        .await
        .unwrap();

    let cases: [(EntryFilter, Vec<Uuid>); 8] = [
        (
            EntryFilter {
                person_id: Some(other),
                ..Default::default()
            },
            vec![theirs],
        ),
        (
            EntryFilter {
                project_id: Some(project),
                ..Default::default()
            },
            vec![beta],
        ),
        (
            EntryFilter {
                task_id: Some(task),
                ..Default::default()
            },
            vec![beta],
        ),
        (
            EntryFilter {
                client_id: Some(ids.client_id),
                ..Default::default()
            },
            vec![mine, theirs],
        ),
        (
            EntryFilter {
                from: Some("2026-09-10".parse().unwrap()),
                ..Default::default()
            },
            vec![beta],
        ),
        (
            EntryFilter {
                to: Some("2026-09-07".parse().unwrap()),
                ..Default::default()
            },
            vec![mine, theirs],
        ),
        (
            EntryFilter {
                is_running: Some(true),
                ..Default::default()
            },
            vec![theirs],
        ),
        (
            EntryFilter {
                updated_since: Some(before_edit),
                ..Default::default()
            },
            vec![mine],
        ),
    ];
    for (filter, expected) in cases {
        let listed = list_as(&pool, ids.org_id, ids.user_id, &filter, None, 100)
            .await
            .unwrap();
        assert_eq!(listed, expected, "{filter:?}");
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn a_walk_returns_each_entry_once_in_id_order(pool: PgPool) {
    let ids = fixture(&pool).await;
    let mut expected = Vec::new();
    for _ in 0..5 {
        expected.push(time_entry(&pool, &ids, EntryState::Open).await);
    }
    let mut seen = Vec::new();
    let mut after = None;
    loop {
        let page = list_as(
            &pool,
            ids.org_id,
            ids.user_id,
            &EntryFilter::default(),
            after,
            2,
        )
        .await
        .unwrap();
        let Some(last) = page.last().copied() else {
            break;
        };
        seen.extend(page);
        after = Some(last);
    }
    assert_eq!(seen, expected);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn reversed_dates_are_invalid(pool: PgPool) {
    let ids = fixture(&pool).await;
    let filter = EntryFilter {
        from: Some("2026-09-08".parse().unwrap()),
        to: Some("2026-09-07".parse().unwrap()),
        ..Default::default()
    };
    let error = list_as(&pool, ids.org_id, ids.user_id, &filter, None, 100).await;
    assert_eq!(
        error,
        Err(OpError::invalid(
            "from",
            "The start date is after the end date"
        ))
    );
}

/// 100,000 entries walked in 1,000-row pages: each row once, each page
/// under a second (plan *Performance Goals*, SC-007).
#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
#[ignore = "100,000-entry walk; run explicitly with --release --nocapture"]
async fn a_walk_of_100_000_entries_reads_each_page_under_a_second(pool: PgPool) {
    let ids = fixture(&pool).await;
    sqlx::query!(
        "INSERT INTO time_entries (id,org_id,user_id,project_id,task_id,spent_date,minutes,billable,state)
         SELECT gen_random_uuid(), $1, $2, $3, $4, DATE '2020-01-01' + (n % 2000), 1, true, 'open'
         FROM generate_series(1, 100000) n",
        ids.org_id,
        ids.user_id,
        ids.project_id,
        ids.task_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!("ANALYZE time_entries")
        .execute(&pool)
        .await
        .unwrap();
    let (mut seen, mut after, mut slowest) = (BTreeSet::new(), None, std::time::Duration::ZERO);
    loop {
        let started = std::time::Instant::now();
        let page = list_as(
            &pool,
            ids.org_id,
            ids.user_id,
            &EntryFilter::default(),
            after,
            1001,
        )
        .await
        .unwrap();
        slowest = slowest.max(started.elapsed());
        let page: Vec<_> = page.into_iter().take(1000).collect();
        let Some(last) = page.last().copied() else {
            break;
        };
        for id in page {
            assert!(seen.insert(id), "{id} returned twice");
        }
        after = Some(last);
    }
    println!("100,000 entries; slowest 1,000-row page {slowest:?}");
    assert_eq!(seen.len(), 100_000);
    assert!(slowest < std::time::Duration::from_secs(1), "{slowest:?}");
}
