//! Scoped directory values, independent of account and permission administration.

use horae_core::permissions::catalog::BuiltInProfile;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::permission_editor::PermissionRequester;

/// Activity filters never grant archive/restore or assignment authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PeopleActivity {
    Active,
    Archived,
    All,
}

/// Exclusive ordering bound, not an identity lookup or access snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PeopleCursor {
    pub name: String,
    pub id: Uuid,
}

/// Identity, the profile's name and one week's time: no grants, financial or
/// authentication metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PersonSummary {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub active: bool,
    /// The person's profile, `None` without stored permissions.
    pub profile: Option<PersonProfile>,
    /// The requested week's time, `None` when the requester may not read this
    /// person's time.
    pub week: Option<WeekMinutes>,
    /// Whether the requester may archive or restore this person now. Never on
    /// the requester's own row: once archived, they could not undo it.
    pub can_change_active: bool,
    /// Whether the requester may read this person's rates.
    pub can_read_rates: bool,
}

/// Where a person's permissions come from. The app labels it; a custom
/// profile's name is data and shows as stored.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PersonProfile {
    BuiltIn(BuiltInProfile),
    Custom(String),
    /// Grants chosen one by one.
    Individual,
}

/// Minutes tracked in one week: all of them, and the billable ones.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WeekMinutes {
    pub total: i64,
    pub billable: i64,
}

/// The whole directory under the requester's grants, ordered by name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PeoplePage {
    pub requester: PermissionRequester,
    pub people: Vec<PersonSummary>,
}
