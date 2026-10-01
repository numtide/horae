use dioxus::prelude::*;
use std::collections::{BTreeSet, HashMap};
use uuid::Uuid;

use super::{is_admin, is_manager, loaded, run_action};
use crate::components::combobox::{ComboOption, Combobox};
use crate::components::controls::Checkbox;
use crate::components::icons::NavIcon;
use crate::components::menu::{Menu, MenuDivider, MenuItem};
use crate::components::modal::Modal;
use crate::components::table::DataTable;
use crate::models::{Client, Project, ProjectBudgetProgress, ProjectDetails, ProjectTagLink};
use crate::route::Route;
use crate::server_fns;
use horae_core::money::format_cents;
use horae_core::types::{BudgetKind, ProjectType};

#[path = "projects/activity.rs"]
mod activity;
#[path = "projects/billing.rs"]
mod billing;
#[path = "projects/breakdown.rs"]
mod breakdown;
#[path = "projects/exports.rs"]
mod exports;
#[path = "projects/summary.rs"]
mod summary;

fn hours(minutes: i64) -> String {
    // Remaining budgets can be negative. Keep the sign and round integer
    // minutes to hundredths without the tracking formatter's zero clamp.
    let hundredths = (u128::from(minutes.unsigned_abs()) * 100 + 30) / 60;
    let fraction = format!("{:02}", hundredths % 100);
    let fraction = fraction.trim_end_matches('0');
    let sign = if minutes < 0 { "-" } else { "" };
    if fraction.is_empty() {
        format!("{sign}{}h", hundredths / 100)
    } else {
        format!("{sign}{}.{fraction}h", hundredths / 100)
    }
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
    let (budget, spent) = match p.budget_kind {
        BudgetKind::Amount => (p.budget_amount_cents, spent_cents),
        BudgetKind::Hours => (p.budget_minutes, spent_minutes),
        BudgetKind::None => (None, spent_cents),
    };
    budget_display(
        p.budget_kind,
        &p.currency,
        budget,
        Some(spent),
        p.project_type == ProjectType::Retainer,
    )
}

fn budget_display(
    kind: BudgetKind,
    currency: &str,
    budget: Option<i64>,
    spent: Option<i64>,
    recurring: bool,
) -> RowSpend {
    let format = |amount| match kind {
        BudgetKind::Hours => hours(amount),
        _ => format_cents(amount, currency.trim()),
    };
    let pct = budget
        .zip(spent)
        .and_then(|(b, s)| horae_core::budget::used_percent(s, b));
    RowSpend {
        budget: budget.map(format).unwrap_or_else(|| "—".to_string()),
        recurring,
        spent: spent
            .map(format)
            .unwrap_or_else(|| "Unavailable".to_string()),
        remaining: budget
            .zip(spent)
            .and_then(|(b, s)| b.checked_sub(s))
            .map(format)
            .unwrap_or_else(|| "—".to_string()),
        pct,
        pct_label: pct.map(|used| format!("({}%)", 100 - used)),
    }
}

fn configured_row_spend(rows: &[ProjectBudgetProgress]) -> Option<RowSpend> {
    let first = rows.first().filter(|row| row.kind != BudgetKind::None)?;
    // A partially allocated budget is not a project-wide allowance. Overflow
    // also stays unavailable rather than wrapping into a plausible total.
    let budget = rows
        .iter()
        .try_fold(0_i64, |total, row| total.checked_add(row.budget?));
    let spent = rows
        .iter()
        .try_fold(0_i64, |total, row| total.checked_add(row.consumed));
    Some(budget_display(
        first.kind,
        &first.currency,
        budget,
        spent,
        first.period_key != "lifetime",
    ))
}

#[cfg(test)]
mod budget_display_tests {
    use super::*;

