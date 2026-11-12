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
    /// The person's own IANA zone; `None` follows `workspace_timezone`.
    pub timezone: Option<String>,
    pub workspace_timezone: String,
    /// How far the effective zone was ahead of UTC when this was read, so the
    /// browser can tell today's date without a zone database.
    pub utc_offset_seconds: i32,
}

/// What the Profile and Workspace settings offer besides the session's values.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProfileOptions {
    /// Host of the identity provider the account is bound to; `None` for an
    /// account that is not bound (development sign-in).
    pub sign_in: Option<String>,
    /// Every IANA zone name the server accepts.
    pub zones: Vec<String>,
}

/// Whether a trimmed profile name is one the server stores.
pub fn profile_name_is_valid(name: &str) -> bool {
    (1..=200).contains(&name.chars().count()) && !name.contains('\0')
}

impl CurrentUser {
    pub fn is_admin(&self) -> bool {
        self.org_role == OrgRole::Admin
    }

    pub fn is_manager_or_above(&self) -> bool {
        self.org_role.is_manager_or_above()
    }
}

/// Identity fields used by the current user lists. These consumers need
/// neither authentication metadata nor financial data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::FromRow))]
pub struct UserListItem {
    pub id: Uuid,
    pub email: String,
    pub name: String,
    pub active: bool,
}

/// A person to add as a member: an email and a display name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewPerson {
    pub email: String,
    pub name: String,
}

impl NewPerson {
    /// Trimmed fields, the email standing in for a blank name. `None` when
    /// there is no usable email.
    #[cfg(feature = "server")]
    pub fn normalized(self) -> Option<Self> {
        let email = self.email.trim();
        if email.is_empty() || !email.contains('@') {
            return None;
        }
        let name = self.name.trim();
        Some(Self {
            email: email.to_owned(),
            name: if name.is_empty() { email } else { name }.to_owned(),
        })
    }
}

/// The outcome of adding several people: the accounts created, and the
/// emails that already had one and were left alone.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AddedPeople {
    pub added: Vec<User>,
    pub present: Vec<String>,
}

/// Whether a new account can sign in straight away.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Activation {
    Active,
    Deactivated,
}

/// A person's default rates, in the workspace's default currency. A rate the
/// viewer may not read is `None` and says nothing about the stored value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersonRates {
    pub person_id: Uuid,
    pub currency: String,
    pub billable: Option<PersonRate>,
    pub cost: Option<PersonRate>,
}

/// One readable rate: its amount in minor units (`None` when unset) and
/// whether the viewer may change it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersonRate {
    pub cents: Option<i64>,
    pub editable: bool,
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
