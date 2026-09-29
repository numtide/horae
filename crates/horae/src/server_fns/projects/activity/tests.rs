use super::*;
use crate::server_fns::test_seed::{SeedIds, seed};
use serial_test::serial;
use sqlx::PgPool;
use uuid::Uuid;

fn interval(from: &str, to: &str) -> Option<ProjectActivityInterval> {
    Some(ProjectActivityInterval {
        from: from.parse().unwrap(),
        to: to.parse().unwrap(),
    })
}

async fn entry(pool: &PgPool, ids: &SeedIds, date: &str, minutes: i32, billable: bool) {
    sqlx::query!(
        "INSERT INTO time_entries (id, org_id, user_id, project_id, task_id, spent_date, minutes, billable, notes, rounded_minutes) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,'Private work note',999)",
        Uuid::now_v7(), ids.org_id, ids.user_id, ids.project_id, ids.task_id,
        date.parse::<chrono::NaiveDate>().unwrap() as chrono::NaiveDate, minutes, billable,
    ).execute(pool).await.unwrap();
}

fn assert_status(error: ServerFnError, expected: u16) {
    assert!(
        matches!(error, ServerFnError::ServerError { code, .. } if code == expected),
        "unexpected error: {error}"
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial]
async fn activity_uses_actual_minutes_in_inclusive_weeks_and_has_no_private_fields(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Admin).await;
    let foreign = seed(&pool, OrgRole::Admin).await;
    entry(&pool, &ids, "2026-09-01", 900, true).await;
    entry(&pool, &ids, "2026-09-02", 61, true).await;
    entry(&pool, &ids, "2026-09-02", 2, false).await;
    entry(&pool, &ids, "2026-09-15", 7, false).await;
    entry(&pool, &ids, "2026-09-16", 900, true).await;
    entry(&pool, &foreign, "2026-09-02", 900, true).await;
    let selected = interval("2026-09-02", "2026-09-15");
    let activity = fetch_project_activity(&pool, ids.org_id, ids.user_id, ids.project_id, selected)
        .await
        .unwrap();
    assert_eq!(activity.interval, selected);
    assert_eq!(activity.week_start, chrono::Weekday::Mon);
    assert_eq!(
        activity
            .weeks
            .iter()
            .map(|w| (
                w.billable_minutes,
                w.non_billable_minutes,
                w.cumulative_minutes
            ))
            .collect::<Vec<_>>(),
        vec![(61, 2, 63), (0, 0, 63), (0, 7, 70)]
    );
    let json = serde_json::to_value(&activity).unwrap();
    assert_eq!(json.as_object().unwrap().len(), 3);
    let keys = json["weeks"][0]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect::<Vec<_>>();
    assert_eq!(
        keys,
        [
            "billable_minutes",
            "cumulative_minutes",
            "from",
            "non_billable_minutes",
            "to"
        ]
    );
    assert!(!json.to_string().contains("Private work note"));
}

