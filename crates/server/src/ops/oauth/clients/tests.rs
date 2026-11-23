//! App registration and administration with a real database.

use horae_core::permissions::catalog::BuiltInProfile;
use horae_core::permissions::oauth::{ClientKind, Scope, ScopeSet};
use sqlx::PgPool;
use uuid::Uuid;

use super::*;
use crate::ops::oauth::testing::{audit_actions, grant};
use crate::ops::testing::Recorder;
use crate::ops::{Caller, begin, begin_read};
use crate::testing::{SeedIds, add_person, seed};

fn public_app(name: &str) -> NewClient {
    NewClient {
        name: name.into(),
        kind: ClientKind::Public,
        redirect_uris: vec!["com.example.app:/callback".into()],
        scopes: ScopeSet::of(&[Scope::TimeWrite]),
    }
}

fn confidential_app(name: &str) -> NewClient {
    NewClient {
        name: name.into(),
        kind: ClientKind::Confidential,
        redirect_uris: vec!["https://app.example.com/callback".into()],
        scopes: ScopeSet::of(&[Scope::CatalogRead]),
    }
}

async fn register_as(
    pool: &PgPool,
    ids: &SeedIds,
    input: NewClient,
) -> Result<(OAuthClient, Option<Secret>), OpError> {
    let mut write = begin(pool, Caller::session(ids.org_id, ids.user_id)).await?;
    let result = register(&mut write.tx, &write.actor, input).await;
    write.commit(result, &Recorder::default()).await
}

async fn list_as(pool: &PgPool, ids: &SeedIds) -> Result<Vec<OAuthClient>, OpError> {
    let mut read = begin_read(pool, Caller::session(ids.org_id, ids.user_id)).await?;
    let result = list(&mut read.tx, &read.actor).await;
    read.finish(result).await
}

async fn update_as(
    pool: &PgPool,
    ids: &SeedIds,
    id: Uuid,
    changes: ClientChanges,
) -> Result<OAuthClient, OpError> {
    let mut write = begin(pool, Caller::session(ids.org_id, ids.user_id)).await?;
    let result = update(&mut write.tx, &write.actor, id, changes).await;
    write.commit(result, &Recorder::default()).await
}

async fn set_enabled_as(
    pool: &PgPool,
    ids: &SeedIds,
    id: Uuid,
    enabled: bool,
) -> Result<OAuthClient, OpError> {
    let mut write = begin(pool, Caller::session(ids.org_id, ids.user_id)).await?;
    let result = set_enabled(&mut write.tx, &write.actor, id, enabled).await;
    write.commit(result, &Recorder::default()).await
}

async fn replace_secret_as(
    pool: &PgPool,
    ids: &SeedIds,
    id: Uuid,
) -> Result<(OAuthClient, Secret), OpError> {
    let mut write = begin(pool, Caller::session(ids.org_id, ids.user_id)).await?;
    let result = replace_secret(&mut write.tx, &write.actor, id).await;
    write.commit(result, &Recorder::default()).await
}

async fn delete_as(pool: &PgPool, ids: &SeedIds, id: Uuid) -> Result<(), OpError> {
    let mut write = begin(pool, Caller::session(ids.org_id, ids.user_id)).await?;
    let result = delete(&mut write.tx, &write.actor, id).await;
    write.commit(result, &Recorder::default()).await
}

async fn stored_secret(pool: &PgPool, id: Uuid) -> Option<Vec<u8>> {
    sqlx::query_scalar!("SELECT secret_sha256 FROM oauth_clients WHERE id = $1", id)
        .fetch_one(pool)
        .await
        .unwrap()
}

