use super::*;
use crate::models::people::PeopleCursor;
use crate::models::project_people::{ProjectPeopleContext as Context, ProjectPeopleQuery as Query};
use crate::server_fns::permissions::project_people::{ProjectPeopleError, read};

async fn grants(pool: &PgPool, actor: Uuid, permissions: &[Permission]) {
    let values: Vec<String> = serde_json::from_value(
        serde_json::to_value(PermissionSelection::new(permissions)).unwrap(),
    )
    .unwrap();
    sqlx::query!(
        "UPDATE person_permission_states SET grants=$2, is_administrator=false WHERE user_id=$1",
        actor,
        &values
    )
    .execute(pool)
    .await
    .unwrap();
}

fn search(query: &str) -> Query {
    Query::Search {
        query: query.into(),
        after: None,
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn project_people_authorizes_the_operation_not_the_directory_or_legacy_role(pool: PgPool) {
    let ids = fixture(&pool).await;
    let foreign = fixture(&pool).await;
    let target = person(&pool, ids.org_id, false, BuiltInProfile::Member).await;
    let create = Context::Create;
    let edit = Context::Edit {
        project_id: ids.project_id,
    };
    sqlx::query!("UPDATE users SET org_role='admin' WHERE id=$1", ids.user_id)
        .execute(&pool)
        .await
        .unwrap();
    for permission in [
        Permission::ProjectReadAll,
        Permission::PeopleWriteAll,
        Permission::BillableRateReadAll,
    ] {
        grants(&pool, ids.user_id, &[permission]).await;
        for context in [create, edit] {
            assert!(matches!(
                read(&pool, ids.org_id, ids.user_id, context, &search("")).await,
                Err(ProjectPeopleError::Forbidden)
            ));
        }
    }
    grants(&pool, ids.user_id, &[Permission::ProjectCreateAll]).await;
    let result = read(&pool, ids.org_id, ids.user_id, create, &search(""))
        .await
        .unwrap();
    assert_eq!(result.requester.user_id, ids.user_id);
    assert_eq!(result.requester.org_id, ids.org_id);
    assert_eq!(result.people.len(), 2);
    assert!(result.people.iter().any(|p| p.id == target));
    assert!(matches!(
        read(&pool, ids.org_id, ids.user_id, edit, &search("")).await,
        Err(ProjectPeopleError::Forbidden)
    ));
    sqlx::query!(
        "UPDATE users SET org_role='member' WHERE id=$1",
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    grants(&pool, ids.user_id, &[Permission::ProjectWriteManaged]).await;
    assert!(matches!(
        read(&pool, ids.org_id, ids.user_id, edit, &search("")).await,
        Err(ProjectPeopleError::Forbidden)
    ));
    sqlx::query!("INSERT INTO project_management_assignments (id,org_id,manager_id,project_id) VALUES ($1,$2,$3,$4)", Uuid::now_v7(), ids.org_id, ids.user_id, ids.project_id).execute(&pool).await.unwrap();
    assert_eq!(
        read(&pool, ids.org_id, ids.user_id, edit, &search(""))
            .await
            .unwrap()
            .people
            .len(),
        2
    );
    for context in [
        create,
        Context::Edit {
            project_id: foreign.project_id,
        },
        Context::Edit {
            project_id: Uuid::now_v7(),
        },
    ] {
        assert!(matches!(
            read(&pool, ids.org_id, ids.user_id, context, &search("")).await,
            Err(ProjectPeopleError::Forbidden)
        ));
    }
    grants(&pool, ids.user_id, &[Permission::ProjectWriteAll]).await;
    sqlx::query!(
        "DELETE FROM project_management_assignments WHERE manager_id=$1",
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE projects SET active=false WHERE id=$1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(
        read(&pool, ids.org_id, ids.user_id, edit, &search(""))
            .await
            .unwrap()
            .people
            .len(),
        2
    );
    assert!(matches!(
        read(
            &pool,
            ids.org_id,
            ids.user_id,
            Context::Edit {
                project_id: foreign.project_id
            },
            &search("")
        )
        .await,
        Err(ProjectPeopleError::Forbidden)
    ));
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn project_people_filters_before_paging_and_resolves_outside_the_search(pool: PgPool) {
    let ids = fixture(&pool).await;
    let foreign = fixture(&pool).await;
    grants(&pool, ids.user_id, &[Permission::ProjectCreateAll]).await;
    let mut expected = Vec::new();
    for _ in 0..51 {
        let id = person(&pool, ids.org_id, false, BuiltInProfile::Member).await;
        sqlx::query!("UPDATE users SET name='Équipe %_ 王', oidc_subject=id::text, billable_rate_cents=9000, cost_rate_cents=5000 WHERE id=$1", id).execute(&pool).await.unwrap();
        expected.push(id);
    }
    expected.sort_unstable();
    let archived = person(&pool, ids.org_id, false, BuiltInProfile::Member).await;
    sqlx::query!(
        "UPDATE users SET active=false, name='Équipe %_ 王' WHERE id=$1",
        archived
    )
    .execute(&pool)
    .await
    .unwrap();
    let first = read(
        &pool,
        ids.org_id,
        ids.user_id,
        Context::Create,
        &search("  %_  "),
    )
    .await
    .unwrap();
    assert_eq!(
        first.people.iter().map(|p| p.id).collect::<Vec<_>>(),
        expected[..50]
    );
    assert_eq!(first.next_after.as_ref().unwrap().id, expected[49]);
    let second = read(
        &pool,
        ids.org_id,
        ids.user_id,
        Context::Create,
        &Query::Search {
            query: "%_".into(),
            after: first.next_after,
        },
    )
    .await
    .unwrap();
    assert_eq!(
        second.people.iter().map(|p| p.id).collect::<Vec<_>>(),
        expected[50..]
    );
    assert_eq!(second.next_after, None);
    let resolved = read(
        &pool,
        ids.org_id,
        ids.user_id,
        Context::Create,
        &Query::Resolve {
            ids: vec![
                expected[50],
                ids.user_id,
                expected[50],
                archived,
                foreign.user_id,
                Uuid::now_v7(),
            ],
        },
    )
    .await
    .unwrap();
    assert_eq!(
        resolved
            .people
            .iter()
            .map(|p| p.id)
            .collect::<std::collections::BTreeSet<_>>(),
        [ids.user_id, expected[50]].into_iter().collect()
    );
    assert_eq!(resolved.next_after, None);
    for person in first.people.iter().chain(&resolved.people) {
        let value = serde_json::to_value(person).unwrap();
        let mut keys: Vec<_> = value
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort_unstable();
        assert_eq!(keys, ["id", "name"]);
    }
    let query = Query::Search {
        query: "%_".into(),
        after: Some(PeopleCursor {
            name: "Équipe %_ 王".into(),
            id: foreign.user_id,
        }),
    };
    assert_eq!(
        read(&pool, ids.org_id, ids.user_id, Context::Create, &query)
            .await
            .unwrap()
            .people
            .len(),
        50
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn project_people_validates_bounded_input_only_after_authorization(pool: PgPool) {
    let ids = fixture(&pool).await;
    let invalid = [
        search("\0"),
        search(&"王".repeat(101)),
        Query::Resolve {
            ids: vec![ids.user_id; 501],
        },
        Query::Search {
            query: "".into(),
            after: Some(PeopleCursor {
                name: "bad\0name".into(),
                id: ids.user_id,
            }),
        },
    ];
    for query in &invalid {
        assert!(matches!(
            read(&pool, ids.org_id, ids.user_id, Context::Create, query).await,
            Err(ProjectPeopleError::InvalidInput)
        ));
    }
    assert!(
        read(
            &pool,
            ids.org_id,
            ids.user_id,
            Context::Create,
            &search(&"王".repeat(100))
        )
        .await
        .unwrap()
        .people
        .is_empty()
    );
    assert_eq!(
        read(
            &pool,
            ids.org_id,
            ids.user_id,
            Context::Create,
            &Query::Resolve {
                ids: vec![ids.user_id; 500]
            }
        )
        .await
        .unwrap()
        .people
        .len(),
        1
    );
    assert!(
        read(
            &pool,
            ids.org_id,
            ids.user_id,
            Context::Create,
            &Query::Resolve { ids: vec![] }
        )
        .await
        .unwrap()
        .people
        .is_empty()
    );
    grants(&pool, ids.user_id, &[]).await;
    for query in &invalid {
        assert!(matches!(
            read(&pool, ids.org_id, ids.user_id, Context::Create, query).await,
            Err(ProjectPeopleError::Forbidden)
        ));
    }
    assert!(matches!(
        read(
            &pool,
            ids.org_id,
            ids.user_id,
            Context::Create,
            &Query::Resolve { ids: vec![] }
        )
        .await,
        Err(ProjectPeopleError::Forbidden)
    ));
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn project_people_rechecks_designation_after_waiting_for_revocation(pool: PgPool) {
    let ids = fixture(&pool).await;
    grants(&pool, ids.user_id, &[Permission::ProjectWriteManaged]).await;
    sqlx::query!("INSERT INTO project_management_assignments (id,org_id,manager_id,project_id) VALUES ($1,$2,$3,$4)", Uuid::now_v7(), ids.org_id, ids.user_id, ids.project_id).execute(&pool).await.unwrap();
    let context = Context::Edit {
        project_id: ids.project_id,
    };
    assert_eq!(
        read(&pool, ids.org_id, ids.user_id, context, &search(""))
            .await
            .unwrap()
            .people
            .len(),
        1
    );
    let mut removal = pool.begin().await.unwrap();
    sqlx::query!(
        "SELECT id FROM organizations WHERE id=$1 FOR UPDATE",
        ids.org_id
    )
    .fetch_one(&mut *removal)
    .await
    .unwrap();
    sqlx::query!(
        "DELETE FROM project_management_assignments WHERE manager_id=$1",
        ids.user_id
    )
    .execute(&mut *removal)
    .await
    .unwrap();
    let holder = sqlx::query_scalar!("SELECT pg_backend_pid()")
        .fetch_one(&mut *removal)
        .await
        .unwrap()
        .unwrap();
    let db = pool.clone();
    let pending =
        tokio::spawn(async move { read(&db, ids.org_id, ids.user_id, context, &search("")).await });
    wait_for_blocked(&pool, holder).await;
    removal.commit().await.unwrap();
    assert!(matches!(
        pending.await.unwrap(),
        Err(ProjectPeopleError::Forbidden)
    ));
    assert!(matches!(
        read(
            &pool,
            ids.org_id,
            ids.user_id,
            context,
            &Query::Resolve {
                ids: vec![ids.user_id]
            }
        )
        .await,
        Err(ProjectPeopleError::Forbidden)
    ));
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn project_people_denies_invalid_authority_even_for_empty_resolution(pool: PgPool) {
    let ids = fixture(&pool).await;
    let foreign = fixture(&pool).await;
    let query = Query::Resolve { ids: vec![] };
    for (org, actor) in [
        (ids.org_id, foreign.user_id),
        (foreign.org_id, ids.user_id),
        (ids.org_id, Uuid::now_v7()),
    ] {
        assert!(matches!(
            read(&pool, org, actor, Context::Create, &query).await,
            Err(ProjectPeopleError::Forbidden)
        ));
    }
    for version in [0, 2] {
        sqlx::query!(
            "UPDATE organizations SET permission_policy_version=$2 WHERE id=$1",
            ids.org_id,
            version
        )
        .execute(&pool)
        .await
        .unwrap();
        assert!(
            read(&pool, ids.org_id, ids.user_id, Context::Create, &query)
                .await
                .is_err()
        );
    }
    sqlx::query!(
        "UPDATE organizations SET permission_policy_version=1 WHERE id=$1",
        ids.org_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!("UPDATE users SET active=false WHERE id=$1", ids.user_id)
        .execute(&pool)
        .await
        .unwrap();
    assert!(matches!(
        read(&pool, ids.org_id, ids.user_id, Context::Create, &query).await,
        Err(ProjectPeopleError::Forbidden)
    ));
    sqlx::query!("UPDATE users SET active=true WHERE id=$1", ids.user_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query!("UPDATE person_permission_states SET grants=ARRAY['unknown_private_grant'] WHERE user_id=$1", ids.user_id).execute(&pool).await.unwrap();
    assert!(matches!(
        read(&pool, ids.org_id, ids.user_id, Context::Create, &query).await,
        Err(ProjectPeopleError::Storage(_))
    ));
    sqlx::query!(
        "DELETE FROM person_permission_states WHERE user_id=$1",
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(matches!(
        read(&pool, ids.org_id, ids.user_id, Context::Create, &query).await,
        Err(ProjectPeopleError::Unavailable)
    ));
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn project_people_rechecks_grants_and_direct_deactivation_after_wait(pool: PgPool) {
    let ids = fixture(&pool).await;
    for deactivate in [false, true] {
        grants(&pool, ids.user_id, &[Permission::ProjectCreateAll]).await;
        let mut revoke = pool.begin().await.unwrap();
        if deactivate {
            sqlx::query!("UPDATE users SET active=false WHERE id=$1", ids.user_id)
                .execute(&mut *revoke)
                .await
                .unwrap();
        } else {
            sqlx::query!(
                "SELECT id FROM organizations WHERE id=$1 FOR UPDATE",
                ids.org_id
            )
            .fetch_one(&mut *revoke)
            .await
            .unwrap();
            let revoked: Vec<String> = serde_json::from_value(
                serde_json::to_value(BuiltInProfile::Member.selection()).unwrap(),
            )
            .unwrap();
            sqlx::query!(
                "UPDATE person_permission_states SET grants=$2 WHERE user_id=$1",
                ids.user_id,
                &revoked
            )
            .execute(&mut *revoke)
            .await
            .unwrap();
        }
        let holder = sqlx::query_scalar!("SELECT pg_backend_pid()")
            .fetch_one(&mut *revoke)
            .await
            .unwrap()
            .unwrap();
        let db = pool.clone();
        let pending = tokio::spawn(async move {
            read(&db, ids.org_id, ids.user_id, Context::Create, &search("")).await
        });
        wait_for_blocked(&pool, holder).await;
        revoke.commit().await.unwrap();
        assert!(matches!(
            pending.await.unwrap(),
            Err(ProjectPeopleError::Forbidden)
        ));
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn project_people_holds_authority_until_read_finishes(pool: PgPool) {
    let ids = fixture(&pool).await;
    let mut hold = pool.begin().await.unwrap();
    // Block after the reader locks its organization and actor, without a test hook.
    sqlx::query!("LOCK TABLE person_permission_states IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *hold)
        .await
        .unwrap();
    let holder = sqlx::query_scalar!("SELECT pg_backend_pid()")
        .fetch_one(&mut *hold)
        .await
        .unwrap()
        .unwrap();
    let db = pool.clone();
    let reader = tokio::spawn(async move {
        read(&db, ids.org_id, ids.user_id, Context::Create, &search("")).await
    });
    wait_for_blocked(&pool, holder).await;
    let reader_pid = sqlx::query_scalar!("SELECT pid FROM pg_stat_activity WHERE datname = current_database() AND $1 = ANY(pg_blocking_pids(pid))", holder).fetch_one(&pool).await.unwrap().unwrap();
    let db = pool.clone();
    let writer = tokio::spawn(async move {
        let mut tx = db.begin().await.unwrap();
        sqlx::query!(
            "SELECT id FROM organizations WHERE id=$1 FOR UPDATE",
            ids.org_id
        )
        .fetch_one(&mut *tx)
        .await
        .unwrap();
        let revoked: Vec<String> = serde_json::from_value(
            serde_json::to_value(BuiltInProfile::Member.selection()).unwrap(),
        )
        .unwrap();
        sqlx::query!(
            "UPDATE person_permission_states SET grants=$2 WHERE user_id=$1",
            ids.user_id,
            &revoked
        )
        .execute(&mut *tx)
        .await
        .unwrap();
        tx.commit().await.unwrap();
    });
    wait_for_blocked(&pool, reader_pid).await;
    hold.commit().await.unwrap();
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(5), reader)
            .await
            .unwrap()
            .unwrap()
            .unwrap()
            .people
            .len(),
        1
    );
    tokio::time::timeout(Duration::from_secs(5), writer)
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(
        read(&pool, ids.org_id, ids.user_id, Context::Create, &search("")).await,
        Err(ProjectPeopleError::Forbidden)
    ));
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn project_people_cancellation_releases_connection_and_preserves_pool_defaults(pool: PgPool) {
    let ids = fixture(&pool).await;
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
        ids.org_id
    )
    .fetch_one(&mut *writer)
    .await
    .unwrap();
    let holder = sqlx::query_scalar!("SELECT pg_backend_pid()")
        .fetch_one(&mut *writer)
        .await
        .unwrap()
        .unwrap();
    let db = reader.clone();
    let pending = tokio::spawn(async move {
        read(&db, ids.org_id, ids.user_id, Context::Create, &search("")).await
    });
    wait_for_blocked(&pool, holder).await;
    pending.abort();
    assert!(pending.await.unwrap_err().is_cancelled());
    writer.rollback().await.unwrap();
    assert_eq!(
        tokio::time::timeout(
            Duration::from_secs(6),
            read(
                &reader,
                ids.org_id,
                ids.user_id,
                Context::Create,
                &search("")
            )
        )
        .await
        .unwrap()
        .unwrap()
        .people
        .len(),
        1
    );
    let defaults = sqlx::query!("SELECT current_setting('default_transaction_isolation') AS isolation, current_setting('default_transaction_read_only') AS readonly").fetch_one(&reader).await.unwrap();
    assert_eq!(defaults.isolation.as_deref(), Some("repeatable read"));
    assert_eq!(defaults.readonly.as_deref(), Some("on"));
    reader.close().await;
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn project_people_accepts_unconfigured_targets_and_exact_page_without_more(pool: PgPool) {
    let ids = fixture(&pool).await;
    let mut expected = Vec::new();
    for _ in 0..50 {
        let target = person(&pool, ids.org_id, false, BuiltInProfile::Member).await;
        sqlx::query!("UPDATE users SET name='Team 王' WHERE id=$1", target)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query!(
            "DELETE FROM person_permission_states WHERE user_id=$1",
            target
        )
        .execute(&pool)
        .await
        .unwrap();
        expected.push(target);
    }
    expected.sort_unstable();
    let page = read(
        &pool,
        ids.org_id,
        ids.user_id,
        Context::Create,
        &search(" tEaM 王 "),
    )
    .await
    .unwrap();
    assert_eq!(
        page.people.iter().map(|p| p.id).collect::<Vec<_>>(),
        expected
    );
    assert_eq!(page.next_after, None);
    // A cursor remains an ordering bound after the corresponding row disappears.
    sqlx::query!("DELETE FROM users WHERE id=$1", expected[0])
        .execute(&pool)
        .await
        .unwrap();
    let next = read(
        &pool,
        ids.org_id,
        ids.user_id,
        Context::Create,
        &Query::Search {
            query: "TEAM".into(),
            after: Some(PeopleCursor {
                name: "Team 王".into(),
                id: expected[0],
            }),
        },
    )
    .await
    .unwrap();
    assert_eq!(
        next.people.iter().map(|p| p.id).collect::<Vec<_>>(),
        expected[1..]
    );
    assert_eq!(next.next_after, None);
}
