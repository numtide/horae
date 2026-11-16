//! Saving a person's permissions, with the relationship losses the
//! administrator confirmed, as one receipted change.

use horae_core::permissions::{
    catalog::{BuiltInProfile, PERMISSION_CATALOG_VERSION, Permission, PermissionSelection},
    person_management::has_person_management_grant,
};
use serde::Serialize;
use sqlx::PgConnection;
use uuid::Uuid;

use super::{
    REQUEST_CONFLICT, REQUESTER_CHANGED, access_revision, increment, load_permission_template,
    load_person_permissions, require_administrator, selection, stored, unavailable,
};
use crate::models::permission_audit::RemovedRelationship as RelationshipRemoval;
use crate::models::permission_editor::{
    PermissionRequester, ProfileAction, ProfileCommand, ProfileDraft, ProfileOutcome,
};
use crate::models::permissions::{PermissionSource, PersonPermissions};
use crate::ops::{Actor, Done, OpError, Tx};
use crate::plugin::AppEvent;

pub(crate) const STALE: &str = "Permission state has changed; confirm again";
pub(crate) const PERSON_NOT_FOUND: &str = "Person or profile not found";
pub(crate) const CONFIRMATION: &str = "Confirm the exact current relationship removals";
pub(crate) const LAST_ADMINISTRATOR: &str = "At least one active administrator must remain";
pub(crate) const ADMINISTRATOR_SELECTION: &str =
    "Administrator selection requires all Administrator permissions";

#[derive(Serialize)]
pub(super) struct ProfileChange {
    pub catalog_version: u32,
    pub before: PersonPermissions,
    pub after: PersonPermissions,
    pub removed_projects: Vec<RelationshipRemoval>,
    pub removed_people: Vec<RelationshipRemoval>,
}

#[derive(Serialize)]
struct ProfileAudit {
    user_id: Uuid,
    previous_access_revision: i64,
    access_revision: i64,
    change: Option<ProfileChange>,
}

fn not_found() -> OpError {
    OpError::not_found(PERSON_NOT_FOUND)
}

fn canonical_ids(ids: &mut [Uuid]) -> Result<(), OpError> {
    ids.sort_unstable();
    if ids.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(OpError::conflict(CONFIRMATION));
    }
    Ok(())
}

fn reads_projects(grants: &PermissionSelection) -> bool {
    grants.contains(Permission::ProjectReadManaged) || grants.contains(Permission::ProjectReadAll)
}

