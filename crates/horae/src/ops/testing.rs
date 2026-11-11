//! Test support for operations: an event recorder and policy-one fixtures.

use std::sync::Mutex;

use horae_core::permissions::catalog::{BuiltInProfile, Permission, PermissionSelection};
use horae_core::types::OrgRole;
use sqlx::PgPool;
use uuid::Uuid;

use super::{AppEvent, EventSink};
use crate::server_fns::test_seed::{SeedIds, seed};

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
/// Stands in for `test_seed::seed(pool, BuiltInProfile)` until it lands; the
/// seeded `org_role` is dead data that no operation reads.
pub(crate) async fn seed_profile(pool: &PgPool, profile: BuiltInProfile) -> SeedIds {
    let ids = seed(pool, OrgRole::Member).await;
    policy_one(
        pool,
        &ids,
        profile.direct_permissions(),
        profile == BuiltInProfile::Administrator,
    )
    .await;
    ids
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
    sqlx::query!(
        "UPDATE organizations SET permission_policy_version = 1 WHERE id = $1",
        ids.org_id
    )
    .execute(pool)
    .await
    .unwrap();
}

/// Stores a permission state for the seeded person without touching the
/// organization's policy, for tests of domains still on legacy roles.
pub(crate) async fn provision(
    pool: &PgPool,
    ids: &SeedIds,
    grants: &[Permission],
    administrator: bool,
) {
    let grants: Vec<String> =
        serde_json::from_value(serde_json::json!(PermissionSelection::new(grants))).unwrap();
    sqlx::query!(
        "INSERT INTO person_permission_states (id,org_id,user_id,catalog_version,grants,is_administrator,source)
         VALUES ($1,$2,$3,1,$4,$5,'individual')",
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
