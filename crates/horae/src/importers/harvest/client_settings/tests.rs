use super::*;
use crate::config::{HarvestClient, JobPolicy};
use crate::jobs::{self, JobPayload};
use horae_core::importers::harvest::types::{ImportMode, SyncScope};

const KEY: &str = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";
const OTHER_KEY: &str = "ff0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";
const URL: &str = "https://horae.example.com/auth/harvest/callback";

async fn organization(pool: &PgPool) -> Uuid {
    let org = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO organizations (id, name) VALUES ($1, 'Client settings test')",
        org
    )
    .execute(pool)
    .await
    .unwrap();
    // The administrator shares the organization's id, as in the credential tests.
    sqlx::query!(
        "INSERT INTO users (id, org_id, email, name, org_role) VALUES ($1, $1, $2, 'Client admin', 'admin')",
        org,
        format!("{org}@test.com"),
    )
    .execute(pool)
    .await
    .unwrap();
    org
}

fn key_only() -> HarvestDeployment {
    HarvestDeployment {
        encryption_key_hex: Some(KEY.into()),
        ..Default::default()
    }
}

fn environment() -> HarvestDeployment {
    HarvestDeployment {
        client: Some(HarvestClient {
            client_id: "env-id".into(),
            client_secret: "env-secret".into(),
        }),
        redirect_url: Some("https://env.example.com/auth/harvest/callback".into()),
        encryption_key_hex: Some(KEY.into()),
    }
}

async fn save_client(
    pool: &PgPool,
    deployment: &HarvestDeployment,
    org: Uuid,
    id: &str,
    secret: Option<&str>,
) -> anyhow::Result<()> {
    // The page confirms any disconnect before it sends this.
    save(pool, deployment, org, org, id, secret, URL, true).await
}

async fn connect(pool: &PgPool, org: Uuid) {
    credentials::store(
        pool, org, org, KEY, "account", "access", "refresh", None, None,
    )
    .await
    .unwrap();
}

async fn revision(pool: &PgPool, org: Uuid) -> i64 {
    account_switch::status(pool, org, true)
        .await
        .unwrap()
        .connection_revision
}

