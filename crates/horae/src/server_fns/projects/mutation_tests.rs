use super::*;
use crate::plugin::event::ActiveTransition;
use crate::server_fns::test_seed::{SeedIds, seed, wait_for_blocked};
use sqlx::PgPool;
use std::time::Duration;

use crate::models::project_creation::ProjectEditRequest;
use crate::server_fns::project_creation::editing::{load_editable_project, save_editable_project};
use horae_core::project::BudgetMode;

mod project {
    use super::*;

    async fn configured_project(pool: &PgPool, mode: &str, scope: &str) -> SeedIds {
        let ids = seed(pool, OrgRole::Admin).await;
        sqlx::query!(
            "UPDATE projects SET budget_kind = 'hours', budget_minutes = 120,
             rate_cents = CASE WHEN $2 = 'project' THEN 10000 ELSE NULL END WHERE id = $1",
            ids.project_id,
            mode,
        )
        .execute(pool)
        .await
        .unwrap();
        sqlx::query!(
            "INSERT INTO project_settings (id, org_id, project_id, creator_id, rate_mode, budget_scope)
             VALUES ($1, $2, $3, $4, $5, $6)",
            uuid::Uuid::now_v7(), ids.org_id, ids.project_id, ids.user_id, mode, scope,
        ).execute(pool).await.unwrap();
        sqlx::query!(
            "INSERT INTO project_tasks (project_id, task_id, billable, rate_cents) VALUES ($1, $2, true, 8000)",
            ids.project_id, ids.task_id,
        ).execute(pool).await.unwrap();
        sqlx::query!(
            "INSERT INTO assignments (id, project_id, user_id, role, rate_cents) VALUES ($1, $2, $3, 'lead', 9000)",
            uuid::Uuid::now_v7(), ids.project_id, ids.user_id,
        ).execute(pool).await.unwrap();
        if scope == "task" {
            sqlx::query!(
                "INSERT INTO project_task_settings (id, org_id, project_id, task_id, budget_minutes) VALUES ($1, $2, $3, $4, 120)",
                uuid::Uuid::now_v7(), ids.org_id, ids.project_id, ids.task_id,
            ).execute(pool).await.unwrap();
        } else if scope == "person" {
            sqlx::query!(
                "INSERT INTO project_member_budgets (id, org_id, project_id, user_id, budget_minutes) VALUES ($1, $2, $3, $4, 120)",
                uuid::Uuid::now_v7(), ids.org_id, ids.project_id, ids.user_id,
            ).execute(pool).await.unwrap();
        }
        ids
    }

    async fn request(pool: &PgPool, ids: &SeedIds) -> ProjectEditRequest {
        let project = load_editable_project(pool, ids.user_id, ids.org_id, ids.project_id)
            .await
            .unwrap();
        ProjectEditRequest {
            id: uuid::Uuid::now_v7(),
            project_id: project.id,
            expected_revision: project.revision,
            form: project.form,
        }
    }

