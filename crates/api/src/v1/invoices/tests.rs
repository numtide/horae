//! `/invoices`, `/invoices/{id}` and `/invoices/{id}/pdf`: every JSON answer
//! checked against the document, the web app's refusals for a member, the
//! filters and the cursor walk, the PDF, and OAuth tokens refused.

use axum::Router;
use axum::http::{StatusCode, header};
use chrono::{SecondsFormat, TimeDelta};
use horae_core::permissions::catalog::BuiltInProfile;
use horae_core::permissions::oauth::{ClientKind, Scope, ScopeSet};
use horae_core::types::EntryState;
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

use crate::ops::oauth::testing::{REDIRECT_URI, all_scopes, approve, exchange, pending, register};
use crate::testing::{SeedIds, add_person, seed, time_entry};
use crate::v1::testing::{get, router, send, token};

/// An invoice of the seeded client with one line of billed time: 90 minutes
/// at 80.00, less a 10% discount, plus 20% tax.
async fn invoice(pool: &PgPool, ids: &SeedIds, number: &str, status: &str) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO invoices (id, org_id, client_id, number, status, issued_on, due_on, currency,
                               total_cents, discount_bps, discount_cents, tax1_bps, tax1_cents,
                               subject, po_number)
         VALUES ($1, $2, $3, $4, $5::text::invoice_status, '2026-09-01', '2026-10-01', 'EUR',
                 12960, 1000, 1200, 2000, 2160, 'September', 'PO-7')",
        id,
        ids.org_id,
        ids.client_id,
        number,
        status,
    )
    .execute(pool)
    .await
    .unwrap();
    let entry = time_entry(pool, ids, EntryState::Open).await;
    sqlx::query!(
        "INSERT INTO invoice_line_items (id, invoice_id, time_entry_id, description, minutes,
                                         rate_cents, amount_cents)
         VALUES ($1, $2, $3, 'Design review', 90, 8000, 12000)",
        Uuid::now_v7(),
        id,
        entry,
    )
    .execute(pool)
    .await
    .unwrap();
    id
}

async fn read(router: &Router, operation: &str, uri: &str, secret: &str) -> (StatusCode, Value) {
    let (status, _, body) = send(router, operation, get(uri, Some(secret))).await;
    (status, body)
}

