use super::*;
use crate::server_fns::test_seed::seed;
use serial_test::serial;
use sqlx::PgPool;
use uuid::Uuid;

#[sqlx::test(migrations = "./migrations")]
#[serial]
async fn project_details_only_offer_task_rate_currency_to_current_managers(pool: PgPool) {
    for project_type in [
        ProjectType::TimeAndMaterials,
        ProjectType::FixedFee,
        ProjectType::NonBillable,
    ] {
        for mode in [None, Some("task"), Some("person"), Some("project")] {
            let ids = seed(&pool, OrgRole::Admin).await;
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
            for role in [OrgRole::Admin, OrgRole::Manager, OrgRole::Member] {
                sqlx::query!(
                    "UPDATE users SET org_role = $2 WHERE id = $1",
                    ids.user_id,
                    role as OrgRole
                )
                .execute(&pool)
                .await
                .unwrap();
                let details = fetch_project_details(&pool, ids.org_id, ids.user_id, ids.project_id)
                    .await
                    .unwrap()
                    .unwrap();
                let json = serde_json::to_value(details).unwrap();
                let expected = if role != OrgRole::Member
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
                    "{role:?}, {project_type:?}, {mode:?}"
                );
                if expected.is_none() {
                    assert!(json.get("task_rate_currency").is_none());
                }
            }
        }
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial]
async fn project_details_and_tags_follow_current_progress_and_private_permissions(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Admin).await;
    let other = seed(&pool, OrgRole::Admin).await;
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

    let details = fetch_project_details(&pool, ids.org_id, ids.user_id, ids.project_id)
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
        fetch_project_details(&pool, ids.org_id, ids.user_id, other.project_id)
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

    sqlx::query!(
        "UPDATE users SET org_role = 'manager' WHERE id = $1",
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let details = fetch_project_details(&pool, ids.org_id, ids.user_id, ids.project_id)
        .await
        .unwrap()
        .unwrap();
    assert!(
        serde_json::to_value(details)
            .unwrap()
            .get("admin_notes")
            .is_none()
    );
    sqlx::query!(
        "UPDATE users SET org_role = 'member' WHERE id = $1",
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(
        fetch_project_details(&pool, ids.org_id, ids.user_id, ids.project_id)
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
    let details = fetch_project_details(&pool, ids.org_id, ids.user_id, ids.project_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(details.tags, ["Launch"]);
    assert!(details.admin_notes.is_none());
    sqlx::query!("UPDATE users SET active = false WHERE id = $1", ids.user_id)
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        fetch_project_details(&pool, ids.org_id, ids.user_id, ids.project_id)
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
}

#[sqlx::test(migrations = "./migrations")]
#[serial]
async fn unlinked_and_private_project_tags_do_not_leak_into_filters(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Member).await;
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
        fetch_project_details(&pool, ids.org_id, ids.user_id, ids.project_id)
            .await
            .unwrap()
            .is_none()
    );
    sqlx::query!(
        "UPDATE assignments SET role = 'lead' WHERE project_id = $1 AND user_id = $2",
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
