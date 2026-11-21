use dioxus::prelude::*;

mod api_tokens;
pub(crate) mod permission_editor;
pub(crate) mod rates;
mod workspace;

pub use api_tokens::AdminApiTokens;
pub use workspace::WorkspaceSettings;

/// Retained requests remain recoverable without mounting a directory or selecting a person.
#[component]
pub(crate) fn PermissionRecovery(on_saved: EventHandler<bool>) -> Element {
    let person = use_signal(|| None);
    rsx! { permission_editor::PermissionEditorDialog { person, on_saved } }
}