#[sqlx::test(migrations = "./migrations")]
#[serial]
async fn activity_resolves_all_time_and_workspace_week_start_without_writes(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Admin).await;
    sqlx::query!(
        "UPDATE organizations SET week_start = 7 WHERE id = $1",
        ids.org_id
    )
    .execute(&pool)
    .await
    .unwrap();
    entry(&pool, &ids, "2025-12-31", 1, true).await;
    entry(&pool, &ids, "2026-01-03", 2, false).await;
    entry(&pool, &ids, "2026-01-04", 4, true).await;
    sqlx::query!(
        "UPDATE projects SET active = false WHERE id = $1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let before = sqlx::query_scalar!(
        "SELECT xmin::text FROM projects WHERE id = $1",
        ids.project_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let activity = fetch_project_activity(&pool, ids.org_id, ids.user_id, ids.project_id, None)
        .await
        .unwrap();
    assert_eq!(activity.interval, interval("2025-12-31", "2026-01-04"));
    assert_eq!(activity.week_start, chrono::Weekday::Sun);
    assert_eq!(
        activity
            .weeks
            .iter()
            .map(|w| w.cumulative_minutes)
            .collect::<Vec<_>>(),
        [3, 7]
    );
    assert_eq!(
        sqlx::query_scalar!(
            "SELECT xmin::text FROM projects WHERE id = $1",
            ids.project_id
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        before
    );
    assert_eq!(
        sqlx::query_scalar!(
            "SELECT count(*) FROM time_entries WHERE project_id = $1",
            ids.project_id
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        Some(3)
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial]
async fn empty_all_time_has_no_invented_dates_but_selected_period_has_empty_weeks(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Admin).await;
    let empty = fetch_project_activity(&pool, ids.org_id, ids.user_id, ids.project_id, None)
        .await
        .unwrap();
    assert!(empty.interval.is_none());
    assert!(empty.weeks.is_empty());
    let selected = fetch_project_activity(
        &pool,
        ids.org_id,
        ids.user_id,
        ids.project_id,
        interval("2026-09-02", "2026-09-15"),
    )
    .await
    .unwrap();
    assert_eq!(selected.weeks.len(), 3);
    assert!(selected.weeks.iter().all(|w| w.cumulative_minutes == 0));
}

#[sqlx::test(migrations = "./migrations")]
#[serial]
async fn activity_requires_current_progress_permission_not_historical_identity(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Member).await;
    entry(&pool, &ids, "2026-09-02", 60, true).await;
    assert_status(
        fetch_project_activity(&pool, ids.org_id, ids.user_id, ids.project_id, None)
            .await
            .unwrap_err(),
        NOT_FOUND,
    );
    sqlx::query!(
        "INSERT INTO assignments (id,project_id,user_id) VALUES ($1,$2,$3)",
        Uuid::now_v7(),
        ids.project_id,
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(
        fetch_project_activity(&pool, ids.org_id, ids.user_id, ids.project_id, None)
            .await
            .is_ok()
    );
    sqlx::query!("INSERT INTO project_settings (id,org_id,project_id,creator_id,rate_mode) VALUES ($1,$2,$3,$4,'person')", Uuid::now_v7(), ids.org_id, ids.project_id, ids.user_id).execute(&pool).await.unwrap();
    assert_status(
        fetch_project_activity(&pool, ids.org_id, ids.user_id, ids.project_id, None)
            .await
            .unwrap_err(),
        NOT_FOUND,
    );
    sqlx::query!(
        "UPDATE project_settings SET report_visibility = 'project_members' WHERE project_id = $1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(
        fetch_project_activity(&pool, ids.org_id, ids.user_id, ids.project_id, None)
            .await
            .is_ok()
    );
    sqlx::query!(
        "UPDATE project_settings SET report_visibility = 'managers' WHERE project_id = $1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE assignments SET role = 'lead' WHERE project_id = $1 AND user_id = $2",
        ids.project_id,
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(
        fetch_project_activity(&pool, ids.org_id, ids.user_id, ids.project_id, None)
            .await
            .is_ok()
    );
    sqlx::query!(
        "UPDATE assignments SET role = 'admin' WHERE project_id = $1 AND user_id = $2",
        ids.project_id,
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(
        fetch_project_activity(&pool, ids.org_id, ids.user_id, ids.project_id, None)
            .await
            .is_ok()
    );
    sqlx::query!(
        "DELETE FROM assignments WHERE project_id = $1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert_status(
        fetch_project_activity(&pool, ids.org_id, ids.user_id, ids.project_id, None)
            .await
            .unwrap_err(),
        NOT_FOUND,
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial]
async fn activity_rechecks_organization_role_deactivation_and_foreign_scope(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Admin).await;
    let foreign = seed(&pool, OrgRole::Admin).await;
    for role in [OrgRole::Admin, OrgRole::Manager] {
        sqlx::query!(
            "UPDATE users SET org_role = $2 WHERE id = $1",
            ids.user_id,
            role as OrgRole
        )
        .execute(&pool)
        .await
        .unwrap();
        assert!(
            fetch_project_activity(&pool, ids.org_id, ids.user_id, ids.project_id, None)
                .await
                .is_ok()
        );
    }
    assert_status(
        fetch_project_activity(&pool, ids.org_id, ids.user_id, foreign.project_id, None)
            .await
            .unwrap_err(),
        NOT_FOUND,
    );
    assert_status(
        fetch_project_activity(&pool, foreign.org_id, ids.user_id, foreign.project_id, None)
            .await
            .unwrap_err(),
        NOT_FOUND,
    );
    sqlx::query!(
        "UPDATE users SET org_role = 'member' WHERE id = $1",
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert_status(
        fetch_project_activity(&pool, ids.org_id, ids.user_id, ids.project_id, None)
            .await
            .unwrap_err(),
        NOT_FOUND,
    );
    sqlx::query!(
        "UPDATE users SET org_role = 'admin', active = false WHERE id = $1",
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert_status(
        fetch_project_activity(&pool, ids.org_id, ids.user_id, ids.project_id, None)
            .await
            .unwrap_err(),
        NOT_FOUND,
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial]
async fn activity_rejects_reversed_and_excessive_ranges_after_authorization(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Admin).await;
    for selected in [
        interval("2026-09-02", "2026-09-01"),
        interval("1000-01-01", "9000-01-01"),
    ] {
        assert_status(
            fetch_project_activity(&pool, ids.org_id, ids.user_id, ids.project_id, selected)
                .await
                .unwrap_err(),
            BAD_REQUEST,
        );
        assert_status(
            fetch_project_activity(&pool, ids.org_id, ids.user_id, Uuid::now_v7(), selected)
                .await
                .unwrap_err(),
            NOT_FOUND,
        );
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial]
async fn activity_rejects_invalid_stored_week_start_even_without_entries(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Admin).await;
    for day in [0_i16, 8] {
        sqlx::query!(
            "UPDATE organizations SET week_start = $2 WHERE id = $1",
            ids.org_id,
            day
        )
        .execute(&pool)
        .await
        .unwrap();
        assert_status(
            fetch_project_activity(&pool, ids.org_id, ids.user_id, ids.project_id, None)
                .await
                .unwrap_err(),
            INTERNAL_ERROR,
        );
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial]
async fn activity_keeps_inactive_history_and_excludes_other_projects_in_same_org(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Admin).await;
    let other_project = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO projects (id,org_id,client_id,name,currency) VALUES ($1,$2,$3,'Other','EUR')",
        other_project,
        ids.org_id,
        ids.client_id
    )
    .execute(&pool)
    .await
    .unwrap();
    entry(&pool, &ids, "2026-09-02", 13, true).await;
    let other = SeedIds {
        project_id: other_project,
        ..ids
    };
    entry(&pool, &other, "2026-09-02", 999, false).await;
    sqlx::query!("UPDATE tasks SET active = false WHERE id = $1", ids.task_id)
        .execute(&pool)
        .await
        .unwrap();
    let former_user = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO users (id,org_id,email,name,active) VALUES ($1,$2,$3,'Former member',false)",
        former_user,
        ids.org_id,
        format!("{former_user}@test.com")
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE time_entries SET user_id = $2 WHERE project_id = $1",
        ids.project_id,
        former_user
    )
    .execute(&pool)
    .await
    .unwrap();
    let activity = fetch_project_activity(&pool, ids.org_id, ids.user_id, ids.project_id, None)
        .await
        .unwrap();
    assert_eq!(activity.weeks.len(), 1);
    assert_eq!(
        (
            activity.weeks[0].billable_minutes,
            activity.weeks[0].non_billable_minutes,
            activity.weeks[0].cumulative_minutes
        ),
        (13, 0, 13)
    );
}
