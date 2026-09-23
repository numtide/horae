use super::*;
use crate::models::project_creation::{
    InvoiceDefaultsInput, ProjectMemberInput, ProjectTaskInput, ReportVisibility, SecondTaxInput,
    TaskSource,
};
use crate::server_fns::test_seed::seed;
use horae_core::project::{BudgetMode, RateMode};
use sqlx::PgPool;

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn editor_load_uses_current_role_across_all_billing_configurations(pool: PgPool) {
    for project_type in [
        ProjectType::TimeAndMaterials,
        ProjectType::FixedFee,
        ProjectType::NonBillable,
        ProjectType::Retainer,
    ] {
        for (mode, scope) in [
            (None, "project"),
            (Some("person"), "project"),
            (Some("task"), "project"),
            (Some("project"), "project"),
            (Some("task"), "task"),
            (Some("person"), "person"),
        ] {
            if project_type == ProjectType::Retainer && mode.is_some() {
                continue;
            }
            let ids = seed(&pool, OrgRole::Admin).await;
            sqlx::query!(
                "UPDATE projects SET project_type = $2 WHERE id = $1",
                ids.project_id,
                project_type as ProjectType
            )
            .execute(&pool)
            .await
            .unwrap();
            if let Some(mode) = mode {
                sqlx::query!(
                    "INSERT INTO project_settings (id, org_id, project_id, creator_id, rate_mode, budget_scope)
             VALUES ($1, $2, $3, $4, $5, $6)",
                    Uuid::now_v7(), ids.org_id, ids.project_id, ids.user_id, mode, scope,
                ).execute(&pool).await.unwrap();
            }
            for role in [OrgRole::Admin, OrgRole::Manager, OrgRole::Member] {
                sqlx::query!(
                    "UPDATE users SET org_role = $2 WHERE id = $1",
                    ids.user_id,
                    role as OrgRole
                )
                .execute(&pool)
                .await
                .unwrap();
                let editor =
                    load_editable_project(&pool, ids.user_id, ids.org_id, ids.project_id).await;
                if role == OrgRole::Member {
                    assert!(editor.is_err());
                    continue;
                }
                let editor = editor.unwrap();
                assert_eq!(editor.id, ids.project_id);
                assert_eq!(editor.configured, mode.is_some());
                assert_eq!(editor.form.project_type, project_type);
                assert_eq!(
                    editor.form.rate_mode,
                    match mode {
                        None => RateMode::Legacy,
                        Some("person") => RateMode::Person,
                        Some("task") => RateMode::Task,
                        Some("project") => RateMode::Project,
                        _ => unreachable!(),
                    }
                );
            }
            sqlx::query!(
                "UPDATE users SET org_role = 'admin', active = false WHERE id = $1",
                ids.user_id
            )
            .execute(&pool)
            .await
            .unwrap();
            assert!(
                load_editable_project(&pool, ids.user_id, ids.org_id, ids.project_id)
                    .await
                    .is_err()
            );
        }
    }
    let a = seed(&pool, OrgRole::Admin).await;
    let b = seed(&pool, OrgRole::Admin).await;
    for (org, user, project) in [
        (a.org_id, a.user_id, b.project_id),
        (a.org_id, b.user_id, a.project_id),
        (a.org_id, a.user_id, Uuid::now_v7()),
    ] {
        assert!(
            load_editable_project(&pool, user, org, project)
                .await
                .is_err()
        );
    }
}

fn edit_request(project: EditableProject) -> ProjectEditRequest {
    ProjectEditRequest {
        id: Uuid::now_v7(),
        project_id: project.id,
        expected_revision: project.revision,
        form: project.form,
    }
}

async fn configured_fixture(
    pool: &PgPool,
) -> (crate::server_fns::test_seed::SeedIds, EditableProject) {
    let owner = seed(pool, OrgRole::Admin).await;
    let form = ProjectForm {
        client_id: Some(owner.client_id),
        name: "Configured fixture".into(),
        admin_notes: "Private notes".into(),
        rate_mode: RateMode::Task,
        tasks: vec![ProjectTaskInput {
            id: Uuid::now_v7(),
            source: TaskSource::Existing {
                task_id: owner.task_id,
            },
            billable: true,
            rate: "25.00".into(),
            budget: String::new(),
            access: TaskAccess::Everyone,
        }],
        team: vec![ProjectMemberInput {
            user_id: owner.user_id,
            manager: true,
            billable_rate: String::new(),
            cost_rate: "10.00".into(),
            budget: String::new(),
        }],
        ..Default::default()
    };
    let draft_id = Uuid::now_v7();
    let draft = save_draft_record(pool, owner.user_id, owner.org_id, draft_id, 0, &form)
        .await
        .unwrap();
    let id = finalize_draft_record(
        pool,
        owner.user_id,
        owner.org_id,
        draft_id,
        draft.revision,
        &form,
        false,
    )
    .await
    .unwrap();
    let project = load_editable_project(pool, owner.user_id, owner.org_id, id)
        .await
        .unwrap();
    (owner, project)
}

