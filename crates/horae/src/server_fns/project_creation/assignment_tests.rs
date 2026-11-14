use super::{finalize_draft_record, load_draft_record, save_draft_record};
use crate::models::project_creation::{
    ProjectForm, ProjectMemberInput, ProjectTaskInput, TaskAccess, TaskSource,
};
use crate::server_fns::test_seed::seed;
use dioxus::prelude::ServerFnError;
use horae_core::permissions::catalog::BuiltInProfile;
use horae_core::project::BudgetMode;
use horae_core::types::ProjectRole;
use sqlx::PgPool;
use uuid::Uuid;

async fn member(pool: &PgPool, org: Uuid, active: bool) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO users (id, org_id, email, name, active, billable_rate_cents, cost_rate_cents)
         VALUES ($1, $2, $3, 'Teammate', $4, 7000, 5000)",
        id,
        org,
        format!("{id}@test.com"),
        active,
    )
    .execute(pool)
    .await
    .unwrap();
    // Eligible for the manager designation that `member_input` asks for.
    let mut connection = pool.acquire().await.unwrap();
    crate::server_fns::provisioning::provision(
        &mut connection,
        org,
        id,
        BuiltInProfile::ProjectManager,
    )
    .await
    .unwrap();
    id
}

fn member_input(id: Uuid) -> ProjectMemberInput {
    ProjectMemberInput {
        user_id: id,
        manager: true,
        billable_rate: "0".into(),
        cost_rate: "62.25".into(),
        budget: "7:30".into(),
    }
}

