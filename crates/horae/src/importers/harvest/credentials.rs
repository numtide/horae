//! Harvest OAuth credentials at rest: AEAD encrypt/decrypt of the tokens and
//! load/store of `harvest_credentials`, plus the incremental-sync watermark
//! (FR-022, FR-024, FR-025).
//!
//! Tokens are sealed with XChaCha20-Poly1305 under the deployment-supplied key
//! (a 32-byte hex string in config). The random 24-byte nonce is stored as a
//! prefix of the ciphertext blob. Decrypted tokens live only in memory while a
//! Harvest call is in flight; they are never returned to the browser or logged.

use chacha20poly1305::aead::{Aead, AeadCore, KeyInit, OsRng};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use chrono::{DateTime, Utc};
use horae_core::importers::harvest::types::EntityType;
use sqlx::Acquire;
use uuid::Uuid;

/// Connection policy failures that are safe to display to an administrator.
#[derive(Debug, thiserror::Error)]
pub enum ConnectionError {
    #[error(
        "This organization is bound to another Harvest account. Reconnect the original account; changing accounts requires an explicit data migration."
    )]
    AccountChange,
    #[error(
        "Existing Harvest import identities have no verified account. Ask the operator to verify and bind the original account before reconnecting."
    )]
    UnidentifiedProvenance,
}

/// A decrypted Harvest connection for one org (in-memory only).
#[derive(Debug, Clone)]
pub struct HarvestConnection {
    pub account_id: String,
    pub access_token: String,
    pub refresh_token: String,
    pub token_expires_at: Option<DateTime<Utc>>,
    /// Per-entity `updated_since` high-water marks (RFC3339 strings).
    pub watermark: serde_json::Value,
}

impl HarvestConnection {
    /// The stored watermark for one entity type, if any.
    pub fn watermark_for(&self, entity: EntityType) -> Option<DateTime<Utc>> {
        self.watermark
            .get(entity.as_str())
            .and_then(|v| v.as_str())
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc))
    }
}

/// Build the AEAD cipher from the 64-char hex key in config.
fn cipher(key_hex: &str) -> anyhow::Result<XChaCha20Poly1305> {
    let key = decode_hex(key_hex)?;
    if key.len() != 32 {
        anyhow::bail!("HORAE_HARVEST_ENC_KEY must be 32 bytes (64 hex chars)");
    }
    XChaCha20Poly1305::new_from_slice(&key).map_err(|e| anyhow::anyhow!("invalid AEAD key: {e}"))
}

/// Encrypt a token: `nonce (24 bytes) || ciphertext`.
pub fn encrypt(key_hex: &str, plaintext: &str) -> anyhow::Result<Vec<u8>> {
    let c = cipher(key_hex)?;
    let nonce = XChaCha20Poly1305::generate_nonce(&mut OsRng);
    let ciphertext = c
        .encrypt(&nonce, plaintext.as_bytes())
        .map_err(|e| anyhow::anyhow!("token encryption failed: {e}"))?;
    let mut blob = Vec::with_capacity(nonce.len() + ciphertext.len());
    blob.extend_from_slice(&nonce);
    blob.extend_from_slice(&ciphertext);
    Ok(blob)
}

/// Decrypt a `nonce || ciphertext` blob back to the token string.
pub fn decrypt(key_hex: &str, blob: &[u8]) -> anyhow::Result<String> {
    const NONCE_LEN: usize = 24;
    if blob.len() < NONCE_LEN {
        anyhow::bail!("ciphertext too short");
    }
    let (nonce_bytes, ciphertext) = blob.split_at(NONCE_LEN);
    let c = cipher(key_hex)?;
    let nonce = XNonce::from_slice(nonce_bytes);
    let plaintext = c
        .decrypt(nonce, ciphertext)
        .map_err(|_| anyhow::anyhow!("token decryption failed (key rotated? reconnect Harvest)"))?;
    String::from_utf8(plaintext).map_err(|e| anyhow::anyhow!("token not valid UTF-8: {e}"))
}

