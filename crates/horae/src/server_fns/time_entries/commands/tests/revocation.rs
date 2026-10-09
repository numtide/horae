use super::*;
use crate::server_fns::test_seed::wait_for_blocked;

#[derive(Clone, Copy, Debug)]
enum Revocation {
    Assignment,
    PersonScope,
    ActorActivity,
    SubjectActivity,
    TaskActivity,
}

async fn revoke(tx: &mut Transaction<'_, Postgres>, f: &Fixture, kind: Revocation) {
    crate::db::lock_organization(tx, f.ids.org_id, crate::db::OrganizationLock::AccessChange)
        .await
        .unwrap();
    match kind {
        Revocation::Assignment => {
            sqlx::query!(
                "DELETE FROM assignments WHERE user_id=$1",
                f.context.subject_id
            )
            .execute(&mut **tx)
            .await
            .unwrap();
        }
        Revocation::PersonScope => {
            sqlx::query!(
                "DELETE FROM person_management_assignments WHERE manager_id=$1",
                f.ids.user_id
            )
            .execute(&mut **tx)
            .await
            .unwrap();
        }
        Revocation::ActorActivity | Revocation::SubjectActivity => {
            let user = if matches!(kind, Revocation::ActorActivity) {
                f.ids.user_id
            } else {
                f.context.subject_id
            };
            sqlx::query!("UPDATE users SET active=false WHERE id=$1", user)
                .execute(&mut **tx)
                .await
                .unwrap();
        }
        Revocation::TaskActivity => {
            sqlx::query!("UPDATE tasks SET active=false WHERE id=$1", f.ids.task_id)
                .execute(&mut **tx)
                .await
                .unwrap();
        }
    }
}

async fn running(pool: &PgPool, f: &Fixture) -> Uuid {
    let id = f.entry(pool).await;
    sqlx::query!(
        "UPDATE time_entries SET is_running=true,started_at=now()-interval '5 minutes' WHERE id=$1",
        id
    )
    .execute(pool)
    .await
    .unwrap();
    id
}

