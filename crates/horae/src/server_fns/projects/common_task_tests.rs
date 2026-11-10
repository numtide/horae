use super::canonical_read_tests::fixture;
use super::task_catalog::fetch_task_catalog;
use super::*;
use crate::models::task::TaskActivity;
use horae_core::permissions::catalog::PermissionSelection;
use sqlx::PgPool;
use uuid::Uuid;

async fn is_default(pool: &PgPool, task_id: Uuid) -> bool {
    sqlx::query_scalar!("SELECT is_default FROM tasks WHERE id=$1", task_id)
        .fetch_one(pool)
        .await
        .unwrap()
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn common_flag_round_trips_through_create_and_update(pool: PgPool) {
    let (ids, _) = fixture(
        &pool,
        OrgRole::Member,
        PermissionSelection::new(&[Permission::TaskWriteAll]),
    )
    .await;
    let preserve = TaskRateEdit::Preserve {};
    let (task, _) = create_catalog_task(
        &pool,
        ids.org_id,
        ids.user_id,
        "Meetings",
        true,
        true,
        &preserve,
    )
    .await
    .unwrap();
    assert!(is_default(&pool, task.id).await);

    let (_, event) = update_task_record(
        &pool,
        ids.org_id,
        ids.user_id,
        task.id,
        "Meetings",
        true,
        false,
        &preserve,
    )
    .await
    .unwrap();
    assert!(event.is_some(), "changing only the flag is a change");
    assert!(!is_default(&pool, task.id).await);

    let (_, event) = update_task_record(
        &pool,
        ids.org_id,
        ids.user_id,
        task.id,
        "Meetings",
        true,
        false,
        &preserve,
    )
    .await
    .unwrap();
    assert!(event.is_none(), "saving the same values is a no-op");
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn common_flag_needs_task_editing_authority(pool: PgPool) {
    let (ids, _) = fixture(
        &pool,
        OrgRole::Admin,
        PermissionSelection::new(&[Permission::TaskReadAll]),
    )
    .await;
    let preserve = TaskRateEdit::Preserve {};
    let created = create_catalog_task(
        &pool,
        ids.org_id,
        ids.user_id,
        "Denied",
        true,
        true,
        &preserve,
    )
    .await;
    assert!(
        matches!(
            created,
            Err(ServerFnError::ServerError {
                code: FORBIDDEN,
                ..
            })
        ),
        "{created:?}"
    );
    let updated = update_task_record(
        &pool,
        ids.org_id,
        ids.user_id,
        ids.task_id,
        "Development",
        true,
        true,
        &preserve,
    )
    .await;
    assert!(
        matches!(
            updated,
            Err(ServerFnError::ServerError {
                code: FORBIDDEN,
                ..
            })
        ),
        "{updated:?}"
    );
    assert!(!is_default(&pool, ids.task_id).await);
}

/// Common tasks come first, then the rest; each group by name. The cursor
/// carries the group, so a page boundary inside either group, or between
/// them, neither skips nor repeats a task.
#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn catalog_lists_common_tasks_first_across_pages(pool: PgPool) {
    let (ids, _) = fixture(
        &pool,
        OrgRole::Member,
        PermissionSelection::new(&[Permission::TaskReadAll]),
    )
    .await;
    let requester = PermissionRequester {
        org_id: ids.org_id,
        user_id: ids.user_id,
    };
    let mut common = Vec::new();
    for _ in 0..101 {
        let id = Uuid::now_v7();
        sqlx::query!(
            "INSERT INTO tasks (id,org_id,name,billable_default,is_default) VALUES ($1,$2,'Zeta',true,true)",
            id,
            ids.org_id
        )
        .execute(&pool)
        .await
        .unwrap();
        common.push(id);
    }
    common.sort();
    let other = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO tasks (id,org_id,name,billable_default) VALUES ($1,$2,'Alpha',true)",
        other,
        ids.org_id
    )
    .execute(&pool)
    .await
    .unwrap();

    let first = fetch_task_catalog(&pool, requester, TaskActivity::All, None)
        .await
        .unwrap();
    let second = fetch_task_catalog(
        &pool,
        requester,
        TaskActivity::All,
        first.next_after.as_ref(),
    )
    .await
    .unwrap();
    assert!(second.next_after.is_none());
    let listed: Vec<_> = first.tasks.iter().chain(&second.tasks).collect();
    let mut expected = common.clone();
    expected.push(other);
    expected.push(ids.task_id);
    let names: Vec<_> = listed.iter().map(|task| task.name.as_str()).collect();
    assert_eq!(
        listed.iter().map(|task| task.id).collect::<Vec<_>>(),
        expected,
        "{names:?}"
    );
    assert!(listed[..101].iter().all(|task| task.is_default));
    assert!(listed[101..].iter().all(|task| !task.is_default));
}
