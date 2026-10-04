//! Exercise the registered permission-history route using real session cookies.

use super::*;
use crate::server_fns::permissions::templates::{self, TemplateAction, TemplateCommand};
use horae_core::permissions::catalog::BuiltInProfile;

pub(super) async fn check(pool: &PgPool, api: &Api) {
    let ids = crate::server_fns::test_seed::seed(pool, OrgRole::Member).await;
    let foreign = crate::server_fns::test_seed::seed(pool, OrgRole::Admin).await;
    let cookie = api.cookie(ids.user_id).await;
    let name = "get_permission_audit";
    let missing = json!({"receipt_id": Uuid::now_v7()});
    assert_eq!(
        api.call(name, missing.clone(), None, false).await.status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        api.call(name, missing.clone(), Some(&cookie), false)
            .await
            .status(),
        StatusCode::FORBIDDEN
    );
    let grants: Vec<String> =
        serde_json::from_value(json!(BuiltInProfile::Administrator.selection())).unwrap();
    for seed in [&ids, &foreign] {
        sqlx::query!("INSERT INTO person_permission_states (id, org_id, user_id, catalog_version, grants, is_administrator, source)
            VALUES ($1,$2,$3,1,$4,true,'individual')", Uuid::now_v7(), seed.org_id, seed.user_id, &grants).execute(pool).await.unwrap();
        sqlx::query!(
            "UPDATE organizations SET permission_policy_version=1 WHERE id=$1",
            seed.org_id
        )
        .execute(pool)
        .await
        .unwrap();
    }
    let command = TemplateCommand {
        request_id: Uuid::now_v7(),
        expected_access_revision: 0,
        action: TemplateAction::Create {
            name: "History readers".into(),
            grants: BuiltInProfile::Member.selection().iter().collect(),
        },
    };
    templates::execute(pool, ids.org_id, ids.user_id, &command)
        .await
        .unwrap();
    let stored = sqlx::query!(
        "SELECT id, audit FROM permission_change_receipts WHERE org_id=$1 AND request_id=$2",
        ids.org_id,
        command.request_id
    )
    .fetch_one(pool)
    .await
    .unwrap();
    let body = json!({"receipt_id": stored.id});
    let historical = api.json(name, body.clone(), &cookie).await;
    assert_eq!(historical["id"], json!(stored.id));
    assert_eq!(
        historical["actor"],
        json!({"kind":"user", "user_id":ids.user_id})
    );
    assert_eq!(
        historical["audit"],
        json!({"kind":"template", "details":stored.audit})
    );
    assert!(historical["created_at"].is_string());
    for private in ["request_id", "intent", "result"] {
        assert!(historical.get(private).is_none());
    }
    check_historical_shapes(pool, api, &ids, &cookie).await;
    check_browsing(pool, api, &ids, &foreign, &cookie).await;
    check_auth_lookup_error(pool, api, stored.id, &cookie).await;
    assert_eq!(api.json(name, missing, &cookie).await, Value::Null);
    let foreign_cookie = api.cookie(foreign.user_id).await;
    assert_eq!(
        api.json(name, body.clone(), &foreign_cookie).await,
        Value::Null
    );
    assert_eq!(api.json(name, json!({"receipt_id":stored.id,"org_id":ids.org_id,"user_id":ids.user_id,"is_administrator":true}), &foreign_cookie).await, Value::Null);
    sqlx::query!(
        "UPDATE person_permission_states SET is_administrator=false WHERE user_id=$1",
        ids.user_id
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query!("UPDATE users SET org_role='admin' WHERE id=$1", ids.user_id)
        .execute(pool)
        .await
        .unwrap();
    // All ordinary grants and claimed identity cannot replace explicit Administrator status.
    let denied = api.call(name, json!({"receipt_id":stored.id,"org_id":foreign.org_id,"user_id":foreign.user_id,"is_administrator":true}), Some(&cookie), false).await;
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);
    assert!(!denied.text().await.unwrap().contains("History readers"));
    sqlx::query!(
        "UPDATE person_permission_states SET is_administrator=true WHERE user_id=$1",
        ids.user_id
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE permission_change_receipts SET format_version=99 WHERE id=$1",
        stored.id
    )
    .execute(pool)
    .await
    .unwrap();
    assert_unavailable(api.call(name, body.clone(), Some(&cookie), false).await).await;
    sqlx::query!(
        "UPDATE permission_change_receipts SET format_version=1, audit='{}'::jsonb WHERE id=$1",
        stored.id
    )
    .execute(pool)
    .await
    .unwrap();
    assert_unavailable(api.call(name, body.clone(), Some(&cookie), false).await).await;
    sqlx::query!(
        "UPDATE person_permission_states SET grants=ARRAY['secret_invalid_grant'] WHERE user_id=$1",
        ids.user_id
    )
    .execute(pool)
    .await
    .unwrap();
    assert_unavailable(api.call(name, body.clone(), Some(&cookie), false).await).await;
    assert_unavailable(
        api.call(
            "list_permission_audit",
            json!({"after":null,"expected_requester":null}),
            Some(&cookie),
            false,
        )
        .await,
    )
    .await;
    sqlx::query!("UPDATE users SET active=false WHERE id=$1", ids.user_id)
        .execute(pool)
        .await
        .unwrap();
    assert_eq!(
        api.call(name, body, Some(&cookie), false).await.status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        api.call(
            "list_permission_audit",
            json!({"after":null,"expected_requester":null}),
            Some(&cookie),
            false
        )
        .await
        .status(),
        StatusCode::UNAUTHORIZED
    );
    api.client
        .post(format!("{}/test/expire", api.base))
        .header("cookie", &foreign_cookie)
        .send()
        .await
        .unwrap();
    assert_eq!(
        api.call(
            name,
            json!({"receipt_id":stored.id}),
            Some(&foreign_cookie),
            false
        )
        .await
        .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        api.call(
            "list_permission_audit",
            json!({"after":null,"expected_requester":null}),
            Some(&foreign_cookie),
            false
        )
        .await
        .status(),
        StatusCode::UNAUTHORIZED
    );
}

async fn check_historical_shapes(
    pool: &PgPool,
    api: &Api,
    ids: &crate::server_fns::test_seed::SeedIds,
    cookie: &str,
) {
    use crate::server_fns::permissions::{profiles, project_management};
    let profile = profiles::ProfileCommand {
        request_id: Uuid::now_v7(),
        expected_access_revision: 1,
        user_id: ids.user_id,
        expected_person_revision: 0,
        action: profiles::ProfileAction::Edit,
        grants: BuiltInProfile::Administrator.selection().iter().collect(),
        remove_projects: vec![],
        remove_people: vec![],
    };
    profiles::execute(pool, ids.org_id, ids.user_id, &profile)
        .await
        .unwrap();
    let project = project_management::ProjectManagersCommand {
        request_id: Uuid::now_v7(),
        expected_access_revision: 1,
        project_id: ids.project_id,
        manager_ids: vec![ids.user_id],
    };
    project_management::execute(pool, ids.org_id, ids.user_id, &project)
        .await
        .unwrap();
    let rows = sqlx::query!(
        "SELECT id, audit FROM permission_change_receipts WHERE org_id=$1 ORDER BY id",
        ids.org_id
    )
    .fetch_all(pool)
    .await
    .unwrap();
    assert_eq!(rows.len(), 3);
    for row in rows {
        let value = api
            .json("get_permission_audit", json!({"receipt_id":row.id}), cookie)
            .await;
        assert_eq!(value["audit"]["details"], row.audit);
        let typed: crate::models::permission_audit::AuditEntry =
            serde_json::from_value(value.clone()).unwrap();
        assert_eq!(serde_json::to_value(typed).unwrap(), value);
    }
    let operator_id = Uuid::now_v7();
    sqlx::query!("INSERT INTO permission_change_receipts (id, org_id, operator_id, operator_command, request_id, format_version, intent, result, audit)
        SELECT $1, org_id, 'operator-invocation', 'permission.change', $2, format_version,
        '{\"private_input\":\"sentinel\"}'::jsonb, '{\"private_result\":\"sentinel\"}'::jsonb, audit
        FROM permission_change_receipts WHERE org_id=$3 AND request_id=$4", operator_id, Uuid::now_v7(), ids.org_id, profile.request_id).execute(pool).await.unwrap();
    let value = api
        .json(
            "get_permission_audit",
            json!({"receipt_id":operator_id}),
            cookie,
        )
        .await;
    assert_eq!(
        value["actor"],
        json!({"kind":"operator", "invocation_id":"operator-invocation", "command":"permission.change"})
    );
    assert!(value["audit"]["details"]["change"].is_null());
    let typed: crate::models::permission_audit::AuditEntry =
        serde_json::from_value(value.clone()).unwrap();
    assert_eq!(serde_json::to_value(typed).unwrap(), value);
    for private in ["private_input", "private_result", "sentinel", "request_id"] {
        assert!(!value.to_string().contains(private));
    }
}

async fn check_browsing(
    pool: &PgPool,
    api: &Api,
    ids: &crate::server_fns::test_seed::SeedIds,
    foreign: &crate::server_fns::test_seed::SeedIds,
    cookie: &str,
) {
    use crate::models::permission_audit::AuditPage;
    // The existing shape checks created template/profile/project-manager and
    // explicit unchanged/operator history. Extend it past one transport page.
    for index in 0..24 {
        let command = TemplateCommand {
            request_id: Uuid::now_v7(),
            expected_access_revision: 2 + index,
            action: TemplateAction::Create {
                name: format!("Browsable history {index}"),
                grants: BuiltInProfile::Member.selection().iter().collect(),
            },
        };
        templates::execute(pool, ids.org_id, ids.user_id, &command)
            .await
            .unwrap();
    }
    sqlx::query!(
        "UPDATE permission_change_receipts SET created_at='2026-09-07 12:00:00+00' WHERE org_id=$1",
        ids.org_id
    )
    .execute(pool)
    .await
    .unwrap();
    let body = json!({"after":null,"expected_requester":null});
    let name = "list_permission_audit";
    assert_eq!(
        api.call(name, body.clone(), None, false).await.status(),
        StatusCode::UNAUTHORIZED
    );
    let first: AuditPage =
        serde_json::from_value(api.json(name, body.clone(), cookie).await).unwrap();
    assert_eq!(first.entries.len(), 25);
    assert_eq!(
        (first.requester.org_id, first.requester.user_id),
        (ids.org_id, ids.user_id)
    );
    let next = json!({"after":first.next_after,"expected_requester":first.requester});
    let second: AuditPage =
        serde_json::from_value(api.json(name, next.clone(), cookie).await).unwrap();
    assert_eq!(second.entries.len(), 3);
    assert!(second.next_after.is_none());
    let stored=sqlx::query_scalar!("SELECT id FROM permission_change_receipts WHERE org_id=$1 ORDER BY created_at DESC,id DESC",ids.org_id).fetch_all(pool).await.unwrap();
    assert_eq!(
        first
            .entries
            .iter()
            .chain(&second.entries)
            .map(|entry| entry.id)
            .collect::<Vec<_>>(),
        stored
    );
    for entry in first.entries.iter().chain(&second.entries) {
        let wire = serde_json::to_value(entry).unwrap();
        assert_eq!(
            api.json(
                "get_permission_audit",
                json!({"receipt_id":entry.id}),
                cookie
            )
            .await,
            wire
        );
        for secret in [
            "private_input",
            "private_result",
            "sentinel",
            "request_id",
            "intent",
        ] {
            assert!(!wire.to_string().contains(secret));
        }
    }
    let foreign_cookie = api.cookie(foreign.user_id).await;
    assert_eq!(
        api.call(name, next.clone(), Some(&foreign_cookie), false)
            .await
            .status(),
        StatusCode::FORBIDDEN
    );
    let forged=api.json(name,json!({"after":first.next_after,"expected_requester":null,"org_id":ids.org_id,"user_id":ids.user_id}),&foreign_cookie).await;
    assert_eq!(forged["entries"], json!([]));
    assert_eq!(forged["requester"]["org_id"], json!(foreign.org_id));
    sqlx::query!(
        "UPDATE person_permission_states SET is_administrator=false WHERE user_id=$1",
        ids.user_id
    )
    .execute(pool)
    .await
    .unwrap();
    let denied = api.call(name, next, Some(cookie), false).await;
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);
    assert!(!denied.text().await.unwrap().contains("Browsable history"));
    sqlx::query!(
        "UPDATE person_permission_states SET is_administrator=true WHERE user_id=$1",
        ids.user_id
    )
    .execute(pool)
    .await
    .unwrap();
    let broken = first.entries[2].id;
    sqlx::query!(
        "UPDATE permission_change_receipts SET format_version=99 WHERE id=$1",
        broken
    )
    .execute(pool)
    .await
    .unwrap();
    assert_unavailable(api.call(name, body.clone(), Some(cookie), false).await).await;
    sqlx::query!(
        "UPDATE permission_change_receipts SET format_version=1 WHERE id=$1",
        broken
    )
    .execute(pool)
    .await
    .unwrap();
    let legacy = crate::server_fns::test_seed::seed(pool, OrgRole::Admin).await;
    assert_eq!(
        api.call(name, body, Some(&api.cookie(legacy.user_id).await), false)
            .await
            .status(),
        StatusCode::FORBIDDEN
    );
}

async fn assert_unavailable(response: reqwest::Response) {
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let text = response.text().await.unwrap();
    assert!(text.contains("Permission history is unavailable"), "{text}");
    for private in [
        "History readers",
        "secret_invalid_grant",
        "permission_change_receipts",
        "person_permission_states",
        "SELECT",
        "sqlx",
        "format_version",
    ] {
        assert!(!text.contains(private), "{text}");
    }
}

async fn check_auth_lookup_error(pool: &PgPool, api: &Api, receipt: Uuid, cookie: &str) {
    for (name, body, expected) in [
        (
            "get_permission_audit",
            json!({"receipt_id":receipt}),
            "Permission history is unavailable",
        ),
        (
            "list_permission_audit",
            json!({"after":null,"expected_requester":null}),
            "Permission history is unavailable",
        ),
        (
            "get_my_permissions",
            json!({}),
            "Permission state is unavailable",
        ),
    ] {
        let mut hold = pool.begin().await.unwrap();
        sqlx::query!("LOCK TABLE users IN ACCESS EXCLUSIVE MODE")
            .execute(&mut *hold)
            .await
            .unwrap();
        let holder = sqlx::query_scalar!("SELECT pg_backend_pid()")
            .fetch_one(&mut *hold)
            .await
            .unwrap()
            .unwrap();
        let cancel = async {
            crate::server_fns::test_seed::wait_for_blocked(pool, holder).await;
            let cancelled = sqlx::query_scalar!("SELECT pg_cancel_backend(pid) FROM pg_stat_activity WHERE datname=current_database() AND $1 = ANY(pg_blocking_pids(pid))", holder).fetch_all(pool).await.unwrap();
            assert_eq!(cancelled, vec![Some(true)]);
            hold.rollback().await.unwrap();
        };
        let (response, ()) = tokio::join!(api.call(name, body, Some(cookie), false), cancel);
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let text = response.text().await.unwrap();
        assert!(text.contains(expected), "{name}: {text}");
        for private in ["canceling statement", "SELECT", "sqlx", "users"] {
            assert!(!text.contains(private), "{name}: {text}");
        }
    }
    assert_eq!(
        api.json(
            "get_permission_audit",
            json!({"receipt_id":receipt}),
            cookie
        )
        .await["id"],
        json!(receipt)
    );
}
