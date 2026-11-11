//! Test support for operations: an event recorder and policy-one fixtures.

use std::sync::Mutex;

use horae_core::permissions::catalog::{Permission, PermissionSelection};
use sqlx::PgPool;
use uuid::Uuid;

use super::{AppEvent, EventSink};
use crate::server_fns::test_seed::SeedIds;

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

/// Moves the seeded organization to policy one and gives the seeded person
/// exactly `grants` (with their prerequisites and the Member floor).
pub(crate) async fn policy_one(
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
    sqlx::query!(
        "UPDATE organizations SET permission_policy_version = 1 WHERE id = $1",
        ids.org_id
    )
    .execute(pool)
    .await
    .unwrap();
}
