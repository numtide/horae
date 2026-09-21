use crate::server_fns::test_seed::seed;
use horae_core::types::OrgRole;
use sqlx::PgPool;
use uuid::Uuid;

use super::{lock_creation_actor, lock_creation_client, validate_draft_form};
use crate::models::project_creation::{ProjectForm, ProjectMemberInput};

#[sqlx::test(migrations = "./migrations")]
async fn selected_catalog_is_scoped_deduplicated_and_redacts_manager_costs(pool: PgPool) {
    let owner = seed(&pool, OrgRole::Admin).await;
    let foreign = seed(&pool, OrgRole::Admin).await;
    sqlx::query!(
        "UPDATE users SET cost_rate_cents = 6200 WHERE id = $1",
        owner.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let task_ids = [
        owner.task_id,
        foreign.task_id,
        Uuid::now_v7(),
        owner.task_id,
    ];
    let user_ids = [
        owner.user_id,
        foreign.user_id,
        Uuid::now_v7(),
        owner.user_id,
    ];
    let selected =
        super::load_selected_catalog(&pool, owner.user_id, owner.org_id, &task_ids, &user_ids)
            .await
            .unwrap();
    assert_eq!(
        selected
            .tasks
            .iter()
            .map(|task| task.id)
            .collect::<Vec<_>>(),
        [owner.task_id]
    );
    assert_eq!(
        selected
            .people
            .iter()
            .map(|person| person.id)
            .collect::<Vec<_>>(),
        [owner.user_id]
    );
    assert_eq!(selected.people[0].cost_rate_cents, Some(6200));

    sqlx::query!(
        "UPDATE users SET org_role = 'manager' WHERE id = $1",
        owner.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let selected =
        super::load_selected_catalog(&pool, owner.user_id, owner.org_id, &task_ids, &user_ids)
            .await
            .unwrap();
    assert!(selected.people[0].cost_rate_cents.is_none());
    assert!(
        !serde_json::to_string(&selected)
            .unwrap()
            .contains("cost_rate_cents")
    );

    sqlx::query!(
        "UPDATE tasks SET active = false WHERE id = $1",
        owner.task_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let selected = super::load_selected_catalog(&pool, owner.user_id, owner.org_id, &task_ids, &[])
        .await
        .unwrap();
    assert!(selected.tasks.is_empty());
    assert!(selected.people.is_empty());
    sqlx::query!(
        "UPDATE users SET org_role = 'member' WHERE id = $1",
        owner.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(
        super::load_selected_catalog(&pool, owner.user_id, owner.org_id, &[], &[])
            .await
            .is_err()
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn selected_catalog_bounds_requests_and_excludes_inactive_people(pool: PgPool) {
    let owner = seed(&pool, OrgRole::Admin).await;
    let person_id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO users (id,org_id,email,name,org_role,active) VALUES ($1,$2,$3,'Inactive teammate','member',false)",
        person_id, owner.org_id, format!("{person_id}@example.test"),
    ).execute(&pool).await.unwrap();
    let selected =
        super::load_selected_catalog(&pool, owner.user_id, owner.org_id, &[], &[person_id])
            .await
            .unwrap();
    assert!(selected.people.is_empty());
    let oversized = vec![Uuid::now_v7(); 501];
    for (tasks, people) in [(&oversized[..], &[][..]), (&[][..], &oversized[..])] {
        let error = super::load_selected_catalog(&pool, owner.user_id, owner.org_id, tasks, people)
            .await
            .unwrap_err();
        assert!(matches!(
            error,
            super::ServerFnError::ServerError {
                code: super::BAD_REQUEST,
                ..
            }
        ));
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn selected_client_lookup_is_scoped_and_keeps_archived_identity(pool: PgPool) {
    let owner = seed(&pool, OrgRole::Manager).await;
    let foreign = seed(&pool, OrgRole::Admin).await;
    let client = super::load_selected_client(&pool, owner.user_id, owner.org_id, owner.client_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(client.name, "Acme");
    assert_eq!(client.currency, "EUR");
    for id in [foreign.client_id, Uuid::now_v7()] {
        assert!(
            super::load_selected_client(&pool, owner.user_id, owner.org_id, id)
                .await
                .unwrap()
                .is_none()
        );
    }
    sqlx::query!(
        "UPDATE clients SET active = false WHERE id = $1",
        owner.client_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let archived = super::load_selected_client(&pool, owner.user_id, owner.org_id, owner.client_id)
        .await
        .unwrap()
        .unwrap();
    assert!(!archived.active);
    assert_eq!(archived.id, owner.client_id);
    sqlx::query!(
        "UPDATE users SET org_role = 'member' WHERE id = $1",
        owner.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(
        super::load_selected_client(&pool, owner.user_id, owner.org_id, owner.client_id)
            .await
            .is_err()
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn missing_draft_responses_do_not_disclose_foreign_ownership(pool: PgPool) {
    let owner = seed(&pool, OrgRole::Admin).await;
    let other = seed(&pool, OrgRole::Admin).await;
    let id = Uuid::now_v7();
    super::save_draft_record(
        &pool,
        owner.user_id,
        owner.org_id,
        id,
        0,
        &ProjectForm::default(),
    )
    .await
    .unwrap();
    for requested in [id, Uuid::now_v7()] {
        let save = super::save_draft_record(
            &pool,
            other.user_id,
            other.org_id,
            requested,
            1,
            &ProjectForm::default(),
        )
        .await
        .unwrap_err();
        let discard = super::discard_draft_record(&pool, other.user_id, other.org_id, requested, 1)
            .await
            .unwrap_err();
        for error in [save, discard] {
            assert!(matches!(
                error,
                super::ServerFnError::ServerError {
                    code: super::NOT_FOUND,
                    ..
                }
            ));
        }
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn client_dialog_validates_details_and_persists_an_explicit_zero_rate(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    let client =
        super::create_client_record(&pool, ids.user_id, ids.org_id, "  New client  ", "USD", "0")
            .await
            .unwrap();
    assert_eq!(
        (
            client.name.as_str(),
            client.currency.as_str(),
            client.default_rate_cents
        ),
        ("New client", "USD", Some(0))
    );
    assert!(
        super::create_client_record(&pool, ids.user_id, ids.org_id, " ", "USD", "0")
            .await
            .is_err()
    );
    assert!(
        super::create_client_record(&pool, ids.user_id, ids.org_id, "Client", "XYZ", "0")
            .await
            .is_err()
    );
    assert!(
        super::create_client_record(&pool, ids.user_id, ids.org_id, "Client", "EUR", "-1")
            .await
            .is_err()
    );
    sqlx::query!(
        "UPDATE users SET org_role = 'member' WHERE id = $1",
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(
        super::create_client_record(&pool, ids.user_id, ids.org_id, "Client", "EUR", "")
            .await
            .is_err()
    );
    assert_eq!(
        sqlx::query_scalar!("SELECT count(*) FROM clients WHERE org_id = $1", ids.org_id)
            .fetch_one(&pool)
            .await
            .unwrap(),
        Some(2)
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn creation_options_are_scoped_bounded_and_redact_manager_costs(pool: PgPool) {
    use crate::models::project_creation::CreationSearch;
    let ids = seed(&pool, OrgRole::Admin).await;
    let _other = seed(&pool, OrgRole::Admin).await;
    sqlx::query!(
        "UPDATE users SET cost_rate_cents = 6200 WHERE id = $1",
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE projects SET code = '0009' WHERE id = $1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let options = super::load_creation_options(
        &pool,
        ids.user_id,
        ids.org_id,
        &CreationSearch::default(),
        false,
    )
    .await
    .unwrap();
    assert_eq!(options.clients.len(), 1);
    assert_eq!(options.tasks.len(), 1);
    assert_eq!(options.people.len(), 1);
    assert_eq!(options.people[0].cost_rate_cents, Some(6200));
    assert_eq!(options.suggested_code.as_deref(), Some("0010"));
    sqlx::query!(
        "UPDATE users SET org_role = 'manager' WHERE id = $1",
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let options = super::load_creation_options(
        &pool,
        ids.user_id,
        ids.org_id,
        &CreationSearch::default(),
        false,
    )
    .await
    .unwrap();
    assert!(!options.can_edit_private_settings);
    assert!(options.people[0].cost_rate_cents.is_none());
    for _ in 0..51 {
        sqlx::query!(
            "INSERT INTO clients (id,org_id,name,currency) VALUES ($1,$2,'Searchable','EUR')",
            Uuid::now_v7(),
            ids.org_id
        )
        .execute(&pool)
        .await
        .unwrap();
    }
    let mut search = CreationSearch::default();
    search.clients.query = "Searchable".into();
    let options = super::load_creation_options(&pool, ids.user_id, ids.org_id, &search, false)
        .await
        .unwrap();
    assert_eq!(options.clients.len(), 50);
    assert!(options.more_clients);
    search.clients.offset = 50;
    let options = super::load_creation_options(&pool, ids.user_id, ids.org_id, &search, false)
        .await
        .unwrap();
    assert_eq!(options.clients.len(), 1);
    assert!(!options.more_clients);
    search.clients.query = "%".into();
    search.clients.offset = 0;
    assert!(
        super::load_creation_options(&pool, ids.user_id, ids.org_id, &search, false)
            .await
            .unwrap()
            .clients
            .is_empty()
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn finalization_persists_task_access_and_project_only_financial_overrides(pool: PgPool) {
    use crate::models::project_creation::{ProjectTaskInput, TaskAccess, TaskSource};
    use horae_core::project::{BudgetMode, RateMode};
    let ids = seed(&pool, OrgRole::Admin).await;
    let id = Uuid::now_v7();
    let form = ProjectForm {
        client_id: Some(ids.client_id),
        name: "Project".into(),
        admin_notes: "Private context".into(),
        rate_mode: RateMode::Task,
        budget_mode: BudgetMode::HoursPerTask,
        team: vec![ProjectMemberInput {
            user_id: ids.user_id,
            manager: true,
            billable_rate: "999".into(),
            cost_rate: "62.25".into(),
            budget: String::new(),
        }],
        tasks: vec![ProjectTaskInput {
            id: Uuid::now_v7(),
            source: TaskSource::Existing {
                task_id: ids.task_id,
            },
            billable: true,
            rate: "95.50".into(),
            budget: "7:30".into(),
            access: TaskAccess::Restricted {
                user_ids: vec![ids.user_id, ids.user_id],
            },
        }],
        ..Default::default()
    };
    super::save_draft_record(&pool, ids.user_id, ids.org_id, id, 0, &form)
        .await
        .unwrap();
    let project = super::finalize_draft_record(&pool, ids.user_id, ids.org_id, id, 1, &form, false)
        .await
        .unwrap();
    let task = sqlx::query!("SELECT p.rate_cents, s.restricted, s.budget_minutes FROM project_tasks p JOIN project_task_settings s USING(project_id,task_id) WHERE p.project_id = $1", project).fetch_one(&pool).await.unwrap();
    assert_eq!(
        (task.rate_cents, task.restricted, task.budget_minutes),
        (Some(9550), true, Some(450))
    );
    assert_eq!(
        sqlx::query_scalar!(
            "SELECT count(*) FROM project_task_members WHERE project_id = $1",
            project
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        Some(1)
    );
    assert_eq!(
        sqlx::query_scalar!(
            "SELECT cost_rate_cents FROM project_member_costs WHERE project_id = $1",
            project
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        6225
    );
    assert_eq!(
        sqlx::query_scalar!(
            "SELECT admin_notes FROM project_private_settings WHERE project_id = $1",
            project
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        "Private context"
    );
    let event = sqlx::query_scalar!(
        "SELECT payload FROM horae_outbox WHERE org_id = $1 AND event_kind = 'project_created'",
        ids.org_id,
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        event["project"],
        serde_json::json!({
            "id": project,
            "client_id": ids.client_id,
            "name": "Project",
            "project_type": "time_and_materials",
            "budget_kind": "hours",
            "active": true,
        })
    );
    assert_eq!(event.as_object().unwrap().len(), 4);
    assert_eq!(
        sqlx::query_scalar!(
            "SELECT rate_cents FROM assignments WHERE project_id = $1",
            project
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        None
    );
    assert_eq!(
        sqlx::query_scalar!(
            "SELECT cost_rate_cents FROM users WHERE id = $1",
            ids.user_id
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        None
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn finalization_rechecks_client_role_revision_and_rate_currency(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Admin).await;
    let foreign = seed(&pool, OrgRole::Admin).await;
    let id = Uuid::now_v7();
    let mut form = ProjectForm {
        client_id: Some(ids.client_id),
        name: "Project".into(),
        ..Default::default()
    };
    super::save_draft_record(&pool, ids.user_id, ids.org_id, id, 0, &form)
        .await
        .unwrap();
    assert!(
        super::finalize_draft_record(&pool, ids.user_id, ids.org_id, id, 2, &form, false)
            .await
            .is_err()
    );
    form.client_id = Some(foreign.client_id);
    assert!(
        super::finalize_draft_record(&pool, ids.user_id, ids.org_id, id, 1, &form, false)
            .await
            .is_err()
    );
    form.client_id = Some(ids.client_id);
    sqlx::query!(
        "UPDATE clients SET active = false WHERE id = $1",
        ids.client_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(
        super::finalize_draft_record(&pool, ids.user_id, ids.org_id, id, 1, &form, false)
            .await
            .is_err()
    );
    sqlx::query!(
        "UPDATE clients SET active = true WHERE id = $1",
        ids.client_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE users SET org_role = 'member' WHERE id = $1",
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(
        super::finalize_draft_record(&pool, ids.user_id, ids.org_id, id, 1, &form, false)
            .await
            .is_err()
    );
    sqlx::query!(
        "UPDATE users SET org_role = 'admin', billable_rate_cents = 9500 WHERE id = $1",
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    form.currency = Some("USD".into());
    form.team.push(ProjectMemberInput {
        user_id: ids.user_id,
        manager: false,
        billable_rate: String::new(),
        cost_rate: String::new(),
        budget: String::new(),
    });
    assert!(
        super::finalize_draft_record(&pool, ids.user_id, ids.org_id, id, 1, &form, false)
            .await
            .is_err()
    );
    form.team[0].billable_rate = "0".into();
    assert!(
        super::finalize_draft_record(&pool, ids.user_id, ids.org_id, id, 1, &form, false)
            .await
            .is_ok()
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn finalize_is_atomic_and_repeated_submission_returns_one_project(pool: PgPool) {
    use horae_core::types::ProjectType;
    let ids = seed(&pool, OrgRole::Admin).await;
    for project_type in [
        ProjectType::TimeAndMaterials,
        ProjectType::FixedFee,
        ProjectType::NonBillable,
    ] {
        let id = Uuid::now_v7();
        let form = ProjectForm {
            client_id: Some(ids.client_id),
            name: "  New project  ".into(),
            code: "ABC-1".into(),
            project_type,
            fee_amount: "100.25".into(),
            tags: vec!["Platform".into(), "platform".into()],
            ..Default::default()
        };
        super::save_draft_record(&pool, ids.user_id, ids.org_id, id, 0, &form)
            .await
            .unwrap();
        let (a, b) = tokio::join!(
            super::finalize_draft_record(&pool, ids.user_id, ids.org_id, id, 1, &form, false),
            super::finalize_draft_record(&pool, ids.user_id, ids.org_id, id, 1, &form, false),
        );
        let project = a.unwrap();
        assert_eq!(b.unwrap(), project);
        let row = sqlx::query!("SELECT p.name, p.code, p.currency, s.fee_amount_cents FROM projects p JOIN project_settings s ON s.project_id = p.id WHERE p.id = $1", project).fetch_one(&pool).await.unwrap();
        assert_eq!(
            (
                row.name.as_str(),
                row.code.as_deref(),
                row.currency.as_str()
            ),
            ("New project", Some("ABC-1"), "EUR")
        );
        assert_eq!(
            row.fee_amount_cents,
            (project_type == ProjectType::FixedFee).then_some(10025)
        );
        assert!(
            super::save_draft_record(&pool, ids.user_id, ids.org_id, id, 1, &form)
                .await
                .is_err()
        );
        assert!(
            super::discard_draft_record(&pool, ids.user_id, ids.org_id, id, 1)
                .await
                .is_err()
        );
    }
    assert_eq!(
        sqlx::query_scalar!(
            "SELECT count(*) FROM projects WHERE org_id = $1",
            ids.org_id
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        Some(4)
    );
    assert_eq!(sqlx::query_scalar!("SELECT count(*) FROM horae_outbox WHERE org_id = $1 AND event_kind = 'project_created'", ids.org_id).fetch_one(&pool).await.unwrap(), Some(3));
    let state = crate::state::AppState::new(
        pool.clone(),
        std::sync::Arc::new(crate::plugin::PluginRegistry::empty()),
    );
    let worker = crate::jobs::outbox::spawn(&state);
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        loop {
            let delivered = sqlx::query_scalar!(
                "SELECT count(*) FROM horae_outbox WHERE org_id = $1 AND event_kind = 'project_created' AND delivered_at IS NOT NULL",
                ids.org_id,
            ).fetch_one(&pool).await.unwrap();
            if delivered == Some(3) {
                break;
            }
            tokio::task::yield_now().await;
        }
    }).await.unwrap();
    worker
        .shutdown(std::time::Duration::from_secs(5))
        .await
        .unwrap();
    assert_eq!(
        sqlx::query_scalar!(
            "SELECT count(*) FROM project_tags WHERE org_id = $1",
            ids.org_id
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        Some(1)
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn invalid_task_rolls_back_project_and_keeps_the_draft(pool: PgPool) {
    use crate::models::project_creation::{ProjectTaskInput, TaskAccess, TaskSource};
    let ids = seed(&pool, OrgRole::Admin).await;
    let other = seed(&pool, OrgRole::Admin).await;
    let id = Uuid::now_v7();
    let mut form = ProjectForm {
        client_id: Some(ids.client_id),
        name: "New project".into(),
        ..Default::default()
    };
    form.tasks.push(ProjectTaskInput {
        id: Uuid::now_v7(),
        source: TaskSource::New {
            name: "Created before error".into(),
        },
        billable: true,
        rate: String::new(),
        budget: String::new(),
        access: TaskAccess::Everyone,
    });
    form.tasks.push(ProjectTaskInput {
        id: Uuid::now_v7(),
        source: TaskSource::Existing {
            task_id: other.task_id,
        },
        billable: true,
        rate: String::new(),
        budget: String::new(),
        access: TaskAccess::Everyone,
    });
    super::save_draft_record(&pool, ids.user_id, ids.org_id, id, 0, &form)
        .await
        .unwrap();
    assert!(
        super::finalize_draft_record(&pool, ids.user_id, ids.org_id, id, 1, &form, false)
            .await
            .is_err()
    );
    assert_eq!(
        sqlx::query_scalar!(
            "SELECT count(*) FROM projects WHERE org_id = $1",
            ids.org_id
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        Some(1)
    );
    assert_eq!(
        sqlx::query_scalar!("SELECT count(*) FROM tasks WHERE org_id = $1", ids.org_id)
            .fetch_one(&pool)
            .await
            .unwrap(),
        Some(1)
    );
    assert_eq!(
        super::load_draft_record(&pool, ids.user_id, ids.org_id)
            .await
            .unwrap()
            .unwrap()
            .revision,
        1
    );
}

#[test]
fn final_validation_requires_real_details_and_ignores_hidden_type_settings() {
    use horae_core::{project::RateMode, types::ProjectType};
    let mut form = ProjectForm::default();
    assert!(super::validate_project_form(&form, "EUR", false).is_err());
    form.name = "  New project  ".into();
    assert!(super::validate_project_form(&form, "EUR", false).is_ok());
    form.starts_on = "2026-09-30".into();
    form.ends_on = "2026-09-01".into();
    assert!(super::validate_project_form(&form, "EUR", false).is_err());
    form.ends_on.clear();
    form.rate_mode = RateMode::Project;
    assert!(super::validate_project_form(&form, "EUR", false).is_err());
    form.project_rate = "0".into();
    assert_eq!(
        super::validate_project_form(&form, "EUR", false)
            .unwrap()
            .rate_cents,
        Some(0)
    );
    form.project_type = ProjectType::NonBillable;
    form.project_rate = "not an amount".into();
    form.fee_amount = "not an amount".into();
    assert_eq!(
        super::validate_project_form(&form, "EUR", false)
            .unwrap()
            .rate_cents,
        None
    );
}

#[test]
fn final_validation_preserves_exact_budgets_and_rejects_unavailable_email() {
    use horae_core::project::BudgetMode;
    let mut form = ProjectForm {
        name: "Project".into(),
        budget_mode: BudgetMode::TotalHours,
        budget_value: "7:30".into(),
        ..Default::default()
    };
    assert_eq!(
        super::validate_project_form(&form, "EUR", false)
            .unwrap()
            .budget_minutes,
        Some(450)
    );
    form.budget_alert = true;
    assert!(super::validate_project_form(&form, "EUR", false).is_err());
    assert!(super::validate_project_form(&form, "EUR", true).is_ok());
    form.budget_alert_at = "101".into();
    assert!(super::validate_project_form(&form, "EUR", true).is_err());
    form.budget_alert = false;
    form.budget_mode = BudgetMode::TotalFees;
    form.budget_value = "1200.05".into();
    assert_eq!(
        super::validate_project_form(&form, "EUR", false)
            .unwrap()
            .budget_cents,
        Some(120005)
    );
}

#[test]
fn final_validation_checks_invoice_defaults_and_scoped_budget_totals() {
    use horae_core::project::BudgetMode;
    let mut form = ProjectForm {
        name: "Project".into(),
        budget_mode: BudgetMode::HoursPerPerson,
        team: vec![ProjectMemberInput {
            user_id: Uuid::now_v7(),
            manager: false,
            billable_rate: String::new(),
            cost_rate: String::new(),
            budget: "1.5".into(),
        }],
        ..Default::default()
    };
    let settings = super::validate_project_form(&form, "EUR", false).unwrap();
    assert_eq!(settings.budget_minutes, Some(90));
    form.invoice_defaults.tax = "21.25".into();
    form.invoice_defaults.discount = "5".into();
    let settings = super::validate_project_form(&form, "EUR", false).unwrap();
    assert_eq!((settings.tax1_bps, settings.discount_bps), (2125, 500));
    form.invoice_defaults.terms_days = "366".into();
    assert!(super::validate_project_form(&form, "EUR", false).is_err());
}

#[test]
fn final_validation_requires_complete_fee_schedules_and_distinct_entities() {
    use crate::models::project_creation::{FeeMode, MilestoneInput};
    use horae_core::types::ProjectType;
    let mut form = ProjectForm {
        name: "Project".into(),
        project_type: ProjectType::FixedFee,
        fee_mode: FeeMode::Milestones,
        ..Default::default()
    };
    assert!(super::validate_project_form(&form, "EUR", false).is_err());
    let milestone = MilestoneInput {
        id: Uuid::now_v7(),
        name: "Delivery".into(),
        due_on: "2026-12-31".into(),
        amount: "400.25".into(),
    };
    form.milestones.push(milestone.clone());
    assert!(super::validate_project_form(&form, "EUR", false).is_ok());
    form.milestones.push(milestone);
    assert!(super::validate_project_form(&form, "EUR", false).is_err());
    form.milestones.pop();
    form.tags = vec![" Platform ".into(), "platform".into(), "Q3".into()];
    assert_eq!(
        super::validate_project_form(&form, "EUR", false)
            .unwrap()
            .tags,
        ["Platform", "Q3"]
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn draft_save_retries_do_not_overwrite_a_newer_revision(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Admin).await;
    let id = Uuid::now_v7();
    let mut form = ProjectForm {
        name: "Original".into(),
        ..Default::default()
    };
    let saved = super::save_draft_record(&pool, ids.user_id, ids.org_id, id, 0, &form)
        .await
        .unwrap();
    assert_eq!(saved.revision, 1);
    let retry = super::save_draft_record(&pool, ids.user_id, ids.org_id, id, 0, &form)
        .await
        .unwrap();
    assert_eq!(retry, saved);
    form.name = "Updated".into();
    let saved = super::save_draft_record(&pool, ids.user_id, ids.org_id, id, 1, &form)
        .await
        .unwrap();
    assert_eq!(saved.revision, 2);
    assert_eq!(
        super::save_draft_record(&pool, ids.user_id, ids.org_id, id, 1, &form)
            .await
            .unwrap(),
        saved
    );
    form.name = "Stale tab".into();
    assert!(
        super::save_draft_record(&pool, ids.user_id, ids.org_id, id, 1, &form)
            .await
            .is_err()
    );
    let draft = super::load_draft_record(&pool, ids.user_id, ids.org_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(draft.form.name, "Updated");
    assert_eq!(draft.revision, 2);
}

#[sqlx::test(migrations = "./migrations")]
async fn draft_ownership_discard_and_role_changes_are_enforced(pool: PgPool) {
    let owner = seed(&pool, OrgRole::Admin).await;
    let other = seed(&pool, OrgRole::Admin).await;
    let id = Uuid::now_v7();
    let form = ProjectForm {
        client_id: Some(owner.client_id),
        name: "Private configuration".into(),
        admin_notes: "Confidential".into(),
        team: vec![ProjectMemberInput {
            user_id: owner.user_id,
            manager: true,
            billable_rate: String::new(),
            cost_rate: "62.25".into(),
            budget: String::new(),
        }],
        ..Default::default()
    };
    assert!(
        super::load_draft_record(&pool, owner.user_id, owner.org_id)
            .await
            .unwrap()
            .is_none()
    );
    super::save_draft_record(&pool, owner.user_id, owner.org_id, id, 0, &form)
        .await
        .unwrap();
    assert!(
        super::save_draft_record(&pool, other.user_id, other.org_id, id, 1, &form)
            .await
            .is_err()
    );
    assert!(
        super::discard_draft_record(&pool, other.user_id, other.org_id, id, 1)
            .await
            .is_err()
    );
    assert!(
        super::save_draft_record(&pool, owner.user_id, owner.org_id, Uuid::now_v7(), 0, &form)
            .await
            .is_err()
    );
    sqlx::query!(
        "UPDATE users SET org_role = 'manager' WHERE id = $1",
        owner.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let draft = super::load_draft_record(&pool, owner.user_id, owner.org_id)
        .await
        .unwrap()
        .unwrap();
    assert!(draft.form.admin_notes.is_empty());
    assert!(draft.form.team[0].cost_rate.is_empty());
    let payload = serde_json::to_string(&draft).unwrap();
    assert!(!payload.contains("Confidential"));
    assert!(!payload.contains("62.25"));
    for result in [
        super::save_draft_record(&pool, owner.user_id, owner.org_id, id, 1, &form)
            .await
            .map(|_| ()),
        super::finalize_draft_record(&pool, owner.user_id, owner.org_id, id, 1, &form, false)
            .await
            .map(|_| ()),
    ] {
        assert!(matches!(
            result,
            Err(super::ServerFnError::ServerError {
                code: super::FORBIDDEN,
                ..
            })
        ));
    }
    assert_eq!(
        sqlx::query_scalar!(
            "SELECT count(*) FROM projects WHERE org_id = $1",
            owner.org_id
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        Some(1),
    );
    assert!(
        super::discard_draft_record(&pool, owner.user_id, owner.org_id, id, 2)
            .await
            .is_err()
    );
    super::discard_draft_record(&pool, owner.user_id, owner.org_id, id, 1)
        .await
        .unwrap();
    assert!(
        super::load_draft_record(&pool, owner.user_id, owner.org_id)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        super::save_draft_record(
            &pool,
            owner.user_id,
            owner.org_id,
            id,
            1,
            &ProjectForm::default()
        )
        .await
        .is_err()
    );
    super::save_draft_record(
        &pool,
        owner.user_id,
        owner.org_id,
        Uuid::now_v7(),
        0,
        &ProjectForm::default(),
    )
    .await
    .unwrap();
}

#[sqlx::test(migrations = "./migrations")]
async fn simultaneous_draft_updates_have_one_winner(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Admin).await;
    let id = Uuid::now_v7();
    super::save_draft_record(
        &pool,
        ids.user_id,
        ids.org_id,
        id,
        0,
        &ProjectForm::default(),
    )
    .await
    .unwrap();
    let left = ProjectForm {
        name: "Left".into(),
        ..Default::default()
    };
    let right = ProjectForm {
        name: "Right".into(),
        ..Default::default()
    };
    let (a, b) = tokio::join!(
        super::save_draft_record(&pool, ids.user_id, ids.org_id, id, 1, &left),
        super::save_draft_record(&pool, ids.user_id, ids.org_id, id, 1, &right),
    );
    assert_ne!(a.is_ok(), b.is_ok());
    assert_eq!(
        super::load_draft_record(&pool, ids.user_id, ids.org_id)
            .await
            .unwrap()
            .unwrap()
            .revision,
        2
    );
}

#[test]
fn draft_validation_preserves_incomplete_input_but_bounds_its_size() {
    let form = ProjectForm {
        starts_on: "2026-".into(),
        ..Default::default()
    };
    assert_eq!(
        validate_draft_form(&form, true).unwrap()["starts_on"],
        "2026-"
    );
    let oversized = ProjectForm {
        name: "x".repeat(256 * 1024),
        ..Default::default()
    };
    assert!(validate_draft_form(&oversized, true).is_err());
    let too_many_tags = ProjectForm {
        tags: vec!["tag".into(); 51],
        ..Default::default()
    };
    assert!(validate_draft_form(&too_many_tags, true).is_err());
    let null_character = ProjectForm {
        admin_notes: "before\0after".into(),
        ..Default::default()
    };
    assert!(validate_draft_form(&null_character, true).is_err());
}

#[test]
fn managers_cannot_store_administrator_notes_or_costs_in_a_draft() {
    let notes = ProjectForm {
        admin_notes: "Private".into(),
        ..Default::default()
    };
    assert!(validate_draft_form(&notes, false).is_err());
    assert!(validate_draft_form(&notes, true).is_ok());
    let costs = ProjectForm {
        team: vec![ProjectMemberInput {
            user_id: Uuid::now_v7(),
            manager: false,
            billable_rate: String::new(),
            cost_rate: "0".into(),
            budget: String::new(),
        }],
        ..Default::default()
    };
    assert!(validate_draft_form(&costs, false).is_err());
    assert!(validate_draft_form(&costs, true).is_ok());
}

#[sqlx::test(migrations = "./migrations")]
async fn creation_actor_is_revalidated_inside_the_transaction(pool: PgPool) {
    let admin = seed(&pool, OrgRole::Admin).await;
    let member = seed(&pool, OrgRole::Member).await;
    let mut tx = pool.begin().await.unwrap();
    assert_eq!(
        lock_creation_actor(&mut tx, admin.user_id, admin.org_id)
            .await
            .unwrap(),
        OrgRole::Admin
    );
    assert!(
        lock_creation_actor(&mut tx, member.user_id, member.org_id)
            .await
            .is_err()
    );
    assert!(
        lock_creation_actor(&mut tx, admin.user_id, member.org_id)
            .await
            .is_err()
    );
    tx.rollback().await.unwrap();
    sqlx::query!(
        "UPDATE users SET active = false WHERE id = $1",
        admin.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let mut tx = pool.begin().await.unwrap();
    assert!(
        lock_creation_actor(&mut tx, admin.user_id, admin.org_id)
            .await
            .is_err()
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn creation_client_must_be_active_and_in_the_same_organization(pool: PgPool) {
    let owner = seed(&pool, OrgRole::Admin).await;
    let other = seed(&pool, OrgRole::Admin).await;
    let mut tx = pool.begin().await.unwrap();
    assert_eq!(
        lock_creation_client(&mut tx, owner.client_id, owner.org_id)
            .await
            .unwrap()
            .currency,
        "EUR"
    );
    assert!(
        lock_creation_client(&mut tx, other.client_id, owner.org_id)
            .await
            .is_err()
    );
    tx.rollback().await.unwrap();
    sqlx::query!(
        "UPDATE clients SET active = false WHERE id = $1",
        owner.client_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let mut tx = pool.begin().await.unwrap();
    assert!(
        lock_creation_client(&mut tx, owner.client_id, owner.org_id)
            .await
            .is_err()
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn draft_schema_enforces_one_current_draft_per_creator(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Admin).await;
    let draft = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO project_drafts (id, org_id, creator_id, payload) VALUES ($1, $2, $3, '{}')",
        draft,
        ids.org_id,
        ids.user_id,
    )
    .execute(&pool)
    .await
    .unwrap();
    let duplicate = sqlx::query!(
        "INSERT INTO project_drafts (id, org_id, creator_id, payload) VALUES ($1, $2, $3, '{}')",
        Uuid::now_v7(),
        ids.org_id,
        ids.user_id,
    )
    .execute(&pool)
    .await
    .unwrap_err();
    assert!(duplicate.as_database_error().unwrap().is_unique_violation());
    sqlx::query!(
        "UPDATE project_drafts SET discarded_at = now() WHERE id = $1",
        draft
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO project_drafts (id, org_id, creator_id, payload) VALUES ($1, $2, $3, '{}')",
        Uuid::now_v7(),
        ids.org_id,
        ids.user_id,
    )
    .execute(&pool)
    .await
    .unwrap();
}

#[sqlx::test(migrations = "./migrations")]
async fn draft_schema_rejects_foreign_creator_and_completed_project(pool: PgPool) {
    let owner = seed(&pool, OrgRole::Admin).await;
    let other = seed(&pool, OrgRole::Admin).await;
    let foreign_owner = sqlx::query!(
        "INSERT INTO project_drafts (id, org_id, creator_id, payload) VALUES ($1, $2, $3, '{}')",
        Uuid::now_v7(),
        owner.org_id,
        other.user_id,
    )
    .execute(&pool)
    .await
    .unwrap_err();
    assert!(
        foreign_owner
            .as_database_error()
            .unwrap()
            .is_foreign_key_violation()
    );
    let foreign_project = sqlx::query!(
        "INSERT INTO project_drafts (id, org_id, creator_id, payload, completed_project_id) VALUES ($1, $2, $3, '{}', $4)",
        Uuid::now_v7(), owner.org_id, owner.user_id, other.project_id,
    ).execute(&pool).await.unwrap_err();
    assert!(
        foreign_project
            .as_database_error()
            .unwrap()
            .is_foreign_key_violation()
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn draft_schema_rejects_oversize_payload_and_invalid_revision(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Admin).await;
    for (revision, payload) in [
        (0_i64, serde_json::json!({})),
        (1, serde_json::json!([])),
        (1, serde_json::json!({"text": "x".repeat(256 * 1024)})),
    ] {
        let error = sqlx::query!(
            "INSERT INTO project_drafts (id, org_id, creator_id, revision, payload) VALUES ($1, $2, $3, $4, $5)",
            Uuid::now_v7(), ids.org_id, ids.user_id, revision, payload,
        ).execute(&pool).await.unwrap_err();
        assert!(error.as_database_error().unwrap().is_check_violation());
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn existing_projects_have_no_implicit_new_billing_configuration(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Admin).await;
    let count = sqlx::query_scalar!(
        "SELECT count(*) FROM project_settings WHERE project_id = $1",
        ids.project_id,
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(count, Some(0));
}

#[sqlx::test(migrations = "./migrations")]
async fn billing_schema_rejects_partial_fee_schedules(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Admin).await;
    for (mode, amount, day) in [
        (None, Some(1_i64), None),
        (None, None, Some("first")),
        (Some("monthly"), Some(1), None),
        (Some("single"), None, None),
        (Some("milestones"), Some(1), None),
    ] {
        let error = sqlx::query!(
            "INSERT INTO project_settings (id, org_id, project_id, creator_id, rate_mode, fee_mode, fee_amount_cents, monthly_day) VALUES ($1, $2, $3, $4, 'person', $5, $6, $7)",
            Uuid::now_v7(), ids.org_id, ids.project_id, ids.user_id, mode, amount, day,
        ).execute(&pool).await.unwrap_err();
        assert!(error.as_database_error().unwrap().is_check_violation());
    }
}
