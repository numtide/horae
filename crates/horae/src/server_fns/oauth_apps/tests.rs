//! The app administration server functions, through the adapters their
//! `#[server]` bodies call after reading the session. Who may do what is
//! tested in `ops::oauth`; these check the answers and their status codes.

use super::*;
use crate::models::oauth_apps::AppKind;
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

fn field(error: &ServerFnError) -> Option<String> {
    match error {
        ServerFnError::ServerError { details, .. } => {
            details.as_ref()?.get("field")?.as_str().map(str::to_owned)
        }
        _ => None,
    }
}

fn app(name: &str, kind: AppKind, redirect: &str) -> AppInput {
    AppInput {
        name: name.into(),
        kind,
        redirect_uris: vec![redirect.into()],
        scopes: vec!["time:read".into(), "catalog:read".into()],
    }
}

fn server_app(name: &str) -> AppInput {
    app(
        name,
        AppKind::Confidential,
        "https://sync.example.com/callback",
    )
}

fn device_app(name: &str) -> AppInput {
    app(name, AppKind::Public, "com.example.timer:/callback")
}

fn built_in(apps: &[AppRow]) -> &AppRow {
    apps.iter().find(|app| app.built_in).unwrap()
}

/// An active authorization of `person` for `app`, as an approval leaves it.
async fn authorize(pool: &PgPool, person: &SeedIds, app: Uuid) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO oauth_grants (id, org_id, client_id, user_id, scopes, expires_at)
         SELECT $1, org_id, id, $3, scopes, now() + interval '1 year'
         FROM oauth_clients WHERE id = $2",
        id,
        app,
        person.user_id,
    )
    .execute(pool)
    .await
    .unwrap();
    id
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_member_is_refused_every_app_function(pool: PgPool) {
    let admin = seed(&pool, BuiltInProfile::Administrator).await;
    let member = add_person(&pool, &admin, BuiltInProfile::Member).await;
    let created = create(&pool, caller(&admin), server_app("Invoice sync"))
        .await
        .unwrap();
    let id = created.app.id;
    let grant = authorize(&pool, &admin, id).await;
    // Authorization comes before validation: an unknown scope is refused
    // as forbidden too.
    let mut unknown_scope = device_app("Timer");
    unknown_scope.scopes = vec!["invoices:write".into()];

    let refused = [
        list(&pool, caller(&member)).await.map(drop),
        get(&pool, caller(&member), id).await.map(drop),
        create(&pool, caller(&member), device_app("Timer"))
            .await
            .map(drop),
        update(&pool, caller(&member), id, server_app("Renamed"))
            .await
            .map(drop),
        set_enabled(&pool, caller(&member), id, false)
            .await
            .map(drop),
        replace_secret(&pool, caller(&member), id).await.map(drop),
        delete(&pool, caller(&member), id).await,
        list_grants(&pool, caller(&member), id).await.map(drop),
        create(&pool, caller(&member), unknown_scope.clone())
            .await
            .map(drop),
        update(&pool, caller(&member), id, unknown_scope)
            .await
            .map(drop),
    ];
    for result in refused {
        assert_eq!(code(&result.unwrap_err()), FORBIDDEN);
    }
    // Someone else's authorization is not the member's to see.
    let error = revoke_grant(&pool, caller(&member), grant)
        .await
        .unwrap_err();
    assert_eq!(code(&error), NOT_FOUND);
    assert_eq!(
        list_grants(&pool, caller(&admin), id).await.unwrap().len(),
        1
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_server_app_gets_its_secret_once_and_a_device_app_none(pool: PgPool) {
    let admin = seed(&pool, BuiltInProfile::Administrator).await;
    let server = create(&pool, caller(&admin), server_app("Invoice sync"))
        .await
        .unwrap();
    let secret = server.secret.clone().unwrap();
    assert!(secret.starts_with("horae_ocs_"));
    assert_eq!(
        server.app.secret_prefix.as_deref(),
        Some(&secret[10..18]),
        "the prefix tells the secret apart"
    );
    assert!(server.app.client_id.starts_with("horae_app_"));
    assert_eq!(server.app.kind, AppKind::Confidential);
    assert_eq!(server.app.scopes, ["time:read", "catalog:read"]);

    let device = create(&pool, caller(&admin), device_app("Timer"))
        .await
        .unwrap();
    assert_eq!(device.secret, None);

    let apps = list(&pool, caller(&admin)).await.unwrap();
    assert!(apps[0].built_in, "Horae's own app comes first");
    assert_eq!(apps.len(), 3);
    assert!(
        !serde_json::to_string(&apps)
            .unwrap()
            .contains(&secret[18..])
    );
    assert_eq!(
        get(&pool, caller(&admin), server.app.id).await.unwrap(),
        server.app
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn invalid_input_is_a_bad_request_naming_the_field(pool: PgPool) {
    let admin = seed(&pool, BuiltInProfile::Administrator).await;
    let fragment = app(
        "Sync",
        AppKind::Confidential,
        "https://sync.example.com/cb#x",
    );
    let error = create(&pool, caller(&admin), fragment).await.unwrap_err();
    assert_eq!(code(&error), BAD_REQUEST);
    assert_eq!(field(&error).as_deref(), Some("redirect_uris"));

    let mut unknown_scope = device_app("Timer");
    unknown_scope.scopes = vec!["invoices:write".into()];
    let error = create(&pool, caller(&admin), unknown_scope)
        .await
        .unwrap_err();
    assert_eq!(code(&error), BAD_REQUEST);
    assert_eq!(field(&error).as_deref(), Some("scopes"));

    let error = create(&pool, caller(&admin), device_app(" "))
        .await
        .unwrap_err();
    assert_eq!(field(&error).as_deref(), Some("name"));

    create(&pool, caller(&admin), device_app("Timer"))
        .await
        .unwrap();
    let error = create(&pool, caller(&admin), server_app("Timer"))
        .await
        .unwrap_err();
    assert_eq!(code(&error), CONFLICT);
    assert_eq!(field(&error).as_deref(), Some("name"), "a taken name");
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn the_built_in_app_can_only_be_enabled_or_disabled(pool: PgPool) {
    let admin = seed(&pool, BuiltInProfile::Administrator).await;
    let apps = list(&pool, caller(&admin)).await.unwrap();
    let android = built_in(&apps).clone();
    assert!(!android.enabled, "it starts disabled");

    let error = delete(&pool, caller(&admin), android.id).await.unwrap_err();
    assert_eq!(code(&error), CONFLICT);
    let unchanged = AppInput {
        name: android.name.clone(),
        kind: android.kind,
        redirect_uris: android.redirect_uris.clone(),
        scopes: android.scopes.clone(),
    };
    let mut moved = unchanged.clone();
    moved.redirect_uris = vec!["com.example.timer:/callback".into()];
    let mut renamed = unchanged.clone();
    renamed.name = "Horae mobile".into();
    let mut narrowed = unchanged;
    narrowed.scopes = vec!["time:read".into()];
    for changes in [moved, renamed, narrowed] {
        let error = update(&pool, caller(&admin), android.id, changes)
            .await
            .unwrap_err();
        assert_eq!(code(&error), CONFLICT);
    }
    assert_eq!(
        get(&pool, caller(&admin), android.id).await.unwrap(),
        android
    );
    let error = replace_secret(&pool, caller(&admin), android.id)
        .await
        .unwrap_err();
    assert_eq!(code(&error), CONFLICT, "a device app has no secret");

    let enabled = set_enabled(&pool, caller(&admin), android.id, true)
        .await
        .unwrap();
    assert!(enabled.enabled);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn editing_and_replacing_the_secret_answer_the_new_state(pool: PgPool) {
    let admin = seed(&pool, BuiltInProfile::Administrator).await;
    let created = create(&pool, caller(&admin), server_app("Invoice sync"))
        .await
        .unwrap();
    let mut changes = server_app("Invoice export");
    changes.redirect_uris = vec![
        "https://sync.example.com/callback".into(),
        "https://sync.example.com/callback/2".into(),
    ];
    let updated = update(&pool, caller(&admin), created.app.id, changes)
        .await
        .unwrap();
    assert_eq!(updated.name, "Invoice export");
    assert_eq!(updated.redirect_uris.len(), 2);

    let replaced = replace_secret(&pool, caller(&admin), created.app.id)
        .await
        .unwrap();
    let secret = replaced.secret.unwrap();
    assert_ne!(Some(secret), created.secret);
    assert_ne!(replaced.app.secret_prefix, created.app.secret_prefix);

    delete(&pool, caller(&admin), created.app.id).await.unwrap();
    let error = get(&pool, caller(&admin), created.app.id)
        .await
        .unwrap_err();
    assert_eq!(code(&error), NOT_FOUND);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn an_administrator_lists_and_revokes_anyones_authorization(pool: PgPool) {
    let admin = seed(&pool, BuiltInProfile::Administrator).await;
    let member = add_person(&pool, &admin, BuiltInProfile::Member).await;
    let created = create(&pool, caller(&admin), device_app("Timer"))
        .await
        .unwrap();
    let grant = authorize(&pool, &member, created.app.id).await;

    let grants = list_grants(&pool, caller(&admin), created.app.id)
        .await
        .unwrap();
    assert_eq!(grants.len(), 1);
    assert_eq!(grants[0].id, grant);
    assert_eq!(grants[0].person_id, member.user_id);
    assert_eq!(grants[0].person_name, "Teammate");
    assert!(!grants[0].person_email.is_empty());
    let app = get(&pool, caller(&admin), created.app.id).await.unwrap();
    assert_eq!(app.active_grants, 1);

    revoke_grant(&pool, caller(&admin), grant).await.unwrap();
    assert!(
        list_grants(&pool, caller(&admin), created.app.id)
            .await
            .unwrap()
            .is_empty()
    );
    let error = list_grants(&pool, caller(&admin), Uuid::now_v7())
        .await
        .unwrap_err();
    assert_eq!(code(&error), NOT_FOUND);
}
