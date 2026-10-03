//! Real session and response checks for the three materialized manager exports.

use super::*;
use crate::server_fns::test_seed::{seed, time_entry, wait_for_blocked};
use horae_core::types::EntryState;
use std::io::{Cursor, Read};

async fn download(api: &Api, path: &str, cookie: Option<&str>) -> reqwest::Response {
    let mut request = api.client.get(format!("{}{path}", api.base));
    if let Some(cookie) = cookie {
        request = request.header("cookie", cookie);
    }
    request.send().await.unwrap()
}

async fn denied(response: reqwest::Response, status: StatusCode) {
    assert_eq!(response.status(), status);
    assert!(response.headers().get("content-disposition").is_none());
    let body = response.text().await.unwrap();
    for private in [
        "EXPORT-PRIVATE",
        "Private export note",
        "Other tenant secret",
    ] {
        assert!(!body.contains(private), "{body}");
    }
}

pub(super) async fn check(pool: &PgPool, api: &Api) {
    let ids = seed(pool, OrgRole::Manager).await;
    let foreign = seed(pool, OrgRole::Admin).await;
    let own_entry = time_entry(pool, &ids, EntryState::Open).await;
    for (entry, note) in [
        (own_entry, "Private export note"),
        (
            time_entry(pool, &foreign, EntryState::Open).await,
            "Other tenant secret",
        ),
    ] {
        sqlx::query!("UPDATE time_entries SET notes=$2 WHERE id=$1", entry, note)
            .execute(pool)
            .await
            .unwrap();
    }
    let invoice_id = Uuid::now_v7();
    sqlx::query!("INSERT INTO invoices (id,org_id,client_id,number,issued_on,due_on,currency,total_cents) VALUES ($1,$2,$3,'EXPORT-PRIVATE','2026-09-07','2026-10-07','EUR',1234)", invoice_id, ids.org_id, ids.client_id).execute(pool).await.unwrap();
    sqlx::query!("INSERT INTO invoice_line_items (id,invoice_id,time_entry_id,description,minutes,rate_cents,amount_cents) VALUES ($1,$2,$3,'Private export note',60,1234,1234)", Uuid::now_v7(), invoice_id, own_entry).execute(pool).await.unwrap();
    let cookie = api.cookie(ids.user_id).await;
    let member = api
        .cookie(user(pool, ids.org_id, OrgRole::Member).await)
        .await;
    let other = api.cookie(foreign.user_id).await;
    let expired = api.cookie(ids.user_id).await;
    assert_eq!(
        api.client
            .post(format!("{}/test/expire", api.base))
            .header("cookie", &expired)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NO_CONTENT
    );
    let paths = [
        "/api/reports/export/xlsx?from=2026-09-07&to=2026-09-07".to_owned(),
        format!("/api/invoices/{invoice_id}/export/xlsx"),
        format!("/api/invoices/{invoice_id}/export/pdf"),
    ];
    for path in &paths {
        denied(download(api, path, None).await, StatusCode::UNAUTHORIZED).await;
        denied(
            download(api, path, Some(&member)).await,
            StatusCode::FORBIDDEN,
        )
        .await;
        denied(
            download(api, path, Some(&expired)).await,
            StatusCode::UNAUTHORIZED,
        )
        .await;
        for role in [OrgRole::Manager, OrgRole::Admin] {
            sqlx::query!(
                "UPDATE users SET org_role=$2 WHERE id=$1",
                ids.user_id,
                role as OrgRole
            )
            .execute(pool)
            .await
            .unwrap();
            let separator = if path.contains('?') { '&' } else { '?' };
            let forged = format!(
                "{path}{separator}org_id={}&actor_id={}&org_role=member",
                foreign.org_id, foreign.user_id
            );
            let response = download(api, &forged, Some(&cookie)).await;
            assert_eq!(response.status(), StatusCode::OK);
            let pdf = path.ends_with("/pdf");
            assert_eq!(
                response.headers()["content-type"],
                if pdf {
                    "application/pdf"
                } else {
                    "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
                }
            );
            assert!(
                response.headers()["content-disposition"]
                    .to_str()
                    .unwrap()
                    .contains(if path.contains("/reports/") {
                        "timesheet.xlsx"
                    } else {
                        "EXPORT-PRIVATE"
                    })
            );
            let bytes = response.bytes().await.unwrap();
            if pdf {
                assert!(bytes.starts_with(b"%PDF-"));
            } else {
                let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
                let mut strings = String::new();
                archive
                    .by_name("xl/sharedStrings.xml")
                    .unwrap()
                    .read_to_string(&mut strings)
                    .unwrap();
                assert!(strings.contains("Private export note"));
                assert!(!strings.contains("Other tenant secret"));
            }
        }
        if path.contains("/invoices/") {
            denied(
                download(api, path, Some(&other)).await,
                StatusCode::NOT_FOUND,
            )
            .await;
            let missing = path.replace(&invoice_id.to_string(), &Uuid::now_v7().to_string());
            denied(
                download(api, &missing, Some(&cookie)).await,
                StatusCode::NOT_FOUND,
            )
            .await;
        } else {
            let filtered = format!("{path}&project_id={}", foreign.project_id);
            let response = download(api, &filtered, Some(&cookie)).await;
            assert_eq!(response.status(), StatusCode::OK);
            let mut archive =
                zip::ZipArchive::new(Cursor::new(response.bytes().await.unwrap())).unwrap();
            let mut sheet = String::new();
            archive
                .by_name("xl/worksheets/sheet1.xml")
                .unwrap()
                .read_to_string(&mut sheet)
                .unwrap();
            assert_eq!(sheet.matches("<row ").count(), 1);
        }
        sqlx::query!("UPDATE users SET active=false WHERE id=$1", ids.user_id)
            .execute(pool)
            .await
            .unwrap();
        denied(
            download(api, path, Some(&cookie)).await,
            StatusCode::UNAUTHORIZED,
        )
        .await;
        sqlx::query!("UPDATE users SET active=true WHERE id=$1", ids.user_id)
            .execute(pool)
            .await
            .unwrap();

        let mut writer = pool.begin().await.unwrap();
        crate::db::lock_organization(
            &mut writer,
            ids.org_id,
            crate::db::OrganizationLock::AccessChange,
        )
        .await
        .unwrap();
        let pid = sqlx::query_scalar!("SELECT pg_backend_pid() as \"pid!\"")
            .fetch_one(&mut *writer)
            .await
            .unwrap();
        let pending = {
            let client = api.client.clone();
            let url = format!("{}{path}", api.base);
            let cookie = cookie.clone();
            tokio::spawn(async move {
                client
                    .get(url)
                    .header("cookie", cookie)
                    .send()
                    .await
                    .unwrap()
            })
        };
        wait_for_blocked(pool, pid).await;
        sqlx::query!(
            "UPDATE users SET org_role='member' WHERE id=$1",
            ids.user_id
        )
        .execute(&mut *writer)
        .await
        .unwrap();
        writer.commit().await.unwrap();
        denied(pending.await.unwrap(), StatusCode::FORBIDDEN).await;
        sqlx::query!(
            "UPDATE users SET org_role='manager' WHERE id=$1",
            ids.user_id
        )
        .execute(pool)
        .await
        .unwrap();
    }
}
