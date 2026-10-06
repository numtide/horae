use super::canonical_fields::{canonical_actor, keep_protected};
use super::*;
use crate::models::project_managers::{ProjectManagersCommand, ProjectManagersCommandKind};
use crate::server_fns::permissions::project_management;
use crate::server_fns::test_seed::SeedIds;
use horae_core::permissions::catalog::{Permission, PermissionSelection};

async fn fixture(pool: &PgPool) -> (SeedIds, EditableProject) {
    let (owner, project) = configured_fixture(pool).await;
    canonical_actor(
        pool,
        &owner,
        &PermissionSelection::new(&[Permission::ProjectWriteAll]),
    )
    .await;
    sqlx::query!(
        "UPDATE assignments SET role = 'freelancer' WHERE project_id = $1",
        project.id
    )
    .execute(pool)
    .await
    .unwrap();
    let editor = load_editable_project(pool, owner.user_id, owner.org_id, project.id)
        .await
        .unwrap();
    (owner, editor)
}

fn request(editor: EditableProject) -> ProjectEditRequest {
    let mut request = edit_request(editor);
    request.unchanged = keep_protected(&request.form);
    request
}

async fn managers(pool: &PgPool, owner: &SeedIds, project: Uuid) -> Vec<Uuid> {
    project_management::read(pool, owner.org_id, owner.user_id, project)
        .await
        .unwrap()
        .managers
        .into_iter()
        .map(|manager| manager.id)
        .collect()
}

