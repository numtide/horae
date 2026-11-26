//! OAuth apps and their authorizations as the administration pages show them
//! (spec 024, data-model *Page DTOs*). Scopes travel by name (`time:read`, …);
//! the pages put them into words.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Whether an app can keep a secret.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AppKind {
    /// Installed on people's devices: no secret, PKCE only.
    Public,
    /// Runs on a server that keeps a secret.
    Confidential,
}

/// One app, never its secret.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppRow {
    pub id: Uuid,
    pub client_id: String,
    pub name: String,
    pub kind: AppKind,
    pub scopes: Vec<String>,
    pub redirect_uris: Vec<String>,
    /// Horae's own Android app: it cannot be edited or deleted.
    pub built_in: bool,
    pub enabled: bool,
    /// The first characters of a confidential app's secret.
    pub secret_prefix: Option<String>,
    /// Authorizations neither revoked nor expired.
    pub active_grants: i64,
    pub created_at: DateTime<Utc>,
    pub created_by_name: Option<String>,
}

/// What an administrator enters to register an app, or to change one; an
/// app's kind stays as registered.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppInput {
    pub name: String,
    pub kind: AppKind,
    pub redirect_uris: Vec<String>,
    pub scopes: Vec<String>,
}

/// An app just registered, or given a new secret: the only answer that
/// carries a confidential app's secret.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct CreatedApp {
    pub app: AppRow,
    pub secret: Option<String>,
}

/// Leaves the secret out, so a logged value never reveals it.
impl std::fmt::Debug for CreatedApp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CreatedApp")
            .field("app", &self.app)
            .field("secret", &self.secret.as_ref().map(|_| "…"))
            .finish()
    }
}

/// An authorization neither revoked nor expired. It holds no token.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GrantRow {
    pub id: Uuid,
    pub app_name: String,
    pub client_id: String,
    pub person_id: Uuid,
    pub person_name: String,
    pub person_email: String,
    pub scopes: Vec<String>,
    pub approved_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub expires_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_leaves_the_secret_out() {
        let created = CreatedApp {
            app: AppRow {
                id: Uuid::nil(),
                client_id: "horae_app_x".into(),
                name: "Sync".into(),
                kind: AppKind::Confidential,
                scopes: vec!["time:read".into()],
                redirect_uris: vec!["https://sync.example/cb".into()],
                built_in: false,
                enabled: true,
                secret_prefix: Some("abcdefgh".into()),
                active_grants: 0,
                created_at: DateTime::UNIX_EPOCH,
                created_by_name: None,
            },
            secret: Some("horae_ocs_abcdefghSECRETSECRET".into()),
        };
        let debug = format!("{created:?}");
        assert!(!debug.contains("SECRET"), "{debug}");
    }

    #[test]
    fn kinds_travel_in_lower_case() {
        assert_eq!(
            serde_json::to_string(&AppKind::Confidential).unwrap(),
            "\"confidential\""
        );
    }
}
