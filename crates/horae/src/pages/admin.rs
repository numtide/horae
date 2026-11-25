use dioxus::prelude::*;

mod api_tokens;
mod apps;
pub(crate) mod permission_editor;
mod plugins;
pub(crate) mod rates;
mod workspace;

pub use api_tokens::AdminApiTokens;
pub use apps::{AdminApp, AdminApps};
pub use plugins::WorkspacePlugins;
pub use workspace::WorkspaceSettings;

/// Retained requests remain recoverable without mounting a directory or selecting a person.
#[component]
pub(crate) fn PermissionRecovery(on_saved: EventHandler<bool>) -> Element {
    let person = use_signal(|| None);
    rsx! { permission_editor::PermissionEditorDialog { person, on_saved } }
}
