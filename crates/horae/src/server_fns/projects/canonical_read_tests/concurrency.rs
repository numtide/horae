use super::*;
use crate::server_fns::test_seed::wait_for_blocked;

#[derive(Clone, Copy, Debug)]
enum Reader {
    List,
    Details,
    Tags,
    Spend,
    Budget,
    Team,
    DetailView,
}

impl Reader {
    async fn projects(self, pool: &PgPool, viewer: &User, project: Uuid) -> Vec<Uuid> {
        match self {
            Self::DetailView => match detail_view::fetch(pool, viewer, project, None).await {
                Ok(view) => vec![view.project.id],
                Err(ServerFnError::ServerError {
                    code: NOT_FOUND | FORBIDDEN,
                    ..
                }) => Vec::new(),
                other => panic!("unexpected detail view result: {other:?}"),
            },
            Self::Team => assignments_for_viewer(pool, viewer, project)
                .await
                .unwrap()
                .into_iter()
                .map(|row| row.project_id)
                .collect(),
            Self::Budget => crate::server_fns::budgets::progress_for_viewer(
                pool,
                viewer.org_id,
                viewer.id,
                "2026-09-07".parse().unwrap(),
            )
            .await
            .unwrap()
            .into_iter()
            .map(|row| row.project_id)
            .collect(),
            Self::Spend => fetch_project_spend(pool, viewer.org_id, viewer.id)
                .await
                .unwrap()
                .into_iter()
                .map(|row| row.project_id)
                .collect(),
            Self::List => projects_for_viewer(pool, viewer, None, true, ProjectRead::Overview)
                .await
                .unwrap()
                .into_iter()
                .map(|row| row.id)
                .collect(),
            Self::Details => fetch_project_details(pool, viewer.org_id, viewer.id, project)
                .await
                .unwrap()
                .into_iter()
                .map(|row| row.id)
                .collect(),
            Self::Tags => fetch_project_tags(pool, viewer.org_id, viewer.id)
                .await
                .unwrap()
                .into_iter()
                .map(|row| row.project_id)
                .collect(),
        }
    }
}

