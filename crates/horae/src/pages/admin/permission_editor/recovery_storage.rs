use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

use crate::models::permission_editor::{
    PermissionRequester, ProfileCommand, ProfileOutcome, TemplateCommand,
};
use crate::server_fns;

pub(super) const STORAGE_SCRIPT: &str =
    include_str!("../../../../assets/js/permission-recovery-storage.js");
const LIMIT: usize = 524288;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PendingPermission {
    pub requester: PermissionRequester,
    pub command: PendingCommand,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub(super) enum PendingCommand {
    Person(ProfileCommand),
    Template(TemplateCommand),
}

#[derive(Clone)]
pub(super) enum RecoveryOutcome {
    Person(ProfileOutcome),
    Template,
}

#[derive(Clone)]
pub(super) struct AcknowledgedPermission {
    request: PendingPermission,
    outcome: RecoveryOutcome,
}

pub(super) enum RecoveryError {
    Storage(String),
    Server(ServerFnError),
    Cleanup(String),
}

impl PendingPermission {
    fn encode(&self) -> Result<String, String> {
        let value = serde_json::to_string(self)
            .map_err(|_| "Cannot encode permission recovery data.".to_owned())?;
        if value.len() > LIMIT {
            return Err("Permission recovery data exceeds 512 KiB.".into());
        }
        Ok(value)
    }

    fn decode(value: &str, requester: PermissionRequester) -> Result<Self, String> {
        let invalid = || {
            "Permission recovery data is invalid or belongs to another requester. Keep this tab and check saved permissions before clearing browser data.".to_owned()
        };
        if value.len() > LIMIT {
            return Err(invalid());
        }
        let pending: Self = serde_json::from_str(value).map_err(|_| invalid())?;
        if pending.requester != requester || pending.encode()? != value {
            return Err(invalid());
        }
        Ok(pending)
    }

    pub(super) async fn load(requester: PermissionRequester) -> Result<Option<Self>, String> {
        storage(requester, "load", None)
            .await?
            .map(|value| Self::decode(&value, requester))
            .transpose()
    }

    pub(super) async fn clear(&self) -> Result<(), String> {
        storage(self.requester, "clear", Some(self.encode()?)).await?;
        Ok(())
    }

    /// Retain the acknowledged result if browser cleanup fails, including self-demotion.
    pub(super) async fn attempt(
        &self,
        mut acknowledged: Signal<Option<AcknowledgedPermission>>,
    ) -> Result<RecoveryOutcome, RecoveryError> {
        let previous = acknowledged.peek().clone();
        let outcome = if let Some(previous) = previous {
            if previous.request != *self {
                return Err(RecoveryError::Storage("The acknowledged request changed. Reload to check recovery before making another change.".into()));
            }
            previous.outcome
        } else {
            let value = self.encode().map_err(RecoveryError::Storage)?;
            storage(self.requester, "store", Some(value))
                .await
                .map_err(RecoveryError::Storage)?;
            let outcome = match &self.command {
                PendingCommand::Person(command) => {
                    server_fns::save_person_permissions(command.clone(), self.requester)
                        .await
                        .map(RecoveryOutcome::Person)
                }
                PendingCommand::Template(command) => {
                    server_fns::save_permission_template(command.clone(), self.requester)
                        .await
                        .map(|_| RecoveryOutcome::Template)
                }
            }
            .map_err(RecoveryError::Server)?;
            acknowledged.set(Some(AcknowledgedPermission {
                request: self.clone(),
                outcome: outcome.clone(),
            }));
            outcome
        };
        self.clear().await.map_err(|message| RecoveryError::Cleanup(format!(
            "The server confirmed this request, but its recovery record could not be cleared. {message}"
        )))?;
        Ok(outcome)
    }
}

async fn storage(
    requester: PermissionRequester,
    operation: &str,
    value: Option<String>,
) -> Result<Option<String>, String> {
    let key = format!(
        "horae-permission-request:v1:{}:{}",
        requester.org_id, requester.user_id
    );
    let mut eval = document::eval(STORAGE_SCRIPT);
    eval.send((key, operation, value))
        .map_err(|_| "Cannot access permission recovery storage.".to_owned())?;
    eval.recv::<Result<Option<String>, String>>()
        .await
        .map_err(|_| {
            "Cannot acknowledge permission recovery storage. Keep this tab and retry.".to_owned()
        })?
}