#[sqlx::test(migrations = "./migrations")]
async fn manager_edit_preserves_private_settings_and_cannot_remove_them_indirectly(pool: PgPool) {
    let (owner, original) = configured_fixture(&pool).await;
    sqlx::query!(
        "UPDATE assignments SET role='admin' WHERE project_id=$1 AND user_id=$2",
        original.id,
        owner.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let assignment = sqlx::query!("SELECT id, role as \"role: ProjectRole\" FROM assignments WHERE project_id=$1 AND user_id=$2", original.id, owner.user_id)
        .fetch_one(&pool).await.unwrap();
    sqlx::query!(
        "UPDATE users SET org_role='manager' WHERE id=$1",
        owner.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let mut request = edit_request(
        load_editable_project(&pool, owner.user_id, owner.org_id, original.id)
            .await
            .unwrap(),
    );
    request.form.name = "Manager rename".into();
    request.form.tasks[0].rate = "30.00".into();
    save_editable_project(&pool, owner.user_id, owner.org_id, &request, false)
        .await
        .unwrap();
    let preserved = sqlx::query!("SELECT id, role as \"role: ProjectRole\" FROM assignments WHERE project_id=$1 AND user_id=$2", original.id, owner.user_id)
        .fetch_one(&pool).await.unwrap();
    assert_eq!(
        (preserved.id, preserved.role),
        (assignment.id, assignment.role)
    );
    let saved = load_editable_project(&pool, owner.user_id, owner.org_id, original.id)
        .await
        .unwrap();
    let mut removal = edit_request(saved.clone());
    removal.form.name = "Must roll back".into();
    removal.form.team.clear();
    assert!(matches!(
        save_editable_project(&pool, owner.user_id, owner.org_id, &removal, false)
            .await
            .unwrap_err(),
        ServerFnError::ServerError {
            code: FORBIDDEN,
            ..
        }
    ));
    assert_eq!(
        load_editable_project(&pool, owner.user_id, owner.org_id, original.id)
            .await
            .unwrap(),
        saved
    );
    sqlx::query!(
        "UPDATE users SET org_role='admin' WHERE id=$1",
        owner.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let admin = load_editable_project(&pool, owner.user_id, owner.org_id, original.id)
        .await
        .unwrap();
    assert_eq!(admin.form.admin_notes, original.form.admin_notes);
    assert_eq!(
        admin.form.team[0].cost_rate,
        original.form.team[0].cost_rate
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn concurrent_edits_have_one_winner_and_association_writes_invalidate_open_forms(
    pool: PgPool,
) {
    let (owner, original) = configured_fixture(&pool).await;
    let mut first = edit_request(original.clone());
    first.form.name = "First writer".into();
    let mut second = edit_request(original.clone());
    second.form.name = "Second writer".into();
    let (a, b) = tokio::join!(
        save_editable_project(&pool, owner.user_id, owner.org_id, &first, false),
        save_editable_project(&pool, owner.user_id, owner.org_id, &second, false),
    );
    assert_eq!(usize::from(a.is_ok()) + usize::from(b.is_ok()), 1);
    let failure = a.err().or_else(|| b.err()).unwrap();
    assert!(matches!(
        failure,
        ServerFnError::ServerError { code: CONFLICT, .. }
    ));
    let saved = load_editable_project(&pool, owner.user_id, owner.org_id, original.id)
        .await
        .unwrap();
    let stale = edit_request(saved.clone());
    sqlx::query!(
        "UPDATE project_tasks SET rate_cents=1000 WHERE project_id=$1 AND task_id=$2",
        original.id,
        owner.task_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let newer = load_editable_project(&pool, owner.user_id, owner.org_id, original.id)
        .await
        .unwrap();
    assert!(newer.revision > saved.revision);
    assert!(matches!(
        save_editable_project(&pool, owner.user_id, owner.org_id, &stale, false)
            .await
            .unwrap_err(),
        ServerFnError::ServerError { code: CONFLICT, .. }
    ));
    assert_eq!(
        load_editable_project(&pool, owner.user_id, owner.org_id, original.id)
            .await
            .unwrap(),
        newer
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn save_preserves_recorded_time_and_rejects_removing_its_task_or_person(pool: PgPool) {
    let (mut owner, original) = configured_fixture(&pool).await;
    owner.project_id = original.id;
    let entry_id = crate::server_fns::test_seed::time_entry(&pool, &owner, EntryState::Open).await;
    let history = sqlx::query_scalar!(
        "SELECT to_jsonb(t) FROM time_entries t WHERE id=$1",
        entry_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let mut request = edit_request(original.clone());
    request.form.name = "Edited with history".into();
    request.form.tasks[0].rate = "42.00".into();
    save_editable_project(&pool, owner.user_id, owner.org_id, &request, false)
        .await
        .unwrap();
    let saved = load_editable_project(&pool, owner.user_id, owner.org_id, original.id)
        .await
        .unwrap();
    for remove_task in [true, false] {
        let mut request = edit_request(saved.clone());
        if remove_task {
            request.form.tasks.clear();
        } else {
            request.form.team.clear();
        }
        assert!(matches!(
            save_editable_project(&pool, owner.user_id, owner.org_id, &request, false)
                .await
                .unwrap_err(),
            ServerFnError::ServerError { code: CONFLICT, .. }
        ));
        assert_eq!(
            load_editable_project(&pool, owner.user_id, owner.org_id, original.id)
                .await
                .unwrap(),
            saved
        );
    }
    assert_eq!(
        sqlx::query_scalar!(
            "SELECT to_jsonb(t) FROM time_entries t WHERE id=$1",
            entry_id
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        history
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn saves_recheck_current_authority_and_do_not_repeat_noops(pool: PgPool) {
    let (owner, original) = configured_fixture(&pool).await;
    let mut request = edit_request(original.clone());
    let (_, changed) = save_editable_project(&pool, owner.user_id, owner.org_id, &request, false)
        .await
        .unwrap();
    assert!(!changed);
    assert_eq!(
        load_editable_project(&pool, owner.user_id, owner.org_id, original.id)
            .await
            .unwrap(),
        original
    );
    request.form.name = "Changed retry payload".into();
    assert!(matches!(
        save_editable_project(&pool, owner.user_id, owner.org_id, &request, false)
            .await
            .unwrap_err(),
        ServerFnError::ServerError { code: CONFLICT, .. }
    ));
    for (role, active) in [(OrgRole::Member, true), (OrgRole::Admin, false)] {
        sqlx::query!(
            "UPDATE users SET org_role=$2, active=$3 WHERE id=$1",
            owner.user_id,
            role as OrgRole,
            active
        )
        .execute(&pool)
        .await
        .unwrap();
        assert!(matches!(
            save_editable_project(&pool, owner.user_id, owner.org_id, &request, false)
                .await
                .unwrap_err(),
            ServerFnError::ServerError {
                code: FORBIDDEN,
                ..
            }
        ));
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn editing_milestones_keeps_invoice_snapshots_and_fee_identities_immutable(pool: PgPool) {
    let owner = seed(&pool, OrgRole::Admin).await;
    let form = ProjectForm {
        client_id: Some(owner.client_id),
        name: "Milestones".into(),
        project_type: ProjectType::FixedFee,
        fee_mode: FeeMode::Milestones,
        starts_on: "2026-09-01".into(),
        milestones: vec![
            MilestoneInput {
                id: Uuid::now_v7(),
                name: "Delivered".into(),
                due_on: "2026-09-05".into(),
                amount: "125.00".into(),
            },
            MilestoneInput {
                id: Uuid::now_v7(),
                name: "Future delivery".into(),
                due_on: "2026-12-31".into(),
                amount: "100.00".into(),
            },
        ],
        ..Default::default()
    };
    let draft_id = Uuid::now_v7();
    let draft = save_draft_record(&pool, owner.user_id, owner.org_id, draft_id, 0, &form)
        .await
        .unwrap();
    let id = finalize_draft_record(
        &pool,
        owner.user_id,
        owner.org_id,
        draft_id,
        draft.revision,
        &form,
        false,
    )
    .await
    .unwrap();
    let invoice = crate::server_fns::invoices::generate_invoice_for_period(
        &pool,
        owner.org_id,
        owner.client_id,
        "2026-09-01".parse().unwrap(),
        "2026-09-30".parse().unwrap(),
    )
    .await
    .unwrap();
    assert_eq!(invoice.lines.len(), 1);
    let snapshot = sqlx::query_scalar!(
        "SELECT jsonb_build_array((SELECT to_jsonb(i) FROM invoices i WHERE id=$1),
         (SELECT jsonb_agg(to_jsonb(l) ORDER BY id) FROM invoice_line_items l WHERE invoice_id=$1),
         (SELECT jsonb_agg(to_jsonb(f) ORDER BY id) FROM project_fee_occurrences f WHERE project_id=$2))",
        invoice.invoice.id, id,
    ).fetch_one(&pool).await.unwrap();
    let original = load_editable_project(&pool, owner.user_id, owner.org_id, id)
        .await
        .unwrap();
    let charged_id = original.form.milestones[0].id;
    let mut request = edit_request(original);
    request.form.name = "Updated milestones".into();
    request.form.milestones.reverse();
    request.form.milestones[0].amount = "200.00".into();
    save_editable_project(&pool, owner.user_id, owner.org_id, &request, false)
        .await
        .unwrap();
    let saved = load_editable_project(&pool, owner.user_id, owner.org_id, id)
        .await
        .unwrap();
    assert_eq!(saved.form, request.form);
    assert_eq!(saved.form.milestones[1].id, charged_id);
    let mut invalid = edit_request(saved.clone());
    invalid.form.milestones[1].amount = "999.00".into();
    assert!(matches!(
        save_editable_project(&pool, owner.user_id, owner.org_id, &invalid, false)
            .await
            .unwrap_err(),
        ServerFnError::ServerError { code: CONFLICT, .. }
    ));
    assert_eq!(
        load_editable_project(&pool, owner.user_id, owner.org_id, id)
            .await
            .unwrap(),
        saved
    );
    assert_eq!(sqlx::query_scalar!(
        "SELECT jsonb_build_array((SELECT to_jsonb(i) FROM invoices i WHERE id=$1),
         (SELECT jsonb_agg(to_jsonb(l) ORDER BY id) FROM invoice_line_items l WHERE invoice_id=$1),
         (SELECT jsonb_agg(to_jsonb(f) ORDER BY id) FROM project_fee_occurrences f WHERE project_id=$2))",
        invoice.invoice.id, id,
    ).fetch_one(&pool).await.unwrap(), snapshot);
}

#[sqlx::test(migrations = "./migrations")]
async fn task_catalog_and_graph_changes_roll_back_together_on_duplicate_resolution(pool: PgPool) {
    let (owner, original) = configured_fixture(&pool).await;
    let mut request = edit_request(original.clone());
    for name in ["Must not survive", "Dev"] {
        request.form.tasks.push(ProjectTaskInput {
            id: Uuid::now_v7(),
            source: TaskSource::New { name: name.into() },
            billable: true,
            rate: "0.00".into(),
            budget: String::new(),
            access: TaskAccess::Everyone,
        });
    }
    request.form.name = "Must not survive".into();
    assert!(matches!(
        save_editable_project(&pool, owner.user_id, owner.org_id, &request, false)
            .await
            .unwrap_err(),
        ServerFnError::ServerError {
            code: BAD_REQUEST,
            ..
        }
    ));
    assert_eq!(
        load_editable_project(&pool, owner.user_id, owner.org_id, original.id)
            .await
            .unwrap(),
        original
    );
    assert_eq!(
        sqlx::query_scalar!("SELECT count(*) FROM tasks WHERE org_id=$1", owner.org_id)
            .fetch_one(&pool)
            .await
            .unwrap(),
        Some(1)
    );
    assert_eq!(
        sqlx::query_scalar!(
            "SELECT count(*) FROM project_edit_requests WHERE org_id=$1",
            owner.org_id
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        Some(0)
    );

    request.form.tasks.pop();
    save_editable_project(&pool, owner.user_id, owner.org_id, &request, false)
        .await
        .unwrap();
    let saved = load_editable_project(&pool, owner.user_id, owner.org_id, original.id)
        .await
        .unwrap();
    assert_eq!(saved.form.tasks.len(), 2);
    assert_eq!(saved.form.tasks[0].id, original.form.tasks[0].id);
    let mut removal = edit_request(saved);
    removal.form.tasks.truncate(1);
    save_editable_project(&pool, owner.user_id, owner.org_id, &removal, false)
        .await
        .unwrap();
    assert_eq!(
        sqlx::query_scalar!("SELECT count(*) FROM tasks WHERE org_id=$1", owner.org_id)
            .fetch_one(&pool)
            .await
            .unwrap(),
        Some(2)
    );
    assert_eq!(
        load_editable_project(&pool, owner.user_id, owner.org_id, original.id)
            .await
            .unwrap()
            .form
            .tasks,
        original.form.tasks
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn foreign_actor_cannot_save_or_retry_another_organizations_edit(pool: PgPool) {
    let (owner, original) = configured_fixture(&pool).await;
    let foreign = seed(&pool, OrgRole::Admin).await;
    let mut request = edit_request(original.clone());
    request.form.name = "Not authorized".into();
    assert!(matches!(
        save_editable_project(&pool, foreign.user_id, foreign.org_id, &request, false)
            .await
            .unwrap_err(),
        ServerFnError::ServerError {
            code: NOT_FOUND,
            ..
        }
    ));
    assert_eq!(
        load_editable_project(&pool, owner.user_id, owner.org_id, original.id)
            .await
            .unwrap(),
        original
    );
    save_editable_project(&pool, owner.user_id, owner.org_id, &request, false)
        .await
        .unwrap();
    assert!(matches!(
        save_editable_project(&pool, foreign.user_id, foreign.org_id, &request, false)
            .await
            .unwrap_err(),
        ServerFnError::ServerError {
            code: NOT_FOUND,
            ..
        }
    ));
}

#[sqlx::test(migrations = "./migrations")]
async fn save_edits_the_same_legacy_project_and_never_creates_a_draft_or_configuration(
    pool: PgPool,
) {
    let owner = seed(&pool, OrgRole::Admin).await;
    sqlx::query!(
        "UPDATE projects SET project_type = 'retainer', currency = 'JPY' WHERE id = $1",
        owner.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let mut request = edit_request(
        load_editable_project(&pool, owner.user_id, owner.org_id, owner.project_id)
            .await
            .unwrap(),
    );
    request.form.name = "Updated retainer".into();
    request.form.code = "RET-1".into();
    request.form.starts_on = "2026-09-01".into();
    request.form.tags = vec!["Retainer".into()];
    request.form.admin_notes = "Private context".into();
    let id = save_editable_project(&pool, owner.user_id, owner.org_id, &request, false)
        .await
        .unwrap()
        .0
        .id;
    assert_eq!(id, owner.project_id);
    let saved = load_editable_project(&pool, owner.user_id, owner.org_id, id)
        .await
        .unwrap();
    assert_eq!(saved.form, request.form);
    assert!(saved.revision > request.expected_revision);
    assert_eq!(
        sqlx::query_scalar!("SELECT count(*) FROM projects")
            .fetch_one(&pool)
            .await
            .unwrap(),
        Some(1)
    );
    assert_eq!(
        sqlx::query_scalar!("SELECT count(*) FROM project_drafts")
            .fetch_one(&pool)
            .await
            .unwrap(),
        Some(0)
    );
    assert_eq!(
        sqlx::query_scalar!("SELECT count(*) FROM project_settings")
            .fetch_one(&pool)
            .await
            .unwrap(),
        Some(0)
    );
    let retry = save_editable_project(&pool, owner.user_id, owner.org_id, &request, false)
        .await
        .unwrap();
    assert_eq!(retry.0.id, id);
    assert!(!retry.1);
    assert_eq!(
        load_editable_project(&pool, owner.user_id, owner.org_id, id)
            .await
            .unwrap(),
        saved
    );
    let mut later = edit_request(saved);
    later.form.name = "Later edit".into();
    save_editable_project(&pool, owner.user_id, owner.org_id, &later, false)
        .await
        .unwrap();
    let error = save_editable_project(&pool, owner.user_id, owner.org_id, &request, false)
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        ServerFnError::ServerError { code: CONFLICT, .. }
    ));
    assert_eq!(
        load_editable_project(&pool, owner.user_id, owner.org_id, id)
            .await
            .unwrap()
            .form
            .name,
        "Later edit"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn save_updates_configured_sections_atomically_and_rejects_stale_or_invalid_edits(
    pool: PgPool,
) {
    let owner = seed(&pool, OrgRole::Admin).await;
    let form = ProjectForm {
        client_id: Some(owner.client_id),
        name: "Configured".into(),
        rate_mode: RateMode::Task,
        budget_mode: BudgetMode::HoursPerTask,
        tasks: vec![ProjectTaskInput {
            id: Uuid::now_v7(),
            source: TaskSource::Existing {
                task_id: owner.task_id,
            },
            billable: true,
            rate: "0".into(),
            budget: "10:00".into(),
            access: TaskAccess::Everyone,
        }],
        team: vec![ProjectMemberInput {
            user_id: owner.user_id,
            manager: true,
            billable_rate: String::new(),
            cost_rate: "10".into(),
            budget: String::new(),
        }],
        ..Default::default()
    };
    let draft_id = Uuid::now_v7();
    let draft = save_draft_record(&pool, owner.user_id, owner.org_id, draft_id, 0, &form)
        .await
        .unwrap();
    let id = finalize_draft_record(
        &pool,
        owner.user_id,
        owner.org_id,
        draft_id,
        draft.revision,
        &form,
        false,
    )
    .await
    .unwrap();
    let original = load_editable_project(&pool, owner.user_id, owner.org_id, id)
        .await
        .unwrap();
    let mut request = edit_request(original.clone());
    request.form.name = "Edited configured".into();
    request.form.admin_notes = "Only admins".into();
    request.form.tasks[0].rate = "14.25".into();
    request.form.tasks[0].budget = "12:30".into();
    request.form.tasks[0].access = TaskAccess::Restricted {
        user_ids: vec![owner.user_id],
    };
    request.form.team[0].cost_rate = "20.00".into();
    request.form.invoice_defaults.po_number = "EDIT-PO".into();
    request.form.invoice_defaults.tax = "5.25".into();
    save_editable_project(&pool, owner.user_id, owner.org_id, &request, false)
        .await
        .unwrap();
    let saved = load_editable_project(&pool, owner.user_id, owner.org_id, id)
        .await
        .unwrap();
    assert_eq!(saved.form, request.form);
    let stale = edit_request(original);
    assert!(matches!(
        save_editable_project(&pool, owner.user_id, owner.org_id, &stale, false)
            .await
            .unwrap_err(),
        ServerFnError::ServerError { code: CONFLICT, .. }
    ));
    let mut invalid = edit_request(saved.clone());
    invalid.form.name = "Must roll back".into();
    invalid.form.tasks[0].access = TaskAccess::Restricted {
        user_ids: vec![Uuid::now_v7()],
    };
    assert!(
        save_editable_project(&pool, owner.user_id, owner.org_id, &invalid, false)
            .await
            .is_err()
    );
    assert_eq!(
        load_editable_project(&pool, owner.user_id, owner.org_id, id)
            .await
            .unwrap(),
        saved
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn editor_loads_legacy_values_without_fabricating_a_billing_mode_or_draft(pool: PgPool) {
    let owner = seed(&pool, OrgRole::Admin).await;
    sqlx::query!(
        "UPDATE projects SET project_type = 'retainer', currency = 'JPY', rate_cents = 0,
         code = 'OLD-01', budget_kind = 'hours', budget_minutes = 1830, active = false WHERE id = $1",
        owner.project_id,
    ).execute(&pool).await.unwrap();
    let project = load_editable_project(&pool, owner.user_id, owner.org_id, owner.project_id)
        .await
        .unwrap();
    assert_eq!(project.id, owner.project_id);
    assert!(!project.configured && !project.active);
    assert_eq!(project.form.name, "Widget");
    assert_eq!(project.form.client_id, Some(owner.client_id));
    assert_eq!(project.form.code, "OLD-01");
    assert_eq!(project.form.project_type, ProjectType::Retainer);
    assert_eq!(project.form.currency.as_deref(), Some("JPY"));
    assert_eq!(project.form.rate_mode, RateMode::Legacy);
    assert_eq!(project.form.project_rate, "0.00");
    assert_eq!(project.form.budget_mode, BudgetMode::TotalHours);
    assert_eq!(project.form.budget_value, "30:30");
    assert_eq!(
        project.form.report_visibility,
        ReportVisibility::ProjectMembers
    );
    assert!(project.form.budget_nonbillable);
    assert_eq!(
        sqlx::query_scalar!("SELECT count(*) FROM project_drafts")
            .fetch_one(&pool)
            .await
            .unwrap(),
        Some(0)
    );
    assert_eq!(
        sqlx::query_scalar!("SELECT count(*) FROM project_settings")
            .fetch_one(&pool)
            .await
            .unwrap(),
        Some(0)
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn editor_recovers_full_configuration_and_redacts_private_values_after_role_change(
    pool: PgPool,
) {
    let owner = seed(&pool, OrgRole::Admin).await;
    let form = ProjectForm {
        client_id: Some(owner.client_id),
        name: "Configured editor".into(),
        code: "EDIT-02".into(),
        starts_on: "2026-09-01".into(),
        ends_on: "2026-12-31".into(),
        currency: Some("EUR".into()),
        tags: vec!["Editor".into()],
        admin_notes: "Private editor notes".into(),
        report_visibility: ReportVisibility::ProjectMembers,
        rate_mode: RateMode::Task,
        budget_mode: BudgetMode::HoursPerTask,
        budget_monthly: true,
        budget_nonbillable: true,
        tasks: vec![ProjectTaskInput {
            id: Uuid::now_v7(),
            source: TaskSource::Existing {
                task_id: owner.task_id,
            },
            billable: false,
            rate: "0".into(),
            budget: "30:30".into(),
            access: TaskAccess::Restricted {
                user_ids: vec![owner.user_id],
            },
        }],
        team: vec![ProjectMemberInput {
            user_id: owner.user_id,
            manager: true,
            billable_rate: String::new(),
            cost_rate: "62.25".into(),
            budget: String::new(),
        }],
        invoice_defaults: InvoiceDefaultsInput {
            terms_days: "45".into(),
            po_number: "PO-editor".into(),
            tax: "21".into(),
            second_tax: Some(SecondTaxInput {
                name: "Additional tax".into(),
                percentage: "5.25".into(),
            }),
            discount: "2.50".into(),
        },
        ..Default::default()
    };
    let draft_id = Uuid::now_v7();
    let saved = save_draft_record(&pool, owner.user_id, owner.org_id, draft_id, 0, &form)
        .await
        .unwrap();
    let id = finalize_draft_record(
        &pool,
        owner.user_id,
        owner.org_id,
        draft_id,
        saved.revision,
        &form,
        false,
    )
    .await
    .unwrap();
    let project = load_editable_project(&pool, owner.user_id, owner.org_id, id)
        .await
        .unwrap();
    assert!(project.configured && project.active);
    assert_eq!(project.form.name, form.name);
    assert_eq!(project.form.code, form.code);
    assert_eq!(project.form.starts_on, form.starts_on);
    assert_eq!(project.form.ends_on, form.ends_on);
    assert_eq!(project.form.tags, form.tags);
    assert_eq!(project.form.admin_notes, form.admin_notes);
    assert_eq!(project.form.report_visibility, form.report_visibility);
    assert_eq!(project.form.rate_mode, RateMode::Task);
    assert_eq!(project.form.budget_mode, BudgetMode::HoursPerTask);
    assert!(project.form.budget_monthly && project.form.budget_nonbillable);
    assert_eq!(project.form.tasks.len(), 1);
    assert_eq!(project.form.tasks[0].source, form.tasks[0].source);
    assert_eq!(project.form.tasks[0].rate, "0.00");
    assert_eq!(project.form.tasks[0].budget, "30:30");
    assert!(!project.form.tasks[0].billable);
    assert_eq!(project.form.tasks[0].access, form.tasks[0].access);
    assert_eq!(project.form.team[0].user_id, owner.user_id);
    assert!(project.form.team[0].manager);
    assert_eq!(project.form.team[0].cost_rate, "62.25");
    assert_eq!(project.form.invoice_defaults.terms_days, "45");
    assert_eq!(project.form.invoice_defaults.po_number, "PO-editor");
    assert_eq!(project.form.invoice_defaults.tax, "21.00");
    assert_eq!(
        project.form.invoice_defaults.second_tax,
        form.invoice_defaults.second_tax
    );
    assert_eq!(project.form.invoice_defaults.discount, "2.50");

    sqlx::query!(
        "UPDATE users SET org_role = 'manager' WHERE id = $1",
        owner.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let manager = load_editable_project(&pool, owner.user_id, owner.org_id, id)
        .await
        .unwrap();
    assert!(manager.form.admin_notes.is_empty());
    assert!(manager.form.team[0].cost_rate.is_empty());
    assert!(manager.selection.people[0].cost_rate_cents.is_none());
    let serialized = serde_json::to_string(&manager).unwrap();
    assert!(!serialized.contains("Private editor notes") && !serialized.contains("62.25"));
    assert_eq!(manager.form.tasks[0].rate, "0.00");
}

#[sqlx::test(migrations = "./migrations")]
async fn editor_rechecks_tenant_role_and_active_status(pool: PgPool) {
    let owner = seed(&pool, OrgRole::Manager).await;
    let foreign = seed(&pool, OrgRole::Admin).await;
    for id in [foreign.project_id, Uuid::now_v7()] {
        let error = load_editable_project(&pool, owner.user_id, owner.org_id, id)
            .await
            .unwrap_err();
        assert!(matches!(
            error,
            ServerFnError::ServerError {
                code: NOT_FOUND,
                ..
            }
        ));
    }
    for (role, active) in [(OrgRole::Member, true), (OrgRole::Admin, false)] {
        sqlx::query!(
            "UPDATE users SET org_role = $2, active = $3 WHERE id = $1",
            owner.user_id,
            role as OrgRole,
            active
        )
        .execute(&pool)
        .await
        .unwrap();
        let error = load_editable_project(&pool, owner.user_id, owner.org_id, owner.project_id)
            .await
            .unwrap_err();
        assert!(matches!(
            error,
            ServerFnError::ServerError {
                code: FORBIDDEN,
                ..
            }
        ));
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn editor_keeps_archived_selected_identities_and_nullable_overrides(pool: PgPool) {
    let owner = seed(&pool, OrgRole::Admin).await;
    let user_id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO users (id, org_id, email, name, active) VALUES ($1, $2, 'archived@test.com', 'Archived person', false)",
        user_id, owner.org_id,
    ).execute(&pool).await.unwrap();
    sqlx::query!(
        "INSERT INTO assignments (id, project_id, user_id, role) VALUES ($1, $2, $3, 'admin')",
        Uuid::now_v7(),
        owner.project_id,
        user_id,
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO project_tasks (project_id, task_id, billable, rate_cents) VALUES ($1, $2, true, NULL)",
        owner.project_id, owner.task_id,
    ).execute(&pool).await.unwrap();
    sqlx::query!(
        "UPDATE tasks SET active = false WHERE id = $1",
        owner.task_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE clients SET active = false WHERE id = $1",
        owner.client_id
    )
    .execute(&pool)
    .await
    .unwrap();

    let project = load_editable_project(&pool, owner.user_id, owner.org_id, owner.project_id)
        .await
        .unwrap();
    assert!(!project.client.active);
    assert_eq!(project.client.name, "Acme");
    assert_eq!(project.inactive_task_ids, vec![owner.task_id]);
    assert_eq!(project.inactive_user_ids, vec![user_id]);
    assert_eq!(project.selection.tasks[0].name, "Dev");
    assert_eq!(project.selection.people[0].name, "Archived person");
    assert_eq!(project.form.tasks[0].id, owner.task_id);
    assert_eq!(project.form.tasks[0].access, TaskAccess::Everyone);
    assert!(project.form.tasks[0].rate.is_empty() && project.form.tasks[0].budget.is_empty());
    assert!(project.form.team[0].manager);
    assert!(
        project.form.team[0].billable_rate.is_empty() && project.form.team[0].cost_rate.is_empty()
    );
    assert!(project.form.team[0].budget.is_empty());
    let mut request = edit_request(project);
    request.form.name = "Rename with archived references".into();
    save_editable_project(&pool, owner.user_id, owner.org_id, &request, false)
        .await
        .unwrap();
    assert_eq!(
        load_editable_project(&pool, owner.user_id, owner.org_id, owner.project_id)
            .await
            .unwrap()
            .form,
        request.form
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn editor_preserves_fee_schedule_and_operational_milestone_ids(pool: PgPool) {
    let owner = seed(&pool, OrgRole::Admin).await;
    for mode in [FeeMode::Single, FeeMode::Milestones, FeeMode::Monthly] {
        let form = ProjectForm {
            client_id: Some(owner.client_id),
            name: format!("Fee {mode:?}"),
            project_type: ProjectType::FixedFee,
            fee_mode: mode,
            fee_amount: "0".into(),
            monthly_day: MonthlyFeeDay::Last,
            milestones: if mode == FeeMode::Milestones {
                vec![MilestoneInput {
                    id: Uuid::now_v7(),
                    name: "Delivery".into(),
                    due_on: "2026-12-31".into(),
                    amount: "123.45".into(),
                }]
            } else {
                vec![]
            },
            ..Default::default()
        };
        let draft_id = Uuid::now_v7();
        let saved = save_draft_record(&pool, owner.user_id, owner.org_id, draft_id, 0, &form)
            .await
            .unwrap();
        let id = finalize_draft_record(
            &pool,
            owner.user_id,
            owner.org_id,
            draft_id,
            saved.revision,
            &form,
            false,
        )
        .await
        .unwrap();
        let project = load_editable_project(&pool, owner.user_id, owner.org_id, id)
            .await
            .unwrap();
        assert_eq!(project.form.fee_mode, mode);
        if mode == FeeMode::Milestones {
            let milestone_id = sqlx::query_scalar!(
                "SELECT id FROM project_fee_milestones WHERE project_id = $1",
                id
            )
            .fetch_one(&pool)
            .await
            .unwrap();
            assert_eq!(
                project.form.milestones,
                vec![MilestoneInput {
                    id: milestone_id,
                    ..form.milestones[0].clone()
                }]
            );
        } else {
            assert_eq!(project.form.fee_amount, "0.00");
            assert!(project.form.milestones.is_empty());
        }
        if mode == FeeMode::Monthly {
            assert_eq!(project.form.monthly_day, MonthlyFeeDay::Last);
        }
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn editor_recovers_each_budget_scope_without_turning_absence_into_zero(pool: PgPool) {
    let owner = seed(&pool, OrgRole::Admin).await;
    for mode in [
        BudgetMode::None,
        BudgetMode::TotalHours,
        BudgetMode::TotalFees,
        BudgetMode::HoursPerTask,
        BudgetMode::FeesPerTask,
        BudgetMode::HoursPerPerson,
    ] {
        let form = ProjectForm {
            client_id: Some(owner.client_id),
            name: format!("Budget {mode:?}"),
            budget_mode: mode,
            budget_value: if mode == BudgetMode::TotalFees {
                "123.45"
            } else {
                ""
            }
            .into(),
            tasks: vec![ProjectTaskInput {
                id: Uuid::now_v7(),
                source: TaskSource::Existing {
                    task_id: owner.task_id,
                },
                billable: true,
                rate: String::new(),
                budget: match mode {
                    BudgetMode::HoursPerTask => "0",
                    BudgetMode::FeesPerTask => "123.45",
                    _ => "",
                }
                .into(),
                access: TaskAccess::Everyone,
            }],
            team: vec![ProjectMemberInput {
                user_id: owner.user_id,
                manager: false,
                billable_rate: String::new(),
                cost_rate: String::new(),
                budget: if mode == BudgetMode::HoursPerPerson {
                    "30:30"
                } else {
                    ""
                }
                .into(),
            }],
            ..Default::default()
        };
        let draft_id = Uuid::now_v7();
        let saved = save_draft_record(&pool, owner.user_id, owner.org_id, draft_id, 0, &form)
            .await
            .unwrap();
        let id = finalize_draft_record(
            &pool,
            owner.user_id,
            owner.org_id,
            draft_id,
            saved.revision,
            &form,
            false,
        )
        .await
        .unwrap();
        let loaded = load_editable_project(&pool, owner.user_id, owner.org_id, id)
            .await
            .unwrap()
            .form;
        assert_eq!(loaded.budget_mode, mode);
        assert_eq!(loaded.budget_value, form.budget_value);
        assert_eq!(
            loaded.tasks[0].budget,
            match mode {
                BudgetMode::HoursPerTask => "0:00",
                BudgetMode::FeesPerTask => "123.45",
                _ => "",
            }
        );
        assert_eq!(loaded.team[0].budget, form.team[0].budget);
    }
}