fn task_input(source: TaskSource) -> ProjectTaskInput {
    ProjectTaskInput {
        id: Uuid::now_v7(),
        source,
        billable: true,
        rate: String::new(),
        budget: String::new(),
        access: TaskAccess::Everyone,
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn finalization_reuses_catalog_and_keeps_lead_overrides_local_on_retry(pool: PgPool) {
    let owner = seed(&pool, BuiltInProfile::Administrator).await;
    let teammate = member(&pool, owner.org_id, true).await;
    sqlx::query!(
        "INSERT INTO assignments (id, project_id, user_id, role, rate_cents) VALUES ($1, $2, $3, 'freelancer', 8000)",
        Uuid::now_v7(), owner.project_id, teammate,
    ).execute(&pool).await.unwrap();
    let before = sqlx::query!(
        "SELECT to_jsonb(u) AS profile, to_jsonb(a) AS assignment, to_jsonb(t) AS task
         FROM users u JOIN assignments a ON a.user_id = u.id
         CROSS JOIN tasks t WHERE u.id = $1 AND a.project_id = $2 AND t.id = $3",
        teammate,
        owner.project_id,
        owner.task_id,
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let mut reused = task_input(TaskSource::New {
        name: " dev ".into(),
    });
    reused.billable = false;
    reused.access = TaskAccess::Restricted {
        user_ids: vec![teammate, teammate],
    };
    let mut fresh = task_input(TaskSource::New {
        name: " Fresh task ".into(),
    });
    fresh.access = TaskAccess::Restricted { user_ids: vec![] };
    let form = ProjectForm {
        client_id: Some(owner.client_id),
        name: "Assignments stay local".into(),
        budget_mode: BudgetMode::HoursPerPerson,
        team: vec![member_input(teammate)],
        tasks: vec![reused, fresh],
        ..Default::default()
    };
    let draft_id = Uuid::now_v7();
    save_draft_record(&pool, owner.user_id, owner.org_id, draft_id, 0, &form)
        .await
        .unwrap();
    let (first, retry) = tokio::join!(
        finalize_draft_record(
            &pool,
            owner.user_id,
            owner.org_id,
            draft_id,
            1,
            &form,
            false
        ),
        finalize_draft_record(
            &pool,
            owner.user_id,
            owner.org_id,
            draft_id,
            1,
            &form,
            false
        ),
    );
    let project = first.unwrap();
    assert_eq!(retry.unwrap(), project);
    let assignments = sqlx::query!(
        r#"SELECT a.user_id, a.role AS "role: ProjectRole", a.rate_cents,
                  c.cost_rate_cents, b.budget_minutes
           FROM assignments a
           JOIN project_member_costs c USING (project_id, user_id)
           JOIN project_member_budgets b USING (project_id, user_id)
           WHERE a.project_id = $1"#,
        project,
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(assignments.len(), 1);
    let assignment = &assignments[0];
    assert_eq!(
        (
            assignment.user_id,
            assignment.role,
            assignment.rate_cents,
            assignment.cost_rate_cents,
            assignment.budget_minutes
        ),
        (teammate, ProjectRole::Lead, Some(0), 6225, 450),
    );
    // The retry returned the first result instead of designating again.
    assert_eq!(designated(&pool, project).await, [teammate]);
    let tasks = sqlx::query!(
        "SELECT t.id, t.name, p.billable, s.restricted,
                (SELECT count(*) FROM project_task_members m WHERE m.project_id = p.project_id AND m.task_id = t.id) AS grants
         FROM project_tasks p JOIN tasks t ON t.id = p.task_id
         JOIN project_task_settings s USING (project_id, task_id)
         WHERE p.project_id = $1 ORDER BY t.name",
        project,
    ).fetch_all(&pool).await.unwrap();
    assert_eq!(tasks.len(), 2);
    assert_eq!(
        (
            tasks[0].id,
            tasks[0].name.as_str(),
            tasks[0].billable,
            tasks[0].restricted,
            tasks[0].grants
        ),
        (owner.task_id, "Dev", false, true, Some(1))
    );
    assert_eq!(
        (
            tasks[1].name.as_str(),
            tasks[1].billable,
            tasks[1].restricted,
            tasks[1].grants
        ),
        ("Fresh task", true, true, Some(0))
    );
    assert_eq!(tasks[1].id.get_version_num(), 7);
    let after = sqlx::query!(
        "SELECT to_jsonb(u) AS profile, to_jsonb(a) AS assignment, to_jsonb(t) AS task
         FROM users u JOIN assignments a ON a.user_id = u.id
         CROSS JOIN tasks t WHERE u.id = $1 AND a.project_id = $2 AND t.id = $3",
        teammate,
        owner.project_id,
        owner.task_id,
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        (after.profile, after.assignment, after.task),
        (before.profile, before.assignment, before.task)
    );
    assert_eq!(creation_counts(&pool, owner.org_id).await, [2, 2, 0, 1]);
}

async fn designated(pool: &PgPool, project: Uuid) -> Vec<Uuid> {
    sqlx::query_scalar!(
        "SELECT manager_id FROM project_management_assignments WHERE project_id = $1 ORDER BY manager_id",
        project,
    )
    .fetch_all(pool)
    .await
    .unwrap()
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn finalization_designates_eligible_managers_like_the_editor(pool: PgPool) {
    use crate::server_fns::permissions::project_management;
    // Project Manager creates projects but edits only the ones it manages.
    let creator = seed(&pool, BuiltInProfile::ProjectManager).await;
    let teammate = member(&pool, creator.org_id, true).await;
    let plain = member(&pool, creator.org_id, true).await;
    crate::server_fns::test_seed::clear_state(&pool, plain).await;
    let mut connection = pool.acquire().await.unwrap();
    crate::server_fns::provisioning::provision(
        &mut connection,
        creator.org_id,
        plain,
        BuiltInProfile::Member,
    )
    .await
    .unwrap();
    drop(connection);
    let person = |user_id, manager| ProjectMemberInput {
        user_id,
        manager,
        billable_rate: String::new(),
        cost_rate: String::new(),
        budget: String::new(),
    };
    let mut form = ProjectForm {
        client_id: Some(creator.client_id),
        name: "Managed from the start".into(),
        team: vec![person(creator.user_id, true), person(plain, true)],
        ..Default::default()
    };
    let draft_id = Uuid::now_v7();
    save_draft_record(&pool, creator.user_id, creator.org_id, draft_id, 0, &form)
        .await
        .unwrap();
    let counts = creation_counts(&pool, creator.org_id).await;
    let revision = || async {
        sqlx::query_scalar!(
            "SELECT access_revision FROM organizations WHERE id = $1",
            creator.org_id
        )
        .fetch_one(&pool)
        .await
        .unwrap()
    };
    let before = revision().await;
    // A Member cannot manage projects, so the whole creation is refused.
    assert!(matches!(
        finalize_draft_record(
            &pool,
            creator.user_id,
            creator.org_id,
            draft_id,
            1,
            &form,
            false
        )
        .await,
        Err(ServerFnError::ServerError {
            code: crate::server_fns::BAD_REQUEST,
            ..
        })
    ));
    assert_eq!(creation_counts(&pool, creator.org_id).await, counts);
    assert_eq!(revision().await, before);

    form.team = vec![
        person(creator.user_id, true),
        person(teammate, true),
        person(plain, false),
    ];
    let project = finalize_draft_record(
        &pool,
        creator.user_id,
        creator.org_id,
        draft_id,
        1,
        &form,
        false,
    )
    .await
    .unwrap();
    let mut expected = vec![creator.user_id, teammate];
    expected.sort_unstable();
    assert_eq!(designated(&pool, project).await, expected);
    assert_eq!(revision().await, before + 1);
    // The designation is what lets the creator open the project editor.
    let managers = project_management::read(&pool, creator.org_id, creator.user_id, project)
        .await
        .unwrap();
    assert_eq!(
        managers
            .managers
            .iter()
            .map(|manager| manager.id)
            .collect::<Vec<_>>(),
        expected
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn creators_without_project_write_all_manage_what_they_create(pool: PgPool) {
    use crate::server_fns::permissions::project_management;
    for (profile, designates) in [
        (BuiltInProfile::ProjectManager, true),
        (BuiltInProfile::ExecutiveManager, false),
        (BuiltInProfile::Administrator, false),
    ] {
        let creator = seed(&pool, profile).await;
        // Nobody is on the team, let alone marked as manager.
        let form = ProjectForm {
            client_id: Some(creator.client_id),
            name: "Created without a team".into(),
            ..Default::default()
        };
        let draft_id = Uuid::now_v7();
        save_draft_record(&pool, creator.user_id, creator.org_id, draft_id, 0, &form)
            .await
            .unwrap();
        let before = sqlx::query_scalar!(
            "SELECT access_revision FROM organizations WHERE id = $1",
            creator.org_id
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        let project = finalize_draft_record(
            &pool,
            creator.user_id,
            creator.org_id,
            draft_id,
            1,
            &form,
            false,
        )
        .await
        .unwrap();
        let after = sqlx::query_scalar!(
            "SELECT access_revision FROM organizations WHERE id = $1",
            creator.org_id
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        if designates {
            assert_eq!(designated(&pool, project).await, [creator.user_id]);
            assert_eq!(after, before + 1);
        } else {
            assert!(designated(&pool, project).await.is_empty(), "{profile:?}");
            assert_eq!(after, before, "{profile:?}");
        }
        // Every creator can open the editor of what they created.
        project_management::read(&pool, creator.org_id, creator.user_id, project)
            .await
            .unwrap();
    }
}

async fn creation_counts(pool: &PgPool, org: Uuid) -> Vec<i64> {
    sqlx::query_scalar!(
        r#"SELECT ARRAY[
            (SELECT count(*) FROM projects WHERE org_id = $1),
            (SELECT count(*) FROM tasks WHERE org_id = $1),
            (SELECT count(*) FROM project_tags WHERE org_id = $1),
            (SELECT count(*) FROM horae_outbox WHERE org_id = $1 AND event_kind = 'project_created')
        ] AS "counts!""#,
        org,
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn invalid_assignment_graphs_leave_no_partial_creation_or_draft_change(pool: PgPool) {
    use crate::server_fns::{BAD_REQUEST, CONFLICT, NOT_FOUND};
    let owner = seed(&pool, BuiltInProfile::Administrator).await;
    let foreign = seed(&pool, BuiltInProfile::Administrator).await;
    let teammate = member(&pool, owner.org_id, true).await;
    let inactive = member(&pool, owner.org_id, false).await;
    let archived_task = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO tasks (id, org_id, name, active) VALUES ($1, $2, 'Archived', false)",
        archived_task,
        owner.org_id,
    )
    .execute(&pool)
    .await
    .unwrap();
    let form = ProjectForm {
        client_id: Some(owner.client_id),
        name: "Nothing partial".into(),
        tags: vec!["Pending tag".into()],
        admin_notes: "Private notes".into(),
        budget_mode: BudgetMode::HoursPerPerson,
        team: vec![member_input(teammate)],
        tasks: vec![
            task_input(TaskSource::New {
                name: "Created before error".into(),
            }),
            task_input(TaskSource::Existing {
                task_id: owner.task_id,
            }),
        ],
        ..Default::default()
    };
    let draft_id = Uuid::now_v7();
    save_draft_record(&pool, owner.user_id, owner.org_id, draft_id, 0, &form)
        .await
        .unwrap();
    let before = load_draft_record(&pool, owner.user_id, owner.org_id)
        .await
        .unwrap();
    let counts = creation_counts(&pool, owner.org_id).await;
    for (case, expected_code) in [
        ("foreign client", NOT_FOUND),
        ("missing client", NOT_FOUND),
        ("duplicate person", BAD_REQUEST),
        ("foreign person", NOT_FOUND),
        ("inactive person", NOT_FOUND),
        ("missing person", NOT_FOUND),
        ("duplicate task row", BAD_REQUEST),
        ("duplicate catalog task", BAD_REQUEST),
        ("existing plus named task", BAD_REQUEST),
        ("duplicate named task", BAD_REQUEST),
        ("foreign task", NOT_FOUND),
        ("inactive task", NOT_FOUND),
        ("missing task", NOT_FOUND),
        ("archived named task", CONFLICT),
        ("unselected restriction", BAD_REQUEST),
    ] {
        let mut invalid = form.clone();
        match case {
            "foreign client" => invalid.client_id = Some(foreign.client_id),
            "missing client" => invalid.client_id = Some(Uuid::now_v7()),
            "duplicate person" => invalid.team.push(member_input(teammate)),
            "foreign person" => invalid.team.push(member_input(foreign.user_id)),
            "inactive person" => invalid.team.push(member_input(inactive)),
            "missing person" => invalid.team.push(member_input(Uuid::now_v7())),
            "duplicate task row" => invalid.tasks.push(invalid.tasks[1].clone()),
            "duplicate catalog task" => invalid.tasks.push(task_input(TaskSource::Existing {
                task_id: owner.task_id,
            })),
            "existing plus named task" => invalid.tasks.push(task_input(TaskSource::New {
                name: " dev ".into(),
            })),
            "duplicate named task" => invalid.tasks.push(task_input(TaskSource::New {
                name: " CREATED BEFORE ERROR ".into(),
            })),
            "foreign task" => {
                invalid.tasks[1].source = TaskSource::Existing {
                    task_id: foreign.task_id,
                }
            }
            "inactive task" => {
                invalid.tasks[1].source = TaskSource::Existing {
                    task_id: archived_task,
                }
            }
            "missing task" => {
                invalid.tasks[1].source = TaskSource::Existing {
                    task_id: Uuid::now_v7(),
                }
            }
            "archived named task" => {
                invalid.tasks[1].source = TaskSource::New {
                    name: " archived ".into(),
                }
            }
            "unselected restriction" => {
                invalid.tasks[1].access = TaskAccess::Restricted {
                    user_ids: vec![owner.user_id],
                }
            }
            _ => unreachable!(),
        }
        let error = finalize_draft_record(
            &pool,
            owner.user_id,
            owner.org_id,
            draft_id,
            1,
            &invalid,
            false,
        )
        .await
        .unwrap_err();
        assert!(
            matches!(error, ServerFnError::ServerError { code, .. } if code == expected_code),
            "{case}: {error:?}"
        );
        let expected_field = match case {
            "foreign client" | "missing client" => Some(serde_json::json!("client")),
            "foreign person" | "inactive person" | "missing person" => {
                Some(serde_json::json!({"person": invalid.team.last().unwrap().user_id}))
            }
            "foreign task" | "inactive task" | "missing task" => {
                Some(serde_json::json!({"task": invalid.tasks[1].id}))
            }
            "archived named task" => Some(serde_json::json!({"task_name": invalid.tasks[1].id})),
            _ => None,
        };
        if let Some(expected) = expected_field {
            let ServerFnError::ServerError { details, .. } = &error else {
                unreachable!()
            };
            assert_eq!(
                details.as_ref().and_then(|value| value.get("field")),
                Some(&expected),
                "{case}"
            );
        }
        assert_eq!(creation_counts(&pool, owner.org_id).await, counts, "{case}");
        assert_eq!(
            load_draft_record(&pool, owner.user_id, owner.org_id)
                .await
                .unwrap(),
            before,
            "{case}"
        );
    }
    assert!(
        finalize_draft_record(
            &pool,
            owner.user_id,
            owner.org_id,
            draft_id,
            1,
            &form,
            false
        )
        .await
        .is_ok()
    );
}
