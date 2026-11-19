//! Personal access tokens: session adapters over `ops::api_tokens`
//! (`specs/022-public-api-v1/contracts/api-tokens.md`).

use super::*;
use crate::models::api_tokens::{ApiTokenRow, CreatedApiToken};
#[cfg(feature = "server")]
use crate::ops::{self, Caller};
use chrono::{DateTime, Utc};

/// The signed-in person's own tokens, newest first.
#[server]
pub async fn list_api_tokens() -> Result<Vec<ApiTokenRow>, ServerFnError> {
    let caller = session_caller().await?;
    list_own(&crate::state::global_state().await.db, caller).await
}

/// Creates a token for the signed-in person. The answer is the only time its
/// secret is shown.
#[server]
pub async fn create_api_token(
    name: String,
    expires_at: Option<DateTime<Utc>>,
) -> Result<CreatedApiToken, ServerFnError> {
    let caller = session_caller().await?;
    create(
        &crate::state::global_state().await.db,
        caller,
        &name,
        expires_at,
    )
    .await
}

/// Revokes one of the person's tokens, or anyone's for an administrator.
#[server]
pub async fn revoke_api_token(id: String) -> Result<ApiTokenRow, ServerFnError> {
    let caller = session_caller().await?;
    let id = parse_uuid(&id, "token ID")?;
    revoke(&crate::state::global_state().await.db, caller, id).await
}

/// Every token in the organization, for an administrator.
#[server]
pub async fn list_org_api_tokens() -> Result<Vec<ApiTokenRow>, ServerFnError> {
    let caller = session_caller().await?;
    list_org(&crate::state::global_state().await.db, caller).await
}

/// The organization's account id for Harvest clients' `Harvest-Account-Id`,
/// or `None` while the Harvest-compatible API is off.
#[server]
pub async fn harvest_account_id() -> Result<Option<i64>, ServerFnError> {
    let caller = session_caller().await?;
    let state = crate::state::global_state().await;
    if !state.harvest_api {
        return Ok(None);
    }
    account_id(&state.db, caller).await.map(Some)
}

#[cfg(feature = "server")]
async fn account_id(db: &sqlx::PgPool, caller: Caller) -> Result<i64, ServerFnError> {
    let mut read = ops::begin_read(db, caller).await.map_err(op_error)?;
    let result = ops::api_tokens::harvest_account_id(&mut read.tx, &read.actor).await;
    read.finish(result).await.map_err(op_error)
}

#[cfg(feature = "server")]
async fn list_own(db: &sqlx::PgPool, caller: Caller) -> Result<Vec<ApiTokenRow>, ServerFnError> {
    let mut read = ops::begin_read(db, caller).await.map_err(op_error)?;
    let result = ops::api_tokens::list_own(&mut read.tx, &read.actor).await;
    read.finish(result).await.map_err(op_error)
}

#[cfg(feature = "server")]
async fn list_org(db: &sqlx::PgPool, caller: Caller) -> Result<Vec<ApiTokenRow>, ServerFnError> {
    let mut read = ops::begin_read(db, caller).await.map_err(op_error)?;
    let result = ops::api_tokens::list_org(&mut read.tx, &read.actor).await;
    read.finish(result).await.map_err(op_error)
}

// Token operations raise no plugin events, so nothing is dispatched after the commit.
#[cfg(feature = "server")]
async fn create(
    db: &sqlx::PgPool,
    caller: Caller,
    name: &str,
    expires_at: Option<DateTime<Utc>>,
) -> Result<CreatedApiToken, ServerFnError> {
    let mut write = ops::begin(db, caller).await.map_err(op_error)?;
    let result = ops::api_tokens::create(&mut write.tx, &write.actor, name, expires_at).await;
    let (secret, row) = write
        .commit(result, &ops::NoEvents)
        .await
        .map_err(op_error)?;
    Ok(CreatedApiToken {
        token: secret.expose().to_owned(),
        row,
    })
}

#[cfg(feature = "server")]
async fn revoke(
    db: &sqlx::PgPool,
    caller: Caller,
    id: uuid::Uuid,
) -> Result<ApiTokenRow, ServerFnError> {
    let mut write = ops::begin(db, caller).await.map_err(op_error)?;
    let result = ops::api_tokens::revoke(&mut write.tx, &write.actor, id).await;
    write.commit(result, &ops::NoEvents).await.map_err(op_error)
}

#[cfg(all(test, feature = "server"))]
mod tests;
