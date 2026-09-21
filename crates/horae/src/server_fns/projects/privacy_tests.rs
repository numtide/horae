use super::*;
use crate::server_fns::test_seed::{SeedIds, seed, time_entry};
use sqlx::PgPool;
use uuid::Uuid;

async fn fixture(pool: &PgPool) -> (SeedIds, User) {
    let ids = seed(pool, OrgRole::Member).await;
    sqlx::query!(
        "INSERT INTO assignments (id, project_id, user_id) VALUES ($1,$2,$3)",
        Uuid::now_v7(),
        ids.project_id,
        ids.user_id
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO project_tasks (project_id, task_id, billable) VALUES ($1,$2,true)",
        ids.project_id,
        ids.task_id
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO project_settings (id, org_id, project_id, creator_id, rate_mode)
        VALUES ($1,$2,$3,$4,'project')",
        Uuid::now_v7(),
        ids.org_id,
        ids.project_id,
        ids.user_id
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query!("UPDATE projects SET rate_cents = 12345, budget_kind = 'hours', budget_minutes = 600 WHERE id = $1",
        ids.project_id).execute(pool).await.unwrap();
    sqlx::query!(
        "UPDATE tasks SET default_rate_cents = 999 WHERE id = $1",
        ids.task_id
    )
    .execute(pool)
    .await
    .unwrap();
    time_entry(pool, &ids, EntryState::Open).await;
    let viewer = User {
        id: ids.user_id,
        org_id: ids.org_id,
        email: format!("{}@test.com", ids.user_id),
        name: "Viewer".into(),
        oidc_subject: None,
        org_role: OrgRole::Member,
        cost_rate_cents: None,
        billable_rate_cents: None,
        active: true,
        created_at: chrono::Utc::now(),
    };
    (ids, viewer)
}

#[sqlx::test(migrations = "./migrations")]
async fn private_progress_does_not_prevent_tracking_or_reveal_financial_fields(pool: PgPool) {
    let (ids, viewer) = fixture(&pool).await;
    assert!(
        projects_for_viewer(&pool, &viewer, None, true, ProjectRead::Overview)
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        fetch_project_spend(&pool, viewer.org_id, viewer.id)
            .await
            .unwrap()
            .is_empty()
    );
    let tracking = projects_for_viewer(&pool, &viewer, None, true, ProjectRead::Tracking)
        .await
        .unwrap();
    assert_eq!(tracking.len(), 1);
    assert_eq!(tracking[0].id, ids.project_id);
    let payload = serde_json::to_value(&tracking[0]).unwrap();
    for field in [
        "rate_cents",
        "budget_minutes",
        "budget_amount_cents",
        "admin_notes",
        "cost_rate_cents",
    ] {
        assert!(
            payload.get(field).is_none(),
            "unexpected field {field}: {payload}"
        );
    }
    let decoded: Project = serde_json::from_value(payload).unwrap();
    assert_eq!(decoded.id, ids.project_id);
    assert!(decoded.rate_cents.is_none());
    let tasks = tasks_for_viewer(&pool, &viewer, None, TaskRead::Tracking)
        .await
        .unwrap();
    assert_eq!(tasks.len(), 1);
    assert!(
        serde_json::to_value(&tasks[0])
            .unwrap()
            .get("default_rate_cents")
            .is_none()
    );
    let decoded: Task = serde_json::from_value(serde_json::to_value(&tasks[0]).unwrap()).unwrap();
    assert_eq!(decoded.id, ids.task_id);
    assert!(decoded.default_rate_cents.is_none());
    let team = assignments_for_viewer(&pool, &viewer, ids.project_id)
        .await
        .unwrap();
    let payload = serde_json::to_value(&team[0]).unwrap();
    assert!(payload.get("rate_cents").is_none());
    let decoded: Assignment = serde_json::from_value(payload).unwrap();
    assert_eq!(decoded.user_id, ids.user_id);
}

#[sqlx::test(migrations = "./migrations")]
async fn legacy_members_retain_progress_without_new_settings(pool: PgPool) {
    let (ids, viewer) = fixture(&pool).await;
    sqlx::query!(
        "DELETE FROM project_settings WHERE project_id = $1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let projects = projects_for_viewer(&pool, &viewer, None, true, ProjectRead::Overview)
        .await
        .unwrap();
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0].budget_minutes, Some(600));
    assert!(projects[0].rate_cents.is_none());
    let totals = fetch_project_spend(&pool, viewer.org_id, viewer.id)
        .await
        .unwrap();
    assert_eq!(
        (totals[0].spent_minutes, totals[0].spent_cents),
        (60, 12345)
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn project_members_and_leads_receive_progress_without_rates(pool: PgPool) {
    let (ids, viewer) = fixture(&pool).await;
    for role in [
        ProjectRole::Freelancer,
        ProjectRole::Lead,
        ProjectRole::Admin,
    ] {
        sqlx::query!(
            "UPDATE project_settings SET report_visibility = $2 WHERE project_id = $1",
            ids.project_id,
            if role == ProjectRole::Freelancer {
                "project_members"
            } else {
                "managers"
            }
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query!(
            "UPDATE assignments SET role = $2 WHERE project_id = $1 AND user_id = $3",
            ids.project_id,
            role as ProjectRole,
            ids.user_id
        )
        .execute(&pool)
        .await
        .unwrap();
        let projects = projects_for_viewer(&pool, &viewer, None, true, ProjectRead::Overview)
            .await
            .unwrap();
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].budget_minutes, Some(600));
        assert_eq!(projects[0].rate_cents, None);
        let progress = fetch_project_spend(&pool, viewer.org_id, viewer.id)
            .await
            .unwrap();
        assert_eq!(
            (progress[0].spent_minutes, progress[0].spent_cents),
            (60, 12345)
        );
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn revoked_membership_preserves_only_own_historical_tracking_identity(pool: PgPool) {
    let (ids, viewer) = fixture(&pool).await;
    sqlx::query!(
        "DELETE FROM assignments WHERE project_id = $1 AND user_id = $2",
        ids.project_id,
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE projects SET active = false WHERE id = $1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!("UPDATE tasks SET active = false WHERE id = $1", ids.task_id)
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        projects_for_viewer(&pool, &viewer, None, true, ProjectRead::Overview)
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        assignments_for_viewer(&pool, &viewer, ids.project_id)
            .await
            .unwrap()
            .is_empty()
    );
    let tracking = projects_for_viewer(&pool, &viewer, None, true, ProjectRead::Tracking)
        .await
        .unwrap();
    assert_eq!(tracking[0].id, ids.project_id);
    assert!(tracking[0].rate_cents.is_none());
    assert!(
        tasks_for_viewer(&pool, &viewer, None, TaskRead::Catalog)
            .await
            .unwrap()
            .is_empty()
    );
    let tasks = tasks_for_viewer(&pool, &viewer, None, TaskRead::Tracking)
        .await
        .unwrap();
    assert_eq!(tasks[0].id, ids.task_id);
    assert!(tasks[0].default_rate_cents.is_none());
}

#[sqlx::test(migrations = "./migrations")]
async fn project_reads_use_current_authority_and_reject_foreign_scope(pool: PgPool) {
    let (ids, mut viewer) = fixture(&pool).await;
    viewer.org_role = OrgRole::Admin;
    assert!(
        projects_for_viewer(&pool, &viewer, None, true, ProjectRead::Overview)
            .await
            .unwrap()
            .is_empty()
    );
    for role in [OrgRole::Manager, OrgRole::Admin] {
        sqlx::query!(
            "UPDATE users SET org_role = $2 WHERE id = $1",
            viewer.id,
            role as OrgRole
        )
        .execute(&pool)
        .await
        .unwrap();
        let rows = projects_for_viewer(&pool, &viewer, None, true, ProjectRead::Overview)
            .await
            .unwrap();
        assert_eq!(rows[0].rate_cents, Some(12345));
        assert_eq!(
            tasks_for_viewer(&pool, &viewer, Some(ids.project_id), TaskRead::Catalog)
                .await
                .unwrap()[0]
                .default_rate_cents,
            Some(999)
        );
        let tracking_tasks = tasks_for_viewer(&pool, &viewer, None, TaskRead::Tracking)
            .await
            .unwrap();
        assert_eq!(tracking_tasks[0].default_rate_cents, None);
        let matching = projects_for_viewer(
            &pool,
            &viewer,
            Some(ids.client_id),
            true,
            ProjectRead::Overview,
        )
        .await
        .unwrap();
        assert_eq!(matching.len(), 1);
        assert!(
            projects_for_viewer(
                &pool,
                &viewer,
                Some(Uuid::now_v7()),
                true,
                ProjectRead::Overview
            )
            .await
            .unwrap()
            .is_empty()
        );
    }
    let foreign = seed(&pool, OrgRole::Admin).await;
    viewer.org_id = foreign.org_id;
    assert!(
        projects_for_viewer(&pool, &viewer, None, true, ProjectRead::Overview)
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        tasks_for_viewer(&pool, &viewer, Some(ids.project_id), TaskRead::Catalog)
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        fetch_project_spend(&pool, viewer.org_id, viewer.id)
            .await
            .unwrap()
            .is_empty()
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn outsiders_and_inactive_viewers_have_no_project_reads(pool: PgPool) {
    let (ids, viewer) = fixture(&pool).await;
    // Remove both independent identity grants, then restore the assignment for
    // the inactive-user case: deactivation must override an existing grant.
    sqlx::query!(
        "DELETE FROM time_entries WHERE org_id = $1 AND user_id = $2",
        ids.org_id,
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "DELETE FROM assignments WHERE project_id = $1 AND user_id = $2",
        ids.project_id,
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    for inactive in [false, true] {
        if inactive {
            sqlx::query!(
                "INSERT INTO assignments (id, project_id, user_id) VALUES ($1,$2,$3)",
                Uuid::now_v7(),
                ids.project_id,
                ids.user_id
            )
            .execute(&pool)
            .await
            .unwrap();
            sqlx::query!(
                "UPDATE users SET active = false, org_role = 'admin' WHERE id = $1",
                ids.user_id
            )
            .execute(&pool)
            .await
            .unwrap();
        }
        for purpose in [ProjectRead::Overview, ProjectRead::Tracking] {
            assert!(
                projects_for_viewer(&pool, &viewer, None, true, purpose)
                    .await
                    .unwrap()
                    .is_empty()
            );
        }
        for purpose in [TaskRead::Catalog, TaskRead::Tracking] {
            assert!(
                tasks_for_viewer(&pool, &viewer, None, purpose)
                    .await
                    .unwrap()
                    .is_empty()
            );
        }
        assert!(
            assignments_for_viewer(&pool, &viewer, ids.project_id)
                .await
                .unwrap()
                .is_empty()
        );
        assert!(
            fetch_project_spend(&pool, viewer.org_id, viewer.id)
                .await
                .unwrap()
                .is_empty()
        );
    }
}
