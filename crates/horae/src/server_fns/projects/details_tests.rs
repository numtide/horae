use super::canonical_read_tests::project_details;
use super::*;
use crate::ops::testing::become_profile;
use crate::server_fns::test_seed::seed;
use horae_core::permissions::catalog::BuiltInProfile;
use serial_test::serial;
use sqlx::PgPool;
use uuid::Uuid;

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial]
async fn project_details_only_offer_task_rate_currency_to_current_managers(pool: PgPool) {
    for project_type in [
        ProjectType::TimeAndMaterials,
        ProjectType::FixedFee,
        ProjectType::NonBillable,
    ] {
        for mode in [None, Some("task"), Some("person"), Some("project")] {
            let ids = seed(&pool, BuiltInProfile::Administrator).await;
            sqlx::query!(
                "UPDATE projects SET project_type = $2 WHERE id = $1",
                ids.project_id,
                project_type as ProjectType
            )
            .execute(&pool)
            .await
            .unwrap();
            sqlx::query!(
                "INSERT INTO assignments (id,project_id,user_id,role) VALUES ($1,$2,$3,'lead')",
                Uuid::now_v7(),
                ids.project_id,
                ids.user_id
            )
            .execute(&pool)
            .await
            .unwrap();
            if let Some(mode) = mode {
                sqlx::query!("INSERT INTO project_settings (id, org_id, project_id, creator_id, rate_mode) VALUES ($1, $2, $3, $4, $5)", Uuid::now_v7(), ids.org_id, ids.project_id, ids.user_id, mode)
                    .execute(&pool).await.unwrap();
            }
            // Task rates need project and billable-rate writes: only the
            // Administrator profile holds both for every project.
            for profile in [
                BuiltInProfile::Administrator,
                BuiltInProfile::ExecutiveManager,
            ] {
                become_profile(&pool, &ids, profile).await;
                let details = project_details(&pool, ids.org_id, ids.user_id, ids.project_id)
                    .await
                    .unwrap()
                    .unwrap();
                let json = serde_json::to_value(details).unwrap();
                let expected = if profile == BuiltInProfile::Administrator
                    && project_type != ProjectType::NonBillable
                    && (mode.is_none()
                        || (project_type == ProjectType::TimeAndMaterials && mode == Some("task")))
                {
                    Some("EUR")
                } else {
                    None
                };
                assert_eq!(
                    json.get("task_rate_currency")
                        .and_then(serde_json::Value::as_str),
                    expected,
                    "{profile:?}, {project_type:?}, {mode:?}"
                );
                if expected.is_none() {
                    assert!(json.get("task_rate_currency").is_none());
                }
            }
        }
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial]
async fn project_details_and_tags_follow_current_progress_and_private_permissions(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Administrator).await;
    let other = seed(&pool, BuiltInProfile::Administrator).await;
    let tag = Uuid::now_v7();
    sqlx::query!("UPDATE projects SET code = 'READ-1', starts_on = '2026-09-01', ends_on = '2026-10-31' WHERE id = $1", ids.project_id).execute(&pool).await.unwrap();
    sqlx::query!("INSERT INTO project_private_settings (id,org_id,project_id,admin_notes) VALUES ($1,$2,$3,'Private launch plan')", Uuid::now_v7(), ids.org_id, ids.project_id).execute(&pool).await.unwrap();
    sqlx::query!(
        "INSERT INTO project_tags (id,org_id,name) VALUES ($1,$2,'Launch')",
        tag,
        ids.org_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO project_tag_links (id,org_id,project_id,tag_id) VALUES ($1,$2,$3,$4)",
        Uuid::now_v7(),
        ids.org_id,
        ids.project_id,
        tag
    )
    .execute(&pool)
    .await
    .unwrap();

    let details = project_details(&pool, ids.org_id, ids.user_id, ids.project_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(details.code.as_deref(), Some("READ-1"));
    assert_eq!(details.starts_on.unwrap().to_string(), "2026-09-01");
    assert_eq!(details.ends_on.unwrap().to_string(), "2026-10-31");
    assert_eq!(details.client_name, "Acme");
    assert_eq!(details.tags, ["Launch"]);
    assert_eq!(details.admin_notes.as_deref(), Some("Private launch plan"));
    let tags = fetch_project_tags(&pool, ids.org_id, ids.user_id)
        .await
        .unwrap();
    assert_eq!(tags.len(), 1);
    assert_eq!(
        (tags[0].project_id, tags[0].tag_id, tags[0].name.as_str()),
        (ids.project_id, tag, "Launch")
    );
    assert!(
        project_details(&pool, ids.org_id, ids.user_id, other.project_id)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        fetch_project_tags(&pool, other.org_id, ids.user_id)
            .await
            .unwrap()
            .is_empty()
    );

    // A Project Manager reads every project but not its private notes.
    become_profile(&pool, &ids, BuiltInProfile::ProjectManager).await;
    let details = project_details(&pool, ids.org_id, ids.user_id, ids.project_id)
        .await
        .unwrap()
        .unwrap();
    assert!(
        serde_json::to_value(details)
            .unwrap()
            .get("admin_notes")
            .is_none()
    );
    become_profile(&pool, &ids, BuiltInProfile::Member).await;
    assert!(
        project_details(&pool, ids.org_id, ids.user_id, ids.project_id)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        fetch_project_tags(&pool, ids.org_id, ids.user_id)
            .await
            .unwrap()
            .is_empty()
    );
    sqlx::query!(
        "INSERT INTO assignments (id,project_id,user_id) VALUES ($1,$2,$3)",
        Uuid::now_v7(),
        ids.project_id,
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let details = project_details(&pool, ids.org_id, ids.user_id, ids.project_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(details.tags, ["Launch"]);
    assert!(details.admin_notes.is_none());
    sqlx::query!("UPDATE users SET active = false WHERE id = $1", ids.user_id)
        .execute(&pool)
        .await
        .unwrap();
    assert!(matches!(
        project_details(&pool, ids.org_id, ids.user_id, ids.project_id).await,
        Err(ServerFnError::ServerError {
            code: FORBIDDEN,
            ..
        })
    ));
    assert!(
        fetch_project_tags(&pool, ids.org_id, ids.user_id)
            .await
            .unwrap()
            .is_empty()
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial]
async fn unlinked_and_private_project_tags_do_not_leak_into_filters(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Member).await;
    let tag = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO project_tags (id,org_id,name) VALUES ($1,$2,'Private tag')",
        tag,
        ids.org_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(
        fetch_project_tags(&pool, ids.org_id, ids.user_id)
            .await
            .unwrap()
            .is_empty()
    );
    sqlx::query!(
        "INSERT INTO assignments (id,project_id,user_id) VALUES ($1,$2,$3)",
        Uuid::now_v7(),
        ids.project_id,
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO project_tag_links (id,org_id,project_id,tag_id) VALUES ($1,$2,$3,$4)",
        Uuid::now_v7(),
        ids.org_id,
        ids.project_id,
        tag
    )
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(
        fetch_project_tags(&pool, ids.org_id, ids.user_id)
            .await
            .unwrap()
            .len(),
        1
    );
    sqlx::query!("INSERT INTO project_settings (id,org_id,project_id,creator_id,rate_mode) VALUES ($1,$2,$3,$4,'person')", Uuid::now_v7(), ids.org_id, ids.project_id, ids.user_id).execute(&pool).await.unwrap();
    assert!(
        fetch_project_tags(&pool, ids.org_id, ids.user_id)
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        project_details(&pool, ids.org_id, ids.user_id, ids.project_id)
            .await
            .unwrap()
            .is_none()
    );
    // A designated manager of the project reads it again.
    use horae_core::permissions::catalog::Permission;
    crate::ops::testing::provision(&pool, &ids, &[Permission::ProjectReadManaged], false).await;
    sqlx::query!(
        "INSERT INTO project_management_assignments (id, org_id, project_id, manager_id)
         VALUES ($1, $2, $3, $4)",
        Uuid::now_v7(),
        ids.org_id,
        ids.project_id,
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(
        fetch_project_tags(&pool, ids.org_id, ids.user_id)
            .await
            .unwrap()
            .len(),
        1
    );
    sqlx::query!(
        "DELETE FROM project_tag_links WHERE project_id = $1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(
        fetch_project_tags(&pool, ids.org_id, ids.user_id)
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        sqlx::query_scalar!("SELECT count(*) FROM project_tags WHERE id = $1", tag)
            .fetch_one(&pool)
            .await
            .unwrap(),
        Some(1)
    );
}
