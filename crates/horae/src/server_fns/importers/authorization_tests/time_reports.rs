//! Session-bound report payloads, including populated private source fields.

use super::*;
use horae_core::permissions::catalog::BuiltInProfile;
use horae_core::types::EntryState;
use std::io::{Cursor, Read};

mod export_filters;

async fn xlsx(api: &Api, cookie: Option<&str>, filter: &str) -> reqwest::Response {
    download(api, cookie, "xlsx", filter).await
}

async fn download(
    api: &Api,
    cookie: Option<&str>,
    format: &str,
    filter: &str,
) -> reqwest::Response {
    let mut request = api.client.get(format!(
        "{}/api/reports/export/{format}?from=2026-09-01&to=2026-09-30{filter}",
        api.base,
    ));
    if let Some(cookie) = cookie {
        request = request.header("cookie", cookie);
    }
    request.send().await.unwrap()
}

pub(super) async fn check(pool: &PgPool, api: &Api) {
    let ids = crate::server_fns::test_seed::seed(pool, OrgRole::Member).await;
    let other = crate::server_fns::test_seed::seed(pool, OrgRole::Admin).await;
    let entry = crate::server_fns::test_seed::time_entry(pool, &ids, EntryState::Open).await;
    sqlx::query!("UPDATE users SET oidc_subject=id::text,billable_rate_cents=123456,cost_rate_cents=987654 WHERE id=$1", ids.user_id).execute(pool).await.unwrap();
    let cookie = api.cookie(ids.user_id).await;
    assert_eq!(xlsx(api, None, "").await.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(
        download(api, None, "csv", "").await.status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        download(api, Some(&cookie), "csv", "").await.status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        xlsx(api, Some(&cookie), "").await.status(),
        StatusCode::FORBIDDEN
    );
    let request = json!({"query": {
        "date_from":"2026-09-01", "date_to":"2026-09-30",
        "client_ids":[], "project_ids":[], "user_ids":[], "task_ids":[], "tag_ids":[],
        "after":null, "expected_requester":null
    }});
    const ENDPOINT: &str = "list_visible_time_report_entries";
    assert_eq!(
        api.call(ENDPOINT, request.clone(), None, false)
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        api.call(ENDPOINT, request.clone(), Some(&cookie), false)
            .await
            .status(),
        StatusCode::FORBIDDEN
    );
    sqlx::query!(
        "UPDATE organizations SET permission_policy_version=1 WHERE id=$1",
        ids.org_id
    )
    .execute(pool)
    .await
    .unwrap();
    assert_eq!(
        xlsx(api, Some(&cookie), "").await.status(),
        StatusCode::INTERNAL_SERVER_ERROR
    );
    assert_eq!(
        download(api, Some(&cookie), "csv", "").await.status(),
        StatusCode::INTERNAL_SERVER_ERROR
    );
    assert_eq!(
        api.call(ENDPOINT, request.clone(), Some(&cookie), false)
            .await
            .status(),
        StatusCode::INTERNAL_SERVER_ERROR
    );
    let grants: Vec<String> =
        serde_json::from_value(json!(BuiltInProfile::Member.selection())).unwrap();
    sqlx::query!("INSERT INTO person_permission_states (id,org_id,user_id,catalog_version,grants,is_administrator,source)
        VALUES ($1,$2,$3,1,$4,false,'individual')", Uuid::now_v7(), ids.org_id, ids.user_id, &grants).execute(pool).await.unwrap();
    let expected = json!({
        "requester":{"org_id":ids.org_id,"user_id":ids.user_id},
        "entries":[{"id":entry,"spent_date":"2026-09-07","project_name":"Widget",
            "task_name":"Dev","user_name":"Test User","minutes":60,"rounded_minutes":60,"billable":true,"notes":null}],
        "next_after":null,
        "totals":{"entry_count":1,"total_minutes":60,"rounded_minutes":60,"billable_minutes":60}
    });
    assert_eq!(api.json(ENDPOINT, request.clone(), &cookie).await, expected);
    let response = download(
        api,
        Some(&cookie),
        "csv",
        &format!("&org_id={}&actor_id={}", other.org_id, other.user_id),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers()["content-disposition"],
        "attachment; filename=\"timesheet.csv\""
    );
    let data = response.text().await.unwrap();
    let mut csv = csv::Reader::from_reader(data.as_bytes());
    assert_eq!(csv.headers().unwrap().len(), 8);
    let records = csv.records().collect::<Result<Vec<_>, _>>().unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(&records[0][1], "Widget");
    assert_eq!(&records[0][3], "Test User");
    for private in [
        "123456",
        "987654",
        "billable_rate",
        "cost_rate",
        "EUR",
        "time_read_own",
        "catalog_version",
    ] {
        assert!(!data.contains(private), "{private}");
    }
    let response = download(
        api,
        Some(&cookie),
        "csv",
        &format!("&project_id={}", other.project_id),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        csv::Reader::from_reader(response.bytes().await.unwrap().as_ref())
            .records()
            .count(),
        0
    );
    let response = xlsx(
        api,
        Some(&cookie),
        &format!("&org_id={}&actor_id={}", other.org_id, other.user_id),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers()["content-disposition"],
        "attachment; filename=\"timesheet.xlsx\""
    );
    let mut archive = zip::ZipArchive::new(Cursor::new(response.bytes().await.unwrap())).unwrap();
    let mut sheet = String::new();
    archive
        .by_name("xl/worksheets/sheet1.xml")
        .unwrap()
        .read_to_string(&mut sheet)
        .unwrap();
    assert_eq!(sheet.matches("<row ").count(), 2);
    let mut strings = String::new();
    archive
        .by_name("xl/sharedStrings.xml")
        .unwrap()
        .read_to_string(&mut strings)
        .unwrap();
    assert!(strings.contains("Widget") && strings.contains("Test User"));
    for private in ["123456", "987654", "billable_rate", "cost_rate", "EUR"] {
        assert!(
            !strings.contains(private) && !sheet.contains(private),
            "{private}"
        );
    }
    let response = xlsx(
        api,
        Some(&cookie),
        &format!("&project_id={}", other.project_id),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let mut archive = zip::ZipArchive::new(Cursor::new(response.bytes().await.unwrap())).unwrap();
    let mut sheet = String::new();
    archive
        .by_name("xl/worksheets/sheet1.xml")
        .unwrap()
        .read_to_string(&mut sheet)
        .unwrap();
    assert_eq!(sheet.matches("<row ").count(), 1);
    let mut forged = request.clone();
    forged["org_id"] = json!(other.org_id);
    forged["user_id"] = json!(other.user_id);
    assert_eq!(api.json(ENDPOINT, forged, &cookie).await, expected);
    let mut filtered = request.clone();
    filtered["query"]["user_ids"] = json!([other.user_id]);
    let empty = api.json(ENDPOINT, filtered, &cookie).await;
    assert_eq!(empty["entries"], json!([]));
    assert_eq!(
        empty["totals"],
        json!({"entry_count":0,"total_minutes":0,"rounded_minutes":0,"billable_minutes":0})
    );

    for _ in 0..500 {
        crate::server_fns::test_seed::time_entry(pool, &ids, EntryState::Open).await;
    }
    let first = api.json(ENDPOINT, request.clone(), &cookie).await;
    let totals = json!({"entry_count":501,"total_minutes":30060,"rounded_minutes":30060,"billable_minutes":30060});
    assert_eq!(first["entries"].as_array().unwrap().len(), 500);
    assert_eq!(first["totals"], totals);
    assert!(!first["next_after"].is_null());
    let mut next = request.clone();
    next["query"]["after"] = first["next_after"].clone();
    let last = api.json(ENDPOINT, next, &cookie).await;
    assert_eq!(last["entries"].as_array().unwrap().len(), 1);
    assert_eq!(last["totals"], totals);
    assert!(last["next_after"].is_null());
    let mut invalid = request.clone();
    invalid["query"]["date_to"] = json!("2026-08-01");
    assert_eq!(
        api.call(ENDPOINT, invalid, Some(&cookie), false)
            .await
            .status(),
        StatusCode::BAD_REQUEST
    );
    let mut changed = request.clone();
    changed["query"]["expected_requester"] = json!({"org_id":ids.org_id,"user_id":other.user_id});
    assert_eq!(
        api.call(ENDPOINT, changed, Some(&cookie), false)
            .await
            .status(),
        StatusCode::FORBIDDEN
    );
    sqlx::query!(
        "UPDATE person_permission_states SET catalog_version=99 WHERE user_id=$1",
        ids.user_id
    )
    .execute(pool)
    .await
    .unwrap();
    assert_eq!(
        api.call(ENDPOINT, request.clone(), Some(&cookie), false)
            .await
            .status(),
        StatusCode::INTERNAL_SERVER_ERROR
    );
    sqlx::query!("UPDATE users SET active=false WHERE id=$1", ids.user_id)
        .execute(pool)
        .await
        .unwrap();
    assert_eq!(
        xlsx(api, Some(&cookie), "").await.status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        download(api, Some(&cookie), "csv", "").await.status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        api.call(ENDPOINT, request, Some(&cookie), false)
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
    export_filters::check(pool, api).await;
}