fn built_in(clients: &[OAuthClient]) -> &OAuthClient {
    clients.iter().find(|client| client.built_in).unwrap()
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn every_organization_has_one_disabled_built_in_client(pool: PgPool) {
    let admin = seed(&pool, BuiltInProfile::Administrator).await;
    let clients = list_as(&pool, &admin).await.unwrap();
    assert_eq!(clients.len(), 1);
    let android = &clients[0];
    assert!(android.built_in);
    assert!(!android.enabled);
    assert_eq!(android.client_id, BUILT_IN_CLIENT_ID);
    assert_eq!(android.kind, ClientKind::Public);
    assert_eq!(android.redirect_uris, [BUILT_IN_REDIRECT_URI]);
    assert_eq!(android.name, BUILT_IN_NAME);
    assert_eq!(android.scopes, ScopeSet::of(&BUILT_IN_SCOPES));
    assert_eq!(android.id.get_version_num(), 7);
}

/// The migration gives organizations that already exist their built-in
/// client too, not only the trigger new ones.
#[sqlx::test(migrations = false)]
async fn the_migration_gives_existing_organizations_a_built_in_client(pool: PgPool) {
    let at = crate::db::MIGRATOR
        .iter()
        .position(|migration| migration.description == "oauth")
        .expect("the oauth migration");
    let (before, oauth) = crate::db::MIGRATOR.migrations.split_at(at);
    for migration in before {
        sqlx::raw_sql(&migration.sql).execute(&pool).await.unwrap();
    }
    let ids = crate::testing::seed_tenant(&pool).await;
    sqlx::raw_sql(&oauth[0].sql).execute(&pool).await.unwrap();
    let row = sqlx::query!(
        r#"SELECT id, client_id, name, kind::text AS "kind!", redirect_uris, scopes, built_in,
                  enabled
           FROM oauth_clients WHERE org_id = $1"#,
        ids.org_id,
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(row.id.get_version_num(), 7);
    assert_eq!(row.client_id, BUILT_IN_CLIENT_ID);
    assert_eq!(row.name, BUILT_IN_NAME);
    assert_eq!(row.kind, "public");
    assert_eq!(row.redirect_uris, [BUILT_IN_REDIRECT_URI]);
    assert_eq!(row.scopes, ScopeSet::of(&BUILT_IN_SCOPES).names());
    assert!(row.built_in && !row.enabled);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn no_token_manages_apps(pool: PgPool) {
    let admin = seed(&pool, BuiltInProfile::Administrator).await;
    let (app, _) = register_as(&pool, &admin, public_app("Widget"))
        .await
        .unwrap();
    let all = ScopeSet::of(&Scope::ALL);
    for caller in [
        Caller::token(admin.org_id, admin.user_id, Uuid::now_v7()),
        Caller::oauth(
            admin.org_id,
            admin.user_id,
            Uuid::now_v7(),
            Uuid::now_v7(),
            all,
        ),
    ] {
        let mut write = begin(&pool, caller).await.unwrap();
        write.actor.unlock_scope(Scope::CatalogRead).unwrap();
        let refused = |result: Result<_, OpError>| {
            assert!(matches!(result, Err(OpError::Forbidden(_))), "{caller:?}");
        };
        refused(list(&mut write.tx, &write.actor).await.map(drop));
        refused(
            register(&mut write.tx, &write.actor, public_app("Mine"))
                .await
                .map(drop),
        );
        refused(
            set_enabled(&mut write.tx, &write.actor, app.id, false)
                .await
                .map(drop),
        );
        refused(delete(&mut write.tx, &write.actor, app.id).await.map(drop));
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn registering_a_public_app_lists_it_after_the_built_in_one(pool: PgPool) {
    let admin = seed(&pool, BuiltInProfile::Administrator).await;
    let (app, secret) = register_as(&pool, &admin, public_app("  Timer widget "))
        .await
        .unwrap();
    assert!(secret.is_none());
    assert_eq!(app.name, "Timer widget");
    assert!(app.enabled && !app.built_in);
    assert!(
        app.client_id
            .starts_with(crate::ops::oauth::CLIENT_ID_PREFIX)
    );
    assert_eq!(app.secret_prefix, None);
    // `time:write` includes `time:read`.
    assert_eq!(
        app.scopes,
        ScopeSet::of(&[Scope::TimeRead, Scope::TimeWrite])
    );
    let listed = list_as(&pool, &admin).await.unwrap();
    assert_eq!(
        listed
            .iter()
            .map(|client| client.built_in)
            .collect::<Vec<_>>(),
        [true, false]
    );
    assert_eq!(listed[1], app);
    assert_eq!(audit_actions(&pool, app.id).await, ["client_registered"]);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_confidential_app_gets_a_secret_stored_only_as_its_digest(pool: PgPool) {
    let admin = seed(&pool, BuiltInProfile::Administrator).await;
    let (app, secret) = register_as(&pool, &admin, confidential_app("Billing sync"))
        .await
        .unwrap();
    let secret = secret.unwrap();
    assert!(
        secret
            .expose()
            .starts_with(crate::ops::oauth::CLIENT_SECRET_PREFIX)
    );
    assert_eq!(app.secret_prefix.as_deref(), Some(secret.display_prefix()));
    assert_eq!(stored_secret(&pool, app.id).await, Some(secret.digest()));
    let data = sqlx::query_scalar!(
        "SELECT data::text FROM audit_log WHERE entity_id = $1",
        app.id
    )
    .fetch_one(&pool)
    .await
    .unwrap()
    .unwrap();
    assert!(!data.contains(secret.expose()));
    assert!(!data.contains(&secret.expose()[10..]));
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn registration_validates_its_input(pool: PgPool) {
    let admin = seed(&pool, BuiltInProfile::Administrator).await;
    let field = |error: OpError| match error {
        OpError::Invalid { field, .. } => field,
        other => panic!("{other:?}"),
    };
    let long = "x".repeat(101);
    for name in ["", "   ", long.as_str()] {
        let error = register_as(&pool, &admin, public_app(name))
            .await
            .unwrap_err();
        assert_eq!(field(error), Some("name"), "{name:?}");
    }
    for uris in [
        vec![],
        vec!["https://a.example/cb".to_owned(); 11],
        vec!["https://a.example/cb".into(), "https://a.example/cb".into()],
        vec!["http://localhost/cb".into()],
        vec!["https://a.example/cb#x".into()],
    ] {
        let mut input = public_app("App");
        input.redirect_uris = uris.clone();
        let error = register_as(&pool, &admin, input).await.unwrap_err();
        assert_eq!(field(error), Some("redirect_uris"), "{uris:?}");
    }
    let mut loopback = confidential_app("Server app");
    loopback.redirect_uris = vec!["http://127.0.0.1/cb".into()];
    let error = register_as(&pool, &admin, loopback).await.unwrap_err();
    assert_eq!(field(error), Some("redirect_uris"));
    let mut no_scopes = public_app("App");
    no_scopes.scopes = ScopeSet::EMPTY;
    let error = register_as(&pool, &admin, no_scopes).await.unwrap_err();
    assert_eq!(field(error), Some("scopes"));
    assert_eq!(list_as(&pool, &admin).await.unwrap().len(), 1);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn two_apps_cannot_share_a_name(pool: PgPool) {
    let admin = seed(&pool, BuiltInProfile::Administrator).await;
    register_as(&pool, &admin, public_app("Widget"))
        .await
        .unwrap();
    assert_eq!(
        register_as(&pool, &admin, confidential_app("Widget"))
            .await
            .unwrap_err(),
        OpError::conflict(NAME_TAKEN)
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn only_an_administrator_manages_apps(pool: PgPool) {
    let admin = seed(&pool, BuiltInProfile::Administrator).await;
    let (app, _) = register_as(&pool, &admin, public_app("Widget"))
        .await
        .unwrap();
    for profile in [BuiltInProfile::Member, BuiltInProfile::PeopleAdmin] {
        let person = add_person(&pool, &admin, profile).await;
        let forbidden = |result: Result<_, OpError>| {
            assert!(matches!(result, Err(OpError::Forbidden(_))), "{profile:?}");
        };
        forbidden(list_as(&pool, &person).await.map(drop));
        forbidden(
            register_as(&pool, &person, public_app("Mine"))
                .await
                .map(drop),
        );
        forbidden(
            set_enabled_as(&pool, &person, app.id, false)
                .await
                .map(drop),
        );
        forbidden(replace_secret_as(&pool, &person, app.id).await.map(drop));
        forbidden(delete_as(&pool, &person, app.id).await);
        let changes = ClientChanges {
            name: "Renamed".into(),
            redirect_uris: app.redirect_uris.clone(),
            scopes: app.scopes,
        };
        forbidden(update_as(&pool, &person, app.id, changes).await.map(drop));
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn another_organizations_app_is_not_found(pool: PgPool) {
    let admin = seed(&pool, BuiltInProfile::Administrator).await;
    let other = seed(&pool, BuiltInProfile::Administrator).await;
    let (app, _) = register_as(&pool, &other, public_app("Theirs"))
        .await
        .unwrap();
    assert!(matches!(
        set_enabled_as(&pool, &admin, app.id, false).await,
        Err(OpError::NotFound(_))
    ));
    assert!(matches!(
        delete_as(&pool, &admin, app.id).await,
        Err(OpError::NotFound(_))
    ));
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn updating_changes_name_uris_and_scopes(pool: PgPool) {
    let admin = seed(&pool, BuiltInProfile::Administrator).await;
    let (app, _) = register_as(&pool, &admin, public_app("Widget"))
        .await
        .unwrap();
    let changes = ClientChanges {
        name: "Widget 2".into(),
        redirect_uris: vec![
            "com.example.app:/callback".into(),
            "http://127.0.0.1/callback".into(),
        ],
        scopes: ScopeSet::of(&[Scope::CatalogRead]),
    };
    let updated = update_as(&pool, &admin, app.id, changes).await.unwrap();
    assert_eq!(updated.name, "Widget 2");
    assert_eq!(updated.redirect_uris.len(), 2);
    assert_eq!(updated.scopes, ScopeSet::of(&[Scope::CatalogRead]));
    assert_eq!(updated.kind, ClientKind::Public);
    assert!(updated.updated_at > app.updated_at);
    assert_eq!(
        audit_actions(&pool, app.id).await,
        ["client_registered", "client_updated"]
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn the_built_in_client_keeps_its_fixed_fields_and_cannot_be_deleted(pool: PgPool) {
    let admin = seed(&pool, BuiltInProfile::Administrator).await;
    let android = built_in(&list_as(&pool, &admin).await.unwrap()).clone();
    let moved = ClientChanges {
        name: android.name.clone(),
        redirect_uris: vec!["com.example.evil:/callback".into()],
        scopes: android.scopes,
    };
    assert_eq!(
        update_as(&pool, &admin, android.id, moved)
            .await
            .unwrap_err(),
        OpError::conflict(BUILT_IN_FIXED)
    );
    assert_eq!(
        delete_as(&pool, &admin, android.id).await.unwrap_err(),
        OpError::conflict(BUILT_IN_KEPT)
    );
    assert_eq!(
        replace_secret_as(&pool, &admin, android.id)
            .await
            .unwrap_err(),
        OpError::conflict(PUBLIC_NO_SECRET)
    );
    // Its name and scopes may change; its id, kind and URIs stay.
    let renamed = ClientChanges {
        name: "Horae mobile".into(),
        redirect_uris: android.redirect_uris.clone(),
        scopes: ScopeSet::of(&[Scope::TimeRead]),
    };
    let updated = update_as(&pool, &admin, android.id, renamed).await.unwrap();
    assert_eq!(updated.client_id, BUILT_IN_CLIENT_ID);
    assert_eq!(updated.name, "Horae mobile");
    let enabled = set_enabled_as(&pool, &admin, android.id, true)
        .await
        .unwrap();
    assert!(enabled.enabled);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn disabling_keeps_the_authorizations(pool: PgPool) {
    let admin = seed(&pool, BuiltInProfile::Administrator).await;
    let (app, _) = register_as(&pool, &admin, public_app("Widget"))
        .await
        .unwrap();
    grant(&pool, &admin, app.id).await;
    let disabled = set_enabled_as(&pool, &admin, app.id, false).await.unwrap();
    assert!(!disabled.enabled);
    assert_eq!(disabled.active_grants, 1);
    // Setting the same state again changes and records nothing.
    set_enabled_as(&pool, &admin, app.id, false).await.unwrap();
    let enabled = set_enabled_as(&pool, &admin, app.id, true).await.unwrap();
    assert!(enabled.enabled);
    assert_eq!(enabled.active_grants, 1);
    assert_eq!(
        audit_actions(&pool, app.id).await,
        ["client_registered", "client_disabled", "client_enabled"]
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn deleting_removes_the_app_with_its_authorizations_and_tokens(pool: PgPool) {
    let admin = seed(&pool, BuiltInProfile::Administrator).await;
    let (app, _) = register_as(&pool, &admin, public_app("Widget"))
        .await
        .unwrap();
    let grant_id = grant(&pool, &admin, app.id).await;
    delete_as(&pool, &admin, app.id).await.unwrap();
    let left = sqlx::query_scalar!(
        r#"SELECT (SELECT count(*) FROM oauth_clients WHERE id = $1)
                + (SELECT count(*) FROM oauth_grants WHERE id = $2)
                + (SELECT count(*) FROM oauth_access_tokens WHERE grant_id = $2)
                + (SELECT count(*) FROM oauth_refresh_tokens WHERE grant_id = $2)
                + (SELECT count(*) FROM oauth_codes WHERE grant_id = $2) AS "left!""#,
        app.id,
        grant_id,
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(left, 0);
    assert_eq!(
        audit_actions(&pool, app.id).await,
        ["client_registered", "client_deleted"]
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn replacing_a_secret_ends_the_old_one(pool: PgPool) {
    let admin = seed(&pool, BuiltInProfile::Administrator).await;
    let (app, first) = register_as(&pool, &admin, confidential_app("Billing sync"))
        .await
        .unwrap();
    let first = first.unwrap();
    let (updated, second) = replace_secret_as(&pool, &admin, app.id).await.unwrap();
    assert_ne!(first.expose(), second.expose());
    assert_eq!(stored_secret(&pool, app.id).await, Some(second.digest()));
    assert_eq!(
        updated.secret_prefix.as_deref(),
        Some(second.display_prefix())
    );
    let (public, _) = register_as(&pool, &admin, public_app("Widget"))
        .await
        .unwrap();
    assert_eq!(
        replace_secret_as(&pool, &admin, public.id)
            .await
            .unwrap_err(),
        OpError::conflict(PUBLIC_NO_SECRET)
    );
    assert_eq!(
        audit_actions(&pool, app.id).await,
        ["client_registered", "client_secret_replaced"]
    );
}
