//! Scoped directory values, independent of account and permission administration.

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
    /// The built-in or custom profile's name, "Individual" for grants chosen
    /// one by one, `None` without stored permissions.
    pub profile: Option<String>,
    /// The requested week's time, `None` when the requester may not read this
    /// person's time.
    pub week: Option<WeekMinutes>,
    /// Whether the requester may archive or restore this person now.
    pub can_change_active: bool,
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
