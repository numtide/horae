use super::*;
use crate::models::scoped_time::{TimesheetPeoplePage, TimesheetPeopleQuery};
use crate::server_fns::permissions::time_entries::{TimeReadError, people};
use crate::server_fns::test_seed::time_entry;
use horae_core::types::EntryState;
use std::collections::BTreeSet;

fn ids(page: &TimesheetPeoplePage) -> BTreeSet<Uuid> {
    page.people.iter().map(|person| person.id).collect()
}

async fn manage_project(pool: &PgPool, seed: &SeedIds) {
    sqlx::query!("INSERT INTO project_management_assignments (id,org_id,manager_id,project_id) VALUES ($1,$2,$3,$4) ON CONFLICT DO NOTHING",
        Uuid::now_v7(), seed.org_id, seed.user_id, seed.project_id).execute(pool).await.unwrap();
}

async fn assign(pool: &PgPool, project: Uuid, user: Uuid) {
    sqlx::query!(
        "INSERT INTO assignments (id,user_id,project_id) VALUES ($1,$2,$3)",
        Uuid::now_v7(),
        user,
        project
    )
    .execute(pool)
    .await
    .unwrap();
}

async fn scope(pool: &PgPool, actor: Uuid, selection: PermissionSelection) {
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
async fn candidates_include_empty_project_members_and_retained_history_without_directory_access(
    pool: PgPool,
) {
    let seed = fixture(&pool).await;
    let participant = person(&pool, seed.org_id, false, BuiltInProfile::Member).await;
    let historical = person(&pool, seed.org_id, false, BuiltInProfile::Member).await;
    let direct = person(&pool, seed.org_id, false, BuiltInProfile::Member).await;
    let outsider = person(&pool, seed.org_id, false, BuiltInProfile::Member).await;
    let history = SeedIds {
        user_id: historical,
        ..seed
    };
    time_entry(&pool, &history, EntryState::Open).await;
    sqlx::query!(
        "INSERT INTO assignments (id,user_id,project_id) VALUES ($1,$2,$3)",
        Uuid::now_v7(),
        participant,
        seed.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!("INSERT INTO project_management_assignments (id,org_id,manager_id,project_id) VALUES ($1,$2,$3,$4)",
        Uuid::now_v7(), seed.org_id, seed.user_id, seed.project_id).execute(&pool).await.unwrap();
    for target in [direct, participant] {
        sqlx::query!("INSERT INTO person_management_assignments (id,org_id,manager_id,managed_user_id) VALUES ($1,$2,$3,$4)",
            Uuid::now_v7(), seed.org_id, seed.user_id, target).execute(&pool).await.unwrap();
    }
    scope(
        &pool,
        seed.user_id,
        PermissionSelection::new(&[Permission::TimeReadManaged]),
    )
    .await;
    let result = people(
        &pool,
        seed.org_id,
        seed.user_id,
        &TimesheetPeopleQuery::default(),
    )
    .await
    .unwrap();
    assert_eq!(
        ids(&result),
        [seed.user_id, participant, historical, direct].into()
    );
    assert_eq!(result.people.len(), 4);
    assert!(!ids(&result).contains(&outsider));
    assert_eq!(result.requester.org_id, seed.org_id);
    assert_eq!(result.requester.user_id, seed.user_id);

    sqlx::query!(
        "DELETE FROM person_management_assignments WHERE manager_id=$1",
        seed.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let result = people(
        &pool,
        seed.org_id,
        seed.user_id,
        &TimesheetPeopleQuery::default(),
    )
    .await
    .unwrap();
    assert_eq!(ids(&result), [seed.user_id, participant, historical].into());
    sqlx::query!("DELETE FROM assignments WHERE user_id=$1", participant)
        .execute(&pool)
        .await
        .unwrap();
    let result = people(
        &pool,
        seed.org_id,
        seed.user_id,
        &TimesheetPeopleQuery::default(),
    )
    .await
    .unwrap();
    assert_eq!(ids(&result), [seed.user_id, historical].into());
    sqlx::query!(
        "DELETE FROM project_management_assignments WHERE manager_id=$1",
        seed.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(
        ids(&people(
            &pool,
            seed.org_id,
            seed.user_id,
            &TimesheetPeopleQuery::default()
        )
        .await
        .unwrap()),
        [seed.user_id].into()
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn profiles_and_custom_grants_use_time_scope_not_legacy_role_or_directory(pool: PgPool) {
    let seed = fixture(&pool).await;
    let target = person(&pool, seed.org_id, false, BuiltInProfile::Member).await;
    let outside = person(&pool, seed.org_id, false, BuiltInProfile::Member).await;
    let foreign = fixture(&pool).await;
    manage_project(&pool, &seed).await;
    assign(&pool, seed.project_id, target).await;
    for profile in BuiltInProfile::ALL {
        scope(&pool, seed.user_id, profile.selection()).await;
        let expected: BTreeSet<_> = match profile {
            BuiltInProfile::Member => [seed.user_id].into(),
            BuiltInProfile::ProjectManager => [seed.user_id, target].into(),
            _ => [seed.user_id, target, outside].into(),
        };
        let page = people(
            &pool,
            seed.org_id,
            seed.user_id,
            &TimesheetPeopleQuery::default(),
        )
        .await
        .unwrap();
        assert_eq!(ids(&page), expected, "{profile:?}");
        assert!(!ids(&page).contains(&foreign.user_id));
    }
    scope(
        &pool,
        seed.user_id,
        PermissionSelection::new(&[Permission::PeopleReadAll, Permission::ProjectReadAll]),
    )
    .await;
    sqlx::query!(
        "UPDATE users SET org_role='admin' WHERE id=$1",
        seed.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE person_permission_states SET is_administrator=true WHERE user_id=$1",
        seed.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let page = people(
        &pool,
        seed.org_id,
        seed.user_id,
        &TimesheetPeopleQuery::default(),
    )
    .await
    .unwrap();
    assert_eq!(ids(&page), [seed.user_id].into());
    for user_id in [target, outside, foreign.user_id, Uuid::now_v7()] {
        assert!(
            people(
                &pool,
                seed.org_id,
                seed.user_id,
                &TimesheetPeopleQuery {
                    user_id: Some(user_id),
                    ..Default::default()
                }
            )
            .await
            .unwrap()
            .people
            .is_empty()
        );
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn history_keeps_archived_context_but_excludes_inactive_people_and_foreign_parents(
    pool: PgPool,
) {
    let seed = fixture(&pool).await;
    let foreign = fixture(&pool).await;
    let target = person(&pool, seed.org_id, false, BuiltInProfile::Member).await;
    let malformed = person(&pool, seed.org_id, false, BuiltInProfile::Member).await;
    manage_project(&pool, &seed).await;
    scope(
        &pool,
        seed.user_id,
        PermissionSelection::new(&[Permission::TimeReadManaged]),
    )
    .await;
    time_entry(
        &pool,
        &SeedIds {
            user_id: target,
            ..seed
        },
        EntryState::Open,
    )
    .await;
    let bad = time_entry(
        &pool,
        &SeedIds {
            user_id: malformed,
            ..seed
        },
        EntryState::Open,
    )
    .await;
    sqlx::query!(
        "UPDATE time_entries SET task_id=$2 WHERE id=$1",
        bad,
        foreign.task_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assign(&pool, seed.project_id, foreign.user_id).await;
    sqlx::query!(
        "UPDATE projects SET active=false WHERE id=$1",
        seed.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!("UPDATE tasks SET active=false WHERE id=$1", seed.task_id)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        ids(&people(
            &pool,
            seed.org_id,
            seed.user_id,
            &TimesheetPeopleQuery::default()
        )
        .await
        .unwrap()),
        [seed.user_id, target].into()
    );
    sqlx::query!(
        "UPDATE time_entries SET task_id=$2, org_id=$3 WHERE id=$1",
        bad,
        seed.task_id,
        foreign.org_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(
        ids(&people(
            &pool,
            seed.org_id,
            seed.user_id,
            &TimesheetPeopleQuery::default()
        )
        .await
        .unwrap()),
        [seed.user_id, target].into()
    );
    sqlx::query!("UPDATE users SET active=false WHERE id=$1", target)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        ids(&people(
            &pool,
            seed.org_id,
            seed.user_id,
            &TimesheetPeopleQuery::default()
        )
        .await
        .unwrap()),
        [seed.user_id].into()
    );
    sqlx::query!("UPDATE users SET active=true WHERE id=$1", target)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query!(
        "UPDATE projects SET client_id=$2 WHERE id=$1",
        seed.project_id,
        foreign.client_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(
        ids(&people(
            &pool,
            seed.org_id,
            seed.user_id,
            &TimesheetPeopleQuery::default()
        )
        .await
        .unwrap()),
        [seed.user_id].into()
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn paging_search_and_identity_filters_only_narrow_current_scope(pool: PgPool) {
    let seed = fixture(&pool).await;
    let mut expected = Vec::new();
    for _ in 0..51 {
        expected.push(person(&pool, seed.org_id, false, BuiltInProfile::Member).await);
    }
    expected.sort_unstable();
    let mut query = TimesheetPeopleQuery {
        search: "  pErSoN  ".into(),
        ..Default::default()
    };
    let first = people(&pool, seed.org_id, seed.user_id, &query)
        .await
        .unwrap();
    assert_eq!(
        first.people.iter().map(|p| p.id).collect::<Vec<_>>(),
        expected[..50]
    );
    let cursor = first.next_after.unwrap();
    sqlx::query!("UPDATE users SET active=false WHERE id=$1", cursor.id)
        .execute(&pool)
        .await
        .unwrap();
    query.after = Some(cursor);
    let second = people(&pool, seed.org_id, seed.user_id, &query)
        .await
        .unwrap();
    assert_eq!(
        second.people.iter().map(|p| p.id).collect::<Vec<_>>(),
        expected[50..]
    );
    assert!(second.next_after.is_none());
    query.after = None;
    query.user_id = expected.last().copied();
    assert_eq!(
        people(&pool, seed.org_id, seed.user_id, &query)
            .await
            .unwrap()
            .people
            .len(),
        1
    );
    query.user_id = None;
    query.search = "%".into();
    assert!(
        people(&pool, seed.org_id, seed.user_id, &query)
            .await
            .unwrap()
            .people
            .is_empty()
    );
    for search in ["\0".into(), "x".repeat(101)] {
        query.search = search;
        assert!(matches!(
            people(&pool, seed.org_id, seed.user_id, &query).await,
            Err(TimeReadError::InvalidQuery)
        ));
    }
    query.search.clear();
    query.after = Some(crate::models::people::PeopleCursor {
        name: "\0".into(),
        id: seed.user_id,
    });
    assert!(matches!(
        people(&pool, seed.org_id, seed.user_id, &query).await,
        Err(TimeReadError::InvalidQuery)
    ));
    query.after = second
        .people
        .last()
        .map(|person| crate::models::people::PeopleCursor {
            name: person.name.clone(),
            id: person.id,
        });
    scope(&pool, seed.user_id, BuiltInProfile::Member.selection()).await;
    assert_eq!(
        ids(&people(&pool, seed.org_id, seed.user_id, &query)
            .await
            .unwrap()),
        [seed.user_id].into()
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn discovery_rechecks_revocation_and_actor_activity_after_waiting(pool: PgPool) {
    let seed = fixture(&pool).await;
    let target = person(&pool, seed.org_id, false, BuiltInProfile::Member).await;
    for mode in 0..4 {
        scope(
            &pool,
            seed.user_id,
            PermissionSelection::new(&[Permission::TimeReadManaged]),
        )
        .await;
        manage_project(&pool, &seed).await;
        sqlx::query!("DELETE FROM assignments WHERE user_id=$1", target)
            .execute(&pool)
            .await
            .unwrap();
        assign(&pool, seed.project_id, target).await;
        let mut revoke = pool.begin().await.unwrap();
        if mode == 3 {
            sqlx::query!("UPDATE users SET active=false WHERE id=$1", seed.user_id)
                .execute(&mut *revoke)
                .await
                .unwrap();
        } else {
            sqlx::query!(
                "SELECT id FROM organizations WHERE id=$1 FOR UPDATE",
                seed.org_id
            )
            .fetch_one(&mut *revoke)
            .await
            .unwrap();
            match mode {
                0 => {
                    let values: Vec<String> = serde_json::from_value(
                        serde_json::to_value(BuiltInProfile::Member.selection()).unwrap(),
                    )
                    .unwrap();
                    sqlx::query!(
                        "UPDATE person_permission_states SET grants=$2 WHERE user_id=$1",
                        seed.user_id,
                        &values
                    )
                    .execute(&mut *revoke)
                    .await
                    .unwrap();
                }
                1 => {
                    sqlx::query!(
                        "DELETE FROM project_management_assignments WHERE manager_id=$1",
                        seed.user_id
                    )
                    .execute(&mut *revoke)
                    .await
                    .unwrap();
                }
                _ => {
                    sqlx::query!("DELETE FROM assignments WHERE user_id=$1", target)
                        .execute(&mut *revoke)
                        .await
                        .unwrap();
                }
            }
        }
        let pid = sqlx::query_scalar!("SELECT pg_backend_pid()")
            .fetch_one(&mut *revoke)
            .await
            .unwrap()
            .unwrap();
        let db = pool.clone();
        let pending = tokio::spawn(async move {
            people(
                &db,
                seed.org_id,
                seed.user_id,
                &TimesheetPeopleQuery::default(),
            )
            .await
        });
        wait_for_blocked(&pool, pid).await;
        revoke.commit().await.unwrap();
        let result = pending.await.unwrap();
        if mode == 3 {
            assert!(matches!(result, Err(TimeReadError::Forbidden)));
        } else {
            assert_eq!(ids(&result.unwrap()), [seed.user_id].into());
        }
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn admitted_discovery_finishes_before_competing_revocation(pool: PgPool) {
    let seed = fixture(&pool).await;
    let target = person(&pool, seed.org_id, false, BuiltInProfile::Member).await;
    manage_project(&pool, &seed).await;
    assign(&pool, seed.project_id, target).await;
    scope(
        &pool,
        seed.user_id,
        PermissionSelection::new(&[Permission::TimeReadManaged]),
    )
    .await;
    let mut assignments = pool.begin().await.unwrap();
    sqlx::query!("LOCK TABLE assignments IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *assignments)
        .await
        .unwrap();
    let blocker = sqlx::query_scalar!("SELECT pg_backend_pid()")
        .fetch_one(&mut *assignments)
        .await
        .unwrap()
        .unwrap();
    let db = pool.clone();
    let pending = tokio::spawn(async move {
        people(
            &db,
            seed.org_id,
            seed.user_id,
            &TimesheetPeopleQuery::default(),
        )
        .await
    });
    wait_for_blocked(&pool, blocker).await;
    let reader_pid = sqlx::query_scalar!("SELECT pid FROM pg_stat_activity WHERE datname=current_database() AND $1=ANY(pg_blocking_pids(pid))", blocker).fetch_one(&pool).await.unwrap().unwrap();
    let db = pool.clone();
    let writer = tokio::spawn(async move {
        let mut tx = db.begin().await.unwrap();
        sqlx::query!(
            "SELECT id FROM organizations WHERE id=$1 FOR UPDATE",
            seed.org_id
        )
        .fetch_one(&mut *tx)
        .await
        .unwrap();
        sqlx::query!("DELETE FROM assignments WHERE user_id=$1", target)
            .execute(&mut *tx)
            .await
            .unwrap();
        tx.commit().await.unwrap();
    });
    wait_for_blocked(&pool, reader_pid).await;
    assignments.rollback().await.unwrap();
    assert_eq!(
        ids(&pending.await.unwrap().unwrap()),
        [seed.user_id, target].into()
    );
    writer.await.unwrap();
    assert_eq!(
        ids(&people(
            &pool,
            seed.org_id,
            seed.user_id,
            &TimesheetPeopleQuery::default()
        )
        .await
        .unwrap()),
        [seed.user_id].into()
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn cancellation_releases_discovery_locks_and_keeps_pool_defaults(pool: PgPool) {
    let seed = fixture(&pool).await;
    let reader = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .after_connect(|connection, _| {
            Box::pin(async move {
                sqlx::query!("SET default_transaction_read_only = on")
                    .execute(&mut *connection)
                    .await?;
                sqlx::query!("SET default_transaction_isolation = 'repeatable read'")
                    .execute(connection)
                    .await?;
                Ok(())
            })
        })
        .connect_with((*pool.connect_options()).clone())
        .await
        .unwrap();
    let mut writer = pool.begin().await.unwrap();
    sqlx::query!(
        "SELECT id FROM organizations WHERE id=$1 FOR UPDATE",
        seed.org_id
    )
    .fetch_one(&mut *writer)
    .await
    .unwrap();
    let blocker = sqlx::query_scalar!("SELECT pg_backend_pid()")
        .fetch_one(&mut *writer)
        .await
        .unwrap()
        .unwrap();
    let db = reader.clone();
    let pending = tokio::spawn(async move {
        people(
            &db,
            seed.org_id,
            seed.user_id,
            &TimesheetPeopleQuery::default(),
        )
        .await
    });
    wait_for_blocked(&pool, blocker).await;
    pending.abort();
    assert!(pending.await.unwrap_err().is_cancelled());
    writer.rollback().await.unwrap();
    let page = tokio::time::timeout(
        Duration::from_secs(6),
        people(
            &reader,
            seed.org_id,
            seed.user_id,
            &TimesheetPeopleQuery::default(),
        ),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(ids(&page), [seed.user_id].into());
    let defaults = sqlx::query!("SELECT current_setting('default_transaction_isolation') AS isolation, current_setting('default_transaction_read_only') AS readonly").fetch_one(&reader).await.unwrap();
    assert_eq!(defaults.isolation.as_deref(), Some("repeatable read"));
    assert_eq!(defaults.readonly.as_deref(), Some("on"));
    reader.close().await;
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn discovery_fails_closed_for_policy_identity_and_invalid_state(pool: PgPool) {
    let seed = fixture(&pool).await;
    let foreign = fixture(&pool).await;
    let query = TimesheetPeopleQuery::default();
    assert!(matches!(
        people(&pool, seed.org_id, foreign.user_id, &query).await,
        Err(TimeReadError::Forbidden)
    ));
    for version in [0, 2] {
        sqlx::query!(
            "UPDATE organizations SET permission_policy_version=$2 WHERE id=$1",
            seed.org_id,
            version
        )
        .execute(&pool)
        .await
        .unwrap();
        let result = people(&pool, seed.org_id, seed.user_id, &query).await;
        assert!(if version == 0 {
            matches!(result, Err(TimeReadError::Forbidden))
        } else {
            matches!(result, Err(TimeReadError::Unavailable))
        });
    }
    sqlx::query!(
        "UPDATE organizations SET permission_policy_version=1 WHERE id=$1",
        seed.org_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE person_permission_states SET grants=ARRAY['invalid_grant'] WHERE user_id=$1",
        seed.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(matches!(
        people(&pool, seed.org_id, seed.user_id, &query).await,
        Err(TimeReadError::Storage(_))
    ));
    sqlx::query!(
        "DELETE FROM person_permission_states WHERE user_id=$1",
        seed.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(matches!(
        people(&pool, seed.org_id, seed.user_id, &query).await,
        Err(TimeReadError::Unavailable)
    ));
}
