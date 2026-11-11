use super::*;
use crate::server_fns::test_seed::{SeedIds, seed};
use sqlx::PgPool;
use uuid::Uuid;

async fn profile(pool: &PgPool, user_id: Uuid) -> (String, Option<String>) {
    let row = sqlx::query!("SELECT name, timezone FROM users WHERE id = $1", user_id)
        .fetch_one(pool)
        .await
        .unwrap();
    (row.name, row.timezone)
}

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

/// A second person in the same organization.
async fn colleague(pool: &PgPool, ids: &SeedIds, role: OrgRole) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO users (id, org_id, email, name, org_role) VALUES ($1, $2, $3, 'Colleague', $4)",
        id,
        ids.org_id,
        format!("{id}@test.com"),
        role as OrgRole,
    )
    .execute(pool)
    .await
    .unwrap();
    id
}

#[track_caller]
fn assert_code(result: Result<(), ServerFnError>, expected: u16) {
    match result {
        Err(ServerFnError::ServerError { code, .. }) => assert_eq!(code, expected),
        other => panic!("expected status {expected}, got {other:?}"),
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn new_organizations_and_people_start_on_utc_without_an_override(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Member).await;
    assert_eq!(workspace_zone(&pool, ids.org_id).await, "UTC");
    assert_eq!(profile(&pool, ids.user_id).await.1, None);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_person_saves_their_name_and_zone(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Member).await;
    update_profile_record(
        &pool,
        ids.org_id,
        ids.user_id,
        "  Ana Ruiz ",
        Some("Europe/Madrid"),
    )
    .await
    .unwrap();
    assert_eq!(
        profile(&pool, ids.user_id).await,
        ("Ana Ruiz".into(), Some("Europe/Madrid".into()))
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn clearing_the_zone_follows_the_workspace_again(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Member).await;
    update_profile_record(&pool, ids.org_id, ids.user_id, "Ana", Some("Asia/Tokyo"))
        .await
        .unwrap();
    update_profile_record(&pool, ids.org_id, ids.user_id, "Ana", None)
        .await
        .unwrap();
    assert_eq!(profile(&pool, ids.user_id).await.1, None);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn an_unknown_zone_is_rejected_and_nothing_changes(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Member).await;
    for bad in ["Mars/Olympus_Mons", "europe/madrid", "+02:00", ""] {
        assert_code(
            update_profile_record(&pool, ids.org_id, ids.user_id, "Ana", Some(bad)).await,
            BAD_REQUEST,
        );
    }
    assert_eq!(
        profile(&pool, ids.user_id).await,
        ("Test User".into(), None)
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_blank_or_overlong_name_is_rejected(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Member).await;
    for bad in ["   ".to_owned(), "x".repeat(201), "a\0b".to_owned()] {
        assert_code(
            update_profile_record(&pool, ids.org_id, ids.user_id, &bad, None).await,
            BAD_REQUEST,
        );
    }
    assert_eq!(profile(&pool, ids.user_id).await.0, "Test User");
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_profile_save_leaves_colleagues_alone(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Member).await;
    let other = colleague(&pool, &ids, OrgRole::Member).await;
    update_profile_record(&pool, ids.org_id, ids.user_id, "Ana", Some("Asia/Tokyo"))
        .await
        .unwrap();
    assert_eq!(profile(&pool, other).await, ("Colleague".into(), None));
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_deactivated_person_cannot_save(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Member).await;
    sqlx::query!("UPDATE users SET active = false WHERE id = $1", ids.user_id)
        .execute(&pool)
        .await
        .unwrap();
    assert_code(
        update_profile_record(&pool, ids.org_id, ids.user_id, "Ana", None).await,
        UNAUTHORIZED,
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_personal_zone_overrides_the_workspace_zone(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Admin).await;
    set_workspace_zone(&pool, ids.org_id, "Pacific/Kiritimati").await;
    assert_eq!(
        crate::timezone::person_zone(&pool, ids.user_id)
            .await
            .unwrap(),
        "Pacific/Kiritimati"
    );
    update_profile_record(&pool, ids.org_id, ids.user_id, "Ana", Some("Etc/GMT+12"))
        .await
        .unwrap();
    assert_eq!(
        crate::timezone::person_zone(&pool, ids.user_id)
            .await
            .unwrap(),
        "Etc/GMT+12"
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn the_session_carries_both_zones_and_the_effective_offset(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Admin).await;
    let me = || async {
        let user = User {
            id: ids.user_id,
            org_id: ids.org_id,
            email: "a@test.com".into(),
            name: "A".into(),
            oidc_subject: None,
            org_role: OrgRole::Admin,
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
    update_profile_record(&pool, ids.org_id, ids.user_id, "A", Some("Etc/GMT+5"))
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