fn numbers(page: &Value) -> Vec<&str> {
    page["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|invoice| invoice["number"].as_str().unwrap())
        .collect()
}

/// The PDF's status, content type, disposition and bytes.
async fn pdf(router: &Router, id: Uuid, secret: &str) -> (StatusCode, String, String, Vec<u8>) {
    let response = router
        .clone()
        .oneshot(get(&format!("/api/v1/invoices/{id}/pdf"), Some(secret)))
        .await
        .unwrap();
    let headers = response.headers();
    let text = |name: header::HeaderName| {
        headers
            .get(name)
            .map(|value| value.to_str().unwrap().to_owned())
            .unwrap_or_default()
    };
    let (status, content_type, disposition) = (
        response.status(),
        text(header::CONTENT_TYPE),
        text(header::CONTENT_DISPOSITION),
    );
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (status, content_type, disposition, bytes.to_vec())
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn an_administrator_reads_invoices_as_documented(pool: PgPool) {
    let admin = seed(&pool, BuiltInProfile::Administrator).await;
    let id = invoice(&pool, &admin, "2026-001", "sent").await;
    let (secret, _) = token(&pool, &admin, "books").await;
    let router = router(&pool, 100);

    let (status, page) = read(&router, "listInvoices", "/api/v1/invoices", &secret).await;
    assert_eq!(status, StatusCode::OK, "{page}");
    assert_eq!(page["next_cursor"], Value::Null);
    let listed = &page["data"][0];
    assert_eq!(listed["id"], json!(id));
    assert_eq!(listed["number"], "2026-001");
    assert_eq!(listed["status"], "sent");
    assert_eq!(listed["source"], "horae");
    assert_eq!(listed["client_id"], json!(admin.client_id));
    assert_eq!(listed["client_name"], "Acme");
    assert_eq!(listed["subject"], "September");
    assert_eq!(listed["issued_on"], "2026-09-01");
    assert_eq!(listed["due_on"], "2026-10-01");
    assert_eq!(listed["currency"], "EUR");
    assert_eq!(
        listed["subtotal"],
        json!({ "amount_minor": 12000, "currency": "EUR" })
    );
    assert_eq!(listed["discount"]["amount_minor"], 1200);
    assert_eq!(listed["tax"]["amount_minor"], 2160);
    assert_eq!(listed["second_tax"]["amount_minor"], 0);
    assert_eq!(listed["second_tax_name"], Value::Null);
    assert_eq!(listed["total"]["amount_minor"], 12960);
    assert_eq!(listed["po_number"], "PO-7");

    let (status, detail) = read(
        &router,
        "getInvoice",
        &format!("/api/v1/invoices/{id}"),
        &secret,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{detail}");
    assert_eq!(detail["number"], "2026-001");
    assert_eq!(detail["updated_at"], listed["updated_at"]);
    assert_eq!(
        detail["lines"],
        json!([{
            "description": "Design review",
            "time_entry_id": detail["lines"][0]["time_entry_id"].as_str().unwrap(),
            "minutes": 90,
            "hourly_rate": { "amount_minor": 8000, "currency": "EUR" },
            "amount": { "amount_minor": 12000, "currency": "EUR" },
        }])
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn the_pdf_is_the_web_apps_and_the_same_every_time(pool: PgPool) {
    let admin = seed(&pool, BuiltInProfile::Administrator).await;
    let id = invoice(&pool, &admin, "2026-002", "sent").await;
    let (secret, _) = token(&pool, &admin, "books").await;
    let router = router(&pool, 100);

    let (status, content_type, disposition, first) = pdf(&router, id, &secret).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(content_type, "application/pdf");
    assert_eq!(
        disposition,
        r#"attachment; filename="invoice-2026-002.pdf""#
    );
    assert!(first.starts_with(b"%PDF"));
    let (_, _, _, second) = pdf(&router, id, &secret).await;
    assert_eq!(first, second, "an unchanged invoice renders the same bytes");
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_member_is_refused_as_in_the_web_app(pool: PgPool) {
    let admin = seed(&pool, BuiltInProfile::Administrator).await;
    let member = add_person(&pool, &admin, BuiltInProfile::Member).await;
    let id = invoice(&pool, &admin, "2026-003", "sent").await;
    let (secret, _) = token(&pool, &member, "script").await;
    let router = router(&pool, 100);

    for (operation, uri) in [
        ("listInvoices", "/api/v1/invoices".to_owned()),
        ("getInvoice", format!("/api/v1/invoices/{id}")),
        ("getInvoicePdf", format!("/api/v1/invoices/{id}/pdf")),
    ] {
        let (status, body) = read(&router, operation, &uri, &secret).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{operation}: {body}");
        assert_eq!(body["code"], "forbidden");
        assert_eq!(body["detail"], "Current access is required");
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn an_invoice_of_another_organization_is_not_found(pool: PgPool) {
    let admin = seed(&pool, BuiltInProfile::Administrator).await;
    let other = seed(&pool, BuiltInProfile::Administrator).await;
    let theirs = invoice(&pool, &other, "2026-004", "sent").await;
    let (secret, _) = token(&pool, &admin, "books").await;
    let router = router(&pool, 100);

    for (operation, uri) in [
        ("getInvoice", format!("/api/v1/invoices/{theirs}")),
        ("getInvoicePdf", format!("/api/v1/invoices/{theirs}/pdf")),
    ] {
        let (status, body) = read(&router, operation, &uri, &secret).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{operation}: {body}");
        assert_eq!(body["code"], "not_found");
    }
    let (_, page) = read(&router, "listInvoices", "/api/v1/invoices", &secret).await;
    assert!(numbers(&page).is_empty(), "{page}");
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn filters_narrow_the_list_and_bind_its_cursor(pool: PgPool) {
    let admin = seed(&pool, BuiltInProfile::Administrator).await;
    invoice(&pool, &admin, "D-1", "draft").await;
    invoice(&pool, &admin, "S-1", "sent").await;
    let paid = invoice(&pool, &admin, "P-1", "paid").await;
    invoice(&pool, &admin, "S-2", "sent").await;
    let (secret, _) = token(&pool, &admin, "books").await;
    let router = router(&pool, 100);

    let (_, sent) = read(
        &router,
        "listInvoices",
        "/api/v1/invoices?status=sent",
        &secret,
    )
    .await;
    assert_eq!(numbers(&sent), ["S-1", "S-2"]);

    let (status, body) = read(
        &router,
        "listInvoices",
        "/api/v1/invoices?status=issued",
        &secret,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");

    // Walk everything one at a time.
    let mut walked = Vec::new();
    let mut uri = "/api/v1/invoices?limit=1".to_owned();
    loop {
        let (status, page) = read(&router, "listInvoices", &uri, &secret).await;
        assert_eq!(status, StatusCode::OK, "{page}");
        walked.extend(numbers(&page).into_iter().map(str::to_owned));
        let Some(cursor) = page["next_cursor"].as_str() else {
            break;
        };
        uri = format!("/api/v1/invoices?limit=1&cursor={cursor}");
    }
    assert_eq!(walked, ["D-1", "S-1", "P-1", "S-2"]);

    // A cursor belongs to its filters.
    let (_, first) = read(&router, "listInvoices", "/api/v1/invoices?limit=1", &secret).await;
    let cursor = first["next_cursor"].as_str().unwrap();
    let (status, body) = read(
        &router,
        "listInvoices",
        &format!("/api/v1/invoices?limit=1&status=sent&cursor={cursor}"),
        &secret,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert_eq!(body["code"], "invalid_cursor");

    // Paying an invoice brings it into the next incremental run.
    let before =
        sqlx::query_scalar!(r#"SELECT clock_timestamp() AS "now!: chrono::DateTime<chrono::Utc>""#)
            .fetch_one(&pool)
            .await
            .unwrap();
    sqlx::query!("UPDATE invoices SET status = 'paid' WHERE number = 'S-2'")
        .execute(&pool)
        .await
        .unwrap();
    let since = (before - TimeDelta::microseconds(1)).to_rfc3339_opts(SecondsFormat::Micros, true);
    let (_, changed) = read(
        &router,
        "listInvoices",
        &format!("/api/v1/invoices?updated_since={since}"),
        &secret,
    )
    .await;
    assert_eq!(numbers(&changed), ["S-2"]);
    let (_, of_client) = read(
        &router,
        "listInvoices",
        &format!("/api/v1/invoices?client_id={}&status=paid", admin.client_id),
        &secret,
    )
    .await;
    assert_eq!(numbers(&of_client), ["P-1", "S-2"]);
    assert_eq!(of_client["data"][0]["id"], json!(paid));
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn oauth_tokens_are_refused_whatever_their_scopes(pool: PgPool) {
    let admin = seed(&pool, BuiltInProfile::Administrator).await;
    let id = invoice(&pool, &admin, "2026-005", "sent").await;
    let app = register(&pool, &admin, "Widget", ClientKind::Public, all_scopes()).await;
    let scopes = ScopeSet::of(&Scope::ALL).normalize();
    let (_, code) = approve(&pool, &admin, &pending(&app, REDIRECT_URI, scopes)).await;
    let tokens = exchange(&pool, &app, &code, REDIRECT_URI).await.unwrap();
    let secret = tokens.access_token.expose().to_owned();
    let router = router(&pool, 100);

    for (operation, uri) in [
        ("listInvoices", "/api/v1/invoices".to_owned()),
        ("getInvoice", format!("/api/v1/invoices/{id}")),
        ("getInvoicePdf", format!("/api/v1/invoices/{id}/pdf")),
    ] {
        let (status, headers, body) = send(&router, operation, get(&uri, Some(&secret))).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{operation}: {body}");
        assert_eq!(body["code"], "insufficient_scope");
        assert_eq!(
            headers[header::WWW_AUTHENTICATE],
            r#"Bearer error="insufficient_scope""#
        );
    }
}
