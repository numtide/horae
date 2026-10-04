//! Real session-route editor lifecycle, including denied direct requests.

use super::*;
use horae_core::permissions::catalog::BuiltInProfile;

pub(super) async fn check(pool: &PgPool, api: &Api) {
    let ids = crate::server_fns::test_seed::seed(pool, OrgRole::Member).await;
    let foreign = crate::server_fns::test_seed::seed(pool, OrgRole::Admin).await;
    let target = super::user(pool, ids.org_id, OrgRole::Member).await;
    let cookie = api.cookie(ids.user_id).await;
    let member_cookie = api.cookie(target).await;
    let lookup = json!({"user_id":target});
    assert_eq!(
        api.call("load_permission_editor", lookup.clone(), None, false)
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        api.call(
            "load_permission_editor",
            lookup.clone(),
            Some(&cookie),
            false
        )
        .await
        .status(),
        StatusCode::FORBIDDEN
    );
    for (user, admin) in [(ids.user_id, true), (target, false)] {
        let grants: Vec<String> =
            serde_json::from_value(json!(BuiltInProfile::Administrator.selection())).unwrap();
        sqlx::query!("INSERT INTO person_permission_states (id,org_id,user_id,catalog_version,grants,is_administrator,source)
            VALUES ($1,$2,$3,1,$4,$5,'individual')", Uuid::now_v7(), ids.org_id, user, &grants, admin)
            .execute(pool).await.unwrap();
    }
    sqlx::query!(
        "UPDATE organizations SET permission_policy_version=1 WHERE id=$1",
        ids.org_id
    )
    .execute(pool)
    .await
    .unwrap();
    let loaded = api
        .json("load_permission_editor", lookup.clone(), &cookie)
        .await;
    check_authentication_failure(pool, api, target, &cookie).await;
    assert_eq!(loaded["user_id"], json!(target));
    assert_eq!(loaded["permissions"]["is_administrator"], false);
    assert_eq!(loaded["templates"], json!([]));
    assert_eq!(
        api.call(
            "load_permission_editor",
            lookup.clone(),
            Some(&member_cookie),
            false
        )
        .await
        .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        api.call(
            "load_permission_editor",
            json!({"user_id":foreign.user_id}),
            Some(&cookie),
            false
        )
        .await
        .status(),
        StatusCode::NOT_FOUND
    );

    let project_link = Uuid::now_v7();
    let person_link = Uuid::now_v7();
    sqlx::query!("INSERT INTO project_management_assignments (id, org_id, manager_id, project_id) VALUES ($1, $2, $3, $4)",
        project_link, ids.org_id, target, ids.project_id).execute(pool).await.unwrap();
    sqlx::query!("INSERT INTO person_management_assignments (id, org_id, manager_id, managed_user_id) VALUES ($1, $2, $3, $4)",
        person_link, ids.org_id, target, ids.user_id).execute(pool).await.unwrap();
    let draft = json!({"user_id":target,"expected_access_revision":0,"expected_person_revision":0,
        "action":{"kind":"built_in","profile":"member"},"grants":BuiltInProfile::Member.selection()});
    let preview = api
        .json(
            "preview_person_permissions",
            json!({"draft":draft}),
            &cookie,
        )
        .await;
    assert_eq!(preview["changed"], true);
    assert_eq!(
        preview["remove_projects"],
        json!([{
            "id":project_link,"subject_id":ids.project_id,"revision":0,"name":"Widget"
        }])
    );
    assert_eq!(
        preview["remove_people"],
        json!([{
            "id":person_link,"subject_id":ids.user_id,"revision":0,"name":"Test User"
        }])
    );
    assert_eq!(
        api.json("load_permission_editor", lookup.clone(), &cookie)
            .await,
        loaded
    );
    let command = json!({"request_id":Uuid::now_v7(),"expected_access_revision":0,"user_id":target,
        "expected_person_revision":0,"action":{"kind":"built_in","profile":"member"},
        "grants":BuiltInProfile::Member.selection(),"remove_projects":[project_link],"remove_people":[person_link]});
    let saved = api
        .json(
            "save_person_permissions",
            json!({"command":command}),
            &cookie,
        )
        .await;
    assert_eq!(saved["access_revision"], 1);
    assert_eq!(
        api.json(
            "save_person_permissions",
            json!({"command":command}),
            &cookie
        )
        .await,
        saved
    );
    assert_eq!(
        api.json("load_permission_editor", lookup.clone(), &cookie)
            .await["permissions"],
        preview["after"]
    );
    assert_eq!(
        api.call(
            "preview_person_permissions",
            json!({"draft":draft}),
            Some(&cookie),
            false
        )
        .await
        .status(),
        StatusCode::CONFLICT
    );

    let create = json!({"request_id":Uuid::now_v7(),"expected_access_revision":1,
        "action":{"kind":"create","name":" Team ","grants":BuiltInProfile::PeopleAdmin.selection()}});
    let created = api
        .json(
            "save_permission_template",
            json!({"command":create}),
            &cookie,
        )
        .await;
    assert_eq!(created["access_revision"], 2);
    let choices = api
        .json("load_permission_editor", lookup.clone(), &cookie)
        .await;
    assert_eq!(choices["templates"][0]["name"], "Team");
    let template = created["template_id"].clone();
    let apply = json!({"request_id":Uuid::now_v7(),"expected_access_revision":2,"user_id":target,
        "expected_person_revision":1,"action":{"kind":"template","id":template,"expected_revision":0},
        "grants":BuiltInProfile::PeopleAdmin.selection(),"remove_projects":[],"remove_people":[]});
    let applied = api
        .json("save_person_permissions", json!({"command":apply}), &cookie)
        .await;
    let deletion =
        json!({"template_id":template,"expected_access_revision":3,"expected_template_revision":0});
    let effects = api
        .json("preview_permission_template_deletion", deletion, &cookie)
        .await;
    assert_eq!(effects["people"][0]["user_id"], json!(target));
    assert_eq!(effects["people"].as_array().unwrap().len(), 1);
    let delete = json!({"request_id":Uuid::now_v7(),"expected_access_revision":3,
        "action":{"kind":"delete","id":template,"expected_revision":0}});
    let removed = api
        .json(
            "save_permission_template",
            json!({"command":delete}),
            &cookie,
        )
        .await;
    assert_eq!(removed["detached_people"], 1);
    let detached = api
        .json("load_permission_editor", lookup.clone(), &cookie)
        .await;
    assert_eq!(
        detached["permissions"]["source"],
        json!({"kind":"individual"})
    );
    assert_eq!(
        detached["permissions"]["grants"],
        effects["people"][0]["permissions"]["grants"]
    );
    assert_eq!(
        api.json("save_person_permissions", json!({"command":apply}), &cookie)
            .await,
        applied
    );
    assert_eq!(
        api.json(
            "save_permission_template",
            json!({"command":delete}),
            &cookie
        )
        .await,
        removed
    );
    assert_eq!(
        api.json("load_permission_editor", lookup.clone(), &cookie)
            .await,
        detached
    );

    for (name, body) in [
        ("preview_person_permissions", json!({"draft":draft})),
        ("save_person_permissions", json!({"command":command})),
        ("save_permission_template", json!({"command":create})),
        (
            "preview_permission_template_deletion",
            json!({"template_id":template,"expected_access_revision":4,"expected_template_revision":0}),
        ),
    ] {
        assert_eq!(
            api.call(name, body.clone(), None, false).await.status(),
            StatusCode::UNAUTHORIZED
        );
        let mut forged = body;
        forged["actor_id"] = json!(ids.user_id);
        forged["org_id"] = json!(ids.org_id);
        forged["is_administrator"] = json!(true);
        assert_eq!(
            api.call(name, forged, Some(&member_cookie), false)
                .await
                .status(),
            StatusCode::FORBIDDEN
        );
    }
    for version in [0, 2] {
        sqlx::query!(
            "UPDATE organizations SET permission_policy_version=$2 WHERE id=$1",
            ids.org_id,
            version
        )
        .execute(pool)
        .await
        .unwrap();
        for (name, body) in [
            ("load_permission_editor", lookup.clone()),
            ("preview_person_permissions", json!({"draft":draft})),
            ("save_person_permissions", json!({"command":command})),
            ("save_permission_template", json!({"command":create})),
            (
                "preview_permission_template_deletion",
                json!({"template_id":template,"expected_access_revision":4,"expected_template_revision":0}),
            ),
        ] {
            assert_eq!(
                api.call(name, body, Some(&cookie), false).await.status(),
                StatusCode::FORBIDDEN
            );
        }
    }
    sqlx::query!(
        "UPDATE organizations SET permission_policy_version=1 WHERE id=$1",
        ids.org_id
    )
    .execute(pool)
    .await
    .unwrap();
    let mut invalid = command.clone();
    invalid["expected_access_revision"] = json!(4);
    invalid["request_id"] = json!(Uuid::now_v7());
    invalid["grants"] = json!([]);
    assert_eq!(
        api.call(
            "save_person_permissions",
            json!({"command":invalid}),
            Some(&cookie),
            false
        )
        .await
        .status(),
        StatusCode::BAD_REQUEST
    );
    invalid["actor_id"] = json!(ids.user_id);
    assert!(
        !api.call(
            "save_person_permissions",
            json!({"command":invalid}),
            Some(&cookie),
            false
        )
        .await
        .status()
        .is_success()
    );
    let mut invalid_template = create.clone();
    invalid_template["request_id"] = json!(Uuid::now_v7());
    invalid_template["expected_access_revision"] = json!(4);
    invalid_template["action"]["name"] = json!(" ");
    assert_eq!(
        api.call(
            "save_permission_template",
            json!({"command":invalid_template}),
            Some(&cookie),
            false
        )
        .await
        .status(),
        StatusCode::BAD_REQUEST
    );
    sqlx::query!("UPDATE person_permission_states SET grants=ARRAY['private-invalid-grant'] WHERE user_id=$1", ids.user_id).execute(pool).await.unwrap();
    let unavailable = api
        .call("load_permission_editor", lookup, Some(&cookie), false)
        .await;
    assert_eq!(unavailable.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let message = unavailable.text().await.unwrap();
    assert!(message.contains("Permission editor is unavailable"));
    assert!(!message.contains("private-invalid-grant"));
}

async fn check_authentication_failure(pool: &PgPool, api: &Api, target: Uuid, cookie: &str) {
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
        let cancelled = sqlx::query_scalar!("SELECT pg_cancel_backend(pid) FROM pg_stat_activity WHERE datname=current_database() AND $1 = ANY(pg_blocking_pids(pid))", holder)
            .fetch_all(pool).await.unwrap();
        assert_eq!(cancelled, vec![Some(true)]);
        hold.rollback().await.unwrap();
    };
    let (response, ()) = tokio::join!(
        api.call(
            "load_permission_editor",
            json!({"user_id":target}),
            Some(cookie),
            false
        ),
        cancel
    );
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let message = response.text().await.unwrap();
    assert!(message.contains("Permission editor is unavailable"));
    for private in ["SELECT", "users", "canceling statement", "sqlx"] {
        assert!(!message.contains(private), "{message}");
    }
    assert_eq!(
        api.json("load_permission_editor", json!({"user_id":target}), cookie)
            .await["user_id"],
        json!(target)
    );
}
