//! Project editor authority and requester binding through registered session routes.

use super::*;
use crate::models::project_creation::{EditableProject, ProjectEditRequest, ProtectedProjectField};
use horae_core::permissions::catalog::{Permission, PermissionSelection};

pub(super) async fn check(pool: &PgPool, api: &Api) {
    let ids = crate::server_fns::test_seed::seed(pool, OrgRole::Member).await;
    let second = user(pool, ids.org_id, OrgRole::Manager).await;
    let first_cookie = api.cookie(ids.user_id).await;
    let second_cookie = api.cookie(second).await;
    let lookup = json!({"project_id":ids.project_id});
    let legacy = api
        .json("load_project_editor", lookup.clone(), &second_cookie)
        .await;
    assert!(legacy["access"].is_null());
    let legacy_save = json!({"request": {
        "id": Uuid::now_v7(), "project_id": ids.project_id,
        "expected_revision": legacy["revision"], "form": legacy["form"],
    }});
    // Legacy policy still denies Members, even after the public role guard is removed.
    for (endpoint, body) in [
        ("load_project_editor", lookup.clone()),
        ("save_project_editor", legacy_save.clone()),
    ] {
        assert_eq!(
            api.call(endpoint, body, Some(&first_cookie), false)
                .await
                .status(),
            StatusCode::FORBIDDEN
        );
    }
    assert_eq!(
        api.json("save_project_editor", legacy_save, &second_cookie)
            .await,
        json!(ids.project_id)
    );
    let grants: Vec<String> = serde_json::from_value(json!(PermissionSelection::new(&[
        Permission::ProjectWriteAll,
    ])))
    .unwrap();
    for actor in [ids.user_id, second] {
        sqlx::query!("INSERT INTO person_permission_states (id,org_id,user_id,catalog_version,grants,is_administrator,source)
        VALUES ($1,$2,$3,1,$4,false,'individual')", Uuid::now_v7(), ids.org_id, actor, &grants)
            .execute(pool).await.unwrap();
    }
    sqlx::query!(
        "UPDATE organizations SET permission_policy_version=1 WHERE id=$1",
        ids.org_id,
    )
    .execute(pool)
    .await
    .unwrap();
    assert_eq!(
        api.call("load_project_editor", lookup.clone(), None, false)
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
    let initial: EditableProject = serde_json::from_value(
        api.json("load_project_editor", lookup.clone(), &first_cookie)
            .await,
    )
    .unwrap();
    let requester = initial.access.as_ref().unwrap().requester;
    assert_eq!(requester.org_id, ids.org_id);
    assert_eq!(requester.user_id, ids.user_id);
    let catalog_request = json!({
        "context":{"project_id":ids.project_id,"requester":requester},
        "search":{"clients":{"query":"","offset":0},"tasks":{"query":"","offset":0}},
    });
    assert_eq!(
        api.call(
            "project_editor_catalog",
            catalog_request.clone(),
            None,
            false
        )
        .await
        .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        api.call(
            "project_editor_catalog",
            catalog_request.clone(),
            Some(&second_cookie),
            false
        )
        .await
        .status(),
        StatusCode::CONFLICT
    );
    let catalog = api
        .json(
            "project_editor_catalog",
            catalog_request.clone(),
            &first_cookie,
        )
        .await;
    assert_eq!(catalog["context"], catalog_request["context"]);
    assert_eq!(catalog["clients"][0]["id"], json!(ids.client_id));
    assert_eq!(catalog["tasks"][0]["id"], json!(ids.task_id));
    assert!(catalog["clients"][0]["default_rate_cents"].is_null());
    assert!(catalog["tasks"][0]["default_rate_cents"].is_null());
    assert!(catalog.get("people").is_none());
    let mut missing_project = catalog_request.clone();
    missing_project["context"]["project_id"] = json!(Uuid::now_v7());
    assert_eq!(
        api.call(
            "project_editor_catalog",
            missing_project,
            Some(&first_cookie),
            false
        )
        .await
        .status(),
        StatusCode::FORBIDDEN
    );
    let mut request = ProjectEditRequest {
        task_activity: Vec::new(),
        id: Uuid::now_v7(),
        project_id: initial.id,
        expected_revision: initial.revision,
        expected_requester: Some(requester),
        managers: initial
            .access
            .as_ref()
            .map(|access| (&access.managers).into()),
        form: initial.form.clone(),
        unchanged: vec![
            ProtectedProjectField::ProjectRate,
            ProtectedProjectField::Budget,
            ProtectedProjectField::Fees,
            ProtectedProjectField::InvoiceDefaults,
            ProtectedProjectField::PrivateNotes,
        ],
    };
    request.form.name = "Session-bound project edit".into();
    let body = json!({"request":request});
    assert_eq!(
        api.call("save_project_editor", body.clone(), None, false)
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
    for expected in [
        Value::Null,
        json!({"org_id":Uuid::now_v7(),"user_id":ids.user_id}),
    ] {
        let mut invalid = body.clone();
        invalid["request"]["expected_requester"] = expected;
        assert_eq!(
            api.call("save_project_editor", invalid, Some(&first_cookie), false)
                .await
                .status(),
            StatusCode::CONFLICT
        );
    }
    let switched = api
        .call(
            "save_project_editor",
            body.clone(),
            Some(&second_cookie),
            false,
        )
        .await;
    assert_eq!(switched.status(), StatusCode::CONFLICT);
    let error = switched.text().await.unwrap();
    assert!(error.contains(crate::models::project_creation::PROJECT_EDITOR_SESSION_CHANGED));
    assert!(!error.contains(&initial.form.name));
    assert!(!error.contains(&ids.user_id.to_string()));
    assert!(!error.contains(&second.to_string()));
    let unchanged = api
        .json("load_project_editor", lookup.clone(), &second_cookie)
        .await;
    assert_eq!(unchanged["revision"], json!(initial.revision));
    assert_eq!(unchanged["form"]["name"], json!(initial.form.name));
    assert_eq!(unchanged["access"]["requester"]["user_id"], json!(second));

    // Failed attempts consumed no receipt; the original session can save and retry.
    for _ in 0..2 {
        assert_eq!(
            api.json("save_project_editor", body.clone(), &first_cookie)
                .await,
            json!(ids.project_id)
        );
    }
    // The other session can independently open and save its own form.
    let current: EditableProject = serde_json::from_value(
        api.json("load_project_editor", lookup.clone(), &second_cookie)
            .await,
    )
    .unwrap();
    request.id = Uuid::now_v7();
    request.expected_revision = current.revision;
    request.expected_requester = current.access.map(|access| access.requester);
    request.form = current.form;
    request.form.name = "Confirmed in the second session".into();
    assert_eq!(
        api.json(
            "save_project_editor",
            json!({"request":request}),
            &second_cookie
        )
        .await,
        json!(ids.project_id)
    );

    let revoked: Vec<String> =
        serde_json::from_value(json!(PermissionSelection::new(&[]))).unwrap();
    // Valid revocation retains the immutable own-work floor. Missing that floor
    // is malformed stored policy, not an ordinary permission denial.
    for (grants, status) in [
        (revoked, StatusCode::FORBIDDEN),
        (Vec::new(), StatusCode::INTERNAL_SERVER_ERROR),
    ] {
        sqlx::query!(
            "UPDATE person_permission_states SET grants=$2 WHERE user_id=$1",
            ids.user_id,
            &grants,
        )
        .execute(pool)
        .await
        .unwrap();
        let response = api
            .call(
                "save_project_editor",
                body.clone(),
                Some(&first_cookie),
                false,
            )
            .await;
        assert_eq!(response.status(), status);
        assert_eq!(
            api.call(
                "load_project_editor",
                lookup.clone(),
                Some(&first_cookie),
                false
            )
            .await
            .status(),
            status
        );
        assert_eq!(
            api.call(
                "project_editor_catalog",
                catalog_request.clone(),
                Some(&first_cookie),
                false
            )
            .await
            .status(),
            status
        );
        let error = response.text().await.unwrap();
        for private in ["person_permission_states", "SELECT", "sqlx", "Widget"] {
            assert!(!error.contains(private), "{error}");
        }
    }
}