    #[test]
    fn tag_filter_matches_identity_without_duplicates_or_name_collisions() {
        let tag_id = Uuid::from_u128(1);
        let project_id = Uuid::from_u128(2);
        let link = ProjectTagLink {
            project_id,
            tag_id,
            name: "Launch".into(),
        };
        let links = [
            link.clone(),
            link,
            ProjectTagLink {
                project_id: Uuid::from_u128(3),
                tag_id: Uuid::from_u128(4),
                name: "Launch".into(),
            },
        ];
        assert_eq!(
            matching_tag_projects(&links, Some(tag_id)),
            BTreeSet::from([project_id])
        );
        assert!(matching_tag_projects(&links, Some(Uuid::nil())).is_empty());
        assert!(matching_tag_projects(&[], Some(tag_id)).is_empty());
    }

    fn scope(budget: Option<i64>, consumed: i64) -> ProjectBudgetProgress {
        ProjectBudgetProgress {
            project_id: Uuid::nil(),
            task_id: Some(Uuid::nil()),
            user_id: None,
            scope: "task".to_string(),
            label: Some("Development".to_string()),
            kind: BudgetKind::Hours,
            currency: "EUR".to_string(),
            period_key: "2026-09".to_string(),
            budget,
            consumed,
        }
    }

    #[test]
    fn configured_scopes_sum_without_hiding_an_individual_overrun() {
        let rows = [scope(Some(60), 120), scope(Some(180), 0)];
        let total = configured_row_spend(&rows).unwrap();
        assert_eq!(
            (
                total.budget.as_str(),
                total.spent.as_str(),
                total.remaining.as_str()
            ),
            ("4h", "2h", "2h")
        );
        assert_eq!(
            (total.pct, total.pct_label.as_deref(), total.recurring),
            (Some(50), Some("(50%)"), true)
        );
        let first = budget_display(
            rows[0].kind,
            &rows[0].currency,
            rows[0].budget,
            Some(rows[0].consumed),
            false,
        );
        assert_eq!(first.remaining, "-1h");
        assert_eq!(
            (first.pct, first.pct_label.as_deref()),
            (Some(100), Some("(0%)"))
        );
    }

    #[test]
    fn unallocated_zero_and_overflow_budgets_are_not_conflated() {
        let unallocated = configured_row_spend(&[scope(Some(60), 30), scope(None, 60)]).unwrap();
        assert_eq!(
            (
                unallocated.budget.as_str(),
                unallocated.spent.as_str(),
                unallocated.remaining.as_str()
            ),
            ("—", "1.5h", "—")
        );
        assert_eq!(unallocated.pct, None);
        let zero = configured_row_spend(&[scope(Some(0), 60)]).unwrap();
        assert_eq!(
            (zero.budget.as_str(), zero.remaining.as_str(), zero.pct),
            ("0h", "-1h", None)
        );
        let overflow =
            configured_row_spend(&[scope(Some(i64::MAX), i64::MAX), scope(Some(1), 1)]).unwrap();
        assert_eq!(
            (
                overflow.budget.as_str(),
                overflow.spent.as_str(),
                overflow.remaining.as_str()
            ),
            ("—", "Unavailable", "—")
        );
        assert!(configured_row_spend(&[]).is_none());
        let mut no_budget = scope(None, 0);
        no_budget.kind = BudgetKind::None;
        assert!(configured_row_spend(&[no_budget]).is_none());
    }

