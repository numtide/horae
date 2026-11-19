//! The reads timer apps make right after sign-in (spec 021 US4): the company
//! settings and the person's project assignments.

use super::*;
use crate::testing::add_person;

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn the_company_reports_horaes_settings_and_features(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Member).await;
    sqlx::query!(
        "UPDATE organizations SET week_start = 7 WHERE id = $1",
        ids.org_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let app = signed_in(&pool, ids.user_id).await;

    for path in ["/harvest/v2/company", "/v2/company.json"] {
        let company = page(&app, path).await;
        assert_eq!(
            company,
            json!({
                "base_uri": null,
                "full_domain": null,
                "name": "Test Org",
                "is_active": true,
                "week_start_day": "Sunday",
                "wants_timestamp_timers": false,
                "time_format": "hours_minutes",
                "date_format": null,
                "plan_type": null,
                "clock": "24h",
                "currency_code_display": null,
                "currency_symbol_display": null,
                "decimal_symbol": null,
                "thousands_separator": null,
                "color_scheme": null,
                "weekly_capacity": null,
                "expense_feature": false,
                "invoice_feature": true,
                "estimate_feature": false,
                "approval_feature": true,
                "team_feature": true,
            }),
            "{path}"
        );
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn the_company_needs_an_authenticated_caller(pool: PgPool) {
    let app = router(pool, limiter());

    let (status, body) = request(&app, "/harvest/v2/company").await;

    assert_eq!(status, StatusCode::UNAUTHORIZED, "{body}");
}

/// Adds a second active task to the seeded project and assigns `person`.
async fn trackable(pool: &PgPool, ids: &crate::testing::SeedIds) -> Uuid {
    let other_task = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO tasks (id, org_id, name) VALUES ($1, $2, 'Review')",
        other_task,
        ids.org_id
    )
    .execute(pool)
    .await
    .unwrap();
    for (task, billable) in [(ids.task_id, true), (other_task, false)] {
        sqlx::query!(
            "INSERT INTO project_tasks (project_id, task_id, billable) VALUES ($1, $2, $3)",
            ids.project_id,
            task,
            billable
        )
        .execute(pool)
        .await
        .unwrap();
    }
    other_task
}

async fn assign(pool: &PgPool, ids: &crate::testing::SeedIds, project: Uuid) {
    sqlx::query!(
        "INSERT INTO assignments (id, project_id, user_id) VALUES ($1, $2, $3)",
        Uuid::now_v7(),
        project,
        ids.user_id
    )
    .execute(pool)
    .await
    .unwrap();
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_member_sees_the_projects_and_tasks_they_may_track(pool: PgPool) {
    let admin = seed(&pool, BuiltInProfile::Administrator).await;
    let member = add_person(&pool, &admin, BuiltInProfile::Member).await;
    let review = trackable(&pool, &admin).await;
    // A second project the member is not assigned to.
    let unassigned = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO projects (id, org_id, client_id, name, currency) VALUES ($1, $2, $3, 'Other', 'EUR')",
        unassigned,
        admin.org_id,
        admin.client_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO project_tasks (project_id, task_id, billable) VALUES ($1, $2, true)",
        unassigned,
        admin.task_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assign(&pool, &member, admin.project_id).await;
    let app = signed_in(&pool, member.user_id).await;

    let body = page(&app, "/harvest/v2/users/me/project_assignments").await;

    assert_eq!(body["total_entries"], 1, "{body}");
    let assignment = &body["project_assignments"][0];
    let project = api_id(&pool, "project", admin.project_id).await;
    assert_eq!(assignment["id"], project);
    assert_eq!(
        assignment["project"],
        json!({ "id": project, "name": "Widget", "code": null })
    );
    assert_eq!(
        assignment["client"],
        json!({
            "id": api_id(&pool, "client", admin.client_id).await,
            "name": "Acme",
            "currency": "EUR",
        })
    );
    assert_eq!(assignment["is_project_manager"], false);
    let tasks: Vec<(Value, Value, Value)> = assignment["task_assignments"]
        .as_array()
        .unwrap()
        .iter()
        .map(|task| {
            (
                task["id"].clone(),
                task["task"]["name"].clone(),
                task["billable"].clone(),
            )
        })
        .collect();
    assert_eq!(
        tasks,
        vec![
            (
                json!(api_id(&pool, "task", admin.task_id).await),
                json!("Dev"),
                json!(true)
            ),
            (
                json!(api_id(&pool, "task", review).await),
                json!("Review"),
                json!(false)
            ),
        ]
    );
    assert_eq!(
        assignment["task_assignments"][0]["task"]["id"],
        assignment["task_assignments"][0]["id"]
    );

    // An administrator tracks every active project, assigned or not.
    let admin_app = signed_in(&pool, admin.user_id).await;
    let body = page(&admin_app, "/harvest/v2/users/me/project_assignments").await;
    assert_eq!(body["total_entries"], 2, "{body}");
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn project_assignments_page_like_the_other_lists(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Administrator).await;
    trackable(&pool, &ids).await;
    for name in ["Beta", "Gamma"] {
        let project = Uuid::now_v7();
        sqlx::query!(
            "INSERT INTO projects (id, org_id, client_id, name, currency) VALUES ($1, $2, $3, $4, 'EUR')",
            project,
            ids.org_id,
            ids.client_id,
            name
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query!(
            "INSERT INTO project_tasks (project_id, task_id, billable) VALUES ($1, $2, true)",
            project,
            ids.task_id
        )
        .execute(&pool)
        .await
        .unwrap();
    }
    let app = signed_in(&pool, ids.user_id).await;

    let body = page(
        &app,
        "/harvest/v2/users/me/project_assignments?per_page=2&page=2",
    )
    .await;

    assert_eq!(
        (&body["total_entries"], &body["total_pages"], &body["page"]),
        (&json!(3), &json!(2), &json!(2))
    );
    let names: Vec<&Value> = body["project_assignments"]
        .as_array()
        .unwrap()
        .iter()
        .map(|assignment| &assignment["project"]["name"])
        .collect();
    assert_eq!(names, vec![&json!("Widget")]);
}
