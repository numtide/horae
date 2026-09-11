use super::*;

/// Safe-to-display metadata for a loaded plugin. Configuration values and the
/// WASM instance itself never cross the server-function boundary.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PluginInfo {
    pub name: String,
    pub version: String,
    pub hooks: String,
}

/// List the plugins currently loaded by the server.
#[server]
pub async fn list_plugins() -> Result<Vec<PluginInfo>, ServerFnError> {
    let _user = require_user().await?;
    let state = crate::state::global_state().await;

    Ok(state
        .plugins
        .summaries()
        .into_iter()
        .map(|plugin| PluginInfo {
            name: plugin.name,
            version: plugin.version,
            hooks: plugin.hooks.join(", "),
        })
        .collect())
}