/// Load and decrypt the org's Harvest connection, if it exists.
pub async fn load<'e, E>(
    exec: E,
    org_id: Uuid,
    key_hex: &str,
) -> anyhow::Result<Option<HarvestConnection>>
where
    E: sqlx::PgExecutor<'e>,
{
    let row = sqlx::query!(
        r#"SELECT harvest_account_id, access_token_enc, refresh_token_enc,
                  token_expires_at as "token_expires_at: DateTime<Utc>",
                  synced_watermark
           FROM harvest_credentials WHERE org_id = $1"#,
        org_id,
    )
    .fetch_optional(exec)
    .await?;

    let Some(row) = row else {
        return Ok(None);
    };
    Ok(Some(HarvestConnection {
        account_id: row.harvest_account_id,
        access_token: decrypt(key_hex, &row.access_token_enc)?,
        refresh_token: decrypt(key_hex, &row.refresh_token_enc)?,
        token_expires_at: row.token_expires_at,
        watermark: row.synced_watermark,
    }))
}

/// Upsert the org's Harvest connection, encrypting the tokens. One row per org
/// (v1); reconnecting the same account overwrites only its credentials. The
/// parameters mirror the persisted columns one-to-one, hence the count.
#[allow(clippy::too_many_arguments)]
pub async fn store(
    pool: &sqlx::PgPool,
    org_id: Uuid,
    key_hex: &str,
    account_id: &str,
    access_token: &str,
    refresh_token: &str,
    token_expires_at: Option<DateTime<Utc>>,
    scope: Option<&str>,
) -> anyhow::Result<()> {
    let access_enc = encrypt(key_hex, access_token)?;
    let refresh_enc = encrypt(key_hex, refresh_token)?;
    let mut connection = super::lock_import(pool, org_id).await?;
    let result = async {
        let mut tx = connection.begin().await?;
        let bound_account = sqlx::query_scalar!(
            "SELECT harvest_account_id FROM harvest_account_bindings WHERE org_id = $1",
            org_id,
        )
        .fetch_optional(&mut *tx)
        .await?;
        match bound_account {
            Some(bound) if bound != account_id => return Err(ConnectionError::AccountChange.into()),
            Some(_) => {}
            None => {
                let has_provenance = sqlx::query_scalar!(
                r#"SELECT EXISTS(SELECT 1 FROM harvest_import_map WHERE org_id = $1) AS "exists!""#,
                org_id,
            )
            .fetch_one(&mut *tx)
            .await?;
                if has_provenance {
                    return Err(ConnectionError::UnidentifiedProvenance.into());
                }
                sqlx::query!(
                "INSERT INTO harvest_account_bindings (org_id, harvest_account_id) VALUES ($1, $2)",
                org_id,
                account_id,
            )
            .execute(&mut *tx)
            .await?;
            }
        }
        let id = Uuid::now_v7();
        let saved = sqlx::query!(
            r#"INSERT INTO harvest_credentials
             (id, org_id, harvest_account_id, access_token_enc, refresh_token_enc,
              token_expires_at, scope)
           VALUES ($1, $2, $3, $4, $5, $6, $7)
           ON CONFLICT (org_id) DO UPDATE SET
             access_token_enc   = EXCLUDED.access_token_enc,
             refresh_token_enc  = EXCLUDED.refresh_token_enc,
             token_expires_at   = EXCLUDED.token_expires_at,
             scope              = EXCLUDED.scope
           WHERE harvest_credentials.harvest_account_id = EXCLUDED.harvest_account_id"#,
            id,
            org_id,
            account_id,
            access_enc,
            refresh_enc,
            token_expires_at as Option<chrono::DateTime<chrono::Utc>>,
            scope,
        )
        .execute(&mut *tx)
        .await?;
        if saved.rows_affected() != 1 {
            return Err(ConnectionError::AccountChange.into());
        }
        tx.commit().await?;
        Ok(())
    }
    .await;
    super::release_import(connection).await?;
    result
}

/// Remove OAuth secrets, retaining account identity and all imported records.
/// Like connecting, this must not race with import or token refresh.
pub async fn disconnect(pool: &sqlx::PgPool, org_id: Uuid) -> Result<(), super::ApiImportError> {
    let mut connection = super::lock_import(pool, org_id).await?;
    let result = sqlx::query!("DELETE FROM harvest_credentials WHERE org_id = $1", org_id)
        .execute(&mut *connection)
        .await
        .map_err(anyhow::Error::from);
    super::release_import(connection).await?;
    result?;
    Ok(())
}

