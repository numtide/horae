//! Directory response boundaries through the registered session endpoint.

use super::*;
use horae_core::permissions::catalog::BuiltInProfile;

/// A person of `org_id` holding `profile`'s permission state.
async fn person(pool: &PgPool, org_id: Uuid, profile: BuiltInProfile) -> Uuid {
    let id = user(pool, org_id, OrgRole::Member).await;
    let mut connection = pool.acquire().await.unwrap();
    crate::server_fns::provisioning::provision(&mut connection, org_id, id, profile)
        .await
        .unwrap();
    id
}

pub(super) async fn check(pool: &PgPool, api: &Api) {
    let ids = crate::server_fns::test_seed::seed(pool, BuiltInProfile::Administrator).await;
    let foreign = crate::server_fns::test_seed::seed(pool, BuiltInProfile::Administrator).await;
    let people_admin = person(pool, ids.org_id, BuiltInProfile::PeopleAdmin).await;
    let project_manager = person(pool, ids.org_id, BuiltInProfile::ProjectManager).await;
    let member = person(pool, ids.org_id, BuiltInProfile::Member).await;
    let archived = person(pool, ids.org_id, BuiltInProfile::Member).await;
    sqlx::query!("UPDATE users SET active = false WHERE id = $1", archived)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query!(
        "UPDATE users SET oidc_subject = id::text, cost_rate_cents = 6000,
            billable_rate_cents = 10000 WHERE org_id = $1",
        ids.org_id
    )
    .execute(pool)
    .await
    .unwrap();

    // Archived people are listed only to those who may view everyone.
    let reads_all = [ids.user_id, people_admin];
    for include_inactive in [false, true] {
        let request = json!({"include_inactive":include_inactive});
        assert_eq!(
            api.call("list_users", request.clone(), None, false)
                .await
                .status(),
            StatusCode::UNAUTHORIZED
        );
        for actor in [ids.user_id, people_admin, project_manager, member, archived] {
            let cookie = api.cookie(actor).await;
            let response = api
                .call("list_users", request.clone(), Some(&cookie), false)
                .await;
            if actor == archived {
                assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
            } else if include_inactive && !reads_all.contains(&actor) {
                assert_eq!(response.status(), StatusCode::FORBIDDEN);
            } else {
                assert_eq!(response.status(), StatusCode::OK);
                let rows: Vec<Value> = response.json().await.unwrap();
                let mut expected_ids = vec![ids.user_id, people_admin, project_manager, member];
                if include_inactive {
                    expected_ids.push(archived);
                }
                expected_ids.sort();
                let mut actual_ids = Vec::new();
                for row in rows {
                    let mut keys: Vec<_> = row
                        .as_object()
                        .unwrap()
                        .keys()
                        .map(String::as_str)
                        .collect();
                    keys.sort();
                    assert_eq!(keys, ["active", "email", "id", "name"]);
                    let id: Uuid = serde_json::from_value(row["id"].clone()).unwrap();
                    assert_ne!(id, foreign.user_id);
                    assert_eq!(row["active"], json!(id != archived));
                    assert_eq!(row["email"], json!(format!("{id}@test.com")));
                    assert_eq!(row["name"], "Test User");
                    actual_ids.push(id);
                }
                actual_ids.sort();
                assert_eq!(actual_ids, expected_ids);
            }
        }
    }

    // Reusing a session must not retain its earlier inactive-directory access.
    let cookie = api.cookie(people_admin).await;
    sqlx::query!(
        "UPDATE person_permission_states SET grants = array_remove(grants, 'people_read_all')
         WHERE user_id = $1",
        people_admin
    )
    .execute(pool)
    .await
    .unwrap();
    assert_eq!(
        api.call(
            "list_users",
            json!({"include_inactive":true}),
            Some(&cookie),
            false
        )
        .await
        .status(),
        StatusCode::FORBIDDEN
    );
}
