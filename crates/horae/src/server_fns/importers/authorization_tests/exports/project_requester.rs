use super::*;

pub(super) async fn check(pool: &PgPool, api: &Api) {
    let ids = seed(pool, OrgRole::Manager).await;
    let same_org = user(pool, ids.org_id, OrgRole::Manager).await;
    let foreign = seed(pool, OrgRole::Admin).await;
    let cookie = api.cookie(ids.user_id).await;
    let changed_cookie = api.cookie(same_org).await;
    let foreign_cookie = api.cookie(foreign.user_id).await;
    for format in ["csv", "xlsx"] {
        let path = format!(
            "/api/projects/export/{format}?scope=active&expected_org_id={}&expected_user_id={}",
            ids.org_id, ids.user_id
        );
        denied(download(api, &path, None).await, StatusCode::UNAUTHORIZED).await;
        for changed in [&changed_cookie, &foreign_cookie] {
            denied(
                download(api, &path, Some(changed)).await,
                StatusCode::FORBIDDEN,
            )
            .await;
        }
        for partial in [
            format!("expected_org_id={}", ids.org_id),
            format!("expected_user_id={}", ids.user_id),
            format!("expected_org_id={}&expected_user_id=invalid", ids.org_id),
        ] {
            denied(
                download(
                    api,
                    &format!("/api/projects/export/{format}?{partial}"),
                    Some(&cookie),
                )
                .await,
                StatusCode::BAD_REQUEST,
            )
            .await;
        }
        let response = download(api, &path, Some(&cookie)).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers()["content-disposition"],
            format!("attachment; filename=\"projects.{format}\"")
        );
        let bytes = response.bytes().await.unwrap();
        let contents = if format == "csv" {
            String::from_utf8(bytes.to_vec()).unwrap()
        } else {
            let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
            let mut contents = String::new();
            archive
                .by_name("xl/sharedStrings.xml")
                .unwrap()
                .read_to_string(&mut contents)
                .unwrap();
            contents
        };
        assert!(contents.contains("Widget"), "{contents}");
    }
}