/// Persist refreshed access/refresh tokens after a transparent token refresh
/// (FR-024), without touching the account id or watermark.
pub async fn update_tokens<'e, E>(
    exec: E,
    org_id: Uuid,
    key_hex: &str,
    access_token: &str,
    refresh_token: &str,
    token_expires_at: Option<DateTime<Utc>>,
) -> anyhow::Result<()>
where
    E: sqlx::PgExecutor<'e>,
{
    let access_enc = encrypt(key_hex, access_token)?;
    let refresh_enc = encrypt(key_hex, refresh_token)?;
    sqlx::query!(
        r#"UPDATE harvest_credentials
           SET access_token_enc = $2, refresh_token_enc = $3,
               token_expires_at = $4
           WHERE org_id = $1"#,
        org_id,
        access_enc,
        refresh_enc,
        token_expires_at as Option<chrono::DateTime<chrono::Utc>>,
    )
    .execute(exec)
    .await?;
    Ok(())
}

/// Advance the per-entity incremental-sync watermark (FR-025). The API importer
/// enlists this write in its data transaction, only for an error-free commit.
pub async fn advance_watermark<'e, E>(
    exec: E,
    org_id: Uuid,
    marks: &[(EntityType, DateTime<Utc>)],
) -> anyhow::Result<()>
where
    E: sqlx::PgExecutor<'e>,
{
    if marks.is_empty() {
        return Ok(());
    }
    let mut obj = serde_json::Map::new();
    for (entity, ts) in marks {
        obj.insert(
            entity.as_str().to_string(),
            serde_json::Value::String(ts.to_rfc3339()),
        );
    }
    let patch = serde_json::Value::Object(obj);
    sqlx::query!(
        r#"UPDATE harvest_credentials
           SET synced_watermark = synced_watermark || $2
           WHERE org_id = $1"#,
        org_id,
        patch,
    )
    .execute(exec)
    .await?;
    Ok(())
}

