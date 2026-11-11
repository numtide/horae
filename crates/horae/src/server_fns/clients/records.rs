//! The client writes as the tests drive them: one actor's request through the
//! same `ops::begin` / `commit` path the server functions take, reporting what
//! the plugin host would have received.

pub(super) use crate::models::client::{ClientRateChange, ClientSummary};
use crate::ops::clients::{ClientReadScope, summaries};
use crate::ops::testing::Recorder;
use crate::plugin::AppEvent;
pub(super) use crate::plugin::event::ActiveTransition;

use super::*;

pub(super) async fn create_client_profile_record(
    db: &sqlx::PgPool,
    org_id: uuid::Uuid,
    actor_id: uuid::Uuid,
    profile: &ClientProfile,
    default_rate: &str,
) -> Result<ClientDetails, ServerFnError> {
    let mut write = ops::begin(db, Caller::session(org_id, actor_id)).await?;
    let result = ops::clients::create(&mut write.tx, &write.actor, profile, default_rate).await;
    Ok(write.commit(result, &Recorder::default()).await?)
}

/// Also reports whether a `ClientUpdated` event was dispatched.
pub(super) async fn update_client_profile_record(
    db: &sqlx::PgPool,
    org_id: uuid::Uuid,
    actor_id: uuid::Uuid,
    client_id: uuid::Uuid,
    edit: &ClientProfileEdit,
) -> Result<(ClientDetails, bool), ServerFnError> {
    let events = Recorder::default();
    let mut write = ops::begin(db, Caller::session(org_id, actor_id)).await?;
    let result = ops::clients::update(&mut write.tx, &write.actor, client_id, edit).await;
    let detail = write.commit(result, &events).await?;
    let events = events.take();
    assert!(
        events
            .iter()
            .all(|event| matches!(event, AppEvent::ClientUpdated { .. })),
        "{events:?}"
    );
    Ok((detail, !events.is_empty()))
}

/// Also reports the activity transition the dispatched event announced.
pub(super) async fn set_client_active_record(
    db: &sqlx::PgPool,
    org_id: uuid::Uuid,
    actor_id: uuid::Uuid,
    client_id: uuid::Uuid,
    active: bool,
) -> Result<(Client, Option<ActiveTransition>), ServerFnError> {
    let events = Recorder::default();
    let mut write = ops::begin(db, Caller::session(org_id, actor_id)).await?;
    let result = ops::clients::set_active(&mut write.tx, &write.actor, client_id, active).await;
    let client = write.commit(result, &events).await?;
    let transitions: Vec<_> = events
        .take()
        .into_iter()
        .map(|event| match event {
            AppEvent::ClientReactivated { .. } => ActiveTransition::Reactivated,
            AppEvent::ClientDeactivated { .. } => ActiveTransition::Deactivated,
            other => panic!("unexpected event {other:?}"),
        })
        .collect();
    assert!(transitions.len() <= 1, "{transitions:?}");
    Ok((client, transitions.into_iter().next()))
}

/// Policy-zero summaries, kept for parity tests of the canonical overview.
pub(super) async fn client_summaries_for_viewer(
    db: &sqlx::PgPool,
    org_id: uuid::Uuid,
    user_id: uuid::Uuid,
) -> Result<Vec<ClientSummary>, ServerFnError> {
    let legacy = ClientReadScope {
        legacy: true,
        catalog: true,
        projects_all: false,
        projects_managed: false,
    };
    Ok(summaries(db, org_id, user_id, legacy).await?)
}
