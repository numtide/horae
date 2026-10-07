//! Identity-only choices authorized by the project operation, not the directory.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::people::PeopleCursor;
use super::permission_editor::PermissionRequester;

/// Creation and editing require separate current authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
#[cfg_attr(
    not(feature = "server"),
    expect(
        dead_code,
        reason = "Project picker UI integration waits for the reviewed policy cutover."
    )
)]
pub enum ProjectPeopleContext {
    Create,
    Edit { project_id: Uuid },
}

/// Resolve selected identities independently of the current search page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
#[cfg_attr(
    not(feature = "server"),
    expect(
        dead_code,
        reason = "Project picker UI integration waits for the reviewed policy cutover."
    )
)]
pub enum ProjectPeopleQuery {
    Search {
        query: String,
        after: Option<PeopleCursor>,
    },
    Resolve {
        ids: Vec<Uuid>,
    },
}

/// Selection reveals no financial, authentication or permission metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectPersonChoice {
    pub id: Uuid,
    pub name: String,
}

/// Choices are labels, not a reservation or authority to save assignments.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(
    not(feature = "server"),
    expect(
        dead_code,
        reason = "Project picker UI integration waits for the reviewed policy cutover."
    )
)]
pub struct ProjectPeopleResult {
    pub requester: PermissionRequester,
    pub people: Vec<ProjectPersonChoice>,
    pub next_after: Option<PeopleCursor>,
}
