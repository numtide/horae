//! Saving the profile is tested in `ops::profile`.

use super::*;
use crate::testing::seed;
use horae_core::permissions::catalog::BuiltInProfile;
use sqlx::PgPool;
use uuid::Uuid;

async fn workspace_zone(pool: &PgPool, org_id: Uuid) -> String {
    sqlx::query_scalar!("SELECT timezone FROM organizations WHERE id = $1", org_id)
        .fetch_one(pool)
        .await
        .unwrap()
}

/// Who may change the workspace zone is tested in `ops::organization`.
async fn set_workspace_zone(pool: &PgPool, org_id: Uuid, zone: &str) {
    sqlx::query!(
        "UPDATE organizations SET timezone = $2 WHERE id = $1",
        org_id,
        zone
    )
    .execute(pool)
    .await
    .unwrap();
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn new_organizations_and_people_start_on_utc_without_an_override(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Member).await;
    assert_eq!(workspace_zone(&pool, ids.org_id).await, "UTC");
    let zone = sqlx::query_scalar!("SELECT timezone FROM users WHERE id = $1", ids.user_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(zone, None);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn the_session_carries_both_zones_and_the_effective_offset(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Administrator).await;
    let me = || async {
        let user = User {
            id: ids.user_id,
            org_id: ids.org_id,
            email: "a@test.com".into(),
            name: "A".into(),
            oidc_subject: None,
            cost_rate_cents: None,
            billable_rate_cents: None,
            active: true,
            created_at: chrono::Utc::now(),
        };
        crate::server_fns::auth::current_user(&pool, user)
            .await
            .unwrap()
    };
    // Etc/GMT-3 is UTC+3 all year (the Etc names invert the sign).
    set_workspace_zone(&pool, ids.org_id, "Etc/GMT-3").await;
    let followed = me().await;
    assert_eq!(
        (
            followed.timezone,
            followed.workspace_timezone,
            followed.utc_offset_seconds
        ),
        (None, "Etc/GMT-3".to_owned(), 3 * 3600)
    );
    sqlx::query!(
        "UPDATE users SET timezone = 'Etc/GMT+5' WHERE id = $1",
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let own = me().await;
    assert_eq!(
        (own.timezone, own.utc_offset_seconds),
        (Some("Etc/GMT+5".to_owned()), -5 * 3600)
    );
}

#[test]
fn a_saved_alias_stays_among_the_choices_once() {
    let zones = zone_choices(["Etc/GMT+5".to_owned(), "Europe/Madrid".to_owned()]);
    assert_eq!(zones.iter().filter(|z| *z == "Etc/GMT+5").count(), 1);
    assert_eq!(zones.iter().filter(|z| *z == "Europe/Madrid").count(), 1);
}

#[test]
fn issuer_host_drops_scheme_and_path() {
    assert_eq!(
        issuer_host("https://sso.example.com/realms/acme"),
        "sso.example.com"
    );
    assert_eq!(
        issuer_host("https://accounts.google.com"),
        "accounts.google.com"
    );
    assert_eq!(issuer_host("sso.example.com"), "sso.example.com");
}
