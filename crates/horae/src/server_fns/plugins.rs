use super::*;

/// Safe-to-display metadata for a loaded plugin. Configuration values and the
/// WASM instance itself never cross the server-function boundary.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PluginInfo {
    pub name: String,
    pub version: String,
    pub hooks: String,
}

/// List the plugins currently loaded by the server, for the workspace
/// settings' Plugins page and with the same permission.
#[server]
pub async fn list_plugins() -> Result<Vec<PluginInfo>, ServerFnError> {
    use crate::ops;
    let caller = unchecked_session_caller().await?;
    let state = crate::state::global_state().await;
    let read = ops::begin_read(&state.db, caller).await.map_err(op_error)?;
    let allowed = ops::organization::require_settings(&read.actor);
    read.finish(allowed).await.map_err(op_error)?;

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
