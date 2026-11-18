//! The permission editor's reads. Previews never authorize a later save.

use uuid::Uuid;

use super::profiles::{PERSON_NOT_FOUND, STALE, evaluate};
use super::{PermissionSource, PersonPermissions};
use super::{
    PermissionStorageError, access_revision, increment, load_permission_template,
    load_person_permissions, require_administrator, restore_grants, selection, stored,
};
use crate::models::permission_editor::{
    PermissionEditor, PermissionSnapshot, PermissionSubject, PermissionSubjectPage, ProfileDraft,
    ProfilePreview, ProfileSource, RelationshipRemoval, TemplateAssignee, TemplateChoice,
    TemplateDeletionPreview,
};
use crate::ops::{Actor, OpError, Tx};

impl From<PersonPermissions> for PermissionSnapshot {
    fn from(state: PersonPermissions) -> Self {
        Self {
            grants: state.grants.iter().collect(),
            is_administrator: state.is_administrator,
            source: match state.source {
                PermissionSource::BuiltIn(profile) => ProfileSource::BuiltIn(profile),
                PermissionSource::Template {
                    id,
                    applied_revision,
                } => ProfileSource::Template {
                    id,
                    applied_revision,
                },
                PermissionSource::Individual => ProfileSource::Individual,
            },
            revision: state.revision,
        }
    }
}

fn not_found() -> OpError {
    OpError::not_found(PERSON_NOT_FOUND)
}

/// A page of the organization's people, active or not, by id.
pub async fn subjects(
    tx: &mut Tx,
    actor: &Actor,
    after: Option<Uuid>,
) -> Result<PermissionSubjectPage, OpError> {
    const PAGE_SIZE: usize = 50;
    require_administrator(actor)?;
    let mut subjects = sqlx::query_as!(
        PermissionSubject,
        "SELECT id, name, active FROM users
         WHERE org_id=$1 AND ($2::uuid IS NULL OR id>$2)
         ORDER BY id LIMIT $3",
        actor.org_id(),
        after,
        (PAGE_SIZE + 1) as i64
    )
    .fetch_all(&mut **tx)
    .await?;
    let next_after = if subjects.len() > PAGE_SIZE {
        subjects.truncate(PAGE_SIZE);
        subjects.last().map(|subject| subject.id)
    } else {
        None
    };
    Ok(PermissionSubjectPage {
        requester: actor.requester(),
        subjects,
        next_after,
    })
}

/// A person's current permissions and the reusable profiles to choose from,
/// without inferring which profile they match.
pub async fn load(tx: &mut Tx, actor: &Actor, user: Uuid) -> Result<PermissionEditor, OpError> {
    require_administrator(actor)?;
    let org = actor.org_id();
    let access_revision = access_revision(tx, org).await?;
    let target = sqlx::query!(
        "SELECT name, active FROM users WHERE org_id=$1 AND id=$2 FOR SHARE",
        org,
        user
    )
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(not_found)?;
    let permissions = load_person_permissions(tx, org, user)
        .await
        .map_err(stored)?
        .ok_or_else(not_found)?
        .into();
    let templates = sqlx::query!(
        "SELECT id, name, catalog_version, grants, revision FROM permission_templates
         WHERE org_id=$1 ORDER BY lower(name), id",
        org
    )
    .fetch_all(&mut **tx)
    .await?
    .into_iter()
    .map(|row| {
        Ok(TemplateChoice {
            id: row.id,
            name: row.name,
            grants: restore_grants(row.catalog_version, &row.grants)?
                .iter()
                .collect(),
            revision: row.revision,
        })
    })
    .collect::<Result<Vec<_>, PermissionStorageError>>()
    .map_err(stored)?;
    Ok(PermissionEditor {
        requester: actor.requester(),
        user_id: user,
        name: target.name,
        active: target.active,
        access_revision,
        permissions,
        templates,
    })
}

