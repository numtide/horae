//! Personal access tokens as the settings and administration pages show them.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// One token, never its secret.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiTokenRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub owner_name: String,
    pub name: String,
    /// The first characters after `horae_pat_`, to tell tokens apart.
    pub display_prefix: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
}

/// A token just created: the only answer that carries its secret.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct CreatedApiToken {
    pub token: String,
    pub row: ApiTokenRow,
}

/// Leaves the secret out, so a logged value never reveals it.
impl std::fmt::Debug for CreatedApiToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CreatedApiToken")
            .field(
                "token",
                &format_args!("horae_pat_{}…", self.row.display_prefix),
            )
            .field("row", &self.row)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_leaves_the_secret_out() {
        let created = CreatedApiToken {
            token: "horae_pat_abcdefghSECRETSECRET".into(),
            row: ApiTokenRow {
                id: Uuid::nil(),
                user_id: Uuid::nil(),
                owner_name: "Ada".into(),
                name: "script".into(),
                display_prefix: "abcdefgh".into(),
                created_at: DateTime::UNIX_EPOCH,
                expires_at: None,
                last_used_at: None,
                revoked_at: None,
            },
        };
        let debug = format!("{created:?}");
        assert!(debug.contains("horae_pat_abcdefgh…"), "{debug}");
        assert!(!debug.contains("SECRET"), "{debug}");
    }
}
