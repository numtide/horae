//! Client server functions.

use super::*;

// ── Clients ──────────────────────────────────────────────────────────────────

/// Lists clients. With `include_inactive = false` only active clients are
/// returned (the set shown in new-entry pickers); pass `true` for the management
/// view that also needs to reactivate deactivated clients.
#[server]
pub async fn list_clients(include_inactive: bool) -> Result<Vec<Client>, ServerFnError> {
    let user = require_user().await?;
    let state = crate::state::global_state().await;

    let clients = sqlx::query_as!(
        Client,
        r#"SELECT id, org_id, name, currency, address, tax_id, active,
                created_at as "created_at: chrono::DateTime<chrono::Utc>"
         FROM clients
         WHERE org_id = $2 AND ($1::bool OR active = true)
         ORDER BY name ASC"#,
        include_inactive,
        user.org_id,
    )
    .fetch_all(&state.db)
    .await
    .map_err(server_err)?;

    Ok(clients)
}

#[server]
pub async fn create_client(
    name: String,
    currency: String,
    address: Option<String>,
    tax_id: Option<String>,
) -> Result<Client, ServerFnError> {
    let manager = require_manager().await?;
    let state = crate::state::global_state().await;
    let id = uuid::Uuid::now_v7();
    let client = sqlx::query_as!(
        Client,
        r#"INSERT INTO clients (id, org_id, name, currency, address, tax_id)
         VALUES ($1, $2, $3, $4, $5, $6)
         RETURNING id, org_id, name, currency, address, tax_id, active,
                   created_at as "created_at: chrono::DateTime<chrono::Utc>""#,
        id,
        manager.org_id,
        name,
        currency,
        address.as_deref(),
        tax_id.as_deref(),
    )
    .fetch_one(&state.db)
    .await
    .map_err(server_err)?;

    state
        .plugins
        .dispatch(crate::plugin::AppEvent::ClientCreated {
            occurred_at: chrono::Utc::now(),
            org_id: manager.org_id,
            client: client_payload(&client),
        });
    Ok(client)
}

#[server]
pub async fn update_client(
    client_id: String,
    name: String,
    currency: String,
    address: Option<String>,
    tax_id: Option<String>,
) -> Result<Client, ServerFnError> {
    let manager = require_manager().await?;
    let state = crate::state::global_state().await;
    let client_id = parse_uuid(&client_id, "client_id")?;
    let (client, changed) = update_client_record(
        &state.db,
        manager.org_id,
        client_id,
        &name,
        &currency,
        address.as_deref(),
        tax_id.as_deref(),
    )
    .await?;
    if changed {
        state
            .plugins
            .dispatch(crate::plugin::AppEvent::ClientUpdated {
                occurred_at: chrono::Utc::now(),
                org_id: manager.org_id,
                client: client_payload(&client),
            });
    }
    Ok(client)
}

#[cfg(feature = "server")]
async fn update_client_record(
    db: &sqlx::PgPool,
    org_id: uuid::Uuid,
    client_id: uuid::Uuid,
    name: &str,
    currency: &str,
    address: Option<&str>,
    tax_id: Option<&str>,
) -> Result<(Client, bool), ServerFnError> {
    let mut tx = db.begin().await.map_err(server_err)?;
    let before = lock_client(&mut tx, org_id, client_id).await?;

    let client = sqlx::query_as!(
        Client,
        r#"UPDATE clients SET name = $3, currency = $4, address = $5, tax_id = $6
         WHERE id = $1 AND org_id = $2
           AND (name, currency, address, tax_id) IS DISTINCT FROM ($3, $4, $5, $6)
         RETURNING id, org_id, name, currency, address, tax_id, active,
                   created_at as "created_at: chrono::DateTime<chrono::Utc>""#,
        client_id,
        org_id,
        name,
        currency,
        address,
        tax_id,
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(server_err)?;

    let changed = client.is_some();
    tx.commit().await.map_err(server_err)?;
    Ok((client.unwrap_or(before), changed))
}

/// Activate or deactivate a client. Deactivated clients are hidden from
/// new-entry pickers but remain linked to existing projects and entries (FR-011).
#[server]
pub async fn set_client_active(client_id: String, active: bool) -> Result<Client, ServerFnError> {
    let manager = require_manager().await?;
    let state = crate::state::global_state().await;
    let client_id = parse_uuid(&client_id, "client_id")?;
    let (client, transition) =
        set_client_active_record(&state.db, manager.org_id, client_id, active).await?;
    if let Some(t) = transition {
        let occurred_at = chrono::Utc::now();
        let client = client_payload(&client);
        state.plugins.dispatch(match t {
            crate::plugin::event::ActiveTransition::Reactivated => {
                crate::plugin::AppEvent::ClientReactivated {
                    occurred_at,
                    org_id: manager.org_id,
                    client,
                }
            }
            crate::plugin::event::ActiveTransition::Deactivated => {
                crate::plugin::AppEvent::ClientDeactivated {
                    occurred_at,
                    org_id: manager.org_id,
                    client,
                }
            }
        });
    }
    Ok(client)
}

#[cfg(feature = "server")]
async fn set_client_active_record(
    db: &sqlx::PgPool,
    org_id: uuid::Uuid,
    client_id: uuid::Uuid,
    active: bool,
) -> Result<(Client, Option<crate::plugin::event::ActiveTransition>), ServerFnError> {
    let mut tx = db.begin().await.map_err(server_err)?;
    let before = lock_client(&mut tx, org_id, client_id).await?;

    let client = sqlx::query_as!(
        Client,
        r#"UPDATE clients SET active = $3
         WHERE id = $1 AND org_id = $2 AND active IS DISTINCT FROM $3
         RETURNING id, org_id, name, currency, address, tax_id, active,
                   created_at as "created_at: chrono::DateTime<chrono::Utc>""#,
        client_id,
        org_id,
        active,
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(server_err)?;

    let transition = client.as_ref().and_then(|updated| {
        crate::plugin::event::active_transition(Some(before.active), updated.active)
    });
    tx.commit().await.map_err(server_err)?;
    Ok((client.unwrap_or(before), transition))
}

#[cfg(feature = "server")]
async fn lock_client(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    org_id: uuid::Uuid,
    client_id: uuid::Uuid,
) -> Result<Client, ServerFnError> {
    // Both mutations compare against the latest locked row, including when
    // the requested values matched an older version before a competing edit.
    sqlx::query_as!(
        Client,
        r#"SELECT id, org_id, name, currency, address, tax_id, active,
                  created_at as "created_at: chrono::DateTime<chrono::Utc>"
           FROM clients WHERE id = $1 AND org_id = $2 FOR UPDATE"#,
        client_id,
        org_id,
    )
    .fetch_optional(&mut **tx)
    .await
    .map_err(server_err)?
    .ok_or_else(|| not_found("Client not found"))
}

#[cfg(all(test, feature = "server"))]
mod tests;
