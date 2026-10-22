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
            Self::DetailView => {
                match detail_view::fetch(pool, viewer, project, None, today()).await {
                    Ok(view) => vec![view.project.id],
                    Err(ServerFnError::ServerError {
                        code: NOT_FOUND | FORBIDDEN,
                        ..
                    }) => Vec::new(),
                    other => panic!("unexpected detail view result: {other:?}"),
                }
            }
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

async fn backend_pid(tx: &mut sqlx::Transaction<'_, sqlx::Postgres>) -> i32 {
    sqlx::query_scalar!("SELECT pg_backend_pid()")
        .fetch_one(&mut **tx)
        .await
        .unwrap()
        .unwrap()
}

async fn blocked_by(pool: &PgPool, blocker: i32) -> i32 {
    wait_for_blocked(pool, blocker).await;
    sqlx::query_scalar!(
        "SELECT pid FROM pg_stat_activity WHERE datname=current_database() AND $1=ANY(pg_blocking_pids(pid))",
        blocker,
    )
    .fetch_one(pool)
    .await
    .unwrap()
    .unwrap()
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn detail_view_reads_identity_and_workflow_labels_from_one_snapshot(pool: PgPool) {
    let (ids, viewer) = tagged_reader(&pool).await;
    let before = detail_view::fetch(&pool, &viewer, ids.project_id, None, today())
        .await
        .unwrap();
    assert!(before.tasks.is_empty());
    // Unlocked data only: the authority rows stay untouched by this writer.
    let mut writer = pool.begin().await.unwrap();
    sqlx::query!("LOCK TABLE project_tasks IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *writer)
        .await
        .unwrap();
    let holder = backend_pid(&mut writer).await;
    let pending_pool = pool.clone();
    let pending_viewer = viewer.clone();
    let pending = tokio::spawn(async move {
        detail_view::fetch(
            &pending_pool,
            &pending_viewer,
            ids.project_id,
            None,
            today(),
        )
        .await
    });
    wait_for_blocked(&pool, holder).await;
    sqlx::query!(
        "UPDATE projects SET name='Renamed while reading' WHERE id=$1",
        ids.project_id
    )
    .execute(&mut *writer)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO project_tasks (project_id,task_id,billable) VALUES ($1,$2,true)",
        ids.project_id,
        ids.task_id,
    )
    .execute(&mut *writer)
    .await
    .unwrap();
    writer.commit().await.unwrap();
    let during = pending.await.unwrap().unwrap();
    assert_eq!(
        (during.project.name.as_str(), during.tasks.len()),
        (before.project.name.as_str(), 0)
    );
    let after = detail_view::fetch(&pool, &viewer, ids.project_id, None, today())
        .await
        .unwrap();
    assert_eq!(
        (after.project.name.as_str(), after.tasks.len()),
        ("Renamed while reading", 1)
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn detail_view_reads_identity_and_budget_from_one_snapshot(pool: PgPool) {
    let (ids, viewer) = tagged_reader(&pool).await;
    sqlx::query!(
        "UPDATE projects SET budget_kind='hours', budget_minutes=120 WHERE id=$1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let before = detail_view::fetch(&pool, &viewer, ids.project_id, None, today())
        .await
        .unwrap();
    let mut writer = pool.begin().await.unwrap();
    sqlx::query!("LOCK TABLE time_entries IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *writer)
        .await
        .unwrap();
    let holder = backend_pid(&mut writer).await;
    let pending_pool = pool.clone();
    let pending_viewer = viewer.clone();
    let pending = tokio::spawn(async move {
        detail_view::fetch(
            &pending_pool,
            &pending_viewer,
            ids.project_id,
            None,
            today(),
        )
        .await
    });
    wait_for_blocked(&pool, holder).await;
    sqlx::query!(
        "UPDATE projects SET name='Renamed while reading', budget_minutes=30 WHERE id=$1",
        ids.project_id
    )
    .execute(&mut *writer)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO time_entries (id,org_id,user_id,project_id,task_id,spent_date,minutes,billable,state)
         VALUES ($1,$2,$3,$4,$5,'2026-09-07',45,true,'open')",
        Uuid::now_v7(),
        ids.org_id,
        ids.user_id,
        ids.project_id,
        ids.task_id,
    )
    .execute(&mut *writer)
    .await
    .unwrap();
    writer.commit().await.unwrap();
    let during = pending.await.unwrap().unwrap();
    assert_eq!(
        (during.project.name, during.budget),
        (before.project.name, before.budget)
    );
    let after = detail_view::fetch(&pool, &viewer, ids.project_id, None, today())
        .await
        .unwrap();
    assert_eq!(
        after.budget.unwrap().headline,
        horae_core::budget::BudgetHeadline::Exact {
            budget: 30,
            spent: 45,
            remaining: -15
        }
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn detail_view_observes_membership_removed_behind_the_lock_only_access_gate(pool: PgPool) {
    let (ids, viewer) = fixture(&pool, OrgRole::Member, PermissionSelection::new(&[])).await;
    let assignment = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO assignments (id,project_id,user_id) VALUES ($1,$2,$3)",
        assignment,
        ids.project_id,
        ids.user_id,
    )
    .execute(&pool)
    .await
    .unwrap();
    let admin_id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO users (id,org_id,email,name,org_role) VALUES ($1,$2,$3,'Gate admin','admin')",
        admin_id,
        ids.org_id,
        format!("{admin_id}@test.com"),
    )
    .execute(&pool)
    .await
    .unwrap();
    let admin = User {
        id: admin_id,
        ..super::detail::legacy_viewer(&ids, OrgRole::Admin)
    };
    detail_view::fetch(&pool, &viewer, ids.project_id, None, today())
        .await
        .unwrap();
    // Pause the production writer after it takes the lock-only access gate.
    let mut hold = pool.begin().await.unwrap();
    sqlx::query!("LOCK TABLE assignments IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *hold)
        .await
        .unwrap();
    let holder = backend_pid(&mut hold).await;
    let writer_pool = pool.clone();
    let writer =
        tokio::spawn(async move { remove_assignment(&writer_pool, &admin, assignment).await });
    let writer_pid = blocked_by(&pool, holder).await;
    let reader_pool = pool.clone();
    let reader_viewer = viewer.clone();
    let reader = tokio::spawn(async move {
        detail_view::fetch(&reader_pool, &reader_viewer, ids.project_id, None, today()).await
    });
    wait_for_blocked(&pool, writer_pid).await;
    hold.rollback().await.unwrap();
    assert!(writer.await.unwrap().unwrap().is_some());
    assert!(
        matches!(
            reader.await.unwrap(),
            Err(ServerFnError::ServerError {
                code: NOT_FOUND | FORBIDDEN,
                ..
            })
        ),
        "a reader that waited for the access gate used a snapshot from before the removal"
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn detail_view_observes_report_visibility_revoked_by_the_project_editor(pool: PgPool) {
    use crate::models::project_creation::{
        ProjectEditRequest, ProtectedProjectField, ReportVisibility,
    };
    use crate::server_fns::project_creation::editing::{
        load_editable_project, save_editable_project,
    };

    // A canonical editor: its save re-enters the gate through the manager
    // authorization path with the manager set unchanged.
    let (ids, _) = fixture(
        &pool,
        OrgRole::Admin,
        PermissionSelection::new(&[Permission::ProjectReadAll, Permission::ProjectWriteAll]),
    )
    .await;
    sqlx::query!(
        "UPDATE projects SET rate_cents=10000 WHERE id=$1",
        ids.project_id
    )
    .execute(&pool)
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
    .execute(&pool)
    .await
    .unwrap();
    let member_id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO users (id,org_id,email,name,org_role) VALUES ($1,$2,$3,'Shared member','member')",
        member_id,
        ids.org_id,
        format!("{member_id}@test.com"),
    )
    .execute(&pool)
    .await
    .unwrap();
    let no_grants: Vec<String> =
        serde_json::from_value(serde_json::to_value(PermissionSelection::new(&[])).unwrap())
            .unwrap();
    sqlx::query!(
        "INSERT INTO person_permission_states
         (id, org_id, user_id, catalog_version, grants, is_administrator, source)
         VALUES ($1, $2, $3, 1, $4, false, 'individual')",
        Uuid::now_v7(),
        ids.org_id,
        member_id,
        &no_grants,
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO assignments (id,project_id,user_id) VALUES ($1,$2,$3)",
        Uuid::now_v7(),
        ids.project_id,
        member_id,
    )
    .execute(&pool)
    .await
    .unwrap();
    let member = User {
        id: member_id,
        ..super::detail::legacy_viewer(&ids, OrgRole::Member)
    };
    detail_view::fetch(&pool, &member, ids.project_id, None, today())
        .await
        .unwrap();
    let editor = load_editable_project(&pool, ids.user_id, ids.org_id, ids.project_id)
        .await
        .unwrap();
    assert!(editor.access.is_some(), "the editor must be canonical");
    let mut unchanged = vec![
        ProtectedProjectField::ProjectRate,
        ProtectedProjectField::Budget,
        ProtectedProjectField::Fees,
        ProtectedProjectField::InvoiceDefaults,
        ProtectedProjectField::PrivateNotes,
    ];
    unchanged.extend(
        editor
            .form
            .tasks
            .iter()
            .map(|task| ProtectedProjectField::TaskRate(task.id)),
    );
    for person in &editor.form.team {
        unchanged.extend([
            ProtectedProjectField::PersonRate(person.user_id),
            ProtectedProjectField::CostRate(person.user_id),
        ]);
    }
    let mut request = ProjectEditRequest {
        task_activity: Vec::new(),
        id: Uuid::now_v7(),
        project_id: editor.id,
        expected_revision: editor.revision,
        expected_requester: editor.access.as_ref().map(|access| access.requester),
        managers: editor
            .access
            .as_ref()
            .map(|access| (&access.managers).into()),
        form: editor.form,
        unchanged,
    };
    request.form.report_visibility = ReportVisibility::Managers;
    // Pause the production save after it takes the access gate.
    let mut hold = pool.begin().await.unwrap();
    sqlx::query!("LOCK TABLE project_settings IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *hold)
        .await
        .unwrap();
    let holder = backend_pid(&mut hold).await;
    let writer_pool = pool.clone();
    let writer = tokio::spawn(async move {
        save_editable_project(&writer_pool, ids.user_id, ids.org_id, &request, false).await
    });
    let writer_pid = blocked_by(&pool, holder).await;
    let reader_pool = pool.clone();
    let reader = tokio::spawn(async move {
        detail_view::fetch(&reader_pool, &member, ids.project_id, None, today()).await
    });
    wait_for_blocked(&pool, writer_pid).await;
    hold.rollback().await.unwrap();
    writer.await.unwrap().unwrap();
    assert!(matches!(
        reader.await.unwrap(),
        Err(ServerFnError::ServerError {
            code: NOT_FOUND | FORBIDDEN,
            ..
        })
    ));
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn membership_removal_waits_for_a_reader_that_already_holds_authority(pool: PgPool) {
    let (ids, viewer) = fixture(&pool, OrgRole::Member, PermissionSelection::new(&[])).await;
    let assignment = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO assignments (id,project_id,user_id) VALUES ($1,$2,$3)",
        assignment,
        ids.project_id,
        ids.user_id,
    )
    .execute(&pool)
    .await
    .unwrap();
    let admin_id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO users (id,org_id,email,name,org_role) VALUES ($1,$2,$3,'Gate admin','admin')",
        admin_id,
        ids.org_id,
        format!("{admin_id}@test.com"),
    )
    .execute(&pool)
    .await
    .unwrap();
    let admin = User {
        id: admin_id,
        ..super::detail::legacy_viewer(&ids, OrgRole::Admin)
    };
    // The reader takes its authority locks, then waits on a later table.
    let mut hold = pool.begin().await.unwrap();
    sqlx::query!("LOCK TABLE project_tags IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *hold)
        .await
        .unwrap();
    let holder = backend_pid(&mut hold).await;
    let reader_pool = pool.clone();
    let reader_viewer = viewer.clone();
    let reader = tokio::spawn(async move {
        detail_view::fetch(&reader_pool, &reader_viewer, ids.project_id, None, today()).await
    });
    let reader_pid = blocked_by(&pool, holder).await;
    let writer_pool = pool.clone();
    let writer =
        tokio::spawn(async move { remove_assignment(&writer_pool, &admin, assignment).await });
    wait_for_blocked(&pool, reader_pid).await;
    hold.rollback().await.unwrap();
    assert_eq!(reader.await.unwrap().unwrap().project.id, ids.project_id);
    assert!(writer.await.unwrap().unwrap().is_some());
    assert!(matches!(
        detail_view::fetch(&pool, &viewer, ids.project_id, None, today()).await,
        Err(ServerFnError::ServerError {
            code: NOT_FOUND | FORBIDDEN,
            ..
        })
    ));
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn access_change_gate_keeps_the_stored_organization_unchanged(pool: PgPool) {
    use crate::db::{OrganizationLock, lock_organization};

    let ids = seed(&pool, OrgRole::Admin).await;
    let stored = |pool: PgPool| async move {
        sqlx::query!(
            r#"SELECT to_jsonb(o) AS "row!", xmin::text AS "version!" FROM organizations o WHERE id=$1"#,
            ids.org_id
        )
        .fetch_one(&pool)
        .await
        .map(|row| (row.row, row.version))
        .unwrap()
    };
    let (before, version) = stored(pool.clone()).await;
    let mut rolled_back = pool.begin().await.unwrap();
    lock_organization(&mut rolled_back, ids.org_id, OrganizationLock::AccessChange)
        .await
        .unwrap();
    rolled_back.rollback().await.unwrap();
    assert_eq!(
        stored(pool.clone()).await,
        (before.clone(), version.clone())
    );
    let mut committed = pool.begin().await.unwrap();
    lock_organization(&mut committed, ids.org_id, OrganizationLock::AccessChange)
        .await
        .unwrap();
    committed.commit().await.unwrap();
    let (after, new_version) = stored(pool.clone()).await;
    // Every stored value, including access_revision, is unchanged; only the
    // row version that fences REPEATABLE READ readers is new.
    assert_eq!(after, before);
    assert_ne!(new_version, version);
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn detail_view_reports_a_conflict_after_three_serialization_failures(pool: PgPool) {
    let (ids, viewer) = tagged_reader(&pool).await;
    // Attempt 1 waits for the organization writer; attempt 2 for the actor
    // writer; attempt 3 queues behind an organization writer that was itself
    // waiting for attempt 2. Each writer commits after that attempt's snapshot.
    let mut organization = pool.begin().await.unwrap();
    sqlx::query!(
        "UPDATE organizations SET access_revision=access_revision+1 WHERE id=$1",
        ids.org_id
    )
    .execute(&mut *organization)
    .await
    .unwrap();
    let organization_pid = backend_pid(&mut organization).await;
    let mut actor = pool.begin().await.unwrap();
    sqlx::query!("UPDATE users SET name=name WHERE id=$1", ids.user_id)
        .execute(&mut *actor)
        .await
        .unwrap();
    let actor_pid = backend_pid(&mut actor).await;
    let pending_pool = pool.clone();
    let pending_viewer = viewer.clone();
    let pending = tokio::spawn(async move {
        detail_view::fetch(
            &pending_pool,
            &pending_viewer,
            ids.project_id,
            None,
            today(),
        )
        .await
    });
    wait_for_blocked(&pool, organization_pid).await;
    organization.commit().await.unwrap();
    let second_attempt = blocked_by(&pool, actor_pid).await;
    let mut third = pool.begin().await.unwrap();
    let third_pid = backend_pid(&mut third).await;
    let third_writer = tokio::spawn(async move {
        sqlx::query!(
            "UPDATE organizations SET access_revision=access_revision+1 WHERE id=$1",
            ids.org_id
        )
        .execute(&mut *third)
        .await
        .unwrap();
        third
    });
    wait_for_blocked(&pool, second_attempt).await;
    actor.commit().await.unwrap();
    let third = third_writer.await.unwrap();
    wait_for_blocked(&pool, third_pid).await;
    third.commit().await.unwrap();
    assert!(matches!(
        pending.await.unwrap(),
        Err(ServerFnError::ServerError { code: CONFLICT, .. })
    ));
    let fresh = detail_view::fetch(&pool, &viewer, ids.project_id, None, today())
        .await
        .unwrap();
    assert_eq!(fresh.project.id, ids.project_id);
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn detail_view_snapshot_preserves_inherited_connection_settings(pool: PgPool) {
    let (ids, viewer) = tagged_reader(&pool).await;
    let expected = detail_view::fetch(&pool, &viewer, ids.project_id, None, today())
        .await
        .unwrap();
    let reader_pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect_with((*pool.connect_options()).clone())
        .await
        .unwrap();
    for isolation in ["read committed", "repeatable read", "serializable"] {
        sqlx::query!("SELECT set_config('default_transaction_isolation', $1, false) AS isolation, set_config('default_transaction_read_only', 'on', false) AS read_only, set_config('statement_timeout', '4000ms', false) AS timeout", isolation).fetch_one(&reader_pool).await.unwrap();
        assert_eq!(
            detail_view::fetch(&reader_pool, &viewer, ids.project_id, None, today())
                .await
                .unwrap(),
            expected
        );
        let settings = sqlx::query!("SELECT current_setting('transaction_isolation') AS \"isolation!\", current_setting('transaction_read_only') AS \"read_only!\", current_setting('statement_timeout') AS \"timeout!\"").fetch_one(&reader_pool).await.unwrap();
        assert_eq!(
            (
                settings.isolation.as_str(),
                settings.read_only.as_str(),
                settings.timeout.as_str()
            ),
            (isolation, "on", "4s")
        );
    }
    reader_pool.close().await;
}