fn client_error(error: &anyhow::Error) -> Option<&ClientError> {
    error.downcast_ref()
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn stored_client_applies_without_restart_and_seals_the_secret(pool: PgPool) {
    let org = organization(&pool).await;
    let deployment = key_only();
    assert!(resolve(&pool, &deployment, org).await.unwrap().is_none());

    save_client(&pool, &deployment, org, "stored-id", Some("stored-secret"))
        .await
        .unwrap();

    let cfg = resolve(&pool, &deployment, org).await.unwrap().unwrap();
    assert_eq!(
        (
            cfg.client_id.as_str(),
            cfg.client_secret.as_str(),
            cfg.redirect_url.as_str()
        ),
        ("stored-id", "stored-secret", URL)
    );
    let sealed = sqlx::query_scalar!(
        "SELECT client_secret_enc FROM harvest_oauth_clients WHERE org_id = $1",
        org
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(!sealed.windows(13).any(|w| w == b"stored-secret"));
    assert_eq!(credentials::decrypt(KEY, &sealed).unwrap(), "stored-secret");
    assert!(credentials::decrypt(OTHER_KEY, &sealed).is_err());
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn view_reports_the_stored_client_without_its_secret(pool: PgPool) {
    let org = organization(&pool).await;
    let deployment = key_only();
    let empty = view(&pool, &deployment, org).await.unwrap();
    assert_eq!(empty.source, ClientSource::None);
    assert!(empty.editable());

    save_client(&pool, &deployment, org, "stored-id", Some("stored-secret"))
        .await
        .unwrap();
    let settings = view(&pool, &deployment, org).await.unwrap();
    assert_eq!(settings.source, ClientSource::Stored);
    assert_eq!(settings.client_id.as_deref(), Some("stored-id"));
    assert!(settings.secret_set && settings.secret_readable);
    assert_eq!(settings.redirect_url.as_deref(), Some(URL));
    assert!(settings.stored_at.is_some());
    let json = serde_json::to_string(&settings).unwrap();
    assert!(!json.contains("stored-secret"), "{json}");
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn environment_client_wins_and_locks_the_stored_one(pool: PgPool) {
    let org = organization(&pool).await;
    save_client(&pool, &key_only(), org, "stored-id", Some("stored-secret"))
        .await
        .unwrap();
    let deployment = environment();

    let cfg = resolve(&pool, &deployment, org).await.unwrap().unwrap();
    assert_eq!(
        (cfg.client_id.as_str(), cfg.client_secret.as_str()),
        ("env-id", "env-secret")
    );
    assert_eq!(
        cfg.redirect_url,
        "https://env.example.com/auth/harvest/callback"
    );

    let settings = view(&pool, &deployment, org).await.unwrap();
    assert_eq!(settings.source, ClientSource::Environment);
    assert!(settings.stored_ignored && settings.redirect_from_environment);
    assert!(!settings.editable());
    assert!(
        !serde_json::to_string(&settings)
            .unwrap()
            .contains("env-secret")
    );

    for error in [
        save_client(&pool, &deployment, org, "new-id", Some("new"))
            .await
            .unwrap_err(),
        remove(&pool, &deployment, org, org, true)
            .await
            .unwrap_err(),
    ] {
        assert!(
            matches!(client_error(&error), Some(ClientError::Environment)),
            "{error}"
        );
    }
    assert_eq!(
        resolve(&pool, &key_only(), org)
            .await
            .unwrap()
            .unwrap()
            .client_id,
        "stored-id"
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn environment_redirect_url_wins_over_the_stored_client(pool: PgPool) {
    let org = organization(&pool).await;
    let deployment = HarvestDeployment {
        redirect_url: Some("https://env.example.com/auth/harvest/callback".into()),
        ..key_only()
    };
    save(
        &pool,
        &deployment,
        org,
        org,
        "stored-id",
        Some("s"),
        "ignored",
        false,
    )
    .await
    .unwrap();
    let cfg = resolve(&pool, &deployment, org).await.unwrap().unwrap();
    assert_eq!(cfg.client_id, "stored-id");
    assert_eq!(
        cfg.redirect_url,
        "https://env.example.com/auth/harvest/callback"
    );
    // The row keeps no redirect URL of its own, so dropping the environment
    // value leaves the client unconfigured rather than pointing elsewhere.
    assert!(resolve(&pool, &key_only(), org).await.unwrap().is_none());
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn missing_key_disables_the_client_and_refuses_saves(pool: PgPool) {
    let org = organization(&pool).await;
    save_client(&pool, &key_only(), org, "stored-id", Some("s"))
        .await
        .unwrap();
    let no_key = HarvestDeployment::default();
    assert!(resolve(&pool, &no_key, org).await.unwrap().is_none());
    let settings = view(&pool, &no_key, org).await.unwrap();
    assert!(!settings.encryption_key_set && !settings.editable());
    let error = save_client(&pool, &no_key, org, "x", Some("y"))
        .await
        .unwrap_err();
    assert!(matches!(
        client_error(&error),
        Some(ClientError::NoEncryptionKey)
    ));
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn rotated_key_requires_the_secret_again(pool: PgPool) {
    let org = organization(&pool).await;
    save_client(&pool, &key_only(), org, "stored-id", Some("old"))
        .await
        .unwrap();
    let rotated = HarvestDeployment {
        encryption_key_hex: Some(OTHER_KEY.into()),
        ..Default::default()
    };
    assert!(resolve(&pool, &rotated, org).await.unwrap().is_none());
    assert!(!view(&pool, &rotated, org).await.unwrap().secret_readable);
    let error = save_client(&pool, &rotated, org, "stored-id", None)
        .await
        .unwrap_err();
    assert!(matches!(
        client_error(&error),
        Some(ClientError::Invalid(InvalidClient::SecretRequired))
    ));
    save_client(&pool, &rotated, org, "stored-id", Some("new"))
        .await
        .unwrap();
    assert_eq!(
        resolve(&pool, &rotated, org)
            .await
            .unwrap()
            .unwrap()
            .client_secret,
        "new"
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn replacing_the_secret_keeps_the_connection(pool: PgPool) {
    let org = organization(&pool).await;
    let deployment = key_only();
    save_client(&pool, &deployment, org, "stored-id", Some("old"))
        .await
        .unwrap();
    connect(&pool, org).await;

    save_client(&pool, &deployment, org, "stored-id", Some("new"))
        .await
        .unwrap();
    save_client(&pool, &deployment, org, "stored-id", None)
        .await
        .unwrap();

    assert!(credentials::load(&pool, org, KEY).await.unwrap().is_some());
    assert_eq!(
        resolve(&pool, &deployment, org)
            .await
            .unwrap()
            .unwrap()
            .client_secret,
        "new"
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn new_client_id_disconnects_but_keeps_the_account_binding(pool: PgPool) {
    let org = organization(&pool).await;
    let deployment = key_only();
    save_client(&pool, &deployment, org, "first", Some("s"))
        .await
        .unwrap();
    connect(&pool, org).await;
    let before = account_switch::status(&pool, org, true).await.unwrap();

    let error = save_client(&pool, &deployment, org, "second", None)
        .await
        .unwrap_err();
    assert!(matches!(
        client_error(&error),
        Some(ClientError::Invalid(InvalidClient::SecretRequired))
    ));
    assert!(credentials::load(&pool, org, KEY).await.unwrap().is_some());

    save_client(&pool, &deployment, org, "second", Some("s2"))
        .await
        .unwrap();
    let after = account_switch::status(&pool, org, true).await.unwrap();
    assert!(!after.connected);
    assert_eq!(after.account_id.as_deref(), Some("account"));
    assert_eq!(after.account_generation, before.account_generation);
    assert!(after.connection_revision > before.connection_revision);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn removal_disconnects_and_unconfigures(pool: PgPool) {
    let org = organization(&pool).await;
    let deployment = key_only();
    remove(&pool, &deployment, org, org, false).await.unwrap();
    assert_eq!(revision(&pool, org).await, 0);

    save_client(&pool, &deployment, org, "stored-id", Some("s"))
        .await
        .unwrap();
    connect(&pool, org).await;
    remove(&pool, &deployment, org, org, true).await.unwrap();

    assert!(resolve(&pool, &deployment, org).await.unwrap().is_none());
    let status = account_switch::status(&pool, org, true).await.unwrap();
    assert!(!status.connected);
    assert_eq!(status.account_id.as_deref(), Some("account"));
    let audit: Vec<(String, serde_json::Value)> = sqlx::query!(
        "SELECT action, data FROM audit_log WHERE org_id = $1 ORDER BY id",
        org
    )
    .fetch_all(&pool)
    .await
    .unwrap()
    .into_iter()
    .map(|row| (row.action, row.data.unwrap()))
    .collect();
    assert_eq!(
        audit,
        [
            (
                "harvest.oauth_client.save".to_owned(),
                serde_json::json!({ "client_id": "stored-id", "disconnected": false })
            ),
            (
                "harvest.oauth_client.remove".to_owned(),
                serde_json::json!({ "client_id": "stored-id", "disconnected": true })
            ),
        ]
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn active_imports_and_the_import_lock_refuse_changes(pool: PgPool) {
    let org = organization(&pool).await;
    let deployment = key_only();
    save_client(&pool, &deployment, org, "stored-id", Some("s"))
        .await
        .unwrap();
    connect(&pool, org).await;
    let before = revision(&pool, org).await;

    let payload = JobPayload::HarvestApi {
        mode: ImportMode::DryRun,
        sync: SyncScope::Full,
    };
    let job = jobs::enqueue_api(&pool, org, &payload, "queued", JobPolicy::default(), 0)
        .await
        .unwrap();
    for error in [
        save_client(&pool, &deployment, org, "other", Some("s"))
            .await
            .unwrap_err(),
        remove(&pool, &deployment, org, org, true)
            .await
            .unwrap_err(),
    ] {
        assert!(error.to_string().contains("Finish or cancel"), "{error}");
    }
    assert!(jobs::cancel(&pool, org, job).await.unwrap());

    let held = super::super::lock_import(&pool, org).await.unwrap();
    let error = save_client(&pool, &deployment, org, "other", Some("s"))
        .await
        .unwrap_err();
    assert!(matches!(
        error.downcast_ref(),
        Some(super::super::ApiImportError::Busy)
    ));
    super::super::release_import(held).await.unwrap();

    assert_eq!(revision(&pool, org).await, before);
    assert_eq!(
        resolve(&pool, &deployment, org)
            .await
            .unwrap()
            .unwrap()
            .client_id,
        "stored-id"
    );
    assert!(credentials::load(&pool, org, KEY).await.unwrap().is_some());
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn authorization_started_before_a_change_cannot_store_tokens(pool: PgPool) {
    let org = organization(&pool).await;
    let deployment = key_only();
    save_client(&pool, &deployment, org, "stored-id", Some("s"))
        .await
        .unwrap();
    let started = account_switch::status(&pool, org, true).await.unwrap();
    save_client(&pool, &deployment, org, "stored-id", Some("rotated"))
        .await
        .unwrap();

    let error = credentials::store_for_attempt(
        &pool,
        org,
        org,
        KEY,
        "account",
        "access",
        "refresh",
        None,
        None,
        account_switch::Version {
            account_generation: started.account_generation,
            connection_revision: started.connection_revision,
        },
    )
    .await
    .unwrap_err();
    assert!(matches!(
        error.downcast_ref(),
        Some(account_switch::ChangeError::Stale)
    ));
    assert!(credentials::load(&pool, org, KEY).await.unwrap().is_none());
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn only_an_active_administrator_of_the_organization_may_change_it(pool: PgPool) {
    let org = organization(&pool).await;
    let other = organization(&pool).await;
    let error = save(&pool, &key_only(), org, other, "id", Some("s"), URL, false)
        .await
        .unwrap_err();
    assert!(matches!(
        error.downcast_ref(),
        Some(credentials::ConnectionError::Unauthorized)
    ));
    assert!(
        view(&pool, &key_only(), org)
            .await
            .unwrap()
            .client_id
            .is_none()
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn disconnecting_needs_the_callers_confirmation(pool: PgPool) {
    let org = organization(&pool).await;
    let deployment = key_only();
    save(&pool, &deployment, org, org, "first", Some("s"), URL, false)
        .await
        .unwrap();
    // Without a connection there is nothing to confirm.
    save(
        &pool,
        &deployment,
        org,
        org,
        "second",
        Some("s"),
        URL,
        false,
    )
    .await
    .unwrap();
    connect(&pool, org).await;
    let before = revision(&pool, org).await;

    // A page loaded before the connection existed never showed the warning.
    for error in [
        save(&pool, &deployment, org, org, "third", Some("s"), URL, false)
            .await
            .unwrap_err(),
        remove(&pool, &deployment, org, org, false)
            .await
            .unwrap_err(),
    ] {
        assert!(
            matches!(
                client_error(&error),
                Some(ClientError::DisconnectUnconfirmed)
            ),
            "{error}"
        );
    }
    assert_eq!(revision(&pool, org).await, before);
    assert!(credentials::load(&pool, org, KEY).await.unwrap().is_some());
    assert_eq!(
        resolve(&pool, &deployment, org)
            .await
            .unwrap()
            .unwrap()
            .client_id,
        "second"
    );

    // Keeping the client ID never disconnects, so it needs no confirmation.
    save(
        &pool,
        &deployment,
        org,
        org,
        "second",
        Some("rotated"),
        URL,
        false,
    )
    .await
    .unwrap();
    assert!(credentials::load(&pool, org, KEY).await.unwrap().is_some());
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_client_can_be_removed_without_the_encryption_key(pool: PgPool) {
    let org = organization(&pool).await;
    save_client(&pool, &key_only(), org, "stored-id", Some("s"))
        .await
        .unwrap();
    let no_key = HarvestDeployment::default();
    assert!(view(&pool, &no_key, org).await.unwrap().source == ClientSource::Stored);
    remove(&pool, &no_key, org, org, false).await.unwrap();
    assert_eq!(
        view(&pool, &no_key, org).await.unwrap().source,
        ClientSource::None
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_client_saved_under_the_environment_redirect_url_asks_for_its_own(pool: PgPool) {
    let org = organization(&pool).await;
    let with_redirect = HarvestDeployment {
        redirect_url: Some("https://env.example.com/auth/harvest/callback".into()),
        ..key_only()
    };
    save(
        &pool,
        &with_redirect,
        org,
        org,
        "stored-id",
        Some("s"),
        "",
        false,
    )
    .await
    .unwrap();
    assert!(
        !view(&pool, &with_redirect, org)
            .await
            .unwrap()
            .needs_redirect()
    );

    // The variable is unset later: the client stops resolving, and says why.
    assert!(resolve(&pool, &key_only(), org).await.unwrap().is_none());
    assert!(
        view(&pool, &key_only(), org)
            .await
            .unwrap()
            .needs_redirect()
    );
    save_client(&pool, &key_only(), org, "stored-id", None)
        .await
        .unwrap();
    assert_eq!(
        resolve(&pool, &key_only(), org)
            .await
            .unwrap()
            .unwrap()
            .redirect_url,
        URL
    );
}
