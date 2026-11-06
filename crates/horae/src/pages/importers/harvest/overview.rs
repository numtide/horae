use dioxus::prelude::*;
use horae_core::importers::harvest::types::{ImportMode, SyncScope};

use super::connection_card::ConnectionCard;
use super::run_list::RunList;
use super::runs_cursor::RunsCursor;
use super::{Conn, IMPORT_SUB, use_connection};
use crate::route::Route;
use crate::server_fns;

/// The Harvest section's landing page: the connection, a preview of the API
/// import, and the latest runs.
#[component]
pub fn HarvestOverview() -> Element {
    let mut status = use_connection();
    let navigator = use_navigator();
    let manage_open = use_signal(|| false);
    let mut busy = use_signal(|| false);
    let mut connection_error = use_signal(|| None::<(&'static str, String)>);
    let mut start_error = use_signal(|| None::<String>);

    // Fetch the OAuth authorize URL and hand the browser to Harvest.
    let connect = use_callback(move |_: MouseEvent| {
        spawn(async move {
            if *busy.peek() {
                return;
            }
            busy.set(true);
            connection_error.set(None);
            match server_fns::harvest_connect_start().await {
                Ok(url) => {
                    let js = format!(
                        "window.location.href = {};",
                        serde_json::to_string(&url).unwrap_or_default()
                    );
                    let _ = document::eval(&js).await;
                }
                Err(e) => connection_error.set(Some((
                    "Could not start Harvest connection. Try connecting again.",
                    e.to_string(),
                ))),
            }
            busy.set(false);
        });
    });
    let disconnect = use_callback(move |_: MouseEvent| {
        spawn(async move {
            if *busy.peek() {
                return;
            }
            busy.set(true);
            connection_error.set(None);
            match server_fns::harvest_disconnect().await {
                Ok(()) => status.restart(),
                Err(e) => connection_error.set(Some((
                    "Could not disconnect Harvest. Check the connection before trying again.",
                    e.to_string(),
                ))),
            }
            busy.set(false);
        });
    });
    let refresh = use_callback(move |_: MouseEvent| {
        connection_error.set(None);
        status.restart();
    });
    let changed = use_callback(move |_: ()| status.restart());

    let conn = Conn::from_resource(&status);
    let generation = conn.ready().map(|s| s.account_generation);
    let preview = move |_| async move {
        if *busy.peek() {
            return;
        }
        busy.set(true);
        start_error.set(None);
        match server_fns::start_harvest_api_import(ImportMode::DryRun, SyncScope::Full, generation)
            .await
        {
            Ok(job) => {
                navigator.push(Route::HarvestRun { id: job.id });
            }
            Err(error) => start_error.set(Some(error.to_string())),
        }
        busy.set(false);
    };

    rsx! {
        div { class: "himp-page min-w-0",
            div { class: "page-header",
                h1 { class: "page-title", "Import from Harvest" }
            }
            p { class: "text-secondary text-sm mb-6", "{IMPORT_SUB}" }
            div { class: "flex items-center gap-3 flex-wrap mb-6",
                Link { to: Route::ImporterIndex {}, class: "imp-back text-primary text-sm", "data-testid": "all-importers", "← All importers" }
                span { class: "text-faint text-sm", "·" }
                Link { to: Route::HarvestCsv {}, class: "imp-back text-secondary text-sm", "data-testid": "use-csv", "Use a CSV instead" }
            }
            ConnectionCard {
                connection: conn.clone(), manage_open, busy,
                onconnect: connect, ondisconnect: disconnect,
                onrefresh: refresh, on_changed: changed,
            }
            if let Some((message, detail)) = connection_error.read().as_ref() {
                div { class: "alert alert-danger mt-4", role: "alert",
                    p { "{message}" }
                    details { summary { "Details" } p { "{detail}" } }
                }
            }
            if conn.ready().is_some_and(|s| s.configured && s.connected) {
                div { class: "flex items-center gap-4 flex-wrap mt-4",
                    button {
                        r#type: "button",
                        class: "btn btn-primary",
                        "data-testid": "preview-api".to_owned(),
                        disabled: busy(),
                        onclick: preview,
                        "Preview import (dry run)"
                    }
                    span { class: "text-faint text-sm",
                        "Preview leaves business data unchanged. Its job and report are retained."
                    }
                }
            }
            if let Some(error) = start_error() {
                div { class: "alert alert-danger mt-4", role: "alert", "Could not start the preview: {error}" }
            }
            section { class: "card mt-6", aria_label: "Latest runs",
                div { class: "flex items-center justify-between gap-3",
                    h2 { class: "text-sm font-semibold", "Latest runs" }
                    Link { to: Route::HarvestRuns { before: RunsCursor::default() }, class: "btn btn-ghost btn-sm", "data-testid": "all-runs", "All runs" }
                }
                RunList { before: None, limit: 5 }
            }
        }
    }
}
