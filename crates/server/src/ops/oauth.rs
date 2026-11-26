//! OAuth 2.0 with PKCE (spec 024): the apps an administrator registers, the
//! authorizations people give them, and the credentials issued under each.
//!
//! Every credential, personal access tokens included, comes from one
//! generator: a prefix that tells its kind apart (so people and secret
//! scanners recognise it) followed by 32 random bytes in base64url. Only the
//! SHA-256 digest of the whole string is stored; the string itself exists in
//! the answer that hands it out and in the requests that present it.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use chacha20poly1305::aead::OsRng;
use chacha20poly1305::aead::rand_core::RngCore;
use sha2::{Digest, Sha256};

mod audit;
pub mod authorize;
pub mod clients;
pub mod grants;
pub mod tokens;

/// An OAuth access token (research R-5).
pub const ACCESS_TOKEN_PREFIX: &str = "horae_oat_";
/// A refresh token.
pub const REFRESH_TOKEN_PREFIX: &str = "horae_ort_";
/// An authorization code.
pub const CODE_PREFIX: &str = "horae_oac_";
/// A confidential app's client secret.
pub const CLIENT_SECRET_PREFIX: &str = "horae_ocs_";
/// A registered app's client id: public, but not guessable from another.
pub const CLIENT_ID_PREFIX: &str = "horae_app_";

/// How many characters after the prefix are kept to tell credentials apart.
const DISPLAY_LEN: usize = 8;
const SECRET_BYTES: usize = 32;
const CLIENT_ID_BYTES: usize = 16;

/// A credential's secret. `Debug` prints only its kind and display prefix.
pub struct Secret {
    prefix: &'static str,
    value: String,
}

impl Secret {
    pub(crate) fn generate(prefix: &'static str) -> Self {
        Self {
            prefix,
            value: format!("{prefix}{}", random_base64url::<SECRET_BYTES>()),
        }
    }

    /// The whole credential, to hand to its holder once.
    pub fn expose(&self) -> &str {
        &self.value
    }

    /// The first characters after the prefix, stored for display.
    pub(crate) fn display_prefix(&self) -> &str {
        &self.value[self.prefix.len()..self.prefix.len() + DISPLAY_LEN]
    }

    pub(crate) fn digest(&self) -> Vec<u8> {
        digest(&self.value)
    }
}

impl std::fmt::Debug for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}…", self.prefix, self.display_prefix())
    }
}

/// The stored form of a credential: SHA-256 of the whole string. The
/// credentials are 256-bit random, so a slow hash would buy nothing.
pub(crate) fn digest(credential: &str) -> Vec<u8> {
    Sha256::digest(credential.as_bytes()).to_vec()
}

/// 128 random bits in base64url: the id of a pending authorization request,
/// which binds the approval page to the session that started it.
pub fn request_id() -> String {
    random_base64url::<CLIENT_ID_BYTES>()
}

/// A new registered app's client id.
pub(crate) fn generate_client_id() -> String {
    format!(
        "{CLIENT_ID_PREFIX}{}",
        random_base64url::<CLIENT_ID_BYTES>()
    )
}

/// Deletes, in the organization, what can no longer be used or recognised:
///
/// - authorizations that ended (revoked or expired) over a day ago, with
///   every code and token issued under them;
/// - codes and access tokens that expired over a day ago;
/// - refresh tokens never used that idle-expired over a day ago.
///
/// A used refresh token stays until its authorization ends: presenting it
/// again must end the authorization however long after (FR-043), because
/// each rotation restarts the idle expiry of the token that replaced it.
pub async fn delete_expired(pool: &sqlx::PgPool, org_id: uuid::Uuid) -> Result<u64, sqlx::Error> {
    let grants = sqlx::query!(
        "DELETE FROM oauth_grants WHERE org_id = $1
           AND (revoked_at < now() - interval '1 day' OR expires_at < now() - interval '1 day')",
        org_id,
    )
    .execute(pool)
    .await?;
    let codes = sqlx::query!(
        "DELETE FROM oauth_codes WHERE org_id = $1 AND expires_at < now() - interval '1 day'",
        org_id,
    )
    .execute(pool)
    .await?;
    let tokens = sqlx::query!(
        "DELETE FROM oauth_access_tokens WHERE org_id = $1 AND expires_at < now() - interval '1 day'",
        org_id,
    )
    .execute(pool)
    .await?;
    let refresh = sqlx::query!(
        "DELETE FROM oauth_refresh_tokens
         WHERE org_id = $1 AND used_at IS NULL AND idle_expires_at < now() - interval '1 day'",
        org_id,
    )
    .execute(pool)
    .await?;
    Ok(grants.rows_affected()
        + codes.rows_affected()
        + tokens.rows_affected()
        + refresh.rows_affected())
}

fn random_base64url<const N: usize>() -> String {
    let mut bytes = [0u8; N];
    OsRng.fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

#[cfg(any(test, feature = "test-support"))]
pub mod testing;

#[cfg(test)]
mod tests;
