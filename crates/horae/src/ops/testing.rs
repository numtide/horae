//! Test support for operations: an event recorder and policy-one fixtures.

use std::sync::Mutex;

use horae_core::permissions::catalog::{BuiltInProfile, Permission, PermissionSelection};
use sqlx::PgPool;
use uuid::Uuid;

use super::{AppEvent, EventSink};
use crate::server_fns::test_seed::{SeedIds, clear_state, seed, use_policy_one};

/// Keeps dispatched events so a test can inspect them.
#[derive(Default)]
pub(crate) struct Recorder(Mutex<Vec<AppEvent>>);

impl Recorder {
    pub fn take(&self) -> Vec<AppEvent> {
        std::mem::take(&mut self.0.lock().unwrap())
    }
}

impl EventSink for Recorder {
    fn dispatch(&self, event: AppEvent) {
        self.0.lock().unwrap().push(event);
    }
}

/// Seeds an organization on policy one whose person holds a built-in profile.
pub(crate) async fn seed_profile(pool: &PgPool, profile: BuiltInProfile) -> SeedIds {
    let ids = seed(pool, profile).await;
    use_policy_one(pool, ids.org_id).await;
    ids
}

/// Replaces the seeded person's permission state with `profile`'s built-in
/// state, as if the permission editor had assigned it.
pub(crate) async fn become_profile(pool: &PgPool, ids: &SeedIds, profile: BuiltInProfile) {
    clear_state(pool, ids.user_id).await;
    let mut connection = pool.acquire().await.unwrap();
    crate::server_fns::provisioning::provision(&mut connection, ids.org_id, ids.user_id, profile)
        .await
        .unwrap();
}

/// Gives the seeded person exactly `permissions` (plus prerequisites and the
/// Member floor), as the permission editor would: the organization's access
/// revision moves in the same transaction, so a reader holding the
/// organization lock makes this wait.
pub(crate) async fn set_grants(
    tx: &mut sqlx::PgConnection,
    ids: &SeedIds,
    permissions: &[Permission],
) -> Result<(), sqlx::Error> {
    let grants: Vec<String> =
        serde_json::from_value(serde_json::json!(PermissionSelection::new(permissions))).unwrap();
    sqlx::query!(
        "UPDATE organizations SET access_revision = access_revision + 1 WHERE id = $1",
        ids.org_id
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query!(
        "UPDATE person_permission_states SET grants = $2 WHERE user_id = $1",
        ids.user_id,
        &grants
    )
    .execute(&mut *tx)
    .await?;
    Ok(())
}

/// Moves the seeded organization to policy one and gives the seeded person
/// exactly `grants` (with their prerequisites and the Member floor).
pub(crate) async fn policy_one(
    pool: &PgPool,
    ids: &SeedIds,
    grants: &[Permission],
    administrator: bool,
) {
    provision(pool, ids, grants, administrator).await;
    use_policy_one(pool, ids.org_id).await;
}

/// Gives a person exactly a built-in profile's grants and administrator
/// identity, replacing any state they had. The organization's policy is left
/// alone: operations that answer only from stored state do not read it.
pub(crate) async fn assign_profile<'e>(
    executor: impl sqlx::PgExecutor<'e>,
    org_id: Uuid,
    user_id: Uuid,
    profile: BuiltInProfile,
) {
    let grants: Vec<String> = serde_json::from_value(serde_json::json!(PermissionSelection::new(
        profile.direct_permissions()
    )))
    .unwrap();
    sqlx::query!(
        "INSERT INTO person_permission_states (id,org_id,user_id,catalog_version,grants,is_administrator,source)
         VALUES ($1,$2,$3,1,$4,$5,'individual')
         ON CONFLICT (org_id,user_id) DO UPDATE
         SET grants = EXCLUDED.grants, is_administrator = EXCLUDED.is_administrator,
             source = 'individual', built_in_profile = NULL, template_id = NULL,
             applied_template_revision = NULL,
             revision = person_permission_states.revision + 1",
        Uuid::now_v7(),
        org_id,
        user_id,
        &grants,
        profile == BuiltInProfile::Administrator,
    )
    .execute(executor)
    .await
    .unwrap();
}

/// Takes a person's authority away inside `tx` as an access change does:
/// deactivating them (a write to their row only), or moving the
/// organization's access revision and replacing their profile with Member.
pub(crate) async fn revoke(
    tx: &mut sqlx::PgConnection,
    org_id: Uuid,
    user_id: Uuid,
    deactivate: bool,
) {
    if deactivate {
        sqlx::query!("UPDATE users SET active = false WHERE id = $1", user_id)
            .execute(&mut *tx)
            .await
            .unwrap();
    } else {
        sqlx::query!(
            "UPDATE organizations SET access_revision = access_revision + 1 WHERE id = $1",
            org_id
        )
        .execute(&mut *tx)
        .await
        .unwrap();
        assign_profile(&mut *tx, org_id, user_id, BuiltInProfile::Member).await;
    }
}

/// Adds an active person to the organization with a built-in profile.
pub(crate) async fn add_person(pool: &PgPool, org_id: Uuid, profile: BuiltInProfile) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO users (id, org_id, email, name) VALUES ($1, $2, $3, 'Test person')",
        id,
        org_id,
        format!("{id}@test.com"),
    )
    .execute(pool)
    .await
    .unwrap();
    assign_profile(pool, org_id, id, profile).await;
    id
}

/// Replaces the seeded person's permission state with exactly `grants`,
/// without touching the organization's policy.
pub(crate) async fn provision(
    pool: &PgPool,
    ids: &SeedIds,
    grants: &[Permission],
    administrator: bool,
) {
    clear_state(pool, ids.user_id).await;
    let grants: Vec<String> =
        serde_json::from_value(serde_json::json!(PermissionSelection::new(grants))).unwrap();
    sqlx::query!(
        "INSERT INTO person_permission_states (id,org_id,user_id,catalog_version,grants,is_administrator,source)
         VALUES ($1,$2,$3,1,$4,$5,'individual')
         ON CONFLICT (org_id,user_id) DO UPDATE
         SET grants = EXCLUDED.grants, is_administrator = EXCLUDED.is_administrator,
             source = 'individual', built_in_profile = NULL, template_id = NULL,
             applied_template_revision = NULL,
             revision = person_permission_states.revision + 1",
        Uuid::now_v7(),
        ids.org_id,
        ids.user_id,
        &grants,
        administrator,
    )
    .execute(pool)
    .await
    .unwrap();
}
