use super::*;
use crate::server_fns::test_seed::{SeedIds, seed, time_entry};
use horae_core::types::EntryState;
use sqlx::PgPool;
use uuid::Uuid;

async fn extra_project(pool: &PgPool, ids: &SeedIds) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO projects (id, org_id, client_id, name, currency) \
         VALUES ($1, $2, $3, 'Widget', 'EUR')",
        id,
        ids.org_id,
        ids.client_id,
    )
    .execute(pool)
    .await
    .unwrap();
    id
}

async fn snapshot(pool: &PgPool, org_id: Uuid) -> serde_json::Value {
    sqlx::query_scalar!(
        "SELECT jsonb_agg(to_jsonb(p) ORDER BY id) FROM projects p WHERE org_id = $1",
        org_id,
    )
    .fetch_one(pool)
    .await
    .unwrap()
    .unwrap()
}

async fn billing_snapshot(pool: &PgPool) -> serde_json::Value {
    sqlx::query_scalar!(
        r#"SELECT jsonb_build_object(
          'entries', (SELECT jsonb_agg(to_jsonb(t)) FROM time_entries t),
          'invoices', (SELECT jsonb_agg(to_jsonb(t)) FROM invoices t),
          'lines', (SELECT jsonb_agg(to_jsonb(t)) FROM invoice_line_items t),
          'assignments', (SELECT jsonb_agg(to_jsonb(t)) FROM assignments t),
          'tasks', (SELECT jsonb_agg(to_jsonb(t)) FROM project_tasks t)
        ) as "snapshot!""#
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn bulk_preserves_invoiced_history_assignments_and_spend(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Admin).await;
    let entry = time_entry(&pool, &ids, EntryState::Open).await;
    sqlx::query!("INSERT INTO project_tasks (project_id, task_id, billable, rate_cents) VALUES ($1, $2, true, $3)", ids.project_id, ids.task_id, Some(6000_i64))
        .execute(&pool).await.unwrap();
    sqlx::query!("INSERT INTO assignments (id, project_id, user_id, role, rate_cents) VALUES ($1, $2, $3, 'lead', $4)", Uuid::now_v7(), ids.project_id, ids.user_id, Some(5000_i64))
        .execute(&pool).await.unwrap();
    let invoice_id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO invoices (id, org_id, client_id, number, status, issued_on, due_on, currency, total_cents)
         VALUES ($1, $2, $3, 'BULK-HISTORY', 'sent', '2026-09-07', '2026-10-07', 'EUR', 6000)",
        invoice_id, ids.org_id, ids.client_id,
    ).execute(&pool).await.unwrap();
    sqlx::query!(
        "INSERT INTO invoice_line_items (id, invoice_id, time_entry_id, description, minutes, rate_cents, amount_cents)
         VALUES ($1, $2, $3, 'Historical time', 60, 6000, 6000)",
        Uuid::now_v7(), invoice_id, entry,
    ).execute(&pool).await.unwrap();
    sqlx::query!(
        "UPDATE time_entries SET invoice_id = $2, state = 'invoiced' WHERE id = $1",
        entry,
        invoice_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let history = billing_snapshot(&pool).await;
    for active in [false, true] {
        set_projects_active_records(&pool, ids.org_id, &[ids.project_id], active)
            .await
            .unwrap();
        assert_eq!(billing_snapshot(&pool).await, history);
        assert_eq!(
            fetch_project_spend(&pool, ids.org_id, ids.user_id)
                .await
                .unwrap()[0]
                .spent_cents,
            6000
        );
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn bulk_accepts_one_hundred_projects_and_mixed_status_noops(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Admin).await;
    let mut targets = vec![ids.project_id];
    for _ in 1..100 {
        targets.push(extra_project(&pool, &ids).await);
    }
    targets.sort_unstable();
    set_project_active_record(&pool, ids.org_id, ids.project_id, false)
        .await
        .unwrap();
    let changed = set_projects_active_records(&pool, ids.org_id, &targets, false)
        .await
        .unwrap();
    assert_eq!(changed.len(), 100);
    assert_eq!(
        changed
            .iter()
            .filter(|(_, transition)| transition.is_some())
            .count(),
        99
    );
}

#[test]
fn bulk_ids_validate_before_writes_and_normalize_duplicates() {
    let id = Uuid::now_v7();
    assert_eq!(
        parse_bulk_project_ids(&[id.to_string(), id.to_string()]).unwrap(),
        vec![id]
    );
    for values in [vec![], vec!["invalid".into()], vec![id.to_string(); 101]] {
        assert!(matches!(
            parse_bulk_project_ids(&values),
            Err(ServerFnError::ServerError {
                code: BAD_REQUEST,
                ..
            })
        ));
    }
    let values: Vec<_> = (0..100).map(|_| Uuid::now_v7().to_string()).collect();
    assert_eq!(parse_bulk_project_ids(&values).unwrap().len(), 100);
}

#[sqlx::test(migrations = "./migrations")]
async fn bulk_status_preserves_details_history_and_noop_versions(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Admin).await;
    let other = extra_project(&pool, &ids).await;
    time_entry(&pool, &ids, EntryState::Open).await;
    let history = sqlx::query_scalar!("SELECT jsonb_agg(to_jsonb(t)) FROM time_entries t")
        .fetch_one(&pool)
        .await
        .unwrap();
    let before = snapshot(&pool, ids.org_id).await;
    let targets = [ids.project_id, other];
    for (active, expected) in [
        (false, crate::plugin::event::ActiveTransition::Deactivated),
        (true, crate::plugin::event::ActiveTransition::Reactivated),
    ] {
        let changed = set_projects_active_records(&pool, ids.org_id, &targets, active)
            .await
            .unwrap();
        assert_eq!(changed.len(), 2);
        assert!(
            changed
                .iter()
                .all(|(p, t)| p.active == active && *t == Some(expected))
        );
        let versions = sqlx::query_scalar!("SELECT xmin::text FROM projects ORDER BY id")
            .fetch_all(&pool)
            .await
            .unwrap();
        let repeated = set_projects_active_records(&pool, ids.org_id, &targets, active)
            .await
            .unwrap();
        assert!(repeated.iter().all(|(_, t)| t.is_none()));
        assert_eq!(
            sqlx::query_scalar!("SELECT xmin::text FROM projects ORDER BY id")
                .fetch_all(&pool)
                .await
                .unwrap(),
            versions
        );
    }
    let mut expected = before;
    for project in expected.as_array_mut().unwrap() {
        // Each real status transition invalidates an editor, but the repeated
        // no-op calls above must still leave the entire row version untouched.
        project["edit_revision"] = (project["edit_revision"].as_i64().unwrap() + 2).into();
    }
    assert_eq!(snapshot(&pool, ids.org_id).await, expected);
    assert_eq!(
        sqlx::query_scalar!("SELECT jsonb_agg(to_jsonb(t)) FROM time_entries t")
            .fetch_one(&pool)
            .await
            .unwrap(),
        history
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn bulk_missing_or_foreign_project_rolls_back_earlier_changes(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    let foreign = seed(&pool, OrgRole::Admin).await;
    let before = snapshot(&pool, ids.org_id).await;
    for last in [foreign.project_id, Uuid::max()] {
        let targets =
            parse_bulk_project_ids(&[ids.project_id.to_string(), last.to_string()]).unwrap();
        assert_eq!(targets[0], ids.project_id);
        let error = set_projects_active_records(&pool, ids.org_id, &targets, false)
            .await
            .unwrap_err();
        assert!(matches!(
            error,
            ServerFnError::ServerError {
                code: NOT_FOUND,
                ..
            }
        ));
        assert_eq!(snapshot(&pool, ids.org_id).await, before);
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn bulk_database_failure_rolls_back_earlier_changes(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Admin).await;
    let other = extra_project(&pool, &ids).await;
    // A rejecting constraint belongs only to this disposable test database.
    sqlx::query!(
        "ALTER TABLE projects ADD CONSTRAINT reject_archiving CHECK (active OR name <> 'Reject')"
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!("UPDATE projects SET name = 'Reject' WHERE id = $1", other)
        .execute(&pool)
        .await
        .unwrap();
    let before = snapshot(&pool, ids.org_id).await;
    assert!(
        set_projects_active_records(&pool, ids.org_id, &[ids.project_id, other], false)
            .await
            .is_err()
    );
    assert_eq!(snapshot(&pool, ids.org_id).await, before);
}

#[sqlx::test(migrations = "./migrations")]
async fn bulk_reversed_overlapping_batches_finish_without_deadlock(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Admin).await;
    let other = extra_project(&pool, &ids).await;
    let forward = parse_bulk_project_ids(&[ids.project_id.to_string(), other.to_string()]).unwrap();
    let reverse = parse_bulk_project_ids(&[other.to_string(), ids.project_id.to_string()]).unwrap();
    assert_eq!(forward, reverse);
    let mut blocker = pool.begin().await.unwrap();
    let pid = sqlx::query_scalar!("SELECT pg_backend_pid()")
        .fetch_one(&mut *blocker)
        .await
        .unwrap()
        .unwrap();
    sqlx::query!(
        "UPDATE projects SET name = 'Concurrent edit' WHERE id = $1",
        ids.project_id
    )
    .execute(&mut *blocker)
    .await
    .unwrap();
    let mut tasks = tokio::task::JoinSet::new();
    for order in [forward, reverse] {
        let db = pool.clone();
        tasks.spawn(
            async move { set_projects_active_records(&db, ids.org_id, &order, false).await },
        );
    }
    crate::server_fns::test_seed::wait_for_blocked(&pool, pid).await;
    blocker.commit().await.unwrap();
    let results = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        let mut transitions = 0;
        while let Some(result) = tasks.join_next().await {
            transitions += result
                .unwrap()
                .unwrap()
                .iter()
                .filter(|(_, t)| t.is_some())
                .count();
        }
        transitions
    })
    .await
    .unwrap();
    assert_eq!(results, 2);
    assert_eq!(
        sqlx::query_scalar!("SELECT name FROM projects WHERE id = $1", ids.project_id)
            .fetch_one(&pool)
            .await
            .unwrap(),
        "Concurrent edit"
    );
}
