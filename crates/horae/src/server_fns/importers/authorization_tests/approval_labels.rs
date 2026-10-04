//! Approval labels come from the authorized records, never a people directory.

use super::*;
use chrono::NaiveDate;
use horae_core::types::EntryState;

async fn approval(
    pool: &PgPool,
    org: Uuid,
    user: Uuid,
    start: NaiveDate,
    state: EntryState,
) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO approvals (id,org_id,user_id,period_start,period_end,state)
        VALUES ($1,$2,$3,$4,$5,$6)",
        id,
        org,
        user,
        start as NaiveDate,
        (start + chrono::Days::new(6)) as NaiveDate,
        state as EntryState
    )
    .execute(pool)
    .await
    .unwrap();
    id
}

pub(super) async fn check(pool: &PgPool, api: &Api) {
    let ids = crate::server_fns::test_seed::seed(pool, OrgRole::Manager).await;
    let foreign = crate::server_fns::test_seed::seed(pool, OrgRole::Admin).await;
    let admin = user(pool, ids.org_id, OrgRole::Admin).await;
    let member = user(pool, ids.org_id, OrgRole::Member).await;
    let archived = user(pool, ids.org_id, OrgRole::Member).await;
    let unused = user(pool, ids.org_id, OrgRole::Member).await;
    let start = NaiveDate::from_ymd_opt(2026, 9, 21).unwrap();
    let name = "Álvaro <script>alert('name')</script> & 王";
    sqlx::query!(
        "UPDATE users SET name=$2, oidc_subject=id::text,
        cost_rate_cents=6000, billable_rate_cents=10000 WHERE id=ANY($1)",
        &[member, archived],
        name
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query!("UPDATE users SET active=false WHERE id=$1", archived)
        .execute(pool)
        .await
        .unwrap();
    let pending = approval(pool, ids.org_id, archived, start, EntryState::Submitted).await;
    let approved = approval(pool, ids.org_id, member, start, EntryState::Approved).await;
    let foreign_approval = approval(
        pool,
        foreign.org_id,
        foreign.user_id,
        start,
        EntryState::Submitted,
    )
    .await;
    let malformed = approval(
        pool,
        ids.org_id,
        foreign.user_id,
        start + chrono::Days::new(7),
        EntryState::Submitted,
    )
    .await;
    for (minutes, billable) in [(60, true), (30, false)] {
        sqlx::query!("INSERT INTO time_entries (id,org_id,user_id,project_id,task_id,spent_date,minutes,billable)
            VALUES ($1,$2,$3,$4,$5,$6,$7,$8)", Uuid::now_v7(), ids.org_id, archived,
            ids.project_id, ids.task_id, start as NaiveDate, minutes, billable).execute(pool).await.unwrap();
    }
    for actor in [ids.user_id, admin] {
        let cookie = api.cookie(actor).await;
        for (status, expected) in [
            (None, vec![pending, approved]),
            (Some("submitted"), vec![pending]),
            (Some("approved"), vec![approved]),
        ] {
            let response = api
                .json("list_approvals", json!({"status":status}), &cookie)
                .await;
            let rows = response.as_array().unwrap();
            // Retained approvals carry names even when the submitter is archived.
            for row in rows
                .iter()
                .filter(|row| row["approval"]["id"] != json!(malformed))
            {
                assert_eq!(row["user_name"], name);
            }
            let actual: std::collections::BTreeSet<Uuid> = rows
                .iter()
                .map(|row| serde_json::from_value(row["approval"]["id"].clone()).unwrap())
                .collect();
            assert_eq!(actual, expected.into_iter().collect());
            for row in rows {
                let mut keys: Vec<_> = row
                    .as_object()
                    .unwrap()
                    .keys()
                    .map(String::as_str)
                    .collect();
                keys.sort();
                assert_eq!(
                    keys,
                    ["approval", "billable_minutes", "total_minutes", "user_name"]
                );
                let is_pending = row["approval"]["id"] == json!(pending);
                assert_eq!(row["total_minutes"], if is_pending { 90 } else { 0 });
                assert_eq!(row["billable_minutes"], if is_pending { 60 } else { 0 });
                for hidden in [foreign_approval, malformed, unused] {
                    assert!(!row.to_string().contains(&hidden.to_string()));
                }
                for field in [
                    "email",
                    "org_role",
                    "oidc_subject",
                    "cost_rate_cents",
                    "billable_rate_cents",
                ] {
                    assert!(!row.to_string().contains(field));
                }
            }
        }
    }
    assert_eq!(
        api.call("list_approvals", json!({"status":null}), None, false)
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
    for (actor, expected) in [
        (member, StatusCode::FORBIDDEN),
        (archived, StatusCode::UNAUTHORIZED),
    ] {
        let cookie = api.cookie(actor).await;
        assert_eq!(
            api.call(
                "list_approvals",
                json!({"status":null}),
                Some(&cookie),
                false
            )
            .await
            .status(),
            expected
        );
    }
    let cookie = api.cookie(ids.user_id).await;
    sqlx::query!(
        "UPDATE users SET name='Renamed archived teammate' WHERE id=$1",
        archived
    )
    .execute(pool)
    .await
    .unwrap();
    let renamed = api
        .json("list_approvals", json!({"status":"submitted"}), &cookie)
        .await;
    assert_eq!(renamed[0]["user_name"], "Renamed archived teammate");
    sqlx::query!(
        "UPDATE users SET org_role='member' WHERE id=$1",
        ids.user_id
    )
    .execute(pool)
    .await
    .unwrap();
    assert_eq!(
        api.call(
            "list_approvals",
            json!({"status":null}),
            Some(&cookie),
            false
        )
        .await
        .status(),
        StatusCode::FORBIDDEN
    );
}
