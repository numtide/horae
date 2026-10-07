//! Scoped directory values, independent of account and permission administration.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::permission_editor::PermissionRequester;

/// Activity filters never grant archive/restore or assignment authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(
    not(feature = "server"),
    expect(
        dead_code,
        reason = "Directory UI integration waits for the reviewed policy cutover."
    )
)]
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

/// Basic identity only: no access roles, financial or authentication metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PersonSummary {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub active: bool,
}

/// Every page independently authorizes its current requester and records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(
    not(feature = "server"),
    expect(
        dead_code,
        reason = "Directory UI integration waits for the reviewed policy cutover."
    )
)]
pub struct PeoplePage {
    pub requester: PermissionRequester,
    pub people: Vec<PersonSummary>,
    pub next_after: Option<PeopleCursor>,
}
