//! Client server functions: session adapters over `ops::clients`.

use super::*;
use crate::models::client::{
    ClientDetailView, ClientDetails, ClientOverview, ClientProfile, ClientProfileEdit,
};
use crate::models::permission_editor::PermissionRequester;
#[cfg(feature = "server")]
use crate::ops::{self, Caller};

#[server]
pub async fn create_client_profile(
    profile: ClientProfile,
    default_rate: String,
) -> Result<ClientDetails, ServerFnError> {
    let user = require_user().await?;
    let state = crate::state::global_state().await;
    let mut write = ops::begin(&state.db, Caller::session(user.org_id, user.id)).await?;
    let result = ops::clients::create(&mut write.tx, &write.actor, &profile, &default_rate).await;
    Ok(write.commit(result, &*state.plugins).await?)
}

#[server]
pub async fn update_client_profile(
    client_id: String,
    edit: ClientProfileEdit,
) -> Result<ClientDetails, ServerFnError> {
    let user = require_user().await?;
    let client_id = client_id
        .parse()
        .map_err(|_| err(BAD_REQUEST, "Invalid client ID"))?;
    let state = crate::state::global_state().await;
    let mut write = ops::begin(&state.db, Caller::session(user.org_id, user.id)).await?;
    let result = ops::clients::update(&mut write.tx, &write.actor, client_id, &edit).await;
    Ok(write.commit(result, &*state.plugins).await?)
}

#[server]
pub async fn get_client_details(client_id: String) -> Result<ClientDetails, ServerFnError> {
    let viewer = require_user().await?;
    let client_id = client_id
        .parse()
        .map_err(|_| err(BAD_REQUEST, "Invalid client ID"))?;
    let state = crate::state::global_state().await;
    client_details_for_viewer(&state.db, viewer.org_id, viewer.id, client_id).await
}

#[cfg(feature = "server")]
async fn client_details_for_viewer(
    db: &sqlx::PgPool,
    org_id: uuid::Uuid,
    viewer_id: uuid::Uuid,
    client_id: uuid::Uuid,
) -> Result<ClientDetails, ServerFnError> {
    let mut read = ops::begin_read(db, Caller::session(org_id, viewer_id)).await?;
    let result = ops::clients::details(&mut read.tx, &read.actor, client_id).await;
    Ok(read.finish(result).await?)
}

#[server]
pub async fn list_client_invoices(client_id: String) -> Result<Vec<Invoice>, ServerFnError> {
    let viewer = require_user().await?;
    let client_id = client_id
        .parse()
        .map_err(|_| err(BAD_REQUEST, "Invalid client ID"))?;
    let state = crate::state::global_state().await;
    client_invoices_for_viewer(&state.db, viewer.org_id, viewer.id, client_id).await
}

#[cfg(feature = "server")]
async fn client_invoices_for_viewer(
    db: &sqlx::PgPool,
    org_id: uuid::Uuid,
    viewer_id: uuid::Uuid,
    client_id: uuid::Uuid,
) -> Result<Vec<Invoice>, ServerFnError> {
    let mut read = ops::begin_read(db, Caller::session(org_id, viewer_id)).await?;
    let result = ops::clients::invoices(&mut read.tx, &read.actor, client_id).await;
    Ok(read.finish(result).await?)
}

/// The canonical Clients page.
#[server]
pub async fn client_overview(
    expected_requester: Option<PermissionRequester>,
) -> Result<ClientOverview, ServerFnError> {
    let viewer = require_user().await?;
    let state = crate::state::global_state().await;
    client_overview_for(&state.db, viewer.org_id, viewer.id, expected_requester).await
}

#[server]
pub async fn client_detail_view(
    client_id: String,
    expected_requester: Option<PermissionRequester>,
) -> Result<ClientDetailView, ServerFnError> {
    let viewer = require_user().await?;
    let client_id = client_id
        .parse()
        .map_err(|_| err(BAD_REQUEST, "Invalid client ID"))?;
    let state = crate::state::global_state().await;
    client_detail_view_for(
        &state.db,
        viewer.org_id,
        viewer.id,
        client_id,
        expected_requester,
    )
    .await
}

