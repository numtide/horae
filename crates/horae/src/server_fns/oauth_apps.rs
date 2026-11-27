//! OAuth apps in administration: session adapters over `ops::oauth::clients`
//! and `ops::oauth::grants` (spec 024, contracts/server-functions.md, PR D).
//! Only a signed-in administrator manages apps; the operations refuse anyone
//! else.

use super::*;
#[cfg(feature = "server")]
use crate::models::oauth_apps::AppKind;
use crate::models::oauth_apps::{AppInput, AppRow, CreatedApp, GrantRow};
#[cfg(feature = "server")]
use crate::ops::{self, Caller};
#[cfg(feature = "server")]
use horae_core::permissions::oauth::{ClientKind, ScopeSet};
#[cfg(feature = "server")]
use uuid::Uuid;

/// Every app of the organization, Horae's own first.
#[server]
pub async fn list_oauth_apps() -> Result<Vec<AppRow>, ServerFnError> {
    let caller = unchecked_session_caller().await?;
    list(&crate::state::global_state().await.db, caller).await
}

/// One app, for its page.
#[server]
pub async fn get_oauth_app(id: String) -> Result<AppRow, ServerFnError> {
    let caller = unchecked_session_caller().await?;
    let id = parse_uuid(&id, "app ID")?;
    get(&crate::state::global_state().await.db, caller, id).await
}

/// Whether apps can connect: the operator has set the public address the
/// authorization server needs. Registering works either way.
#[server]
pub async fn oauth_apps_can_connect() -> Result<bool, ServerFnError> {
    active_session_caller().await?;
    Ok(crate::state::global_state().await.public_url.is_some())
}

/// Registers an app. The answer is the only time a confidential app's secret
/// is shown.
#[server]
pub async fn create_oauth_app(app: AppInput) -> Result<CreatedApp, ServerFnError> {
    let caller = unchecked_session_caller().await?;
    create(&crate::state::global_state().await.db, caller, app).await
}

/// Changes an app's name, redirect URIs and scopes; its kind stays.
#[server]
pub async fn update_oauth_app(id: String, changes: AppInput) -> Result<AppRow, ServerFnError> {
    let caller = unchecked_session_caller().await?;
    let id = parse_uuid(&id, "app ID")?;
    update(&crate::state::global_state().await.db, caller, id, changes).await
}

#[server]
pub async fn set_oauth_app_enabled(id: String, enabled: bool) -> Result<AppRow, ServerFnError> {
    let caller = unchecked_session_caller().await?;
    let id = parse_uuid(&id, "app ID")?;
    set_enabled(&crate::state::global_state().await.db, caller, id, enabled).await
}

/// Gives a confidential app a new secret, shown this once; the old one stops
/// working at once.
#[server]
pub async fn replace_oauth_app_secret(id: String) -> Result<CreatedApp, ServerFnError> {
    let caller = unchecked_session_caller().await?;
    let id = parse_uuid(&id, "app ID")?;
    replace_secret(&crate::state::global_state().await.db, caller, id).await
}

/// Deletes an app with its authorizations and tokens.
#[server]
pub async fn delete_oauth_app(id: String) -> Result<(), ServerFnError> {
    let caller = unchecked_session_caller().await?;
    let id = parse_uuid(&id, "app ID")?;
    delete(&crate::state::global_state().await.db, caller, id).await
}

/// The app's active authorizations, newest first.
#[server]
pub async fn list_oauth_app_grants(id: String) -> Result<Vec<GrantRow>, ServerFnError> {
    let caller = unchecked_session_caller().await?;
    let id = parse_uuid(&id, "app ID")?;
    list_grants(&crate::state::global_state().await.db, caller, id).await
}

/// Ends anyone's authorization, for an administrator.
#[server]
pub async fn revoke_oauth_grant(id: String) -> Result<(), ServerFnError> {
    let caller = unchecked_session_caller().await?;
    let id = parse_uuid(&id, "authorization ID")?;
    revoke_grant(&crate::state::global_state().await.db, caller, id).await
}

#[cfg(feature = "server")]
fn app_row(client: ops::oauth::clients::OAuthClient) -> AppRow {
    AppRow {
        id: client.id,
        client_id: client.client_id,
        name: client.name,
        kind: match client.kind {
            ClientKind::Public => AppKind::Public,
            ClientKind::Confidential => AppKind::Confidential,
        },
        scopes: scope_names(client.scopes),
        redirect_uris: client.redirect_uris,
        built_in: client.built_in,
        enabled: client.enabled,
        secret_prefix: client.secret_prefix,
        active_grants: client.active_grants,
        created_at: client.created_at,
        created_by_name: client.created_by_name,
    }
}

#[cfg(feature = "server")]
fn grant_row(grant: ops::oauth::grants::GrantRow) -> GrantRow {
    GrantRow {
        id: grant.id,
        app_name: grant.app_name,
        client_id: grant.client_id,
        person_id: grant.person_id,
        person_name: grant.person_name,
        person_email: grant.person_email,
        scopes: scope_names(grant.scopes),
        approved_at: grant.approved_at,
        last_used_at: grant.last_used_at,
        expires_at: grant.expires_at,
    }
}

#[cfg(feature = "server")]
fn scope_names(scopes: ScopeSet) -> Vec<String> {
    scopes.names().into_iter().map(str::to_owned).collect()
}

