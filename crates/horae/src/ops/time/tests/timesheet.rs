//! The Timesheet commands through `ops::begin` / `ops::begin_read`, as the
//! session adapters run them.

use super::*;
use crate::models::permission_editor::PermissionRequester;
use crate::testing::time_entry;
use horae_core::permissions::catalog::PermissionSelection;
use sqlx::{Postgres, Transaction};

mod daily_limit;
mod guards;
mod revocation;
mod timezone;

async fn tracking(
    pool: &PgPool,
    org: Uuid,
    actor: Uuid,
    context: &TimesheetWriteContext,
) -> Result<Vec<TimesheetTrackingOption>, OpError> {
    let mut read = ops::begin_read(pool, Caller::session(org, actor)).await?;
    let result = crate::ops::time::tracking(&mut read.tx, &read.actor, context).await;
    read.finish(result).await
}

/// Commits the command and returns its result with the events it would
/// dispatch.
async fn apply(
    pool: &PgPool,
    org: Uuid,
    actor: Uuid,
    context: &TimesheetWriteContext,
    command: TimesheetCommand,
) -> Result<Done<TimesheetChange>, OpError> {
    let mut write = ops::begin(pool, Caller::session(org, actor)).await?;
    let done = crate::ops::time::apply(&mut write.tx, &write.actor, context, command).await?;
    write.tx.commit().await?;
    Ok(done)
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn archived_project_task_excludes_choices_and_new_time_without_changing_global_task(
    pool: PgPool,
) {
    let fixture = fixture(&pool).await;
    sqlx::query!(
        "UPDATE project_tasks SET active=false WHERE project_id=$1 AND task_id=$2",
        fixture.ids.project_id,
        fixture.ids.task_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(
        tracking(
            &pool,
            fixture.ids.org_id,
            fixture.ids.user_id,
            &fixture.context
        )
        .await
        .unwrap()
        .is_empty()
    );
    for command in [
        TimesheetCommand::Create {
            entry: fixture.input(),
        },
        TimesheetCommand::StartTimer {
            project_id: fixture.ids.project_id,
            task_id: fixture.ids.task_id,
            notes: None,
        },
    ] {
        let error = fixture.execute(&pool, command).await.unwrap_err();
        assert!(matches!(error, OpError::Conflict(_)), "{error:?}");
    }
    assert!(
        sqlx::query_scalar!("SELECT active FROM tasks WHERE id=$1", fixture.ids.task_id)
            .fetch_one(&pool)
            .await
            .unwrap()
    );
}

struct Fixture {
    ids: SeedIds,
    context: TimesheetWriteContext,
}

impl Fixture {
    fn input(&self) -> TimesheetEntryInput {
        TimesheetEntryInput {
            project_id: self.ids.project_id,
            task_id: self.ids.task_id,
            spent_date: "2026-09-07".parse().unwrap(),
            minutes: 45,
            notes: Some("Target work".into()),
            billable: true,
            start_minute: None,
        }
    }

    async fn entry(&self, pool: &PgPool) -> Uuid {
        time_entry(
            pool,
            &SeedIds {
                user_id: self.context.subject_id,
                ..self.ids
            },
            EntryState::Open,
        )
        .await
    }

    async fn execute(&self, pool: &PgPool, command: TimesheetCommand) -> Result<(), OpError> {
        apply(
            pool,
            self.ids.org_id,
            self.ids.user_id,
            &self.context,
            command,
        )
        .await
        .map(|_| ())
    }
}

async fn fixture(pool: &PgPool) -> Fixture {
    let ids = seed(pool, BuiltInProfile::ExecutiveManager).await;
    let target = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO users (id,org_id,email,name) VALUES ($1,$2,$3,'Target')",
        target,
        ids.org_id,
        format!("{target}@example.test")
    )
    .execute(pool)
    .await
    .unwrap();
    // The target stops their own timer too, which loads their stored state.
    crate::ops::permissions::provisioning::provision(
        &mut pool.acquire().await.unwrap(),
        ids.org_id,
        target,
        BuiltInProfile::Member,
    )
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO assignments (id,user_id,project_id) VALUES ($1,$2,$3)",
        Uuid::now_v7(),
        target,
        ids.project_id
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO project_tasks (project_id,task_id,billable) VALUES ($1,$2,true)",
        ids.project_id,
        ids.task_id
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query!("INSERT INTO person_management_assignments (id,org_id,manager_id,managed_user_id) VALUES ($1,$2,$3,$4)", Uuid::now_v7(), ids.org_id, ids.user_id, target)
        .execute(pool).await.unwrap();
    let grants: Vec<String> = serde_json::from_value(
        serde_json::to_value(PermissionSelection::new(&[Permission::TimeWriteManaged])).unwrap(),
    )
    .unwrap();
    crate::testing::clear_state(pool, ids.user_id).await;
    sqlx::query!("INSERT INTO person_permission_states (id,org_id,user_id,catalog_version,grants,is_administrator,source) VALUES ($1,$2,$3,1,$4,false,'individual')", Uuid::now_v7(), ids.org_id, ids.user_id, &grants)
        .execute(pool).await.unwrap();
    let context = TimesheetWriteContext {
        expected_requester: PermissionRequester {
            org_id: ids.org_id,
            user_id: ids.user_id,
        },
        subject_id: target,
    };
    Fixture { ids, context }
}

/// An actor without stored permission state is refused like everywhere else in
/// `ops`, not reported as an outage of the sheet.
#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn an_actor_without_stored_state_is_forbidden(pool: PgPool) {
    let f = fixture(&pool).await;
    crate::testing::clear_state(&pool, f.ids.user_id).await;

    let read = tracking(&pool, f.ids.org_id, f.ids.user_id, &f.context).await;
    let write = f
        .execute(&pool, TimesheetCommand::Create { entry: f.input() })
        .await;

    for result in [read.map(|_| ()), write] {
        assert!(matches!(result, Err(OpError::Forbidden(_))), "{result:?}");
    }
}

