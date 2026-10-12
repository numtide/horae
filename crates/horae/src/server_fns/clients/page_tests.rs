//! Canonical Clients page readers under both permission policies.

use super::*;
use crate::server_fns::test_seed::{SeedIds, seed, wait_for_blocked};
use horae_core::permissions::catalog::{Permission, PermissionSelection};
use sqlx::PgPool;
use uuid::Uuid;

const ADDRESS: &str = "Hidden Street 1";
const TAX_ID: &str = "HIDDEN-VAT";

async fn set_grants(pool: &PgPool, user_id: Uuid, permissions: &[Permission]) {
    let grants: Vec<String> =
        serde_json::from_value(serde_json::json!(PermissionSelection::new(permissions))).unwrap();
    sqlx::query!(
        "UPDATE person_permission_states SET grants=$2 WHERE user_id=$1",
        user_id,
        &grants
    )
    .execute(pool)
    .await
    .unwrap();
}

/// A disposable organization on policy one, with a client carrying every
/// protected field so payload checks can look for leaks.
async fn policy_one(pool: &PgPool, role: OrgRole, permissions: &[Permission]) -> SeedIds {
    let ids = seed(pool, role).await;
    sqlx::query!(
        "UPDATE clients SET address=$2, tax_id=$3, default_rate_cents=12345 WHERE id=$1",
        ids.client_id,
        ADDRESS,
        TAX_ID,
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO person_permission_states (id,org_id,user_id,catalog_version,grants,is_administrator,source)
         VALUES ($1,$2,$3,1,'{}',false,'individual')",
        Uuid::now_v7(), ids.org_id, ids.user_id,
    )
    .execute(pool)
    .await
    .unwrap();
    set_grants(pool, ids.user_id, permissions).await;
    sqlx::query!(
        "UPDATE organizations SET permission_policy_version=1 WHERE id=$1",
        ids.org_id
    )
    .execute(pool)
    .await
    .unwrap();
    ids
}

async fn overview(pool: &PgPool, ids: &SeedIds) -> Result<ClientOverview, ServerFnError> {
    client_overview_for(pool, ids.org_id, ids.user_id, None).await
}

async fn detail(pool: &PgPool, ids: &SeedIds) -> Result<ClientDetailView, ServerFnError> {
    client_detail_view_for(pool, ids.org_id, ids.user_id, ids.client_id, None).await
}

