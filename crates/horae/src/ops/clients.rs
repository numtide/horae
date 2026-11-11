//! Client operations (OP17/OP18, FR-035/036 of `specs/015-scoped-permissions`).
//!
//! Every decision goes through [`Actor::has`], so one predicate serves both
//! permission policies: under policy zero the legacy role maps to the same
//! grants (`ops::actor::legacy_has`). Only the project counts on the Clients
//! page keep a legacy branch, because they follow the project domain's reader.

use horae_core::client::{optional_text, parse_default_rate, validate_profile};
use horae_core::permissions::catalog::Permission;
use horae_core::types::InvoiceStatus;
use uuid::Uuid;

use super::{Actor, Done, OpError, Tx};
use crate::models::client::{
    ClientBilling, ClientDetailView, ClientDetails, ClientOverview, ClientProfile,
    ClientProfileEdit, ClientRateChange, ClientSummary,
};
use crate::models::permission_editor::PermissionRequester;
use crate::models::{Client, Invoice};
use crate::plugin::AppEvent;
use crate::plugin::event::{ActiveTransition, ClientPayload, active_transition};

const NOT_FOUND: &str = "Client not found";
const WRITE_REQUIRED: &str = "Client management access required";
const RATE_WRITE_REQUIRED: &str =
    "Changing a client's default rate requires permission to edit billable rates for everyone";
const INVOICES_REQUIRED: &str = "Invoice access required";
const ARCHIVE_PROJECTS_FIRST: &str = "Archive this client's projects before archiving the client.";

/// FR-035: the client default is a global rate. Reading it needs the client
/// catalog plus everyone's billable rates; managed-rate grants never apply.
fn reads_default_rate(actor: &Actor) -> bool {
    actor.has(Permission::ClientReadAll) && actor.has(Permission::BillableRateReadAll)
}

fn writes_default_rate(actor: &Actor) -> bool {
    actor.has(Permission::ClientWriteAll) && actor.has(Permission::BillableRateWriteAll)
}

fn authorize_write(actor: &Actor) -> Result<(), OpError> {
    if actor.has(Permission::ClientWriteAll) {
        Ok(())
    } else {
        Err(OpError::forbidden(WRITE_REQUIRED))
    }
}

fn authorize_rate_write(actor: &Actor) -> Result<(), OpError> {
    if writes_default_rate(actor) {
        Ok(())
    } else {
        Err(OpError::forbidden(RATE_WRITE_REQUIRED))
    }
}

fn validate(profile: &ClientProfile) -> Result<(), OpError> {
    use horae_core::client::ClientValidationError as E;
    validate_profile(
        &profile.name,
        &profile.currency,
        profile.address.as_deref(),
        profile.tax_id.as_deref(),
    )
    .map_err(|error| {
        let field = match error {
            E::Name => "name",
            E::Currency => "currency",
            E::Address => "address",
            E::TaxId => "tax_id",
            E::DefaultRate => "default_rate",
        };
        OpError::invalid(field, error)
    })
}

fn parse_rate(value: &str) -> Result<Option<i64>, OpError> {
    parse_default_rate(value).map_err(|error| OpError::invalid("default_rate", error))
}

fn billing(actor: &Actor, default_rate_cents: Option<i64>) -> Option<ClientBilling> {
    reads_default_rate(actor).then_some(ClientBilling { default_rate_cents })
}

fn payload(client: &Client) -> ClientPayload {
    ClientPayload {
        id: client.id,
        name: client.name.clone(),
        currency: client.currency.clone(),
        active: client.active,
    }
}

fn requester(
    actor: &Actor,
    expected: Option<PermissionRequester>,
) -> Result<PermissionRequester, OpError> {
    let requester = PermissionRequester {
        org_id: actor.org_id(),
        user_id: actor.id(),
    };
    if expected.is_some_and(|expected| expected != requester) {
        return Err(OpError::forbidden("Client requester has changed"));
    }
    Ok(requester)
}

/// Creates a client, standalone or inline from the project form. A default
/// rate needs [`writes_default_rate`]; an empty one leaves it unset.
pub(crate) async fn create(
    tx: &mut Tx,
    actor: &Actor,
    profile: &ClientProfile,
    default_rate: &str,
) -> Result<Done<ClientDetails>, OpError> {
    authorize_write(actor)?;
    validate(profile)?;
    let rate = parse_rate(default_rate)?;
    if rate.is_some() {
        authorize_rate_write(actor)?;
    }
    let client = sqlx::query_as!(
        Client,
        r#"INSERT INTO clients (id, org_id, name, currency, address, tax_id, default_rate_cents)
           VALUES ($1,$2,$3,$4,$5,$6,$7)
           RETURNING id, org_id, name, currency, address, tax_id, active,
                     created_at as "created_at: chrono::DateTime<chrono::Utc>""#,
        Uuid::now_v7(),
        actor.org_id(),
        profile.name.trim(),
        profile.currency,
        profile.address.as_deref().and_then(optional_text),
        profile.tax_id.as_deref().and_then(optional_text),
        rate,
    )
    .fetch_one(&mut **tx)
    .await?;
    let event = AppEvent::ClientCreated {
        occurred_at: chrono::Utc::now(),
        org_id: actor.org_id(),
        client: payload(&client),
    };
    Ok(Done::new(ClientDetails {
        billing: billing(actor, rate),
        client,
    })
    .with(event))
}