/// Saves a person's permissions under the organization's `AccessChange` gate
/// (`ops::begin_gated`), which the adapter takes before the actor's locks. A
/// save that changes them announces `user_permissions_changed`; a replayed
/// request returns its receipt's outcome and announces nothing again. No
/// user/project DML or external work happens while the gate is held.
pub(crate) async fn save(
    tx: &mut Tx,
    actor: &Actor,
    request: &ProfileCommand,
    expected: PermissionRequester,
) -> Result<Done<ProfileOutcome>, OpError> {
    actor.expect_requester(Some(expected), REQUESTER_CHANGED)?;
    require_administrator(actor)?;
    let (org_id, actor_id) = (actor.org_id(), actor.id());
    let current_revision = access_revision(tx, org_id).await?;
    let grants = selection(&request.grants)?;
    let mut intent = request.clone();
    intent.grants = grants.iter().collect();
    canonical_ids(&mut intent.remove_projects)?;
    canonical_ids(&mut intent.remove_people)?;
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
        let outcome = serde_json::from_value(receipt.result).map_err(unavailable)?;
        return Ok(Done::new(outcome));
    }
    if current_revision != intent.expected_access_revision {
        return Err(OpError::conflict(STALE));
    }
    let draft = ProfileDraft {
        user_id: intent.user_id,
        expected_access_revision: intent.expected_access_revision,
        expected_person_revision: intent.expected_person_revision,
        action: intent.action.clone(),
        grants: intent.grants.clone(),
    };
    let ProfileChange {
        before,
        after,
        removed_projects,
        removed_people,
        ..
    } = evaluate(tx, org_id, &draft, grants).await?;
    if !removed_projects
        .iter()
        .map(|row| row.id)
        .eq(intent.remove_projects.iter().copied())
        || !removed_people
            .iter()
            .map(|row| row.id)
            .eq(intent.remove_people.iter().copied())
    {
        return Err(OpError::conflict(CONFIRMATION));
    }
    let changed = before != after;
    let access_revision = if changed {
        increment(current_revision)?
    } else {
        current_revision
    };
    if changed {
        persist(tx, org_id, intent.user_id, &after).await?;
        sqlx::query!("DELETE FROM project_management_assignments WHERE org_id = $1 AND manager_id = $2 AND id = ANY($3)",
            org_id, intent.user_id, &intent.remove_projects).execute(&mut **tx).await?;
        sqlx::query!("DELETE FROM person_management_assignments WHERE org_id = $1 AND manager_id = $2 AND id = ANY($3)",
            org_id, intent.user_id, &intent.remove_people).execute(&mut **tx).await?;
        sqlx::query!(
            "UPDATE organizations SET access_revision = $2 WHERE id = $1",
            org_id,
            access_revision
        )
        .execute(&mut **tx)
        .await?;
    }
    let result = ProfileOutcome {
        user_id: intent.user_id,
        access_revision,
        person_revision: after.revision,
        changed,
    };
    let audit = ProfileAudit {
        user_id: intent.user_id,
        previous_access_revision: current_revision,
        access_revision,
        change: changed.then_some(ProfileChange {
            catalog_version: PERMISSION_CATALOG_VERSION,
            before,
            after,
            removed_projects,
            removed_people,
        }),
    };
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
    if !changed {
        return Ok(Done::new(result));
    }
    let person = sqlx::query!(
        "SELECT email, name FROM users WHERE org_id = $1 AND id = $2",
        org_id,
        intent.user_id
    )
    .fetch_one(&mut **tx)
    .await?;
    let user =
        crate::plugin::event::user_payload(tx, org_id, intent.user_id, &person.email, &person.name)
            .await?;
    Ok(Done::new(result).with(AppEvent::UserPermissionsChanged {
        occurred_at: chrono::Utc::now(),
        org_id,
        user,
    }))
}

