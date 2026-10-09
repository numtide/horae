//! Actual session-bound delegation, not a second SQL implementation of the command.

use super::*;
use horae_core::permissions::catalog::{Permission, PermissionSelection};

async fn grants(pool: &PgPool, org: Uuid, person: Uuid, selection: &[Permission]) {
    let grants: Vec<String> =
        serde_json::from_value(json!(PermissionSelection::new(selection))).unwrap();
    sqlx::query!("INSERT INTO person_permission_states (id,org_id,user_id,catalog_version,grants,is_administrator,source)
        VALUES ($1,$2,$3,1,$4,false,'individual') ON CONFLICT (org_id,user_id) DO UPDATE SET grants=EXCLUDED.grants",
        Uuid::now_v7(),org,person,&grants).execute(pool).await.unwrap();
}

pub(super) async fn check(pool: &PgPool, api: &Api) {
    let ids = crate::server_fns::test_seed::seed(pool, OrgRole::Member).await;
    let foreign = crate::server_fns::test_seed::seed(pool, OrgRole::Admin).await;
    let target = user(pool, ids.org_id, OrgRole::Member).await;
    let cookie = api.cookie(ids.user_id).await;
    let requester = json!({"org_id":ids.org_id,"user_id":ids.user_id});
    let lookup = json!({"project_id":ids.project_id,"expected_requester":requester});
    let command = json!({"kind":"replace_project_managers","request_id":Uuid::now_v7(),
        "expected_access_revision":0,"project_id":ids.project_id,"manager_ids":[ids.user_id,target]});
    let save = json!({"command":command,"expected_requester":requester});
    for (name, body) in [
        ("load_project_managers", lookup.clone()),
        ("save_project_managers", save.clone()),
    ] {
        let anonymous = api.call(name, body.clone(), None, false).await;
        assert_eq!(
            anonymous.status(),
            StatusCode::UNAUTHORIZED,
            "{name}: {}",
            anonymous.text().await.unwrap()
        );
        assert_eq!(
            api.call(name, body, Some(&cookie), false).await.status(),
            StatusCode::FORBIDDEN
        );
    }
    grants(
        pool,
        ids.org_id,
        ids.user_id,
        &[Permission::ProjectWriteAll],
    )
    .await;
    grants(pool, ids.org_id, target, &[Permission::ProjectReadManaged]).await;
    sqlx::query!(
        "UPDATE organizations SET permission_policy_version=1 WHERE id=$1",
        ids.org_id
    )
    .execute(pool)
    .await
    .unwrap();
    assert_eq!(
        api.json("load_project_managers", lookup.clone(), &cookie)
            .await,
        json!({
            "requester":requester,"project_id":ids.project_id,"access_revision":0,"managers":[]
        })
    );
    for forged_requester in [
        json!({"org_id":foreign.org_id,"user_id":ids.user_id}),
        json!({"org_id":ids.org_id,"user_id":target}),
    ] {
        for (name, mut body) in [
            ("load_project_managers", lookup.clone()),
            ("save_project_managers", save.clone()),
        ] {
            body["expected_requester"] = forged_requester.clone();
            assert_eq!(
                api.call(name, body, Some(&cookie), false).await.status(),
                StatusCode::FORBIDDEN
            );
        }
    }
    let other_cookie = api.cookie(target).await;
    grants(pool, ids.org_id, target, &[Permission::ProjectWriteAll]).await;
    assert_eq!(
        api.call(
            "save_project_managers",
            save.clone(),
            Some(&other_cookie),
            false
        )
        .await
        .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        sqlx::query_scalar!(
            "SELECT count(*) FROM permission_change_receipts WHERE org_id=$1",
            ids.org_id
        )
        .fetch_one(pool)
        .await
        .unwrap(),
        Some(0)
    );
    for project in [foreign.project_id, Uuid::now_v7()] {
        let mut body = lookup.clone();
        body["project_id"] = json!(project);
        assert_eq!(
            api.call("load_project_managers", body, Some(&cookie), false)
                .await
                .status(),
            StatusCode::NOT_FOUND
        );
    }
    let outcome = api
        .json("save_project_managers", save.clone(), &cookie)
        .await;
    assert_eq!(
        outcome,
        json!({"project_id":ids.project_id,"access_revision":1,"changed":true})
    );
    assert_eq!(
        api.json("save_project_managers", save.clone(), &cookie)
            .await,
        outcome
    );
    let mut changed = save.clone();
    changed["command"]["manager_ids"] = json!([]);
    assert_eq!(
        api.call("save_project_managers", changed, Some(&cookie), false)
            .await
            .status(),
        StatusCode::CONFLICT
    );
    for (managers, status, revision) in [
        (
            json!([ids.user_id, target, target]),
            StatusCode::BAD_REQUEST,
            1,
        ),
        (
            json!([ids.user_id, target, foreign.user_id]),
            StatusCode::BAD_REQUEST,
            1,
        ),
        (json!([]), StatusCode::CONFLICT, 0),
    ] {
        let mut body = save.clone();
        body["command"]["request_id"] = json!(Uuid::now_v7());
        body["command"]["manager_ids"] = managers;
        body["command"]["expected_access_revision"] = json!(revision);
        assert_eq!(
            api.call("save_project_managers", body, Some(&cookie), false)
                .await
                .status(),
            status
        );
    }
    sqlx::query!("UPDATE users SET active=false, oidc_subject='private-subject',cost_rate_cents=12345,billable_rate_cents=54321 WHERE id=$1",target).execute(pool).await.unwrap();
    sqlx::query!(
        "DELETE FROM person_permission_states WHERE user_id=$1",
        target
    )
    .execute(pool)
    .await
    .unwrap();
    assert_eq!(
        api.json("load_project_managers", lookup.clone(), &cookie)
            .await,
        json!({
            "requester":requester,"project_id":ids.project_id,"access_revision":1,
            "managers":[{"id":ids.user_id,"name":"Test User","active":true},{"id":target,"name":"Test User","active":false}]
        })
    );
    let mut keep = save.clone();
    keep["command"]["request_id"] = json!(Uuid::now_v7());
    keep["command"]["expected_access_revision"] = json!(1);
    assert_eq!(
        api.json("save_project_managers", keep, &cookie).await,
        json!({"project_id":ids.project_id,"access_revision":1,"changed":false})
    );
    let request_id: Uuid = serde_json::from_value(command["request_id"].clone()).unwrap();
    let receipt_id=sqlx::query_scalar!("SELECT id FROM permission_change_receipts WHERE org_id=$1 AND actor_user_id=$2 AND request_id=$3",ids.org_id,ids.user_id,request_id).fetch_one(pool).await.unwrap();
    for (name, body) in [
        ("get_permission_audit", json!({"receipt_id":receipt_id})),
        (
            "list_permission_audit",
            json!({"after":null,"expected_requester":requester}),
        ),
    ] {
        assert_eq!(
            api.call(name, body, Some(&cookie), false).await.status(),
            StatusCode::FORBIDDEN
        );
    }
    grants(
        pool,
        ids.org_id,
        ids.user_id,
        &[Permission::ProjectWriteManaged, Permission::ProjectReadAll],
    )
    .await;
    let mut remove = save.clone();
    remove["command"]["request_id"] = json!(Uuid::now_v7());
    remove["command"]["expected_access_revision"] = json!(1);
    remove["command"]["manager_ids"] = json!([target]);
    assert_eq!(
        api.json("save_project_managers", remove.clone(), &cookie)
            .await,
        json!({"project_id":ids.project_id,"access_revision":2,"changed":true})
    );
    for (name, body) in [
        ("load_project_managers", lookup.clone()),
        ("save_project_managers", remove),
    ] {
        assert_eq!(
            api.call(name, body, Some(&cookie), false).await.status(),
            StatusCode::FORBIDDEN
        );
    }
    sqlx::query!("UPDATE person_permission_states SET grants=ARRAY['private_invalid_grant'] WHERE user_id=$1",ids.user_id).execute(pool).await.unwrap();
    for (name, body) in [
        ("load_project_managers", lookup.clone()),
        ("save_project_managers", save.clone()),
    ] {
        let response = api.call(name, body, Some(&cookie), false).await;
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let message = response.text().await.unwrap();
        assert!(message.contains("Project managers are unavailable"));
        assert!(!message.contains("private_invalid_grant"));
        assert!(!message.contains("person_permission_states"));
    }
    sqlx::query!("UPDATE users SET active=false WHERE id=$1", ids.user_id)
        .execute(pool)
        .await
        .unwrap();
    for (name, body) in [
        ("load_project_managers", lookup),
        ("save_project_managers", save),
    ] {
        assert_eq!(
            api.call(name, body, Some(&cookie), false).await.status(),
            StatusCode::UNAUTHORIZED
        );
    }
    assert_eq!(
        sqlx::query_scalar!(
            "SELECT count(*) FROM permission_change_receipts WHERE org_id=$1",
            ids.org_id
        )
        .fetch_one(pool)
        .await
        .unwrap(),
        Some(3)
    );
}
