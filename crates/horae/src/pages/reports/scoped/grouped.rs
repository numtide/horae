use dioxus::prelude::*;
use horae_core::duration::format_hours2 as hours;

use super::{Selection, period};
use crate::components::table::DataTable;
use crate::models::permission_editor::PermissionRequester;
use crate::models::time_report::{TimeReportGroupCursor, TimeReportGroupQuery, TimeReportGrouping};
use crate::server_fns;

const DIMENSIONS: [(TimeReportGrouping, &str, &str); 4] = [
    (TimeReportGrouping::Client, "client", "Clients"),
    (TimeReportGrouping::Project, "project", "Projects"),
    (TimeReportGrouping::Task, "task", "Tasks"),
    (TimeReportGrouping::Person, "person", "Team"),
];

#[component]
pub(super) fn GroupedTimeReport(
    requester: PermissionRequester,
    from: Signal<String>,
    to: Signal<String>,
    mut dimension: Signal<TimeReportGrouping>,
    mut cursors: Signal<Vec<Option<TimeReportGroupCursor>>>,
    on_detail: EventHandler<Selection>,
) -> Element {
    let mut page = use_resource(move || async move {
        let key = (
            dimension(),
            cursors.read().last().cloned().flatten(),
            from(),
            to(),
        );
        let result = async {
            let (date_from, date_to) = period(&key.2, &key.3)?;
            let loaded = server_fns::list_visible_time_report_groups(TimeReportGroupQuery {
                date_from,
                date_to,
                client_ids: vec![],
                project_ids: vec![],
                user_ids: vec![],
                task_ids: vec![],
                tag_ids: vec![],
                group_by: key.0,
                after: key.1.clone(),
                expected_requester: Some(requester),
            })
            .await
            .map_err(|_| "Could not load the report. Check your access and retry.")?;
            if loaded.requester != requester {
                return Err("Your session changed. Reload this page before viewing reports.");
            }
            Ok(loaded)
        }
        .await;
        (key, result)
    });
    let key = (
        dimension(),
        cursors.read().last().cloned().flatten(),
        from(),
        to(),
    );
    let response = page.read();
    let current = response.as_ref().filter(|(requested, _)| *requested == key);
    let ready = page.state()() == UseResourceState::Ready && current.is_some();
    let loaded = current
        .filter(|_| ready)
        .and_then(|(_, result)| result.as_ref().ok());
    let next = loaded.and_then(|page| page.next_after.clone());

    rsx! {
        div { class: "flex flex-wrap items-center justify-between gap-4 mb-4",
            h2 { class: "text-lg m-0", "Time" }
            button { id: "report-group-refresh", r#type: "button", class: "btn btn-secondary", disabled: !ready,
                onclick: move |_| { if ready { page.restart(); } }, "Refresh report"
            }
        }
        div { class: "segmented flex-wrap mb-6", role: "group", aria_label: "Group time by",
            for (value, id, label) in DIMENSIONS {
                button { id: "report-group-{id}", r#type: "button", aria_pressed: dimension() == value,
                    class: if dimension() == value { "segmented-item active" } else { "segmented-item" },
                    onclick: move |_| { cursors.set(vec![None]); dimension.set(value); }, "{label}"
                }
            }
        }
        if !ready {
            p { role: "status", class: "text-secondary", "Loading time report…" }
        } else if let Some((_, Err(message))) = current {
            p { role: "alert", class: "alert alert-danger", "{message}" }
        }
        if let Some(loaded) = loaded {
            dl { class: "flex flex-wrap gap-6 mb-6", aria_label: "Full-period totals",
                div { dt { "Total hours" } dd { class: "text-mono m-0", "{hours(loaded.totals.rounded_minutes)}" } }
                div { dt { "Billable hours" } dd { class: "text-mono m-0", "{hours(loaded.totals.billable_minutes)}" } }
                div { dt { "Non-billable hours" } dd { class: "text-mono m-0", "{hours(loaded.totals.rounded_minutes - loaded.totals.billable_minutes)}" } }
            }
            if loaded.groups.is_empty() {
                p { role: "status", class: "text-secondary", "No time on this page. Try another date range or return to the previous page." }
            } else {
                DataTable { table {
                    thead { tr {
                        th { scope: "col", "Name" }
                        th { scope: "col", class: "text-right", "Hours" }
                        th { scope: "col", class: "text-right", "Billable hours" }
                        th { scope: "col", class: "text-right", "Non-billable hours" }
                    } }
                    tbody {
                        for group in &loaded.groups {
                            tr { key: "{group.id}",
                                td { "{group.name}" }
                                td { class: "text-right text-mono",
                                    button { id: "report-hours-{group.id}", r#type: "button", class: "btn btn-ghost btn-sm text-mono",
                                        aria_label: "View detailed time for {group.name}",
                                        onclick: {
                                            let selected = Selection { dimension: key.0, id: group.id, name: group.name.clone() };
                                            move |_| { if ready { on_detail.call(selected.clone()); } }
                                        },
                                        "{hours(group.totals.rounded_minutes)}"
                                    }
                                }
                                td { class: "text-right text-mono", "{hours(group.totals.billable_minutes)}" }
                                td { class: "text-right text-mono", "{hours(group.totals.rounded_minutes - group.totals.billable_minutes)}" }
                            }
                        }
                    }
                } }
            }
        }
        nav { class: "flex flex-wrap items-center gap-3 mt-4", aria_label: "Grouped report pages",
            button { id: "report-group-previous", r#type: "button", class: "btn btn-secondary", disabled: !ready || cursors.read().len() <= 1,
                onclick: move |_| { if ready && cursors.read().len() > 1 { cursors.write().pop(); } }, "Previous"
            }
            button { id: "report-group-next", r#type: "button", class: "btn btn-secondary", disabled: !ready || next.is_none(),
                onclick: move |_| { if ready && let Some(cursor) = next.clone() { cursors.write().push(Some(cursor)); } }, "Next"
            }
        }
    }
}
