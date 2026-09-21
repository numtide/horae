use dioxus::prelude::*;
use std::collections::{BTreeSet, HashMap};
use uuid::Uuid;

use super::{is_admin, is_manager, loaded, run_action};
use crate::components::combobox::{ComboOption, Combobox};
use crate::components::controls::Checkbox;
use crate::components::form::{FormCard, FormGroup, Input, Select};
use crate::components::icons::NavIcon;
use crate::components::menu::{Menu, MenuDivider, MenuItem};
use crate::components::modal::Modal;
use crate::components::table::DataTable;
use crate::models::{Client, Project};
use crate::route::Route;
use crate::server_fns;
use horae_core::money::{format_cents, format_cents_plain};
use horae_core::types::{BudgetKind, ProjectType};

fn hours(minutes: i64) -> String {
    format!("{}h", horae_core::duration::format_decimal(minutes))
}

/// Budget / Spent / Budget-remaining for one row, expressed in the project's own
/// budget unit (money for amount budgets, hours for hours budgets).
struct RowSpend {
    budget: String,
    recurring: bool,
    spent: String,
    remaining: String,
    /// Consumption for the progress bar, clamped 0..=100 (None = no budget set).
    pct: Option<u8>,
    /// "(NN%)" shown next to Budget remaining (None = no budget set).
    pct_label: Option<String>,
}

fn row_spend(p: &Project, spent_minutes: i64, spent_cents: i64) -> RowSpend {
    let cur = p.currency.trim();
    let recurring = matches!(p.project_type, ProjectType::Retainer);
    // The bar fills with what has been consumed; the label beside "Budget
    // remaining" states what is left, so the two read as complements.
    let pct_of = |spent: i64, budget: i64| -> (Option<u8>, Option<String>) {
        if budget > 0 {
            let consumed = (spent as f64 / budget as f64 * 100.0).round() as i64;
            let left = ((budget - spent) as f64 / budget as f64 * 100.0).round() as i64;
            (
                Some(consumed.clamp(0, 100) as u8),
                Some(format!("({}%)", left.max(0))),
            )
        } else {
            (None, None)
        }
    };
    match p.budget_kind {
        BudgetKind::Amount => {
            let budget = p.budget_amount_cents.unwrap_or(0);
            let (pct, pct_label) = pct_of(spent_cents, budget);
            RowSpend {
                budget: format_cents(budget, cur),
                recurring,
                spent: format_cents(spent_cents, cur),
                remaining: format_cents(budget - spent_cents, cur),
                pct,
                pct_label,
            }
        }
        BudgetKind::Hours => {
            let budget = p.budget_minutes.unwrap_or(0);
            let (pct, pct_label) = pct_of(spent_minutes, budget);
            RowSpend {
                budget: hours(budget),
                recurring,
                spent: hours(spent_minutes),
                remaining: hours(budget - spent_minutes),
                pct,
                pct_label,
            }
        }
        BudgetKind::None => RowSpend {
            budget: "—".to_string(),
            recurring,
            spent: format_cents(spent_cents, cur),
            remaining: "—".to_string(),
            pct: None,
            pct_label: None,
        },
    }
}

fn matches_project_filters(
    project: &Project,
    query: &str,
    scope: &str,
    client: &str,
    client_names: &HashMap<Uuid, String>,
) -> bool {
    let scope_matches = match scope {
        "budgeted" => project.active && project.budget_kind != BudgetKind::None,
        "archived" => !project.active,
        _ => project.active,
    };
    scope_matches
        && (client.is_empty() || project.client_id.to_string() == client)
        && (query.is_empty()
            || project.name.to_lowercase().contains(query)
            || client_names
                .get(&project.client_id)
                .is_some_and(|name| name.to_lowercase().contains(query)))
}

#[derive(Clone)]
struct BulkProjectAction {
    activate: bool,
    projects: Vec<(Uuid, String)>,
}

