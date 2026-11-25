use dioxus::prelude::*;
use uuid::Uuid;

use super::history_table::HistoryTable;
use crate::components::screen_state::{EmptyState, LoadError, Loading};
use crate::models::JobStatus;
use crate::server_fns;

/// Runs newest first, each opening its own page. While the last page read was
/// full, "Load more" appends the next older one.
#[component]
pub(super) fn RunList(limit: i64) -> Element {
    let tr = crate::i18n::use_tr();
    let mut runs = use_resource(use_reactive!(|limit| async move {
        server_fns::list_harvest_import_jobs(None, Some(limit)).await
    }));
    let retry = use_callback(move |_| runs.restart());
    // The pages "Load more" appended, and whether the last one read was full.
    // `None` until the first click; after it, an empty page ends the list too.
    let mut older = use_signal(Vec::<JobStatus>::new);
    let mut older_full = use_signal(|| None::<bool>);
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
                    let full = page.len() as i64 == limit;
                    // The button goes away with focus on it: move focus to the
                    // first new run, or to the last one when none came back.
                    let target = page.first().map_or(last, |job| job.id);
                    older_full.set(Some(full));
                    older.write().extend(page);
                    if !full {
                        let target = serde_json::json!(format!("run-{target}"));
                        document::eval(&format!(
                            "requestAnimationFrame(() => document.querySelector(`[data-testid=\"${{{target}}}\"]`)?.focus());"
                        ));
                    }
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
                EmptyState { icon: "import", title: "No retained runs", body: "Start a preview to create one." }
            },
            Some(Ok(jobs)) => {
                let older = older.read();
                let full = older_full().unwrap_or(jobs.len() as i64 == limit);
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
