use dioxus::prelude::*;
use horae_core::importers::harvest::types::ImportMode;

use super::presentation;
use crate::models::{CommitAvailability, JobStatus, RetryAvailability};

/// What the run is (dry run or commit, source, state, start) and what can be
/// done with it now.
#[component]
pub(super) fn RunHeader(
    job: JobStatus,
    busy: Signal<bool>,
    on_cancel: EventHandler<()>,
    on_retry: EventHandler<()>,
    on_commit: EventHandler<()>,
) -> Element {
    let preview = job.mode == Some(ImportMode::DryRun);
    let mode_tone = if preview {
        "badge-warning"
    } else {
        "badge-neutral"
    };
    let cancelling = job.phase.as_deref() == Some("cancelling");
    rsx! {
        div { class: "page-header",
            h1 { class: "page-title flex items-center gap-3 flex-wrap",
                "{presentation::source_label(&job.kind)}"
                span { class: "badge {mode_tone}", "data-testid": "run-mode", "{presentation::mode_label(job.mode)}" }
                span { class: "badge badge-neutral", "{presentation::state_label(&job.status)}" }
            }
        }
        p { class: "text-secondary text-sm",
            "Started "
            time { datetime: job.created_at.to_rfc3339(), {job.created_at.format("%d %b %Y, %H:%M UTC").to_string()} }
            if preview { ". A dry run writes no business data." }
        }
        div { class: "flex items-center gap-3 flex-wrap mt-4",
            if job.is_active() {
                button {
                    r#type: "button",
                    class: "btn btn-ghost btn-sm",
                    "data-testid": "cancel-import".to_owned(),
                    disabled: busy() || cancelling,
                    onclick: move |_| on_cancel.call(()),
                    "Cancel"
                }
            }
            if job.can_retry() {
                if job.retry_availability == RetryAvailability::Available {
                    button {
                        r#type: "button",
                        class: "btn btn-secondary btn-sm",
                        "data-testid": "retry-run".to_owned(),
                        disabled: busy(),
                        onclick: move |_| on_retry.call(()),
                        "Retry import"
                    }
                } else {
                    p { class: "text-sm text-faint", "{presentation::retry_reason(job.retry_availability)}" }
                }
            }
            if preview && job.status == "succeeded" {
                if job.commit_availability == CommitAvailability::Available {
                    button {
                        r#type: "button",
                        class: "btn btn-primary btn-sm",
                        "data-testid": "commit-preview".to_owned(),
                        disabled: busy(),
                        onclick: move |_| on_commit.call(()),
                        "Commit this dry run"
                    }
                } else {
                    p { class: "text-sm text-faint", "data-testid": "commit-unavailable", "{presentation::commit_reason(job.commit_availability)}" }
                }
            }
        }
    }
}
