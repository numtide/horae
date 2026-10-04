//! Current Administrator editor reads. Previews never authorize a later save.

use horae_core::permissions::catalog::{PERMISSION_CATALOG_VERSION, PermissionSelection};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use super::profiles::{self, ProfileCommandError};
use super::{load_permission_template, load_person_permissions, restore_grants};
use crate::models::permission_editor::{
    PermissionEditor, PermissionRequester, PermissionSnapshot, PermissionSubject,
    PermissionSubjectPage, ProfileDraft, ProfilePreview, ProfileSource, RelationshipRemoval,
    TemplateAssignee, TemplateChoice, TemplateDeletionPreview,
};
use crate::models::permissions::{PermissionSource, PersonPermissions};

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

async fn begin(
    pool: &PgPool,
    org: Uuid,
    actor: Uuid,
) -> Result<(Transaction<'static, Postgres>, i64), ProfileCommandError> {
    let mut tx = pool.begin().await?;
    super::configure_administration(&mut tx).await?;
    let organization = sqlx::query!(
        "SELECT permission_policy_version, access_revision FROM organizations WHERE id=$1 FOR SHARE",
        org
    ).fetch_optional(&mut *tx).await?.ok_or(ProfileCommandError::Forbidden)?;
    if organization.permission_policy_version != 1 {
        return Err(ProfileCommandError::Forbidden);
    }
    let active = sqlx::query_scalar!(
        "SELECT active FROM users WHERE org_id = $1 AND id = $2 FOR SHARE",
        org,
        actor
    )
    .fetch_optional(&mut *tx)
    .await?;
    if active != Some(true)
        || !load_person_permissions(&mut tx, org, actor)
            .await?
            .is_some_and(|state| state.is_administrator)
    {
        return Err(ProfileCommandError::Forbidden);
    }
    Ok((tx, organization.access_revision))
}

pub(crate) async fn subjects(
    pool: &PgPool,
    org: Uuid,
    actor: Uuid,
    after: Option<Uuid>,
) -> Result<PermissionSubjectPage, ProfileCommandError> {
    const PAGE_SIZE: usize = 50;
    let (mut tx, _) = begin(pool, org, actor).await?;
    let mut subjects = sqlx::query_as!(
        PermissionSubject,
        "SELECT id, name, active FROM users
         WHERE org_id=$1 AND ($2::uuid IS NULL OR id>$2)
         ORDER BY id LIMIT $3",
        org,
        after,
        (PAGE_SIZE + 1) as i64
    )
    .fetch_all(&mut *tx)
    .await?;
    let next_after = if subjects.len() > PAGE_SIZE {
        subjects.truncate(PAGE_SIZE);
        subjects.last().map(|subject| subject.id)
    } else {
        None
    };
    tx.commit().await?;
    Ok(PermissionSubjectPage {
        requester: PermissionRequester {
            org_id: org,
            user_id: actor,
        },
        subjects,
        next_after,
    })
}

pub(crate) async fn load(
    pool: &PgPool,
    org: Uuid,
    actor: Uuid,
    user: Uuid,
) -> Result<PermissionEditor, ProfileCommandError> {
    let (mut tx, access_revision) = begin(pool, org, actor).await?;
    let target = sqlx::query!(
        "SELECT name, active FROM users WHERE org_id=$1 AND id=$2 FOR SHARE",
        org,
        user
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(ProfileCommandError::NotFound)?;
    let permissions = load_person_permissions(&mut tx, org, user)
        .await?
        .ok_or(ProfileCommandError::NotFound)?
        .into();
    let templates = sqlx::query!(
        "SELECT id, name, catalog_version, grants, revision FROM permission_templates
         WHERE org_id=$1 ORDER BY lower(name), id",
        org
    )
    .fetch_all(&mut *tx)
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
    .collect::<Result<Vec<_>, ProfileCommandError>>()?;
    tx.commit().await?;
    Ok(PermissionEditor {
        requester: PermissionRequester {
            org_id: org,
            user_id: actor,
        },
        user_id: user,
        name: target.name,
        active: target.active,
        access_revision,
        permissions,
        templates,
    })
}

pub(crate) async fn preview(
    pool: &PgPool,
    org: Uuid,
    actor: Uuid,
    draft: &ProfileDraft,
) -> Result<ProfilePreview, ProfileCommandError> {
    let (mut tx, access_revision) = begin(pool, org, actor).await?;
    if access_revision != draft.expected_access_revision {
        return Err(ProfileCommandError::Stale);
    }
    let grants = PermissionSelection::from_stored(PERMISSION_CATALOG_VERSION, &draft.grants)?;
    let change = profiles::evaluate(&mut tx, org, draft, grants).await?;
    let changed = change.before != change.after;
    if changed && access_revision.checked_add(1).is_none() {
        return Err(ProfileCommandError::RevisionExhausted);
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
    .fetch_all(&mut *tx)
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
    .fetch_all(&mut *tx)
    .await?;
    if remove_projects.len() != project_ids.len() || remove_people.len() != person_ids.len() {
        return Err(ProfileCommandError::NotFound);
    }
    let result = ProfilePreview {
        user_id: draft.user_id,
        access_revision,
        before: change.before.into(),
        after: change.after.into(),
        changed,
        remove_projects,
        remove_people,
    };
    tx.commit().await?;
    Ok(result)
}

pub(crate) async fn preview_template_deletion(
    pool: &PgPool,
    org: Uuid,
    actor: Uuid,
    template_id: Uuid,
    expected_access_revision: i64,
    expected_template_revision: i64,
) -> Result<TemplateDeletionPreview, ProfileCommandError> {
    let (mut tx, access_revision) = begin(pool, org, actor).await?;
    if access_revision != expected_access_revision {
        return Err(ProfileCommandError::Stale);
    }
    let template = load_permission_template(&mut tx, org, template_id)
        .await?
        .ok_or(ProfileCommandError::NotFound)?;
    if template.revision != expected_template_revision {
        return Err(ProfileCommandError::Stale);
    }
    if access_revision.checked_add(1).is_none() {
        return Err(ProfileCommandError::RevisionExhausted);
    }
    let rows = sqlx::query!(
        "SELECT p.user_id, u.name FROM person_permission_states p
         JOIN users u ON u.org_id=p.org_id AND u.id=p.user_id
         WHERE p.org_id=$1 AND p.template_id=$2 ORDER BY p.user_id",
        org,
        template_id
    )
    .fetch_all(&mut *tx)
    .await?;
    let mut people = Vec::with_capacity(rows.len());
    for row in rows {
        let permissions = load_person_permissions(&mut tx, org, row.user_id)
            .await?
            .ok_or(ProfileCommandError::NotFound)?;
        if permissions.revision.checked_add(1).is_none() {
            return Err(ProfileCommandError::RevisionExhausted);
        }
        people.push(TemplateAssignee {
            user_id: row.user_id,
            name: row.name,
            permissions: permissions.into(),
        });
    }
    tx.commit().await?;
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