async fn target(pool: &PgPool, owner: &SeedIds) -> Uuid {
    let user = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO users (id, org_id, email, name) VALUES ($1, $2, $3, 'Manager')",
        user,
        owner.org_id,
        format!("{user}@test.com")
    )
    .execute(pool)
    .await
    .unwrap();
    canonical_actor(
        pool,
        &SeedIds {
            user_id: user,
            ..*owner
        },
        &PermissionSelection::new(&[Permission::ProjectReadManaged]),
    )
    .await;
    user
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn canonical_form_manager_only_change_and_historical_replay_do_not_need_team_changes(
    pool: PgPool,
) {
    let (owner, editor) = fixture(&pool).await;
    let outside = target(&pool, &owner).await;
    let original_revision = editor.revision;
    let mut request = request(editor);
    request.managers.as_mut().unwrap().manager_ids = vec![outside];
    assert!(
        save_editable_project(&pool, owner.user_id, owner.org_id, &request, false)
            .await
            .unwrap()
            .1
    );
    let current = load_editable_project(&pool, owner.user_id, owner.org_id, request.project_id)
        .await
        .unwrap();
    assert_eq!(current.revision, original_revision);
    assert_eq!(current.form.team.len(), 1);
    assert_eq!(
        managers(&pool, &owner, request.project_id).await,
        vec![outside]
    );
    project_management::execute(
        &pool,
        owner.org_id,
        owner.user_id,
        &ProjectManagersCommand {
            kind: ProjectManagersCommandKind::ReplaceProjectManagers,
            request_id: Uuid::now_v7(),
            expected_access_revision: 1,
            project_id: request.project_id,
            manager_ids: vec![],
        },
    )
    .await
    .unwrap();
    assert!(
        !save_editable_project(&pool, owner.user_id, owner.org_id, &request, false)
            .await
            .unwrap()
            .1
    );
    assert!(managers(&pool, &owner, request.project_id).await.is_empty());
    request.id = Uuid::now_v7();
    request.managers.as_mut().unwrap().manager_ids.clear();
    let result = save_editable_project(&pool, owner.user_id, owner.org_id, &request, false).await;
    assert!(
        matches!(
            result,
            Err(ServerFnError::ServerError { code: CONFLICT, .. })
        ),
        "unchanged form must still validate the access revision: {result:?}"
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn canonical_form_retains_archived_manager_with_different_legacy_role(pool: PgPool) {
    let (owner, editor) = fixture(&pool).await;
    let retained = target(&pool, &owner).await;
    sqlx::query!(
        "INSERT INTO assignments (id, project_id, user_id) VALUES ($1, $2, $3)",
        Uuid::now_v7(),
        editor.id,
        retained
    )
    .execute(&pool)
    .await
    .unwrap();
    project_management::execute(
        &pool,
        owner.org_id,
        owner.user_id,
        &ProjectManagersCommand {
            kind: ProjectManagersCommandKind::ReplaceProjectManagers,
            request_id: Uuid::now_v7(),
            expected_access_revision: 0,
            project_id: editor.id,
            manager_ids: vec![retained],
        },
    )
    .await
    .unwrap();
    sqlx::query!("UPDATE users SET active = false WHERE id = $1", retained)
        .execute(&pool)
        .await
        .unwrap();
    let editor = load_editable_project(&pool, owner.user_id, owner.org_id, editor.id)
        .await
        .unwrap();
    assert!(
        editor
            .form
            .team
            .iter()
            .find(|member| member.user_id == retained)
            .unwrap()
            .manager
    );
    let mut request = request(editor);
    request.form.name = "Keep archived manager".into();
    assert!(
        save_editable_project(&pool, owner.user_id, owner.org_id, &request, false)
            .await
            .unwrap()
            .1
    );
    assert_eq!(
        managers(&pool, &owner, request.project_id).await,
        vec![retained]
    );
    let current = load_editable_project(&pool, owner.user_id, owner.org_id, request.project_id)
        .await
        .unwrap();
    request = edit_request(current);
    request.unchanged = keep_protected(&request.form);
    request.managers.as_mut().unwrap().manager_ids.clear();
    request
        .form
        .team
        .iter_mut()
        .find(|member| member.user_id == retained)
        .unwrap()
        .manager = false;
    assert!(
        save_editable_project(&pool, owner.user_id, owner.org_id, &request, false)
            .await
            .unwrap()
            .1
    );
    assert!(managers(&pool, &owner, request.project_id).await.is_empty());
    let current = load_editable_project(&pool, owner.user_id, owner.org_id, request.project_id)
        .await
        .unwrap();
    assert!(
        current
            .form
            .team
            .iter()
            .any(|member| member.user_id == retained)
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn canonical_form_self_removal_commits_but_cannot_replay_without_authority(pool: PgPool) {
    let (owner, editor) = fixture(&pool).await;
    project_management::execute(
        &pool,
        owner.org_id,
        owner.user_id,
        &ProjectManagersCommand {
            kind: ProjectManagersCommandKind::ReplaceProjectManagers,
            request_id: Uuid::now_v7(),
            expected_access_revision: 0,
            project_id: editor.id,
            manager_ids: vec![owner.user_id],
        },
    )
    .await
    .unwrap();
    let grants: Vec<String> = serde_json::from_value(
        serde_json::to_value(PermissionSelection::new(&[Permission::ProjectWriteManaged])).unwrap(),
    )
    .unwrap();
    sqlx::query!(
        "UPDATE person_permission_states SET grants = $2 WHERE user_id = $1",
        owner.user_id,
        &grants
    )
    .execute(&pool)
    .await
    .unwrap();
    let editor = load_editable_project(&pool, owner.user_id, owner.org_id, editor.id)
        .await
        .unwrap();
    let mut request = request(editor);
    request.form.name = "Last authorized edit".into();
    request.form.team[0].manager = false;
    request.managers.as_mut().unwrap().manager_ids.clear();
    assert!(
        save_editable_project(&pool, owner.user_id, owner.org_id, &request, false)
            .await
            .unwrap()
            .1
    );
    let result = save_editable_project(&pool, owner.user_id, owner.org_id, &request, false).await;
    assert!(
        matches!(
            result,
            Err(ServerFnError::ServerError {
                code: FORBIDDEN,
                ..
            })
        ),
        "{result:?}"
    );
    assert!(
        load_editable_project(&pool, owner.user_id, owner.org_id, request.project_id)
            .await
            .is_err()
    );
    assert_eq!(
        sqlx::query_scalar!(
            "SELECT name FROM projects WHERE id = $1",
            request.project_id
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        request.form.name
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn canonical_form_saves_managers_without_changing_legacy_membership_roles(pool: PgPool) {
    let (owner, editor) = fixture(&pool).await;
    let mut request = request(editor);
    request.form.name = "Delegation and project edit".into();
    request.form.team[0].manager = true;
    request.managers.as_mut().unwrap().manager_ids = vec![owner.user_id];
    assert!(
        save_editable_project(&pool, owner.user_id, owner.org_id, &request, false)
            .await
            .unwrap()
            .1
    );
    assert_eq!(
        managers(&pool, &owner, request.project_id).await,
        vec![owner.user_id]
    );
    let role = sqlx::query_scalar!(
        r#"SELECT role as "role: ProjectRole" FROM assignments WHERE project_id=$1 AND user_id=$2"#,
        request.project_id,
        owner.user_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(role, ProjectRole::Freelancer);
    assert!(
        !save_editable_project(&pool, owner.user_id, owner.org_id, &request, false)
            .await
            .unwrap()
            .1
    );
    let editor = load_editable_project(&pool, owner.user_id, owner.org_id, request.project_id)
        .await
        .unwrap();
    assert!(editor.form.team[0].manager);
    assert_eq!(editor.form.name, request.form.name);
    assert_eq!(editor.access.unwrap().managers.access_revision, 1);
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn canonical_form_rejects_invalid_or_stale_manager_selection_atomically(pool: PgPool) {
    let (owner, editor) = fixture(&pool).await;
    let original = editor.clone();
    let mut request = request(editor);
    request.form.name = "Must not be saved".into();
    let valid = request.managers.clone().unwrap();
    let mut stale = valid.clone();
    stale.expected_access_revision += 1;
    let mut missing_person = valid.clone();
    missing_person.manager_ids.push(Uuid::now_v7());
    let mut duplicate = valid.clone();
    duplicate.manager_ids = vec![Uuid::nil(), Uuid::nil()];
    for invalid in [None, Some(stale), Some(missing_person), Some(duplicate)] {
        request.managers = invalid;
        assert!(
            save_editable_project(&pool, owner.user_id, owner.org_id, &request, false)
                .await
                .is_err()
        );
        let current = load_editable_project(&pool, owner.user_id, owner.org_id, original.id)
            .await
            .unwrap();
        assert_eq!(current, original);
    }
    request.managers = Some(valid);
    request.form.team[0].manager = true;
    assert!(
        save_editable_project(&pool, owner.user_id, owner.org_id, &request, false)
            .await
            .is_err(),
        "checkboxes cannot disagree with the complete selection"
    );
    request.form.team[0].manager = false;
    assert!(
        save_editable_project(&pool, owner.user_id, owner.org_id, &request, false)
            .await
            .unwrap()
            .1
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn canonical_form_cannot_reuse_an_independent_delegation_receipt(pool: PgPool) {
    let (owner, editor) = fixture(&pool).await;
    let mut request = request(editor);
    request.form.name = "Not authorized by a delegation receipt".into();
    project_management::execute(
        &pool,
        owner.org_id,
        owner.user_id,
        &ProjectManagersCommand {
            kind: ProjectManagersCommandKind::ReplaceProjectManagers,
            request_id: request.id,
            expected_access_revision: 0,
            project_id: request.project_id,
            manager_ids: vec![],
        },
    )
    .await
    .unwrap();
    let result = save_editable_project(&pool, owner.user_id, owner.org_id, &request, false).await;
    assert!(
        matches!(
            result,
            Err(ServerFnError::ServerError { code: CONFLICT, .. })
        ),
        "{result:?}"
    );
    let editor = load_editable_project(&pool, owner.user_id, owner.org_id, request.project_id)
        .await
        .unwrap();
    assert_ne!(editor.form.name, request.form.name);
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn canonical_form_receipt_failure_rolls_back_delegation_and_project(pool: PgPool) {
    let (owner, editor) = fixture(&pool).await;
    let original = editor.clone();
    let mut request = request(editor);
    request.form.name = "Atomic receipt failure".into();
    request.form.team[0].manager = true;
    request
        .managers
        .as_mut()
        .unwrap()
        .manager_ids
        .push(owner.user_id);
    sqlx::query!("ALTER TABLE project_edit_requests ADD CONSTRAINT reject_form_receipt CHECK (false) NOT VALID")
        .execute(&pool).await.unwrap();
    assert!(
        save_editable_project(&pool, owner.user_id, owner.org_id, &request, false)
            .await
            .is_err()
    );
    let current = load_editable_project(&pool, owner.user_id, owner.org_id, original.id)
        .await
        .unwrap();
    assert_eq!(current, original);
    assert_eq!(
        sqlx::query_scalar!(
            "SELECT count(*) FROM permission_change_receipts WHERE request_id = $1",
            request.id
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        Some(0)
    );
    sqlx::query!("ALTER TABLE project_edit_requests DROP CONSTRAINT reject_form_receipt")
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        save_editable_project(&pool, owner.user_id, owner.org_id, &request, false)
            .await
            .unwrap()
            .1
    );
}
