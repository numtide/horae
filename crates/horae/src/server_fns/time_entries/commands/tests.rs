use super::*;
use crate::models::permission_editor::PermissionRequester;
use crate::models::scoped_time::{TimesheetEntryInput, TimesheetPolicy};
use crate::server_fns::test_seed::{SeedIds, seed, time_entry};
use horae_core::permissions::catalog::{Permission, PermissionSelection};

mod guards;
mod revocation;

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

    async fn execute(&self, pool: &PgPool, command: TimesheetCommand) -> Result<(), ServerFnError> {
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
    let ids = seed(pool, OrgRole::Manager).await;
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
    sqlx::query!("INSERT INTO person_permission_states (id,org_id,user_id,catalog_version,grants,is_administrator,source) VALUES ($1,$2,$3,1,$4,false,'individual')", Uuid::now_v7(), ids.org_id, ids.user_id, &grants)
        .execute(pool).await.unwrap();
    sqlx::query!(
        "UPDATE organizations SET permission_policy_version=1 WHERE id=$1",
        ids.org_id
    )
    .execute(pool)
    .await
    .unwrap();
    let context = TimesheetWriteContext {
        expected_requester: PermissionRequester {
            org_id: ids.org_id,
            user_id: ids.user_id,
        },
        subject_id: target,
        expected_policy: TimesheetPolicy::Scoped,
    };
    Fixture { ids, context }
}

#[sqlx::test(migrations = "./migrations")]
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

#[sqlx::test(migrations = "./migrations")]
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