/// Like [`op_error`], and names the field an invalid input blames, so the
/// form shows the message under it. A conflict names its field itself.
#[cfg(feature = "server")]
fn form_error(error: ops::OpError) -> ServerFnError {
    let field = match error.kind() {
        ops::OpError::Invalid {
            field: Some(field), ..
        } => Some(*field),
        _ => None,
    };
    op_error(match field {
        Some(field) => error.with_field(field),
        None => error,
    })
}

#[cfg(feature = "server")]
async fn list(db: &sqlx::PgPool, caller: Caller) -> Result<Vec<AppRow>, ServerFnError> {
    let mut read = ops::begin_read(db, caller).await.map_err(op_error)?;
    let result = ops::oauth::clients::list(&mut read.tx, &read.actor).await;
    let clients = read.finish(result).await.map_err(op_error)?;
    Ok(clients.into_iter().map(app_row).collect())
}

#[cfg(feature = "server")]
async fn get(db: &sqlx::PgPool, caller: Caller, id: Uuid) -> Result<AppRow, ServerFnError> {
    let mut read = ops::begin_read(db, caller).await.map_err(op_error)?;
    let result = ops::oauth::clients::get(&mut read.tx, &read.actor, id).await;
    read.finish(result).await.map(app_row).map_err(op_error)
}

// The app operations raise no plugin events, so nothing is dispatched after the commit.
#[cfg(feature = "server")]
async fn create(
    db: &sqlx::PgPool,
    caller: Caller,
    app: AppInput,
) -> Result<CreatedApp, ServerFnError> {
    let input = ops::oauth::clients::NewClient {
        scopes: app.scopes,
        name: app.name,
        kind: match app.kind {
            AppKind::Public => ClientKind::Public,
            AppKind::Confidential => ClientKind::Confidential,
        },
        redirect_uris: app.redirect_uris,
    };
    let mut write = ops::begin(db, caller).await.map_err(op_error)?;
    let result = ops::oauth::clients::register(&mut write.tx, &write.actor, input).await;
    let (client, secret) = write
        .commit(result, &ops::NoEvents)
        .await
        .map_err(form_error)?;
    Ok(CreatedApp {
        app: app_row(client),
        secret: secret.map(|secret| secret.expose().to_owned()),
    })
}

#[cfg(feature = "server")]
async fn update(
    db: &sqlx::PgPool,
    caller: Caller,
    id: Uuid,
    changes: AppInput,
) -> Result<AppRow, ServerFnError> {
    let changes = ops::oauth::clients::ClientChanges {
        scopes: changes.scopes,
        name: changes.name,
        redirect_uris: changes.redirect_uris,
    };
    let mut write = ops::begin(db, caller).await.map_err(op_error)?;
    let result = ops::oauth::clients::update(&mut write.tx, &write.actor, id, changes).await;
    write
        .commit(result, &ops::NoEvents)
        .await
        .map(app_row)
        .map_err(form_error)
}

#[cfg(feature = "server")]
async fn set_enabled(
    db: &sqlx::PgPool,
    caller: Caller,
    id: Uuid,
    enabled: bool,
) -> Result<AppRow, ServerFnError> {
    let mut write = ops::begin(db, caller).await.map_err(op_error)?;
    let result = ops::oauth::clients::set_enabled(&mut write.tx, &write.actor, id, enabled).await;
    write
        .commit(result, &ops::NoEvents)
        .await
        .map(app_row)
        .map_err(op_error)
}

#[cfg(feature = "server")]
async fn replace_secret(
    db: &sqlx::PgPool,
    caller: Caller,
    id: Uuid,
) -> Result<CreatedApp, ServerFnError> {
    let mut write = ops::begin(db, caller).await.map_err(op_error)?;
    let result = ops::oauth::clients::replace_secret(&mut write.tx, &write.actor, id).await;
    let (client, secret) = write
        .commit(result, &ops::NoEvents)
        .await
        .map_err(op_error)?;
    Ok(CreatedApp {
        app: app_row(client),
        secret: Some(secret.expose().to_owned()),
    })
}

#[cfg(feature = "server")]
async fn delete(db: &sqlx::PgPool, caller: Caller, id: Uuid) -> Result<(), ServerFnError> {
    let mut write = ops::begin(db, caller).await.map_err(op_error)?;
    let result = ops::oauth::clients::delete(&mut write.tx, &write.actor, id).await;
    write.commit(result, &ops::NoEvents).await.map_err(op_error)
}

#[cfg(feature = "server")]
async fn list_grants(
    db: &sqlx::PgPool,
    caller: Caller,
    app_id: Uuid,
) -> Result<Vec<GrantRow>, ServerFnError> {
    let mut read = ops::begin_read(db, caller).await.map_err(op_error)?;
    let result = ops::oauth::grants::list_for_client(&mut read.tx, &read.actor, app_id).await;
    let grants = read.finish(result).await.map_err(op_error)?;
    Ok(grants.into_iter().map(grant_row).collect())
}

#[cfg(feature = "server")]
async fn revoke_grant(db: &sqlx::PgPool, caller: Caller, id: Uuid) -> Result<(), ServerFnError> {
    let mut write = ops::begin(db, caller).await.map_err(op_error)?;
    let result = ops::oauth::grants::revoke(&mut write.tx, &write.actor, id).await;
    write.commit(result, &ops::NoEvents).await.map_err(op_error)
}

#[cfg(all(test, feature = "server"))]
mod tests;
