use super::billing::{InvoiceHistory, InvoiceHistoryState};
use super::*;
use crate::components::avatar::{Avatar, first_initial};
use crate::models::project::{ProjectActivityInterval, ProjectBreakdown, ProjectWorkEntity};
use horae_core::project_breakdown::WorkTotals;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Tab {
    Tasks,
    Team,
    Invoices,
}

impl Tab {
    fn label(self) -> &'static str {
        match self {
            Self::Tasks => "Tasks",
            Self::Team => "Team",
            Self::Invoices => "Invoices",
        }
    }
    fn id(self) -> &'static str {
        match self {
            Self::Tasks => "project-tab-tasks",
            Self::Team => "project-tab-team",
            Self::Invoices => "project-tab-invoices",
        }
    }
}

#[component]
pub(super) fn ProjectBreakdownPanel(
    project_id: Uuid,
    interval: ReadSignal<Option<ProjectActivityInterval>>,
    revision: ReadSignal<u64>,
    invoice_state: InvoiceHistoryState,
    on_invoice_retry: EventHandler<()>,
) -> Element {
    let mut data = use_resource(move || {
        let requested = interval();
        let _ = revision();
        async move {
            (
                requested,
                server_fns::breakdown::get_project_breakdown(project_id.to_string(), requested)
                    .await,
            )
        }
    });
    let loading = data.state()() != UseResourceState::Ready
        || data
            .read()
            .as_ref()
            .is_none_or(|(requested, _)| *requested != interval());
    rsx! {
        section { class: "mt-8 min-w-0", aria_label: "Project breakdown", aria_busy: loading,
            if loading { p { role: "status", "Loading project breakdown…" } }
            else {
                match &*data.read() {
                    Some((_, Ok(value))) => rsx! { for period in [interval()] {
                        BreakdownTables { key: "{period:?}", project_id, data: value.clone(), invoice_state: invoice_state.clone(),
                            on_invoice_retry: move |_| on_invoice_retry.call(()) }
                    } },
                    Some((_, Err(error))) => rsx! {
                        p { role: "alert", class: "text-danger", "Could not load project breakdown: {error}" }
                        button { r#type: "button", class: "btn btn-secondary min-h-control", onclick: move |_| data.restart(), "Retry breakdown" }
                    },
                    None => rsx! {},
                }
            }
        }
    }
}

fn sorted_entities(data: &ProjectBreakdown, tab: Tab, descending: bool) -> Vec<&ProjectWorkEntity> {
    let (entities, totals) = match tab {
        Tab::Tasks => (&data.tasks, &data.totals.by_task),
        Tab::Team => (&data.people, &data.totals.by_person),
        Tab::Invoices => return Vec::new(),
    };
    let mut rows: Vec<_> = entities.iter().collect();
    rows.sort_by(|left, right| {
        let minutes =
            |row: &ProjectWorkEntity| totals.get(&row.id).map_or(0, |value| value.minutes);
        let order = minutes(left).cmp(&minutes(right));
        (if descending { order.reverse() } else { order })
            .then_with(|| left.name.cmp(&right.name))
            .then_with(|| left.id.cmp(&right.id))
    });
    rows
}

fn cost_label(totals: WorkTotals, currency: Option<&str>) -> String {
    match (currency, totals.cost_cents) {
        (Some(currency), Some(cost)) => format_cents(cost, currency),
        (Some(_), None) => "N/A".into(),
        (None, _) => "—".into(),
    }
}

fn keyboard_tab(current: Tab, key: &Key, with_invoices: bool) -> Option<Tab> {
    let tabs: &[Tab] = if with_invoices {
        &[Tab::Tasks, Tab::Team, Tab::Invoices]
    } else {
        &[Tab::Tasks, Tab::Team]
    };
    let index = tabs.iter().position(|tab| *tab == current).unwrap_or(0);
    match key {
        Key::ArrowLeft => Some(tabs[(index + tabs.len() - 1) % tabs.len()]),
        Key::ArrowRight => Some(tabs[(index + 1) % tabs.len()]),
        Key::Home => Some(Tab::Tasks),
        Key::End => tabs.last().copied(),
        _ => None,
    }
}

fn entity_label(entity: &ProjectWorkEntity, person: bool) -> Element {
    rsx! { span { class: "inline-flex items-center gap-2",
        if person { span { aria_hidden: "true", Avatar { initials: first_initial(&entity.name), size: "sm" } } }
        "{entity.name}"
    } }
}

#[component]
fn BreakdownTables(
    data: ProjectBreakdown,
    #[props(default)] project_id: Option<Uuid>,
    #[props(default)] invoice_state: InvoiceHistoryState,
    #[props(default)] on_invoice_retry: Option<EventHandler<()>>,
) -> Element {
    let mut tab = use_signal(|| Tab::Tasks);
    let with_invoices = invoice_state != InvoiceHistoryState::Hidden;
    let current = if tab() == Tab::Invoices && !with_invoices {
        Tab::Tasks
    } else {
        tab()
    };
    let tabs: &[Tab] = if with_invoices {
        &[Tab::Tasks, Tab::Team, Tab::Invoices]
    } else {
        &[Tab::Tasks, Tab::Team]
    };
    rsx! {
        div { class: "report-tabs flex flex-wrap gap-6", role: "tablist", aria_label: "Project breakdown views",
            for target in tabs.iter().copied() {
                button {
                    id: target.id(), r#type: "button", role: "tab",
                    class: if current == target { "report-tab active min-h-control" } else { "report-tab min-h-control" },
                    aria_selected: current == target, aria_controls: "project-breakdown-panel",
                    tabindex: if current == target { "0" } else { "-1" },
                    onclick: move |_| tab.set(target),
                    onkeydown: move |event| {
                        if let Some(next) = keyboard_tab(target, &event.key(), with_invoices) {
                            event.prevent_default(); tab.set(next);
                            spawn(async move { let _ = document::eval(&format!("document.getElementById('{}')?.focus()", next.id())).await; });
                        }
                    },
                    "{target.label()}"
                    span { class: "chip chip-plain font-mono text-xs ml-2",
                        match target {
                            Tab::Tasks => rsx! { "{data.tasks.len()}" },
                            Tab::Team => rsx! { "{data.people.len()}" },
                            Tab::Invoices => match &invoice_state {
                                InvoiceHistoryState::Ready(value) => rsx! { "{value.invoices.len()}" },
                                InvoiceHistoryState::Failed(_) => rsx! { span { aria_label: "Unavailable", "!" } },
                                _ => rsx! { span { aria_label: "Loading", "…" } },
                            },
                        }
                    }
                }
            }
        }
        div { id: "project-breakdown-panel", role: "tabpanel", aria_labelledby: current.id(), tabindex: "0",
            if current == Tab::Invoices {
                if let Some(id) = project_id {
                    div { class: "flex flex-wrap gap-3 mt-6",
                        Link { to: Route::NewProjectInvoice { id }, class: "btn btn-primary min-h-control", "New invoice" }
                    }
                }
                match &invoice_state {
                    InvoiceHistoryState::Ready(value) => rsx! { InvoiceHistory { data: value.clone() } },
                    InvoiceHistoryState::Failed(error) => rsx! {
                        p { role: "alert", class: "text-danger mt-4", "Could not load project invoices: {error}" }
                        if let Some(retry) = on_invoice_retry {
                            button { r#type: "button", class: "btn btn-secondary min-h-control", onclick: move |_| retry.call(()), "Retry invoices" }
                        }
                    },
                    InvoiceHistoryState::Loading => rsx! { p { role: "status", class: "mt-4", "Loading project invoices…" } },
                    InvoiceHistoryState::Hidden => rsx! {},
                }
            } else {
                WorkTable { key: "{current.id()}", data, tab: current }
            }
        }
    }
}

#[component]
fn WorkTable(data: ProjectBreakdown, tab: Tab) -> Element {
    let mut descending = use_signal(|| true);
    let mut expanded = use_signal(BTreeSet::<Uuid>::new);
    let current = tab;
    let entities = sorted_entities(&data, current, descending());
    let currency = data.cost_currency.as_deref();
    let costs_visible = currency.is_some();
    let totals = if current == Tab::Tasks {
        &data.totals.by_task
    } else {
        &data.totals.by_person
    };
    let counterparts: HashMap<_, _> = if current == Tab::Tasks {
        &data.people
    } else {
        &data.tasks
    }
    .iter()
    .map(|entity| (entity.id, entity))
    .collect();
    let mut children: HashMap<Uuid, Vec<_>> = HashMap::new();
    for cell in &data.cells {
        let (parent, child) = if current == Tab::Tasks {
            (cell.task_id, cell.user_id)
        } else {
            (cell.user_id, cell.task_id)
        };
        if let Some(identity) = counterparts.get(&child) {
            children
                .entry(parent)
                .or_default()
                .push((*identity, cell.totals));
        }
    }
    for rows in children.values_mut() {
        rows.sort_by(|(left, left_totals), (right, right_totals)| {
            let order = left_totals.minutes.cmp(&right_totals.minutes);
            (if descending() { order.reverse() } else { order })
                .then_with(|| left.name.cmp(&right.name))
                .then_with(|| left.id.cmp(&right.id))
        });
    }
    rsx! {
        div {
            div { class: "flex flex-wrap items-center justify-between gap-3 mt-6 mb-4",
                h2 { class: "text-2xl font-semibold text-strong m-0",
                    if let Some(period) = data.interval { "{period.from.format(\"%d %b %Y\")} – {period.to.format(\"%d %b %Y\")}" }
                    else { "All time" }
                }
                p { class: "text-xs text-muted m-0", "Period follows the chart above." }
            }
            div { class: "bg-secondary rounded-xl",
                DataTable { table { aria_label: "{current.label()} — actual tracked hours and internal costs",
                    thead { tr {
                        th { scope: "col", "{current.label()}" }
                        th { scope: "col", class: "text-right", aria_sort: if descending() { "descending" } else { "ascending" },
                            button { r#type: "button", class: "btn btn-ghost btn-sm min-h-control whitespace-nowrap",
                                aria_label: if descending() { "Sort hours ascending" } else { "Sort hours descending" },
                                onclick: move |_| descending.toggle(), "Hours "
                                span { aria_hidden: "true", if descending() { "↓" } else { "↑" } }
                            }
                        }
                        th { scope: "col", class: "text-right whitespace-nowrap", "Costs" }
                    } }
                    tbody {
                        for entity in &entities {
                            {
                                let id = entity.id;
                                let row = totals.get(&id).copied().unwrap_or_else(|| WorkTotals::empty(costs_visible));
                                let has_children = children.contains_key(&id);
                                let open = expanded.read().contains(&id);
                                rsx! {
                                    tr { key: "parent-{id}",
                                        th { scope: "row", class: "px-5 border-b border-light text-left font-normal wrap-anywhere",
                                            if has_children {
                                                button { r#type: "button", class: "btn btn-ghost p-0 font-normal text-default text-left min-h-control gap-2",
                                                    aria_expanded: open,
                                                    onclick: move |_| { let mut open = expanded.write(); if !open.remove(&id) { open.insert(id); } },
                                                    span { aria_hidden: "true", if open { "▾" } else { "▸" } }
                                                    {entity_label(entity, current == Tab::Team)}
                                                }
                                            } else { span { class: "pl-4 text-muted", {entity_label(entity, current == Tab::Team)} } }
                                            if entity.manager { span { class: "chip chip-plain ml-2", "Manager" } }
                                            if !entity.active || !entity.current { span { class: "text-xs text-muted ml-2", "Historical" } }
                                        }
                                        td { class: "text-right font-mono whitespace-nowrap",
                                            title: "{row.minutes} minutes; {row.billable_minutes} billable; {row.minutes - row.billable_minutes} non-billable",
                                            "{hours(row.minutes)}"
                                        }
                                        td { class: "text-right font-mono whitespace-nowrap text-secondary", "{cost_label(row, currency)}" }
                                    }
                                    if open {
                                        for (child, value) in children.get(&id).into_iter().flatten() {
                                            tr { key: "child-{id}-{child.id}", class: "bg-tertiary",
                                                th { scope: "row", class: "text-left font-normal pl-10 pr-5 py-3 border-b border-light wrap-anywhere", {entity_label(child, current == Tab::Tasks)}
                                                    if !child.active || !child.current { span { class: "text-xs text-muted ml-2", "Historical" } }
                                                }
                                                td { class: "text-right font-mono whitespace-nowrap", title: "{value.minutes} minutes; {value.billable_minutes} billable; {value.minutes - value.billable_minutes} non-billable", "{hours(value.minutes)}" }
                                                td { class: "text-right font-mono whitespace-nowrap text-secondary", "{cost_label(*value, currency)}" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        if entities.is_empty() {
                            tr { td { colspan: "3", class: "p-5 text-muted", "No {current.label().to_lowercase()} or tracked work in this period." } }
                        }
                    }
                    tfoot { tr { class: "report-total-row bg-tertiary",
                        td { class: "px-5 py-4", "Total" }
                        td { class: "px-5 py-4 text-right font-mono", title: "{data.totals.total.minutes} minutes", "{hours(data.totals.total.minutes)}" }
                        td { class: "px-5 py-4 text-right font-mono whitespace-nowrap", "{cost_label(data.totals.total, currency)}" }
                    } }
                } }
            }
            p { class: "text-xs text-muted mt-3",
                "Actual time; exact minute splits are available on each hours value. "
                if let Some(currency) = currency { "Costs use current rates in {currency}. N/A means a contributing cost rate is missing." }
                else { "Internal costs are not available with your permissions." }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use horae_core::project_breakdown::{WorkCell, summarize};

    fn fixture() -> ProjectBreakdown {
        let tasks: Vec<_> = [(1, "Zulu"), (2, "Alpha"), (3, "Alpha")]
            .into_iter()
            .map(|(id, name)| ProjectWorkEntity {
                id: Uuid::from_u128(id),
                name: name.into(),
                active: true,
                current: true,
                manager: false,
            })
            .collect();
        let people = vec![ProjectWorkEntity {
            id: Uuid::from_u128(10),
            name: "Alice".into(),
            active: false,
            current: false,
            manager: false,
        }];
        let cells = vec![WorkCell {
            task_id: tasks[0].id,
            user_id: people[0].id,
            totals: WorkTotals {
                minutes: 60,
                billable_minutes: 60,
                cost_cents: Some(100),
            },
        }];
        ProjectBreakdown {
            interval: None,
            cost_currency: Some("USD".into()),
            totals: summarize(&cells, true).unwrap(),
            tasks,
            people,
            cells,
        }
    }

    #[test]
    fn hours_sort_has_stable_name_and_identity_ties_in_both_directions() {
        let data = fixture();
        assert_eq!(
            sorted_entities(&data, Tab::Tasks, true)
                .iter()
                .map(|row| row.id.as_u128())
                .collect::<Vec<_>>(),
            [1, 2, 3]
        );
        assert_eq!(
            sorted_entities(&data, Tab::Tasks, false)
                .iter()
                .map(|row| row.id.as_u128())
                .collect::<Vec<_>>(),
            [2, 3, 1]
        );
    }

    #[test]
    fn keyboard_tabs_wrap_and_cost_labels_do_not_expose_private_values() {
        assert_eq!(
            keyboard_tab(Tab::Tasks, &Key::ArrowLeft, false),
            Some(Tab::Team)
        );
        assert_eq!(
            keyboard_tab(Tab::Team, &Key::ArrowRight, false),
            Some(Tab::Tasks)
        );
        assert_eq!(keyboard_tab(Tab::Team, &Key::Home, false), Some(Tab::Tasks));
        assert_eq!(keyboard_tab(Tab::Tasks, &Key::End, false), Some(Tab::Team));
        assert_eq!(keyboard_tab(Tab::Tasks, &Key::Tab, false), None);
        assert_eq!(
            keyboard_tab(Tab::Team, &Key::ArrowRight, true),
            Some(Tab::Invoices)
        );
        assert_eq!(
            keyboard_tab(Tab::Invoices, &Key::ArrowRight, true),
            Some(Tab::Tasks)
        );
        assert_eq!(
            keyboard_tab(Tab::Tasks, &Key::ArrowLeft, true),
            Some(Tab::Invoices)
        );
        assert_eq!(
            keyboard_tab(Tab::Tasks, &Key::End, true),
            Some(Tab::Invoices)
        );
        let value = WorkTotals {
            minutes: 1,
            billable_minutes: 1,
            cost_cents: Some(100),
        };
        assert_eq!(cost_label(value, None), "—");
        assert_eq!(
            cost_label(
                WorkTotals {
                    cost_cents: None,
                    ..value
                },
                Some("USD")
            ),
            "N/A"
        );
        assert_eq!(
            cost_label(
                WorkTotals {
                    cost_cents: Some(0),
                    ..value
                },
                Some("USD")
            ),
            "USD 0.00"
        );
    }

    #[test]
    fn breakdown_renders_real_counts_zero_rows_and_labeled_period_costs() {
        let html = dioxus::ssr::render_element(rsx! { BreakdownTables { data: fixture() } });
        for expected in [
            "Tasks",
            "Team",
            "role=\"tab\"",
            "aria-selected=true",
            ">3</span>",
            "Alpha",
            "Zulu",
            "All time",
            "USD 1.00",
            "0h",
            "60 minutes",
            "aria-label=\"Tasks — actual tracked hours and internal costs\"",
            "class=\"px-5 py-4\"",
        ] {
            assert!(html.contains(expected), "missing {expected}: {html}");
        }
    }

    #[test]
    fn invoice_tab_opens_history_without_changing_work_period() {
        use dioxus::core::Mutation;
        use dioxus::html::{
            PlatformEventData, SerializedHtmlEventConverter, SerializedMouseData,
            set_event_converter,
        };
        use std::{any::Any, rc::Rc};

        set_event_converter(Box::new(SerializedHtmlEventConverter));
        let mut data = fixture();
        data.interval = Some(ProjectActivityInterval {
            from: "2026-09-01".parse().unwrap(),
            to: "2026-09-30".parse().unwrap(),
        });
        let mut dom = VirtualDom::new_with_props(
            BreakdownTables,
            BreakdownTablesProps {
                data,
                project_id: None,
                invoice_state: InvoiceHistoryState::Ready(
                    crate::models::project::ProjectInvoices {
                        invoices: Vec::new(),
                        totals: Default::default(),
                    },
                ),
                on_invoice_retry: None,
            },
        );
        let mutations = dom.rebuild_to_vec();
        let tabs: Vec<_> = mutations
            .edits
            .into_iter()
            .filter_map(|mutation| match mutation {
                Mutation::NewEventListener { name, id } if name == "click" => Some(id),
                _ => None,
            })
            .take(3)
            .collect();
        assert_eq!(tabs.len(), 3);
        let click = |dom: &mut VirtualDom, id| {
            let event = Event::new(
                Rc::new(PlatformEventData::new(Box::<SerializedMouseData>::default()))
                    as Rc<dyn Any>,
                true,
            );
            dom.runtime().handle_event("click", event, id);
            dom.render_immediate_to_vec();
        };
        click(&mut dom, tabs[2]);
        let html = dioxus::ssr::render(&dom);
        assert!(html.contains("No invoices for this project"), "{html}");
        assert!(
            html.contains("aria-labelledby=\"project-tab-invoices\""),
            "{html}"
        );
        click(&mut dom, tabs[0]);
        let html = dioxus::ssr::render(&dom);
        assert!(html.contains("01 Sep 2026 – 30 Sep 2026"), "{html}");
        assert!(!html.contains("No invoices for this project"), "{html}");
    }
}