fn assert_code(error: ServerFnError, expected: u16) {
    assert!(
        matches!(error, ServerFnError::ServerError { code, .. } if code == expected),
        "expected {expected}, got {error:?}"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn read_only_client_grant_shows_catalog_without_writes_rates_or_projects(pool: PgPool) {
    let ids = policy_one(&pool, OrgRole::Member, &[Permission::ClientReadAll]).await;
    let page = overview(&pool, &ids).await.unwrap();
    assert!(page.canonical_permissions && page.can_read && !page.can_manage);
    assert_eq!(page.clients.len(), 1);
    let summary = &page.clients[0];
    assert_eq!(summary.client.id, ids.client_id);
    assert_eq!(summary.client.address.as_deref(), Some(ADDRESS));
    assert_eq!((summary.active_projects, summary.total_projects), (0, 0));
    assert!(summary.project_currencies.is_empty());

    let view = detail(&pool, &ids).await.unwrap();
    assert!(view.canonical_permissions && !view.can_manage);
    assert_eq!(view.details.client.tax_id.as_deref(), Some(TAX_ID));
    assert!(view.details.billing.is_none());
    assert!(!serde_json::to_string(&view).unwrap().contains("12345"));
}

#[sqlx::test(migrations = "./migrations")]
async fn unrelated_grants_and_legacy_roles_do_not_open_the_catalog(pool: PgPool) {
    for (role, permissions) in [
        (OrgRole::Admin, vec![]),
        (OrgRole::Manager, vec![Permission::ProjectReadAll]),
        (OrgRole::Member, vec![Permission::InvoiceReadAll]),
        (OrgRole::Member, vec![Permission::BillableRateReadAll]),
        (OrgRole::Member, vec![Permission::ReportProfitabilityRead]),
    ] {
        let ids = policy_one(&pool, role, &permissions).await;
        let page = overview(&pool, &ids).await.unwrap();
        assert!(
            page.canonical_permissions && !page.can_read && !page.can_manage,
            "{permissions:?}"
        );
        let payload = serde_json::to_string(&page).unwrap();
        for hidden in ["Acme", ADDRESS, TAX_ID, "12345"] {
            assert!(!payload.contains(hidden), "{hidden} in {payload}");
        }
        assert_code(detail(&pool, &ids).await.unwrap_err(), NOT_FOUND);
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn default_rate_requires_the_global_billable_rate_grant(pool: PgPool) {
    for (rate_grant, visible) in [
        (Permission::BillableRateReadAll, true),
        (Permission::BillableRateReadManaged, false),
    ] {
        let ids = policy_one(
            &pool,
            OrgRole::Manager,
            &[Permission::ClientReadAll, rate_grant],
        )
        .await;
        let billing = detail(&pool, &ids).await.unwrap().details.billing;
        assert_eq!(
            billing.map(|billing| billing.default_rate_cents),
            visible.then_some(Some(12345)),
            "{rate_grant:?}"
        );
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn project_counts_follow_the_policy_one_project_reader(pool: PgPool) {
    let ids = policy_one(
        &pool,
        OrgRole::Member,
        &[Permission::ClientReadAll, Permission::ProjectReadAll],
    )
    .await;
    let counts = |page: ClientOverview| {
        let summary = &page.clients[0];
        (summary.total_projects, summary.project_currencies.clone())
    };
    assert_eq!(
        counts(overview(&pool, &ids).await.unwrap()),
        (1, vec!["EUR".to_owned()])
    );

    set_grants(&pool, ids.user_id, &[Permission::ClientReadAll]).await;
    assert_eq!(counts(overview(&pool, &ids).await.unwrap()), (0, vec![]));
    sqlx::query!(
        "INSERT INTO assignments (id, project_id, user_id) VALUES ($1,$2,$3)",
        Uuid::now_v7(),
        ids.project_id,
        ids.user_id,
    )
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(counts(overview(&pool, &ids).await.unwrap()).0, 1);
    sqlx::query!(
        "INSERT INTO project_settings (id, org_id, project_id, creator_id, rate_mode, report_visibility)
         VALUES ($1,$2,$3,$4,'project','managers')",
        Uuid::now_v7(), ids.org_id, ids.project_id, ids.user_id,
    ).execute(&pool).await.unwrap();
    assert_eq!(counts(overview(&pool, &ids).await.unwrap()).0, 0);
}

#[sqlx::test(migrations = "./migrations")]
async fn managed_project_counts_require_an_actual_management_link(pool: PgPool) {
    let ids = policy_one(
        &pool,
        OrgRole::Member,
        &[Permission::ClientReadAll, Permission::ProjectReadManaged],
    )
    .await;
    let total = |page: ClientOverview| page.clients[0].total_projects;
    assert_eq!(total(overview(&pool, &ids).await.unwrap()), 0);
    sqlx::query!(
        "INSERT INTO project_management_assignments (id,org_id,manager_id,project_id) VALUES ($1,$2,$3,$4)",
        Uuid::now_v7(), ids.org_id, ids.user_id, ids.project_id,
    ).execute(&pool).await.unwrap();
    assert_eq!(total(overview(&pool, &ids).await.unwrap()), 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn visible_default_rate_distinguishes_unset_from_zero(pool: PgPool) {
    let ids = policy_one(
        &pool,
        OrgRole::Member,
        &[Permission::ClientReadAll, Permission::BillableRateReadAll],
    )
    .await;
    for (rate, serialized) in [
        (None, r#""billing":{"default_rate_cents":null}"#),
        (Some(0), r#""billing":{"default_rate_cents":0}"#),
    ] {
        sqlx::query!(
            "UPDATE clients SET default_rate_cents = $2 WHERE id = $1",
            ids.client_id,
            rate
        )
        .execute(&pool)
        .await
        .unwrap();
        let payload = serde_json::to_string(&detail(&pool, &ids).await.unwrap()).unwrap();
        assert!(payload.contains(serialized), "{payload}");
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn foreign_missing_and_rebound_requests_fail_closed(pool: PgPool) {
    let ids = policy_one(&pool, OrgRole::Member, &[Permission::ClientReadAll]).await;
    let foreign = seed(&pool, OrgRole::Admin).await;
    for client_id in [foreign.client_id, Uuid::now_v7()] {
        let error = client_detail_view_for(&pool, ids.org_id, ids.user_id, client_id, None)
            .await
            .unwrap_err();
        assert_code(error, NOT_FOUND);
    }
    // A known organization does not admit an actor from another one.
    let error = client_overview_for(&pool, ids.org_id, foreign.user_id, None)
        .await
        .unwrap_err();
    assert_code(error, FORBIDDEN);
    let error = client_detail_view_for(&pool, ids.org_id, foreign.user_id, ids.client_id, None)
        .await
        .unwrap_err();
    assert_code(error, NOT_FOUND);

    let page = overview(&pool, &ids).await.unwrap();
    let bound = Some(page.requester);
    assert_eq!(
        client_overview_for(&pool, ids.org_id, ids.user_id, bound)
            .await
            .unwrap(),
        page
    );
    let error = client_overview_for(&pool, foreign.org_id, foreign.user_id, bound)
        .await
        .unwrap_err();
    assert_code(error, FORBIDDEN);
    let error = client_detail_view_for(
        &pool,
        foreign.org_id,
        foreign.user_id,
        foreign.client_id,
        bound,
    )
    .await
    .unwrap_err();
    assert_code(error, FORBIDDEN);
}

#[sqlx::test(migrations = "./migrations")]
async fn unavailable_permission_state_and_inactive_actors_fail_closed(pool: PgPool) {
    for state in ["unknown_policy", "missing", "malformed", "inactive"] {
        let ids = policy_one(&pool, OrgRole::Admin, &[Permission::ClientReadAll]).await;
        match state {
            "unknown_policy" => sqlx::query!(
                "UPDATE organizations SET permission_policy_version=99 WHERE id=$1",
                ids.org_id
            )
            .execute(&pool)
            .await
            .unwrap(),
            "missing" => sqlx::query!(
                "DELETE FROM person_permission_states WHERE user_id=$1",
                ids.user_id
            )
            .execute(&pool)
            .await
            .unwrap(),
            "malformed" => sqlx::query!(
                "UPDATE person_permission_states SET grants=ARRAY['unknown_permission'] WHERE user_id=$1",
                ids.user_id
            )
            .execute(&pool)
            .await
            .unwrap(),
            "inactive" => sqlx::query!("UPDATE users SET active=false WHERE id=$1", ids.user_id)
                .execute(&pool)
                .await
                .unwrap(),
            _ => unreachable!(),
        };
        assert_code(overview(&pool, &ids).await.unwrap_err(), FORBIDDEN);
        let expected = if state == "inactive" {
            NOT_FOUND
        } else {
            FORBIDDEN
        };
        assert_code(detail(&pool, &ids).await.unwrap_err(), expected);
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn readers_observe_revocation_after_waiting_for_current_authority(pool: PgPool) {
    for read_detail in [false, true] {
        let ids = policy_one(&pool, OrgRole::Admin, &[Permission::ClientReadAll]).await;
        let mut hold = pool.begin().await.unwrap();
        sqlx::query!(
            "UPDATE organizations SET access_revision=access_revision+1 WHERE id=$1",
            ids.org_id
        )
        .execute(&mut *hold)
        .await
        .unwrap();
        let blocker = sqlx::query_scalar!("SELECT pg_backend_pid() AS \"pid!\"")
            .fetch_one(&mut *hold)
            .await
            .unwrap();
        let run_pool = pool.clone();
        let (org_id, user_id, client_id) = (ids.org_id, ids.user_id, ids.client_id);
        let pending = tokio::spawn(async move {
            if read_detail {
                client_detail_view_for(&run_pool, org_id, user_id, client_id, None)
                    .await
                    .map(|_| ())
            } else {
                let page = client_overview_for(&run_pool, org_id, user_id, None).await?;
                assert!(!page.can_read && page.clients.is_empty());
                Ok(())
            }
        });
        wait_for_blocked(&pool, blocker).await;
        let grants: Vec<String> =
            serde_json::from_value(serde_json::json!(PermissionSelection::new(&[]))).unwrap();
        sqlx::query!(
            "UPDATE person_permission_states SET grants=$2 WHERE user_id=$1",
            ids.user_id,
            &grants
        )
        .execute(&mut *hold)
        .await
        .unwrap();
        hold.commit().await.unwrap();
        let result = pending.await.unwrap();
        if read_detail {
            assert_code(result.unwrap_err(), NOT_FOUND);
        } else {
            result.unwrap();
        }
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn policy_zero_keeps_legacy_catalog_billing_and_manager_affordance(pool: PgPool) {
    for (role, manager) in [
        (OrgRole::Admin, true),
        (OrgRole::Manager, true),
        (OrgRole::Member, false),
    ] {
        let ids = seed(&pool, role).await;
        sqlx::query!(
            "UPDATE clients SET default_rate_cents=12345 WHERE id=$1",
            ids.client_id
        )
        .execute(&pool)
        .await
        .unwrap();
        let page = overview(&pool, &ids).await.unwrap();
        assert!(!page.canonical_permissions && page.can_read);
        assert_eq!(page.can_manage, manager, "{role:?}");
        assert_eq!(
            page.clients,
            client_summaries_for_viewer(&pool, ids.org_id, ids.user_id)
                .await
                .unwrap()
        );
        let view = detail(&pool, &ids).await.unwrap();
        assert_eq!(view.can_manage, manager);
        assert_eq!(
            view.details,
            client_details_for_viewer(&pool, ids.org_id, ids.user_id, ids.client_id)
                .await
                .unwrap()
        );
        assert_eq!(view.details.billing.is_some(), manager);
    }
}

/// Deferred: the shared legacy readers are not cut over by this page slice.
#[sqlx::test(migrations = "./migrations")]
async fn legacy_workflow_readers_are_unchanged_under_policy_one(pool: PgPool) {
    let ids = policy_one(&pool, OrgRole::Manager, &[]).await;
    let legacy = client_details_for_viewer(&pool, ids.org_id, ids.user_id, ids.client_id)
        .await
        .unwrap();
    assert_eq!(legacy.client.tax_id.as_deref(), Some(TAX_ID));
    assert!(legacy.billing.is_some());
    assert_eq!(
        client_summaries_for_viewer(&pool, ids.org_id, ids.user_id)
            .await
            .unwrap()
            .len(),
        1
    );
}
