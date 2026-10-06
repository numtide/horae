use super::*;
use crate::models::project_creation::{ProjectForm, ProjectMemberInput, ReportVisibility};

const PATH: &str = "/api/projects/export/xlsx";

async fn strings(response: reqwest::Response) -> String {
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers()["content-disposition"],
        "attachment; filename=\"projects.xlsx\""
    );
    assert_eq!(
        response.headers()["content-type"],
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
    );
    let bytes = response.bytes().await.unwrap();
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut result = String::new();
    archive
        .by_name("xl/sharedStrings.xml")
        .unwrap()
        .read_to_string(&mut result)
        .unwrap();
    result
}

fn copy_api(api: &Api) -> Api {
    Api {
        base: api.base.clone(),
        client: api.client.clone(),
    }
}

async fn export_after_writer(
    pool: &PgPool,
    api: &Api,
    client_id: Uuid,
    cookies: (&str, &str),
    operation: &'static str,
    request: Value,
) -> (Value, String) {
    let mut blocker = pool.begin().await.unwrap();
    sqlx::query!("SELECT id FROM clients WHERE id=$1 FOR UPDATE", client_id)
        .fetch_one(&mut *blocker)
        .await
        .unwrap();
    let pid = sqlx::query_scalar!("SELECT pg_backend_pid() as \"pid!\"")
        .fetch_one(&mut *blocker)
        .await
        .unwrap();
    let writer = {
        let api = copy_api(api);
        let cookie = cookies.0.to_owned();
        tokio::spawn(async move { api.json(operation, request, &cookie).await })
    };
    wait_for_blocked(pool, pid).await;
    let writer_pid = sqlx::query_scalar!(
        r#"SELECT pid AS "pid!" FROM pg_stat_activity
           WHERE datname=current_database() AND $1=ANY(pg_blocking_pids(pid)) LIMIT 1"#,
        pid,
    )
    .fetch_one(pool)
    .await
    .unwrap();
    let reader = {
        let api = copy_api(api);
        let cookie = cookies.1.to_owned();
        tokio::spawn(async move { strings(download(&api, PATH, Some(&cookie)).await).await })
    };
    wait_for_blocked(pool, writer_pid).await;
    blocker.commit().await.unwrap();
    (writer.await.unwrap(), reader.await.unwrap())
}

pub(super) async fn check(pool: &PgPool, api: &Api) {
    let ids = seed(pool, OrgRole::Manager).await;
    let member = user(pool, ids.org_id, OrgRole::Member).await;
    let manager_cookie = api.cookie(ids.user_id).await;
    let member_cookie = api.cookie(member).await;
    denied(download(api, PATH, None).await, StatusCode::UNAUTHORIZED).await;
    assert!(
        !strings(download(api, PATH, Some(&member_cookie)).await)
            .await
            .contains("Widget")
    );

    let draft_id = Uuid::now_v7();
    let form = ProjectForm {
        client_id: Some(ids.client_id),
        name: "Newly assigned export project".to_owned(),
        report_visibility: ReportVisibility::ProjectMembers,
        team: vec![ProjectMemberInput {
            user_id: member,
            manager: false,
            billable_rate: "100.00".to_owned(),
            cost_rate: String::new(),
            budget: String::new(),
        }],
        ..ProjectForm::default()
    };
    let saved = api
        .json(
            "save_project_draft",
            json!({
                "draft_id":draft_id,"expected_revision":0,"form":form
            }),
            &manager_cookie,
        )
        .await;
    let revision = sqlx::query_scalar!(
        "SELECT access_revision FROM organizations WHERE id=$1",
        ids.org_id
    )
    .fetch_one(pool)
    .await
    .unwrap();
    let (project_id, exported) = export_after_writer(
        pool,
        api,
        ids.client_id,
        (&manager_cookie, &member_cookie),
        "finalize_project_draft",
        json!({
            "draft_id":draft_id,"expected_revision":saved["revision"],"form":form
        }),
    )
    .await;
    assert!(exported.contains(&form.name));
    assert!(!exported.contains("Widget"));
    assert_eq!(
        sqlx::query_scalar!(
            "SELECT access_revision FROM organizations WHERE id=$1",
            ids.org_id
        )
        .fetch_one(pool)
        .await
        .unwrap(),
        revision
    );

    let mut editor = api
        .json(
            "load_project_editor",
            json!({"project_id":project_id}),
            &manager_cookie,
        )
        .await;
    editor["form"]["report_visibility"] = json!("managers");
    api.json(
        "save_project_editor",
        json!({"request":{
            "id":Uuid::now_v7(),"project_id":project_id,
            "expected_revision":editor["revision"],"form":editor["form"]
        }}),
        &manager_cookie,
    )
    .await;
    assert!(
        !strings(download(api, PATH, Some(&member_cookie)).await)
            .await
            .contains(&form.name)
    );

    let mut editor = api
        .json(
            "load_project_editor",
            json!({"project_id":project_id}),
            &manager_cookie,
        )
        .await;
    editor["form"]["report_visibility"] = json!("project_members");
    let (_, exported) = export_after_writer(
        pool,
        api,
        ids.client_id,
        (&manager_cookie, &member_cookie),
        "save_project_editor",
        json!({"request":{
            "id":Uuid::now_v7(),"project_id":project_id,
            "expected_revision":editor["revision"],"form":editor["form"]
        }}),
    )
    .await;
    assert!(exported.contains(&form.name));

    let foreign = seed(pool, OrgRole::Admin).await;
    let forged = format!(
        "{PATH}?org_id={}&viewer_id={}&actor_id={}",
        foreign.org_id, foreign.user_id, foreign.user_id
    );
    let exported = strings(download(api, &forged, Some(&member_cookie)).await).await;
    assert!(exported.contains(&form.name));
    assert!(!exported.contains("Widget"));
    let own_project: Uuid = serde_json::from_value(project_id).unwrap();
    sqlx::query!(
        "UPDATE projects SET name='Other tenant secret' WHERE id=$1",
        foreign.project_id
    )
    .execute(pool)
    .await
    .unwrap();
    assert!(
        !strings(download(api, PATH, Some(&member_cookie)).await)
            .await
            .contains("Other tenant secret")
    );
    sqlx::query!(
        "DELETE FROM assignments WHERE user_id=$1 AND project_id=$2",
        member,
        own_project
    )
    .execute(pool)
    .await
    .unwrap();
    assert!(
        !strings(download(api, PATH, Some(&member_cookie)).await)
            .await
            .contains(&form.name)
    );
    sqlx::query!("UPDATE users SET active=false WHERE id=$1", member)
        .execute(pool)
        .await
        .unwrap();
    denied(
        download(api, PATH, Some(&member_cookie)).await,
        StatusCode::UNAUTHORIZED,
    )
    .await;
}
