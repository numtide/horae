use dioxus::prelude::*;
use horae_core::duration::format_hours2 as hours;

use super::{Selection, expanded::ExpandedTimeReport, period};
use crate::components::icons::NavIcon;
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
    mut context: Signal<Option<Selection>>,
    on_detail: EventHandler<Vec<Selection>>,
) -> Element {
    let mut expanded = use_signal(|| None::<uuid::Uuid>);
    let mut page = use_resource(move || async move {
        let key = (
            dimension(),
            cursors.read().last().cloned().flatten(),
            from(),
            to(),
            context(),
        );
        let result = async {
            let (date_from, date_to) = period(&key.2, &key.3)?;
            let mut query = TimeReportGroupQuery {
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
            };
            if let Some(selected) = &key.4 {
                selected.apply_grouped(&mut query);
            }
            let loaded = server_fns::list_visible_time_report_groups(query)
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
        context(),
    );
    let response = page.read();
    let current = response.as_ref().filter(|(requested, _)| *requested == key);
    let ready = page.state()() == UseResourceState::Ready && current.is_some();
    let loaded = current
        .filter(|_| ready)
        .and_then(|(_, result)| result.as_ref().ok());
    let next = loaded.and_then(|page| page.next_after.clone());
    let download = loaded.map(|_| {
        let group_by = match key.0 {
            TimeReportGrouping::Client => "client",
            TimeReportGrouping::Project => "project",
            TimeReportGrouping::Task => "task",
            TimeReportGrouping::Person => "person",
        };
        let filter = key.4.as_ref().map(|selected| format!("&{}={}", selected.filter_key(), selected.id)).unwrap_or_default();
        format!("group_by={group_by}&from={}&to={}&expected_org_id={}&expected_user_id={}&expected_policy=scoped{filter}", key.2, key.3, requester.org_id, requester.user_id)
    });
    let expansion = key
        .4
        .as_ref()
        .and_then(|selected| selected.expansion(key.0));
    let can_open = key.4.is_none()
        || matches!((&key.4, key.0), (Some(selected), TimeReportGrouping::Project) if selected.dimension == TimeReportGrouping::Client);

    rsx! {
        if key.4.is_some() {
            div { class: "mb-4",
                button { id: "report-group-root", r#type: "button", class: "btn btn-ghost text-sm",
                    onclick: move |_| { context.set(None); dimension.set(TimeReportGrouping::Client); cursors.set(vec![None]); expanded.set(None); },
                    NavIcon { name: "arrow-left" } "Back to Time"
                }
            }
        }
        div { class: "flex flex-wrap items-center justify-between gap-4 mb-4",
            h2 { class: "text-lg m-0 wrap-anywhere",
                if loaded.is_some() { if let Some(selected) = &key.4 { "{selected.name}" } else { "Time" } }
                else { "Time" }
            }
            div { class: "flex flex-wrap items-center gap-3",
                if let Some(query) = download {
                    button { id: "report-group-detail", r#type: "button", class: "btn btn-secondary",
                        onclick: move |_| { if ready { on_detail.call(context().into_iter().collect()); } }, "Detailed report"
                    }
                    a { class: "btn btn-secondary", href: "/api/reports/time/grouped/csv?{query}", "Export CSV" }
                    a { class: "btn btn-secondary", href: "/api/reports/time/grouped/xlsx?{query}", "Export XLSX" }
                }
                button { id: "report-group-refresh", r#type: "button", class: "btn btn-secondary", disabled: !ready,
                    onclick: move |_| { if ready { expanded.set(None); page.restart(); } }, "Refresh report"
                }
            }
        }
        div { class: "segmented flex-wrap mb-6", role: "group", aria_label: "Group time by",
            for (value, id, label) in DIMENSIONS {
                if key.4.as_ref().is_none_or(|selected| selected.includes_tab(value)) {
                button { id: "report-group-{id}", r#type: "button", aria_pressed: dimension() == value,
                    class: if dimension() == value { "segmented-item active" } else { "segmented-item" },
                    onclick: move |_| { expanded.set(None); cursors.set(vec![None]); dimension.set(value); }, "{label}"
                }
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
                                td {
                                    if can_open {
                                        button { id: "report-name-{group.id}", r#type: "button", class: "btn btn-ghost text-left",
                                            aria_label: "View time report for {group.name}",
                                            onclick: {
                                                let selected = Selection { dimension: key.0, id: group.id, name: group.name.clone() };
                                                move |_| { if ready {
                                                    dimension.set(selected.initial_dimension()); context.set(Some(selected.clone()));
                                                    cursors.set(vec![None]); expanded.set(None);
                                                } }
                                            }, "{group.name}"
                                        }
                                    } else if expansion.is_some() {
                                        button { id: "report-expand-{group.id}", r#type: "button", class: "btn btn-ghost text-left",
                                            aria_expanded: expanded() == Some(group.id), aria_controls: "report-breakdown-{group.id}",
                                            onclick: { let id = group.id; move |_| {
                                                if ready { let next = (expanded() != Some(id)).then_some(id); expanded.set(next); }
                                            } },
                                            NavIcon { name: if expanded() == Some(group.id) { "chevron-down" } else { "chevron-right" } }
                                            "{group.name}"
                                        }
                                    } else { "{group.name}" }
                                }
                                td { class: "text-right text-mono",
                                    button { id: "report-hours-{group.id}", r#type: "button", class: "btn btn-ghost btn-sm text-mono",
                                        aria_label: "View detailed time for {group.name}",
                                        onclick: {
                                            let selected = Selection { dimension: key.0, id: group.id, name: group.name.clone() };
                                            let mut filters: Vec<_> = key.4.clone().into_iter().collect();
                                            filters.push(selected);
                                            move |_| { if ready { on_detail.call(filters.clone()); } }
                                        },
                                        "{hours(group.totals.rounded_minutes)}"
                                    }
                                }
                                td { class: "text-right text-mono", "{hours(group.totals.billable_minutes)}" }
                                td { class: "text-right text-mono", "{hours(group.totals.rounded_minutes - group.totals.billable_minutes)}" }
                            }
                            if let Some(group_by) = expansion {
                                tr { id: "report-breakdown-{group.id}", key: "breakdown-{group.id}", hidden: expanded() != Some(group.id),
                                    if expanded() == Some(group.id) {
                                        td { colspan: "4", class: "bg-secondary",
                                            ExpandedTimeReport {
                                                requester, from: key.2.clone(), to: key.3.clone(), dimension: group_by,
                                                filters: {
                                                    let mut filters: Vec<_> = key.4.clone().into_iter().collect();
                                                    filters.push(Selection { dimension: key.0, id: group.id, name: group.name.clone() });
                                                    filters
                                                },
                                                on_detail
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                } }
            }
        }
        nav { class: "flex flex-wrap items-center gap-3 mt-4", aria_label: "Grouped report pages",
            button { id: "report-group-previous", r#type: "button", class: "btn btn-secondary", disabled: !ready || cursors.read().len() <= 1,
                onclick: move |_| { if ready && cursors.read().len() > 1 { expanded.set(None); cursors.write().pop(); } }, "Previous"
            }
            button { id: "report-group-next", r#type: "button", class: "btn btn-secondary", disabled: !ready || next.is_none(),
                onclick: move |_| { if ready && let Some(cursor) = next.clone() { expanded.set(None); cursors.write().push(Some(cursor)); } }, "Next"
            }
        }
    }
}
