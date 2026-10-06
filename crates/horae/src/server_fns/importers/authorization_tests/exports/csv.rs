use super::*;

pub(super) async fn check(pool: &PgPool, api: &Api) {
    let ids = seed(pool, OrgRole::Member).await;
    let foreign = seed(pool, OrgRole::Admin).await;
    sqlx::query!(
        "UPDATE projects SET name='Other tenant secret' WHERE id=$1",
        foreign.project_id
    )
    .execute(pool)
    .await
    .unwrap();
    let cookie = api.cookie(ids.user_id).await;
    let path = "/api/projects/export/csv";
    denied(download(api, path, None).await, StatusCode::UNAUTHORIZED).await;
    let response = download(api, path, Some(&cookie)).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["content-type"], "text/csv");
    let bytes = response.bytes().await.unwrap();
    assert_eq!(
        ::csv::Reader::from_reader(bytes.as_ref()).records().count(),
        0
    );
    sqlx::query!(
        "INSERT INTO assignments (id,project_id,user_id) VALUES ($1,$2,$3)",
        Uuid::now_v7(),
        ids.project_id,
        ids.user_id
    )
    .execute(pool)
    .await
    .unwrap();
    let forged = format!(
        "{path}?org_id={}&actor_id={}&org_role=admin",
        foreign.org_id, foreign.user_id
    );
    let response = download(api, &forged, Some(&cookie)).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["content-type"], "text/csv");
    let text = response.text().await.unwrap();
    assert!(text.contains("Widget"));
    assert!(!text.contains("Other tenant secret"));
    sqlx::query!(
        "DELETE FROM assignments WHERE project_id=$1 AND user_id=$2",
        ids.project_id,
        ids.user_id
    )
    .execute(pool)
    .await
    .unwrap();
    let response = download(api, path, Some(&cookie)).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["content-type"], "text/csv");
    let bytes = response.bytes().await.unwrap();
    assert_eq!(
        ::csv::Reader::from_reader(bytes.as_ref()).records().count(),
        0
    );
    sqlx::query!("UPDATE users SET active=false WHERE id=$1", ids.user_id)
        .execute(pool)
        .await
        .unwrap();
    denied(
        download(api, path, Some(&cookie)).await,
        StatusCode::UNAUTHORIZED,
    )
    .await;
}