/// Decode a hex string into bytes.
fn decode_hex(s: &str) -> anyhow::Result<Vec<u8>> {
    let s = s.trim();
    if !s.len().is_multiple_of(2) {
        anyhow::bail!("hex key has an odd number of digits");
    }
    (0..s.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&s[i..i + 2], 16)
                .map_err(|_| anyhow::anyhow!("invalid hex in encryption key"))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn organization(pool: &sqlx::PgPool) -> Uuid {
        let org = Uuid::now_v7();
        sqlx::query!(
            "INSERT INTO organizations (id, name) VALUES ($1, 'Account binding test')",
            org
        )
        .execute(pool)
        .await
        .unwrap();
        org
    }

    async fn connect(pool: &sqlx::PgPool, org: Uuid, account: &str) -> anyhow::Result<()> {
        store(pool, org, KEY, account, "access", "refresh", None, None).await
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn another_account_cannot_replace_credentials_or_watermarks(pool: sqlx::PgPool) {
        let org = organization(&pool).await;
        connect(&pool, org, "original").await.unwrap();
        let mark = Utc::now();
        advance_watermark(&pool, org, &[(EntityType::TimeEntry, mark)])
            .await
            .unwrap();

        assert!(connect(&pool, org, "different").await.is_err());
        let stored = load(&pool, org, KEY).await.unwrap().unwrap();
        assert_eq!(stored.account_id, "original");
        assert_eq!(stored.watermark_for(EntityType::TimeEntry), Some(mark));
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn unidentified_legacy_provenance_requires_operator_review(pool: sqlx::PgPool) {
        let org = organization(&pool).await;
        super::super::provenance::upsert(&pool, org, EntityType::Client, 1, Uuid::now_v7(), None)
            .await
            .unwrap();
        let error = connect(&pool, org, "unknown").await.unwrap_err();
        assert!(matches!(
            error.downcast_ref(),
            Some(ConnectionError::UnidentifiedProvenance)
        ));
        assert!(load(&pool, org, KEY).await.unwrap().is_none());
        // After verifying the legacy source, an operator can restore only the
        // missing identity; no provenance or imported data needs deletion.
        sqlx::query!(
            "INSERT INTO harvest_account_bindings (org_id, harvest_account_id) VALUES ($1, $2)",
            org,
            "verified-original",
        )
        .execute(&pool)
        .await
        .unwrap();
        connect(&pool, org, "verified-original").await.unwrap();
        assert!(connect(&pool, org, "different").await.is_err());
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn disconnect_preserves_identity_and_provenance_for_the_original_account(
        pool: sqlx::PgPool,
    ) {
        let org = organization(&pool).await;
        connect(&pool, org, "original").await.unwrap();
        let mapped = Uuid::now_v7();
        super::super::provenance::upsert(&pool, org, EntityType::Client, 1, mapped, None)
            .await
            .unwrap();
        disconnect(&pool, org).await.unwrap();
        disconnect(&pool, org).await.unwrap();
        assert!(load(&pool, org, KEY).await.unwrap().is_none());
        let error = connect(&pool, org, "different").await.unwrap_err();
        assert!(matches!(
            error.downcast_ref(),
            Some(ConnectionError::AccountChange)
        ));
        connect(&pool, org, "original").await.unwrap();
        assert_eq!(
            super::super::provenance::lookup(&pool, org, EntityType::Client, 1)
                .await
                .unwrap(),
            Some(mapped)
        );
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn same_account_reconnect_rotates_encryption_without_resetting_watermarks(
        pool: sqlx::PgPool,
    ) {
        let org = organization(&pool).await;
        connect(&pool, org, "original").await.unwrap();
        let mark = Utc::now();
        advance_watermark(&pool, org, &[(EntityType::TimeEntry, mark)])
            .await
            .unwrap();
        let new_key = "ff0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";
        store(
            &pool,
            org,
            new_key,
            "original",
            "new-access",
            "new-refresh",
            None,
            None,
        )
        .await
        .unwrap();
        let stored = load(&pool, org, new_key).await.unwrap().unwrap();
        assert_eq!(
            (stored.access_token.as_str(), stored.refresh_token.as_str()),
            ("new-access", "new-refresh")
        );
        assert_eq!(stored.watermark_for(EntityType::TimeEntry), Some(mark));
        assert!(load(&pool, org, KEY).await.is_err());
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn connect_and_disconnect_share_import_exclusion_without_blocking_other_orgs(
        pool: sqlx::PgPool,
    ) {
        let org = organization(&pool).await;
        connect(&pool, org, "original").await.unwrap();
        let held = super::super::lock_import(&pool, org).await.unwrap();
        let connect_error = connect(&pool, org, "original").await.unwrap_err();
        assert!(matches!(
            connect_error.downcast_ref(),
            Some(super::super::ApiImportError::Busy)
        ));
        assert!(matches!(
            disconnect(&pool, org).await,
            Err(super::super::ApiImportError::Busy)
        ));
        let other = organization(&pool).await;
        connect(&pool, other, "different").await.unwrap();
        assert_eq!(
            load(&pool, org, KEY).await.unwrap().unwrap().account_id,
            "original"
        );
        super::super::release_import(held).await.unwrap();
        disconnect(&pool, org).await.unwrap();
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn completed_credential_changes_allow_immediate_retries(pool: sqlx::PgPool) {
        let org = organization(&pool).await;
        for _ in 0..64 {
            connect(&pool, org, "original").await.unwrap();
            disconnect(&pool, org).await.unwrap();
        }
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn failed_credential_write_does_not_leave_an_account_binding(pool: sqlx::PgPool) {
        let org = organization(&pool).await;
        sqlx::query!(
            "ALTER TABLE harvest_credentials ADD CONSTRAINT reject_scope CHECK (scope IS NULL)"
        )
        .execute(&pool)
        .await
        .unwrap();
        assert!(
            store(
                &pool,
                org,
                KEY,
                "failed",
                "access",
                "refresh",
                None,
                Some("rejected")
            )
            .await
            .is_err()
        );
        connect(&pool, org, "different").await.unwrap();
        assert_eq!(
            load(&pool, org, KEY).await.unwrap().unwrap().account_id,
            "different"
        );
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn concurrent_first_connections_cannot_bind_different_accounts(pool: sqlx::PgPool) {
        let org = organization(&pool).await;
        let (first, second) =
            tokio::join!(connect(&pool, org, "first"), connect(&pool, org, "second"));
        assert_ne!(first.is_ok(), second.is_ok());
        let account = if first.is_ok() { "first" } else { "second" };
        assert_eq!(
            load(&pool, org, KEY).await.unwrap().unwrap().account_id,
            account
        );
        let losing = if first.is_ok() { "second" } else { "first" };
        assert!(connect(&pool, org, losing).await.is_err());
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn connection_changes_work_with_a_single_connection_pool(pool: sqlx::PgPool) {
        let org = organization(&pool).await;
        let single = sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect_with((*pool.connect_options()).clone())
            .await
            .unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            connect(&single, org, "original").await.unwrap();
            disconnect(&single, org).await.unwrap();
            connect(&single, org, "original").await.unwrap();
        })
        .await
        .unwrap();
        single.close().await;
    }

    #[sqlx::test(migrations = false)]
    async fn migration_binds_existing_connections_without_changing_secrets_or_markers(
        pool: sqlx::PgPool,
    ) {
        let mut before = sqlx::migrate!("./migrations");
        before.migrations = before
            .iter()
            .filter(|m| m.version < 21)
            .cloned()
            .collect::<Vec<_>>()
            .into();
        before.run(&pool).await.unwrap();
        let org = organization(&pool).await;
        let access = encrypt(KEY, "legacy-access").unwrap();
        let refresh = encrypt(KEY, "legacy-refresh").unwrap();
        sqlx::query!(
            "INSERT INTO harvest_credentials (id, org_id, harvest_account_id, access_token_enc, refresh_token_enc) VALUES ($1, $2, 'legacy', $3, $4)",
            Uuid::now_v7(), org, access, refresh,
        ).execute(&pool).await.unwrap();
        let mark = Utc::now();
        advance_watermark(&pool, org, &[(EntityType::TimeEntry, mark)])
            .await
            .unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        let stored = load(&pool, org, KEY).await.unwrap().unwrap();
        assert_eq!(
            (stored.access_token.as_str(), stored.refresh_token.as_str()),
            ("legacy-access", "legacy-refresh")
        );
        assert_eq!(stored.watermark_for(EntityType::TimeEntry), Some(mark));
        assert!(connect(&pool, org, "different").await.is_err());
        disconnect(&pool, org).await.unwrap();
        assert!(connect(&pool, org, "different").await.is_err());
        connect(&pool, org, "legacy").await.unwrap();
    }

    // A deterministic 32-byte key for round-trip tests.
    const KEY: &str = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";

    #[test]
    fn encrypt_then_decrypt_round_trips() {
        let blob = encrypt(KEY, "secret-token").unwrap();
        assert_eq!(decrypt(KEY, &blob).unwrap(), "secret-token");
    }

    #[test]
    fn nonce_makes_ciphertext_nondeterministic() {
        let a = encrypt(KEY, "same").unwrap();
        let b = encrypt(KEY, "same").unwrap();
        // Different random nonces → different blobs, both decrypting correctly.
        assert_ne!(a, b);
        assert_eq!(decrypt(KEY, &a).unwrap(), "same");
        assert_eq!(decrypt(KEY, &b).unwrap(), "same");
    }

    #[test]
    fn wrong_key_fails_to_decrypt() {
        let blob = encrypt(KEY, "secret").unwrap();
        let other = "ff0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";
        assert!(decrypt(other, &blob).is_err());
    }

    #[test]
    fn bad_key_length_is_rejected() {
        assert!(encrypt("abcd", "x").is_err());
    }

    #[test]
    fn decode_hex_rejects_bad_input() {
        assert!(decode_hex("zz").is_err());
        assert!(decode_hex("abc").is_err());
        assert_eq!(decode_hex("00ff").unwrap(), vec![0x00, 0xff]);
    }
}