    #[test]
    fn display_preserves_units_sign_and_large_integer_values() {
        assert_eq!(hours(-1), "-0.02h");
        assert_eq!(hours(59), "0.98h");
        assert_eq!(hours(60), "1h");
        assert_eq!(hours(i64::MIN), "-153722867280912930.13h");
        let amount = budget_display(BudgetKind::Amount, " EUR ", Some(100), Some(150), false);
        assert_eq!(
            (
                amount.budget.as_str(),
                amount.spent.as_str(),
                amount.remaining.as_str()
            ),
            ("EUR 1.00", "EUR 1.50", "EUR -0.50")
        );
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
    let mut tags_res = use_resource(|| async move { server_fns::list_project_tags().await });
    let me = use_resource(|| async move { server_fns::get_me().await });
    let mut spend_res = use_resource(|| async move { server_fns::list_project_spend().await });
    let mut budget_res =
        use_resource(|| async move { server_fns::list_project_budget_progress().await });

    let navigator = use_navigator();
    let action_error = use_signal(|| None::<String>);

    // Filters over the loaded list (client-side; the design's status/client
    // dropdowns and search all narrow the same set).
    let mut query = use_signal(String::new);
    // Status scope: "active" | "budgeted" (has a budget) | "archived" (inactive).
    let mut scope = use_signal(|| "active".to_string());
    let mut client_filter = use_signal(String::new);
    let mut tag_filter = use_signal(|| None::<Uuid>);
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
    let spend_ready = spend_res.state()() == UseResourceState::Ready
        && budget_res.state()() == UseResourceState::Ready
        && matches!(&*spend_res.read(), Some(Ok(_)))
        && matches!(&*budget_res.read(), Some(Ok(_)));

    let client_names: HashMap<Uuid, String> = match &*clients_res.read() {
        Some(Ok(cs)) => cs.iter().map(|c| (c.id, c.name.clone())).collect(),
        _ => HashMap::new(),
    };
    let query_lower = query().to_lowercase();
    // Resources retain their previous value while a restart is pending.
    let projects_loading = projects.state()() != UseResourceState::Ready;
    let tags_ready =
        tags_res.state()() == UseResourceState::Ready && matches!(&*tags_res.read(), Some(Ok(_)));
    let tags = tags_res.read();
    let tag_links = tags.as_ref().and_then(|result| result.as_ref().ok());
    let tag_projects = matching_tag_projects(
        tag_links.map(Vec::as_slice).unwrap_or_default(),
        tag_filter(),
    );
    let mut tag_options = Vec::new();
    let mut seen_tags = BTreeSet::new();
    for tag in tag_links.into_iter().flatten() {
        if seen_tags.insert(tag.tag_id) {
            tag_options.push((tag.tag_id, tag.name.clone()));
        }
    }
    let tag_label = match tag_filter() {
        Some(id) => tag_options
            .iter()
            .find(|(tag, _)| *tag == id)
            .map(|(_, name)| format!("Tag: {name}"))
            .unwrap_or_else(|| "Selected tag unavailable".into()),
        None => "All tags".into(),
    };
    let visible: Vec<Project> = projects
        .read()
        .as_ref()
        .and_then(|result| result.as_ref().ok())
        .into_iter()
        .filter(|_| !projects_loading)
        .flatten()
        .filter(|p| {
            matches_project_filters(p, &query_lower, &scope(), &client_filter(), &client_names)
                && (tag_filter().is_none() || (tags_ready && tag_projects.contains(&p.id)))
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
    let mut budget_map: HashMap<Uuid, Vec<ProjectBudgetProgress>> = HashMap::new();
    if spend_ready && let Some(Ok(rows)) = &*budget_res.read() {
        for row in rows {
            budget_map
                .entry(row.project_id)
                .or_default()
                .push(row.clone());
        }
    }
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

    rsx! {
        div {
            div { class: "page-header mb-5",
                h1 { class: "page-title text-4xl font-semibold text-strong tracking-tight", "Projects" }
                div { class: "page-actions items-center gap-4",
                    if is_manager {
                        Link { to: Route::NewProject {}, class: "btn btn-primary py-2 px-4", "New project" }
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
                if !tag_options.is_empty() || tag_filter().is_some() {
                    Menu { id: "project-tag-menu", label: tag_label, trigger_class: "text-sm px-4",
                        MenuItem { selected: tag_filter().is_none(), onclick: move |_| { selected.write().clear(); tag_filter.set(None); }, "All tags" }
                        for (tag_id, tag_name) in tag_options {
                            MenuItem { key: "{tag_id}", selected: tag_filter() == Some(tag_id), disabled: !tags_ready,
                                onclick: move |_| { selected.write().clear(); tag_filter.set(Some(tag_id)); }, "{tag_name}"
                            }
                        }
                    }
                }
                Combobox {
                    trigger_class: "text-sm px-4",
                    options: client_options,
                    value: client_filter(),
                    placeholder: "All clients",
                    all_label: "All clients",
                    onselect: move |v| { selected.write().clear(); client_filter.set(v); },
                }
            }


            if let Some(message) = action_error() {
                div { class: "alert alert-danger", role: "alert", "Could not change project status: {message}" }
            }
            if let Some(message) = bulk_success() {
                div { class: "alert alert-success", role: "status", "{message}" }
            }
            if matches!(&*spend_res.read(), Some(Err(_))) || matches!(&*budget_res.read(), Some(Err(_))) {
                    div { class: "alert alert-danger", role: "alert",
                        "Could not load project progress. Budget, spent and remaining amounts are unavailable. "
                        button { class: "btn btn-secondary btn-sm", onclick: move |_| {
                            spend_res.restart();
                            budget_res.restart();
                        }, "Retry" }
                    }
            } else if !spend_ready {
                p { class: "text-sm text-secondary mb-4", role: "status", "Loading project progress…" }
            }

            if matches!(&*tags_res.read(), Some(Err(_))) {
                div { class: "alert alert-danger", role: "alert",
                    "Could not load project tags. "
                    button { r#type: "button", class: "btn btn-secondary btn-sm", onclick: move |_| { selected.write().clear(); tags_res.restart(); }, "Retry tags" }
                }
            }
            if tag_filter().is_some() && !tags_ready {
                p { class: "text-sm text-secondary", role: "status", "Tagged projects are unavailable until tags finish loading." }
            } else if projects_loading {
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
                                        if is_manager {
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
                                    p { class: "empty-state-text empty-state-copy text-subtle m-0", "Try another search, client, tag or project status." }
                                    button {
                                        class: "btn btn-secondary",
                                        onclick: move |_| {
                                            selected.write().clear();
                                            query.set(String::new());
                                            client_filter.set(String::new());
                                            tag_filter.set(None);
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
                                            let budget_rows = budget_map.get(&p.id).map(Vec::as_slice).unwrap_or_default();
                                            let rs = configured_row_spend(budget_rows).unwrap_or_else(|| row_spend(&p, sm, sc));
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
                                            div { class: "min-w-0",
                                              div { class: "flex items-center gap-3 min-w-0",
                                                Link {
                                                    to: Route::project_detail(p.id),
                                                    class: "font-semibold text-strong min-w-0 proj-namelink",
                                                    "{pname}"
                                                }
                                                span { class: "chip px-2 flex-none whitespace-nowrap", "{p.project_type.label()}" }
                                                if !p.active {
                                                    span { class: "badge badge-neutral", "Inactive" }
                                                }
                                              }
                                              if let Some(config) = budget_rows.first().filter(|row| row.kind != BudgetKind::None) {
                                                p { class: "text-xs text-muted mt-2",
                                                    if config.period_key != "lifetime" { "Budget period: {config.period_key} · " }
                                                    "Total tracked: {hours(sm)}"
                                                }
                                                if config.scope != "project" {
                                                  details { class: "mt-2 text-xs",
                                                    summary { class: "cursor-pointer text-primary", "Budget by {config.scope}" }
                                                    ul { class: "mt-2",
                                                      for row in budget_rows {
                                                        {
                                                            let values = budget_display(row.kind, &row.currency, row.budget, Some(row.consumed), false);
                                                            let label = row.label.as_deref().unwrap_or("No allocated budget");
                                                            rsx! { li { class: "py-2",
                                                                span { class: "font-semibold", "{label}: " }
                                                                if row.budget.is_some() { "Budget {values.budget} · " } else { "No budget set · " }
                                                                "Spent {values.spent} · Remaining {values.remaining}"
                                                            } }
                                                        }
                                                      }
                                                    }
                                                  }
                                                }
                                              }
                                            }
                                            div { class: "flex items-center justify-end gap-2 font-mono",
                                                if spend_ready {
                                                    span { class: "whitespace-nowrap", "{rs.budget}" }
                                                    if rs.recurring {
                                                        span { class: "text-faint", aria_label: "Recurring budget", "⟳" }
                                                    }
                                                } else {
                                                    span { class: "text-muted", aria_label: "Budget unavailable", "—" }
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
                                                                let id = p.id;
                                                                move |_| { navigator.push(Route::EditProject { id }); }
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
                                                        to: Route::project_detail(p.id),
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
pub fn ProjectDetail(
    id: Uuid,
    from: Option<String>,
    to: Option<String>,
    tab: Option<String>,
) -> Element {
    let initial_interval = match horae_core::project_activity::ActivityRange::parse_optional(
        from.as_deref(),
        to.as_deref(),
    ) {
        Ok(range) => range.map(|range| crate::models::project::ProjectActivityInterval {
            from: range.from(),
            to: range.to(),
        }),
        Err(error) => return rsx! { p { role: "alert", class: "alert alert-danger", "{error}" } },
    };
    let initial_tab = match tab.as_deref() {
        None | Some("tasks") => breakdown::Tab::Tasks,
        Some("team") => breakdown::Tab::Team,
        Some("invoices") => breakdown::Tab::Invoices,
        _ => {
            return rsx! { p { role: "alert", class: "alert alert-danger", "Invalid project view" } };
        }
    };
    // Reset assignments, tasks and form state together when the router reuses
    // this page for another project. Keys take effect in a dynamic fragment.
    rsx! { for id in [id] { ProjectDetailContent { key: "{id}", id, initial_interval, initial_tab } } }
}

fn project_view_route(
    id: Uuid,
    interval: Option<crate::models::project::ProjectActivityInterval>,
    tab: breakdown::Tab,
) -> Route {
    Route::ProjectDetail {
        id,
        from: interval.map(|range| range.from.to_string()),
        to: interval.map(|range| range.to.to_string()),
        tab: Some(tab.label().to_lowercase()),
    }
}

#[component]
fn ProjectDetailContent(
    id: Uuid,
    initial_interval: Option<crate::models::project::ProjectActivityInterval>,
    initial_tab: breakdown::Tab,
) -> Element {
    let mut interval = use_signal(move || initial_interval);
    let mut tab = use_signal(move || initial_tab);
    let navigator = use_navigator();
    use_effect(use_reactive!(|(initial_interval, initial_tab)| {
        if *interval.peek() != initial_interval {
            interval.set(initial_interval);
        }
        if *tab.peek() != initial_tab {
            tab.set(initial_tab);
        }
    }));
    let mut details =
        use_resource(move || async move { server_fns::get_project_details(id.to_string()).await });
    let me = use_resource(|| async move { server_fns::get_me().await });
    let summary_data = use_resource(move || async move {
        server_fns::summary::get_project_summary(id.to_string()).await
    });
    let chart_budget = if summary_data.state()() == UseResourceState::Ready {
        summary_data
            .read()
            .as_ref()
            .and_then(|result| result.as_ref().ok())
            .and_then(|value| summary::chart_budget(value, interval()))
    } else {
        None
    };
    let mut invoices = use_resource(move || {
        let allowed = is_manager(&me);
        async move {
            if allowed {
                Some(server_fns::billing::get_project_invoices(id.to_string()).await)
            } else {
                None
            }
        }
    });
    let invoice_state = if !is_manager(&me) {
        billing::InvoiceHistoryState::Hidden
    } else if invoices.state()() != UseResourceState::Ready {
        billing::InvoiceHistoryState::Loading
    } else {
        match &*invoices.read() {
            Some(Some(Ok(value))) => billing::InvoiceHistoryState::Ready(value.clone()),
            Some(Some(Err(error))) => billing::InvoiceHistoryState::Failed(error.to_string()),
            _ => billing::InvoiceHistoryState::Loading,
        }
    };
    let mut status_action = use_signal(|| None::<bool>);
    let mut status_busy = use_signal(|| false);
    let mut status_error = use_signal(|| None::<String>);
    let mut status_success = use_signal(|| None::<String>);
    let status_verb = if status_action() == Some(true) {
        "Reactivate"
    } else {
        "Archive"
    };

    rsx! {
        div {
            Link {
                id: "project-detail-back",
                to: Route::ProjectList {},
                class: "btn btn-ghost text-sm text-muted font-normal border-0 px-2.5 py-1.5 -ml-2.5 min-h-control",
                span { class: "inline-flex", aria_hidden: "true", NavIcon { name: "arrow-left", class: "size-4" } }
                "Back to Projects"
            }
            section { class: "mt-5 pb-6 border-b border-light wrap-anywhere", aria_label: "Project details",
                if details.state()() != UseResourceState::Ready {
                    p { role: "status", "Loading project details…" }
                } else {
                    match &*details.read() {
                        Some(Ok(project)) => rsx! {
                            SavedProjectDetails {
                                project: project.clone(), can_manage: is_manager(&me), busy: status_busy(),
                                on_status_change: move |active| {
                                    if status_busy() { return; }
                                    status_error.set(None);
                                    status_success.set(None);
                                    status_action.set(Some(active));
                                },
                            }
                        },
                        Some(Err(error)) => rsx! {
                            p { class: "text-danger", role: "alert", "Could not load project details: {error}" }
                            button { r#type: "button", class: "btn btn-secondary btn-sm", onclick: move |_| details.restart(), "Retry details" }
                        },
                        None => rsx! {},
                    }
                }
            }

            if let Some(message) = status_success() {
                p { class: "text-sm text-primary mt-4", role: "status", "{message}" }
            }
            Modal {
                id: "project-status-dialog", labelledby: "project-status-title",
                focus_fallback: "project-detail-back",
                open: status_action().is_some(), busy: status_busy(),
                on_dismiss: move |_| status_action.set(None),
                h2 { id: "project-status-title", class: "modal-title m-0", "{status_verb} project?" }
                div { class: "modal-body",
                    if let Some(Ok(project)) = details.read().as_ref() {
                        p { class: "text-sm font-semibold wrap-anywhere", "{project.name}" }
                    }
                    p { class: "text-sm text-secondary", "Only project status changes. Existing time entries, invoices and budgets are kept." }
                    if let Some(message) = status_error() {
                        p { class: "alert alert-danger", role: "alert", "Could not confirm project status: {message}. You can retry safely or cancel and refresh." }
                    }
                    div { class: "modal-actions",
                        button {
                            id: "project-status-confirm", r#type: "button", class: "btn btn-primary min-h-control",
                            disabled: status_busy() || !is_manager(&me),
                            onclick: move |_| {
                                if status_busy() || !is_manager(&me) { return; }
                                let Some(active) = status_action() else { return; };
                                status_busy.set(true);
                                status_error.set(None);
                                spawn(async move {
                                    match server_fns::set_project_active(id.to_string(), active).await {
                                        Ok(project) => {
                                            // Keep the menu mounted so the dialog can restore focus.
                                            if let Some(Ok(current)) = details.write().as_mut() {
                                                current.active = project.active;
                                            }
                                            status_action.set(None);
                                            status_success.set(Some(if active { "Project reactivated." } else { "Project archived." }.into()));
                                        }
                                        Err(error) => status_error.set(Some(error.to_string())),
                                    }
                                    status_busy.set(false);
                                });
                            },
                            if status_busy() { "Updating project…" } else { "{status_verb} project" }
                        }
                        button {
                            r#type: "button", class: "btn btn-secondary min-h-control", disabled: status_busy(),
                            onclick: move |_| status_action.set(None), "Cancel"
                        }
                    }
                }
            }

            activity::ProjectActivityPanel { project_id: id, interval, budget: chart_budget }
            summary::ProjectSummaryPanel { project_id: id, can_manage: is_manager(&me), summary: summary_data,
                billing::InvoiceSummary { state: invoice_state.clone(), on_retry: move |_| invoices.restart() }
            }
            breakdown::ProjectBreakdownPanel { project_id: id, interval, invoice_state,
                can_report: is_manager(&me), initial_tab: tab(),
                on_tab_change: move |value| { tab.set(value); navigator.replace(project_view_route(id, interval(), value)); },
                on_interval_change: move |value| { navigator.replace(project_view_route(id, value, tab())); },
                on_invoice_retry: move |_| invoices.restart() }

        }
    }
}

fn matching_tag_projects(links: &[ProjectTagLink], selected: Option<Uuid>) -> BTreeSet<Uuid> {
    links
        .iter()
        .filter(|link| Some(link.tag_id) == selected)
        .map(|link| link.project_id)
        .collect()
}

#[component]
fn SavedProjectDetails(
    project: ProjectDetails,
    can_manage: bool,
    busy: bool,
    on_status_change: EventHandler<bool>,
) -> Element {
    let title = match project.code.as_deref().filter(|code| !code.is_empty()) {
        Some(code) => format!("[{code}] {}", project.name),
        None => project.name.clone(),
    };
    rsx! {
        header { class: "flex flex-col lg:flex-row items-end gap-4",
            div { class: "flex-1 min-w-0 w-full",
                if can_manage {
                    Link { to: Route::ClientDetail { id: project.client_id }, class: "text-sm font-semibold", "{project.client_name}" }
                } else {
                    span { class: "text-sm font-semibold text-muted", "{project.client_name}" }
                }
                div { class: "flex flex-wrap items-center gap-3 mt-2",
                    h1 { class: "font-display text-4xl font-semibold text-strong tracking-tight m-0", "{title}" }
                    span { class: "chip chip-plain whitespace-nowrap", "{project.project_type.label()}" }
                }
                p { class: "text-sm text-muted m-0 mt-2",
                    if project.active { "Active" } else { "Archived" }
                    " · {project.currency}"
                }
            }
            if can_manage {
                div { class: "flex flex-wrap items-center gap-3",
                    Link { to: Route::EditProject { id: project.id }, class: "btn btn-secondary min-h-control", "Edit project" }
                    Menu { id: "project-detail-actions", label: "Actions", align_right: true,
                        trigger_class: "min-h-control", disabled: busy,
                        MenuItem { disabled: busy, onclick: move |_| on_status_change.call(!project.active),
                            if project.active { "Archive" } else { "Reactivate" }
                        }
                    }
                }
            }
        }
        details { class: "mt-4 text-sm",
            summary { class: "cursor-pointer text-muted", "Project information" }
            dl { class: "grid sm:grid-cols-2 gap-4 m-0 mt-4",
                div { dt { class: "text-sm text-subtle", "Currency" } dd { class: "m-0", "{project.currency}" } }
                div { dt { class: "text-sm text-subtle", "Planning dates" }
                    dd { class: "m-0",
                        "{project.starts_on.map(|date| date.format(\"%d %b %Y\").to_string()).unwrap_or_else(|| \"No start date\".into())} — "
                        "{project.ends_on.map(|date| date.format(\"%d %b %Y\").to_string()).unwrap_or_else(|| \"No end date\".into())}"
                    }
                }
            }
            div { class: "mt-4",
                h3 { class: "text-sm text-subtle m-0 mb-2", "Tags" }
                if project.tags.is_empty() { p { class: "text-sm m-0", "No tags" } }
                else { div { class: "flex flex-wrap gap-2", for tag in project.tags { span { class: "chip", "{tag}" } } } }
            }
            if let Some(notes) = project.admin_notes.filter(|notes| !notes.is_empty()) {
                div { class: "mt-4",
                    h3 { class: "text-sm text-subtle m-0 mb-2", "Administrator notes" }
                    for line in notes.lines() { p { class: "text-sm m-0", "{line}" } }
                }
            }
        }
    }
}
