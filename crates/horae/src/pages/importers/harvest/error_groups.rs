use dioxus::prelude::*;
use uuid::Uuid;

use super::{entity_label, presentation};
use crate::models::ErrorGroup;

/// The run's errors, one row per problem with how often it happened and where.
#[component]
pub(super) fn ErrorGroups(job_id: Uuid, groups: Vec<ErrorGroup>, omitted: u64) -> Element {
    if groups.is_empty() {
        return rsx! {};
    }
    rsx! {
        section { class: "card p-0 overflow-hidden mt-4", aria_label: "Record errors",
            div { class: "flex items-center gap-3 p-4 bg-secondary",
                h2 { class: "text-sm font-semibold", "Record errors" }
                span { class: "badge badge-danger badge-sm", "{groups.len()} problems" }
            }
            div { class: "overflow-x-auto border-t",
                table { class: "table",
                    thead {
                        tr {
                            th { class: "whitespace-nowrap", "Entity" }
                            th { class: "whitespace-nowrap", "Problem" }
                            th { class: "whitespace-nowrap", "Count" }
                            th { class: "whitespace-nowrap", "Examples" }
                        }
                    }
                    tbody {
                        for group in groups.iter() {
                            tr {
                                td { class: "whitespace-nowrap", "{entity_label(group.entity)}" }
                                td {
                                    if let Some(kind) = group.kind {
                                        div { "{presentation::person_kind_label(kind)}" }
                                        div { class: "text-xs text-faint", "{group.reason}" }
                                    } else {
                                        "{group.reason}"
                                    }
                                }
                                td { class: "text-mono whitespace-nowrap", "{group.count}" }
                                td { class: "text-mono text-xs",
                                    {group.samples.iter().map(|sample| sample.location.as_str()).collect::<Vec<_>>().join(", ")}
                                }
                            }
                        }
                    }
                }
            }
            if omitted > 0 {
                div { class: "p-4 border-t text-faint text-sm", "{omitted} more errors are in less frequent problems." }
            }
            div { class: "p-4 border-t",
                a { class: "btn btn-secondary btn-sm",
                    href: format!("/api/import/harvest/jobs/{job_id}/errors"),
                    "Download all errors (NDJSON)"
                }
            }
        }
    }
}
