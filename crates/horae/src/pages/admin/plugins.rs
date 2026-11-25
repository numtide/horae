use dioxus::prelude::*;

use crate::components::badge::Badge;
use crate::components::page_header::PageHeader;
use crate::components::screen_state::{EmptyState, LoadError};
use crate::components::table::{DataTable, TableMessage, TableSkeleton};
use crate::server_fns;

/// The plugins this server loaded at startup. The server installation chooses
/// them, so they are listed in the workspace and not in anyone's own settings.
/// Only plugins that loaded are listed: a malformed one is skipped at startup.
#[component]
pub fn WorkspacePlugins() -> Element {
    let tr = crate::i18n::use_tr();
    let mut plugins = use_resource(server_fns::list_plugins);
    let retry = use_callback(move |_| plugins.restart());
    let pending = plugins.state()() != UseResourceState::Ready;
    // No plugin can appear without a restart, so an empty list is the whole
    // screen's state, not a row of the table.
    if let Some(Ok(items)) = &*plugins.read()
        && items.is_empty()
    {
        return rsx! {
            PageHeader { title: tr.workspace_plugins_title(), title_id: "plugins-title" }
            EmptyState { icon: "plugin", title: tr.workspace_plugins_empty(),
                body: tr.workspace_plugins_empty_hint(),
            }
        };
    }
    rsx! {
        PageHeader { title: tr.workspace_plugins_title(), title_id: "plugins-title" }
        p { class: "text-secondary mb-4", {tr.workspace_plugins_intro()} }
        DataTable { label: tr.workspace_plugins_table_label(),
            table { aria_label: tr.workspace_plugins_table_label(),
                thead {
                    tr {
                        th { scope: "col", {tr.workspace_plugins_column_name()} }
                        th { scope: "col", {tr.workspace_plugins_column_version()} }
                        th { scope: "col", {tr.workspace_plugins_column_status()} }
                        th { scope: "col", {tr.workspace_plugins_column_hooks()} }
                    }
                }
                tbody {
                    match &*plugins.read() {
                        None => rsx! {
                            TableSkeleton { columns: 4, label: tr.workspace_plugins_loading() }
                        },
                        Some(Err(error)) => rsx! {
                            TableMessage { columns: 4,
                                LoadError { error: error.clone(), what: tr.workspace_plugins_what(), retry, pending, inset: true,
                                    focus_on_recovery: "plugins-title",
                                }
                            }
                        },
                        Some(Ok(items)) => rsx! {
                            for plugin in items.iter() {
                                tr { key: "{plugin.name}",
                                    td { class: "font-semibold", "{plugin.name}" }
                                    td { class: "text-mono text-sm", {tr.workspace_plugin_version(plugin.version.clone())} }
                                    td { Badge { variant: "success", {tr.workspace_plugins_status_loaded()} } }
                                    td { class: "text-sm text-secondary", "{plugin.hooks}" }
                                }
                            }
                        },
                    }
                }
            }
        }
    }
}