/// Edits a client's profile. Keeping the rate preserves it even when the
/// actor cannot see it; replacing or clearing it needs [`writes_default_rate`].
pub(crate) async fn update(
    tx: &mut Tx,
    actor: &Actor,
    client_id: Uuid,
    edit: &ClientProfileEdit,
) -> Result<Done<ClientDetails>, OpError> {
    authorize_write(actor)?;
    let profile = &edit.profile;
    validate(profile)?;
    let replacement = match &edit.rate_change {
        ClientRateChange::Keep => None,
        ClientRateChange::Replace(amount) => Some(parse_rate(amount)?.ok_or_else(|| {
            OpError::invalid(
                "default_rate",
                "Enter a replacement rate or explicitly clear the default rate",
            )
        })?),
        ClientRateChange::Clear => None,
    };
    if edit.rate_change != ClientRateChange::Keep {
        authorize_rate_write(actor)?;
    }
    let org_id = actor.org_id();
    let before = lock(tx, org_id, client_id).await?;
    let current_rate = sqlx::query_scalar!(
        "SELECT default_rate_cents FROM clients WHERE id = $1 AND org_id = $2",
        client_id,
        org_id,
    )
    .fetch_one(&mut **tx)
    .await?;
    // An actor who cannot read the rate submits no snapshot of it.
    let rate_changed =
        reads_default_rate(actor) && edit.original.default_rate_cents != current_rate;
    if edit.original.currency != before.currency || rate_changed {
        return Err(OpError::conflict(
            "Client currency or default rate changed. Reload the client before saving.",
        ));
    }
    let rate = match edit.rate_change {
        ClientRateChange::Keep => {
            // The kept rate would silently change currency. Someone who cannot
            // see it learns only that it exists, and whom to ask.
            if profile.currency != before.currency && current_rate.is_some() {
                return Err(OpError::conflict(if writes_default_rate(actor) {
                    "Changing currency requires explicitly replacing or clearing the default rate."
                } else {
                    "This client has a default rate in its current currency. Someone who can edit billable rates must replace or clear it before the currency changes."
                }));
            }
            current_rate
        }
        ClientRateChange::Replace(_) => replacement,
        ClientRateChange::Clear => None,
    };
    let updated = sqlx::query_as!(
        Client,
        r#"UPDATE clients SET name = $3, currency = $4, address = $5, tax_id = $6, default_rate_cents = $7
           WHERE id = $1 AND org_id = $2
             AND (name, currency, address, tax_id, default_rate_cents)
                 IS DISTINCT FROM ($3, $4, $5, $6, $7)
           RETURNING id, org_id, name, currency, address, tax_id, active,
                     created_at as "created_at: chrono::DateTime<chrono::Utc>""#,
        client_id, org_id, profile.name.trim(), profile.currency,
        profile.address.as_deref().and_then(optional_text),
        profile.tax_id.as_deref().and_then(optional_text), rate,
    )
    .fetch_optional(&mut **tx)
    .await?;
    let event = updated.as_ref().map(|client| AppEvent::ClientUpdated {
        occurred_at: chrono::Utc::now(),
        org_id,
        client: payload(client),
    });
    let done = Done::new(ClientDetails {
        client: updated.unwrap_or(before),
        billing: billing(actor, rate),
    });
    Ok(match event {
        Some(event) => done.with(event),
        None => done,
    })
}