/// A start outside the day is invalid input, and nothing is written.
#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_start_outside_the_day_is_rejected_as_invalid(pool: PgPool) {
    let f = fixture(&pool).await;
    let entry = TimesheetEntryInput {
        start_minute: Some(1440),
        ..f.input()
    };

    let result = f.execute(&pool, TimesheetCommand::Create { entry }).await;

    assert!(
        matches!(
            &result,
            Err(OpError::Invalid { message, .. })
                if message == "start time must be within the day (0..=1439)"
        ),
        "{result:?}"
    );
    let entries = sqlx::query_scalar!(
        r#"SELECT count(*) AS "count!" FROM time_entries WHERE org_id=$1"#,
        f.ids.org_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(entries, 0);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn tracking_is_subject_eligible_and_actor_writable_without_financial_fields(pool: PgPool) {
    let f = fixture(&pool).await;
    let options = tracking(&pool, f.ids.org_id, f.ids.user_id, &f.context)
        .await
        .unwrap();
    assert_eq!(options.len(), 1);
    assert_eq!(
        (options[0].project_id, options[0].task_id),
        (f.ids.project_id, f.ids.task_id)
    );
    let json = serde_json::to_value(&options[0]).unwrap();
    assert_eq!(json.as_object().unwrap().len(), 5);
    assert_eq!(json["project_name"], "Widget");
    sqlx::query!(
        "DELETE FROM assignments WHERE user_id=$1",
        f.context.subject_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(
        tracking(&pool, f.ids.org_id, f.ids.user_id, &f.context)
            .await
            .unwrap()
            .is_empty()
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn delegated_commands_keep_owner_and_apply_each_ordinary_intent(pool: PgPool) {
    let f = fixture(&pool).await;
    f.execute(&pool, TimesheetCommand::Create { entry: f.input() })
        .await
        .unwrap();
    let created = sqlx::query!(
        "SELECT id,user_id,minutes,notes FROM time_entries WHERE org_id=$1",
        f.ids.org_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        (created.user_id, created.minutes),
        (f.context.subject_id, 45)
    );
    assert_eq!(created.notes.as_deref(), Some("Target work"));
    f.execute(
        &pool,
        TimesheetCommand::Update {
            entry_id: created.id,
            entry: TimesheetEntryInput {
                minutes: 75,
                ..f.input()
            },
        },
    )
    .await
    .unwrap();
    f.execute(
        &pool,
        TimesheetCommand::Reschedule {
            entry_id: created.id,
            spent_date: "2026-09-08".parse().unwrap(),
            start_minute: 600,
            minutes: 90,
        },
    )
    .await
    .unwrap();
    let moved = sqlx::query!(r#"SELECT user_id,minutes,start_minute,spent_date AS "spent_date: chrono::NaiveDate" FROM time_entries WHERE id=$1"#, created.id).fetch_one(&pool).await.unwrap();
    assert_eq!(moved.user_id, f.context.subject_id);
    assert_eq!(
        (moved.minutes, moved.start_minute, moved.spent_date),
        (90, Some(600), "2026-09-08".parse().unwrap())
    );
    let untimed = f.entry(&pool).await;
    f.execute(
        &pool,
        TimesheetCommand::Reorder {
            spent_date: "2026-09-09".parse().unwrap(),
            entry_ids: vec![untimed],
        },
    )
    .await
    .unwrap();
    f.execute(
        &pool,
        TimesheetCommand::StartTimer {
            project_id: f.ids.project_id,
            task_id: f.ids.task_id,
            notes: None,
        },
    )
    .await
    .unwrap();
    let timer = sqlx::query_scalar!(
        "SELECT id FROM time_entries WHERE user_id=$1 AND is_running",
        f.context.subject_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    f.execute(&pool, TimesheetCommand::StopTimer { entry_id: timer })
        .await
        .unwrap();
    assert!(
        !sqlx::query_scalar!("SELECT is_running FROM time_entries WHERE id=$1", timer)
            .fetch_one(&pool)
            .await
            .unwrap()
    );
    f.execute(
        &pool,
        TimesheetCommand::Delete {
            entry_ids: vec![created.id, untimed, timer],
        },
    )
    .await
    .unwrap();
    assert_eq!(
        sqlx::query_scalar!(
            "SELECT count(*) FROM time_entries WHERE org_id=$1",
            f.ids.org_id
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        Some(0)
    );
}
