use dioxus::prelude::*;
use horae_core::duration::format_hours2 as hours;

use super::{Selection, period};
use crate::models::permission_editor::PermissionRequester;
use crate::models::time_report::{TimeReportGroupCursor, TimeReportGroupQuery, TimeReportGrouping};
use crate::server_fns;

#[component]
pub(super) fn ExpandedTimeReport(
    requester: PermissionRequester,
    from: ReadSignal<String>,
    to: ReadSignal<String>,
    dimension: ReadSignal<TimeReportGrouping>,
    filters: ReadSignal<Vec<Selection>>,
    active_projects_only: ReadSignal<bool>,
    on_detail: EventHandler<Vec<Selection>>,
) -> Element {
    let mut cursors = use_signal(|| vec![None::<TimeReportGroupCursor>]);
    let mut page = use_resource(move || async move {
        let key = (
            from(),
            to(),
            dimension(),
            filters(),
            cursors.read().last().cloned().flatten(),
            active_projects_only(),
        );
        let result = async {
            let (date_from, date_to) = period(&key.0, &key.1)?;
            let mut query = TimeReportGroupQuery {
                date_from,
                date_to,
                active_projects_only: key.5,
                group_by: key.2,
                after: key.4.clone(),
                client_ids: vec![],
                project_ids: vec![],
                task_ids: vec![],
                user_ids: vec![],
                tag_ids: vec![],
                expected_requester: Some(requester),
            };
            for selected in &key.3 {
                selected.apply_grouped(&mut query);
            }
            let loaded = server_fns::list_visible_time_report_groups(query)
                .await
                .map_err(|_| "Could not load the breakdown. Check your access and retry.")?;
            if loaded.requester != requester {
                return Err("Your session changed. Reload this page before viewing reports.");
            }
            Ok(loaded)
        }
        .await;
        (key, result)
    });
    let key = (
        from(),
        to(),
        dimension(),
        filters(),
        cursors.read().last().cloned().flatten(),
        active_projects_only(),
    );
    let response = page.read();
    let current = response.as_ref().filter(|(requested, _)| *requested == key);
    let ready = page.state()() == UseResourceState::Ready && current.is_some();
    let loaded = current
        .filter(|_| ready)
        .and_then(|(_, result)| result.as_ref().ok());
    let next = loaded.and_then(|page| page.next_after.clone());
    let label = match key.2 {
        TimeReportGrouping::Client => "Clients",
        TimeReportGrouping::Project => "Projects",
        TimeReportGrouping::Task => "Tasks",
        TimeReportGrouping::Person => "Team",
    };
    rsx! {
        if !ready { p { role: "status", class: "text-secondary", "Loading breakdown…" } }
        else if let Some((_, Err(message))) = current {
            p { role: "alert", class: "alert alert-danger", "{message}" }
        }
        if let Some(loaded) = loaded {
            if loaded.groups.is_empty() {
                p { role: "status", class: "text-secondary", "No time in this breakdown. Refresh the report or try another date range." }
            } else {
                table { aria_label: "{label} breakdown",
                    thead { tr {
                        th { scope: "col", "{label}" }
                        th { scope: "col", class: "text-right", "Hours" }
                        th { scope: "col", class: "text-right", "Billable hours" }
                        th { scope: "col", class: "text-right", "Non-billable hours" }
                    } }
                    tbody { for group in &loaded.groups {
                        tr { key: "{group.id}",
                            td { "{group.name}" }
                            td { class: "text-right text-mono",
                                button { id: "report-expanded-hours-{group.id}", r#type: "button", class: "btn btn-ghost btn-sm text-mono",
                                    aria_label: "View detailed time for {group.name}",
                                    onclick: {
                                        let mut selected = key.3.clone();
                                        selected.push(Selection { dimension: key.2, id: group.id, name: group.name.clone() });
                                        move |_| { if ready { on_detail.call(selected.clone()); } }
                                    }, "{hours(group.totals.rounded_minutes)}"
                                }
                            }
                            td { class: "text-right text-mono", "{hours(group.totals.billable_minutes)}" }
                            td { class: "text-right text-mono", "{hours(group.totals.rounded_minutes - group.totals.billable_minutes)}" }
                        }
                    } }
                }
            }
        }
        nav { class: "flex flex-wrap items-center gap-3 mt-4", aria_label: "Breakdown pages",
            button { id: "report-expanded-previous", r#type: "button", class: "btn btn-secondary btn-sm",
                disabled: !ready || cursors.read().len() <= 1,
                onclick: move |_| { if ready && cursors.read().len() > 1 { cursors.write().pop(); } }, "Previous"
            }
            button { id: "report-expanded-next", r#type: "button", class: "btn btn-secondary btn-sm", disabled: !ready || next.is_none(),
                onclick: move |_| { if ready && let Some(cursor) = next.clone() { cursors.write().push(Some(cursor)); } }, "Next"
            }
            button { id: "report-expanded-refresh", r#type: "button", class: "btn btn-secondary btn-sm", disabled: !ready,
                onclick: move |_| { if ready { page.restart(); } }, "Refresh breakdown"
            }
        }
    }
}