/// Archives or restores a client (FR-036). Archiving needs every linked
/// project archived first; restoring changes only the client. Archived
/// clients are hidden from new-entry pickers but stay linked to existing
/// projects and entries.
///
/// Runs at READ COMMITTED ([`super::with_tx`]): the linked-project check must
/// see projects committed while this waited for the client lock.
pub(crate) async fn set_active(
    tx: &mut Tx,
    actor: &Actor,
    client_id: Uuid,
    active: bool,
) -> Result<Done<Client>, OpError> {
    authorize_write(actor)?;
    let org_id = actor.org_id();
    let before = lock(tx, org_id, client_id).await?;
    // Project writers share-lock this client before making a project active, so
    // a separate statement after the lock sees any that committed meanwhile.
    if !active
        && before.active
        && sqlx::query_scalar!(
            r#"SELECT EXISTS(SELECT 1 FROM projects WHERE org_id = $1 AND client_id = $2 AND active) AS "exists!""#,
            org_id,
            client_id,
        )
        .fetch_one(&mut **tx)
        .await?
    {
        return Err(OpError::conflict(ARCHIVE_PROJECTS_FIRST));
    }
    let updated = sqlx::query_as!(
        Client,
        r#"UPDATE clients SET active = $3
         WHERE id = $1 AND org_id = $2 AND active IS DISTINCT FROM $3
         RETURNING id, org_id, name, currency, address, tax_id, active,
                   created_at as "created_at: chrono::DateTime<chrono::Utc>""#,
        client_id,
        org_id,
        active,
    )
    .fetch_optional(&mut **tx)
    .await?;
    let event = updated.as_ref().and_then(|client| {
        let occurred_at = chrono::Utc::now();
        let client_payload = payload(client);
        Some(
            match active_transition(Some(before.active), client.active)? {
                ActiveTransition::Reactivated => AppEvent::ClientReactivated {
                    occurred_at,
                    org_id,
                    client: client_payload,
                },
                ActiveTransition::Deactivated => AppEvent::ClientDeactivated {
                    occurred_at,
                    org_id,
                    client: client_payload,
                },
            },
        )
    });
    let done = Done::new(updated.unwrap_or(before));
    Ok(match event {
        Some(event) => done.with(event),
        None => done,
    })
}

/// Locks the latest client row, so both mutations compare against it even
/// when the request matched an older version before a competing edit.
async fn lock(tx: &mut Tx, org_id: Uuid, client_id: Uuid) -> Result<Client, OpError> {
    sqlx::query_as!(
        Client,
        r#"SELECT id, org_id, name, currency, address, tax_id, active,
                  created_at as "created_at: chrono::DateTime<chrono::Utc>"
           FROM clients WHERE id = $1 AND org_id = $2 FOR UPDATE"#,
        client_id,
        org_id,
    )
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(|| OpError::not_found(NOT_FOUND))
}

/// One client's catalog fields, plus its default rate when the actor may read
/// it. A missing, foreign or unreadable client is not found.
pub(crate) async fn details(
    tx: &mut Tx,
    actor: &Actor,
    client_id: Uuid,
) -> Result<ClientDetails, OpError> {
    if !actor.has(Permission::ClientReadAll) {
        return Err(OpError::not_found(NOT_FOUND));
    }
    let row = sqlx::query!(
        r#"SELECT id, org_id, name, currency, address, tax_id, active, default_rate_cents,
                  created_at as "created_at: chrono::DateTime<chrono::Utc>"
           FROM clients WHERE org_id = $1 AND id = $2"#,
        actor.org_id(),
        client_id,
    )
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(|| OpError::not_found(NOT_FOUND))?;
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
        billing: billing(actor, row.default_rate_cents),
    })
}

/// The client detail page: [`details`] plus what the actor may do there.
pub(crate) async fn detail_view(
    tx: &mut Tx,
    actor: &Actor,
    client_id: Uuid,
    expected: Option<PermissionRequester>,
) -> Result<ClientDetailView, OpError> {
    let requester = requester(actor, expected)?;
    let details = details(tx, actor, client_id).await?;
    Ok(ClientDetailView {
        requester,
        canonical_permissions: !actor.is_legacy(),
        can_manage: actor.has(Permission::ClientWriteAll),
        can_read_invoices: actor.has(Permission::InvoiceReadAll),
        details,
    })
}

/// The Clients page. Without catalog access it reports `can_read: false`
/// rather than an empty catalog.
pub(crate) async fn overview(
    tx: &mut Tx,
    actor: &Actor,
    expected: Option<PermissionRequester>,
) -> Result<ClientOverview, OpError> {
    let requester = requester(actor, expected)?;
    if !actor.is_active() {
        return Err(OpError::forbidden("Current client access is required"));
    }
    let scope = ClientReadScope::of(actor);
    let clients = summaries(&mut **tx, actor.org_id(), actor.id(), scope).await?;
    Ok(ClientOverview {
        requester,
        canonical_permissions: !actor.is_legacy(),
        can_read: scope.catalog,
        can_manage: actor.has(Permission::ClientWriteAll),
        clients,
    })
}

