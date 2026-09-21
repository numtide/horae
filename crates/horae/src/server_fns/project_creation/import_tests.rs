use super::{finalize_draft_record, save_draft_record};
use crate::importers::harvest::{VecSource, csv_source::import_body, run_import};
use crate::models::project_creation::{
    FeeMode, InvoiceDefaultsInput, MilestoneInput, ProjectForm, ProjectMemberInput,
    ProjectTaskInput, SecondTaxInput, TaskAccess, TaskSource,
};
use crate::server_fns::test_seed::seed;
use horae_core::importers::harvest::types::{ImportMode, SourceKind, SourceRow};
use horae_core::project::{BudgetMode, RateMode};
use horae_core::types::{OrgRole, ProjectType};
use sqlx::PgPool;
use uuid::Uuid;

async fn configuration(pool: &PgPool, org: Uuid) -> serde_json::Value {
    sqlx::query_scalar!(
        "SELECT jsonb_build_object(
          'projects', (SELECT jsonb_agg(to_jsonb(p) ORDER BY p.id) FROM projects p WHERE p.org_id = $1),
          'clients', (SELECT jsonb_agg(to_jsonb(c) ORDER BY c.id) FROM clients c WHERE c.org_id = $1),
          'tasks', (SELECT jsonb_agg(to_jsonb(t) ORDER BY t.id) FROM tasks t WHERE t.org_id = $1),
          'users', (SELECT jsonb_agg(to_jsonb(u) ORDER BY u.id) FROM users u WHERE u.org_id = $1),
          'drafts', (SELECT jsonb_agg(to_jsonb(d) ORDER BY d.id) FROM project_drafts d WHERE d.org_id = $1),
          'settings', (SELECT jsonb_agg(to_jsonb(s) ORDER BY s.id) FROM project_settings s WHERE s.org_id = $1),
          'private', (SELECT jsonb_agg(to_jsonb(s) ORDER BY s.id) FROM project_private_settings s WHERE s.org_id = $1),
          'assignments', (SELECT jsonb_agg(to_jsonb(a) ORDER BY a.id) FROM assignments a JOIN projects p ON p.id = a.project_id WHERE p.org_id = $1),
          'costs', (SELECT jsonb_agg(to_jsonb(c) ORDER BY c.id) FROM project_member_costs c WHERE c.org_id = $1),
          'member_budgets', (SELECT jsonb_agg(to_jsonb(b) ORDER BY b.id) FROM project_member_budgets b WHERE b.org_id = $1),
          'project_tasks', (SELECT jsonb_agg(to_jsonb(t) ORDER BY t.project_id, t.task_id) FROM project_tasks t JOIN projects p ON p.id = t.project_id WHERE p.org_id = $1),
          'task_settings', (SELECT jsonb_agg(to_jsonb(s) ORDER BY s.id) FROM project_task_settings s WHERE s.org_id = $1),
          'task_members', (SELECT jsonb_agg(to_jsonb(m) ORDER BY m.id) FROM project_task_members m WHERE m.org_id = $1),
          'tags', (SELECT jsonb_agg(to_jsonb(t) ORDER BY t.id) FROM project_tags t WHERE t.org_id = $1),
          'tag_links', (SELECT jsonb_agg(to_jsonb(l) ORDER BY l.id) FROM project_tag_links l WHERE l.org_id = $1),
          'milestones', (SELECT jsonb_agg(to_jsonb(m) ORDER BY m.id) FROM project_fee_milestones m WHERE m.org_id = $1)
        ) AS \"snapshot!\"",
        org,
    ).fetch_one(pool).await.unwrap()
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn imports_preserve_finalized_project_configuration_and_restricted_access(pool: PgPool) {
    for source in [SourceKind::Csv, SourceKind::HarvestApi] {
        for (project_type, rate_mode, expected_billing) in [
            (ProjectType::TimeAndMaterials, RateMode::Task, 10000),
            (ProjectType::TimeAndMaterials, RateMode::Person, 6875),
            (ProjectType::TimeAndMaterials, RateMode::Project, 8250),
            (ProjectType::FixedFee, RateMode::Task, 0),
        ] {
            let ids = seed(&pool, OrgRole::Admin).await;
            let member = Uuid::now_v7();
            let email = format!("{member}@test.com");
            sqlx::query!(
                "INSERT INTO users (id,org_id,email,name,org_role,billable_rate_cents,cost_rate_cents)
                 VALUES ($1,$2,$3,'Local member','member',5000,2500)",
                member, ids.org_id, email,
            ).execute(&pool).await.unwrap();
            let form = ProjectForm {
                client_id: Some(ids.client_id),
                name: "Configured import".into(),
                code: "CFG-1".into(),
                starts_on: "2026-09-01".into(),
                ends_on: "2026-10-31".into(),
                currency: Some("USD".into()),
                admin_notes: "Private local notes".into(),
                tags: vec!["Local tag".into()],
                project_type,
                rate_mode,
                project_rate: "66".into(),
                budget_mode: if rate_mode == RateMode::Person {
                    BudgetMode::HoursPerPerson
                } else {
                    BudgetMode::HoursPerTask
                },
                budget_monthly: true,
                budget_nonbillable: true,
                budget_alert: true,
                budget_alert_at: "90".into(),
                fee_mode: FeeMode::Milestones,
                milestones: vec![MilestoneInput {
                    id: Uuid::now_v7(),
                    name: "Local milestone".into(),
                    due_on: "2026-10-01".into(),
                    amount: "1000".into(),
                }],
                tasks: vec![ProjectTaskInput {
                    id: Uuid::now_v7(),
                    source: TaskSource::Existing {
                        task_id: ids.task_id,
                    },
                    billable: true,
                    rate: "80".into(),
                    budget: "8:00".into(),
                    access: TaskAccess::Restricted {
                        user_ids: vec![ids.user_id],
                    },
                }],
                team: [ids.user_id, member]
                    .into_iter()
                    .map(|user_id| ProjectMemberInput {
                        user_id,
                        manager: user_id == ids.user_id,
                        billable_rate: "55".into(),
                        cost_rate: "22.50".into(),
                        budget: "10:00".into(),
                    })
                    .collect(),
                invoice_defaults: InvoiceDefaultsInput {
                    terms_days: "14".into(),
                    po_number: "LOCAL-PO".into(),
                    tax: "5".into(),
                    second_tax: Some(SecondTaxInput {
                        name: "Local tax".into(),
                        percentage: "2".into(),
                    }),
                    discount: "7.50".into(),
                },
                ..Default::default()
            };
            let draft = Uuid::now_v7();
            save_draft_record(&pool, ids.user_id, ids.org_id, draft, 0, &form)
                .await
                .unwrap();
            let project =
                finalize_draft_record(&pool, ids.user_id, ids.org_id, draft, 1, &form, true)
                    .await
                    .unwrap();
            let before = configuration(&pool, ids.org_id).await;
            for (table, count) in [
                ("settings", 1),
                ("private", 1),
                ("assignments", 2),
                ("costs", 2),
                ("project_tasks", 1),
                ("task_settings", 1),
                ("task_members", 1),
                ("tags", 1),
                ("tag_links", 1),
            ] {
                assert_eq!(before[table].as_array().unwrap().len(), count, "{table}");
            }
            assert_eq!(
                before["member_budgets"].as_array().map(Vec::len),
                (rate_mode == RateMode::Person).then_some(2),
            );
            assert_eq!(
                before["milestones"].as_array().map(Vec::len),
                (project_type == ProjectType::FixedFee).then_some(1),
            );
            for (mode, created, persisted) in [
                (ImportMode::DryRun, 1, 0),
                (ImportMode::Commit, 1, 1),
                (ImportMode::Commit, 0, 1),
            ] {
                let report = match source {
                    SourceKind::Csv => {
                        let csv = format!(
                            "Date,Client,Project,Project Code,Task,Email,Hours,Billable?,Invoiced?,Billable Rate,Cost Rate,Currency\n\
                             2026-09-07, acme ,Remote rename,CFG-1, dev ,{email},1.25,Yes,Yes,99,999,GBP\n"
                        );
                        import_body(&pool, ids.org_id, "EUR", axum::body::Body::from(csv), mode)
                            .await
                            .unwrap()
                    }
                    SourceKind::HarvestApi => {
                        let row = SourceRow {
                            harvest_client_id: Some(10),
                            harvest_project_id: Some(20),
                            harvest_task_id: Some(30),
                            harvest_time_entry_id: Some(40),
                            harvest_user_id: Some(50),
                            client_name: " acme ".into(),
                            client_address: Some("Remote address".into()),
                            client_active: false,
                            project_name: "Remote rename".into(),
                            project_code: Some("CFG-1".into()),
                            project_active: false,
                            project_starts_on: Some("2010-01-01".parse().unwrap()),
                            project_ends_on: None,
                            task_name: " dev ".into(),
                            task_billable_default: false,
                            user_email: Some(email.clone()),
                            user_name: Some("Remote person name".into()),
                            spent_date: "2026-09-07".parse().unwrap(),
                            hours: "1.25".into(),
                            notes: None,
                            billable: true,
                            invoiced: true,
                            billable_rate: Some("99".into()),
                            billable_amount: None,
                            cost_rate: Some("999".into()),
                            cost_amount: None,
                            currency: Some("GBP".into()),
                            harvest_updated_at: Some("2026-09-08T00:00:00Z".parse().unwrap()),
                            source_location: "time_entry 40".into(),
                        };
                        run_import(
                            &pool,
                            ids.org_id,
                            "EUR",
                            source,
                            mode,
                            VecSource::new(vec![row]),
                        )
                        .await
                        .unwrap()
                    }
                };
                assert_eq!(
                    report.error_count(),
                    0,
                    "{source:?}/{rate_mode:?}/{mode:?}: {report:?}"
                );
                assert_eq!(report.summary.time_entries.created, created);
                assert_eq!(
                    configuration(&pool, ids.org_id).await,
                    before,
                    "{source:?}/{project_type:?}/{rate_mode:?}/{mode:?}"
                );
                let entry = sqlx::query!(
                    "SELECT minutes, state::text AS state, invoice_id FROM time_entries WHERE project_id = $1",
                    project,
                ).fetch_all(&pool).await.unwrap();
                assert_eq!(entry.len(), persisted);
                if let Some(entry) = entry.first() {
                    assert_eq!(
                        (entry.minutes, entry.state.as_deref(), entry.invoice_id),
                        (75, Some("open"), None)
                    );
                }
                let can_track = sqlx::query_scalar!(
                    "SELECT EXISTS(SELECT 1 FROM time_entry_contexts WHERE user_id = $1 AND project_id = $2 AND task_id = $3)",
                    member, project, ids.task_id,
                ).fetch_one(&pool).await.unwrap();
                assert_eq!(
                    can_track,
                    Some(false),
                    "privileged import must not grant ordinary task access"
                );
                let access = sqlx::query!(
                    "SELECT can_view_progress, can_view_rates FROM project_read_access WHERE user_id = $1 AND project_id = $2",
                    member, project,
                ).fetch_one(&pool).await.unwrap();
                assert_eq!(
                    (access.can_view_progress, access.can_view_rates),
                    (Some(false), Some(false))
                );
            }
            let day = "2026-09-07".parse().unwrap();
            let report = crate::server_fns::reports::fetch_report(
                &pool,
                ids.user_id,
                (day, day),
                "project",
                crate::reports::ReportFilters::default(),
            )
            .await
            .unwrap();
            assert_eq!(report.len(), 1);
            assert_eq!(
                (report[0].billable_cents, report[0].cost_cents),
                (expected_billing, Some(2813))
            );
            assert_eq!(
                (
                    report[0].currency.as_str(),
                    report[0].cost_currency.as_str()
                ),
                ("USD", "EUR")
            );
        }
    }
}
