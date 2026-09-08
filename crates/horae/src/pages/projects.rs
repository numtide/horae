use dioxus::prelude::*;
use std::collections::HashMap;
use uuid::Uuid;

use super::{is_admin, is_manager, loaded, run_action};
use crate::components::combobox::{ComboOption, Combobox};
use crate::components::form::{FormCard, FormGroup, Input, Select};
use crate::components::menu::{Menu, MenuDivider, MenuItem};
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

#[component]
pub fn ProjectList() -> Element {
    // Management view: `include_inactive = true` also lists deactivated projects
    // so managers can reactivate them; new-entry pickers pass `false`.
    let projects = use_resource(|| async move { server_fns::list_projects(None, true).await });
    // All clients (including inactive) so a project under a deactivated client
    // still resolves to its real name; the create form filters to active ones.
    let clients_res = use_resource(|| async move { server_fns::list_clients(true).await });
    let me = use_resource(|| async move { server_fns::get_me().await });
    let mut spend_res = use_resource(|| async move { server_fns::list_project_spend().await });

    let mut show_form = use_signal(|| false);
    // `Some(id)` while editing an existing project, `None` while creating.
    let mut editing_id = use_signal(|| None::<Uuid>);
    let mut client_id = use_signal(String::new);
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
    // Export modal: open state + chosen scope and format.
    let mut export_open = use_signal(|| false);
    let mut export_scope = use_signal(|| "active".to_string());
    let mut export_fmt = use_signal(|| "csv".to_string());

    let is_manager = is_manager(&me);

    let client_names: HashMap<Uuid, String> = match &*clients_res.read() {
        Some(Ok(cs)) => cs.iter().map(|c| (c.id, c.name.clone())).collect(),
        _ => HashMap::new(),
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
        client_id.set(String::new());
        name.set(String::new());
        project_type.set("time_and_materials".to_string());
        currency.set("USD".to_string());
        rate_value.set(String::new());
        budget_kind.set("none".to_string());
        budget_value.set(String::new());
        error.set(None);
        show_form.set(false);
    };

    let form_title = if editing_id().is_some() {
        "Edit Project"
    } else {
        "New Project"
    };
    // The create form only offers active clients; the placeholder keeps the
    // picker unset until one is chosen.
    let form_client_opts: Vec<(String, String)> =
        std::iter::once((String::new(), "Select a client...".to_string()))
            .chain(
                clients_res
                    .read()
                    .as_ref()
                    .and_then(|r| r.as_ref().ok())
                    .into_iter()
                    .flatten()
                    .filter(|c| c.active)
                    .map(|c| (c.id.to_string(), c.name.clone())),
            )
            .collect();
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
            div { class: "page-header",
                h1 { class: "page-title", "Projects" }
                button {
                    class: "btn btn-secondary btn-sm ml-auto",
                    onclick: move |_| export_open.set(true),
                    "Export"
                }
                div { class: "proj-search",
                    span { class: "proj-search-icon", aria_hidden: "true", "⌕" }
                    input {
                        class: "proj-search-input",
                        r#type: "text",
                        placeholder: "Search by project or client",
                        aria_label: "Search by project or client",
                        value: "{query}",
                        oninput: move |e| query.set(e.value()),
                    }
                }
                if is_manager {
                    button {
                        class: "btn btn-primary",
                        onclick: move |_| {
                            if show_form() {
                                reset_form();
                            } else {
                                editing_id.set(None);
                                show_form.set(true);
                            }
                        },
                        if show_form() { "Cancel" } else { "Add Project" }
                    }
                }
            }

            div { class: "flex items-center gap-4 mb-6",
                Menu { label: "{scope_label}",
                    MenuItem {
                        selected: scope() == "active",
                        onclick: move |_| scope.set("active".to_string()),
                        "Active projects ({active_count})"
                    }
                    MenuItem {
                        selected: scope() == "budgeted",
                        onclick: move |_| scope.set("budgeted".to_string()),
                        "Budgeted projects ({budgeted_count})"
                    }
                    MenuItem {
                        selected: scope() == "archived",
                        onclick: move |_| scope.set("archived".to_string()),
                        "Archived projects ({archived_count})"
                    }
                }
                div { class: "flex-1" }
                Combobox {
                    options: client_options,
                    value: client_filter(),
                    placeholder: "Filter by client",
                    all_label: "All clients",
                    onselect: move |v| client_filter.set(v),
                }
            }

            if show_form() && is_manager {
                FormCard { title: "{form_title}", error,
                    // The client is fixed at creation; only shown when creating.
                    if editing_id().is_none() {
                        FormGroup { label: "Client", id: "proj-client",
                            Select {
                                id: "proj-client",
                                options: form_client_opts,
                                selected: client_id(),
                                onchange: move |e: FormEvent| client_id.set(e.value()),
                            }
                        }
                    }
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
                            let editing = editing_id();
                            let cid = client_id();
                            let n = name();
                            let pt = project_type();
                            let c = currency();
                            let bk = budget_kind();
                            let bv = budget_value();
                            let rv = rate_value();
                            run_action(
                                async move {
                                    match editing {
                                        Some(id) => {
                                            server_fns::update_project(id.to_string(), n, pt, c, bk, bv, rv).await
                                        }
                                        None => server_fns::create_project(cid, n, pt, c, bk, bv, rv).await,
                                    }
                                },
                                projects,
                                error,
                                move || {
                                    spend_res.restart();
                                    reset_form();
                                },
                            );
                        },
                        if editing_id().is_some() { "Save Changes" } else { "Create Project" }
                    }
                }
            }

            if let Some(message) = action_error() {
                div { class: "alert alert-danger", role: "alert", "Could not change project status: {message}" }
            }

            {loaded(&*projects.read(), |list| {
                    let q = query().to_lowercase();
                    let cf = client_filter();
                    let sc = scope();
                    let mut items: Vec<Project> = list
                        .iter()
                        .filter(|p| {
                            let scope_ok = match sc.as_str() {
                                "budgeted" => p.active && p.budget_kind != BudgetKind::None,
                                "archived" => !p.active,
                                _ => p.active,
                            };
                            scope_ok && (cf.is_empty() || p.client_id.to_string() == cf)
                        })
                        .filter(|p| {
                            if q.is_empty() {
                                return true;
                            }
                            let cn = client_names.get(&p.client_id).cloned().unwrap_or_default();
                            p.name.to_lowercase().contains(&q) || cn.to_lowercase().contains(&q)
                        })
                        .cloned()
                        .collect();
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
                            div { class: "proj-card",
                                div { class: "proj-empty", "No projects match your filters." }
                            }
                        }
                    } else {
                        rsx! {
                            div { class: "proj-card",
                                div { class: "proj-scroll",
                                div { class: "proj-head",
                                    span { "Client" }
                                    span { class: "text-right", "Budget" }
                                    span { class: "text-right", "⚑ Scheduled" }
                                    span { class: "text-right", "Delta" }
                                    span { class: "text-right", "Spent" }
                                    span { class: "text-right", "Budget remaining" }
                                    span {}
                                }
                                for (group_name, group) in groups {
                                    div { key: "grp-{group_name}", class: "proj-group", "{group_name}" }
                                    for p in group {
                                        {
                                            let (sm, sc) = spend_map.get(&p.id).copied().unwrap_or((0, 0));
                                            let rs = row_spend(&p, sm, sc);
                                            let pname = match &p.code {
                                                Some(c) => format!("[{c}] {}", p.name),
                                                None => p.name.clone(),
                                            };
                                            rsx! {
                                            div { class: "proj-row", key: "{p.id}",
                                            div { class: "flex items-center gap-3 min-w-0",
                                                Link {
                                                    to: Route::ProjectDetail { id: p.id },
                                                    class: "font-semibold proj-namelink",
                                                    "{pname}"
                                                }
                                                span { class: "badge badge-neutral", "{p.project_type.label()}" }
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
                                            span { class: "font-mono text-right text-faint", "–" }
                                            span { class: "font-mono text-right text-faint", "–" }
                                            div { class: "flex items-center justify-end gap-3 font-mono",
                                                span { class: "whitespace-nowrap", "{rs.spent}" }
                                                if let Some(pct) = rs.pct {
                                                    div { class: "proj-bar",
                                                        div { class: "proj-bar-fill", style: "width: {pct}%" }
                                                    }
                                                }
                                            }
                                            div { class: "flex items-baseline justify-end gap-2 font-mono",
                                                span { class: "whitespace-nowrap", "{rs.remaining}" }
                                                if let Some(lbl) = rs.pct_label.clone() {
                                                    span { class: "text-faint", "{lbl}" }
                                                }
                                            }
                                            div { class: "flex justify-end",
                                                if is_manager {
                                                    Menu { label: "Actions", align_right: true,
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
            })}

            if export_open() {
                {
                    let url = format!(
                        "/api/projects/export/{}?scope={}",
                        export_fmt(),
                        export_scope()
                    );
                    rsx! {
                        div {
                            class: "modal-overlay",
                            onclick: move |_| export_open.set(false),
                            div {
                                class: "modal",
                                onclick: move |e| e.stop_propagation(),
                                div { class: "modal-title", "Export projects" }
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
    }
}

#[component]
pub fn ProjectDetail(id: Uuid) -> Element {
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

            ProjectTasks { key: "{id}", project_id: id, can_manage: is_manager(&me) }

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
