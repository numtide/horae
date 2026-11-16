//! Creating and deleting reusable permission profiles (templates) as receipted
//! changes. Deleting one detaches its people and keeps their grants.

use horae_core::permissions::catalog::{Permission, validate_template_name};
use serde::{Deserialize, Serialize};
use sqlx::PgConnection;
use uuid::Uuid;

use super::{
    PermissionStorageError, REQUEST_CONFLICT, REQUESTER_CHANGED, access_revision, increment,
    require_administrator, restore_grants, selection, stored, unavailable,
};
use crate::models::permission_editor::{
    PermissionRequester, TemplateAction, TemplateCommand, TemplateOutcome,
};
use crate::ops::{Actor, Done, OpError, Tx};

pub(crate) const STALE: &str = "The permission state has changed; confirm again";
pub(crate) const NAME_CONFLICT: &str = "A profile with this name already exists";
pub(crate) const LIMIT: &str = "At most 50 reusable profiles are allowed";
pub(crate) const NOT_FOUND: &str = "Profile not found";

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TemplateSnapshot {
    id: Uuid,
    name: String,
    catalog_version: i32,
    grants: Vec<Permission>,
    revision: i64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct DetachedPerson {
    user_id: Uuid,
    catalog_version: i32,
    grants: Vec<Permission>,
    is_administrator: bool,
    previous_template_id: Uuid,
    previous_applied_revision: i64,
    previous_revision: i64,
    revision: i64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TemplateAudit {
    previous_access_revision: i64,
    access_revision: i64,
    before: Option<TemplateSnapshot>,
    after: Option<TemplateSnapshot>,
    /// Each entry transitions from its recorded template to individual provenance.
    detached_people: Vec<DetachedPerson>,
}

impl TemplateCommand {
    fn canonical(&self) -> Result<Self, OpError> {
        let action = match &self.action {
            TemplateAction::Create { name, grants } => TemplateAction::Create {
                name: validate_template_name(name)
                    .map_err(|error| OpError::invalid("name", error))?
                    .to_owned(),
                grants: selection(grants)?.iter().collect(),
            },
            action @ TemplateAction::Delete { .. } => action.clone(),
        };
        Ok(Self {
            request_id: self.request_id,
            expected_access_revision: self.expected_access_revision,
            action,
        })
    }
}

/// Creates or deletes a template under the organization's `AccessChange`
/// gate (`ops::begin_gated`), which the adapter takes before the actor's
/// locks. A replayed request returns its receipt's outcome. No user/project
/// DML happens here.
pub(crate) async fn save(
    tx: &mut Tx,
    actor: &Actor,
    request: &TemplateCommand,
    expected: PermissionRequester,
) -> Result<Done<TemplateOutcome>, OpError> {
    actor.expect_requester(Some(expected), REQUESTER_CHANGED)?;
    require_administrator(actor)?;
    let (org_id, actor_id) = (actor.org_id(), actor.id());
    let current_revision = access_revision(tx, org_id).await?;
    let intent = request.canonical()?;
    let intent_json = serde_json::to_value(&intent).map_err(unavailable)?;
    if let Some(receipt) = sqlx::query!(
        "SELECT format_version, intent, result FROM permission_change_receipts
         WHERE org_id = $1 AND actor_user_id = $2 AND request_id = $3",
        org_id,
        actor_id,
        intent.request_id
    )
    .fetch_optional(&mut **tx)
    .await?
    {
        if receipt.format_version != 1 {
            return Err(unavailable("Unsupported stored command format"));
        }
        if receipt.intent != intent_json {
            return Err(OpError::conflict(REQUEST_CONFLICT));
        }
        return Ok(Done::new(
            serde_json::from_value(receipt.result).map_err(unavailable)?,
        ));
    }
    if intent.expected_access_revision != current_revision {
        return Err(OpError::conflict(STALE));
    }
    let next_revision = increment(current_revision)?;
    let (before, after, detached_people) = match &intent.action {
        TemplateAction::Create { name, grants } => (
            None,
            Some(create(tx, org_id, name, grants).await?),
            Vec::new(),
        ),
        TemplateAction::Delete {
            id,
            expected_revision,
        } => {
            let (before, people) = delete(tx, org_id, *id, *expected_revision).await?;
            (Some(before), None, people)
        }
    };
    let template_id = match (&before, &after) {
        (Some(template), _) | (_, Some(template)) => template.id,
        _ => return Err(OpError::not_found(NOT_FOUND)),
    };
    let result = TemplateOutcome {
        template_id,
        access_revision: next_revision,
        detached_people: detached_people.len(),
    };
    let audit = TemplateAudit {
        previous_access_revision: current_revision,
        access_revision: next_revision,
        before,
        after,
        detached_people,
    };
    sqlx::query!(
        "UPDATE organizations SET access_revision = $2 WHERE id = $1",
        org_id,
        next_revision
    )
    .execute(&mut **tx)
    .await?;
    sqlx::query!(
        "INSERT INTO permission_change_receipts
         (id, org_id, actor_user_id, request_id, format_version, intent, result, audit)
         VALUES ($1, $2, $3, $4, 1, $5, $6, $7)",
        Uuid::now_v7(),
        org_id,
        actor_id,
        intent.request_id,
        intent_json,
        serde_json::to_value(&result).map_err(unavailable)?,
        serde_json::to_value(&audit).map_err(unavailable)?
    )
    .execute(&mut **tx)
    .await?;
    Ok(Done::new(result))
}

async fn create(
    connection: &mut PgConnection,
    org_id: Uuid,
    name: &str,
    grants: &[Permission],
) -> Result<TemplateSnapshot, OpError> {
    let count = sqlx::query_scalar!(
        "SELECT count(*) FROM permission_templates WHERE org_id = $1",
        org_id
    )
    .fetch_one(&mut *connection)
    .await?;
    if count.is_some_and(|count| count >= 50) {
        return Err(OpError::conflict(LIMIT));
    }
    let id = Uuid::now_v7();
    let grant_ids: Vec<String> = serde_json::to_value(grants)
        .and_then(serde_json::from_value)
        .map_err(unavailable)?;
    sqlx::query!(
        "INSERT INTO permission_templates (id, org_id, name, catalog_version, grants)
         VALUES ($1, $2, $3, 1, $4)",
        id,
        org_id,
        name,
        &grant_ids
    )
    .execute(connection)
    .await
    .map_err(|error| {
        if error
            .as_database_error()
            .is_some_and(|error| error.constraint() == Some("permission_templates_org_name_key"))
        {
            OpError::conflict(NAME_CONFLICT)
        } else {
            error.into()
        }
    })?;
    Ok(TemplateSnapshot {
        id,
        name: name.to_owned(),
        catalog_version: 1,
        grants: grants.to_vec(),
        revision: 0,
    })
}

async fn delete(
    connection: &mut PgConnection,
    org_id: Uuid,
    id: Uuid,
    expected_revision: i64,
) -> Result<(TemplateSnapshot, Vec<DetachedPerson>), OpError> {
    let row = sqlx::query!(
        "SELECT name, catalog_version, grants, revision FROM permission_templates WHERE org_id = $1 AND id = $2 FOR UPDATE", org_id, id
    ).fetch_optional(&mut *connection).await?.ok_or_else(|| OpError::not_found(NOT_FOUND))?;
    if row.revision != expected_revision {
        return Err(OpError::conflict(STALE));
    }
    let template = TemplateSnapshot {
        id,
        name: row.name,
        catalog_version: row.catalog_version,
        grants: restore_grants(row.catalog_version, &row.grants)
            .map_err(stored)?
            .iter()
            .collect(),
        revision: row.revision,
    };
    let rows = sqlx::query!(
        "SELECT user_id, catalog_version, grants, is_administrator, applied_template_revision, revision
         FROM person_permission_states WHERE org_id = $1 AND template_id = $2 ORDER BY user_id FOR UPDATE", org_id, id
    ).fetch_all(&mut *connection).await?;
    let mut people = Vec::with_capacity(rows.len());
    for row in rows {
        people.push(DetachedPerson {
            user_id: row.user_id,
            catalog_version: row.catalog_version,
            grants: restore_grants(row.catalog_version, &row.grants)
                .map_err(stored)?
                .iter()
                .collect(),
            is_administrator: row.is_administrator,
            previous_template_id: id,
            previous_applied_revision: row
                .applied_template_revision
                .ok_or_else(|| stored(PermissionStorageError::Provenance))?,
            previous_revision: row.revision,
            revision: increment(row.revision)?,
        });
    }
    sqlx::query!(
        "UPDATE person_permission_states SET source = 'individual', template_id = NULL,
         applied_template_revision = NULL, built_in_profile = NULL, revision = revision + 1
         WHERE org_id = $1 AND template_id = $2",
        org_id,
        id
    )
    .execute(&mut *connection)
    .await?;
    sqlx::query!(
        "DELETE FROM permission_templates WHERE org_id = $1 AND id = $2",
        org_id,
        id
    )
    .execute(connection)
    .await?;
    Ok((template, people))
}
