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
use uuid::Uuid;

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
pub async fn load(
    pool: &sqlx::PgPool,
    org_id: Uuid,
    key_hex: &str,
) -> anyhow::Result<Option<HarvestConnection>> {
    let row = sqlx::query!(
        r#"SELECT harvest_account_id, access_token_enc, refresh_token_enc,
                  token_expires_at as "token_expires_at: DateTime<Utc>",
                  synced_watermark
           FROM harvest_credentials WHERE org_id = $1"#,
        org_id,
    )
    .fetch_optional(pool)
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
/// (v1); reconnecting overwrites the previous credentials. The parameters mirror
/// the persisted columns one-to-one, hence the count.
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
    let id = Uuid::now_v7();
    sqlx::query!(
        r#"INSERT INTO harvest_credentials
             (id, org_id, harvest_account_id, access_token_enc, refresh_token_enc,
              token_expires_at, scope)
           VALUES ($1, $2, $3, $4, $5, $6, $7)
           ON CONFLICT (org_id) DO UPDATE SET
             harvest_account_id = EXCLUDED.harvest_account_id,
             access_token_enc   = EXCLUDED.access_token_enc,
             refresh_token_enc  = EXCLUDED.refresh_token_enc,
             token_expires_at   = EXCLUDED.token_expires_at,
             scope              = EXCLUDED.scope,
             updated_at         = now()"#,
        id,
        org_id,
        account_id,
        access_enc,
        refresh_enc,
        token_expires_at as Option<chrono::DateTime<chrono::Utc>>,
        scope,
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// Persist refreshed access/refresh tokens after a transparent token refresh
/// (FR-024), without touching the account id or watermark.
pub async fn update_tokens(
    pool: &sqlx::PgPool,
    org_id: Uuid,
    key_hex: &str,
    access_token: &str,
    refresh_token: &str,
    token_expires_at: Option<DateTime<Utc>>,
) -> anyhow::Result<()> {
    let access_enc = encrypt(key_hex, access_token)?;
    let refresh_enc = encrypt(key_hex, refresh_token)?;
    sqlx::query!(
        r#"UPDATE harvest_credentials
           SET access_token_enc = $2, refresh_token_enc = $3,
               token_expires_at = $4, updated_at = now()
           WHERE org_id = $1"#,
        org_id,
        access_enc,
        refresh_enc,
        token_expires_at as Option<chrono::DateTime<chrono::Utc>>,
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// Advance the per-entity incremental-sync watermark after a successful
/// committing run (FR-025). Never called for a dry-run.
pub async fn advance_watermark(
    pool: &sqlx::PgPool,
    org_id: Uuid,
    marks: &[(EntityType, DateTime<Utc>)],
) -> anyhow::Result<()> {
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
           SET synced_watermark = synced_watermark || $2, updated_at = now()
           WHERE org_id = $1"#,
        org_id,
        patch,
    )
    .execute(pool)
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
