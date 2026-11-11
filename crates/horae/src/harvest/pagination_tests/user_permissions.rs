use super::project_permissions::{canonical, selection};
use super::*;
use horae_core::permissions::catalog::Permission;

async fn with_rates(pool: &PgPool, user_id: Uuid) {
    sqlx::query!(
        "UPDATE users SET billable_rate_cents = 9000, cost_rate_cents = 4000 WHERE id = $1",
        user_id
    )
    .execute(pool)
    .await
    .unwrap();
}

/// The caller's row from `/users`, which lists everyone in the seeded org.
async fn listed(pool: &PgPool, app: &Router, user_id: Uuid) -> Value {
    let id = api_id(pool, "user", user_id).await;
    let body = page(app, "/harvest/v2/users").await;
    body["users"]
        .as_array()
        .unwrap()
        .iter()
        .find(|user| user["id"] == id)
        .cloned()
        .expect("caller missing from /users")
}

fn rates(user: &Value) -> (Option<f64>, Option<f64>) {
    (
        user.get("default_hourly_rate").map(|v| v.as_f64().unwrap()),
        user.get("cost_rate").map(|v| v.as_f64().unwrap()),
    )
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn user_api_follows_people_and_rate_grants_not_the_legacy_role(pool: PgPool) {
    // A legacy manager whose detailed permissions carry no rate grant.
    let (ids, app) = canonical(&pool, OrgRole::Manager, &[Permission::PeopleReadAll]).await;
    with_rates(&pool, ids.user_id).await;
    assert_eq!(rates(&listed(&pool, &app, ids.user_id).await), (None, None));
    assert_eq!(
        rates(&page(&app, "/harvest/v2/users/me").await),
        (None, None)
    );

    // Each org-wide grant shows only its own rate.
    let cases = [
        (Permission::BillableRateReadAll, (Some(90.0), None)),
        (Permission::CostRateReadAll, (None, Some(40.0))),
        // Managed-only grants are not honoured by the API.
        (Permission::BillableRateReadManaged, (None, None)),
    ];
    for (grant, expected) in cases {
        selection(&pool, ids.user_id, &[Permission::PeopleReadAll, grant]).await;
        assert_eq!(
            rates(&listed(&pool, &app, ids.user_id).await),
            expected,
            "{grant:?}"
        );
        assert_eq!(
            rates(&page(&app, "/harvest/v2/users/me").await),
            expected,
            "{grant:?}"
        );
    }

    // Rate grants without people access, as in the Accounting profile: the
    // list is refused and /users/me shows no rate.
    selection(
        &pool,
        ids.user_id,
        &[Permission::BillableRateReadAll, Permission::CostRateReadAll],
    )
    .await;
    assert_eq!(
        request(&app, "/harvest/v2/users").await.0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        rates(&page(&app, "/harvest/v2/users/me").await),
        (None, None)
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn user_api_gives_a_legacy_admin_nothing_their_grants_do_not(pool: PgPool) {
    let (ids, app) = canonical(&pool, OrgRole::Admin, &[]).await;
    with_rates(&pool, ids.user_id).await;
    assert_eq!(
        request(&app, "/harvest/v2/users").await.0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        rates(&page(&app, "/harvest/v2/users/me").await),
        (None, None)
    );
}