#[component]
pub fn ProjectList() -> Element {
    // Management view: `include_inactive = true` also lists deactivated projects
    // so managers can reactivate them; new-entry pickers pass `false`.
    let mut projects = use_resource(|| async move { server_fns::list_projects(None, true).await });
    // All clients so projects under deactivated clients still resolve their names.
    let clients_res = use_resource(|| async move { server_fns::list_clients(true).await });
    let me = use_resource(|| async move { server_fns::get_me().await });
    let mut spend_res = use_resource(|| async move { server_fns::list_project_spend().await });

    let mut show_form = use_signal(|| false);
    // Creation has its own route; this form only edits an existing project.
    let mut editing_id = use_signal(|| None::<Uuid>);
    let mut name = use_signal(String::new);
    let mut project_type = use_signal(|| "time_and_materials".to_string());
    let mut currency = use_signal(|| "USD".to_string());
    let mut rate_value = use_signal(String::new);
    let mut budget_kind = use_signal(|| "none".to_string());
    // The figure that goes with the kind — an amount or a number of hours.
    let mut budget_value = use_signal(String::new);
    let mut error = use_signal(|| None::<String>);
    let action_error = use_signal(|| None::<String>);

    // Filters over the loaded list (client-side; the design's status/client
    // dropdowns and search all narrow the same set).
    let mut query = use_signal(String::new);
    // Status scope: "active" | "budgeted" (has a budget) | "archived" (inactive).
    let mut scope = use_signal(|| "active".to_string());
    let mut client_filter = use_signal(String::new);
    let mut selected = use_signal(BTreeSet::<Uuid>::new);
    let mut bulk_action = use_signal(|| None::<BulkProjectAction>);
    let mut bulk_busy = use_signal(|| false);
    let mut bulk_error = use_signal(|| None::<String>);
    let mut bulk_success = use_signal(|| None::<String>);
    // Export modal: open state + chosen scope and format.
    let mut export_open = use_signal(|| false);
    let mut export_scope = use_signal(|| "active".to_string());
    let mut export_fmt = use_signal(|| "csv".to_string());

    let can_import = is_admin(&me);
    let is_manager = is_manager(&me);
    let spend_ready = matches!(&*spend_res.read(), Some(Ok(_)));

    let client_names: HashMap<Uuid, String> = match &*clients_res.read() {
        Some(Ok(cs)) => cs.iter().map(|c| (c.id, c.name.clone())).collect(),
        _ => HashMap::new(),
    };
    let query_lower = query().to_lowercase();
    // Resources retain their previous value while a restart is pending.
    let projects_loading = projects.state()() != UseResourceState::Ready;
    let visible: Vec<Project> = projects
        .read()
        .as_ref()
        .and_then(|result| result.as_ref().ok())
        .into_iter()
        .filter(|_| !projects_loading)
        .flatten()
        .filter(|p| {
            matches_project_filters(p, &query_lower, &scope(), &client_filter(), &client_names)
        })
        .cloned()
        .collect();
    let selection: Vec<(Uuid, String)> = visible
        .iter()
        .filter(|p| selected.read().contains(&p.id))
        .map(|p| {
            (
                p.id,
                match &p.code {
                    Some(code) => format!("[{code}] {}", p.name),
                    None => p.name.clone(),
                },
            )
        })
        .collect();
    let selected_count = selection.len();
    let bulk_label = if scope() == "archived" {
        "Reactivate projects"
    } else {
        "Archive projects"
    };
    let selection_label = match selected_count {
        0 => "Select projects first".to_string(),
        1 => "1 project selected".to_string(),
        count => format!("{count} projects selected"),
    };
    let confirmation = bulk_action();
    let confirm_count = confirmation
        .as_ref()
        .map_or(0, |action| action.projects.len());
    let confirm_noun = if confirm_count == 1 {
        "project"
    } else {
        "projects"
    };
    let confirm_verb = if confirmation.as_ref().is_some_and(|action| action.activate) {
        "Reactivate"
    } else {
        "Archive"
    };
    // project_id -> (spent_minutes, spent_cents); missing = no tracked time yet.
    let spend_map: HashMap<Uuid, (i64, i64)> = match &*spend_res.read() {
        Some(Ok(v)) => v
            .iter()
            .map(|s| (s.project_id, (s.spent_minutes, s.spent_cents)))
            .collect(),
        _ => HashMap::new(),
    };
    let (active_count, budgeted_count, archived_count) = match &*projects.read() {
        Some(Ok(list)) => (
            list.iter().filter(|p| p.active).count(),
            list.iter()
                .filter(|p| p.active && p.budget_kind != BudgetKind::None)
                .count(),
            list.iter().filter(|p| !p.active).count(),
        ),
        _ => (0, 0, 0),
    };
    let scope_label = match scope().as_str() {
        "budgeted" => format!("Budgeted projects ({budgeted_count})"),
        "archived" => format!("Archived projects ({archived_count})"),
        _ => format!("Active projects ({active_count})"),
    };
    // Client options for the filter combobox, grouped Active / Archived.
    let client_options: Vec<ComboOption> = match &*clients_res.read() {
        Some(Ok(cs)) => {
            let mut active: Vec<&Client> = cs.iter().filter(|c| c.active).collect();
            let mut archived: Vec<&Client> = cs.iter().filter(|c| !c.active).collect();
            let by_name =
                |a: &&Client, b: &&Client| a.name.to_lowercase().cmp(&b.name.to_lowercase());
            active.sort_by(by_name);
            archived.sort_by(by_name);
            active
                .into_iter()
                .map(|c| ComboOption::grouped(c.id.to_string(), c.name.clone(), "Active clients"))
                .chain(archived.into_iter().map(|c| {
                    ComboOption::grouped(c.id.to_string(), c.name.clone(), "Archived clients")
                }))
                .collect()
        }
        _ => Vec::new(),
    };

    let mut reset_form = move || {
        editing_id.set(None);
        name.set(String::new());
        project_type.set("time_and_materials".to_string());
        currency.set("USD".to_string());
        rate_value.set(String::new());
        budget_kind.set("none".to_string());
        budget_value.set(String::new());
        error.set(None);
        show_form.set(false);
    };

    // Value is the enum's snake_case `Display`; label via ProjectType::label,
    // so the pill and this picker share one source of truth.
    let type_opts: Vec<(String, String)> = [
        ProjectType::TimeAndMaterials,
        ProjectType::FixedFee,
        ProjectType::NonBillable,
        ProjectType::Retainer,
    ]
    .iter()
    .map(|t| (t.to_string(), t.label().to_string()))
    .collect();
    let budget_is_hours = budget_kind() == "hours";
    let budget_label = if budget_is_hours {
        "Budget hours"
    } else {
        "Budget amount"
    };
    let budget_placeholder = if budget_is_hours {
        "120 or 7:30"
    } else {
        "12000 or 12,000.50"
    };
    // The figure only means something once a kind is chosen, and what it means
    // differs, so the field follows the kind.
    let budget_hint = if budget_is_hours {
        "Total hours for this project. Leave blank to set it later."
    } else {
        "Total fees in the project's currency. Leave blank to set it later."
    };

    rsx! {
        div {
            div { class: "page-header mb-5",
                h1 { class: "page-title text-4xl font-semibold text-strong tracking-tight", "Projects" }
                div { class: "page-actions items-center gap-4",
                    if is_manager {
                        if show_form() {
                            button {
                                class: "btn btn-secondary py-2 px-4",
                                onclick: move |_| reset_form(),
                                "Cancel editing"
                            }
                        } else {
                            Link { to: Route::NewProject {}, class: "btn btn-primary py-2 px-4", "New project" }
                        }
                        Menu {
                            id: "project-bulk-menu", label: "⚡ Actions", align_right: true,
                            trigger_class: "text-sm py-2 px-4",
                            disabled: selected_count == 0 || bulk_busy(),
                            div { class: "px-3 pt-1 pb-2 text-xs uppercase text-label", "{selection_label}" }
                            if selected_count > 100 {
                                p { class: "px-3 text-sm text-secondary", "Select at most 100 projects" }
                            }
                            MenuItem {
                                disabled: selected_count == 0 || selected_count > 100 || bulk_busy(),
                                onclick: move |_| {
                                    if selection.is_empty() || selection.len() > 100 || bulk_busy() { return; }
                                    bulk_error.set(None);
                                    bulk_success.set(None);
                                    bulk_action.set(Some(BulkProjectAction {
                                        activate: scope() == "archived",
                                        projects: selection.clone(),
                                    }));
                                },
                                "{bulk_label}"
                            }
                        }
                    }
                    if can_import {
                        Link { to: Route::HarvestImport {}, class: "btn btn-secondary py-2 px-4", "Import" }
                    }
                    button {
                        class: "btn btn-secondary py-2 px-4",
                        onclick: move |_| export_open.set(true),
                        "Export"
                    }
                    div { class: "proj-search flex items-center gap-2 py-2 px-3 bg-secondary border rounded-btn",
                        span { class: "text-faint flex-none", aria_hidden: "true", "⌕" }
                        input {
                            class: "proj-search-input",
                            r#type: "text",
                            placeholder: "Search by project or client",
                            aria_label: "Search by project or client",
                            value: "{query}",
                            oninput: move |e| { selected.write().clear(); query.set(e.value()); },
                        }
                    }
                }
            }

            div { class: "flex flex-wrap items-center gap-4 mb-5",
                Menu { id: "project-scope-menu", label: "{scope_label}", trigger_class: "text-sm px-4",
                    MenuItem {
                        selected: scope() == "active",
                        onclick: move |_| { selected.write().clear(); scope.set("active".to_string()); },
                        "Active projects ({active_count})"
                    }
                    MenuItem {
                        selected: scope() == "budgeted",
                        onclick: move |_| { selected.write().clear(); scope.set("budgeted".to_string()); },
                        "Budgeted projects ({budgeted_count})"
                    }
                    MenuItem {
                        selected: scope() == "archived",
                        onclick: move |_| { selected.write().clear(); scope.set("archived".to_string()); },
                        "Archived projects ({archived_count})"
                    }
                }
                div { class: "flex-1" }
                Combobox {
                    trigger_class: "text-sm px-4",
                    options: client_options,
                    value: client_filter(),
                    placeholder: "All clients",
                    all_label: "All clients",
                    onselect: move |v| { selected.write().clear(); client_filter.set(v); },
                }
            }

            if show_form() && is_manager {
                FormCard { title: "Edit Project", error,
                    FormGroup { label: "Name", id: "proj-name",
                        Input {
                            id: "proj-name",
                            placeholder: "Project name",
                            value: "{name}",
                            oninput: move |e: FormEvent| name.set(e.value()),
                        }
                    }
                    FormGroup { label: "Type", id: "proj-type",
                        Select {
                            id: "proj-type",
                            options: type_opts,
                            selected: project_type(),
                            onchange: move |e: FormEvent| project_type.set(e.value()),
                        }
                    }
                    FormGroup { label: "Currency", id: "proj-currency",
                        Input {
                            id: "proj-currency",
                            placeholder: "USD",
                            value: "{currency}",
                            oninput: move |e: FormEvent| currency.set(e.value()),
                        }
                    }
                    FormGroup { label: "Hourly rate", id: "proj-rate", hint: "Leave blank to use the user's default. Task and assignment overrides take priority. Zero is a free rate.",
                        Input {
                            id: "proj-rate",
                            placeholder: "120.00",
                            value: "{rate_value}",
                            oninput: move |e: FormEvent| rate_value.set(e.value()),
                        }
                    }
                    FormGroup { label: "Budget", id: "proj-budget",
                        Select {
                            id: "proj-budget",
                            options: vec![
                                ("none".to_string(), "None".to_string()),
                                ("amount".to_string(), "Amount".to_string()),
                                ("hours".to_string(), "Hours".to_string()),
                            ],
                            selected: budget_kind(),
                            onchange: move |e: FormEvent| budget_kind.set(e.value()),
                        }
                    }
                    if budget_kind() != "none" {
                        FormGroup { label: "{budget_label}", id: "proj-budget-value", hint: "{budget_hint}",
                            Input {
                                id: "proj-budget-value",
                                placeholder: "{budget_placeholder}",
                                value: "{budget_value}",
                                oninput: move |e: FormEvent| budget_value.set(e.value()),
                            }
                        }
                    }
                    button {
                        class: "btn btn-primary",
                        onclick: move |_| {
                            let Some(id) = editing_id() else { return; };
                            let n = name();
                            let pt = project_type();
                            let c = currency();
                            let bk = budget_kind();
                            let bv = budget_value();
                            let rv = rate_value();
                            run_action(
                                async move {
                                    server_fns::update_project(id.to_string(), n, pt, c, bk, bv, rv).await
                                },
                                projects,
                                error,
                                move || {
                                    spend_res.restart();
                                    reset_form();
                                },
                            );
                        },
                        "Save Changes"
                    }
                }
            }

            if let Some(message) = action_error() {
                div { class: "alert alert-danger", role: "alert", "Could not change project status: {message}" }
            }
            if let Some(message) = bulk_success() {
                div { class: "alert alert-success", role: "status", "{message}" }
            }
            match &*spend_res.read() {
                None => rsx! { p { class: "text-sm text-secondary mb-4", role: "status", "Loading project spend…" } },
                Some(Err(_)) => rsx! {
                    div { class: "alert alert-danger", role: "alert",
                        "Could not load project spend. Spent and remaining amounts are unavailable. "
                        button { class: "btn btn-secondary btn-sm", onclick: move |_| spend_res.restart(), "Retry" }
                    }
                },
                Some(Ok(_)) => rsx! {},
            }

            if projects_loading {
                p { class: "text-sm text-secondary", aria_busy: "true", "Loading projects…" }
            } else if matches!(&*projects.read(), Some(Err(_))) {
                div { class: "alert alert-danger", role: "alert",
                    "Could not load projects. Retry to refresh the list. "
                    button { r#type: "button", class: "btn btn-secondary btn-sm",
                        onclick: move |_| {
                            selected.write().clear();
                            projects.restart();
                            document::eval("document.getElementById('project-scope-menu-trigger')?.focus();");
                        },
                        "Retry"
                    }
                }
            } else {
            {loaded(&*projects.read(), |list| {
                    let visible_ids: BTreeSet<Uuid> = visible.iter().map(|p| p.id).collect();
                    let all_selected = !visible_ids.is_empty() && selected_count == visible_ids.len();
                    let mut items = visible.clone();
                    // Group by client, ordered by client name then project name.
                    items.sort_by(|a, b| {
                        let an = client_names.get(&a.client_id).cloned().unwrap_or_default();
                        let bn = client_names.get(&b.client_id).cloned().unwrap_or_default();
                        an.to_lowercase()
                            .cmp(&bn.to_lowercase())
                            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
                    });
                    let mut groups: Vec<(String, Vec<Project>)> = Vec::new();
                    for p in items {
                        let cn = client_names
                            .get(&p.client_id)
                            .cloned()
                            .unwrap_or_else(|| "Unknown client".to_string());
                        match groups.last_mut() {
                            Some((n, v)) if *n == cn => v.push(p),
                            _ => groups.push((cn, vec![p])),
                        }
                    }
                    if groups.is_empty() {
                        rsx! {
                            div { class: "empty-state bg-secondary border rounded-xl px-6 justify-center",
                                span { class: "empty-state-icon empty-state-icon-tile", aria_hidden: "true", NavIcon { name: "briefcase" } }
                                if list.is_empty() {
                                    h2 { class: "empty-state-title text-xl m-0", "No projects yet" }
                                    p { class: "empty-state-text empty-state-copy text-subtle m-0",
                                        if is_manager {
                                            "Projects hold the tasks your team tracks against. Create one to get started."
                                        } else {
                                            "Projects hold the tasks your team tracks against. Ask a manager to create your first project."
                                        }
                                    }
                                    div { class: "flex flex-wrap items-center justify-center gap-3 mt-1",
                                        if is_manager && !show_form() {
                                            Link {
                                                to: Route::NewProject {},
                                                class: "btn btn-primary py-3 px-5",
                                                "New project"
                                            }
                                        }
                                        if can_import {
                                            Link { to: Route::HarvestImport {}, class: "text-sm font-semibold", "Import from Harvest →" }
                                        }
                                    }
                                } else {
                                    h2 { class: "empty-state-title text-xl m-0", "No projects match your filters" }
                                    p { class: "empty-state-text empty-state-copy text-subtle m-0", "Try another search, client or project status." }
                                    button {
                                        class: "btn btn-secondary",
                                        onclick: move |_| {
                                            selected.write().clear();
                                            query.set(String::new());
                                            client_filter.set(String::new());
                                            scope.set("active".to_string());
                                        },
                                        "Reset filters"
                                    }
                                }
                            }
                        }
                    } else {
                        rsx! {
                            div { class: "bg-secondary border rounded-xl overflow-hidden",
                                div { class: "proj-scroll overflow-x-auto", role: "region", aria_label: "Projects by client", tabindex: "0",
                                div { class: if is_manager { "proj-grid proj-grid-selectable grid" } else { "proj-grid grid" },
                                div { class: "proj-head grid items-center py-3 px-5 text-xs uppercase text-label border-b",
                                    if is_manager {
                                        div { class: "flex items-center",
                                            Checkbox {
                                                checked: all_selected, mixed: selected_count > 0 && !all_selected,
                                                compact: true, label: "Select all visible projects", disabled: bulk_busy(),
                                                onclick: move |_| {
                                                    if all_selected { selected.write().clear(); } else { selected.set(visible_ids.clone()); }
                                                },
                                            }
                                        }
                                    }
                                    span { "Client" }
                                    span { class: "text-right", "Budget" }
                                    span { class: "text-right", "Spent" }
                                    span { class: "text-right", "Budget remaining" }
                                    span {}
                                }
                                for (group_name, group) in groups {
                                    div { key: "grp-{group_name}", class: "proj-group py-3 px-5 text-sm font-semibold text-primary", "{group_name}" }
                                    for p in group {
                                        {
                                            let (sm, sc) = spend_map.get(&p.id).copied().unwrap_or((0, 0));
                                            let rs = row_spend(&p, sm, sc);
                                            let pname = match &p.code {
                                                Some(c) => format!("[{c}] {}", p.name),
                                                None => p.name.clone(),
                                            };
                                            rsx! {
                                            div { class: "proj-row grid items-center py-4 px-5 text-sm", key: "{p.id}",
                                            if is_manager {
                                                div { class: "flex items-center",
                                                    Checkbox {
                                                        checked: selected.read().contains(&p.id), compact: true,
                                                        label: "Select {pname}", disabled: bulk_busy(),
                                                        onclick: move |_| {
                                                            let mut ids = selected.write();
                                                            if !ids.remove(&p.id) { ids.insert(p.id); }
                                                        },
                                                    }
                                                }
                                            }
                                            div { class: "flex items-center gap-3 min-w-0",
                                                Link {
                                                    to: Route::ProjectDetail { id: p.id },
                                                    class: "font-semibold text-strong min-w-0 proj-namelink",
                                                    "{pname}"
                                                }
                                                span { class: "chip px-2 flex-none whitespace-nowrap", "{p.project_type.label()}" }
                                                if !p.active {
                                                    span { class: "badge badge-neutral", "Inactive" }
                                                }
                                            }
                                            div { class: "flex items-center justify-end gap-2 font-mono",
                                                span { class: "whitespace-nowrap", "{rs.budget}" }
                                                if rs.recurring {
                                                    span { class: "text-faint", "⟳" }
                                                }
                                            }
                                            div { class: "flex items-center justify-end gap-3 font-mono",
                                                if spend_ready {
                                                    span { class: "whitespace-nowrap", "{rs.spent}" }
                                                    if let Some(pct) = rs.pct {
                                                        progress {
                                                            class: "proj-bar",
                                                            aria_label: "Budget used for {p.name}",
                                                            max: "100", value: "{pct}", "{pct}%"
                                                        }
                                                    }
                                                } else {
                                                    span { class: "text-muted", aria_label: "Spent unavailable", "—" }
                                                }
                                            }
                                            div { class: "flex items-baseline justify-end gap-2 font-mono",
                                                if spend_ready {
                                                    span { class: "whitespace-nowrap", "{rs.remaining}" }
                                                    if let Some(lbl) = rs.pct_label.clone() {
                                                        span { class: "text-faint text-xs whitespace-nowrap", "{lbl}" }
                                                    }
                                                } else {
                                                    span { class: "text-muted", aria_label: "Remaining unavailable", "—" }
                                                }
                                            }
                                            div { class: "flex justify-end",
                                                if is_manager {
                                                    Menu { id: "project-actions-{p.id}", label: "Actions", align_right: true,
                                                        MenuItem {
                                                            onclick: {
                                                                let p = p.clone();
                                                                move |_| {
                                                                    editing_id.set(Some(p.id));
                                                                    name.set(p.name.clone());
                                                                    project_type.set(p.project_type.to_string());
                                                                    currency.set(p.currency.clone());
                                                                    rate_value.set(p.rate_cents.map(format_cents_plain).unwrap_or_default());
                                                                    budget_kind.set(p.budget_kind.to_string());
                                                                    // Seed the figure so saving an untouched
                                                                    // form doesn't clear the budget.
                                                                    budget_value
                                                                        .set(match p.budget_kind {
                                                                            BudgetKind::Amount => p
                                                                                .budget_amount_cents
                                                                                .map(format_cents_plain)
                                                                                .unwrap_or_default(),
                                                                            BudgetKind::Hours => p
                                                                                .budget_minutes
                                                                                .map(horae_core::duration::format_hhmm)
                                                                                .unwrap_or_default(),
                                                                            BudgetKind::None => String::new(),
                                                                        });
                                                                    error.set(None);
                                                                    show_form.set(true);
                                                                }
                                                            },
                                                            "Edit"
                                                        }
                                                        MenuDivider {}
                                                        MenuItem {
                                                            onclick: {
                                                                let id = p.id;
                                                                let next_active = !p.active;
                                                                move |_| run_action(
                                                                    server_fns::set_project_active(id.to_string(), next_active),
                                                                    projects,
                                                                    action_error,
                                                                    || (),
                                                                )
                                                            },
                                                            if p.active { "Archive" } else { "Unarchive" }
                                                        }
                                                    }
                                                } else {
                                                    Link {
                                                        to: Route::ProjectDetail { id: p.id },
                                                        class: "btn btn-secondary btn-sm",
                                                        "View"
                                                    }
                                                }
                                            }
                                            }
                                            }
                                        }
                                    }
                                }
                                }
                                }
                            }
                        }
                    }
            })}
            }

            Modal {
                id: "bulk-projects-dialog", labelledby: "bulk-projects-title",
                focus_fallback: "project-scope-menu-trigger",
                open: confirmation.is_some(), busy: bulk_busy(),
                on_dismiss: move |_| bulk_action.set(None),
                h2 { id: "bulk-projects-title", class: "modal-title m-0", "{confirm_verb} {confirm_count} {confirm_noun}?" }
                div { class: "modal-body",
                    p { class: "text-sm text-secondary", "Only project status changes. Existing time entries, invoices and budgets are kept." }
                    ul { class: "text-sm",
                        if let Some(action) = &confirmation {
                            for (id, name) in &action.projects {
                                li { key: "{id}", class: "proj-confirm-name", "{name}" }
                            }
                        }
                    }
                    if let Some(message) = bulk_error() {
                        div { class: "alert alert-danger", role: "alert", "Could not confirm project status: {message}. You can retry safely or cancel and refresh the list." }
                    }
                    div { class: "modal-actions",
                        button {
                            r#type: "button", class: "btn btn-primary", disabled: bulk_busy(),
                            onclick: move |_| {
                                if bulk_busy() { return; }
                                let Some(action) = bulk_action() else { return; };
                                bulk_busy.set(true);
                                bulk_error.set(None);
                                spawn(async move {
                                    let ids = action.projects.iter().map(|(id, _)| id.to_string()).collect();
                                    match server_fns::set_projects_active(ids, action.activate).await {
                                        Ok(_) => {
                                            let verb = if action.activate { "Reactivated" } else { "Archived" };
                                            let noun = if action.projects.len() == 1 { "project" } else { "projects" };
                                            bulk_success.set(Some(format!("{verb} {} {noun}", action.projects.len())));
                                            selected.write().clear();
                                            bulk_action.set(None);
                                            projects.restart();
                                        }
                                        Err(error) => bulk_error.set(Some(error.to_string())),
                                    }
                                    bulk_busy.set(false);
                                });
                            },
                            if bulk_busy() { "Updating projects…" } else { "{confirm_verb} projects" }
                        }
                        button { r#type: "button", class: "btn btn-secondary", disabled: bulk_busy(),
                            onclick: move |_| bulk_action.set(None), "Cancel"
                        }
                    }
                }
            }

            Modal {
                id: "export-projects-dialog",
                labelledby: "export-projects-title",
                open: export_open(),
                on_dismiss: move |_| export_open.set(false),
                {
                    let url = format!(
                        "/api/projects/export/{}?scope={}",
                        export_fmt(),
                        export_scope()
                    );
                    rsx! {
                        div { id: "export-projects-title", class: "modal-title", "Export projects" }
                        div { class: "modal-body",
                            div { class: "modal-label", "Which projects?" }
                            div { class: "seg-row",
                                for (val , lbl) in [("active", "Active"), ("budgeted", "Budgeted"), ("archived", "Archived")] {
                                    button {
                                        class: if export_scope() == val { "seg-btn selected" } else { "seg-btn" },
                                        onclick: move |_| export_scope.set(val.to_string()),
                                        "{lbl}"
                                    }
                                }
                            }
                            div { class: "modal-label", "Format" }
                            div { class: "seg-row",
                                for (val , lbl) in [("csv", "CSV"), ("xlsx", "Excel")] {
                                    button {
                                        class: if export_fmt() == val { "seg-btn selected" } else { "seg-btn" },
                                        onclick: move |_| export_fmt.set(val.to_string()),
                                        "{lbl}"
                                    }
                                }
                            }
                            div { class: "modal-actions",
                                a {
                                    class: "btn btn-primary",
                                    href: "{url}",
                                    onclick: move |_| export_open.set(false),
                                    "Export projects"
                                }
                                button {
                                    class: "btn btn-secondary",
                                    onclick: move |_| export_open.set(false),
                                    "Cancel"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn ProjectDetail(id: Uuid) -> Element {
    // Reset assignments, tasks and form state together when the router reuses
    // this page for another project. Keys take effect in a dynamic fragment.
    rsx! { for id in [id] { ProjectDetailContent { key: "{id}", id } } }
}

#[component]
fn ProjectDetailContent(id: Uuid) -> Element {
    let me = use_resource(|| async move { server_fns::get_me().await });
    let assignments = use_resource(move || {
        let pid = id.to_string();
        async move { server_fns::list_assignments(pid).await }
    });
    let users_res = use_resource(|| async move { server_fns::list_users(false).await });

    let mut show_assign_form = use_signal(|| false);
    let mut assign_user_id = use_signal(String::new);
    let mut assign_role = use_signal(|| "freelancer".to_string());
    let error = use_signal(|| None::<String>);
    let action_error = use_signal(|| None::<String>);

    let is_admin = is_admin(&me);

    // Build a lookup from user_id -> user name
    let users_map: std::collections::HashMap<uuid::Uuid, String> = match &*users_res.read() {
        Some(Ok(users)) => users.iter().map(|u| (u.id, u.name.clone())).collect(),
        _ => std::collections::HashMap::new(),
    };

    let assign_user_opts: Vec<(String, String)> =
        std::iter::once((String::new(), "Select a user...".to_string()))
            .chain(
                users_res
                    .read()
                    .as_ref()
                    .and_then(|r| r.as_ref().ok())
                    .into_iter()
                    .flatten()
                    .map(|u| (u.id.to_string(), format!("{} ({})", u.name, u.email))),
            )
            .collect();

    rsx! {
        div {
            div { class: "page-header",
                h1 { class: "page-title", "Project" }
            }
            div { class: "card",
                p { class: "text-muted p-5", "Project detail for {id}" }
            }

            ProjectTasks { project_id: id, can_manage: is_manager(&me) }

            // ── Assignments section ─────────────────────────────────────
            div { class: "mt-6",
                div { class: "page-header",
                    h2 { class: "page-title text-xl", "Assignments" }
                    div { class: "page-actions",
                        if is_admin {
                            button {
                                class: "btn btn-primary",
                                onclick: move |_| show_assign_form.set(!show_assign_form()),
                                if show_assign_form() { "Cancel" } else { "Assign User" }
                            }
                        }
                    }
                }

                if show_assign_form() && is_admin {
                    FormCard { title: "Assign User", error,
                        FormGroup { label: "User", id: "assign-user",
                            Select {
                                id: "assign-user",
                                options: assign_user_opts,
                                selected: assign_user_id(),
                                onchange: move |e: FormEvent| assign_user_id.set(e.value()),
                            }
                        }
                        FormGroup { label: "Role", id: "assign-role",
                            Select {
                                id: "assign-role",
                                options: vec![
                                    ("lead".to_string(), "Lead".to_string()),
                                    ("freelancer".to_string(), "Freelancer".to_string()),
                                    ("admin".to_string(), "Admin".to_string()),
                                ],
                                selected: assign_role(),
                                onchange: move |e: FormEvent| assign_role.set(e.value()),
                            }
                        }
                        button {
                            class: "btn btn-primary",
                            onclick: move |_| {
                                let pid = id.to_string();
                                let uid = assign_user_id();
                                let r = assign_role();
                                run_action(
                                    server_fns::create_assignment(pid, uid, r),
                                    assignments,
                                    error,
                                    move || {
                                        assign_user_id.set(String::new());
                                        assign_role.set("freelancer".to_string());
                                        show_assign_form.set(false);
                                    },
                                );
                            },
                            "Assign"
                        }
                    }
                }

                if let Some(message) = action_error() {
                    div { class: "alert alert-danger", role: "alert", "Could not remove assignment: {message}" }
                }

                div { class: "card",
                    {loaded(&*assignments.read(), |list| {
                        if list.is_empty() {
                            return rsx! {
                                p { class: "text-muted text-sm p-5", "No users assigned yet." }
                            };
                        }
                        rsx! {
                            DataTable {
                                table {
                                    thead {
                                        tr {
                                            th { "User" }
                                            th { "Role" }
                                            if is_admin {
                                                th { "Actions" }
                                            }
                                        }
                                    }
                                    tbody {
                                        for a in list.iter() {
                                            {
                                                let aid = a.id.to_string();
                                                let user_name = users_map.get(&a.user_id).cloned().unwrap_or_else(|| a.user_id.to_string());
                                                rsx! {
                                                    tr { key: "{a.id}",
                                                        td { "{user_name}" }
                                                        td { "{a.role}" }
                                                        if is_admin {
                                                            td {
                                                                button {
                                                                    class: "btn btn-danger btn-sm",
                                                                    onclick: move |_| run_action(
                                                                        server_fns::delete_assignment(aid.clone()),
                                                                        assignments,
                                                                        action_error,
                                                                        || (),
                                                                    ),
                                                                    "Remove"
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    })}
                }
            }
        }
    }
}

#[component]
fn ProjectTasks(project_id: Uuid, can_manage: bool) -> Element {
    let mut enabled = use_resource(move || async move {
        server_fns::list_project_tasks(project_id.to_string()).await
    });
    let mut catalog = use_resource(|| async move { server_fns::list_tasks().await });
    let mut selected = use_signal(String::new);
    let mut name = use_signal(String::new);
    let mut billable = use_signal(|| true);
    let mut error = use_signal(|| None::<String>);
    let mut saving = use_signal(|| false);

    rsx! {
        div { class: "card mt-6 p-5",
            h2 { class: "page-title text-xl", "Project tasks" }
            {loaded(&*enabled.read(), |tasks| rsx! {
                if tasks.is_empty() { p { "No tasks enabled yet." } }
                ul { for task in tasks { li { key: "{task.id}", "{task.name}" } } }
            })}
            if let Some(message) = error() {
                div { class: "alert alert-danger", role: "alert", "{message}" }
            }
            if can_manage {
                FormGroup { label: "Enable an existing task", id: "project-task",
                    {loaded(&*catalog.read(), |tasks| rsx! {
                        select {
                            id: "project-task", class: "form-select", value: "{selected}",
                            disabled: saving(), onchange: move |e| selected.set(e.value()),
                            option { value: "", "Select task…" }
                            for task in tasks { option { value: "{task.id}", "{task.name}" } }
                        }
                    })}
                }
                button {
                    class: "btn btn-secondary", disabled: saving() || selected().is_empty(),
                    onclick: move |_| {
                        let task_id = selected();
                        saving.set(true);
                        spawn(async move {
                            match server_fns::link_project_task(project_id.to_string(), task_id).await {
                                Ok(()) => { error.set(None); selected.set(String::new()); enabled.restart(); }
                                Err(e) => error.set(Some(e.to_string())),
                            }
                            saving.set(false);
                        });
                    },
                    "Enable task"
                }
                FormGroup { label: "New task name", id: "new-project-task",
                    Input { id: "new-project-task", value: "{name}", oninput: move |e: FormEvent| name.set(e.value()) }
                }
                label { class: "form-label flex items-center gap-2",
                    input { r#type: "checkbox", checked: billable(), onchange: move |e| billable.set(e.checked()) }
                    "Billable on this project"
                }
                button {
                    class: "btn btn-primary", disabled: saving() || name().trim().is_empty(),
                    onclick: move |_| {
                        let task_name = name();
                        let task_billable = billable();
                        saving.set(true);
                        spawn(async move {
                            match server_fns::create_task(task_name, task_billable, Some(project_id.to_string())).await {
                                Ok(_) => { error.set(None); name.set(String::new()); enabled.restart(); catalog.restart(); }
                                Err(e) => error.set(Some(e.to_string())),
                            }
                            saving.set(false);
                        });
                    },
                    "Create and enable task"
                }
            }
        }
    }
}
