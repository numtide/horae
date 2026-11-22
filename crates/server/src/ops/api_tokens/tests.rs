//! Token secrets, and the operations over them with a real database.

use horae_core::permissions::catalog::BuiltInProfile;
use sqlx::PgPool;

use super::*;
use crate::ops::testing::Recorder;
use crate::ops::{Channel, begin};
use crate::testing::{SeedIds, add_person, seed};

#[test]
fn a_secret_is_the_prefix_and_32_random_bytes_in_base64url() {
    use base64::Engine;
    let secret = Secret::generate(PREFIX);
    let encoded = secret.expose().strip_prefix(PREFIX).unwrap();
    let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(encoded);
    assert_eq!(decoded.unwrap().len(), 32);
    assert_ne!(secret.expose(), Secret::generate(PREFIX).expose());
}

#[test]
fn the_display_prefix_is_the_first_eight_characters_after_the_prefix() {
    let secret = Secret::generate(PREFIX);
    assert_eq!(
        secret.display_prefix(),
        &secret.expose()[PREFIX.len()..PREFIX.len() + 8]
    );
}

#[test]
fn the_digest_is_sha256_of_the_whole_token() {
    // SHA-256("abc"), the FIPS 180-2 test vector.
    assert_eq!(
        digest("abc"),
        [
            0xba, 0x78, 0x16, 0xbf, 0x8f, 0x01, 0xcf, 0xea, 0x41, 0x41, 0x40, 0xde, 0x5d, 0xae,
            0x22, 0x23, 0xb0, 0x03, 0x61, 0xa3, 0x96, 0x17, 0x7a, 0x9c, 0xb4, 0x10, 0xff, 0x61,
            0xf2, 0x00, 0x15, 0xad,
        ]
    );
}

#[test]
fn debug_shows_only_the_display_prefix() {
    let secret = Secret::generate(PREFIX);
    let debug = format!("{secret:?}");
    assert_eq!(debug, format!("{PREFIX}{}…", secret.display_prefix()));
    assert!(!debug.contains(&secret.expose()[PREFIX.len() + 8..]));
}

async fn create_as(
    pool: &PgPool,
    ids: &SeedIds,
    name: &str,
    expires_at: Option<DateTime<Utc>>,
) -> Result<(Secret, ApiTokenRow), OpError> {
    let mut write = begin(pool, Caller::session(ids.org_id, ids.user_id)).await?;
    let result = create(&mut write.tx, &write.actor, name, expires_at).await;
    write.commit(result, &Recorder::default()).await
}

async fn revoke_as(pool: &PgPool, ids: &SeedIds, id: Uuid) -> Result<ApiTokenRow, OpError> {
    let mut write = begin(pool, Caller::session(ids.org_id, ids.user_id)).await?;
    let result = revoke(&mut write.tx, &write.actor, id).await;
    write.commit(result, &Recorder::default()).await
}

async fn list_org_as(pool: &PgPool, ids: &SeedIds) -> Result<Vec<ApiTokenRow>, OpError> {
    let mut read = crate::ops::begin_read(pool, Caller::session(ids.org_id, ids.user_id)).await?;
    let result = list_org(&mut read.tx, &read.actor).await;
    read.finish(result).await
}

async fn list_own_as(pool: &PgPool, ids: &SeedIds) -> Vec<ApiTokenRow> {
    let mut read = crate::ops::begin_read(pool, Caller::session(ids.org_id, ids.user_id))
        .await
        .unwrap();
    let result = list_own(&mut read.tx, &read.actor).await;
    read.finish(result).await.unwrap()
}

async fn set_active(pool: &PgPool, user_id: Uuid, active: bool) {
    sqlx::query!(
        "UPDATE users SET active = $2 WHERE id = $1",
        user_id,
        active
    )
    .execute(pool)
    .await
    .unwrap();
}

