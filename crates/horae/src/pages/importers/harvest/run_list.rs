use dioxus::prelude::*;
use uuid::Uuid;

use super::presentation;
use super::runs_cursor::RunsCursor;
use crate::route::Route;
use crate::server_fns;

/// Runs newest first, each linking to its own page. With `paged`, a full page
/// links to the next older one.
#[component]
pub(super) fn RunList(before: Option<Uuid>, limit: i64, #[props(default)] paged: bool) -> Element {
    let mut runs = use_resource(use_reactive!(|before, limit| async move {
        server_fns::list_harvest_import_jobs(before, Some(limit)).await
    }));
    let loading = !matches!(*runs.state().read(), UseResourceState::Ready);
    rsx! {
        match &*runs.read() {
            None => rsx! { p { class: "text-sm mt-4", "Loading runs…" } },
            Some(Err(error)) => rsx! {
                div { class: "mt-4", role: "alert",
                    p { "Could not load runs: {error}" }
                    button { r#type: "button", class: "btn btn-ghost btn-sm", "data-testid": "refresh-history".to_owned(),
                        onclick: move |_| runs.restart(), "Try again"
                    }
                }
            },
            Some(Ok(jobs)) if jobs.is_empty() => rsx! { p { class: "text-sm mt-4",
                if before.is_none() { "No retained runs. Start a preview to create one." }
                else { "No older runs." }
            } },
            Some(Ok(jobs)) => rsx! {
                ul { class: "flex flex-col gap-3 mt-4", aria_busy: loading.to_string(),
                    for job in jobs {
                        li { key: "{job.id}", class: "flex items-center gap-3 flex-wrap",
                            Link { to: Route::HarvestRun { id: job.id }, class: "btn btn-ghost btn-sm flex-wrap text-left himp-history",
                                "data-testid": "run-{job.id}",
                                span { "{presentation::source_label(&job.kind)}" }
                                span { class: "badge badge-neutral badge-sm", "{presentation::mode_label(job.mode)}" }
                                time { datetime: job.created_at.to_rfc3339(), class: "text-xs text-faint", {job.created_at.format("%d %b %Y, %H:%M UTC").to_string()} }
                                span { class: "badge badge-neutral badge-sm", "{presentation::state_label(&job.status)}" }
                            }
                        }
                    }
                }
                if paged && jobs.len() as i64 == limit && let Some(last) = jobs.last() {
                    Link { to: Route::HarvestRuns { before: RunsCursor(Some(last.id)) }, class: "btn btn-ghost btn-sm mt-4", "data-testid": "older-history", "Older runs" }
                }
            },
        }
    }
}
