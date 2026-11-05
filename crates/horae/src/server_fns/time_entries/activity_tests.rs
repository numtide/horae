use super::update_tests::editable_entry;
use super::*;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Clone, Copy, Debug)]
enum Operation {
    Create,
    Start,
    Stop,
}

async fn attempt(
    pool: &PgPool,
    actor: Uuid,
    entry: &TimeEntry,
    operation: Operation,
) -> Result<(), ServerFnError> {
    match operation {
        Operation::Create | Operation::Start => insert_time_entry(
            pool,
            actor,
            NewTimeEntry {
                project_id: entry.project_id,
                task_id: entry.task_id,
                spent_date: entry.spent_date,
                minutes: 15,
                notes: None,
                billable: true,
                start_minute: None,
                is_running: matches!(operation, Operation::Start),
            },
        )
        .await
        .map(|_| ()),
        Operation::Stop => stop_entry_timer(pool, actor, entry.id).await.map(|_| ()),
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn every_time_mutation_denies_inactive_and_missing_owners_without_changes(pool: PgPool) {
    for role in [OrgRole::Member, OrgRole::Manager, OrgRole::Admin] {
        for operation in [Operation::Create, Operation::Start, Operation::Stop] {
            let entry = editable_entry(&pool, true).await;
            sqlx::query!(
                "UPDATE time_entries SET start_minute=NULL, is_running=$2,
                started_at=CASE WHEN $2 THEN now() ELSE NULL END WHERE id=$1",
                entry.id,
                matches!(operation, Operation::Stop)
            )
            .execute(&pool)
            .await
            .unwrap();
            sqlx::query!(
                "UPDATE users SET active=false,org_role=$2 WHERE id=$1",
                entry.user_id,
                role as OrgRole
            )
            .execute(&pool)
            .await
            .unwrap();
            let before = sqlx::query_scalar!(
                "SELECT jsonb_agg(to_jsonb(e) ORDER BY e.id) FROM time_entries e WHERE user_id=$1",
                entry.user_id
            )
            .fetch_one(&pool)
            .await
            .unwrap();
            for actor in [entry.user_id, Uuid::now_v7()] {
                let result = attempt(&pool, actor, &entry, operation).await;
                assert!(
                    matches!(
                        result,
                        Err(ServerFnError::ServerError {
                            code: FORBIDDEN,
                            ..
                        })
                    ),
                    "{role:?} {operation:?}: {result:?}"
                );
                let after = sqlx::query_scalar!("SELECT jsonb_agg(to_jsonb(e) ORDER BY e.id) FROM time_entries e WHERE user_id=$1", entry.user_id)
                    .fetch_one(&pool).await.unwrap();
                assert_eq!(after, before);
            }
        }
    }
}