    async fn save(
        pool: &PgPool,
        ids: &SeedIds,
        request: &ProjectEditRequest,
    ) -> Result<(Project, bool), ServerFnError> {
        save_editable_project(pool, ids.user_id, ids.org_id, request, false).await
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn configured_edits_reject_incompatible_financial_changes_atomically(pool: PgPool) {
        for (mode, scope, field, value, message) in [
            ("task", "project", "currency", "USD", "currency"),
            ("project", "project", "rate", "", "rate is required"),
            ("task", "task", "task_budget", "-1", "valid hours"),
            ("person", "person", "person_budget", "NaN", "valid hours"),
        ] {
            let ids = configured_project(&pool, mode, scope).await;
            let mut edit = request(&pool, &ids).await;
            edit.form.name = "Must not be saved".into();
            match field {
                "currency" => edit.form.currency = Some(value.into()),
                "rate" => edit.form.project_rate = value.into(),
                "task_budget" => edit.form.tasks[0].budget = value.into(),
                "person_budget" => edit.form.team[0].budget = value.into(),
                _ => unreachable!(),
            }
            let before = version(&pool, ids.project_id).await;
            let error = save(&pool, &ids, &edit).await.expect_err(message);
            assert!(
                error.to_string().to_lowercase().contains(message),
                "{error}"
            );
            assert_eq!(version(&pool, ids.project_id).await, before, "{message}");
        }
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn configured_edits_preserve_compatible_changes_and_idempotence(pool: PgPool) {
        for (mode, scope, budget, value, rate, expected_spend) in [
            ("task", "task", BudgetMode::HoursPerTask, "2", "", 8000),
            (
                "person",
                "person",
                BudgetMode::HoursPerPerson,
                "2",
                "",
                9000,
            ),
            ("task", "project", BudgetMode::TotalHours, "3", "", 8000),
            ("person", "project", BudgetMode::TotalHours, "0", "", 9000),
            ("project", "project", BudgetMode::TotalHours, "2", "0", 0),
            (
                "project",
                "project",
                BudgetMode::TotalHours,
                "2",
                "80.25",
                8025,
            ),
            ("task", "project", BudgetMode::TotalFees, "1.25", "", 8000),
            ("person", "project", BudgetMode::None, "", "", 9000),
        ] {
            let ids = configured_project(&pool, mode, scope).await;
            crate::server_fns::test_seed::time_entry(
                &pool,
                &ids,
                horae_core::types::EntryState::Open,
            )
            .await;
            let mut edit = request(&pool, &ids).await;
            edit.form.name = "Renamed".into();
            edit.form.currency = Some(" eur ".into());
            edit.form.budget_mode = budget;
            edit.form.budget_value = value.into();
            edit.form.project_rate = rate.into();
            let (updated, changed) = save(&pool, &ids, &edit).await.unwrap();
            assert!(changed);
            assert_eq!(updated.name, "Renamed");
            assert_eq!(updated.currency, "EUR");
            let expected_budget = match value {
                "" => (None, None),
                "0" => (None, Some(0)),
                "2" => (None, Some(120)),
                "3" => (None, Some(180)),
                "1.25" => (Some(125), None),
                _ => unreachable!("fixture budget must have an explicit expectation"),
            };
            assert_eq!(
                (updated.budget_amount_cents, updated.budget_minutes),
                expected_budget
            );
            assert_eq!(updated.rate_cents, parse_project_rate(rate).unwrap());
            let before = version(&pool, ids.project_id).await;
            let (repeated, changed) = save(&pool, &ids, &edit).await.unwrap();
            assert_eq!(
                (repeated, changed, version(&pool, ids.project_id).await),
                (updated, false, before)
            );
            let settings = sqlx::query!(
                "SELECT rate_mode, budget_scope FROM project_settings WHERE project_id = $1",
                ids.project_id
            )
            .fetch_one(&pool)
            .await
            .unwrap();
            assert_eq!(
                (settings.rate_mode.as_str(), settings.budget_scope.as_str()),
                (mode, scope)
            );
            let spend = fetch_project_spend(&pool, ids.org_id, ids.user_id)
                .await
                .unwrap();
            assert_eq!(spend.len(), 1);
            assert_eq!(
                (spend[0].spent_minutes, spend[0].spent_cents),
                (60, expected_spend)
            );
        }
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn configured_edits_recheck_settings_after_waiting_for_the_project(pool: PgPool) {
        let ids = seed(&pool, OrgRole::Admin).await;
        let mut edit = request(&pool, &ids).await;
        edit.form.name = "Must not be saved".into();
        let mut first = pool.begin().await.unwrap();
        let blocker = sqlx::query_scalar!("SELECT pg_backend_pid()")
            .fetch_one(&mut *first)
            .await
            .unwrap()
            .unwrap();
        lock_project(&mut first, ids.org_id, ids.project_id)
            .await
            .unwrap();
        sqlx::query!(
            "INSERT INTO project_settings (id, org_id, project_id, creator_id, rate_mode, budget_scope)
             VALUES ($1, $2, $3, $4, $5, $6)",
            uuid::Uuid::now_v7(), ids.org_id, ids.project_id, ids.user_id, "person", "project",
        ).execute(&mut *first).await.unwrap();
        let before = sqlx::query_scalar!(
            "SELECT xmin::text FROM projects WHERE id = $1",
            ids.project_id
        )
        .fetch_one(&mut *first)
        .await
        .unwrap();
        let run_pool = pool.clone();
        let mut run = tokio::task::JoinSet::new();
        run.spawn(async move {
            save_editable_project(&run_pool, ids.user_id, ids.org_id, &edit, false).await
        });
        wait_for_blocked(&pool, blocker).await;
        first.commit().await.unwrap();
        let error = tokio::time::timeout(Duration::from_secs(5), run.join_next())
            .await
            .unwrap()
            .unwrap()
            .unwrap()
            .unwrap_err();
        assert!(
            matches!(error, ServerFnError::ServerError { code: CONFLICT, .. }),
            "{error}"
        );
        assert_eq!(version(&pool, ids.project_id).await, before);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn configured_edits_preserve_non_hourly_billing(pool: PgPool) {
        for project_type in [ProjectType::FixedFee, ProjectType::NonBillable] {
            let ids = configured_project(&pool, "person", "project").await;
            sqlx::query!(
                "UPDATE projects SET project_type = $2 WHERE id = $1",
                ids.project_id,
                project_type as ProjectType
            )
            .execute(&pool)
            .await
            .unwrap();
            if project_type == ProjectType::FixedFee {
                sqlx::query!("UPDATE project_settings SET fee_mode = 'single', fee_amount_cents = 10000 WHERE project_id = $1", ids.project_id).execute(&pool).await.unwrap();
            }
            let mut edit = request(&pool, &ids).await;
            edit.form.name = "Renamed".into();
            let (updated, changed) = save(&pool, &ids, &edit).await.unwrap();
            assert!(changed);
            assert_eq!(
                (
                    updated.name.as_str(),
                    updated.project_type,
                    updated.rate_cents
                ),
                ("Renamed", project_type, None)
            );
            crate::server_fns::test_seed::time_entry(
                &pool,
                &ids,
                horae_core::types::EntryState::Open,
            )
            .await;
            for field in ["type", "currency", "budget"] {
                let mut rejected = request(&pool, &ids).await;
                match field {
                    "type" => rejected.form.project_type = ProjectType::TimeAndMaterials,
                    "currency" => rejected.form.currency = Some("USD".into()),
                    "budget" => rejected.form.budget_mode = BudgetMode::TotalFees,
                    _ => unreachable!(),
                }
                let before = version(&pool, ids.project_id).await;
                assert!(save(&pool, &ids, &rejected).await.is_err());
                assert_eq!(version(&pool, ids.project_id).await, before);
            }
            let mut irrelevant_rate = request(&pool, &ids).await;
            irrelevant_rate.form.project_rate = "1".into();
            let (unchanged, changed) = save(&pool, &ids, &irrelevant_rate).await.unwrap();
            assert!(
                !changed,
                "Hidden hourly input must not change non-hourly billing"
            );
            assert_eq!(unchanged.rate_cents, None);
        }
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn inactive_project_edits_preserve_identity_and_reject_implicit_migrations(pool: PgPool) {
        let ids = seed(&pool, OrgRole::Admin).await;
        let (mut expected, _) = set_project_active_record(&pool, ids.org_id, ids.project_id, false)
            .await
            .unwrap();
        let mut edit = request(&pool, &ids).await;
        edit.form.name = "Renamed".into();
        expected.name = "Renamed".into();
        let (updated, changed) = save(&pool, &ids, &edit).await.unwrap();
        assert_eq!((changed, updated), (true, expected.clone()));
        let before = version(&pool, ids.project_id).await;
        let (repeated, changed) = save(&pool, &ids, &edit).await.unwrap();
        assert_eq!(
            (changed, repeated, version(&pool, ids.project_id).await),
            (false, expected, before.clone())
        );
        for field in ["type", "currency"] {
            let mut rejected = request(&pool, &ids).await;
            if field == "type" {
                rejected.form.project_type = ProjectType::FixedFee;
            } else {
                rejected.form.currency = Some("USD".into());
            }
            assert!(save(&pool, &ids, &rejected).await.is_err());
            assert_eq!(version(&pool, ids.project_id).await, before);
        }
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn budgets_and_rates_distinguish_null_zero_and_normalized_values(pool: PgPool) {
        let ids = seed(&pool, OrgRole::Admin).await;
        for (budget, value, rate, amount, minutes, cents) in [
            (BudgetMode::TotalFees, "1.25", "", Some(125), None, None),
            (BudgetMode::TotalFees, "0", "", Some(0), None, None),
            (BudgetMode::TotalFees, "", "", None, None, None),
            (BudgetMode::TotalHours, "1:30", "", None, Some(90), None),
            (BudgetMode::TotalHours, "2", "", None, Some(120), None),
            (BudgetMode::TotalHours, "", "", None, None, None),
            (BudgetMode::TotalHours, "", "0", None, None, Some(0)),
            (
                BudgetMode::TotalHours,
                "",
                "120.50",
                None,
                None,
                Some(12050),
            ),
            (BudgetMode::TotalHours, "", "", None, None, None),
            (BudgetMode::None, "", "", None, None, None),
        ] {
            let mut edit = request(&pool, &ids).await;
            edit.form.budget_mode = budget;
            edit.form.budget_value = value.into();
            edit.form.project_rate = rate.into();
            let (updated, changed) = save(&pool, &ids, &edit).await.unwrap();
            assert_eq!(
                (
                    changed,
                    updated.budget_amount_cents,
                    updated.budget_minutes,
                    updated.rate_cents
                ),
                (true, amount, minutes, cents)
            );
            let before = version(&pool, ids.project_id).await;
            let mut normalized = request(&pool, &ids).await;
            normalized.form.budget_value = format!(" {value} ");
            normalized.form.project_rate = format!(" {rate} ");
            let (repeated, changed) = save(&pool, &ids, &normalized).await.unwrap();
            assert_eq!(
                (changed, repeated, version(&pool, ids.project_id).await),
                (false, updated, before)
            );
        }
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn invalid_project_fields_leave_the_row_unchanged(pool: PgPool) {
        let ids = seed(&pool, OrgRole::Admin).await;
        let before = version(&pool, ids.project_id).await;
        for (field, value) in [
            ("amount", "-1"),
            ("hours", "NaN"),
            ("rate", "-1"),
            ("rate", "92233720368547758.08"),
        ] {
            let mut edit = request(&pool, &ids).await;
            match field {
                "amount" => {
                    edit.form.budget_mode = BudgetMode::TotalFees;
                    edit.form.budget_value = value.into();
                }
                "hours" => {
                    edit.form.budget_mode = BudgetMode::TotalHours;
                    edit.form.budget_value = value.into();
                }
                "rate" => edit.form.project_rate = value.into(),
                _ => unreachable!(),
            }
            assert!(save(&pool, &ids, &edit).await.is_err());
        }
        for field in ["project_type", "budget_mode"] {
            let mut payload = serde_json::to_value(request(&pool, &ids).await).unwrap();
            payload["form"][field] = serde_json::json!("unknown");
            assert!(serde_json::from_value::<ProjectEditRequest>(payload).is_err());
        }
        assert_eq!(version(&pool, ids.project_id).await, before);
    }

    async fn edit(pool: &PgPool, ids: &SeedIds, name: &str) -> (Project, bool) {
        let mut request = request(pool, ids).await;
        request.form.name = name.into();
        save(pool, ids, &request).await.unwrap()
    }

    async fn version(pool: &PgPool, id: uuid::Uuid) -> Option<String> {
        sqlx::query_scalar!("SELECT xmin::text FROM projects WHERE id = $1", id)
            .fetch_one(pool)
            .await
            .unwrap()
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn unchanged_edit_preserves_the_row(pool: PgPool) {
        let ids = seed(&pool, OrgRole::Admin).await;
        let before = version(&pool, ids.project_id).await;
        let (returned, changed) = edit(&pool, &ids, "Widget").await;
        let after = version(&pool, ids.project_id).await;
        assert_eq!(
            (changed, returned.name.as_str(), after),
            (false, "Widget", before)
        );
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn unchanged_activation_preserves_the_row(pool: PgPool) {
        let ids = seed(&pool, OrgRole::Admin).await;
        let before = version(&pool, ids.project_id).await;
        let (returned, transition) =
            set_project_active_record(&pool, ids.org_id, ids.project_id, true)
                .await
                .unwrap();
        let after = version(&pool, ids.project_id).await;
        assert_eq!((transition, returned.active, after), (None, true, before));
    }

    async fn competing_edit(pool: &PgPool, name: &'static str) -> ServerFnError {
        let ids = seed(pool, OrgRole::Admin).await;
        let mut pending = request(pool, &ids).await;
        pending.form.name = name.into();
        let mut first = pool.begin().await.unwrap();
        let blocker = sqlx::query_scalar!("SELECT pg_backend_pid()")
            .fetch_one(&mut *first)
            .await
            .unwrap()
            .unwrap();
        sqlx::query!(
            "UPDATE projects SET name = 'Changed' WHERE id = $1",
            ids.project_id
        )
        .execute(&mut *first)
        .await
        .unwrap();
        let run_pool = pool.clone();
        let mut run = tokio::task::JoinSet::new();
        run.spawn(async move {
            save_editable_project(&run_pool, ids.user_id, ids.org_id, &pending, false).await
        });
        wait_for_blocked(pool, blocker).await;
        first.commit().await.unwrap();
        let error = tokio::time::timeout(Duration::from_secs(5), run.join_next())
            .await
            .unwrap()
            .unwrap()
            .unwrap()
            .unwrap_err();
        assert_eq!(request(pool, &ids).await.form.name, "Changed");
        error
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn repeating_a_competing_edit_requires_a_fresh_revision(pool: PgPool) {
        let error = competing_edit(&pool, "Changed").await;
        assert!(matches!(
            error,
            ServerFnError::ServerError { code: CONFLICT, .. }
        ));
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn restoring_a_competing_edit_does_not_overwrite_the_other_session(pool: PgPool) {
        let error = competing_edit(&pool, "Widget").await;
        assert!(matches!(
            error,
            ServerFnError::ServerError { code: CONFLICT, .. }
        ));
    }

    async fn competing_activation(
        pool: &PgPool,
        active: bool,
    ) -> (Project, Option<ActiveTransition>) {
        let ids = seed(pool, OrgRole::Admin).await;
        let mut first = pool.begin().await.unwrap();
        let blocker = sqlx::query_scalar!("SELECT pg_backend_pid()")
            .fetch_one(&mut *first)
            .await
            .unwrap()
            .unwrap();
        sqlx::query!(
            "UPDATE projects SET active = false WHERE id = $1",
            ids.project_id
        )
        .execute(&mut *first)
        .await
        .unwrap();
        let run_pool = pool.clone();
        let mut run = tokio::task::JoinSet::new();
        run.spawn(async move {
            set_project_active_record(&run_pool, ids.org_id, ids.project_id, active).await
        });
        wait_for_blocked(pool, blocker).await;
        first.commit().await.unwrap();
        tokio::time::timeout(Duration::from_secs(5), run.join_next())
            .await
            .unwrap()
            .unwrap()
            .unwrap()
            .unwrap()
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn repeating_a_competing_deactivation_reports_no_transition(pool: PgPool) {
        let (returned, transition) = competing_activation(&pool, false).await;
        assert_eq!((transition, returned.active), (None, false));
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn restoring_a_competing_deactivation_reports_reactivation(pool: PgPool) {
        let (returned, transition) = competing_activation(&pool, true).await;
        assert_eq!(
            (transition, returned.active),
            (Some(ActiveTransition::Reactivated), true)
        );
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn missing_and_foreign_rows_are_not_noops(pool: PgPool) {
        let ids = seed(&pool, OrgRole::Admin).await;
        let other = seed(&pool, OrgRole::Admin).await;
        let before = version(&pool, ids.project_id).await;
        for (org_id, id) in [
            (ids.org_id, uuid::Uuid::now_v7()),
            (other.org_id, ids.project_id),
        ] {
            let mut pending = request(&pool, &ids).await;
            pending.project_id = id;
            let actor = if org_id == ids.org_id {
                ids.user_id
            } else {
                other.user_id
            };
            let error = save_editable_project(&pool, actor, org_id, &pending, false)
                .await
                .unwrap_err();
            assert!(matches!(
                error,
                ServerFnError::ServerError {
                    code: NOT_FOUND,
                    ..
                }
            ));
            let error = set_project_active_record(&pool, org_id, id, true)
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
        assert_eq!(version(&pool, ids.project_id).await, before);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn both_activation_transitions_preserve_details_and_change_once(pool: PgPool) {
        let ids = seed(&pool, OrgRole::Admin).await;
        let (mut expected, _) = edit(&pool, &ids, "Renamed").await;
        for (active, transition) in [
            (false, ActiveTransition::Deactivated),
            (true, ActiveTransition::Reactivated),
        ] {
            expected.active = active;
            let (updated, actual) =
                set_project_active_record(&pool, ids.org_id, ids.project_id, active)
                    .await
                    .unwrap();
            assert_eq!((actual, updated), (Some(transition), expected.clone()));
            let before = version(&pool, ids.project_id).await;
            let (updated, actual) =
                set_project_active_record(&pool, ids.org_id, ids.project_id, active)
                    .await
                    .unwrap();
            let after = version(&pool, ids.project_id).await;
            assert_eq!((actual, updated, after), (None, expected.clone(), before));
        }
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn concurrent_deletion_is_rejected_by_both_mutations(pool: PgPool) {
        for activation in [false, true] {
            let ids = seed(&pool, OrgRole::Admin).await;
            let pending = request(&pool, &ids).await;
            let mut first = pool.begin().await.unwrap();
            let blocker = sqlx::query_scalar!("SELECT pg_backend_pid()")
                .fetch_one(&mut *first)
                .await
                .unwrap()
                .unwrap();
            sqlx::query!("DELETE FROM projects WHERE id = $1", ids.project_id)
                .execute(&mut *first)
                .await
                .unwrap();
            let run_pool = pool.clone();
            let mut run = tokio::task::JoinSet::new();
            run.spawn(async move {
                if activation {
                    set_project_active_record(&run_pool, ids.org_id, ids.project_id, true)
                        .await
                        .map(|_| ())
                } else {
                    save_editable_project(&run_pool, ids.user_id, ids.org_id, &pending, false)
                        .await
                        .map(|_| ())
                }
            });
            wait_for_blocked(&pool, blocker).await;
            first.commit().await.unwrap();
            let error = tokio::time::timeout(Duration::from_secs(5), run.join_next())
                .await
                .unwrap()
                .unwrap()
                .unwrap()
                .unwrap_err();
            assert!(matches!(
                error,
                ServerFnError::ServerError {
                    code: NOT_FOUND | CONFLICT,
                    ..
                }
            ));
        }
    }
}

mod task {
    use super::*;

    #[sqlx::test(migrations = "./migrations")]
    async fn rate_edits_set_currency_without_relabelling_renames_or_noops(pool: PgPool) {
        let ids = seed(&pool, OrgRole::Admin).await;
        sqlx::query!(
            "UPDATE tasks SET default_rate_cents = 8000 WHERE id = $1",
            ids.task_id
        )
        .execute(&pool)
        .await
        .unwrap();
        for (name, rate, currency) in [
            ("Renamed", Some(8000), None),
            ("Renamed", Some(8000), None),
            ("Renamed", Some(0), Some("EUR")),
            ("Again", Some(0), Some("EUR")),
            ("Again", None, None),
        ] {
            update_task_record(&pool, ids.org_id, ids.task_id, name, true, rate)
                .await
                .unwrap();
            let stored = sqlx::query_scalar!(
                "SELECT default_rate_currency FROM tasks WHERE id = $1",
                ids.task_id
            )
            .fetch_one(&pool)
            .await
            .unwrap();
            assert_eq!(stored.as_deref(), currency);
        }
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn each_detail_field_changes_once_and_preserves_inactive_status(pool: PgPool) {
        let ids = seed(&pool, OrgRole::Admin).await;
        let (mut expected, _) = set_task_active_record(&pool, ids.org_id, ids.task_id, false)
            .await
            .unwrap();
        for (name, billable, rate) in [
            ("Renamed", true, None),
            ("Renamed", false, None),
            ("Renamed", false, Some(0)),
            ("Renamed", false, Some(12050)),
            ("Renamed", false, None),
        ] {
            expected.name = name.into();
            expected.billable_default = billable;
            expected.default_rate_cents = rate;
            let (updated, changed) =
                update_task_record(&pool, ids.org_id, ids.task_id, name, billable, rate)
                    .await
                    .unwrap();
            assert_eq!((changed, updated), (true, expected.clone()));
            let before = version(&pool, ids.task_id).await;
            let (repeated, changed) =
                update_task_record(&pool, ids.org_id, ids.task_id, name, billable, rate)
                    .await
                    .unwrap();
            let after = version(&pool, ids.task_id).await;
            assert_eq!(
                (changed, repeated, after),
                (false, expected.clone(), before)
            );
        }
    }

    async fn edit(pool: &PgPool, ids: &SeedIds, name: &str) -> (Task, bool) {
        update_task_record(pool, ids.org_id, ids.task_id, name, true, None)
            .await
            .unwrap()
    }

    async fn version(pool: &PgPool, id: uuid::Uuid) -> Option<String> {
        sqlx::query_scalar!("SELECT xmin::text FROM tasks WHERE id = $1", id)
            .fetch_one(pool)
            .await
            .unwrap()
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn unchanged_edit_preserves_the_row(pool: PgPool) {
        let ids = seed(&pool, OrgRole::Admin).await;
        let before = version(&pool, ids.task_id).await;
        let (returned, changed) = edit(&pool, &ids, "Dev").await;
        let after = version(&pool, ids.task_id).await;
        assert_eq!(
            (changed, returned.name.as_str(), after),
            (false, "Dev", before)
        );
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn unchanged_activation_preserves_the_row(pool: PgPool) {
        let ids = seed(&pool, OrgRole::Admin).await;
        let before = version(&pool, ids.task_id).await;
        let (returned, transition) = set_task_active_record(&pool, ids.org_id, ids.task_id, true)
            .await
            .unwrap();
        let after = version(&pool, ids.task_id).await;
        assert_eq!((transition, returned.active, after), (None, true, before));
    }

    async fn competing_edit(pool: &PgPool, name: &'static str) -> (Task, bool) {
        let ids = seed(pool, OrgRole::Admin).await;
        let mut first = pool.begin().await.unwrap();
        let blocker = sqlx::query_scalar!("SELECT pg_backend_pid()")
            .fetch_one(&mut *first)
            .await
            .unwrap()
            .unwrap();
        sqlx::query!(
            "UPDATE tasks SET name = 'Changed' WHERE id = $1",
            ids.task_id
        )
        .execute(&mut *first)
        .await
        .unwrap();
        let run_pool = pool.clone();
        let mut run = tokio::task::JoinSet::new();
        run.spawn(async move { edit(&run_pool, &ids, name).await });
        wait_for_blocked(pool, blocker).await;
        first.commit().await.unwrap();
        tokio::time::timeout(Duration::from_secs(5), run.join_next())
            .await
            .unwrap()
            .unwrap()
            .unwrap()
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn repeating_a_competing_edit_does_not_report_a_change(pool: PgPool) {
        let (returned, changed) = competing_edit(&pool, "Changed").await;
        assert_eq!((changed, returned.name.as_str()), (false, "Changed"));
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn restoring_a_competing_edit_reports_a_change(pool: PgPool) {
        let (returned, changed) = competing_edit(&pool, "Dev").await;
        assert_eq!((changed, returned.name.as_str()), (true, "Dev"));
    }

    async fn competing_activation(pool: &PgPool, active: bool) -> (Task, Option<ActiveTransition>) {
        let ids = seed(pool, OrgRole::Admin).await;
        let mut first = pool.begin().await.unwrap();
        let blocker = sqlx::query_scalar!("SELECT pg_backend_pid()")
            .fetch_one(&mut *first)
            .await
            .unwrap()
            .unwrap();
        sqlx::query!("UPDATE tasks SET active = false WHERE id = $1", ids.task_id)
            .execute(&mut *first)
            .await
            .unwrap();
        let run_pool = pool.clone();
        let mut run = tokio::task::JoinSet::new();
        run.spawn(async move {
            set_task_active_record(&run_pool, ids.org_id, ids.task_id, active).await
        });
        wait_for_blocked(pool, blocker).await;
        first.commit().await.unwrap();
        tokio::time::timeout(Duration::from_secs(5), run.join_next())
            .await
            .unwrap()
            .unwrap()
            .unwrap()
            .unwrap()
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn repeating_a_competing_deactivation_reports_no_transition(pool: PgPool) {
        let (returned, transition) = competing_activation(&pool, false).await;
        assert_eq!((transition, returned.active), (None, false));
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn restoring_a_competing_deactivation_reports_reactivation(pool: PgPool) {
        let (returned, transition) = competing_activation(&pool, true).await;
        assert_eq!(
            (transition, returned.active),
            (Some(ActiveTransition::Reactivated), true)
        );
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn missing_and_foreign_rows_are_not_noops(pool: PgPool) {
        let ids = seed(&pool, OrgRole::Admin).await;
        let other = seed(&pool, OrgRole::Admin).await;
        let before = version(&pool, ids.task_id).await;
        for (org_id, id) in [
            (ids.org_id, uuid::Uuid::now_v7()),
            (other.org_id, ids.task_id),
        ] {
            let error = update_task_record(&pool, org_id, id, "Dev", true, None)
                .await
                .unwrap_err();
            assert!(matches!(
                error,
                ServerFnError::ServerError {
                    code: NOT_FOUND,
                    ..
                }
            ));
            let error = set_task_active_record(&pool, org_id, id, true)
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
        assert_eq!(version(&pool, ids.task_id).await, before);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn both_activation_transitions_preserve_details_and_change_once(pool: PgPool) {
        let ids = seed(&pool, OrgRole::Admin).await;
        let (mut expected, _) = edit(&pool, &ids, "Renamed").await;
        for (active, transition) in [
            (false, ActiveTransition::Deactivated),
            (true, ActiveTransition::Reactivated),
        ] {
            expected.active = active;
            let (updated, actual) = set_task_active_record(&pool, ids.org_id, ids.task_id, active)
                .await
                .unwrap();
            assert_eq!((actual, updated), (Some(transition), expected.clone()));
            let before = version(&pool, ids.task_id).await;
            let (updated, actual) = set_task_active_record(&pool, ids.org_id, ids.task_id, active)
                .await
                .unwrap();
            let after = version(&pool, ids.task_id).await;
            assert_eq!((actual, updated, after), (None, expected.clone(), before));
        }
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn concurrent_deletion_returns_not_found_for_both_mutations(pool: PgPool) {
        for activation in [false, true] {
            let ids = seed(&pool, OrgRole::Admin).await;
            let mut first = pool.begin().await.unwrap();
            let blocker = sqlx::query_scalar!("SELECT pg_backend_pid()")
                .fetch_one(&mut *first)
                .await
                .unwrap()
                .unwrap();
            sqlx::query!("DELETE FROM tasks WHERE id = $1", ids.task_id)
                .execute(&mut *first)
                .await
                .unwrap();
            let run_pool = pool.clone();
            let mut run = tokio::task::JoinSet::new();
            run.spawn(async move {
                if activation {
                    set_task_active_record(&run_pool, ids.org_id, ids.task_id, true)
                        .await
                        .map(|_| ())
                } else {
                    update_task_record(&run_pool, ids.org_id, ids.task_id, "Dev", true, None)
                        .await
                        .map(|_| ())
                }
            });
            wait_for_blocked(&pool, blocker).await;
            first.commit().await.unwrap();
            let error = tokio::time::timeout(Duration::from_secs(5), run.join_next())
                .await
                .unwrap()
                .unwrap()
                .unwrap()
                .unwrap_err();
            assert!(matches!(
                error,
                ServerFnError::ServerError {
                    code: NOT_FOUND,
                    ..
                }
            ));
        }
    }
}
