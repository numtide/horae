use super::*;
use crate::plugin::event::ActiveTransition;
use crate::server_fns::test_seed::{SeedIds, seed, wait_for_blocked};
use sqlx::PgPool;
use std::time::Duration;

fn project_edit(name: &str) -> ProjectEdit<'_> {
    ProjectEdit {
        name,
        project_type: "time_and_materials",
        currency: "EUR",
        budget_kind: "none",
        budget_value: "",
        rate_value: "",
    }
}

mod project {
    use super::*;

    #[sqlx::test(migrations = "./migrations")]
    async fn each_detail_field_changes_once_on_an_inactive_project(pool: PgPool) {
        for fields in [
            ProjectEdit {
                name: "Renamed",
                ..project_edit("Widget")
            },
            ProjectEdit {
                project_type: "fixed_fee",
                ..project_edit("Widget")
            },
            ProjectEdit {
                currency: "USD",
                ..project_edit("Widget")
            },
        ] {
            let ids = seed(&pool, OrgRole::Admin).await;
            let (mut expected, _) =
                set_project_active_record(&pool, ids.org_id, ids.project_id, false)
                    .await
                    .unwrap();
            expected.name = fields.name.into();
            expected.project_type = parse_enum(fields.project_type, "project_type").unwrap();
            expected.currency = fields.currency.into();
            let (updated, changed) =
                update_project_record(&pool, ids.org_id, ids.project_id, &fields)
                    .await
                    .unwrap();
            assert_eq!((changed, updated), (true, expected.clone()));
            let before = version(&pool, ids.project_id).await;
            let (repeated, changed) =
                update_project_record(&pool, ids.org_id, ids.project_id, &fields)
                    .await
                    .unwrap();
            let after = version(&pool, ids.project_id).await;
            assert_eq!((changed, repeated, after), (false, expected, before));
        }
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn budgets_and_rates_distinguish_null_zero_and_normalized_values(pool: PgPool) {
        let ids = seed(&pool, OrgRole::Admin).await;
        for (kind, value, rate, amount, minutes, cents) in [
            ("amount", "1.25", "", Some(125), None, None),
            ("amount", "0", "", Some(0), None, None),
            ("amount", "", "", None, None, None),
            ("hours", "1:30", "", None, Some(90), None),
            ("hours", "2", "", None, Some(120), None),
            ("hours", "", "", None, None, None),
            ("hours", "", "0", None, None, Some(0)),
            ("hours", "", "120.50", None, None, Some(12050)),
            ("hours", "", "", None, None, None),
            ("none", "", "", None, None, None),
        ] {
            let fields = ProjectEdit {
                budget_kind: kind,
                budget_value: value,
                rate_value: rate,
                ..project_edit("Widget")
            };
            let (updated, changed) =
                update_project_record(&pool, ids.org_id, ids.project_id, &fields)
                    .await
                    .unwrap();
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
            let padded_value = format!(" {value} ");
            let padded_rate = format!(" {rate} ");
            let normalized = ProjectEdit {
                budget_value: &padded_value,
                rate_value: &padded_rate,
                ..fields
            };
            let (repeated, changed) =
                update_project_record(&pool, ids.org_id, ids.project_id, &normalized)
                    .await
                    .unwrap();
            let after = version(&pool, ids.project_id).await;
            assert_eq!((changed, repeated, after), (false, updated, before));
        }
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn invalid_project_fields_leave_the_row_unchanged(pool: PgPool) {
        let ids = seed(&pool, OrgRole::Admin).await;
        let before = version(&pool, ids.project_id).await;
        for fields in [
            ProjectEdit {
                project_type: "unknown",
                ..project_edit("Widget")
            },
            ProjectEdit {
                budget_kind: "unknown",
                ..project_edit("Widget")
            },
            ProjectEdit {
                budget_kind: "amount",
                budget_value: "-1",
                ..project_edit("Widget")
            },
            ProjectEdit {
                budget_kind: "hours",
                budget_value: "NaN",
                ..project_edit("Widget")
            },
            ProjectEdit {
                rate_value: "-1",
                ..project_edit("Widget")
            },
            ProjectEdit {
                rate_value: "92233720368547758.08",
                ..project_edit("Widget")
            },
        ] {
            assert!(
                update_project_record(&pool, ids.org_id, ids.project_id, &fields)
                    .await
                    .is_err()
            );
        }
        assert_eq!(version(&pool, ids.project_id).await, before);
    }

    async fn edit(pool: &PgPool, ids: &SeedIds, name: &str) -> (Project, bool) {
        update_project_record(pool, ids.org_id, ids.project_id, &project_edit(name))
            .await
            .unwrap()
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

    async fn competing_edit(pool: &PgPool, name: &'static str) -> (Project, bool) {
        let ids = seed(pool, OrgRole::Admin).await;
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
        let (returned, changed) = competing_edit(&pool, "Widget").await;
        assert_eq!((changed, returned.name.as_str()), (true, "Widget"));
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
            let error = update_project_record(&pool, org_id, id, &project_edit("Widget"))
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
    async fn concurrent_deletion_returns_not_found_for_both_mutations(pool: PgPool) {
        for activation in [false, true] {
            let ids = seed(&pool, OrgRole::Admin).await;
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
                    update_project_record(
                        &run_pool,
                        ids.org_id,
                        ids.project_id,
                        &project_edit("Widget"),
                    )
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

mod task {
    use super::*;

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
