use super::*;
use crate::models::project_creation::{ProjectFieldAccess, ProtectedProjectField};
use crate::server_fns::test_seed::SeedIds;
use horae_core::permissions::catalog::{BuiltInProfile, Permission, PermissionSelection};

pub(super) async fn canonical_actor(pool: &PgPool, owner: &SeedIds, grants: &PermissionSelection) {
    let grants: Vec<String> =
        serde_json::from_value(serde_json::to_value(grants).unwrap()).unwrap();
    sqlx::query!(
        "INSERT INTO person_permission_states
         (id, org_id, user_id, catalog_version, grants, is_administrator, source)
         VALUES ($1, $2, $3, 1, $4, false, 'individual')",
        Uuid::now_v7(),
        owner.org_id,
        owner.user_id,
        &grants,
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE organizations SET permission_policy_version = 1 WHERE id = $1",
        owner.org_id,
    )
    .execute(pool)
    .await
    .unwrap();
}

pub(super) fn keep_protected(form: &ProjectForm) -> Vec<ProtectedProjectField> {
    use ProtectedProjectField::*;
    let mut fields = vec![ProjectRate, Budget, Fees, InvoiceDefaults, PrivateNotes];
    fields.extend(form.tasks.iter().map(|task| TaskRate(task.id)));
    for member in &form.team {
        fields.extend([PersonRate(member.user_id), CostRate(member.user_id)]);
    }
    fields
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn project_editor_keeps_inactive_member_budgets_on_name_save_and_replay(pool: PgPool) {
    let (owner, original) = configured_fixture(&pool).await;
    assert_eq!(original.form.budget_mode, BudgetMode::None);
    let budget_id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO project_member_budgets (id,org_id,project_id,user_id,budget_minutes)
         VALUES ($1,$2,$3,$4,125)",
        budget_id,
        owner.org_id,
        original.id,
        owner.user_id,
    )
    .execute(&pool)
    .await
    .unwrap();
    canonical_actor(
        &pool,
        &owner,
        &PermissionSelection::new(&[Permission::ProjectWriteAll]),
    )
    .await;
    let editor = load_editable_project(&pool, owner.user_id, owner.org_id, original.id)
        .await
        .unwrap();
    let mut request = edit_request(editor);
    request.form.name = "Preserve inactive member budget".into();
    request.unchanged = keep_protected(&request.form);
    for expected_change in [true, false] {
        let (project, changed) =
            save_editable_project(&pool, owner.user_id, owner.org_id, &request, false)
                .await
                .unwrap();
        assert_eq!(changed, expected_change);
        assert_eq!(project.name, request.form.name);
        let budget = sqlx::query_scalar!(
            "SELECT budget_minutes FROM project_member_budgets WHERE id=$1",
            budget_id,
        )
        .fetch_optional(&pool)
        .await
        .unwrap();
        assert_eq!(
            budget,
            Some(125),
            "keeping Budget must preserve retained inactive member rows"
        );
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn project_editor_explicit_hour_budget_change_clears_inactive_member_budget(pool: PgPool) {
    let (owner, original) = configured_fixture(&pool).await;
    sqlx::query!(
        "INSERT INTO project_member_budgets (id,org_id,project_id,user_id,budget_minutes)
         VALUES ($1,$2,$3,$4,125)",
        Uuid::now_v7(),
        owner.org_id,
        original.id,
        owner.user_id,
    )
    .execute(&pool)
    .await
    .unwrap();
    canonical_actor(
        &pool,
        &owner,
        &PermissionSelection::new(&[Permission::ProjectWriteAll]),
    )
    .await;
    let editor = load_editable_project(&pool, owner.user_id, owner.org_id, original.id)
        .await
        .unwrap();
    let mut request = edit_request(editor);
    request.unchanged = keep_protected(&request.form);
    request
        .unchanged
        .retain(|field| *field != ProtectedProjectField::Budget);
    request.form.budget_mode = BudgetMode::TotalHours;
    request.form.budget_value = "10".into();
    let (project, changed) =
        save_editable_project(&pool, owner.user_id, owner.org_id, &request, false)
            .await
            .unwrap();
    assert!(changed);
    assert_eq!(project.budget_minutes, Some(600));
    let budget = sqlx::query_scalar!(
        "SELECT budget_minutes FROM project_member_budgets WHERE project_id=$1 AND user_id=$2",
        original.id,
        owner.user_id,
    )
    .fetch_optional(&pool)
    .await
    .unwrap();
    assert_eq!(budget, None);
}

async fn assert_removed_budget_recomputes_total(
    pool: &PgPool,
    mode: BudgetMode,
    retained_budget: &str,
    expected: Option<i64>,
) {
    let (owner, original) = configured_fixture(pool).await;
    let removed_person = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO users (id,org_id,email,name) VALUES ($1,$2,$3,'Removable teammate')",
        removed_person,
        owner.org_id,
        format!("{removed_person}@test.com"),
    )
    .execute(pool)
    .await
    .unwrap();
    let mut setup = edit_request(original.clone());
    setup.form.budget_mode = mode;
    if mode == BudgetMode::HoursPerPerson {
        setup.form.team[0].budget = retained_budget.into();
        setup.form.team.push(ProjectMemberInput {
            user_id: removed_person,
            manager: false,
            billable_rate: String::new(),
            cost_rate: String::new(),
            budget: "20".into(),
        });
    } else {
        setup.form.tasks[0].budget = retained_budget.into();
        setup.form.tasks.push(ProjectTaskInput {
            id: Uuid::now_v7(),
            source: TaskSource::New {
                name: "Removable budget".into(),
            },
            billable: true,
            rate: String::new(),
            budget: "20".into(),
            access: TaskAccess::Everyone,
        });
    }
    setup.form.tasks.push(ProjectTaskInput {
        id: Uuid::now_v7(),
        source: TaskSource::New {
            name: "Retained inactive money".into(),
        },
        billable: true,
        rate: String::new(),
        budget: String::new(),
        access: TaskAccess::Everyone,
    });
    save_editable_project(pool, owner.user_id, owner.org_id, &setup, false)
        .await
        .unwrap();
    if mode != BudgetMode::FeesPerTask {
        sqlx::query!(
            "UPDATE project_task_settings SET budget_cents=86753
             WHERE project_id=$1 AND task_id IN
               (SELECT id FROM tasks WHERE org_id=$2 AND name='Retained inactive money')",
            original.id,
            owner.org_id,
        )
        .execute(pool)
        .await
        .unwrap();
    }
    let grants = if mode == BudgetMode::FeesPerTask {
        PermissionSelection::new(&[
            Permission::ProjectWriteAll,
            Permission::BillableRateWriteAll,
        ])
    } else {
        PermissionSelection::new(&[Permission::ProjectWriteAll])
    };
    canonical_actor(pool, &owner, &grants).await;
    let editor = load_editable_project(pool, owner.user_id, owner.org_id, original.id)
        .await
        .unwrap();
    let removable = editor
        .selection
        .tasks
        .iter()
        .find(|task| task.name == "Removable budget")
        .map(|task| task.id);
    let mut request = edit_request(editor);
    request
        .form
        .team
        .retain(|person| person.user_id != removed_person);
    request.form.tasks.retain(|task| !matches!(task.source, TaskSource::Existing { task_id } if Some(task_id) == removable));
    request.unchanged = keep_protected(&request.form);
    let set_grants = async |grants: &PermissionSelection| {
        let grants: Vec<String> =
            serde_json::from_value(serde_json::to_value(grants).unwrap()).unwrap();
        sqlx::query!(
            "UPDATE person_permission_states SET grants=$2 WHERE user_id=$1",
            owner.user_id,
            &grants
        )
        .execute(pool)
        .await
        .unwrap();
    };
    let project_only = PermissionSelection::new(&[Permission::ProjectWriteAll]);
    if mode == BudgetMode::FeesPerTask {
        set_grants(&project_only).await;
        let denied =
            save_editable_project(pool, owner.user_id, owner.org_id, &request, false).await;
        assert!(
            matches!(
                denied,
                Err(ServerFnError::ServerError {
                    code: FORBIDDEN,
                    ..
                })
            ),
            "{denied:?}"
        );
        set_grants(&grants).await;
    }
    let (saved, changed) =
        save_editable_project(pool, owner.user_id, owner.org_id, &request, false)
            .await
            .unwrap();
    assert!(changed);
    if mode == BudgetMode::FeesPerTask {
        assert_eq!(saved.budget_amount_cents, expected);
    } else {
        assert_eq!(saved.budget_minutes, expected);
        let retained = sqlx::query_scalar!(
            "SELECT budget_cents FROM project_task_settings WHERE project_id=$1 AND task_id IN
               (SELECT id FROM tasks WHERE org_id=$2 AND name='Retained inactive money')",
            original.id,
            owner.org_id,
        )
        .fetch_one(pool)
        .await
        .unwrap();
        assert_eq!(
            retained,
            Some(86753),
            "inactive money must not be cleared by an hours-only edit"
        );
    }
    let (replayed, changed) =
        save_editable_project(pool, owner.user_id, owner.org_id, &request, false)
            .await
            .unwrap();
    assert!(!changed);
    assert_eq!(replayed.budget_minutes, saved.budget_minutes);
    assert_eq!(replayed.budget_amount_cents, saved.budget_amount_cents);
    if mode == BudgetMode::FeesPerTask {
        set_grants(&project_only).await;
        let denied =
            save_editable_project(pool, owner.user_id, owner.org_id, &request, false).await;
        assert!(
            matches!(
                denied,
                Err(ServerFnError::ServerError {
                    code: FORBIDDEN,
                    ..
                })
            ),
            "{denied:?}"
        );
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn canonical_task_removal_recomputes_hour_budget_without_financial_writes(pool: PgPool) {
    for (budget, expected) in [("5", Some(300)), ("0", Some(0)), ("", None)] {
        assert_removed_budget_recomputes_total(&pool, BudgetMode::HoursPerTask, budget, expected)
            .await;
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn canonical_member_removal_recomputes_hour_budget_without_financial_writes(pool: PgPool) {
    for (budget, expected) in [("5", Some(300)), ("0", Some(0)), ("", None)] {
        assert_removed_budget_recomputes_total(&pool, BudgetMode::HoursPerPerson, budget, expected)
            .await;
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn canonical_task_removal_recomputes_fee_budget_with_financial_authority(pool: PgPool) {
    for (budget, expected) in [("5", Some(500)), ("0", Some(0)), ("", None)] {
        assert_removed_budget_recomputes_total(&pool, BudgetMode::FeesPerTask, budget, expected)
            .await;
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn canonical_blank_task_removal_preserves_stale_fee_total_without_financial_writes(
    pool: PgPool,
) {
    let (owner, original) = configured_fixture(&pool).await;
    let mut setup = edit_request(original.clone());
    setup.form.budget_mode = BudgetMode::FeesPerTask;
    setup.form.tasks[0].budget = "5".into();
    setup.form.tasks.push(ProjectTaskInput {
        id: Uuid::now_v7(),
        source: TaskSource::New {
            name: "No budget contribution".into(),
        },
        billable: true,
        rate: String::new(),
        budget: String::new(),
        access: TaskAccess::Everyone,
    });
    save_editable_project(&pool, owner.user_id, owner.org_id, &setup, false)
        .await
        .unwrap();
    sqlx::query!(
        "UPDATE projects SET budget_amount_cents=777 WHERE id=$1",
        original.id
    )
    .execute(&pool)
    .await
    .unwrap();
    canonical_actor(
        &pool,
        &owner,
        &PermissionSelection::new(&[Permission::ProjectWriteAll]),
    )
    .await;
    let editor = load_editable_project(&pool, owner.user_id, owner.org_id, original.id)
        .await
        .unwrap();
    let mut request = edit_request(editor);
    request.form.tasks.retain(|task| {
        task.source
            == TaskSource::Existing {
                task_id: owner.task_id,
            }
    });
    request.unchanged = keep_protected(&request.form);
    let (saved, changed) =
        save_editable_project(&pool, owner.user_id, owner.org_id, &request, false)
            .await
            .unwrap();
    assert!(changed);
    assert_eq!(
        saved.budget_amount_cents,
        Some(777),
        "a nonfinancial edit must not repair unrelated stored money"
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn canonical_editor_uses_designations_instead_of_legacy_team_roles(pool: PgPool) {
    use crate::models::project_managers::{ProjectManagersCommand, ProjectManagersCommandKind};
    use crate::server_fns::permissions::project_management;

    let (owner, original) = configured_fixture(&pool).await;
    assert!(
        original.form.team[0].manager,
        "legacy lead remains a legacy manager"
    );
    canonical_actor(
        &pool,
        &owner,
        &PermissionSelection::new(&[Permission::ProjectWriteAll]),
    )
    .await;
    let outside = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO users (id, org_id, email, name) VALUES ($1, $2, $3, 'Manager')",
        outside,
        owner.org_id,
        format!("{outside}@test.com")
    )
    .execute(&pool)
    .await
    .unwrap();
    canonical_actor(
        &pool,
        &SeedIds {
            user_id: outside,
            ..owner
        },
        &PermissionSelection::new(&[Permission::ProjectReadManaged]),
    )
    .await;
    project_management::execute(
        &pool,
        owner.org_id,
        owner.user_id,
        &ProjectManagersCommand {
            kind: ProjectManagersCommandKind::ReplaceProjectManagers,
            request_id: Uuid::now_v7(),
            expected_access_revision: 0,
            project_id: original.id,
            manager_ids: vec![outside],
        },
    )
    .await
    .unwrap();
    sqlx::query!("UPDATE users SET active = false WHERE id = $1", outside)
        .execute(&pool)
        .await
        .unwrap();
    let editor = load_editable_project(&pool, owner.user_id, owner.org_id, original.id)
        .await
        .unwrap();
    assert!(
        !editor.form.team[0].manager,
        "legacy role must not claim a canonical designation"
    );
    assert_eq!(
        editor.form.team.len(),
        1,
        "responsibility must not add tracking membership"
    );
    let wire = serde_json::to_value(&editor).unwrap();
    assert_eq!(wire["access"]["managers"]["access_revision"], 1);
    assert_eq!(
        wire["access"]["managers"]["managers"][0]["id"],
        outside.to_string()
    );
    assert_eq!(wire["access"]["managers"]["managers"][0]["active"], false);
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn canonical_save_requires_the_requester_that_opened_the_editor(pool: PgPool) {
    use crate::models::permission_editor::PermissionRequester;

    let (owner, original) = configured_fixture(&pool).await;
    canonical_actor(
        &pool,
        &owner,
        &PermissionSelection::new(&[Permission::ProjectWriteAll]),
    )
    .await;
    let editor = load_editable_project(&pool, owner.user_id, owner.org_id, original.id)
        .await
        .unwrap();
    let mut request = edit_request(editor);
    request.unchanged = keep_protected(&request.form);
    request.form.name = "Bound editor change".into();
    let requester = request.expected_requester;
    for unexpected in [
        None,
        Some(PermissionRequester {
            org_id: owner.org_id,
            user_id: Uuid::now_v7(),
        }),
        Some(PermissionRequester {
            org_id: Uuid::now_v7(),
            user_id: owner.user_id,
        }),
    ] {
        request.expected_requester = unexpected;
        let result =
            save_editable_project(&pool, owner.user_id, owner.org_id, &request, false).await;
        assert!(
            matches!(
                result,
                Err(ServerFnError::ServerError { code: CONFLICT, .. })
            ),
            "{result:?}"
        );
        let unchanged = load_editable_project(&pool, owner.user_id, owner.org_id, original.id)
            .await
            .unwrap();
        assert_eq!(unchanged.revision, original.revision);
        assert_eq!(unchanged.form.name, original.form.name);
    }
    // The same request ID must still be available after each rejected attempt.
    request.expected_requester = requester;
    let (_, changed) = save_editable_project(&pool, owner.user_id, owner.org_id, &request, false)
        .await
        .unwrap();
    assert!(changed);
    request.expected_requester = None;
    let result = save_editable_project(&pool, owner.user_id, owner.org_id, &request, false).await;
    assert!(
        matches!(
            result,
            Err(ServerFnError::ServerError { code: CONFLICT, .. })
        ),
        "{result:?}"
    );
    request.expected_requester = requester;
    let (_, changed) = save_editable_project(&pool, owner.user_id, owner.org_id, &request, false)
        .await
        .unwrap();
    assert!(!changed);
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn canonical_editor_cannot_save_after_switching_to_another_authorized_person(pool: PgPool) {
    let (owner, original) = configured_fixture(&pool).await;
    let second = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO users (id, org_id, email, name) VALUES ($1, $2, $3, 'Manager')",
        second,
        owner.org_id,
        format!("{second}@test.com")
    )
    .execute(&pool)
    .await
    .unwrap();
    let second_ids = SeedIds {
        user_id: second,
        ..owner
    };
    for actor in [&owner, &second_ids] {
        canonical_actor(
            &pool,
            actor,
            &PermissionSelection::new(&[Permission::ProjectWriteAll]),
        )
        .await;
    }
    let editor = load_editable_project(&pool, owner.user_id, owner.org_id, original.id)
        .await
        .unwrap();
    let mut request = edit_request(editor);
    request.unchanged = keep_protected(&request.form);
    request.form.name = "Edit from the first session".into();
    let result = save_editable_project(&pool, second, owner.org_id, &request, false).await;
    assert!(
        matches!(
            result,
            Err(ServerFnError::ServerError { code: CONFLICT, .. })
        ),
        "{result:?}"
    );

    let current = load_editable_project(&pool, second, owner.org_id, original.id)
        .await
        .unwrap();
    assert_eq!(current.revision, original.revision);
    assert_eq!(current.form.name, original.form.name);
    let mut fresh = edit_request(current);
    fresh.unchanged = keep_protected(&fresh.form);
    fresh.form.name = "Confirmed in the second session".into();
    let (_, changed) = save_editable_project(&pool, second, owner.org_id, &fresh, false)
        .await
        .unwrap();
    assert!(
        changed,
        "the second person has project authority, but cannot reuse the first person's form"
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn canonical_task_rate_zero_reset_and_revoked_retry_remain_distinct(pool: PgPool) {
    let (owner, original) = configured_fixture(&pool).await;
    sqlx::query!(
        "UPDATE tasks SET default_rate_cents=12345, default_rate_currency='EUR' WHERE id=$1",
        owner.task_id,
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE project_tasks SET rate_cents=NULL WHERE project_id=$1 AND task_id=$2",
        original.id,
        owner.task_id
    )
    .execute(&pool)
    .await
    .unwrap();
    canonical_actor(
        &pool,
        &owner,
        &PermissionSelection::new(&[
            Permission::ProjectWriteAll,
            Permission::BillableRateWriteAll,
        ]),
    )
    .await;
    for (index, (input, expected)) in [("", Some(12345)), ("0", Some(0)), ("", Some(12345))]
        .into_iter()
        .enumerate()
    {
        let editor = load_editable_project(&pool, owner.user_id, owner.org_id, original.id)
            .await
            .unwrap();
        let mut request = edit_request(editor);
        request.unchanged = keep_protected(&request.form);
        request
            .unchanged
            .retain(|field| *field != ProtectedProjectField::TaskRate(request.form.tasks[0].id));
        request.form.tasks[0].rate = input.into();
        save_editable_project(&pool, owner.user_id, owner.org_id, &request, false)
            .await
            .unwrap();
        let rate = sqlx::query_scalar!(
            "SELECT rate_cents FROM project_tasks WHERE project_id=$1 AND task_id=$2",
            original.id,
            owner.task_id
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(rate, expected);
        let (_, changed) =
            save_editable_project(&pool, owner.user_id, owner.org_id, &request, false)
                .await
                .unwrap();
        assert!(!changed);
        if index == 2 {
            let grants = PermissionSelection::new(&[
                Permission::ProjectWriteAll,
                Permission::BillableRateReadAll,
            ]);
            let grant_ids: Vec<String> =
                serde_json::from_value(serde_json::to_value(grants).unwrap()).unwrap();
            sqlx::query!(
                "UPDATE person_permission_states SET grants=$2 WHERE user_id=$1",
                owner.user_id,
                &grant_ids
            )
            .execute(&pool)
            .await
            .unwrap();
            let retry =
                save_editable_project(&pool, owner.user_id, owner.org_id, &request, false).await;
            assert!(
                matches!(
                    retry,
                    Err(ServerFnError::ServerError {
                        code: FORBIDDEN,
                        ..
                    })
                ),
                "{retry:?}"
            );
        }
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn canonical_save_preserves_withheld_fields_and_original_retry_intent(pool: PgPool) {
    let (owner, original) = configured_fixture(&pool).await;
    sqlx::query!(
        "UPDATE projects SET rate_cents=45678 WHERE id=$1",
        original.id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE project_settings SET fee_mode='monthly', fee_amount_cents=987654, monthly_day='last',
         terms_days=47, tax2_name='Private tax', tax2_bps=375 WHERE project_id=$1",
        original.id,
    ).execute(&pool).await.unwrap();
    sqlx::query!(
        "UPDATE project_task_settings SET budget_minutes=NULL, budget_cents=86753 WHERE project_id=$1 AND task_id=$2",
        original.id, owner.task_id,
    ).execute(&pool).await.unwrap();
    sqlx::query!(
        "INSERT INTO project_fee_milestones (id,org_id,project_id,position,name,due_on,amount_cents)
         VALUES ($1,$2,$3,0,'Inactive milestone',DATE '2026-10-05',556677)",
        Uuid::now_v7(), owner.org_id, original.id,
    )
    .execute(&pool)
    .await
    .unwrap();
    let original = load_editable_project(&pool, owner.user_id, owner.org_id, original.id)
        .await
        .unwrap();
    canonical_actor(
        &pool,
        &owner,
        &PermissionSelection::new(&[Permission::ProjectWriteAll]),
    )
    .await;
    let editor = load_editable_project(&pool, owner.user_id, owner.org_id, original.id)
        .await
        .unwrap();
    assert!(editor.form.tasks[0].rate.is_empty());
    assert!(editor.form.team[0].cost_rate.is_empty());
    let mut request = edit_request(editor);
    request.unchanged = keep_protected(&request.form);
    request.form.name = "Ordinary edit with withheld fields".into();
    let (_, changed) = save_editable_project(&pool, owner.user_id, owner.org_id, &request, false)
        .await
        .expect("project editing alone can preserve protected fields");
    assert!(changed);
    let mut tx = pool.begin().await.unwrap();
    let stored = load_project_form(&mut tx, owner.org_id, original.id, true)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    let mut expected = original.form;
    expected.name.clone_from(&request.form.name);
    assert_eq!(stored.form, expected);
    let task_budget = sqlx::query_scalar!(
        "SELECT budget_cents FROM project_task_settings WHERE project_id=$1 AND task_id=$2",
        original.id,
        owner.task_id,
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(task_budget, Some(86753));
    let (_, changed) = save_editable_project(&pool, owner.user_id, owner.org_id, &request, false)
        .await
        .expect("retry compares original intent, not the merged secret values");
    assert!(!changed);

    let receipt = sqlx::query_scalar!(
        "SELECT payload FROM project_edit_requests WHERE id=$1",
        request.id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let payload = serde_json::to_string(&receipt).unwrap();
    for secret in [
        "25.00",
        "10.00",
        "Private notes",
        "456.78",
        "9876.54",
        "Inactive milestone",
        "Private tax",
    ] {
        assert!(!payload.contains(secret));
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn canonical_new_task_alias_cannot_bypass_billable_write_authority(pool: PgPool) {
    let (owner, original) = configured_fixture(&pool).await;
    sqlx::query!(
        "UPDATE project_tasks SET rate_cents=NULL WHERE project_id=$1 AND task_id=$2",
        original.id,
        owner.task_id
    )
    .execute(&pool)
    .await
    .unwrap();
    canonical_actor(
        &pool,
        &owner,
        &PermissionSelection::new(&[Permission::ProjectWriteAll, Permission::TaskWriteAll]),
    )
    .await;
    let editor = load_editable_project(&pool, owner.user_id, owner.org_id, original.id)
        .await
        .unwrap();
    let task_name = editor.selection.tasks[0].name.clone();
    let mut request = edit_request(editor);
    request.form.tasks[0].id = Uuid::now_v7();
    request.form.tasks[0].source = TaskSource::New { name: task_name };
    request.form.tasks[0].billable = false;
    request.unchanged = keep_protected(&request.form);
    let result = save_editable_project(&pool, owner.user_id, owner.org_id, &request, false).await;
    assert!(
        matches!(
            result,
            Err(ServerFnError::ServerError {
                code: BAD_REQUEST,
                ..
            })
        ),
        "{result:?}"
    );
    let billable = sqlx::query_scalar!(
        "SELECT billable FROM project_tasks WHERE project_id=$1 AND task_id=$2",
        original.id,
        owner.task_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(billable);
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn canonical_save_rejects_equal_value_edit_without_write_authority(pool: PgPool) {
    let (owner, original) = configured_fixture(&pool).await;
    canonical_actor(
        &pool,
        &owner,
        &PermissionSelection::new(&[Permission::ProjectWriteAll, Permission::BillableRateReadAll]),
    )
    .await;
    let editor = load_editable_project(&pool, owner.user_id, owner.org_id, original.id)
        .await
        .unwrap();
    let mut request = edit_request(editor);
    request.unchanged = keep_protected(&request.form);
    request
        .unchanged
        .retain(|field| *field != ProtectedProjectField::TaskRate(request.form.tasks[0].id));
    assert_eq!(request.form.tasks[0].rate, "25.00");
    let result = save_editable_project(&pool, owner.user_id, owner.org_id, &request, false).await;
    assert!(
        matches!(
            result,
            Err(ServerFnError::ServerError {
                code: FORBIDDEN,
                ..
            })
        ),
        "{result:?}"
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn canonical_editor_without_rate_read_withholds_stored_task_rate(pool: PgPool) {
    let (owner, original) = configured_fixture(&pool).await;
    assert_eq!(original.form.tasks[0].rate, "25.00");
    sqlx::query!(
        "UPDATE tasks SET default_rate_cents=12345, default_rate_currency='EUR' WHERE id=$1",
        owner.task_id,
    )
    .execute(&pool)
    .await
    .unwrap();
    canonical_actor(
        &pool,
        &owner,
        &PermissionSelection::new(&[Permission::ProjectWriteAll]),
    )
    .await;

    let editor = load_editable_project(&pool, owner.user_id, owner.org_id, original.id)
        .await
        .expect("project editing does not require a rate grant");
    let task = editor.form.tasks.iter().find(|task| {
        matches!(task.source, TaskSource::Existing { task_id } if task_id == owner.task_id)
    }).expect("withholding rates must retain the assigned task");
    assert!(
        task.rate.is_empty(),
        "the form must not disclose the task override"
    );
    let choice = editor
        .selection
        .tasks
        .iter()
        .find(|task| task.id == owner.task_id)
        .expect("the selected task identity must remain available");
    assert_eq!(choice.default_rate_cents, None);
    let payload = serde_json::to_value(editor).unwrap();
    assert!(
        !serde_json::to_string(&payload)
            .unwrap()
            .contains("\"25.00\""),
        "the stored task rate must not appear anywhere in the editor payload"
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn canonical_cost_reader_does_not_require_legacy_administrator(pool: PgPool) {
    let (owner, original) = configured_fixture(&pool).await;
    assert_eq!(original.form.team[0].cost_rate, "10.00");
    canonical_actor(
        &pool,
        &owner,
        &PermissionSelection::new(&[Permission::ProjectWriteAll, Permission::CostRateReadAll]),
    )
    .await;
    sqlx::query!(
        "UPDATE users SET org_role='manager' WHERE id=$1",
        owner.user_id
    )
    .execute(&pool)
    .await
    .unwrap();

    let editor = load_editable_project(&pool, owner.user_id, owner.org_id, original.id)
        .await
        .unwrap();
    assert_eq!(editor.form.team[0].cost_rate, "10.00");
    assert!(editor.form.admin_notes.is_empty());
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn canonical_editor_withholds_monetary_settings_without_billable_read(pool: PgPool) {
    let (owner, original) = configured_fixture(&pool).await;
    let mut setup = edit_request(original.clone());
    setup.form.budget_mode = BudgetMode::TotalFees;
    setup.form.budget_value = "1234.57".into();
    setup.form.invoice_defaults.terms_days = "47".into();
    setup.form.invoice_defaults.po_number = "PRIVATE-PO".into();
    setup.form.invoice_defaults.tax = "11.75".into();
    setup.form.invoice_defaults.discount = "3.75".into();
    save_editable_project(&pool, owner.user_id, owner.org_id, &setup, false)
        .await
        .unwrap();
    canonical_actor(
        &pool,
        &owner,
        &PermissionSelection::new(&[Permission::ProjectWriteAll]),
    )
    .await;
    let editor = load_editable_project(&pool, owner.user_id, owner.org_id, original.id)
        .await
        .unwrap();
    assert!(
        editor.form.budget_value.is_empty(),
        "a project grant cannot reveal a fee budget"
    );
    assert!(editor.form.fee_amount.is_empty());
    assert!(editor.form.invoice_defaults.terms_days.is_empty());
    assert!(editor.form.invoice_defaults.po_number.is_empty());
    assert!(editor.form.invoice_defaults.tax.is_empty());
    assert!(editor.form.invoice_defaults.discount.is_empty());
    let access = editor.access.as_ref().unwrap();
    assert_eq!(access.billable, ProjectFieldAccess::Withheld);
    assert_eq!(access.costs, ProjectFieldAccess::Withheld);
    assert_eq!(access.private_notes, ProjectFieldAccess::Withheld);
    assert_eq!(access.requester.org_id, owner.org_id);
    assert_eq!(access.requester.user_id, owner.user_id);
    let payload = serde_json::to_string(&editor).unwrap();
    for withheld in ["1234.57", "PRIVATE-PO", "11.75", "3.75"] {
        assert!(
            !payload.contains(withheld),
            "protected value appeared in the editor: {withheld}"
        );
    }

    for (grant, expected_access) in [
        (
            Permission::BillableRateReadAll,
            ProjectFieldAccess::ReadOnly,
        ),
        (
            Permission::BillableRateWriteAll,
            ProjectFieldAccess::Editable,
        ),
    ] {
        let grants = PermissionSelection::new(&[Permission::ProjectWriteAll, grant]);
        let grant_ids: Vec<String> =
            serde_json::from_value(serde_json::to_value(&grants).unwrap()).unwrap();
        sqlx::query!(
            "UPDATE person_permission_states SET grants=$2 WHERE user_id=$1",
            owner.user_id,
            &grant_ids,
        )
        .execute(&pool)
        .await
        .unwrap();
        let editor = load_editable_project(&pool, owner.user_id, owner.org_id, original.id)
            .await
            .unwrap();
        assert_eq!(editor.form.budget_value, "1234.57");
        assert_eq!(editor.form.invoice_defaults.terms_days, "47");
        assert_eq!(editor.form.invoice_defaults.po_number, "PRIVATE-PO");
        assert_eq!(editor.form.invoice_defaults.tax, "11.75");
        assert_eq!(editor.form.invoice_defaults.discount, "3.75");
        let access = editor.access.unwrap();
        assert_eq!(access.billable, expected_access);
        assert_eq!(access.costs, ProjectFieldAccess::Withheld);
        assert_eq!(access.private_notes, ProjectFieldAccess::Withheld);
        assert!(editor.form.admin_notes.is_empty());
        assert!(
            editor
                .form
                .team
                .iter()
                .all(|person| person.cost_rate.is_empty())
        );
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn canonical_hour_budget_edits_preserve_inactive_parent_money(pool: PgPool) {
    for previous_kind in [BudgetKind::Hours, BudgetKind::None] {
        for next_mode in [BudgetMode::TotalHours, BudgetMode::None] {
            for cents in [None, Some(0), Some(86753)] {
                let (owner, original) = configured_fixture(&pool).await;
                sqlx::query!(
                    "UPDATE projects SET budget_kind=$2,budget_minutes=600,budget_amount_cents=$3 WHERE id=$1",
                    original.id, previous_kind as BudgetKind, cents,
                ).execute(&pool).await.unwrap();
                canonical_actor(
                    &pool,
                    &owner,
                    &PermissionSelection::new(&[Permission::ProjectWriteAll]),
                )
                .await;
                let editor = load_editable_project(&pool, owner.user_id, owner.org_id, original.id)
                    .await
                    .unwrap();
                assert_eq!(
                    editor.access.as_ref().unwrap().billable,
                    ProjectFieldAccess::Withheld
                );
                assert!(!serde_json::to_string(&editor).unwrap().contains("86753"));
                let mut request = edit_request(editor);
                request.unchanged = keep_protected(&request.form);
                request
                    .unchanged
                    .retain(|field| *field != ProtectedProjectField::Budget);
                request.form.name = "Hour-only budget edit".into();
                request.form.budget_mode = next_mode;
                request.form.budget_value = if next_mode == BudgetMode::TotalHours {
                    "11"
                } else {
                    ""
                }
                .into();
                for _ in 0..2 {
                    save_editable_project(&pool, owner.user_id, owner.org_id, &request, false)
                        .await
                        .unwrap();
                    let saved = sqlx::query!(
                        r#"SELECT budget_kind as "budget_kind: BudgetKind", budget_minutes, budget_amount_cents FROM projects WHERE id=$1"#,
                        original.id,
                    ).fetch_one(&pool).await.unwrap();
                    assert_eq!(
                        saved.budget_amount_cents, cents,
                        "{previous_kind:?} -> {next_mode:?}"
                    );
                    assert_eq!(
                        saved.budget_minutes,
                        (next_mode == BudgetMode::TotalHours).then_some(660)
                    );
                    assert_eq!(
                        saved.budget_kind,
                        if next_mode == BudgetMode::TotalHours {
                            BudgetKind::Hours
                        } else {
                            BudgetKind::None
                        }
                    );
                }
            }
        }
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn canonical_fee_to_hours_requires_financial_authority_on_save_and_replay(pool: PgPool) {
    let (owner, original) = configured_fixture(&pool).await;
    let mut setup = edit_request(original.clone());
    setup.form.budget_mode = BudgetMode::TotalFees;
    setup.form.budget_value = "867.53".into();
    save_editable_project(&pool, owner.user_id, owner.org_id, &setup, false)
        .await
        .unwrap();
    canonical_actor(
        &pool,
        &owner,
        &PermissionSelection::new(&[Permission::ProjectWriteAll]),
    )
    .await;
    let editor = load_editable_project(&pool, owner.user_id, owner.org_id, original.id)
        .await
        .unwrap();
    let mut request = edit_request(editor);
    request.unchanged = keep_protected(&request.form);
    request
        .unchanged
        .retain(|field| *field != ProtectedProjectField::Budget);
    request.form.budget_mode = BudgetMode::TotalHours;
    request.form.budget_value = "11".into();

    for (financial_write, expected_kind, expected_cents, expected_minutes) in [
        (false, BudgetKind::Amount, Some(86753), None),
        (true, BudgetKind::Hours, None, Some(660)),
        (false, BudgetKind::Hours, None, Some(660)),
    ] {
        let grants = if financial_write {
            PermissionSelection::new(&[
                Permission::ProjectWriteAll,
                Permission::BillableRateWriteAll,
            ])
        } else {
            PermissionSelection::new(&[Permission::ProjectWriteAll])
        };
        let grant_ids: Vec<String> =
            serde_json::from_value(serde_json::to_value(grants).unwrap()).unwrap();
        sqlx::query!(
            "UPDATE person_permission_states SET grants=$2 WHERE user_id=$1",
            owner.user_id,
            &grant_ids,
        )
        .execute(&pool)
        .await
        .unwrap();
        let result =
            save_editable_project(&pool, owner.user_id, owner.org_id, &request, false).await;
        if financial_write {
            assert!(result.unwrap().1);
            let (_, changed) =
                save_editable_project(&pool, owner.user_id, owner.org_id, &request, false)
                    .await
                    .unwrap();
            assert!(!changed);
        } else {
            assert!(
                matches!(
                    result,
                    Err(ServerFnError::ServerError {
                        code: FORBIDDEN,
                        ..
                    })
                ),
                "{result:?}"
            );
        }
        let saved = sqlx::query!(
            r#"SELECT budget_kind as "budget_kind: BudgetKind", budget_minutes, budget_amount_cents FROM projects WHERE id=$1"#,
            original.id,
        ).fetch_one(&pool).await.unwrap();
        assert_eq!(saved.budget_kind, expected_kind);
        assert_eq!(saved.budget_amount_cents, expected_cents);
        assert_eq!(saved.budget_minutes, expected_minutes);
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn canonical_editor_keeps_hour_budgets_without_financial_grants(pool: PgPool) {
    let (owner, original) = configured_fixture(&pool).await;
    let mut setup = edit_request(original);
    setup.form.project_type = ProjectType::FixedFee;
    setup.form.fee_amount = "9876.54".into();
    setup.form.budget_mode = BudgetMode::TotalHours;
    setup.form.budget_value = "123:45".into();
    save_editable_project(&pool, owner.user_id, owner.org_id, &setup, false)
        .await
        .unwrap();
    let before = load_editable_project(&pool, owner.user_id, owner.org_id, setup.project_id)
        .await
        .unwrap();
    canonical_actor(
        &pool,
        &owner,
        &PermissionSelection::new(&[Permission::ProjectWriteAll]),
    )
    .await;
    let editor = load_editable_project(&pool, owner.user_id, owner.org_id, setup.project_id)
        .await
        .unwrap();
    assert_eq!(editor.form.budget_mode, BudgetMode::TotalHours);
    assert_eq!(editor.form.budget_value, before.form.budget_value);
    assert!(!editor.form.budget_value.is_empty());
    assert_eq!(editor.form.budget_alert_at, before.form.budget_alert_at);
    assert_eq!(before.form.fee_amount, "9876.54");
    assert!(editor.form.fee_amount.is_empty());
    assert!(!serde_json::to_string(&editor).unwrap().contains("9876.54"));
    assert_eq!(
        editor.access.unwrap().billable,
        ProjectFieldAccess::Withheld
    );
    let grants =
        PermissionSelection::new(&[Permission::ProjectWriteAll, Permission::BillableRateReadAll]);
    let grant_ids: Vec<String> =
        serde_json::from_value(serde_json::to_value(&grants).unwrap()).unwrap();
    sqlx::query!(
        "UPDATE person_permission_states SET grants=$2 WHERE user_id=$1",
        owner.user_id,
        &grant_ids,
    )
    .execute(&pool)
    .await
    .unwrap();
    let reader = load_editable_project(&pool, owner.user_id, owner.org_id, setup.project_id)
        .await
        .unwrap();
    assert_eq!(reader.form.fee_amount, "9876.54");
    assert_eq!(reader.form.budget_value, before.form.budget_value);
    assert_eq!(
        reader.access.unwrap().billable,
        ProjectFieldAccess::ReadOnly
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn canonical_editor_managed_rates_require_designation_and_withhold_catalog_defaults(
    pool: PgPool,
) {
    let (owner, original) = configured_fixture(&pool).await;
    sqlx::query!(
        "UPDATE tasks SET default_rate_cents=12345, default_rate_currency='EUR' WHERE id=$1",
        owner.task_id,
    )
    .execute(&pool)
    .await
    .unwrap();
    canonical_actor(
        &pool,
        &owner,
        &PermissionSelection::new(&[
            Permission::ProjectWriteAll,
            Permission::BillableRateReadManaged,
        ]),
    )
    .await;
    let undesignated = load_editable_project(&pool, owner.user_id, owner.org_id, original.id)
        .await
        .unwrap();
    assert!(undesignated.form.tasks[0].rate.is_empty());
    assert_eq!(
        undesignated.access.unwrap().billable,
        ProjectFieldAccess::Withheld
    );

    sqlx::query!(
        "INSERT INTO project_management_assignments (id,org_id,manager_id,project_id) VALUES ($1,$2,$3,$4)",
        Uuid::now_v7(), owner.org_id, owner.user_id, original.id,
    ).execute(&pool).await.unwrap();
    let designated = load_editable_project(&pool, owner.user_id, owner.org_id, original.id)
        .await
        .unwrap();
    assert_eq!(designated.form.tasks[0].rate, "25.00");
    assert_eq!(
        designated.access.unwrap().billable,
        ProjectFieldAccess::ReadOnly
    );
    let task = designated
        .selection
        .tasks
        .iter()
        .find(|task| task.id == owner.task_id)
        .unwrap();
    assert_eq!(task.default_rate_cents, None);
    assert_eq!(task.default_rate_currency, None);
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn canonical_editor_never_projects_stale_task_money_as_hours(pool: PgPool) {
    let (owner, original) = configured_fixture(&pool).await;
    let mut setup = edit_request(original.clone());
    setup.form.budget_mode = BudgetMode::HoursPerTask;
    setup.form.tasks[0].budget = "20:00".into();
    save_editable_project(&pool, owner.user_id, owner.org_id, &setup, false)
        .await
        .unwrap();
    // The database allows a task's stored units to disagree with its parent mode.
    sqlx::query!(
        "UPDATE project_task_settings SET budget_minutes=NULL, budget_cents=86753 WHERE project_id=$1 AND task_id=$2",
        original.id, owner.task_id,
    ).execute(&pool).await.unwrap();
    canonical_actor(
        &pool,
        &owner,
        &PermissionSelection::new(&[Permission::ProjectWriteAll]),
    )
    .await;
    let editor = load_editable_project(&pool, owner.user_id, owner.org_id, original.id)
        .await
        .unwrap();
    assert_eq!(editor.form.budget_mode, BudgetMode::HoursPerTask);
    assert!(editor.form.tasks[0].budget.is_empty());
    let payload = serde_json::to_string(&editor).unwrap();
    assert!(!payload.contains("867.53"));
    assert!(!payload.contains("86753"));
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn canonical_read_only_billable_grant_rejects_task_rate_update(pool: PgPool) {
    let (owner, original) = configured_fixture(&pool).await;
    sqlx::query!(
        "INSERT INTO project_management_assignments (id,org_id,manager_id,project_id) VALUES ($1,$2,$3,$4)",
        Uuid::now_v7(), owner.org_id, owner.user_id, original.id,
    ).execute(&pool).await.unwrap();
    // Isolate the rate change: a private-note denial must not satisfy this test.
    sqlx::query!(
        "UPDATE project_private_settings SET admin_notes='' WHERE project_id=$1 AND org_id=$2",
        original.id,
        owner.org_id,
    )
    .execute(&pool)
    .await
    .unwrap();
    let mut grants = BuiltInProfile::Administrator.selection();
    grants.remove(Permission::BillableRateWriteAll).unwrap();
    grants.remove(Permission::BillableRateWriteManaged).unwrap();
    assert!(grants.contains(Permission::BillableRateReadAll));
    canonical_actor(&pool, &owner, &grants).await;
    let original = load_editable_project(&pool, owner.user_id, owner.org_id, original.id)
        .await
        .unwrap();
    let mut request = edit_request(original.clone());
    request.unchanged.push(ProtectedProjectField::PrivateNotes);
    request.form.tasks[0].rate = "30.00".into();

    let result = save_editable_project(&pool, owner.user_id, owner.org_id, &request, false).await;
    assert!(
        matches!(
            result,
            Err(ServerFnError::ServerError {
                code: FORBIDDEN,
                ..
            })
        ),
        "read-only rate authority must reject the explicit financial edit: {result:?}"
    );
    let saved_rate = sqlx::query_scalar!(
        "SELECT rate_cents FROM project_tasks WHERE project_id=$1 AND task_id=$2",
        original.id,
        owner.task_id,
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(saved_rate, Some(2500));
    let rejected_state = sqlx::query!(
        r#"SELECT edit_revision,
           (SELECT count(*) FROM project_edit_requests WHERE id=$2) as "receipts!"
           FROM projects WHERE id=$1"#,
        original.id,
        request.id,
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(rejected_state.edit_revision, original.revision);
    assert_eq!(rejected_state.receipts, 0);

    // The same operation must succeed when only its missing rate grant changes.
    grants.add(Permission::BillableRateWriteAll);
    let grant_ids: Vec<String> =
        serde_json::from_value(serde_json::to_value(&grants).unwrap()).unwrap();
    sqlx::query!(
        "UPDATE person_permission_states SET grants=$2 WHERE user_id=$1",
        owner.user_id,
        &grant_ids,
    )
    .execute(&pool)
    .await
    .unwrap();
    save_editable_project(&pool, owner.user_id, owner.org_id, &request, false)
        .await
        .expect("the added billable-write grant must authorize the unchanged request");
    let saved_rate = sqlx::query_scalar!(
        "SELECT rate_cents FROM project_tasks WHERE project_id=$1 AND task_id=$2",
        original.id,
        owner.task_id,
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(saved_rate, Some(3000));
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn canonical_project_editor_is_not_rejected_by_legacy_member_role(pool: PgPool) {
    let (owner, original) = configured_fixture(&pool).await;
    canonical_actor(
        &pool,
        &owner,
        &PermissionSelection::new(&[Permission::ProjectWriteAll]),
    )
    .await;
    sqlx::query!(
        "UPDATE users SET org_role='member' WHERE id=$1",
        owner.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let editor = load_editable_project(&pool, owner.user_id, owner.org_id, original.id)
        .await
        .expect("canonical project-write authority must not depend on the legacy role");
    assert_eq!(editor.id, original.id);
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn canonical_financial_grants_do_not_replace_project_editing_authority(pool: PgPool) {
    let (owner, original) = configured_fixture(&pool).await;
    canonical_actor(
        &pool,
        &owner,
        &PermissionSelection::new(&[
            Permission::BillableRateWriteAll,
            Permission::CostRateWriteAll,
        ]),
    )
    .await;
    let result = load_editable_project(&pool, owner.user_id, owner.org_id, original.id).await;
    assert!(
        matches!(
            result,
            Err(ServerFnError::ServerError {
                code: FORBIDDEN,
                ..
            })
        ),
        "rate grants and a legacy Admin role must not open the project editor: {result:?}"
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn canonical_managed_editor_requires_the_project_designation(pool: PgPool) {
    let (owner, original) = configured_fixture(&pool).await;
    canonical_actor(
        &pool,
        &owner,
        &PermissionSelection::new(&[Permission::ProjectWriteManaged]),
    )
    .await;
    // The fixture has legacy tracking leadership, not a canonical designation.
    let result = load_editable_project(&pool, owner.user_id, owner.org_id, original.id).await;
    assert!(
        matches!(
            result,
            Err(ServerFnError::ServerError {
                code: FORBIDDEN,
                ..
            })
        ),
        "tracking membership must not substitute for project management: {result:?}"
    );
    sqlx::query!(
        "INSERT INTO project_management_assignments (id,org_id,manager_id,project_id) VALUES ($1,$2,$3,$4)",
        Uuid::now_v7(), owner.org_id, owner.user_id, original.id,
    ).execute(&pool).await.unwrap();
    assert!(
        load_editable_project(&pool, owner.user_id, owner.org_id, original.id)
            .await
            .is_ok()
    );
}
