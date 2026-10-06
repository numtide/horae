//! Project responsibility is separate from tracking membership and global grants.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::permission_editor::PermissionRequester;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectManager {
    pub id: Uuid,
    pub name: String,
    pub active: bool,
}

/// The complete retained set, including archived people; never a candidate list.
#[cfg_attr(
    not(feature = "server"),
    expect(dead_code, reason = "Project-manager UI integration is pending.")
)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectManagers {
    pub requester: PermissionRequester,
    pub project_id: Uuid,
    pub access_revision: i64,
    pub managers: Vec<ProjectManager>,
}

#[cfg_attr(
    not(feature = "server"),
    expect(dead_code, reason = "Project-manager UI integration is pending.")
)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectManagersCommand {
    pub kind: ProjectManagersCommandKind,
    pub request_id: Uuid,
    pub expected_access_revision: i64,
    pub project_id: Uuid,
    pub manager_ids: Vec<Uuid>,
}

/// Preserve the durable intent discriminator and validate it on incoming requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectManagersCommandKind {
    ReplaceProjectManagers,
}

#[cfg_attr(
    not(feature = "server"),
    expect(dead_code, reason = "Project-manager UI integration is pending.")
)]
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectManagersOutcome {
    pub project_id: Uuid,
    pub access_revision: i64,
    pub changed: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn command_preserves_durable_intent_and_strictly_decodes_its_discriminator() {
        let command = ProjectManagersCommand {
            kind: ProjectManagersCommandKind::ReplaceProjectManagers,
            request_id: Uuid::now_v7(),
            expected_access_revision: 7,
            project_id: Uuid::now_v7(),
            manager_ids: vec![Uuid::now_v7()],
        };
        let wire = json!({
            "kind": "replace_project_managers",
            "request_id": command.request_id,
            "expected_access_revision": 7,
            "project_id": command.project_id,
            "manager_ids": command.manager_ids,
        });
        assert_eq!(serde_json::to_value(&command).unwrap(), wire);
        assert_eq!(
            serde_json::from_value::<ProjectManagersCommand>(wire.clone()).unwrap(),
            command
        );
        let mut missing = wire.clone();
        missing.as_object_mut().unwrap().remove("kind");
        let mut wrong = wire.clone();
        wrong["kind"] = json!("replace_person_managers");
        let mut extra = wire;
        extra["is_administrator"] = json!(true);
        for invalid in [missing, wrong, extra] {
            assert!(serde_json::from_value::<ProjectManagersCommand>(invalid).is_err());
        }
    }
}
