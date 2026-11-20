use dioxus::prelude::*;
use uuid::Uuid;

use super::history_table::HistoryTable;
use super::runs_cursor::RunsCursor;
use crate::components::screen_state::{EmptyState, LoadError, Loading};
use crate::route::Route;
use crate::server_fns;

/// Runs newest first, each linking to its own page. With `paged`, a full page
/// links to the next older one.
#[component]
pub(super) fn RunList(before: Option<Uuid>, limit: i64, #[props(default)] paged: bool) -> Element {
    let mut runs = use_resource(use_reactive!(|before, limit| async move {
        server_fns::list_harvest_import_jobs(before, Some(limit)).await
    }));
    let retry = use_callback(move |_| runs.restart());
    let loading = !matches!(*runs.state().read(), UseResourceState::Ready);
    rsx! {
        match &*runs.read() {
            None => rsx! { Loading { label: "Loading runs…", class: "mt-4" } },
            Some(Err(error)) => rsx! {
                LoadError { error: error.clone(), what: "runs", retry, pending: loading,
                    retry_id: "refresh-history", focus_on_recovery: "harvest-runs-heading",
                }
            },
            Some(Ok(jobs)) if jobs.is_empty() => rsx! {
                if before.is_none() {
                    EmptyState { icon: "import", title: "No retained runs", body: "Start a preview to create one." }
                } else {
                    EmptyState { icon: "import", title: "No older runs", body: "Every retained run is on the newer pages." }
                }
            },
            Some(Ok(jobs)) => rsx! {
                div { class: "mt-4", aria_busy: loading.to_string(),
                    HistoryTable { jobs: jobs.clone() }
                }
                if paged && jobs.len() as i64 == limit && let Some(last) = jobs.last() {
                    Link { to: Route::HarvestRuns { before: RunsCursor(Some(last.id)) }, class: "btn btn-ghost btn-sm mt-4", "data-testid": "older-history", "Older runs" }
                }
            },
        }
    }
}