#[sqlx::test(migrations = "./migrations")]
async fn delegated_stop_waits_for_revocation_and_observes_commit_or_rollback(pool: PgPool) {
    for kind in [
        Revocation::Assignment,
        Revocation::PersonScope,
        Revocation::ActorActivity,
        Revocation::SubjectActivity,
        Revocation::TaskActivity,
    ] {
        for commit in [true, false] {
            let f = fixture(&pool).await;
            let id = running(&pool, &f).await;
            let mut change = pool.begin().await.unwrap();
            revoke(&mut change, &f, kind).await;
            let pid = sqlx::query_scalar!("SELECT pg_backend_pid() AS \"pid!\"")
                .fetch_one(&mut *change)
                .await
                .unwrap();
            let db = pool.clone();
            let context = f.context;
            let actor = f.ids.user_id;
            let org = f.ids.org_id;
            let mut operations = tokio::task::JoinSet::new();
            operations.spawn(async move {
                apply(
                    &db,
                    org,
                    actor,
                    &context,
                    TimesheetCommand::StopTimer { entry_id: id },
                )
                .await
                .map(|_| ())
            });
            wait_for_blocked(&pool, pid).await;
            if commit {
                change.commit().await.unwrap();
            } else {
                change.rollback().await.unwrap();
            }
            let result = operations.join_next().await.unwrap().unwrap();
            assert_eq!(result.is_err(), commit, "{kind:?}: {result:?}");
            assert_eq!(
                sqlx::query_scalar!("SELECT is_running FROM time_entries WHERE id=$1", id)
                    .fetch_one(&pool)
                    .await
                    .unwrap(),
                commit
            );
        }
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn admitted_delegated_stop_finishes_before_assignment_revocation(pool: PgPool) {
    let f = fixture(&pool).await;
    let id = running(&pool, &f).await;
    let mut held = pool.begin().await.unwrap();
    sqlx::query!("SELECT id FROM time_entries WHERE id=$1 FOR UPDATE", id)
        .fetch_one(&mut *held)
        .await
        .unwrap();
    let pid = sqlx::query_scalar!("SELECT pg_backend_pid() AS \"pid!\"")
        .fetch_one(&mut *held)
        .await
        .unwrap();
    let db = pool.clone();
    let context = f.context;
    let actor = f.ids.user_id;
    let org = f.ids.org_id;
    let mut writes = tokio::task::JoinSet::new();
    writes.spawn(async move {
        apply(
            &db,
            org,
            actor,
            &context,
            TimesheetCommand::StopTimer { entry_id: id },
        )
        .await
        .map(|_| ())
    });
    wait_for_blocked(&pool, pid).await;
    let writer=sqlx::query_scalar!("SELECT pid FROM pg_stat_activity WHERE datname=current_database() AND $1=ANY(pg_blocking_pids(pid))",pid).fetch_one(&pool).await.unwrap().unwrap();
    let db = pool.clone();
    let subject = f.context.subject_id;
    let mut changes = tokio::task::JoinSet::new();
    changes.spawn(async move {
        let mut tx = db.begin().await.unwrap();
        crate::db::lock_organization(&mut tx, org, crate::db::OrganizationLock::AccessChange)
            .await
            .unwrap();
        sqlx::query!("DELETE FROM assignments WHERE user_id=$1", subject)
            .execute(&mut *tx)
            .await
            .unwrap();
        tx.commit().await.unwrap();
    });
    wait_for_blocked(&pool, writer).await;
    held.commit().await.unwrap();
    writes.join_next().await.unwrap().unwrap().unwrap();
    changes.join_next().await.unwrap().unwrap();
    assert!(
        !sqlx::query_scalar!("SELECT is_running FROM time_entries WHERE id=$1", id)
            .fetch_one(&pool)
            .await
            .unwrap()
    );
    let id = running(&pool, &f).await;
    assert!(
        f.execute(&pool, TimesheetCommand::StopTimer { entry_id: id })
            .await
            .is_err()
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn cancelling_waiting_command_releases_authority_and_subject_barriers(pool: PgPool) {
    let f = fixture(&pool).await;
    let id = f.entry(&pool).await;
    let mut held = pool.begin().await.unwrap();
    sqlx::query!("SELECT id FROM time_entries WHERE id=$1 FOR UPDATE", id)
        .fetch_one(&mut *held)
        .await
        .unwrap();
    let pid = sqlx::query_scalar!("SELECT pg_backend_pid() AS \"pid!\"")
        .fetch_one(&mut *held)
        .await
        .unwrap();
    let db = pool.clone();
    let context = f.context;
    let actor = f.ids.user_id;
    let org = f.ids.org_id;
    let mut writes = tokio::task::JoinSet::new();
    writes.spawn(async move {
        apply(
            &db,
            org,
            actor,
            &context,
            TimesheetCommand::Delete {
                entry_ids: vec![id],
            },
        )
        .await
        .map(|_| ())
    });
    wait_for_blocked(&pool, pid).await;
    writes.abort_all();
    assert!(
        writes
            .join_next()
            .await
            .unwrap()
            .unwrap_err()
            .is_cancelled()
    );
    held.rollback().await.unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(5),async {
        let mut tx=pool.begin().await.unwrap();
        crate::db::lock_organization(&mut tx,org,crate::db::OrganizationLock::AccessChange).await.unwrap();
        sqlx::query!(r#"SELECT pg_advisory_xact_lock(hashtextextended('horae.timesheet:' || $1::uuid::text,0)) AS "lock!: ()""#,context.subject_id).execute(&mut *tx).await.unwrap();
        tx.rollback().await.unwrap();
    }).await.unwrap();
    assert_eq!(
        sqlx::query_scalar!("SELECT minutes FROM time_entries WHERE id=$1", id)
            .fetch_one(&pool)
            .await
            .unwrap(),
        60
    );
}
