//! Session-owned legacy readers reauthorize after the initial session lookup.

use super::*;
use crate::server_fns::test_seed::{seed, time_entry, wait_for_blocked};
use horae_core::types::EntryState;

pub(super) async fn check(pool: &PgPool, api: &Api) {
    let ids = seed(pool, OrgRole::Manager).await;
    let foreign = seed(pool, OrgRole::Admin).await;
    time_entry(pool, &ids, EntryState::Open).await;
    let invoice = Uuid::now_v7();
    sqlx::query!("INSERT INTO invoices (id,org_id,client_id,number,issued_on,due_on,currency,total_cents) VALUES ($1,$2,$3,'PRIVATE-HTTP','2026-09-07','2026-10-07','EUR',1200)", invoice, ids.org_id, ids.client_id).execute(pool).await.unwrap();
    let cookie = api.cookie(ids.user_id).await;
    let foreign_cookie = api.cookie(foreign.user_id).await;
    let body = json!({
        "from": "2026-09-07", "to": "2026-09-07", "client_id": null,
        "project_id": null, "user_id": null, "tag_id": null, "status": null,
        "invoice_id": invoice, "actor_id": foreign.user_id, "org_id": foreign.org_id,
        "org_role": "admin"
    });
    for name in ["report_detailed", "list_invoices", "get_invoice"] {
        assert_eq!(
            api.call(name, body.clone(), None, false).await.status(),
            StatusCode::UNAUTHORIZED
        );
        let expected = api.json(name, body.clone(), &cookie).await;
        match name {
            "report_detailed" => {
                assert_eq!(expected.as_array().unwrap().len(), 1);
                assert_eq!(expected[0]["minutes"], 60);
            }
            "list_invoices" => {
                assert_eq!(expected.as_array().unwrap().len(), 1);
                assert_eq!(expected[0]["number"], "PRIVATE-HTTP");
            }
            _ => assert_eq!(expected["invoice"]["number"], "PRIVATE-HTTP"),
        }
        let response = api
            .call(name, body.clone(), Some(&foreign_cookie), false)
            .await;
        if name == "get_invoice" {
            assert_eq!(response.status(), StatusCode::NOT_FOUND);
            assert!(!response.text().await.unwrap().contains("PRIVATE-HTTP"));
        } else {
            assert_eq!(response.status(), StatusCode::OK);
            assert_eq!(response.json::<Value>().await.unwrap(), json!([]));
        }
        for deactivate in [false, true] {
            let mut writer = pool.begin().await.unwrap();
            sqlx::query!(
                "SELECT id FROM organizations WHERE id = $1 FOR NO KEY UPDATE",
                ids.org_id
            )
            .fetch_one(&mut *writer)
            .await
            .unwrap();
            if deactivate {
                sqlx::query!("UPDATE users SET active=false WHERE id=$1", ids.user_id)
                    .execute(&mut *writer)
                    .await
                    .unwrap();
            } else {
                sqlx::query!(
                    "UPDATE users SET org_role='member' WHERE id=$1",
                    ids.user_id
                )
                .execute(&mut *writer)
                .await
                .unwrap();
            }
            let pid = sqlx::query_scalar!("SELECT pg_backend_pid() as \"pid!\"")
                .fetch_one(&mut *writer)
                .await
                .unwrap();
            // The session gate sees the committed Manager. Its materialized
            // reader must wait for and observe this winning revocation.
            let (response, ()) =
                tokio::join!(api.call(name, body.clone(), Some(&cookie), false), async {
                    wait_for_blocked(pool, pid).await;
                    writer.commit().await.unwrap();
                });
            assert_eq!(response.status(), StatusCode::FORBIDDEN, "{name}");
            let error = response.text().await.unwrap();
            for field in ["PRIVATE-HTTP", "total_cents", "project_name", "minutes"] {
                assert!(!error.contains(field), "{name}: {error}");
            }
            assert_eq!(
                api.call(name, body.clone(), Some(&cookie), false)
                    .await
                    .status(),
                if deactivate {
                    StatusCode::UNAUTHORIZED
                } else {
                    StatusCode::FORBIDDEN
                }
            );
            sqlx::query!(
                "UPDATE users SET active=true,org_role='manager' WHERE id=$1",
                ids.user_id
            )
            .execute(pool)
            .await
            .unwrap();
            assert_eq!(api.json(name, body.clone(), &cookie).await, expected);
        }
    }
}
