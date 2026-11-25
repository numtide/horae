use dioxus::prelude::*;
use uuid::Uuid;

use super::history_table::HistoryTable;
use crate::components::screen_state::{EmptyState, LoadError, Loading};
use crate::models::JobStatus;
use crate::server_fns;

/// Runs newest first from `before`, each opening its own page. While the last
/// page read was full, "Load more" appends the next older one.
#[component]
pub(super) fn RunList(before: Option<Uuid>, limit: i64) -> Element {
    let tr = crate::i18n::use_tr();
    let mut runs = use_resource(use_reactive!(|before, limit| async move {
        server_fns::list_harvest_import_jobs(before, Some(limit)).await
    }));
    let retry = use_callback(move |_| runs.restart());
    // The pages "Load more" appended, and whether the last one was full.
    let mut older = use_signal(Vec::<JobStatus>::new);
    let mut older_full = use_signal(|| true);
    let mut loading_more = use_signal(|| false);
    let mut more_failed = use_signal(|| false);
    let load_more = use_callback(move |last: Uuid| {
        if *loading_more.peek() {
            return;
        }
        loading_more.set(true);
        more_failed.set(false);
        spawn(async move {
            match server_fns::list_harvest_import_jobs(Some(last), Some(limit)).await {
                Ok(page) => {
                    older_full.set(page.len() as i64 == limit);
                    older.write().extend(page);
                }
                Err(_) => more_failed.set(true),
            }
            loading_more.set(false);
        });
    });
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
            Some(Ok(jobs)) => {
                let older = older.read();
                let full = if older.is_empty() { jobs.len() as i64 == limit } else { older_full() };
                let last = older.last().or(jobs.last()).map(|job| job.id);
                let shown: Vec<JobStatus> = jobs.iter().chain(older.iter()).cloned().collect();
                rsx! {
                    div { class: "mt-4",
                        HistoryTable { jobs: shown, busy: loading || loading_more() }
                    }
                    if more_failed() {
                        p { class: "alert alert-danger mt-4", role: "alert", {tr.harvest_runs_more_failed()} }
                    }
                    if full && let Some(last) = last {
                        div { class: "flex justify-center mt-4",
                            button { id: "load-more-runs", "data-testid": "load-more-runs".to_owned(),
                                r#type: "button", class: "btn btn-secondary",
                                disabled: loading_more(), onclick: move |_| load_more.call(last),
                                {tr.action_load_more()}
                            }
                        }
                    }
                }
            }
        }
    }
}
