pub(super) use crate::ops::testing::become_profile;
use crate::server_fns::test_seed::seed;
use horae_core::permissions::catalog::BuiltInProfile;
use sqlx::PgPool;
use uuid::Uuid;

use super::{
    OrganizationLock, PrivateFields, lock_creation_actor, lock_creation_client, validate_draft_form,
};
use crate::models::project_creation::{ProjectForm, ProjectMemberInput};

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn selected_catalog_is_scoped_deduplicated_and_redacts_manager_costs(pool: PgPool) {
    let owner = seed(&pool, BuiltInProfile::Administrator).await;
    let foreign = seed(&pool, BuiltInProfile::Administrator).await;
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

    // An Executive Manager reads cost rates; a Project Manager does not.
    become_profile(&pool, &owner, BuiltInProfile::ExecutiveManager).await;
    let selected =
        super::load_selected_catalog(&pool, owner.user_id, owner.org_id, &task_ids, &user_ids)
            .await
            .unwrap();
    assert_eq!(selected.people[0].cost_rate_cents, Some(6200));
    become_profile(&pool, &owner, BuiltInProfile::ProjectManager).await;
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
    become_profile(&pool, &owner, BuiltInProfile::Member).await;
    assert!(
        super::load_selected_catalog(&pool, owner.user_id, owner.org_id, &[], &[])
            .await
            .is_err()
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn selected_catalog_bounds_requests_and_excludes_inactive_people(pool: PgPool) {
    let owner = seed(&pool, BuiltInProfile::Administrator).await;
    let person_id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO users (id,org_id,email,name,active) VALUES ($1,$2,$3,'Inactive teammate',false)",
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

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn selected_client_lookup_is_scoped_and_keeps_archived_identity(pool: PgPool) {
    let owner = seed(&pool, BuiltInProfile::ExecutiveManager).await;
    let foreign = seed(&pool, BuiltInProfile::Administrator).await;
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
    become_profile(&pool, &owner, BuiltInProfile::Member).await;
    assert!(
        super::load_selected_client(&pool, owner.user_id, owner.org_id, owner.client_id)
            .await
            .is_err()
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn missing_draft_responses_do_not_disclose_foreign_ownership(pool: PgPool) {
    let owner = seed(&pool, BuiltInProfile::Administrator).await;
    let other = seed(&pool, BuiltInProfile::Administrator).await;
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

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn client_dialog_validates_details_and_persists_an_explicit_zero_rate(pool: PgPool) {
    use crate::ops::testing::{seed_profile, set_grants};
    let ids = seed_profile(&pool, BuiltInProfile::Administrator).await;
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
    set_grants(&mut pool.acquire().await.unwrap(), &ids, &[])
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

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn creation_options_are_scoped_bounded_and_redact_manager_costs(pool: PgPool) {
    use crate::models::project_creation::CreationSearch;
    let ids = seed(&pool, BuiltInProfile::Administrator).await;
    let _other = seed(&pool, BuiltInProfile::Administrator).await;
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
    assert!(
        options.can_edit_cost_rates
            && options.can_edit_private_notes
            && options.can_edit_billable_settings
    );
    // Private notes follow `ProjectWriteAll`, cost rates `CostRateWriteAll`
    // and billable rates `BillableRateWriteAll`, which only Administrator has.
    for (profile, notes, costs) in [
        (BuiltInProfile::ExecutiveManager, true, Some(6200)),
        (BuiltInProfile::ProjectManager, false, None),
    ] {
        become_profile(&pool, &ids, profile).await;
        let options = super::load_creation_options(
            &pool,
            ids.user_id,
            ids.org_id,
            &CreationSearch::default(),
            false,
        )
        .await
        .unwrap();
        assert!(!options.can_edit_cost_rates, "{profile:?}");
        assert!(!options.can_edit_billable_settings, "{profile:?}");
        assert_eq!(options.can_edit_private_notes, notes, "{profile:?}");
        assert_eq!(options.people[0].cost_rate_cents, costs, "{profile:?}");
    }
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

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn creation_options_offer_active_common_tasks_of_this_organization_only(pool: PgPool) {
    use crate::models::project_creation::CreationSearch;
    let ids = seed(&pool, BuiltInProfile::Administrator).await;
    let foreign = seed(&pool, BuiltInProfile::Administrator).await;
    let (common, archived) = (Uuid::now_v7(), Uuid::now_v7());
    sqlx::query!(
        "INSERT INTO tasks (id,org_id,name,billable_default,is_default,active,default_rate_cents,default_rate_currency)
         VALUES ($1,$3,'Meetings',false,true,true,4000,'EUR'), ($2,$3,'Old common',true,true,false,NULL,NULL)",
        common,
        archived,
        ids.org_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE tasks SET is_default=true WHERE id=$1",
        foreign.task_id
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
    assert_eq!(
        options
            .common_tasks
            .iter()
            .map(|task| (task.id, task.billable, task.default_rate_cents))
            .collect::<Vec<_>>(),
        [(common, false, Some(4000))]
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn finalization_persists_task_access_and_project_only_financial_overrides(pool: PgPool) {
    use crate::models::project_creation::{ProjectTaskInput, TaskAccess, TaskSource};
    use horae_core::project::{BudgetMode, RateMode};
    let ids = seed(&pool, BuiltInProfile::Administrator).await;
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
    assert_eq!(
        sqlx::query_scalar!(
            "SELECT manager_id FROM project_management_assignments WHERE project_id = $1",
            project
        )
        .fetch_all(&pool)
        .await
        .unwrap(),
        [ids.user_id]
    );
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

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn finalization_rechecks_client_role_revision_and_rate_currency(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Administrator).await;
    let foreign = seed(&pool, BuiltInProfile::Administrator).await;
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
    become_profile(&pool, &ids, BuiltInProfile::Member).await;
    assert!(
        super::finalize_draft_record(&pool, ids.user_id, ids.org_id, id, 1, &form, false)
            .await
            .is_err()
    );
    become_profile(&pool, &ids, BuiltInProfile::Administrator).await;
    sqlx::query!(
        "UPDATE users SET billable_rate_cents = 9500 WHERE id = $1",
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

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn finalize_is_atomic_and_repeated_submission_returns_one_project(pool: PgPool) {
    use horae_core::types::ProjectType;
    let ids = seed(&pool, BuiltInProfile::Administrator).await;
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

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn invalid_task_rolls_back_project_and_keeps_the_draft(pool: PgPool) {
    use crate::models::project_creation::{ProjectTaskInput, TaskAccess, TaskSource};
    let ids = seed(&pool, BuiltInProfile::Administrator).await;
    let other = seed(&pool, BuiltInProfile::Administrator).await;
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

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn draft_save_retries_do_not_overwrite_a_newer_revision(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Administrator).await;
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

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn draft_ownership_discard_and_permission_changes_are_enforced(pool: PgPool) {
    let owner = seed(&pool, BuiltInProfile::Administrator).await;
    let other = seed(&pool, BuiltInProfile::Administrator).await;
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
    // The creator keeps project creation but loses both private-field grants.
    become_profile(&pool, &owner, BuiltInProfile::ProjectManager).await;
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

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn simultaneous_draft_updates_have_one_winner(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Administrator).await;
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
    let all = PrivateFields {
        read_costs: true,
        read_billable: true,
        write_costs: true,
        write_notes: true,
        write_billable: true,
    };
    let form = ProjectForm {
        starts_on: "2026-".into(),
        ..Default::default()
    };
    assert_eq!(
        validate_draft_form(&form, all).unwrap()["starts_on"],
        "2026-"
    );
    let oversized = ProjectForm {
        name: "x".repeat(256 * 1024),
        ..Default::default()
    };
    assert!(validate_draft_form(&oversized, all).is_err());
    let too_many_tags = ProjectForm {
        tags: vec!["tag".into(); 51],
        ..Default::default()
    };
    assert!(validate_draft_form(&too_many_tags, all).is_err());
    let null_character = ProjectForm {
        admin_notes: "before\0after".into(),
        ..Default::default()
    };
    assert!(validate_draft_form(&null_character, all).is_err());
}

#[test]
fn managers_cannot_store_administrator_notes_or_costs_in_a_draft() {
    let notes = ProjectForm {
        admin_notes: "Private".into(),
        ..Default::default()
    };
    let none = PrivateFields {
        read_costs: true,
        read_billable: true,
        write_costs: false,
        write_notes: false,
        write_billable: false,
    };
    let notes_only = PrivateFields {
        write_notes: true,
        ..none
    };
    let costs_only = PrivateFields {
        write_costs: true,
        ..none
    };
    assert!(validate_draft_form(&notes, none).is_err());
    assert!(validate_draft_form(&notes, costs_only).is_err());
    assert!(validate_draft_form(&notes, notes_only).is_ok());
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
    assert!(validate_draft_form(&costs, none).is_err());
    assert!(validate_draft_form(&costs, notes_only).is_err());
    assert!(validate_draft_form(&costs, costs_only).is_ok());
}

#[test]
fn billable_settings_in_a_draft_need_the_billable_rate_grant() {
    use crate::models::project_creation::{
        InvoiceDefaultsInput, ProjectTaskInput, TaskAccess, TaskSource,
    };
    use horae_core::project::BudgetMode;
    use horae_core::types::ProjectType;
    let without = PrivateFields {
        read_costs: false,
        read_billable: false,
        write_costs: false,
        write_notes: false,
        write_billable: false,
    };
    let with = PrivateFields {
        write_billable: true,
        ..without
    };
    let blank_member = ProjectMemberInput {
        user_id: Uuid::now_v7(),
        manager: false,
        billable_rate: String::new(),
        cost_rate: String::new(),
        budget: String::new(),
    };
    let blank_task = ProjectTaskInput {
        id: Uuid::now_v7(),
        source: TaskSource::New {
            name: "Design".into(),
        },
        billable: true,
        rate: String::new(),
        budget: String::new(),
        access: TaskAccess::Everyone,
    };
    let blank = ProjectForm {
        team: vec![blank_member.clone()],
        tasks: vec![blank_task.clone()],
        ..Default::default()
    };
    // Blank rates inherit, so creating without the grant stays possible.
    assert!(validate_draft_form(&blank, without).is_ok());
    let project_rate = ProjectForm {
        project_rate: "0".into(),
        ..blank.clone()
    };
    let person_rate = ProjectForm {
        team: vec![ProjectMemberInput {
            billable_rate: "80".into(),
            ..blank_member
        }],
        ..blank.clone()
    };
    let task_rate = ProjectForm {
        tasks: vec![ProjectTaskInput {
            rate: "95".into(),
            ..blank_task.clone()
        }],
        ..blank.clone()
    };
    let fixed_fee = ProjectForm {
        project_type: ProjectType::FixedFee,
        fee_amount: "1000".into(),
        ..blank.clone()
    };
    let money_budget = ProjectForm {
        budget_mode: BudgetMode::TotalFees,
        budget_value: "5000".into(),
        ..blank.clone()
    };
    let task_fee_budget = ProjectForm {
        budget_mode: BudgetMode::FeesPerTask,
        tasks: vec![ProjectTaskInput {
            budget: "300".into(),
            ..blank_task
        }],
        ..blank.clone()
    };
    let invoice_defaults = ProjectForm {
        invoice_defaults: InvoiceDefaultsInput {
            po_number: "PO-1".into(),
            ..Default::default()
        },
        ..blank.clone()
    };
    // An hour budget and a fee left behind by a type switch are not billable.
    let hours_budget = ProjectForm {
        budget_mode: BudgetMode::TotalHours,
        budget_value: "40".into(),
        fee_amount: "1000".into(),
        ..blank
    };
    assert!(validate_draft_form(&hours_budget, without).is_ok());
    for form in [
        project_rate,
        person_rate,
        task_rate,
        fixed_fee,
        money_budget,
        task_fee_budget,
        invoice_defaults,
    ] {
        assert!(matches!(
            validate_draft_form(&form, without),
            Err(super::ServerFnError::ServerError {
                code: super::FORBIDDEN,
                ..
            })
        ));
        assert!(validate_draft_form(&form, with).is_ok());
        let mut pruned = form.clone();
        without.prune(&mut pruned);
        assert!(validate_draft_form(&pruned, without).is_ok());
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn creation_refuses_billable_rates_without_the_grant_and_prunes_old_drafts(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Administrator).await;
    let id = Uuid::now_v7();
    let form = ProjectForm {
        client_id: Some(ids.client_id),
        name: "Rates".into(),
        team: vec![ProjectMemberInput {
            user_id: ids.user_id,
            manager: false,
            billable_rate: "80".into(),
            cost_rate: String::new(),
            budget: String::new(),
        }],
        ..Default::default()
    };
    super::save_draft_record(&pool, ids.user_id, ids.org_id, id, 0, &form)
        .await
        .unwrap();
    // Executive Manager creates projects but has no `BillableRateWriteAll`.
    become_profile(&pool, &ids, BuiltInProfile::ExecutiveManager).await;
    let draft = super::load_draft_record(&pool, ids.user_id, ids.org_id)
        .await
        .unwrap()
        .unwrap();
    assert!(draft.form.team[0].billable_rate.is_empty());
    for result in [
        super::save_draft_record(&pool, ids.user_id, ids.org_id, id, 1, &form)
            .await
            .map(|_| ()),
        super::finalize_draft_record(&pool, ids.user_id, ids.org_id, id, 1, &form, false)
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
    let project =
        super::finalize_draft_record(&pool, ids.user_id, ids.org_id, id, 1, &draft.form, false)
            .await
            .unwrap();
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
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn creation_actor_is_revalidated_inside_the_transaction(pool: PgPool) {
    let admin = seed(&pool, BuiltInProfile::Administrator).await;
    let member = seed(&pool, BuiltInProfile::Member).await;
    let mut tx = pool.begin().await.unwrap();
    assert!(
        lock_creation_actor(
            &mut tx,
            admin.user_id,
            admin.org_id,
            OrganizationLock::Shared
        )
        .await
        .unwrap()
        .administrator()
    );
    assert!(
        lock_creation_actor(
            &mut tx,
            member.user_id,
            member.org_id,
            OrganizationLock::Shared
        )
        .await
        .is_err()
    );
    assert!(
        lock_creation_actor(
            &mut tx,
            admin.user_id,
            member.org_id,
            OrganizationLock::Shared
        )
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
        lock_creation_actor(
            &mut tx,
            admin.user_id,
            admin.org_id,
            OrganizationLock::Shared
        )
        .await
        .is_err()
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn creation_client_must_be_active_and_in_the_same_organization(pool: PgPool) {
    let owner = seed(&pool, BuiltInProfile::Administrator).await;
    let other = seed(&pool, BuiltInProfile::Administrator).await;
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

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn draft_schema_enforces_one_current_draft_per_creator(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Administrator).await;
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

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn draft_schema_rejects_foreign_creator_and_completed_project(pool: PgPool) {
    let owner = seed(&pool, BuiltInProfile::Administrator).await;
    let other = seed(&pool, BuiltInProfile::Administrator).await;
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

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn draft_schema_rejects_oversize_payload_and_invalid_revision(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Administrator).await;
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

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn existing_projects_have_no_implicit_new_billing_configuration(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Administrator).await;
    let count = sqlx::query_scalar!(
        "SELECT count(*) FROM project_settings WHERE project_id = $1",
        ids.project_id,
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(count, Some(0));
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn billing_schema_rejects_partial_fee_schedules(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Administrator).await;
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

/// A linked task inherits its catalog default only where billing uses task
/// rates, and a non-billable project never makes the link billable.
#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn finalized_task_links_inherit_default_rates_only_for_task_billing(pool: PgPool) {
    use crate::models::project_creation::{ProjectTaskInput, TaskAccess, TaskSource};
    use horae_core::project::RateMode;
    use horae_core::types::ProjectType;
    let ids = seed(&pool, BuiltInProfile::Administrator).await;
    sqlx::query!(
        "UPDATE tasks SET default_rate_cents=8000, default_rate_currency='EUR' WHERE id=$1",
        ids.task_id
    )
    .execute(&pool)
    .await
    .unwrap();
    for project_type in [
        ProjectType::TimeAndMaterials,
        ProjectType::FixedFee,
        ProjectType::NonBillable,
    ] {
        for rate_mode in [RateMode::Person, RateMode::Project, RateMode::Task] {
            let form = ProjectForm {
                client_id: Some(ids.client_id),
                name: format!("{project_type:?} {rate_mode:?}"),
                project_type,
                rate_mode,
                project_rate: "10".into(),
                fee_amount: "100".into(),
                tasks: vec![ProjectTaskInput {
                    id: Uuid::now_v7(),
                    source: TaskSource::Existing {
                        task_id: ids.task_id,
                    },
                    billable: true,
                    rate: String::new(),
                    budget: String::new(),
                    access: TaskAccess::Everyone,
                }],
                ..Default::default()
            };
            let draft = Uuid::now_v7();
            super::save_draft_record(&pool, ids.user_id, ids.org_id, draft, 0, &form)
                .await
                .unwrap();
            let project = super::finalize_draft_record(
                &pool,
                ids.user_id,
                ids.org_id,
                draft,
                1,
                &form,
                false,
            )
            .await
            .unwrap();
            let link = sqlx::query!(
                "SELECT billable, rate_cents FROM project_tasks WHERE project_id=$1 AND task_id=$2",
                project,
                ids.task_id
            )
            .fetch_one(&pool)
            .await
            .unwrap();
            let task_billing =
                project_type == ProjectType::TimeAndMaterials && rate_mode == RateMode::Task;
            assert_eq!(
                (link.billable, link.rate_cents),
                (
                    project_type != ProjectType::NonBillable,
                    task_billing.then_some(8000)
                ),
                "{project_type:?} {rate_mode:?}"
            );
        }
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn creation_catalogs_show_billable_rates_only_to_rate_readers(pool: PgPool) {
    use crate::models::project_creation::CreationSearch;
    let ids = seed(&pool, BuiltInProfile::Administrator).await;
    sqlx::query!(
        "UPDATE users SET billable_rate_cents = 9000 WHERE id = $1",
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE tasks SET default_rate_cents = 7000, default_rate_currency = 'EUR', is_default = true
         WHERE id = $1",
        ids.task_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE clients SET default_rate_cents = 8000 WHERE id = $1",
        ids.client_id
    )
    .execute(&pool)
    .await
    .unwrap();
    // A project being created has no managers, so only `BillableRateReadAll`
    // shows the rates the form would inherit.
    for (profile, reads) in [
        (BuiltInProfile::Administrator, true),
        (BuiltInProfile::ExecutiveManager, true),
        (BuiltInProfile::ProjectManager, false),
    ] {
        become_profile(&pool, &ids, profile).await;
        let options = super::load_creation_options(
            &pool,
            ids.user_id,
            ids.org_id,
            &CreationSearch::default(),
            false,
        )
        .await
        .unwrap();
        let selected = super::load_selected_catalog(
            &pool,
            ids.user_id,
            ids.org_id,
            &[ids.task_id],
            &[ids.user_id],
        )
        .await
        .unwrap();
        let client = super::load_selected_client(&pool, ids.user_id, ids.org_id, ids.client_id)
            .await
            .unwrap()
            .unwrap();
        let task_rate = reads.then_some(7000);
        let task_currency = reads.then(|| "EUR".to_owned());
        for task in [
            &options.tasks[0],
            &options.common_tasks[0],
            &selected.tasks[0],
        ] {
            assert_eq!(task.default_rate_cents, task_rate, "{profile:?}");
            assert_eq!(task.default_rate_currency, task_currency, "{profile:?}");
        }
        for person in [&options.people[0], &selected.people[0]] {
            assert_eq!(
                person.billable_rate_cents,
                reads.then_some(9000),
                "{profile:?}"
            );
        }
        for client in [&options.clients[0], &client] {
            assert_eq!(
                client.default_rate_cents,
                reads.then_some(8000),
                "{profile:?}"
            );
        }
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn hourly_projects_in_another_currency_need_the_billable_rate_grant(pool: PgPool) {
    use horae_core::project::RateMode;
    use horae_core::types::ProjectType;
    let ids = seed(&pool, BuiltInProfile::Administrator).await;
    let org_currency = sqlx::query_scalar!(
        "SELECT default_currency FROM organizations WHERE id = $1",
        ids.org_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let other = if org_currency == "USD" { "EUR" } else { "USD" };
    let form = |name: &str, currency: &str, project_type, rate_mode, rate: &str| ProjectForm {
        client_id: Some(ids.client_id),
        name: name.into(),
        currency: Some(currency.into()),
        project_type,
        rate_mode,
        team: vec![ProjectMemberInput {
            user_id: ids.user_id,
            manager: false,
            billable_rate: rate.into(),
            cost_rate: String::new(),
            budget: String::new(),
        }],
        ..Default::default()
    };
    let (user_id, org_id) = (ids.user_id, ids.org_id);
    let create = |form: ProjectForm| {
        let pool = pool.clone();
        async move {
            // A refused creation leaves its draft open, and a creator has one.
            sqlx::query!(
                "DELETE FROM project_drafts WHERE creator_id = $1 AND completed_project_id IS NULL",
                user_id
            )
            .execute(&pool)
            .await
            .unwrap();
            let id = Uuid::now_v7();
            super::save_draft_record(&pool, user_id, org_id, id, 0, &form).await?;
            super::finalize_draft_record(&pool, user_id, org_id, id, 1, &form, false).await
        }
    };
    let hourly = ProjectType::TimeAndMaterials;
    for profile in [
        BuiltInProfile::ExecutiveManager,
        BuiltInProfile::ProjectManager,
    ] {
        become_profile(&pool, &ids, profile).await;
        // Refused on the currency alone: the answer is the same whether or
        // not the teammate or a task has a rate the project would inherit.
        let mut messages = Vec::new();
        for teammate_rate in [None, Some(9000_i64)] {
            sqlx::query!(
                "UPDATE users SET billable_rate_cents = $2 WHERE id = $1",
                ids.user_id,
                teammate_rate
            )
            .execute(&pool)
            .await
            .unwrap();
            for rate_mode in [RateMode::Person, RateMode::Task] {
                match create(form("Abroad", other, hourly, rate_mode, "")).await {
                    Err(super::ServerFnError::ServerError {
                        code: super::FORBIDDEN,
                        message,
                        ..
                    }) => messages.push(message),
                    result => panic!("{profile:?} {rate_mode:?}: {result:?}"),
                }
            }
        }
        messages.dedup();
        assert_eq!(messages.len(), 1, "{profile:?}: {messages:?}");
        // The organization's currency, or a project that bills no rates,
        // needs no grant.
        create(form("Home", &org_currency, hourly, RateMode::Person, ""))
            .await
            .unwrap();
        create(form(
            "Unbilled abroad",
            other,
            ProjectType::NonBillable,
            RateMode::Person,
            "",
        ))
        .await
        .unwrap();
    }
    // With the grant, explicit rates make the other currency work.
    become_profile(&pool, &ids, BuiltInProfile::Administrator).await;
    create(form("Abroad", other, hourly, RateMode::Person, "80"))
        .await
        .unwrap();
}
