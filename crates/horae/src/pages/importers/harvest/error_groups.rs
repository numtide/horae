use dioxus::prelude::*;
use horae_core::importers::harvest::types::{IdNotice, ReportPerson};
use uuid::Uuid;

use super::{entity_label, presentation};
use crate::models::ErrorGroup;

/// "Needs your attention": each problem once, with how often it happened and
/// the rows behind it, plus the people the run adds to Horae.
#[component]
pub(super) fn ErrorGroups(
    job_id: Uuid,
    groups: Vec<ErrorGroup>,
    omitted: u64,
    /// People new to Horae; listed so their emails can be checked.
    new_people: Vec<ReportPerson>,
    /// Records that could not keep their Harvest id as their id in the
    /// Harvest-compatible API, and how many more the report only counted.
    id_notices: Vec<IdNotice>,
    omitted_id_notices: u64,
) -> Element {
    if groups.is_empty() && new_people.is_empty() && id_notices.is_empty() {
        return rsx! {};
    }
    rsx! {
        section { class: "flex flex-col gap-3 mt-6", aria_labelledby: "attention-title",
            div { class: "flex items-center justify-between gap-3 flex-wrap",
                h2 { id: "attention-title", class: "himp-title", "Needs your attention" }
                if !groups.is_empty() {
                    a { class: "text-sm", href: format!("/api/import/harvest/jobs/{job_id}/errors"),
                        "Download every issue (NDJSON)"
                    }
                }
            }
            div { class: "card p-0 overflow-hidden",
                for (index, group) in groups.iter().enumerate() {
                    details { key: "{index}", class: "himp-group", open: index == 0,
                        summary {
                            span { class: "himp-group-count text-warning", "{presentation::count(group.count)}" }
                            span { class: "flex-1 flex flex-col gap-1",
                                strong { "{heading(group)}" }
                                span { class: "text-sm text-secondary", "{advice(group)}" }
                            }
                        }
                        div { class: "overflow-x-auto px-5 pb-4",
                            table { class: "table",
                                thead {
                                    tr {
                                        th { scope: "col", "Record" }
                                        th { scope: "col", "Detail" }
                                    }
                                }
                                tbody {
                                    for sample in group.samples.iter() {
                                        tr {
                                            td { class: "text-mono whitespace-nowrap", "{sample.location}" }
                                            td { class: "text-sm", "{sample.reason}" }
                                        }
                                    }
                                }
                            }
                            if group.count > group.samples.len() as u64 {
                                p { class: "text-sm text-faint mt-2",
                                    "{presentation::count(group.count - group.samples.len() as u64)} more are in the download."
                                }
                            }
                        }
                    }
                }
                if !new_people.is_empty() {
                    details { class: "himp-group",
                        summary {
                            span { class: "himp-group-count text-secondary", "{presentation::count(new_people.len() as u64)}" }
                            span { class: "flex-1 flex flex-col gap-1",
                                strong { "People new to Horae, added as deactivated members" }
                                span { class: "text-sm text-secondary",
                                    "Their time comes across under their name. They can't sign in until you activate them on Team. Not an error; listed so you can check the emails."
                                }
                            }
                        }
                        div { class: "overflow-x-auto px-5 pb-4",
                            table { class: "table",
                                thead {
                                    tr {
                                        th { scope: "col", "Name" }
                                        th { scope: "col", "Email" }
                                    }
                                }
                                tbody {
                                    for person in new_people.iter() {
                                        tr {
                                            td { "{person.name}" }
                                            td { class: "text-mono text-sm", {person.email.clone().unwrap_or_default()} }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                if !id_notices.is_empty() {
                    details { class: "himp-group",
                        summary {
                            span { class: "himp-group-count text-secondary", "{presentation::count(id_notices.len() as u64 + omitted_id_notices)}" }
                            span { class: "flex-1 flex flex-col gap-1",
                                strong { "Records with a new id in the Harvest-compatible API" }
                                span { class: "text-sm text-secondary",
                                    "Their Harvest id was taken, or close to the numbers Horae gives its own records, so tools that use Horae's Harvest API find them under a new one. Not an error. A preview shows which records are affected; the new ids are set on import."
                                }
                            }
                        }
                        div { class: "overflow-x-auto px-5 pb-4",
                            table { class: "table",
                                thead {
                                    tr {
                                        th { scope: "col", "Kind" }
                                        th { scope: "col", "Harvest id" }
                                        th { scope: "col", "New id" }
                                    }
                                }
                                tbody {
                                    for notice in id_notices.iter() {
                                        tr { key: "{notice.entity:?}-{notice.harvest_id}",
                                            td { {entity_label(notice.entity)} }
                                            td { class: "text-mono", "{notice.harvest_id}" }
                                            td { class: "text-mono", {notice.api_id.map_or_else(|| "on import".to_owned(), |id| id.to_string())} }
                                        }
                                    }
                                }
                            }
                            if omitted_id_notices > 0 {
                                p { class: "text-sm text-faint mt-2",
                                    "{presentation::count(omitted_id_notices)} more are only counted."
                                }
                            }
                        }
                    }
                }
                if omitted > 0 {
                    p { class: "p-4 border-t text-faint text-sm",
                        "{presentation::count(omitted)} more issues are in less frequent problems."
                    }
                }
            }
        }
    }
}

fn heading(group: &ErrorGroup) -> String {
    if let Some(kind) = group.entry_kind {
        return presentation::entry_kind_text(kind).0.to_owned();
    }
    let what = entity_label(group.entity);
    match group.kind {
        Some(kind) => format!("{what}: {}", presentation::person_kind_label(kind)),
        None => format!("{what}: {}", group.reason),
    }
}

fn advice(group: &ErrorGroup) -> &'static str {
    match (group.entry_kind, group.kind) {
        (Some(kind), _) => presentation::entry_kind_text(kind).1,
        (None, Some(kind)) => presentation::person_fix(kind),
        (None, None) => "Not imported. Fix the record in Harvest, then preview again.",
    }
}