/// A client's invoices. Whole-invoice authority is organization-wide invoice
/// read; managed invoice scope belongs to the invoices domain.
pub(crate) async fn invoices(
    tx: &mut Tx,
    actor: &Actor,
    client_id: Uuid,
) -> Result<Vec<Invoice>, OpError> {
    details(tx, actor, client_id).await?;
    if !actor.has(Permission::InvoiceReadAll) {
        return Err(OpError::forbidden(INVOICES_REQUIRED));
    }
    Ok(sqlx::query_as!(
        Invoice,
        r#"SELECT id, org_id, client_id, number,
                  status as "status: InvoiceStatus",
                  issued_on as "issued_on: chrono::NaiveDate",
                  due_on as "due_on: chrono::NaiveDate",
                  currency, total_cents, notes, terms_days, po_number,
                  discount_bps, tax1_bps, tax2_name, tax2_bps,
                  subtotal_cents, discount_cents, tax1_cents, tax2_cents,
                  created_at as "created_at: chrono::DateTime<chrono::Utc>"
           FROM invoices
           WHERE org_id = $1 AND client_id = $2
           ORDER BY created_at DESC, id DESC"#,
        actor.org_id(),
        client_id,
    )
    .fetch_all(&mut **tx)
    .await?)
}

/// Clients for pickers and filters. Catalog readers get every client; anyone
/// else gets the names of clients whose projects they work on or may read,
/// without address or tax ID.
pub(crate) async fn list(
    tx: &mut Tx,
    actor: &Actor,
    include_inactive: bool,
) -> Result<Vec<Client>, OpError> {
    if !actor.is_active() {
        return Err(OpError::forbidden("Account is not active"));
    }
    let catalog = actor.has(Permission::ClientReadAll);
    Ok(sqlx::query_as!(
        Client,
        r#"SELECT c.id, c.org_id, c.name, c.currency,
                  CASE WHEN $3 THEN c.address END AS address,
                  CASE WHEN $3 THEN c.tax_id END AS tax_id,
                  c.active, c.created_at as "created_at: chrono::DateTime<chrono::Utc>"
           FROM clients c
           WHERE c.org_id = $2 AND ($1::bool OR c.active)
             AND ($3 OR EXISTS (
               SELECT 1 FROM projects p
               WHERE p.org_id = c.org_id AND p.client_id = c.id
                 AND ($4 OR ($5 AND EXISTS (
                   SELECT 1 FROM project_management_assignments m
                   WHERE m.org_id = p.org_id AND m.project_id = p.id AND m.manager_id = $6
                 )) OR EXISTS (
                   SELECT 1 FROM assignments a WHERE a.project_id = p.id AND a.user_id = $6
                 ))
             ))
           ORDER BY c.name ASC"#,
        include_inactive,
        actor.org_id(),
        catalog,
        actor.has(Permission::ProjectReadAll),
        actor.has(Permission::ProjectReadManaged),
        actor.id(),
    )
    .fetch_all(&mut **tx)
    .await?)
}

/// What the Clients page may show: the catalog itself and which projects
/// count toward each client's totals.
#[derive(Clone, Copy)]
pub(crate) struct ClientReadScope {
    pub legacy: bool,
    pub catalog: bool,
    pub projects_all: bool,
    pub projects_managed: bool,
}

impl ClientReadScope {
    fn of(actor: &Actor) -> Self {
        Self {
            legacy: actor.is_legacy(),
            catalog: actor.has(Permission::ClientReadAll),
            projects_all: actor.has(Permission::ProjectReadAll),
            projects_managed: actor.has(Permission::ProjectReadManaged),
        }
    }
}

/// Client summaries with project counts. Policy zero counts what the legacy
/// `project_read_access` view shows; policy one counts the projects
/// `list_projects` would show the same viewer.
pub(crate) async fn summaries<'e>(
    db: impl sqlx::Executor<'e, Database = sqlx::Postgres>,
    org_id: Uuid,
    user_id: Uuid,
    scope: ClientReadScope,
) -> Result<Vec<ClientSummary>, OpError> {
    if !scope.catalog {
        return Ok(Vec::new());
    }
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
             AND CASE WHEN $3 THEN EXISTS (
               SELECT 1 FROM project_read_access a
               WHERE a.project_id = p.id AND a.org_id = c.org_id
                 AND a.user_id = u.id AND a.can_view_progress
             ) ELSE $4 OR ($5 AND EXISTS (
               SELECT 1 FROM project_management_assignments m
               WHERE m.org_id = p.org_id AND m.project_id = p.id AND m.manager_id = u.id
             )) OR (EXISTS (
               SELECT 1 FROM assignments member
               WHERE member.project_id = p.id AND member.user_id = u.id
             ) AND COALESCE((
               SELECT s.report_visibility FROM project_settings s
               WHERE s.project_id = p.id AND s.org_id = p.org_id
             ), 'project_members') = 'project_members') END
           WHERE c.org_id = $1
           GROUP BY c.id
           ORDER BY c.name, c.id"#,
        org_id,
        user_id,
        scope.legacy,
        scope.projects_all,
        scope.projects_managed,
    )
    .fetch_all(db)
    .await?;
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

#[cfg(test)]
mod tests;
