//! The four token server functions, through the adapters their `#[server]`
//! bodies call after reading the session. Who may do what is tested in
//! `ops::api_tokens`; these check the answers and their status codes.

use super::*;
use crate::testing::{SeedIds, add_person, seed};
use horae_core::permissions::catalog::BuiltInProfile;
use sqlx::PgPool;

fn caller(ids: &SeedIds) -> Caller {
    Caller::session(ids.org_id, ids.user_id)
}

fn code(error: &ServerFnError) -> u16 {
    match error {
        ServerFnError::ServerError { code, .. } => *code,
        other => panic!("not a server error: {other:?}"),
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_created_token_is_answered_once_and_listed_without_its_secret(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Member).await;
    let created = create(&pool, caller(&ids), "Calendar sync", None)
        .await
        .unwrap();
    assert!(created.token.starts_with("horae_pat_"));
    assert!(created.token[10..].starts_with(&created.row.display_prefix));
    let listed = list_own(&pool, caller(&ids)).await.unwrap();
    assert_eq!(listed, std::slice::from_ref(&created.row));
    assert!(
        !serde_json::to_string(&listed)
            .unwrap()
            .contains(&created.token[18..])
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_bad_name_or_expiry_is_a_bad_request(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Member).await;
    let error = create(&pool, caller(&ids), " ", None).await.unwrap_err();
    assert_eq!(code(&error), BAD_REQUEST);
    let past = Some(Utc::now() - chrono::Duration::hours(1));
    let error = create(&pool, caller(&ids), "script", past)
        .await
        .unwrap_err();
    assert_eq!(code(&error), BAD_REQUEST);
    assert!(list_own(&pool, caller(&ids)).await.unwrap().is_empty());
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn revoking_answers_the_row_and_another_persons_token_is_not_found(pool: PgPool) {
    let admin = seed(&pool, BuiltInProfile::Administrator).await;
    let member = add_person(&pool, &admin, BuiltInProfile::Member).await;
    let theirs = create(&pool, caller(&admin), "admin script", None)
        .await
        .unwrap();
    let error = revoke(&pool, caller(&member), theirs.row.id)
        .await
        .unwrap_err();
    assert_eq!(code(&error), NOT_FOUND);

    let mine = create(&pool, caller(&member), "member script", None)
        .await
        .unwrap();
    let revoked = revoke(&pool, caller(&member), mine.row.id).await.unwrap();
    assert_eq!(revoked.id, mine.row.id);
    assert!(revoked.revoked_at.is_some());
    // An administrator revokes anyone's.
    let revoked = revoke(&pool, caller(&admin), mine.row.id).await.unwrap();
    assert!(revoked.revoked_at.is_some());
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn only_an_administrator_lists_the_organizations_tokens(pool: PgPool) {
    let admin = seed(&pool, BuiltInProfile::Administrator).await;
    let member = add_person(&pool, &admin, BuiltInProfile::Member).await;
    let created = create(&pool, caller(&member), "member script", None)
        .await
        .unwrap();
    let listed = list_org(&pool, caller(&admin)).await.unwrap();
    assert_eq!(listed, [created.row]);
    assert_eq!(listed[0].owner_name, "Teammate");
    let error = list_org(&pool, caller(&member)).await.unwrap_err();
    assert_eq!(code(&error), FORBIDDEN);
}