/// Evaluates confirmed input under an already-held organization lock.
/// Both preview and save use this calculation; no state is written here.
pub(super) async fn evaluate(
    connection: &mut PgConnection,
    org_id: Uuid,
    draft: &ProfileDraft,
    grants: PermissionSelection,
) -> Result<ProfileChange, OpError> {
    if let ProfileAction::Template {
        id,
        expected_revision,
    } = &draft.action
    {
        let template = load_permission_template(connection, org_id, *id)
            .await
            .map_err(stored)?
            .ok_or_else(not_found)?;
        if template.revision != *expected_revision {
            return Err(OpError::conflict(STALE));
        }
    }
    let target_active = sqlx::query_scalar!(
        "SELECT active FROM users WHERE org_id = $1 AND id = $2 FOR SHARE",
        org_id,
        draft.user_id
    )
    .fetch_optional(&mut *connection)
    .await?
    .ok_or_else(not_found)?;
    let before = load_person_permissions(connection, org_id, draft.user_id)
        .await
        .map_err(stored)?
        .ok_or_else(not_found)?;
    if before.revision != draft.expected_person_revision {
        return Err(OpError::conflict(STALE));
    }
    let (source, is_administrator) = match draft.action {
        ProfileAction::Edit => (
            before.source,
            before.is_administrator && grants == before.grants,
        ),
        ProfileAction::BuiltIn { profile } => {
            if profile == BuiltInProfile::Administrator && grants != profile.selection() {
                return Err(OpError::invalid("grants", ADMINISTRATOR_SELECTION));
            }
            (
                PermissionSource::BuiltIn(profile),
                profile == BuiltInProfile::Administrator,
            )
        }
        ProfileAction::Template {
            id,
            expected_revision,
        } => (
            PermissionSource::Template {
                id,
                applied_revision: expected_revision,
            },
            false,
        ),
    };
    let mut after = PersonPermissions {
        grants,
        is_administrator,
        source,
        revision: before.revision,
    };
    let removed_projects = if reads_projects(&before.grants) && !reads_projects(&after.grants) {
        sqlx::query_as!(
            RelationshipRemoval,
            "SELECT id, project_id AS subject_id, revision FROM project_management_assignments
             WHERE org_id = $1 AND manager_id = $2 ORDER BY id",
            org_id,
            draft.user_id
        )
        .fetch_all(&mut *connection)
        .await?
    } else {
        vec![]
    };
    let removed_people =
        if has_person_management_grant(&before.grants)
            && !has_person_management_grant(&after.grants)
        {
            sqlx::query_as!(RelationshipRemoval,
            "SELECT id, managed_user_id AS subject_id, revision FROM person_management_assignments
             WHERE org_id = $1 AND manager_id = $2 ORDER BY id", org_id, draft.user_id
        )
            .fetch_all(&mut *connection)
            .await?
        } else {
            vec![]
        };
    if target_active && before.is_administrator && !after.is_administrator {
        ensure_other_administrator(connection, org_id, draft.user_id).await?;
    }
    if before != after {
        after.revision = increment(before.revision)?;
    }
    Ok(ProfileChange {
        catalog_version: PERMISSION_CATALOG_VERSION,
        before,
        after,
        removed_projects,
        removed_people,
    })
}

async fn ensure_other_administrator(
    connection: &mut PgConnection,
    org: Uuid,
    target: Uuid,
) -> Result<(), OpError> {
    let others = sqlx::query_scalar!(
        "SELECT p.user_id FROM person_permission_states p
         JOIN users u ON u.id = p.user_id AND u.org_id = p.org_id
         WHERE p.org_id = $1 AND p.user_id <> $2 AND p.is_administrator AND u.active
         ORDER BY p.user_id FOR SHARE OF u",
        org,
        target
    )
    .fetch_all(&mut *connection)
    .await?;
    for user in &others {
        load_person_permissions(connection, org, *user)
            .await
            .map_err(stored)?
            .ok_or_else(not_found)?;
    }
    if others.is_empty() {
        return Err(OpError::conflict(LAST_ADMINISTRATOR));
    }
    Ok(())
}

async fn persist(
    connection: &mut PgConnection,
    org: Uuid,
    user: Uuid,
    state: &PersonPermissions,
) -> Result<(), OpError> {
    let (source, built_in, template, applied_revision) = match state.source {
        PermissionSource::BuiltIn(profile) => (
            "built_in",
            Some(
                serde_json::to_value(profile)
                    .and_then(serde_json::from_value::<String>)
                    .map_err(unavailable)?,
            ),
            None,
            None,
        ),
        PermissionSource::Template {
            id,
            applied_revision,
        } => ("template", None, Some(id), Some(applied_revision)),
        PermissionSource::Individual => ("individual", None, None, None),
    };
    let grant_ids: Vec<String> = serde_json::to_value(&state.grants)
        .and_then(serde_json::from_value)
        .map_err(unavailable)?;
    sqlx::query!(
        "UPDATE person_permission_states SET grants = $3, is_administrator = $4,
         source = $5, built_in_profile = $6, template_id = $7, applied_template_revision = $8,
         revision = $9 WHERE org_id = $1 AND user_id = $2",
        org,
        user,
        &grant_ids,
        state.is_administrator,
        source,
        built_in,
        template,
        applied_revision,
        state.revision
    )
    .execute(connection)
    .await?;
    Ok(())
}
