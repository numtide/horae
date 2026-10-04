//! Atomic person permission changes; runtime policy activation is separate.

use horae_core::permissions::{
    catalog::{
        BuiltInProfile, PERMISSION_CATALOG_VERSION, Permission, PermissionSelection,
        StoredPermissionError,
    },
    person_management::has_person_management_grant,
};
use serde::Serialize;
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use super::{PermissionStorageError, load_permission_template, load_person_permissions};
use crate::models::permissions::{PermissionSource, PersonPermissions};

pub(crate) use crate::models::permission_editor::{ProfileAction, ProfileCommand, ProfileOutcome};
use crate::models::permission_editor::{ProfileDraft, RelationshipRemoval};

#[derive(Debug, thiserror::Error)]
pub(crate) enum ProfileCommandError {
    #[error("Current administrator authority is required")]
    Forbidden,
    #[error("Permission state has changed; confirm again")]
    Stale,
    #[error("Person or profile not found")]
    NotFound,
    #[error("The request identifier was used for different intent")]
    RequestConflict,
    #[error("Confirm the exact current relationship removals")]
    Confirmation,
    #[error("At least one active administrator must remain")]
    LastAdministrator,
    #[error("Administrator selection requires all Administrator permissions")]
    AdministratorSelection,
    #[error("Permission revision exhausted")]
    RevisionExhausted,
    #[error("Unsupported stored command format")]
    ReceiptVersion,
    #[error(transparent)]
    Selection(#[from] StoredPermissionError),
    #[error(transparent)]
    Storage(#[from] PermissionStorageError),
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

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

fn increment(revision: i64) -> Result<i64, ProfileCommandError> {
    revision
        .checked_add(1)
        .ok_or(ProfileCommandError::RevisionExhausted)
}

fn canonical_ids(ids: &mut [Uuid]) -> Result<(), ProfileCommandError> {
    ids.sort_unstable();
    if ids.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(ProfileCommandError::Confirmation);
    }
    Ok(())
}

fn reads_projects(grants: &PermissionSelection) -> bool {
    grants.contains(Permission::ProjectReadManaged) || grants.contains(Permission::ProjectReadAll)
}

/// Caller supplies authenticated organization/actor IDs. No user/project DML or
/// external work occurs while holding the organization serialization gate.
pub(crate) async fn execute(
    pool: &PgPool,
    org_id: Uuid,
    actor_id: Uuid,
    request: &ProfileCommand,
) -> Result<ProfileOutcome, ProfileCommandError> {
    let mut tx = pool.begin().await?;
    super::configure_administration(&mut tx).await?;
    let org = sqlx::query!(
        "SELECT permission_policy_version, access_revision FROM organizations WHERE id = $1 FOR UPDATE", org_id
    ).fetch_optional(&mut *tx).await?.ok_or(ProfileCommandError::Forbidden)?;
    let active = sqlx::query_scalar!(
        "SELECT active FROM users WHERE org_id = $1 AND id = $2 FOR SHARE",
        org_id,
        actor_id
    )
    .fetch_optional(&mut *tx)
    .await?;
    if org.permission_policy_version != 1 || active != Some(true) {
        return Err(ProfileCommandError::Forbidden);
    }
    if !load_person_permissions(&mut tx, org_id, actor_id)
        .await?
        .is_some_and(|state| state.is_administrator)
    {
        return Err(ProfileCommandError::Forbidden);
    }
    let grants = PermissionSelection::from_stored(PERMISSION_CATALOG_VERSION, &request.grants)?;
    let mut intent = request.clone();
    intent.grants = grants.iter().collect();
    canonical_ids(&mut intent.remove_projects)?;
    canonical_ids(&mut intent.remove_people)?;
    let intent_json = serde_json::to_value(&intent)?;
    if let Some(receipt) = sqlx::query!(
        "SELECT format_version, intent, result FROM permission_change_receipts
         WHERE org_id = $1 AND actor_user_id = $2 AND request_id = $3",
        org_id,
        actor_id,
        intent.request_id
    )
    .fetch_optional(&mut *tx)
    .await?
    {
        if receipt.format_version != 1 {
            return Err(ProfileCommandError::ReceiptVersion);
        }
        if receipt.intent != intent_json {
            return Err(ProfileCommandError::RequestConflict);
        }
        let outcome = serde_json::from_value(receipt.result)?;
        tx.commit().await?;
        return Ok(outcome);
    }
    if org.access_revision != intent.expected_access_revision {
        return Err(ProfileCommandError::Stale);
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
    } = evaluate(&mut tx, org_id, &draft, grants).await?;
    if !removed_projects
        .iter()
        .map(|row| row.id)
        .eq(intent.remove_projects.iter().copied())
        || !removed_people
            .iter()
            .map(|row| row.id)
            .eq(intent.remove_people.iter().copied())
    {
        return Err(ProfileCommandError::Confirmation);
    }
    let changed = before != after;
    let access_revision = if changed {
        increment(org.access_revision)?
    } else {
        org.access_revision
    };
    if changed {
        persist(&mut tx, org_id, intent.user_id, &after).await?;
        sqlx::query!("DELETE FROM project_management_assignments WHERE org_id = $1 AND manager_id = $2 AND id = ANY($3)",
            org_id, intent.user_id, &intent.remove_projects).execute(&mut *tx).await?;
        sqlx::query!("DELETE FROM person_management_assignments WHERE org_id = $1 AND manager_id = $2 AND id = ANY($3)",
            org_id, intent.user_id, &intent.remove_people).execute(&mut *tx).await?;
        sqlx::query!(
            "UPDATE organizations SET access_revision = $2 WHERE id = $1",
            org_id,
            access_revision
        )
        .execute(&mut *tx)
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
        previous_access_revision: org.access_revision,
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
        serde_json::to_value(&result)?,
        serde_json::to_value(&audit)?
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(result)
}

/// Evaluates confirmed input under an already-held organization gate.
/// Both preview and save use this calculation; no state is written here.
pub(super) async fn evaluate(
    connection: &mut PgConnection,
    org_id: Uuid,
    draft: &ProfileDraft,
    grants: PermissionSelection,
) -> Result<ProfileChange, ProfileCommandError> {
    if let ProfileAction::Template {
        id,
        expected_revision,
    } = &draft.action
    {
        let template = load_permission_template(connection, org_id, *id)
            .await?
            .ok_or(ProfileCommandError::NotFound)?;
        if template.revision != *expected_revision {
            return Err(ProfileCommandError::Stale);
        }
    }
    let target_active = sqlx::query_scalar!(
        "SELECT active FROM users WHERE org_id = $1 AND id = $2 FOR SHARE",
        org_id,
        draft.user_id
    )
    .fetch_optional(&mut *connection)
    .await?
    .ok_or(ProfileCommandError::NotFound)?;
    let before = load_person_permissions(connection, org_id, draft.user_id)
        .await?
        .ok_or(ProfileCommandError::NotFound)?;
    if before.revision != draft.expected_person_revision {
        return Err(ProfileCommandError::Stale);
    }
    let (source, is_administrator) = match draft.action {
        ProfileAction::Edit => (
            before.source,
            before.is_administrator && grants == before.grants,
        ),
        ProfileAction::BuiltIn { profile } => {
            if profile == BuiltInProfile::Administrator && grants != profile.selection() {
                return Err(ProfileCommandError::AdministratorSelection);
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
) -> Result<(), ProfileCommandError> {
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
            .await?
            .ok_or(ProfileCommandError::NotFound)?;
    }
    if others.is_empty() {
        return Err(ProfileCommandError::LastAdministrator);
    }
    Ok(())
}

async fn persist(
    connection: &mut PgConnection,
    org: Uuid,
    user: Uuid,
    state: &PersonPermissions,
) -> Result<(), ProfileCommandError> {
    let (source, built_in, template, applied_revision) = match state.source {
        PermissionSource::BuiltIn(profile) => (
            "built_in",
            Some(serde_json::from_value::<String>(serde_json::to_value(
                profile,
            )?)?),
            None,
            None,
        ),
        PermissionSource::Template {
            id,
            applied_revision,
        } => ("template", None, Some(id), Some(applied_revision)),
        PermissionSource::Individual => ("individual", None, None, None),
    };
    let grant_ids: Vec<String> = serde_json::from_value(serde_json::to_value(&state.grants)?)?;
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