async fn tagged_reader(pool: &PgPool) -> (SeedIds, User) {
    let (ids, viewer) = fixture(
        pool,
        OrgRole::Admin,
        PermissionSelection::new(&[Permission::ProjectReadAll]),
    )
    .await;
    sqlx::query!(
        "INSERT INTO project_settings (id,org_id,project_id,creator_id,rate_mode)
         VALUES ($1,$2,$3,$4,'project')",
        Uuid::now_v7(),
        ids.org_id,
        ids.project_id,
        ids.user_id,
    )
    .execute(pool)
    .await
    .unwrap();
    let tag_id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO project_tags (id,org_id,name) VALUES ($1,$2,'Launch')",
        tag_id,
        ids.org_id,
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO project_tag_links (id,org_id,project_id,tag_id) VALUES ($1,$2,$3,$4)",
        Uuid::now_v7(),
        ids.org_id,
        ids.project_id,
        tag_id,
    )
    .execute(pool)
    .await
    .unwrap();
    team::teammate(pool, &ids).await;
    (ids, viewer)
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn project_readers_observe_grant_revocation_after_waiting_for_organization(pool: PgPool) {
    for reader in [
        Reader::List,
        Reader::Details,
        Reader::Tags,
        Reader::Spend,
        Reader::Budget,
        Reader::Team,
        Reader::DetailView,
    ] {
        let (ids, viewer) = tagged_reader(&pool).await;
        assert_eq!(
            reader.projects(&pool, &viewer, ids.project_id).await,
            [ids.project_id]
        );
        let mut hold = pool.begin().await.unwrap();
        sqlx::query!(
            "UPDATE organizations SET access_revision=access_revision+1 WHERE id=$1",
            ids.org_id,
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
        let pending = tokio::spawn(async move {
            reader
                .projects(&pending_pool, &viewer, ids.project_id)
                .await
        });
        wait_for_blocked(&pool, holder).await;
        let floor: Vec<String> =
            serde_json::from_value(serde_json::to_value(PermissionSelection::new(&[])).unwrap())
                .unwrap();
        sqlx::query!(
            "UPDATE person_permission_states SET grants=$2 WHERE user_id=$1",
            ids.user_id,
            &floor,
        )
        .execute(&mut *hold)
        .await
        .unwrap();
        hold.commit().await.unwrap();
        assert!(pending.await.unwrap().is_empty(), "{reader:?}");
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn project_readers_observe_deactivation_after_waiting_for_actor(pool: PgPool) {
    for reader in [
        Reader::List,
        Reader::Details,
        Reader::Tags,
        Reader::Spend,
        Reader::Budget,
        Reader::Team,
        Reader::DetailView,
    ] {
        let (ids, viewer) = tagged_reader(&pool).await;
        let mut hold = pool.begin().await.unwrap();
        sqlx::query!("UPDATE users SET active=false WHERE id=$1", ids.user_id)
            .execute(&mut *hold)
            .await
            .unwrap();
        let holder = sqlx::query_scalar!("SELECT pg_backend_pid()")
            .fetch_one(&mut *hold)
            .await
            .unwrap()
            .unwrap();
        let pending_pool = pool.clone();
        let pending = tokio::spawn(async move {
            reader
                .projects(&pending_pool, &viewer, ids.project_id)
                .await
        });
        wait_for_blocked(&pool, holder).await;
        hold.commit().await.unwrap();
        assert!(pending.await.unwrap().is_empty(), "{reader:?}");
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn project_readers_hold_authority_until_materialization_before_deactivation(pool: PgPool) {
    for reader in [
        Reader::List,
        Reader::Details,
        Reader::Tags,
        Reader::Spend,
        Reader::Budget,
        Reader::Team,
        Reader::DetailView,
    ] {
        let (ids, viewer) = tagged_reader(&pool).await;
        let mut hold = pool.begin().await.unwrap();
        if matches!(reader, Reader::Team) {
            sqlx::query!("LOCK TABLE assignments IN ACCESS EXCLUSIVE MODE")
                .execute(&mut *hold)
                .await
                .unwrap();
        } else {
            sqlx::query!("LOCK TABLE project_settings IN ACCESS EXCLUSIVE MODE")
                .execute(&mut *hold)
                .await
                .unwrap();
        }
        let holder = sqlx::query_scalar!("SELECT pg_backend_pid()")
            .fetch_one(&mut *hold)
            .await
            .unwrap()
            .unwrap();
        let pending_pool = pool.clone();
        let pending_viewer = viewer.clone();
        let pending = tokio::spawn(async move {
            reader
                .projects(&pending_pool, &pending_viewer, ids.project_id)
                .await
        });
        wait_for_blocked(&pool, holder).await;
        let mut contender = pool.begin().await.unwrap();
        let error = sqlx::query_scalar!(
            "SELECT id FROM organizations WHERE id=$1 FOR UPDATE NOWAIT",
            ids.org_id,
        )
        .fetch_one(&mut *contender)
        .await
        .unwrap_err();
        assert_eq!(
            error.as_database_error().unwrap().code().as_deref(),
            Some("55P03")
        );
        contender.rollback().await.unwrap();
        let reader_pid = sqlx::query_scalar!(
            "SELECT pid FROM pg_stat_activity WHERE datname=current_database() AND $1=ANY(pg_blocking_pids(pid))",
            holder,
        ).fetch_one(&pool).await.unwrap().unwrap();
        let revoke_pool = pool.clone();
        let revoke = tokio::spawn(async move {
            sqlx::query!("UPDATE users SET active=false WHERE id=$1", ids.user_id)
                .execute(&revoke_pool)
                .await
                .unwrap();
        });
        wait_for_blocked(&pool, reader_pid).await;
        hold.rollback().await.unwrap();
        assert_eq!(pending.await.unwrap(), [ids.project_id], "{reader:?}");
        revoke.await.unwrap();
        assert!(
            reader
                .projects(&pool, &viewer, ids.project_id)
                .await
                .is_empty(),
            "{reader:?}"
        );
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn cancelling_project_readers_releases_organization_and_actor_fences(pool: PgPool) {
    for reader in [
        Reader::List,
        Reader::Details,
        Reader::Tags,
        Reader::Spend,
        Reader::Budget,
        Reader::Team,
        Reader::DetailView,
    ] {
        let (ids, viewer) = tagged_reader(&pool).await;
        let mut hold = pool.begin().await.unwrap();
        if matches!(reader, Reader::Team) {
            sqlx::query!("LOCK TABLE assignments IN ACCESS EXCLUSIVE MODE")
                .execute(&mut *hold)
                .await
                .unwrap();
        } else {
            sqlx::query!("LOCK TABLE project_settings IN ACCESS EXCLUSIVE MODE")
                .execute(&mut *hold)
                .await
                .unwrap();
        }
        let holder = sqlx::query_scalar!("SELECT pg_backend_pid()")
            .fetch_one(&mut *hold)
            .await
            .unwrap()
            .unwrap();
        let pending_pool = pool.clone();
        let pending = tokio::spawn(async move {
            reader
                .projects(&pending_pool, &viewer, ids.project_id)
                .await
        });
        wait_for_blocked(&pool, holder).await;
        pending.abort();
        assert!(pending.await.unwrap_err().is_cancelled());
        hold.rollback().await.unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(3), async {
            let mut contender = pool.begin().await.unwrap();
            sqlx::query!(
                "SELECT id FROM organizations WHERE id=$1 FOR UPDATE",
                ids.org_id
            )
            .fetch_one(&mut *contender)
            .await
            .unwrap();
            sqlx::query!("SELECT id FROM users WHERE id=$1 FOR UPDATE", ids.user_id)
                .fetch_one(&mut *contender)
                .await
                .unwrap();
            contender.rollback().await.unwrap();
        })
        .await
        .unwrap();
    }
}
