//! Client server functions.

use super::*;
#[cfg(feature = "server")]
use crate::models::client::ClientBilling;
use crate::models::client::{ClientDetails, ClientSummary};

// ── Clients ──────────────────────────────────────────────────────────────────

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
    let row = sqlx::query!(
        r#"SELECT c.id, c.org_id, c.name, c.currency, c.address, c.tax_id, c.active,
                  c.created_at as "created_at: chrono::DateTime<chrono::Utc>",
                  u.org_role IN ('admin', 'manager') as "can_view_billing!",
                  CASE WHEN u.org_role IN ('admin', 'manager')
                       THEN c.default_rate_cents END as default_rate_cents
           FROM clients c
           JOIN users u ON u.org_id = c.org_id AND u.id = $2 AND u.active
           WHERE c.org_id = $1 AND c.id = $3"#,
        org_id,
        viewer_id,
        client_id,
    )
    .fetch_optional(db)
    .await
    .map_err(server_err)?
    .ok_or_else(|| not_found("Client not found"))?;
    Ok(ClientDetails {
        client: Client {
            id: row.id,
            org_id: row.org_id,
            name: row.name,
            currency: row.currency,
            address: row.address,
            tax_id: row.tax_id,
            active: row.active,
            created_at: row.created_at,
        },
        billing: row.can_view_billing.then_some(ClientBilling {
            default_rate_cents: row.default_rate_cents,
        }),
    })
}

#[server]
pub async fn list_client_invoices(client_id: String) -> Result<Vec<Invoice>, ServerFnError> {
    let viewer = require_manager().await?;
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
    let detail = client_details_for_viewer(db, org_id, viewer_id, client_id).await?;
    if detail.billing.is_none() {
        return Err(forbidden("Manager access required"));
    }
    super::invoices::fetch_invoices(db, org_id, None, Some(client_id)).await
}

/// Project counts and currency filters use the same progress access as Projects.
#[server]
pub async fn list_client_summaries() -> Result<Vec<ClientSummary>, ServerFnError> {
    let user = require_user().await?;
    let state = crate::state::global_state().await;
    client_summaries_for_viewer(&state.db, user.org_id, user.id).await
}

#[cfg(feature = "server")]
async fn client_summaries_for_viewer(
    db: &sqlx::PgPool,
    org_id: uuid::Uuid,
    user_id: uuid::Uuid,
) -> Result<Vec<ClientSummary>, ServerFnError> {
    let rows = sqlx::query!(
        r#"SELECT c.id, c.org_id, c.name, c.currency, c.address, c.tax_id, c.active,
                  c.created_at as "created_at: chrono::DateTime<chrono::Utc>",
                  COUNT(p.id) FILTER (WHERE p.active) as "active_projects!",
                  COUNT(p.id) as "total_projects!",
                  COALESCE(ARRAY_AGG(DISTINCT p.currency ORDER BY p.currency)
                      FILTER (WHERE p.id IS NOT NULL), ARRAY[]::text[]) as "project_currencies!"
           FROM clients c
           JOIN users u ON u.org_id = c.org_id AND u.id = $2 AND u.active
           LEFT JOIN projects p ON p.client_id = c.id AND p.org_id = c.org_id
             AND EXISTS (
               SELECT 1 FROM project_read_access a
               WHERE a.project_id = p.id AND a.org_id = c.org_id
                 AND a.user_id = u.id AND a.can_view_progress
             )
           WHERE c.org_id = $1
           GROUP BY c.id
           ORDER BY c.name, c.id"#,
        org_id,
        user_id,
    )
    .fetch_all(db)
    .await
    .map_err(server_err)?;

    Ok(rows
        .into_iter()
        .map(|row| ClientSummary {
            client: Client {
                id: row.id,
                org_id: row.org_id,
                name: row.name,
                currency: row.currency,
                address: row.address,
                tax_id: row.tax_id,
                active: row.active,
                created_at: row.created_at,
            },
            active_projects: row.active_projects,
            total_projects: row.total_projects,
            project_currencies: row.project_currencies,
        })
        .collect())
}

/// Explicit client creation from the project form, including its default rate.
#[server]
pub async fn create_project_client(
    name: String,
    currency: String,
    default_rate: String,
) -> Result<crate::models::project_creation::CreationClient, ServerFnError> {
    let actor = require_manager().await?;
    let state = crate::state::global_state().await;
    let client = super::project_creation::create_client_record(
        &state.db,
        actor.id,
        actor.org_id,
        &name,
        &currency,
        &default_rate,
    )
    .await?;
    state
        .plugins
        .dispatch(crate::plugin::AppEvent::ClientCreated {
            occurred_at: chrono::Utc::now(),
            org_id: actor.org_id,
            client: crate::plugin::event::ClientPayload {
                id: client.id,
                name: client.name.clone(),
                currency: client.currency.clone(),
                active: client.active,
            },
        });
    Ok(client)
}

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

    // This editor cannot replace the rate, so changing its denomination would
    // silently reinterpret the amount inherited by projects.
    let currency = if currency != before.currency
        && sqlx::query_scalar!(
            r#"SELECT default_rate_cents IS NOT NULL AS "has_rate!"
               FROM clients WHERE id = $1 AND org_id = $2"#,
            client_id,
            org_id,
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(server_err)?
    {
        if !currency.trim().eq_ignore_ascii_case(before.currency.trim()) {
            return Err(conflict(
                "Cannot change currency while the client has a default rate; keep its current currency",
            ));
        }
        before.currency.as_str()
    } else {
        currency
    };

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
