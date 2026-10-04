use chrono::{DateTime, Utc};
use horae_core::types::OrgRole;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Session identity for account display and requester-bound form recovery.
/// Financial data and identity-provider metadata are not part of this response.
/// The legacy role supports existing presentation, not canonical authorization.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CurrentUser {
    pub id: Uuid,
    pub org_id: Uuid,
    pub email: String,
    pub name: String,
    pub org_role: OrgRole,
}

impl CurrentUser {
    pub fn is_admin(&self) -> bool {
        self.org_role == OrgRole::Admin
    }

    pub fn is_manager_or_above(&self) -> bool {
        self.org_role.is_manager_or_above()
    }
}

/// Identity and legacy administration fields used by the current user lists.
/// These consumers need neither authentication metadata nor financial data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::FromRow))]
pub struct UserListItem {
    pub id: Uuid,
    pub email: String,
    pub name: String,
    pub org_role: OrgRole,
    pub active: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::FromRow))]
pub struct User {
    pub id: Uuid,
    pub org_id: Uuid,
    pub email: String,
    pub name: String,
    pub oidc_subject: Option<String>,
    pub org_role: OrgRole,
    pub cost_rate_cents: Option<i64>,
    pub billable_rate_cents: Option<i64>,
    pub active: bool,
    pub created_at: DateTime<Utc>,
}

#[cfg(feature = "server")]
impl User {
    pub fn is_admin(&self) -> bool {
        self.org_role == OrgRole::Admin
    }

    pub fn is_manager_or_above(&self) -> bool {
        self.org_role.is_manager_or_above()
    }
}
