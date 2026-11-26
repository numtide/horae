//! Every export refusal rolls back before it is answered, so the exporting
//! person's `FOR SHARE` locks do not outlive the response.

use super::*;
use crate::models::permission_editor::PermissionRequester;
use crate::models::time_report::TimeReportGrouping;
use crate::testing::organization_lockable;
use horae_core::permissions::catalog::BuiltInProfile;

fn assert_released(pool: &PgPool, ids: &SeedIds, path: &str) {
    assert!(
        organization_lockable(pool, ids.org_id),
        "{path} still holds the organization lock"
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn refused_invoice_exports_release_their_locks(pool: PgPool) {
    let member = seed(&pool, BuiltInProfile::Member).await;
    assert_eq!(
        crate::exports::begin_invoice_export(
            &pool,
            crate::ops::Caller::session(member.org_id, member.user_id)
        )
        .await
        .err(),
        Some(StatusCode::FORBIDDEN)
    );
    assert_released(&pool, &member, "begin_invoice_export");

    let ids = seed(&pool, BuiltInProfile::Administrator).await;
    let missing = Uuid::now_v7();
    let caller = crate::ops::Caller::session(ids.org_id, ids.user_id);
    assert_eq!(
        invoice(&pool, caller, missing).await.err(),
        Some(StatusCode::NOT_FOUND)
    );
    assert_released(&pool, &ids, "invoice");
    assert_eq!(
        pdf(&pool, caller, missing).await.err(),
        Some(StatusCode::NOT_FOUND)
    );
    assert_released(&pool, &ids, "pdf");
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn refused_project_exports_release_their_locks(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Administrator).await;
    assert_eq!(
        authorize_projects(&pool, ids.org_id, ids.user_id, &[Uuid::now_v7()], &[]).await,
        Err(StatusCode::FORBIDDEN)
    );
    assert_released(&pool, &ids, "authorize_projects");
    sqlx::query!(
        "UPDATE projects SET name=repeat('x',40000) WHERE id=$1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(
        projects(&pool, ids.org_id, ids.user_id, "active")
            .await
            .err(),
        Some(StatusCode::PAYLOAD_TOO_LARGE)
    );
    assert_released(&pool, &ids, "projects");
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn refused_time_exports_release_their_locks(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Administrator).await;
    let entry = add_entries(&pool, &ids, 1).await[0];
    sqlx::query!(
        "UPDATE time_entries SET notes=repeat('x',40000) WHERE id=$1",
        entry
    )
    .execute(&pool)
    .await
    .unwrap();
    let query = params().time_query().unwrap();
    assert_eq!(
        time::entries(&pool, ids.org_id, ids.user_id, &query)
            .await
            .err(),
        Some(StatusCode::PAYLOAD_TOO_LARGE)
    );
    assert_released(&pool, &ids, "time entries");

    sqlx::query!(
        "UPDATE projects SET name=repeat('n',40000) WHERE id=$1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let requester = PermissionRequester {
        org_id: ids.org_id,
        user_id: ids.user_id,
    };
    assert_eq!(
        time::groups::read(&pool, requester, &query, TimeReportGrouping::Project)
            .await
            .err(),
        Some(StatusCode::PAYLOAD_TOO_LARGE)
    );
    assert_released(&pool, &ids, "grouped time");
}
