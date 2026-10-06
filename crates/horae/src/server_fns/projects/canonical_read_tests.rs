use super::*;
use crate::server_fns::test_seed::{SeedIds, seed};
use horae_core::permissions::catalog::{Permission, PermissionSelection};
use sqlx::PgPool;
use uuid::Uuid;

mod budgets;
mod concurrency;
mod detail;
mod overview;
mod spend;
mod tasks;
mod team;

pub(super) async fn fixture(
    pool: &PgPool,
    legacy_role: OrgRole,
    grants: PermissionSelection,
) -> (SeedIds, User) {
    let ids = seed(pool, legacy_role).await;
    let stored: Vec<String> =
        serde_json::from_value(serde_json::to_value(grants).unwrap()).unwrap();
    sqlx::query!(
        "INSERT INTO person_permission_states
         (id, org_id, user_id, catalog_version, grants, is_administrator, source)
         VALUES ($1, $2, $3, 1, $4, false, 'individual')",
        Uuid::now_v7(),
        ids.org_id,
        ids.user_id,
        &stored,
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE organizations SET permission_policy_version = 1 WHERE id = $1",
        ids.org_id,
    )
    .execute(pool)
    .await
    .unwrap();
    let viewer = User {
        id: ids.user_id,
        org_id: ids.org_id,
        email: format!("{}@test.com", ids.user_id),
        name: "Canonical project reader".into(),
        oidc_subject: None,
        org_role: legacy_role,
        cost_rate_cents: None,
        billable_rate_cents: None,
        active: true,
        created_at: chrono::Utc::now(),
    };
    (ids, viewer)
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn canonical_project_reader_lists_without_legacy_management_or_tracking(pool: PgPool) {
    let (ids, viewer) = fixture(
        &pool,
        OrgRole::Member,
        PermissionSelection::new(&[Permission::ProjectReadAll]),
    )
    .await;
    let projects = projects_for_viewer(&pool, &viewer, None, true, ProjectRead::Overview)
        .await
        .unwrap();
    assert_eq!(
        projects.iter().map(|p| p.id).collect::<Vec<_>>(),
        [ids.project_id]
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn canonical_project_reader_opens_without_legacy_management_or_tracking(pool: PgPool) {
    let (ids, _) = fixture(
        &pool,
        OrgRole::Member,
        PermissionSelection::new(&[Permission::ProjectReadAll]),
    )
    .await;
    let details = fetch_project_details(&pool, ids.org_id, ids.user_id, ids.project_id)
        .await
        .unwrap();
    assert_eq!(details.map(|p| p.id), Some(ids.project_id));
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn canonical_own_only_member_does_not_inherit_legacy_manager_project_list(pool: PgPool) {
    let (_, viewer) = fixture(&pool, OrgRole::Manager, PermissionSelection::new(&[])).await;
    let projects = projects_for_viewer(&pool, &viewer, None, true, ProjectRead::Overview)
        .await
        .unwrap();
    assert!(
        projects.is_empty(),
        "legacy Manager disclosed: {projects:?}"
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn canonical_own_only_member_does_not_inherit_legacy_manager_project_details(pool: PgPool) {
    let (ids, _) = fixture(&pool, OrgRole::Manager, PermissionSelection::new(&[])).await;
    let details = fetch_project_details(&pool, ids.org_id, ids.user_id, ids.project_id)
        .await
        .unwrap();
    assert!(details.is_none(), "legacy Manager disclosed: {details:?}");
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn canonical_project_read_does_not_inherit_legacy_admin_financial_fields(pool: PgPool) {
    let (ids, viewer) = fixture(
        &pool,
        OrgRole::Admin,
        PermissionSelection::new(&[Permission::ProjectReadAll]),
    )
    .await;
    sqlx::query!(
        "UPDATE projects SET rate_cents = 12345, budget_kind = 'amount',
         budget_amount_cents = 86753 WHERE id = $1",
        ids.project_id,
    )
    .execute(&pool)
    .await
    .unwrap();
    let projects = projects_for_viewer(&pool, &viewer, None, true, ProjectRead::Overview)
        .await
        .unwrap();
    assert_eq!(projects.len(), 1);
    let payload = serde_json::to_value(&projects[0]).unwrap();
    for field in ["rate_cents", "budget_amount_cents"] {
        assert!(
            payload.get(field).is_none(),
            "legacy Admin disclosed {field}: {payload}"
        );
    }
}

async fn replace_grants(pool: &PgPool, ids: &SeedIds, permissions: &[Permission]) {
    let grants: Vec<String> = serde_json::from_value(
        serde_json::to_value(PermissionSelection::new(permissions)).unwrap(),
    )
    .unwrap();
    sqlx::query!(
        "UPDATE person_permission_states SET grants=$3 WHERE org_id=$1 AND user_id=$2",
        ids.org_id,
        ids.user_id,
        &grants,
    )
    .execute(pool)
    .await
    .unwrap();
}

async fn designate(pool: &PgPool, ids: &SeedIds) {
    sqlx::query!(
        "INSERT INTO project_management_assignments (id, org_id, project_id, manager_id)
         VALUES ($1,$2,$3,$4)",
        Uuid::now_v7(),
        ids.org_id,
        ids.project_id,
        ids.user_id,
    )
    .execute(pool)
    .await
    .unwrap();
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn canonical_managed_project_reads_require_current_designation(pool: PgPool) {
    let (ids, viewer) = fixture(
        &pool,
        OrgRole::Manager,
        PermissionSelection::new(&[Permission::ProjectReadManaged]),
    )
    .await;
    for designated in [false, true, false] {
        if designated {
            designate(&pool, &ids).await;
        } else {
            sqlx::query!(
                "DELETE FROM project_management_assignments WHERE org_id=$1 AND manager_id=$2",
                ids.org_id,
                ids.user_id
            )
            .execute(&pool)
            .await
            .unwrap();
        }
        let projects = projects_for_viewer(&pool, &viewer, None, true, ProjectRead::Overview)
            .await
            .unwrap();
        assert_eq!(projects.len(), usize::from(designated));
        let details = fetch_project_details(&pool, ids.org_id, ids.user_id, ids.project_id)
            .await
            .unwrap();
        assert_eq!(details.is_some(), designated);
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn canonical_tags_follow_project_reads_and_revocation(pool: PgPool) {
    let (ids, _) = fixture(
        &pool,
        OrgRole::Manager,
        PermissionSelection::new(&[Permission::ProjectReadAll]),
    )
    .await;
    let tag_id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO project_tags (id,org_id,name) VALUES ($1,$2,'Launch')",
        tag_id,
        ids.org_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO project_tag_links (id,org_id,project_id,tag_id) VALUES ($1,$2,$3,$4)",
        Uuid::now_v7(),
        ids.org_id,
        ids.project_id,
        tag_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let tags = fetch_project_tags(&pool, ids.org_id, ids.user_id)
        .await
        .unwrap();
    assert_eq!(tags.len(), 1);
    assert_eq!(
        (tags[0].project_id, tags[0].tag_id),
        (ids.project_id, tag_id)
    );
    replace_grants(&pool, &ids, &[]).await;
    assert!(
        fetch_project_tags(&pool, ids.org_id, ids.user_id)
            .await
            .unwrap()
            .is_empty()
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn canonical_project_details_withhold_legacy_admin_notes_and_rate_controls(pool: PgPool) {
    let (ids, _) = fixture(
        &pool,
        OrgRole::Admin,
        PermissionSelection::new(&[Permission::ProjectReadAll]),
    )
    .await;
    sqlx::query!("INSERT INTO project_private_settings (id,org_id,project_id,admin_notes) VALUES ($1,$2,$3,'Private launch plan')",
        Uuid::now_v7(), ids.org_id, ids.project_id).execute(&pool).await.unwrap();
    let details = fetch_project_details(&pool, ids.org_id, ids.user_id, ids.project_id)
        .await
        .unwrap()
        .unwrap();
    let payload = serde_json::to_value(details).unwrap();
    for field in ["admin_notes", "task_rate_currency"] {
        assert!(
            payload.get(field).is_none(),
            "unexpected {field}: {payload}"
        );
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn canonical_project_financial_projection_matches_owning_project_evaluator(pool: PgPool) {
    use horae_core::permissions::rates::{
        BillableRateOwner, BillableRateResource, RateAction, billable_rate_access,
    };
    use horae_core::permissions::{Actor, ManagementAssignments};

    let (ids, viewer) = fixture(
        &pool,
        OrgRole::Admin,
        PermissionSelection::new(&[Permission::ProjectReadAll]),
    )
    .await;
    sqlx::query!("UPDATE projects SET rate_cents=0, budget_kind='amount', budget_amount_cents=86753 WHERE id=$1",
        ids.project_id).execute(&pool).await.unwrap();
    for designated in [false, true] {
        if designated {
            designate(&pool, &ids).await;
        }
        for financial in [
            None,
            Some(Permission::BillableRateReadManaged),
            Some(Permission::BillableRateReadAll),
        ] {
            let mut permissions = vec![Permission::ProjectReadAll];
            permissions.extend(financial);
            replace_grants(&pool, &ids, &permissions).await;
            let grants = PermissionSelection::new(&permissions);
            let actor = Actor {
                id: ids.user_id,
                org_id: ids.org_id,
                active: true,
            };
            let scope = ManagementAssignments {
                actor_id: ids.user_id,
                org_id: ids.org_id,
                people: &[ids.user_id],
                projects: if designated {
                    std::slice::from_ref(&ids.project_id)
                } else {
                    &[]
                },
            };
            let allowed = billable_rate_access(
                &grants,
                RateAction::Read,
                &actor,
                &BillableRateResource {
                    org_id: ids.org_id,
                    owner: BillableRateOwner::Project(ids.project_id),
                },
                &scope,
            );
            let projects = projects_for_viewer(&pool, &viewer, None, true, ProjectRead::Overview)
                .await
                .unwrap();
            assert_eq!(
                (projects[0].rate_cents, projects[0].budget_amount_cents),
                (allowed.then_some(0), allowed.then_some(86753)),
                "{financial:?}, designated={designated}"
            );
        }
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn canonical_project_reads_reject_unknown_policy_and_invalid_grants(pool: PgPool) {
    let (ids, viewer) = fixture(
        &pool,
        OrgRole::Admin,
        PermissionSelection::new(&[Permission::ProjectReadAll]),
    )
    .await;
    for policy in [2, 1] {
        sqlx::query!(
            "UPDATE organizations SET permission_policy_version=$2 WHERE id=$1",
            ids.org_id,
            policy
        )
        .execute(&pool)
        .await
        .unwrap();
        if policy == 1 {
            sqlx::query!("UPDATE person_permission_states SET grants=ARRAY['unknown_permission'] WHERE org_id=$1",
                ids.org_id).execute(&pool).await.unwrap();
        }
        assert!(matches!(
            assignments_for_viewer(&pool, &viewer, ids.project_id).await,
            Err(ServerFnError::ServerError {
                code: FORBIDDEN,
                ..
            })
        ));
        assert!(matches!(
            projects_for_viewer(&pool, &viewer, None, true, ProjectRead::Overview).await,
            Err(ServerFnError::ServerError {
                code: FORBIDDEN,
                ..
            })
        ));
        assert!(
            fetch_project_details(&pool, ids.org_id, ids.user_id, ids.project_id)
                .await
                .is_err()
        );
        assert!(
            fetch_project_tags(&pool, ids.org_id, ids.user_id)
                .await
                .is_err()
        );
        assert!(matches!(
            crate::server_fns::budgets::progress_for_viewer(
                &pool,
                ids.org_id,
                ids.user_id,
                "2026-09-07".parse().unwrap()
            )
            .await,
            Err(ServerFnError::ServerError {
                code: FORBIDDEN,
                ..
            })
        ));
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn canonical_shared_report_and_historical_tracking_are_distinct(pool: PgPool) {
    let (ids, viewer) = fixture(&pool, OrgRole::Member, PermissionSelection::new(&[])).await;
    sqlx::query!(
        "INSERT INTO assignments (id, project_id, user_id) VALUES ($1,$2,$3)",
        Uuid::now_v7(),
        ids.project_id,
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!("UPDATE projects SET budget_kind='hours', budget_minutes=600, budget_amount_cents=86753 WHERE id=$1",
        ids.project_id).execute(&pool).await.unwrap();
    let projects = projects_for_viewer(&pool, &viewer, None, true, ProjectRead::Overview)
        .await
        .unwrap();
    assert_eq!(
        (projects[0].budget_minutes, projects[0].budget_amount_cents),
        (Some(600), None)
    );
    crate::server_fns::test_seed::time_entry(&pool, &ids, EntryState::Open).await;
    sqlx::query!(
        "DELETE FROM assignments WHERE project_id=$1 AND user_id=$2",
        ids.project_id,
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(
        projects_for_viewer(&pool, &viewer, None, true, ProjectRead::Overview)
            .await
            .unwrap()
            .is_empty()
    );
    let tracking = projects_for_viewer(&pool, &viewer, None, true, ProjectRead::Tracking)
        .await
        .unwrap();
    assert_eq!(tracking.len(), 1);
    assert_eq!(
        (
            tracking[0].rate_cents,
            tracking[0].budget_minutes,
            tracking[0].budget_amount_cents
        ),
        (None, None, None)
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn canonical_project_reads_exclude_foreign_client_parent_links(pool: PgPool) {
    let (ids, viewer) = fixture(
        &pool,
        OrgRole::Admin,
        PermissionSelection::new(&[Permission::ProjectReadAll]),
    )
    .await;
    let other = seed(&pool, OrgRole::Admin).await;
    sqlx::query!(
        "UPDATE projects SET client_id=$2 WHERE id=$1",
        ids.project_id,
        other.client_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let tag_id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO project_tags (id,org_id,name) VALUES ($1,$2,'Launch')",
        tag_id,
        ids.org_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO project_tag_links (id,org_id,project_id,tag_id) VALUES ($1,$2,$3,$4)",
        Uuid::now_v7(),
        ids.org_id,
        ids.project_id,
        tag_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(
        projects_for_viewer(&pool, &viewer, None, true, ProjectRead::Overview)
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        fetch_project_details(&pool, ids.org_id, ids.user_id, ids.project_id)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        fetch_project_tags(&pool, ids.org_id, ids.user_id)
            .await
            .unwrap()
            .is_empty()
    );
}