async fn last_used(pool: &PgPool, id: Uuid) -> Option<DateTime<Utc>> {
    sqlx::query_scalar!(
        r#"SELECT last_used_at AS "last_used_at: DateTime<Utc>" FROM api_tokens WHERE id = $1"#,
        id
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn creating_stores_only_the_digest_and_answers_the_secret_once(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Member).await;
    let (secret, row) = create_as(&pool, &ids, "  Calendar sync ", None)
        .await
        .unwrap();
    assert_eq!(row.name, "Calendar sync");
    assert_eq!(row.user_id, ids.user_id);
    assert_eq!(row.owner_name, "Test User");
    assert_eq!(row.display_prefix, secret.display_prefix());
    let stored = sqlx::query!(
        "SELECT token_sha256, display_prefix FROM api_tokens WHERE id = $1",
        row.id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(stored.token_sha256, digest(secret.expose()));
    assert_eq!(stored.display_prefix, row.display_prefix);
    // No column holds the rest of the secret, in any encoding Postgres prints.
    let whole_row = sqlx::query_scalar!(
        r#"SELECT row_to_json(t)::text AS "row!" FROM api_tokens t WHERE id = $1"#,
        row.id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(
        !whole_row.contains(&secret.expose()[PREFIX.len() + 8..]),
        "{whole_row}"
    );
    assert_eq!(list_own_as(&pool, &ids).await, [row]);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_name_or_expiry_out_of_range_is_refused(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Member).await;
    let past = Some(Utc::now() - chrono::Duration::minutes(1));
    for (name, expires_at, field) in [
        ("   ", None, "name"),
        (&*"x".repeat(101), None, "name"),
        ("script", past, "expires_at"),
    ] {
        let error = create_as(&pool, &ids, name, expires_at).await.unwrap_err();
        assert!(
            matches!(&error, OpError::Invalid { field: Some(f), .. } if *f == field),
            "{error:?}"
        );
    }
    let longest = "é".repeat(100);
    let future = Some(Utc::now() + chrono::Duration::days(30));
    let (_, row) = create_as(&pool, &ids, &longest, future).await.unwrap();
    assert_eq!(row.name, longest);
    assert!(row.expires_at.is_some());
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_valid_token_authenticates_as_its_owner(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Member).await;
    let (secret, row) = create_as(&pool, &ids, "script", None).await.unwrap();
    let caller = authenticate(&pool, secret.expose()).await.unwrap().unwrap();
    assert_eq!(
        (caller.org_id, caller.user_id, caller.channel),
        (ids.org_id, ids.user_id, Channel::Token { token_id: row.id })
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn unknown_and_malformed_tokens_are_refused(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Member).await;
    let (secret, _) = create_as(&pool, &ids, "script", None).await.unwrap();
    let unknown = Secret::generate(PREFIX);
    let without_prefix = &secret.expose()[PREFIX.len()..];
    for token in [unknown.expose(), without_prefix, "", "horae_pat_"] {
        assert!(
            authenticate(&pool, token).await.unwrap().is_none(),
            "{token:?}"
        );
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn revoked_and_expired_tokens_are_refused(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Member).await;
    let (revoked, row) = create_as(&pool, &ids, "revoked", None).await.unwrap();
    revoke_as(&pool, &ids, row.id).await.unwrap();
    assert!(
        authenticate(&pool, revoked.expose())
            .await
            .unwrap()
            .is_none()
    );

    let soon = Some(Utc::now() + chrono::Duration::days(1));
    let (expired, row) = create_as(&pool, &ids, "expired", soon).await.unwrap();
    sqlx::query!(
        "UPDATE api_tokens SET expires_at = now() - interval '1 second' WHERE id = $1",
        row.id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(
        authenticate(&pool, expired.expose())
            .await
            .unwrap()
            .is_none()
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn deactivating_the_owner_suspends_the_token_until_reactivation(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Member).await;
    let (secret, _) = create_as(&pool, &ids, "script", None).await.unwrap();
    set_active(&pool, ids.user_id, false).await;
    assert!(
        authenticate(&pool, secret.expose())
            .await
            .unwrap()
            .is_none()
    );
    set_active(&pool, ids.user_id, true).await;
    assert!(
        authenticate(&pool, secret.expose())
            .await
            .unwrap()
            .is_some()
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn last_use_is_stamped_at_most_once_a_minute(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Member).await;
    let (secret, row) = create_as(&pool, &ids, "script", None).await.unwrap();
    assert_eq!(last_used(&pool, row.id).await, None);

    authenticate(&pool, secret.expose()).await.unwrap().unwrap();
    let first = last_used(&pool, row.id).await.unwrap();

    // Thirty seconds ago: too recent to stamp again.
    sqlx::query!(
        "UPDATE api_tokens SET last_used_at = now() - interval '30 seconds' WHERE id = $1",
        row.id
    )
    .execute(&pool)
    .await
    .unwrap();
    let recent = last_used(&pool, row.id).await.unwrap();
    assert!(recent < first);
    authenticate(&pool, secret.expose()).await.unwrap().unwrap();
    assert_eq!(last_used(&pool, row.id).await, Some(recent));

    // Two minutes ago: stamped again.
    sqlx::query!(
        "UPDATE api_tokens SET last_used_at = now() - interval '2 minutes' WHERE id = $1",
        row.id
    )
    .execute(&pool)
    .await
    .unwrap();
    let stale = last_used(&pool, row.id).await.unwrap();
    authenticate(&pool, secret.expose()).await.unwrap().unwrap();
    assert!(last_used(&pool, row.id).await.unwrap() > stale);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn an_administrator_lists_and_revokes_everyones_tokens(pool: PgPool) {
    let admin = seed(&pool, BuiltInProfile::Administrator).await;
    let member = add_person(&pool, &admin, BuiltInProfile::Member).await;
    let (_, mine) = create_as(&pool, &admin, "admin script", None)
        .await
        .unwrap();
    let (theirs_secret, theirs) = create_as(&pool, &member, "member script", None)
        .await
        .unwrap();

    let listed = list_org_as(&pool, &admin).await.unwrap();
    assert_eq!(
        listed.iter().map(|row| row.id).collect::<Vec<_>>(),
        [theirs.id, mine.id],
        "newest first"
    );
    assert_eq!(listed[0].owner_name, "Teammate");

    let revoked = revoke_as(&pool, &admin, theirs.id).await.unwrap();
    assert!(revoked.revoked_at.is_some());
    assert!(
        authenticate(&pool, theirs_secret.expose())
            .await
            .unwrap()
            .is_none()
    );
    // Repeating it is a no-op that keeps the first time.
    let again = revoke_as(&pool, &admin, theirs.id).await.unwrap();
    assert_eq!(again.revoked_at, revoked.revoked_at);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn the_owner_revokes_their_own_token(pool: PgPool) {
    let admin = seed(&pool, BuiltInProfile::Administrator).await;
    let member = add_person(&pool, &admin, BuiltInProfile::Member).await;
    let (_, row) = create_as(&pool, &member, "script", None).await.unwrap();
    assert!(
        revoke_as(&pool, &member, row.id)
            .await
            .unwrap()
            .revoked_at
            .is_some()
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_member_can_neither_list_nor_revoke_someone_elses_tokens(pool: PgPool) {
    let admin = seed(&pool, BuiltInProfile::Administrator).await;
    let member = add_person(&pool, &admin, BuiltInProfile::Member).await;
    let (secret, row) = create_as(&pool, &admin, "admin script", None)
        .await
        .unwrap();

    let listed = list_org_as(&pool, &member).await;
    assert!(matches!(listed, Err(OpError::Forbidden(_))), "{listed:?}");
    let revoked = revoke_as(&pool, &member, row.id).await;
    assert!(matches!(revoked, Err(OpError::NotFound(_))), "{revoked:?}");
    assert!(
        authenticate(&pool, secret.expose())
            .await
            .unwrap()
            .is_some()
    );
    assert!(list_own_as(&pool, &member).await.is_empty());
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn another_organizations_token_is_not_found_even_for_an_administrator(pool: PgPool) {
    let admin = seed(&pool, BuiltInProfile::Administrator).await;
    let foreign = seed(&pool, BuiltInProfile::Administrator).await;
    let (_, row) = create_as(&pool, &foreign, "foreign", None).await.unwrap();
    let revoked = revoke_as(&pool, &admin, row.id).await;
    assert!(matches!(revoked, Err(OpError::NotFound(_))), "{revoked:?}");
    assert!(list_org_as(&pool, &admin).await.unwrap().is_empty());
}

async fn identity_as(pool: &PgPool, caller: Caller) -> Result<TokenIdentity, OpError> {
    let mut read = crate::ops::begin_read(pool, caller).await?;
    let result = identity(&mut read.tx, &read.actor).await;
    read.finish(result).await
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_token_caller_has_an_identity(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Member).await;
    let (secret, row) = create_as(&pool, &ids, "calendar sync", None).await.unwrap();
    let caller = authenticate(&pool, secret.expose()).await.unwrap().unwrap();
    let found = identity_as(&pool, caller).await.unwrap();
    assert_eq!(
        found,
        TokenIdentity {
            person_email: format!("{}@test.com", ids.user_id),
            organization_name: "Test Org".into(),
            token: ApiTokenRow {
                last_used_at: found.token.last_used_at,
                ..row
            },
        }
    );
    assert_eq!(found.token.owner_name, "Test User");
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn only_an_active_token_caller_has_an_identity(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Member).await;
    let session = identity_as(&pool, Caller::session(ids.org_id, ids.user_id)).await;
    assert!(matches!(session, Err(OpError::NotFound(_))), "{session:?}");

    let (_, row) = create_as(&pool, &ids, "script", None).await.unwrap();
    let caller = Caller::token(ids.org_id, ids.user_id, row.id);
    let other = add_person(&pool, &ids, BuiltInProfile::Member).await;
    let theirs = Caller::token(ids.org_id, other.user_id, row.id);
    let error = identity_as(&pool, theirs).await;
    assert!(matches!(error, Err(OpError::NotFound(_))), "{error:?}");

    set_active(&pool, ids.user_id, false).await;
    let inactive = identity_as(&pool, caller).await;
    assert!(
        matches!(inactive, Err(OpError::Forbidden(_))),
        "{inactive:?}"
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn every_active_person_reads_the_harvest_account_id(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Member).await;
    let stored = fetch_harvest_account_id(&pool, ids.org_id).await.unwrap();
    let mut read = crate::ops::begin_read(&pool, Caller::session(ids.org_id, ids.user_id))
        .await
        .unwrap();
    let result = harvest_account_id(&mut read.tx, &read.actor).await;
    assert_eq!(read.finish(result).await.unwrap(), stored);

    set_active(&pool, ids.user_id, false).await;
    let mut read = crate::ops::begin_read(&pool, Caller::session(ids.org_id, ids.user_id))
        .await
        .unwrap();
    let result = harvest_account_id(&mut read.tx, &read.actor).await;
    let refused = read.finish(result).await;
    assert!(matches!(refused, Err(OpError::Forbidden(_))), "{refused:?}");
}

/// Tokens are managed in the web app only: no personal or OAuth token, even
/// an OAuth token with every scope, lists, mints or revokes one.
#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn no_token_manages_personal_tokens(pool: PgPool) {
    use horae_core::permissions::oauth::{Scope, ScopeSet};
    let ids = seed(&pool, BuiltInProfile::Administrator).await;
    let (_, row) = create_as(&pool, &ids, "script", None).await.unwrap();
    let all = ScopeSet::of(&Scope::ALL);
    for caller in [
        Caller::token(ids.org_id, ids.user_id, row.id),
        Caller::oauth(ids.org_id, ids.user_id, Uuid::now_v7(), Uuid::now_v7(), all),
    ] {
        let mut write = begin(&pool, caller).await.unwrap();
        write.actor.unlock_scope(Scope::TimeWrite).unwrap();
        let refused = |result: Result<_, OpError>| {
            assert!(matches!(result, Err(OpError::Forbidden(_))), "{caller:?}");
        };
        refused(list_own(&mut write.tx, &write.actor).await.map(drop));
        refused(list_org(&mut write.tx, &write.actor).await.map(drop));
        refused(
            create(&mut write.tx, &write.actor, "minted", None)
                .await
                .map(drop),
        );
        refused(revoke(&mut write.tx, &write.actor, row.id).await.map(drop));
    }
    assert_eq!(list_own_as(&pool, &ids).await.len(), 1);
}
