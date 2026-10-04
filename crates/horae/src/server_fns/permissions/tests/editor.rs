use super::*;
use crate::models::permission_editor::ProfileDraft;
use crate::server_fns::permissions::editor;

fn draft(request: &ProfileCommand) -> ProfileDraft {
    ProfileDraft {
        user_id: request.user_id,
        expected_access_revision: request.expected_access_revision,
        expected_person_revision: request.expected_person_revision,
        action: request.action.clone(),
        grants: request.grants.clone(),
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn editor_preview_matches_saved_effects_without_writing(pool: PgPool) {
    let ids = fixture(&pool).await;
    let user = person(&pool, ids.org_id, false, BuiltInProfile::ProjectManager).await;
    let (project, managed_person, incoming) = relationships(&pool, &ids, user).await;
    let mut command = apply(user, BuiltInProfile::Member);
    let loaded = editor::load(&pool, ids.org_id, ids.user_id, user)
        .await
        .unwrap();
    let preview = editor::preview(&pool, ids.org_id, ids.user_id, &draft(&command))
        .await
        .unwrap();
    assert_eq!(preview.before, loaded.permissions);
    assert!(preview.changed);
    assert_eq!(
        preview
            .remove_projects
            .iter()
            .map(|row| row.id)
            .collect::<Vec<_>>(),
        vec![project]
    );
    assert_eq!(
        preview
            .remove_people
            .iter()
            .map(|row| row.id)
            .collect::<Vec<_>>(),
        vec![managed_person]
    );
    assert_eq!(
        editor::load(&pool, ids.org_id, ids.user_id, user)
            .await
            .unwrap(),
        loaded
    );
    assert_eq!(
        sqlx::query_scalar!(
            "SELECT count(*) FROM permission_change_receipts WHERE org_id=$1",
            ids.org_id
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        Some(0)
    );
    assert!(matches!(
        execute(&pool, ids.org_id, ids.user_id, &command).await,
        Err(ProfileCommandError::Confirmation)
    ));
    command.remove_projects = vec![project];
    command.remove_people = vec![managed_person];
    let saved = execute(&pool, ids.org_id, ids.user_id, &command)
        .await
        .unwrap();
    let reloaded = editor::load(&pool, ids.org_id, ids.user_id, user)
        .await
        .unwrap();
    assert_eq!(reloaded.permissions, preview.after);
    assert_eq!(reloaded.access_revision, saved.access_revision);
    assert_eq!(
        relationship_ids(&pool, ids.org_id).await,
        (vec![], vec![incoming])
    );
    assert!(matches!(
        editor::preview(&pool, ids.org_id, ids.user_id, &draft(&command)).await,
        Err(ProfileCommandError::Stale)
    ));
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn editor_noop_and_last_admin_proposals_are_truthful(pool: PgPool) {
    let ids = fixture(&pool).await;
    let mut unchanged = apply(ids.user_id, BuiltInProfile::Administrator);
    unchanged.action = ProfileAction::Edit;
    let preview = editor::preview(&pool, ids.org_id, ids.user_id, &draft(&unchanged))
        .await
        .unwrap();
    assert!(!preview.changed);
    assert_eq!(preview.before, preview.after);
    let demote = apply(ids.user_id, BuiltInProfile::Member);
    assert!(matches!(
        editor::preview(&pool, ids.org_id, ids.user_id, &draft(&demote)).await,
        Err(ProfileCommandError::LastAdministrator)
    ));
    assert_eq!(state(&pool, ids.org_id, ids.user_id).await.revision, 0);
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn every_builtin_preview_matches_its_actual_save(pool: PgPool) {
    let ids = fixture(&pool).await;
    let user = person(&pool, ids.org_id, false, BuiltInProfile::Member).await;
    for profile in BuiltInProfile::ALL {
        let current = editor::load(&pool, ids.org_id, ids.user_id, user)
            .await
            .unwrap();
        let mut command = apply(user, *profile);
        command.expected_access_revision = current.access_revision;
        command.expected_person_revision = current.permissions.revision;
        let preview = editor::preview(&pool, ids.org_id, ids.user_id, &draft(&command))
            .await
            .unwrap();
        execute(&pool, ids.org_id, ids.user_id, &command)
            .await
            .unwrap();
        let saved = editor::load(&pool, ids.org_id, ids.user_id, user)
            .await
            .unwrap();
        assert_eq!(saved.permissions, preview.after);
        assert_eq!(
            saved.permissions.is_administrator,
            *profile == BuiltInProfile::Administrator
        );
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn editor_denies_wrong_authority_policy_and_foreign_targets(pool: PgPool) {
    let ids = fixture(&pool).await;
    let foreign = fixture(&pool).await;
    let user = person(&pool, ids.org_id, false, BuiltInProfile::Administrator).await;
    assert!(matches!(
        editor::load(&pool, ids.org_id, user, ids.user_id).await,
        Err(ProfileCommandError::Forbidden)
    ));
    for target in [foreign.user_id, Uuid::now_v7()] {
        assert!(matches!(
            editor::load(&pool, ids.org_id, ids.user_id, target).await,
            Err(ProfileCommandError::NotFound)
        ));
    }
    for version in [0, 2] {
        sqlx::query!(
            "UPDATE organizations SET permission_policy_version=$2 WHERE id=$1",
            ids.org_id,
            version
        )
        .execute(&pool)
        .await
        .unwrap();
        assert!(matches!(
            editor::load(&pool, ids.org_id, ids.user_id, user).await,
            Err(ProfileCommandError::Forbidden)
        ));
        assert!(matches!(
            editor::preview(
                &pool,
                ids.org_id,
                ids.user_id,
                &draft(&apply(user, BuiltInProfile::Member))
            )
            .await,
            Err(ProfileCommandError::Forbidden)
        ));
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn editor_direct_actor_deactivation_wins_preview_and_load(pool: PgPool) {
    let ids = fixture(&pool).await;
    let mut revoke = pool.begin().await.unwrap();
    sqlx::query!("UPDATE users SET active=false WHERE id=$1", ids.user_id)
        .execute(&mut *revoke)
        .await
        .unwrap();
    let holder = sqlx::query_scalar!("SELECT pg_backend_pid()")
        .fetch_one(&mut *revoke)
        .await
        .unwrap()
        .unwrap();
    let connection = pool.clone();
    let pending = tokio::spawn(async move {
        editor::preview(
            &connection,
            ids.org_id,
            ids.user_id,
            &draft(&apply(ids.user_id, BuiltInProfile::Member)),
        )
        .await
    });
    wait_for_blocked(&pool, holder).await;
    revoke.commit().await.unwrap();
    assert!(matches!(
        pending.await.unwrap(),
        Err(ProfileCommandError::Forbidden)
    ));
    assert!(matches!(
        editor::load(&pool, ids.org_id, ids.user_id, ids.user_id).await,
        Err(ProfileCommandError::Forbidden)
    ));
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn template_deletion_preview_preserves_people_and_fences_its_affected_set(pool: PgPool) {
    use crate::models::permission_editor::ProfileSource;
    use crate::server_fns::permissions::templates::{self, TemplateAction, TemplateCommand};
    let ids = fixture(&pool).await;
    let target = person(&pool, ids.org_id, false, BuiltInProfile::Member).await;
    let created = templates::execute(
        &pool,
        ids.org_id,
        ids.user_id,
        &TemplateCommand {
            request_id: Uuid::now_v7(),
            expected_access_revision: 0,
            action: TemplateAction::Create {
                name: "Team".into(),
                grants: BuiltInProfile::Member.selection().iter().collect(),
            },
        },
    )
    .await
    .unwrap();
    let mut command = apply(target, BuiltInProfile::Member);
    command.expected_access_revision = 1;
    command.action = ProfileAction::Template {
        id: created.template_id,
        expected_revision: 0,
    };
    command.grants = BuiltInProfile::PeopleAdmin.selection().iter().collect();
    execute(&pool, ids.org_id, ids.user_id, &command)
        .await
        .unwrap();
    let before = editor::load(&pool, ids.org_id, ids.user_id, target)
        .await
        .unwrap();
    let effects = editor::preview_template_deletion(
        &pool,
        ids.org_id,
        ids.user_id,
        created.template_id,
        2,
        0,
    )
    .await
    .unwrap();
    assert_eq!(effects.people.len(), 1);
    assert_eq!(effects.people[0].user_id, target);
    assert_eq!(effects.people[0].permissions, before.permissions);
    assert_eq!(
        editor::load(&pool, ids.org_id, ids.user_id, target)
            .await
            .unwrap(),
        before
    );
    assert_eq!(
        sqlx::query_scalar!(
            "SELECT count(*) FROM permission_change_receipts WHERE org_id=$1",
            ids.org_id
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        Some(2)
    );
    let mut deletion = TemplateCommand {
        request_id: Uuid::now_v7(),
        expected_access_revision: 2,
        action: TemplateAction::Delete {
            id: created.template_id,
            expected_revision: 0,
        },
    };
    let mut change = apply(target, BuiltInProfile::Accounting);
    change.expected_access_revision = 2;
    change.expected_person_revision = 1;
    execute(&pool, ids.org_id, ids.user_id, &change)
        .await
        .unwrap();
    assert!(matches!(
        templates::execute(&pool, ids.org_id, ids.user_id, &deletion).await,
        Err(templates::TemplateCommandError::Stale)
    ));
    // Reapply the template with individual grants to exercise actual detachment.
    command.request_id = Uuid::now_v7();
    command.expected_access_revision = 3;
    command.expected_person_revision = 2;
    execute(&pool, ids.org_id, ids.user_id, &command)
        .await
        .unwrap();
    let current = editor::preview_template_deletion(
        &pool,
        ids.org_id,
        ids.user_id,
        created.template_id,
        4,
        0,
    )
    .await
    .unwrap();
    deletion.expected_access_revision = 4;
    let result = templates::execute(&pool, ids.org_id, ids.user_id, &deletion)
        .await
        .unwrap();
    assert_eq!(result.detached_people, current.people.len());
    let after = editor::load(&pool, ids.org_id, ids.user_id, target)
        .await
        .unwrap();
    assert_eq!(
        after.permissions.grants,
        current.people[0].permissions.grants
    );
    assert_eq!(
        after.permissions.is_administrator,
        current.people[0].permissions.is_administrator
    );
    assert_eq!(after.permissions.source, ProfileSource::Individual);
    assert_eq!(
        after.permissions.revision,
        current.people[0].permissions.revision + 1
    );
    assert!(after.templates.is_empty());
}