/// The exact effects a save of `draft` would have now, computed as the save
/// computes them.
pub async fn preview(
    tx: &mut Tx,
    actor: &Actor,
    draft: &ProfileDraft,
) -> Result<ProfilePreview, OpError> {
    require_administrator(actor)?;
    let org = actor.org_id();
    let access_revision = access_revision(tx, org).await?;
    if access_revision != draft.expected_access_revision {
        return Err(OpError::conflict(STALE));
    }
    let grants = selection(&draft.grants)?;
    let change = evaluate(tx, org, draft, grants).await?;
    let changed = change.before != change.after;
    if changed {
        increment(access_revision)?;
    }
    // Enrich only the evaluated effects, under the same authorization gate.
    // Names are display data and must not change stored audits or command intent.
    let project_ids: Vec<_> = change.removed_projects.iter().map(|row| row.id).collect();
    let person_ids: Vec<_> = change.removed_people.iter().map(|row| row.id).collect();
    let remove_projects = sqlx::query_as!(
        RelationshipRemoval,
        "SELECT a.id, a.project_id AS subject_id, a.revision, p.name
         FROM project_management_assignments a
         JOIN projects p ON p.org_id=a.org_id AND p.id=a.project_id
         WHERE a.org_id=$1 AND a.id=ANY($2) ORDER BY a.id",
        org,
        &project_ids
    )
    .fetch_all(&mut **tx)
    .await?;
    let remove_people = sqlx::query_as!(
        RelationshipRemoval,
        "SELECT a.id, a.managed_user_id AS subject_id, a.revision, u.name
         FROM person_management_assignments a
         JOIN users u ON u.org_id=a.org_id AND u.id=a.managed_user_id
         WHERE a.org_id=$1 AND a.id=ANY($2) ORDER BY a.id",
        org,
        &person_ids
    )
    .fetch_all(&mut **tx)
    .await?;
    if remove_projects.len() != project_ids.len() || remove_people.len() != person_ids.len() {
        return Err(not_found());
    }
    Ok(ProfilePreview {
        user_id: draft.user_id,
        access_revision,
        before: change.before.into(),
        after: change.after.into(),
        changed,
        remove_projects,
        remove_people,
    })
}

/// The people a template's deletion would detach, with the grants they keep.
pub async fn preview_template_deletion(
    tx: &mut Tx,
    actor: &Actor,
    template_id: Uuid,
    expected_access_revision: i64,
    expected_template_revision: i64,
) -> Result<TemplateDeletionPreview, OpError> {
    require_administrator(actor)?;
    let org = actor.org_id();
    let access_revision = access_revision(tx, org).await?;
    if access_revision != expected_access_revision {
        return Err(OpError::conflict(STALE));
    }
    let template = load_permission_template(tx, org, template_id)
        .await
        .map_err(stored)?
        .ok_or_else(not_found)?;
    if template.revision != expected_template_revision {
        return Err(OpError::conflict(STALE));
    }
    increment(access_revision)?;
    let rows = sqlx::query!(
        "SELECT p.user_id, u.name FROM person_permission_states p
         JOIN users u ON u.org_id=p.org_id AND u.id=p.user_id
         WHERE p.org_id=$1 AND p.template_id=$2 ORDER BY p.user_id",
        org,
        template_id
    )
    .fetch_all(&mut **tx)
    .await?;
    let mut people = Vec::with_capacity(rows.len());
    for row in rows {
        let permissions = load_person_permissions(tx, org, row.user_id)
            .await
            .map_err(stored)?
            .ok_or_else(not_found)?;
        increment(permissions.revision)?;
        people.push(TemplateAssignee {
            user_id: row.user_id,
            name: row.name,
            permissions: permissions.into(),
        });
    }
    Ok(TemplateDeletionPreview {
        access_revision,
        template: TemplateChoice {
            id: template_id,
            name: template.name,
            grants: template.grants.iter().collect(),
            revision: template.revision,
        },
        people,
    })
}