#[cfg(feature = "server")]
async fn client_overview_for(
    db: &sqlx::PgPool,
    org_id: uuid::Uuid,
    viewer_id: uuid::Uuid,
    expected_requester: Option<PermissionRequester>,
) -> Result<ClientOverview, ServerFnError> {
    let mut read = ops::begin_read(db, Caller::session(org_id, viewer_id)).await?;
    let result = ops::clients::overview(&mut read.tx, &read.actor, expected_requester).await;
    Ok(read.finish(result).await?)
}

#[cfg(feature = "server")]
async fn client_detail_view_for(
    db: &sqlx::PgPool,
    org_id: uuid::Uuid,
    viewer_id: uuid::Uuid,
    client_id: uuid::Uuid,
    expected_requester: Option<PermissionRequester>,
) -> Result<ClientDetailView, ServerFnError> {
    let mut read = ops::begin_read(db, Caller::session(org_id, viewer_id)).await?;
    let result =
        ops::clients::detail_view(&mut read.tx, &read.actor, client_id, expected_requester).await;
    Ok(read.finish(result).await?)
}

/// Explicit client creation from the project form, including its default rate.
#[server]
pub async fn create_project_client(
    name: String,
    currency: String,
    default_rate: String,
) -> Result<crate::models::project_creation::CreationClient, ServerFnError> {
    let user = require_user().await?;
    let state = crate::state::global_state().await;
    create_project_client_record(
        &state.db,
        &*state.plugins,
        Caller::session(user.org_id, user.id),
        &name,
        &currency,
        &default_rate,
    )
    .await
}

/// Inline creation follows the standalone rules (OP18); only the reply's
/// shape differs.
#[cfg(feature = "server")]
pub(super) async fn create_project_client_record(
    db: &sqlx::PgPool,
    events: &dyn ops::EventSink,
    caller: Caller,
    name: &str,
    currency: &str,
    default_rate: &str,
) -> Result<crate::models::project_creation::CreationClient, ServerFnError> {
    let profile = ClientProfile {
        name: name.to_owned(),
        currency: currency.to_owned(),
        address: None,
        tax_id: None,
    };
    let mut write = ops::begin(db, caller).await?;
    let result = ops::clients::create(&mut write.tx, &write.actor, &profile, default_rate).await;
    let details = write.commit(result, events).await?;
    Ok(crate::models::project_creation::CreationClient {
        id: details.client.id,
        name: details.client.name,
        currency: details.client.currency,
        active: details.client.active,
        default_rate_cents: details
            .billing
            .and_then(|billing| billing.default_rate_cents),
    })
}

/// Lists clients. With `include_inactive = false` only active clients are
/// returned (the set shown in new-entry pickers); pass `true` for the management
/// view that also needs to reactivate deactivated clients.
#[server]
pub async fn list_clients(include_inactive: bool) -> Result<Vec<Client>, ServerFnError> {
    let user = require_user().await?;
    let state = crate::state::global_state().await;
    let mut read = ops::begin_read(&state.db, Caller::session(user.org_id, user.id)).await?;
    let result = ops::clients::list(&mut read.tx, &read.actor, include_inactive).await;
    Ok(read.finish(result).await?)
}

/// Archive or restore a client (FR-036).
#[server]
pub async fn set_client_active(client_id: String, active: bool) -> Result<Client, ServerFnError> {
    let user = require_user().await?;
    let client_id = parse_uuid(&client_id, "client_id")?;
    let state = crate::state::global_state().await;
    let mut write = ops::begin(&state.db, Caller::session(user.org_id, user.id)).await?;
    let result = ops::clients::set_active(&mut write.tx, &write.actor, client_id, active).await;
    Ok(write.commit(result, &*state.plugins).await?)
}

#[cfg(all(test, feature = "server"))]
mod records;
#[cfg(all(test, feature = "server"))]
use records::*;

#[cfg(all(test, feature = "server"))]
mod lifecycle_tests;
#[cfg(all(test, feature = "server"))]
mod page_tests;
#[cfg(all(test, feature = "server"))]
mod tests;
