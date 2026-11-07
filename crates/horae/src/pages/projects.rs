use dioxus::prelude::*;
use std::collections::{BTreeSet, HashMap};
use std::rc::Rc;
use uuid::Uuid;

use super::run_action;
use crate::components::badge::Badge;
use crate::components::combobox::{ComboOption, Combobox};
use crate::components::controls::Checkbox;
use crate::components::icons::NavIcon;
use crate::components::menu::{Menu, MenuDivider, MenuItem};
use crate::components::modal::Modal;
use crate::components::toast::{Toast, ToastContainer};
use crate::models::permission_editor::PermissionRequester;
use crate::models::project::ProjectOverview;
use crate::models::{Project, ProjectBudgetOverview, ProjectDetails, ProjectTagLink};
use crate::route::{EditReturn, Route};
use crate::server_fns;
use horae_core::money::format_cents;
use horae_core::types::{BudgetKind, ProjectType};

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

fn row_spend(p: &Project, spent_minutes: i64, spent_cents: Option<i64>) -> RowSpend {
    let (budget, spent) = match p.budget_kind {
        BudgetKind::Amount => (p.budget_amount_cents, spent_cents),
        BudgetKind::Hours => (p.budget_minutes, Some(spent_minutes)),
        BudgetKind::None => (None, spent_cents),
    };
    budget_display(
        p.budget_kind,
        &p.currency,
        budget,
        spent,
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

fn configured_row_spend(progress: &ProjectBudgetOverview) -> Option<RowSpend> {
    if progress.kind == BudgetKind::None {
        return None;
    }
    Some(budget_display(
        progress.kind,
        &progress.currency,
        progress.budget,
        progress.consumed,
        progress.period_key != "lifetime",
    ))
}

#[cfg(test)]
mod budget_display_tests {
    use super::*;
    use crate::models::ProjectBudgetProgress;

    #[test]
    fn project_spend_keeps_withheld_money_distinct_from_zero_and_visible_hours() {
        let mut project = Project {
            id: Uuid::nil(),
            org_id: Uuid::nil(),
            client_id: Uuid::nil(),
            code: None,
            name: "Project".into(),
            project_type: ProjectType::TimeAndMaterials,
            currency: "EUR".into(),
            rate_cents: None,
            starts_on: None,
            ends_on: None,
            budget_kind: BudgetKind::Amount,
            budget_amount_cents: Some(10000),
            budget_minutes: Some(600),
            active: true,
            created_at: chrono::Utc::now(),
        };
        let hidden = row_spend(&project, 60, None);
        assert_eq!(hidden.spent, "Unavailable");
        assert_eq!(hidden.remaining, "—");
        assert!(hidden.pct.is_none());
        let zero = row_spend(&project, 60, Some(0));
        assert_eq!(zero.spent, format_cents(0, "EUR"));
        assert_eq!(zero.remaining, format_cents(10000, "EUR"));
        assert_eq!(zero.pct, Some(0));
        project.budget_kind = BudgetKind::Hours;
        let hours = row_spend(&project, 60, None);
        assert_eq!(
            (hours.spent.as_str(), hours.remaining.as_str()),
            ("1h", "9h")
        );
        assert_eq!(hours.pct, Some(10));
    }

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

    fn summary(budget: Option<i64>, consumed: Option<i64>) -> ProjectBudgetOverview {
        ProjectBudgetOverview {
            project_id: Uuid::nil(),
            scope: "person".into(),
            kind: BudgetKind::Hours,
            currency: "EUR".into(),
            period_key: "2026-09".into(),
            budget,
            consumed,
            breakdown: vec![],
        }
    }

    #[test]
    fn configured_summary_does_not_recompute_totals_from_private_breakdown() {
        let mut progress = summary(Some(240), Some(120));
        progress.breakdown.push(scope(Some(60), 120));
        let total = configured_row_spend(&progress).unwrap();
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
            progress.breakdown[0].kind,
            &progress.breakdown[0].currency,
            progress.breakdown[0].budget,
            Some(progress.breakdown[0].consumed),
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
        let unallocated = configured_row_spend(&summary(None, Some(0))).unwrap();
        assert_eq!(
            (
                unallocated.budget.as_str(),
                unallocated.spent.as_str(),
                unallocated.remaining.as_str()
            ),
            ("—", "0h", "—")
        );
        assert_eq!(unallocated.pct, None);
        let zero = configured_row_spend(&summary(Some(0), Some(60))).unwrap();
        assert_eq!(
            (zero.budget.as_str(), zero.remaining.as_str(), zero.pct),
            ("0h", "-1h", None)
        );
        let overflow = configured_row_spend(&summary(None, None)).unwrap();
        assert_eq!(
            (
                overflow.budget.as_str(),
                overflow.spent.as_str(),
                overflow.remaining.as_str()
            ),
            ("—", "Unavailable", "—")
        );
        let mut no_budget = summary(None, Some(0));
        no_budget.kind = BudgetKind::None;
        assert!(configured_row_spend(&no_budget).is_none());
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

/// One loaded project with the lowercase keys search and ordering compare.
#[derive(PartialEq)]
struct ListedProject {
    project: Project,
    /// "[code] name", as rows and confirmations show it.
    label: String,
    name_key: String,
    client_name: Option<String>,
    client_key: String,
}

impl ListedProject {
    /// `query` is already lowercase.
    fn matches(&self, query: &str, scope: &str, client: &str) -> bool {
        let project = &self.project;
        let scope_matches = match scope {
            "budgeted" => project.active && project.budget_kind != BudgetKind::None,
            "archived" => !project.active,
            _ => project.active,
        };
        scope_matches
            && (client.is_empty() || project.client_id.to_string() == client)
            && (query.is_empty()
                || self.name_key.contains(query)
                || self.client_key.contains(query))
    }
}

/// Everything the list derives from one overview, built once per load so that
/// typing and selecting neither re-sort the rows nor lowercase every name.
#[derive(Default, PartialEq)]
struct ProjectCatalog {
    /// Grouped by client identity, ordered by client name then project name;
    /// the ID keeps same-named clients apart.
    rows: Vec<ListedProject>,
    editable: BTreeSet<Uuid>,
    active_count: usize,
    budgeted_count: usize,
    archived_count: usize,
    /// Client filter choices, grouped Active / Archived.
    client_options: Vec<ComboOption>,
}

impl ProjectCatalog {
    fn new(overview: &ProjectOverview) -> Self {
        let client_names: HashMap<Uuid, &str> = overview
            .projects
            .iter()
            .filter_map(|row| row.client.as_ref())
            .map(|client| (client.id, client.name.as_str()))
            .collect();
        let mut rows: Vec<ListedProject> = overview
            .projects
            .iter()
            .map(|row| {
                let project = row.project.clone();
                let client_name = client_names
                    .get(&project.client_id)
                    .map(|name| name.to_string());
                ListedProject {
                    label: match &project.code {
                        Some(code) => format!("[{code}] {}", project.name),
                        None => project.name.clone(),
                    },
                    name_key: project.name.to_lowercase(),
                    client_key: client_name.as_deref().unwrap_or_default().to_lowercase(),
                    client_name,
                    project,
                }
            })
            .collect();
        rows.sort_by(|a, b| {
            (&a.client_key, a.project.client_id, &a.name_key).cmp(&(
                &b.client_key,
                b.project.client_id,
                &b.name_key,
            ))
        });

        let mut seen_clients = BTreeSet::new();
        let mut clients: Vec<_> = overview
            .projects
            .iter()
            .filter_map(|row| row.client.as_ref())
            .filter(|client| seen_clients.insert(client.id))
            .collect();
        clients
            .sort_by_cached_key(|client| (!client.active, client.name.to_lowercase(), client.id));
        let client_options = clients
            .into_iter()
            .map(|client| {
                ComboOption::grouped(
                    client.id.to_string(),
                    client.name.clone(),
                    if client.active {
                        "Active clients"
                    } else {
                        "Archived clients"
                    },
                )
            })
            .collect();

        let projects = || overview.projects.iter().map(|row| &row.project);
        Self {
            editable: overview
                .projects
                .iter()
                .filter(|row| row.can_edit)
                .map(|row| row.project.id)
                .collect(),
            active_count: projects().filter(|p| p.active).count(),
            budgeted_count: projects()
                .filter(|p| p.active && p.budget_kind != BudgetKind::None)
                .count(),
            archived_count: projects().filter(|p| !p.active).count(),
            rows,
            client_options,
        }
    }
}

/// The client-side filters over a loaded list. `tagged` is `None` for all tags,
/// otherwise the projects carrying the selected tag (empty until tags load).
#[derive(Clone, Copy)]
struct ProjectFilters {
    query: Signal<String>,
    /// "active" | "budgeted" (has a budget) | "archived" (inactive).
    scope: Signal<String>,
    client: Signal<String>,
    tagged: Memo<Option<BTreeSet<Uuid>>>,
}

/// The catalog for the admitted overview and the indices of its rows that pass
/// `filters`, in display order. The catalog is rebuilt only when a reload admits
/// a new overview, so filters and selection survive it. A filter change rescans
/// the prepared rows without re-sorting them or lowercasing every name.
fn use_project_list(
    admitted: Signal<Option<ProjectOverview>>,
    filters: ProjectFilters,
) -> (Rc<ProjectCatalog>, Memo<Vec<usize>>) {
    // Empty only for the render in which a dropped overview unmounts the list.
    let catalog = use_memo(move || {
        Rc::new(
            admitted
                .read()
                .as_ref()
                .map(ProjectCatalog::new)
                .unwrap_or_default(),
        )
    });
    let visible = use_memo(move || {
        let catalog = catalog.read();
        let query = filters.query.read().to_lowercase();
        let (scope, client, tagged) = (
            filters.scope.read(),
            filters.client.read(),
            filters.tagged.read(),
        );
        catalog
            .rows
            .iter()
            .enumerate()
            .filter(|(_, row)| {
                row.matches(&query, &scope, &client)
                    && tagged
                        .as_ref()
                        .is_none_or(|ids| ids.contains(&row.project.id))
            })
            .map(|(index, _)| index)
            .collect()
    });
    (catalog(), visible)
}

#[derive(Clone)]
struct BulkProjectAction {
    activate: bool,
    projects: Vec<(Uuid, String)>,
}

#[component]
pub fn ProjectsForClient(client: String) -> Element {
    rsx! { for client in [client] { ClientProjectList { key: "{client}", client: client.clone() } } }
}

#[component]
fn ClientProjectList(client: String) -> Element {
    let mut context = use_resource(move || {
        let client = client.clone();
        async move {
            let id = client
                .parse::<Uuid>()
                .map_err(|_| ServerFnError::new("Invalid client link."))?;
            server_fns::get_client_details(id.to_string()).await?;
            Ok::<_, ServerFnError>(id)
        }
    });
    match &*context.read() {
        Some(Ok(id)) => rsx! { ProjectList { initial_client: Some(*id) } },
        Some(Err(error)) => rsx! {
            h1 { class: "page-title", "Projects" }
            div { class: "alert alert-danger", role: "alert", "Could not load this client's projects: {error}" }
            button { class: "btn btn-secondary", onclick: move |_| context.restart(), "Retry" }
            Link { to: Route::ProjectList {}, class: "btn btn-ghost", "Back to Projects" }
        },
        None => rsx! { p { role: "status", "Loading client…" } },
    }
}

fn restore_project_list_focus() {
    document::eval(
        "const active = document.activeElement;
        if (!active || active === document.body ||
            ['project-list-heading', 'projects-retry-access'].includes(active.id)) {
            (document.getElementById('project-scope-menu-trigger') ||
             document.getElementById('projects-retry-access') ||
             document.getElementById('project-list-heading'))?.focus();
        }",
    );
}

#[component]
pub fn ProjectList(#[props(default)] initial_client: Option<Uuid>) -> Element {
    let mut binding = use_signal(|| None::<(PermissionRequester, bool)>);
    // The last overview admitted under `binding`. It stays on screen while a
    // reload runs or after one fails, so the content below keeps its filters
    // and selection. A denied or changed requester drops it, and the content
    // remounts with fresh auxiliary reads once access is admitted again.
    let mut admitted = use_signal(|| None::<ProjectOverview>);
    let bulk_success = use_signal(|| None::<String>);
    let mut projects = use_resource(move || async move {
        let expected = *binding.peek();
        let response = match server_fns::get_project_overview(
            expected.map(|(requester, _)| requester),
        )
        .await
        {
            Ok(response) => response,
            Err(error) => {
                if access_denied(&error) {
                    admitted.set(None);
                }
                return Err(error);
            }
        };
        let current = (response.requester, response.canonical_permissions);
        if expected.is_some_and(|expected| expected != current) {
            admitted.set(None);
            return Err(ServerFnError::new(
                "Your session or project policy changed. Reload this page to continue.",
            ));
        }
        binding.set(Some(current));
        admitted.set(Some(response));
        Ok(())
    });
    if let Some(overview) = admitted() {
        return rsx! {
            ProjectListContent { overview, projects, admitted, bulk_success, initial_client }
        };
    }
    // Nothing is admitted: the first load, an access refresh or a refused read.
    match (&*projects.read(), projects.state()()) {
        (Some(Err(_)), UseResourceState::Ready) => rsx! {
            div { class: "page-header mb-5", h1 { class: "page-title text-4xl font-semibold text-strong tracking-tight", "Projects" } }
            if let Some(message) = bulk_success() {
                div { class: "alert alert-success", role: "status", "{message}" }
            }
            div { class: "alert alert-danger", role: "alert", "Project access is unavailable. Sign in again or reload this page if your account changed." }
            button { id: "projects-retry-access", r#type: "button", class: "btn btn-secondary",
                onmounted: move |_| { if bulk_success.peek().is_some() { restore_project_list_focus(); } },
                onclick: move |_| projects.restart(), "Retry access"
            }
        },
        _ => rsx! {
            div { class: "page-header mb-5",
                onmounted: move |_| { if bulk_success.peek().is_some() { restore_project_list_focus(); } },
                h1 { id: "project-list-heading", tabindex: "-1", class: "page-title text-4xl font-semibold text-strong tracking-tight", "Projects" }
            }
            if let Some(message) = bulk_success() {
                div { class: "alert alert-success", role: "status", "{message}" }
            }
            p { class: "text-sm text-secondary", role: "status", "Loading projects…" }
        },
    }
}

const UNAUTHORIZED: u16 = 401;
const FORBIDDEN: u16 = 403;

fn access_denied(error: &ServerFnError) -> bool {
    matches!(
        error,
        ServerFnError::ServerError {
            code: UNAUTHORIZED | FORBIDDEN,
            ..
        }
    )
}

#[component]
fn ProjectListContent(
    initial_client: Option<Uuid>,
    overview: ProjectOverview,
    mut projects: Resource<Result<(), ServerFnError>>,
    mut admitted: Signal<Option<ProjectOverview>>,
    mut bulk_success: Signal<Option<String>>,
) -> Element {
    let requester = overview.requester;
    let mut tags_res =
        use_resource(move || async move { server_fns::list_project_tags(Some(requester)).await });
    let mut spend_res =
        use_resource(move || async move { server_fns::list_project_spend(Some(requester)).await });
    let mut budget_res = use_resource(move || async move {
        server_fns::list_project_budget_progress(Some(requester)).await
    });

    let navigator = use_navigator();
    let action_error = use_signal(|| None::<String>);

    // Filters over the loaded list (client-side; the design's status/client
    // dropdowns and search all narrow the same set).
    let mut query = use_signal(String::new);
    // Status scope: "active" | "budgeted" (has a budget) | "archived" (inactive).
    let mut scope = use_signal(|| "active".to_string());
    let mut client_filter =
        use_signal(|| initial_client.map(|id| id.to_string()).unwrap_or_default());
    let mut tag_filter = use_signal(|| None::<Uuid>);
    let mut selected = use_signal(BTreeSet::<Uuid>::new);
    let mut bulk_action = use_signal(|| None::<BulkProjectAction>);
    let mut bulk_busy = use_signal(|| false);
    let mut bulk_error = use_signal(|| None::<String>);
    // Export modal: open state + chosen scope and format.
    let mut export_open = use_signal(|| false);
    let mut export_scope = use_signal(|| "active".to_string());
    let mut export_fmt = use_signal(|| "csv".to_string());

    let can_import = overview.can_import;
    let can_create = overview.can_create;
    let is_manager = overview.can_change_legacy_status;
    let tagged = use_memo(move || {
        tag_filter().map(|tag| {
            let tags = tags_res.read();
            let ready = tags_res.state()() == UseResourceState::Ready;
            match tags.as_ref().filter(|_| ready) {
                Some(Ok(links)) => matching_tag_projects(links, Some(tag)),
                _ => BTreeSet::new(),
            }
        })
    });
    let (catalog, visible) = use_project_list(
        admitted,
        ProjectFilters {
            query,
            scope,
            client: client_filter,
            tagged,
        },
    );
    let spend_ready = spend_res.state()() == UseResourceState::Ready
        && budget_res.state()() == UseResourceState::Ready
        && matches!(&*spend_res.read(), Some(Ok(_)))
        && matches!(&*budget_res.read(), Some(Ok(_)));

    // A reload keeps `overview`, the last admitted list, on screen until it settles.
    let projects_loading = projects.state()() != UseResourceState::Ready;
    // Bulk actions apply only to rows on screen and current: not to a held
    // list while it reloads, nor to rows a failed reload has hidden.
    let bulk_locked = bulk_busy() || projects_loading || matches!(&*projects.read(), Some(Err(_)));
    let tags_ready =
        tags_res.state()() == UseResourceState::Ready && matches!(&*tags_res.read(), Some(Ok(_)));
    let tag_options = use_memo(move || {
        let mut seen_tags = BTreeSet::new();
        tags_res
            .read()
            .as_ref()
            .and_then(|result| result.as_ref().ok())
            .into_iter()
            .flatten()
            .filter(|tag| seen_tags.insert(tag.tag_id))
            .map(|tag| (tag.tag_id, tag.name.clone()))
            .collect::<Vec<_>>()
    });
    let tag_label = match tag_filter() {
        Some(id) => tag_options
            .read()
            .iter()
            .find(|(tag, _)| *tag == id)
            .map(|(_, name)| format!("Tag: {name}"))
            .unwrap_or_else(|| "Selected tag unavailable".into()),
        None => "All tags".into(),
    };
    let selection: Vec<(Uuid, String)> = visible
        .read()
        .iter()
        .map(|&index| &catalog.rows[index])
        .filter(|row| selected.read().contains(&row.project.id))
        .map(|row| (row.project.id, row.label.clone()))
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
    // Missing/withheld money is not a zero. Empty authorized projects have explicit totals.
    let spend_map = use_memo(move || match &*spend_res.read() {
        Some(Ok(v)) => v
            .iter()
            .map(|s| (s.project_id, (s.spent_minutes, s.spent_cents)))
            .collect(),
        _ => HashMap::<Uuid, (i64, Option<i64>)>::new(),
    });
    let budget_map = use_memo(move || {
        let ready = spend_res.state()() == UseResourceState::Ready
            && budget_res.state()() == UseResourceState::Ready
            && matches!(&*spend_res.read(), Some(Ok(_)));
        match &*budget_res.read() {
            Some(Ok(rows)) if ready => rows
                .iter()
                .map(|row| (row.project_id, row.clone()))
                .collect(),
            _ => HashMap::<Uuid, ProjectBudgetOverview>::new(),
        }
    });
    let (active_count, budgeted_count, archived_count) = (
        catalog.active_count,
        catalog.budgeted_count,
        catalog.archived_count,
    );
    let scope_label = match scope().as_str() {
        "budgeted" => format!("Budgeted projects ({budgeted_count})"),
        "archived" => format!("Archived projects ({archived_count})"),
        _ => format!("Active projects ({active_count})"),
    };

    if matches!(&*tags_res.read(), Some(Err(error)) if access_denied(error))
        || matches!(&*spend_res.read(), Some(Err(error)) if access_denied(error))
        || matches!(&*budget_res.read(), Some(Err(error)) if access_denied(error))
    {
        return rsx! {
            div { class: "page-header mb-5", h1 { class: "page-title text-4xl font-semibold text-strong tracking-tight", "Projects" } }
            div { class: "alert alert-danger", role: "alert", "Your session or project access changed. Refresh access before continuing." }
            // Dropping the admitted list remounts this content, and so its
            // auxiliary reads, only once the reload admits access again.
            button { id: "projects-refresh-access", r#type: "button", class: "btn btn-secondary",
                onclick: move |_| { admitted.set(None); projects.restart(); }, "Refresh access"
            }
        };
    }

    rsx! {
        div {
            onmounted: move |_| { if bulk_success.peek().is_some() { restore_project_list_focus(); } },
            div { class: "page-header mb-5",
                h1 { class: "page-title text-4xl font-semibold text-strong tracking-tight", "Projects" }
                div { class: "page-actions items-center gap-4",
                    if can_create {
                        Link { to: Route::NewProject {}, class: "btn btn-primary py-2 px-4", "New project" }
                    }
                    if is_manager {
                        Menu {
                            id: "project-bulk-menu", label: "⚡ Actions", align_right: true,
                            trigger_class: "text-sm py-2 px-4",
                            disabled: selected_count == 0 || bulk_locked,
                            div { class: "px-3 pt-1 pb-2 text-xs uppercase text-label", "{selection_label}" }
                            if selected_count > 100 {
                                p { class: "px-3 text-sm text-secondary", "Select at most 100 projects" }
                            }
                            MenuItem {
                                disabled: selected_count == 0 || selected_count > 100 || bulk_locked,
                                onclick: move |_| {
                                    if selection.is_empty() || selection.len() > 100 || bulk_locked { return; }
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
                        Link { to: Route::ImporterIndex {}, class: "btn btn-secondary py-2 px-4", "Import" }
                    }
                    button {
                        r#type: "button",
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
                if !tag_options.read().is_empty() || tag_filter().is_some() {
                    Menu { id: "project-tag-menu", label: tag_label, trigger_class: "text-sm px-4",
                        MenuItem { selected: tag_filter().is_none(), onclick: move |_| { selected.write().clear(); tag_filter.set(None); }, "All tags" }
                        for (tag_id, tag_name) in tag_options() {
                            MenuItem { key: "{tag_id}", selected: tag_filter() == Some(tag_id), disabled: !tags_ready,
                                onclick: move |_| { selected.write().clear(); tag_filter.set(Some(tag_id)); }, "{tag_name}"
                            }
                        }
                    }
                }
                Combobox {
                    trigger_class: "text-sm px-4",
                    options: catalog.client_options.clone(),
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
            } else if !projects_loading && matches!(&*projects.read(), Some(Err(_))) {
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
            div {
                class: "proj-content",
                inert: projects_loading.then_some("true"),
                "aria-busy": projects_loading.then_some("true"),
                {
                    let list = &overview.projects;
                    let visible = visible.read();
                    let visible_ids: BTreeSet<Uuid> = visible.iter().map(|&index| catalog.rows[index].project.id).collect();
                    let all_selected = !visible_ids.is_empty() && selected_count == visible_ids.len();
                    // The catalog is already in display order, so consecutive rows
                    // of one client form its group.
                    let mut groups: Vec<(Uuid, Option<&str>, Vec<&ListedProject>)> = Vec::new();
                    for row in visible.iter().map(|&index| &catalog.rows[index]) {
                        match groups.last_mut() {
                            Some((id, _, v)) if *id == row.project.client_id => v.push(row),
                            _ => groups.push((row.project.client_id, row.client_name.as_deref(), vec![row])),
                        }
                    }
                    // Client Detail admits every legacy reader to their organization's
                    // clients; the overview carries no policy-one client grant.
                    let link_clients = !overview.canonical_permissions;
                    if groups.is_empty() {
                        rsx! {
                            div { class: "empty-state bg-secondary border rounded-xl px-6 justify-center",
                                span { class: "empty-state-icon empty-state-icon-tile", aria_hidden: "true", NavIcon { name: "briefcase" } }
                                if list.is_empty() {
                                    h2 { class: "empty-state-title text-xl m-0", "No projects yet" }
                                    p { class: "empty-state-text empty-state-copy text-subtle m-0",
                                        if can_create {
                                            "Projects hold the tasks your team tracks against. Create one to get started."
                                        } else {
                                            "Projects hold the tasks your team tracks against. Ask a manager to create your first project."
                                        }
                                    }
                                    div { class: "flex flex-wrap items-center justify-center gap-3 mt-1",
                                        if can_create {
                                            Link {
                                                to: Route::NewProject {},
                                                class: "btn btn-primary py-3 px-5",
                                                "New project"
                                            }
                                        }
                                        if can_import {
                                            Link { to: Route::ImporterIndex {}, class: "text-sm font-semibold", "Import from Harvest →" }
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
                                for (client_id, client_name, group) in groups {
                                    div { key: "grp-{client_id}", class: "proj-group py-3 px-5 text-sm font-semibold text-primary",
                                        if let Some(name) = client_name.filter(|_| link_clients) {
                                            Link { to: Route::ClientDetail { id: client_id }, "{name}" }
                                        } else if let Some(name) = client_name {
                                            "{name}"
                                        } else {
                                            "Unknown client"
                                        }
                                    }
                                    for row in group {
                                        {
                                            let p = &row.project;
                                            let id = p.id;
                                            let (sm, sc) = spend_map.read().get(&id).copied().unwrap_or((0, None));
                                            let budgets = budget_map.read();
                                            let budget_config = budgets.get(&id);
                                            let rs = budget_config.and_then(configured_row_spend).unwrap_or_else(|| row_spend(p, sm, sc));
                                            let pname = &row.label;
                                            rsx! {
                                            div { class: "proj-row grid items-center py-4 px-5 text-sm", key: "{p.id}",
                                            if is_manager {
                                                div { class: "flex items-center",
                                                    Checkbox {
                                                        checked: selected.read().contains(&p.id), compact: true,
                                                        label: "Select {pname}", disabled: bulk_busy(),
                                                        onclick: move |_| {
                                                            let mut ids = selected.write();
                                                            if !ids.remove(&id) { ids.insert(id); }
                                                        },
                                                    }
                                                }
                                            }
                                            div { class: "min-w-0",
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
                                              if let Some(config) = budget_config.filter(|row| row.kind != BudgetKind::None) {
                                                p { class: "text-xs text-muted mt-2",
                                                    if config.period_key != "lifetime" { "Budget period: {config.period_key} · " }
                                                    "Total tracked: {hours(sm)}"
                                                }
                                                if config.scope != "project" && !config.breakdown.is_empty() {
                                                  details { class: "mt-2 text-xs",
                                                    summary { class: "cursor-pointer text-primary", "Budget by {config.scope}" }
                                                    ul { class: "mt-2",
                                                      for row in &config.breakdown {
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
                                                if catalog.editable.contains(&p.id) || is_manager {
                                                    Menu { id: "project-actions-{p.id}", label: "Actions", align_right: true,
                                                        if catalog.editable.contains(&p.id) {
                                                        MenuItem {
                                                            onclick: {
                                                                let id = p.id;
                                                                move |_| { navigator.push(Route::EditProject { id, return_to: EditReturn::default() }); }
                                                            },
                                                            "Edit"
                                                        }
                                                        }
                                                        if is_manager {
                                                        MenuDivider {}
                                                        MenuItem {
                                                            onclick: {
                                                                let id = p.id;
                                                                let next_active = !p.active;
                                                                move |_| run_action(
                                                                    server_fns::set_project_active(id.to_string(), next_active, Some(requester)),
                                                                    projects,
                                                                    action_error,
                                                                    || (),
                                                                )
                                                            },
                                                            if p.active { "Archive" } else { "Unarchive" }
                                                        }
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
                }
            }
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
                                    match server_fns::set_projects_active(ids, action.activate, Some(requester)).await {
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
                        "/api/projects/export/{}?scope={}&expected_org_id={}&expected_user_id={}",
                        export_fmt(),
                        export_scope(),
                        requester.org_id,
                        requester.user_id,
                    );
                    rsx! {
                        h2 { id: "export-projects-title", class: "modal-title m-0", "Export projects" }
                        div { class: "modal-body",
                            div { id: "export-projects-scope", class: "modal-label", "Which projects?" }
                            div { class: "seg-row", role: "group", aria_labelledby: "export-projects-scope",
                                for (val , lbl) in [("active", "Active"), ("budgeted", "Budgeted"), ("archived", "Archived")] {
                                    button {
                                        r#type: "button",
                                        class: if export_scope() == val { "seg-btn selected" } else { "seg-btn" },
                                        "aria-pressed": export_scope() == val,
                                        onclick: move |_| export_scope.set(val.to_string()),
                                        "{lbl}"
                                    }
                                }
                            }
                            div { id: "export-projects-format", class: "modal-label", "Format" }
                            div { class: "seg-row", role: "group", aria_labelledby: "export-projects-format",
                                for (val , lbl) in [("csv", "CSV"), ("xlsx", "Excel")] {
                                    button {
                                        r#type: "button",
                                        class: if export_fmt() == val { "seg-btn selected" } else { "seg-btn" },
                                        "aria-pressed": export_fmt() == val,
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
                                    r#type: "button",
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

/// A status change the server refuses outright, such as restoring a project
/// whose client is archived.
const CONFLICT: u16 = 409;

#[component]
fn ProjectDetailContent(id: Uuid) -> Element {
    let mut binding = use_signal(|| None::<(PermissionRequester, bool)>);
    let mut access_lost = use_signal(|| false);
    let mut details = use_resource(move || async move {
        let expected = *binding.peek();
        let view = server_fns::get_project_detail_view(
            id.to_string(),
            expected.map(|(requester, _)| requester),
        )
        .await?;
        let current = (view.requester, view.canonical_permissions);
        if view.project.id != id || expected.is_some_and(|expected| expected != current) {
            return Err(ServerFnError::new("Project access changed"));
        }
        binding.set(Some(current));
        access_lost.set(false);
        Ok(view)
    });
    // The requested status; the dialog is open while it is set.
    let mut status_change = use_signal(|| None::<bool>);
    let mut status_busy = use_signal(|| false);
    let mut status_failed = use_signal(|| false);
    // A definite refusal, with the client to open when it must be restored first.
    let mut status_refused = use_signal(|| None::<(String, Option<Uuid>)>);
    // One-shot: move focus to a refusal shown outside the dialog once it renders,
    // only when focus was lost with Undo's button; any focused control, even
    // Back, is the user's choice and stays.
    let mut focus_refusal = use_signal(|| false);
    use_effect(move || {
        if status_refused().is_some() && status_change().is_none() && focus_refusal() {
            focus_refusal.set(false);
            document::eval(
                "requestAnimationFrame(() => {
                    const active = document.activeElement;
                    if (!active || active === document.body) {
                        document.getElementById('project-detail-status-refusal')?.focus();
                    }
                });",
            );
        }
    });
    // The status a change from this view saved; only an archive offers Undo.
    let mut status_saved = use_signal(|| None::<bool>);
    let mut undo_failed = use_signal(|| false);
    let mut refocus_actions = use_signal(|| false);
    // The confirmation and Undo both write the absolute status under the bound
    // requester. A failure shows where it was requested: the dialog or the toast.
    let mut write_status = move |active: bool| {
        let Some((requester, _)) = *binding.peek() else {
            return;
        };
        if status_busy() {
            return;
        }
        let undoing = status_change().is_none();
        let client_id = match &*details.peek() {
            Some(Ok(view)) => Some(view.client_id),
            _ => None,
        };
        status_busy.set(true);
        status_failed.set(false);
        status_refused.set(None);
        undo_failed.set(false);
        spawn(async move {
            match server_fns::set_project_active(id.to_string(), active, Some(requester)).await {
                Ok(_) => {
                    status_saved.set(Some(active));
                    status_change.set(None);
                    refocus_actions.set(true);
                    // Show the persisted state under current authority.
                    details.restart();
                }
                // Denied authority hides the loaded project; Retry reloads
                // under the bound requester.
                Err(ServerFnError::ServerError {
                    code: 401 | 403, ..
                }) => {
                    status_change.set(None);
                    status_saved.set(None);
                    access_lost.set(true);
                }
                // Restoring under an archived client is refused every time, so
                // say why instead of offering a retry, from the dialog or Undo.
                Err(ServerFnError::ServerError {
                    code: CONFLICT,
                    message,
                    ..
                }) => {
                    status_saved.set(None);
                    status_refused.set(Some((message, client_id)));
                    // Undo's notice disappears with its focused button.
                    focus_refusal.set(undoing);
                }
                // A Dismiss while Undo was pending must not hide its retry.
                Err(_) if undoing => {
                    status_saved.set(Some(false));
                    undo_failed.set(true);
                }
                Err(_) => status_failed.set(true),
            }
            status_busy.set(false);
        });
    };
    use_effect(move || {
        if details.state()() != UseResourceState::Ready {
            return;
        }
        // A reload that loses access must not leave the previous success beside it.
        let loaded = !access_lost() && matches!(&*details.read(), Some(Ok(_)));
        if !loaded {
            status_saved.set(None);
            status_refused.set(None);
            restore_detail_focus("project-detail-back");
        }
        if refocus_actions.take() && loaded {
            restore_detail_focus("project-detail-actions-trigger");
        }
    });
    let ready = details.state()() == UseResourceState::Ready;
    let view = if ready && !access_lost() {
        details
            .read()
            .as_ref()
            .and_then(|result| result.as_ref().ok())
            .cloned()
    } else {
        None
    };
    let can_change_status = view.as_ref().is_some_and(|view| view.can_change_status);
    let loaded = view.is_some();
    let reactivate = status_change() == Some(true);
    // Undo needs the current loaded authority, not the one that archived.
    let undo_label = match (status_saved(), can_change_status, status_busy()) {
        (Some(false), true, true) => "Updating project…",
        (Some(false), true, false) => "Undo",
        _ => "",
    };
    let (status_message, status_variant, status_icon) = match status_saved() {
        // The response may be lost after the write committed, so claim no outcome.
        _ if undo_failed() => ("Could not confirm restore. Retry Undo.", "danger", "!"),
        Some(true) => ("Project reactivated.", "success", "✓"),
        _ => ("Project archived.", "success", "✓"),
    };

    rsx! {
        div {
            Link {
                id: "project-detail-back",
                to: Route::ProjectList {},
                class: "btn btn-ghost text-sm text-muted font-normal border-0 px-2.5 py-1.5 -ml-2.5",
                span { class: "inline-flex", aria_hidden: "true", NavIcon { name: "arrow-left", class: "size-4" } }
                "Back to Projects"
            }
            if !ready {
                p { class: "mt-5", role: "status", "Loading project details…" }
            } else if let Some(view) = view {
                section { class: "mt-5 pb-6 border-b border-light wrap-anywhere", aria_label: "Project details",
                    header { class: "flex flex-wrap items-end gap-4",
                        div { class: "flex-1 min-w-0",
                            if view.can_open_client {
                                Link { to: Route::ClientDetail { id: view.client_id }, class: "text-sm font-semibold",
                                    "{view.project.client_name}"
                                }
                            } else {
                                span { class: "text-sm font-semibold text-muted", "{view.project.client_name}" }
                            }
                            div { class: "flex flex-wrap items-center gap-3 mt-2",
                                h1 { class: "page-title text-4xl font-semibold text-strong tracking-tight wrap-anywhere",
                                    if let Some(code) = view.project.code.as_deref().filter(|code| !code.is_empty()) {
                                        "[{code}] "
                                    }
                                    "{view.project.name}"
                                }
                                span { class: "text-xs text-secondary rounded-full border bg-tertiary px-2 py-1 whitespace-nowrap",
                                    "{view.project_type.label()}"
                                }
                                span { class: "font-mono text-xs text-secondary rounded-full border bg-tertiary px-2 py-1",
                                    "{view.project.currency}"
                                }
                                if !view.active { Badge { variant: "neutral", "Archived" } }
                            }
                        }
                        div { class: "flex flex-wrap items-center gap-3",
                            button { id: "project-detail-refresh", r#type: "button", class: "btn btn-ghost btn-sm",
                                onclick: move |_| details.restart(), "Refresh details"
                            }
                            if view.can_edit {
                                Link { id: "project-detail-edit", class: "btn btn-secondary", to: Route::EditProject { id, return_to: EditReturn::detail(id) },
                                    span { class: "inline-flex", aria_hidden: "true", NavIcon { name: "pencil", class: "size-4" } }
                                    "Edit project"
                                }
                            }
                            if view.can_change_status {
                                Menu { id: "project-detail-actions", label: "Actions", align_right: true,
                                    trigger_class: "text-sm py-3 px-4", disabled: status_busy(),
                                    MenuItem {
                                        disabled: status_busy(),
                                        onclick: {
                                            let next = !view.active;
                                            move |_| {
                                                status_failed.set(false);
                                                status_refused.set(None);
                                                status_saved.set(None);
                                                status_change.set(Some(next));
                                            }
                                        },
                                        if view.active { "Archive" } else { "Reactivate" }
                                    }
                                }
                            }
                        }
                    }
                    SavedProjectDetails { project: view.project }
                }
                section { class: "card mt-6 p-5 wrap-anywhere", aria_label: "Project tasks",
                    h2 { class: "page-title text-xl", "Project tasks" }
                    if view.tasks.is_empty() { p { class: "text-muted text-sm", "No tasks enabled yet." } }
                    else { ul { for task in view.tasks { li { key: "{task.id}", "{task.name}" } } } }
                }
                section { class: "card mt-6 p-5 wrap-anywhere", aria_label: "Project team",
                    h2 { class: "page-title text-xl", "Team" }
                    if view.team.is_empty() { p { class: "text-muted text-sm", "No visible team members." } }
                    else { ul { for person in view.team { li { key: "{person.id}", "{person.name}" } } } }
                }
            } else {
                p { class: "text-danger mt-5", role: "alert",
                    "Project details are unavailable. Your session or access may have changed."
                }
                button { id: "project-detail-retry", r#type: "button", class: "btn btn-secondary btn-sm",
                    onclick: move |_| details.restart(), "Retry details"
                }
            }

            Modal {
                id: "project-status-dialog", labelledby: "project-status-title",
                focus_fallback: "project-detail-back",
                open: status_change().is_some() && can_change_status, busy: status_busy(),
                // A dialog refusal ends with the dialog; only Undo shows one outside it.
                on_dismiss: move |_| {
                    status_refused.set(None);
                    status_change.set(None);
                },
                if can_change_status {
                    h2 { id: "project-status-title", class: "modal-title m-0",
                        if reactivate { "Reactivate this project?" } else { "Archive this project?" }
                    }
                    div { class: "modal-body px-3 sm:px-6 wrap-anywhere",
                        p { class: "text-sm text-secondary",
                            if reactivate {
                                "Only project status changes. Existing time entries, invoices and budgets are kept."
                            } else {
                                "Time already tracked stays in reports and invoices. Nobody can log new time until it is reactivated."
                            }
                        }
                        if status_failed() {
                            p { class: "alert alert-danger", role: "alert",
                                "Could not update project status. You can retry safely, or cancel and refresh."
                            }
                        }
                        if let Some((message, client)) = status_refused() {
                            div { class: "alert alert-danger", role: "alert",
                                p { class: "m-0", "{message}" }
                                if let Some(client) = client {
                                    Link { to: Route::ClientDetail { id: client }, class: "font-semibold", "Open the client" }
                                }
                            }
                        }
                        div { class: "modal-actions flex-wrap",
                            button {
                                id: "project-status-confirm", r#type: "button", class: "btn btn-primary",
                                disabled: status_busy(),
                                onclick: move |_| {
                                    if let Some(active) = status_change() {
                                        write_status(active);
                                    }
                                },
                                if status_busy() { "Updating project…" }
                                else if reactivate { "Reactivate project" }
                                else { "Archive project" }
                            }
                            button { r#type: "button", class: "btn btn-secondary", disabled: status_busy(),
                                onclick: move |_| {
                                    status_refused.set(None);
                                    status_change.set(None);
                                },
                                "Cancel"
                            }
                        }
                    }
                }
            }
            // A refusal outside the dialog, such as from Undo.
            if status_change().is_none() && loaded {
                if let Some((message, client)) = status_refused() {
                    div { id: "project-detail-status-refusal", tabindex: "-1",
                        class: "alert alert-danger mt-5", role: "alert",
                        p { class: "m-0", "{message}" }
                        if let Some(client) = client {
                            Link { to: Route::ClientDetail { id: client }, class: "font-semibold", "Open the client" }
                        }
                    }
                }
            }
            ToastContainer {
                if status_saved().is_some() {
                    Toast { message: "{status_message}", variant: "{status_variant}", icon: "{status_icon}", dismissible: true,
                        ondismiss: move |_| status_saved.set(None),
                        action_label: "{undo_label}", action_busy: status_busy(),
                        onaction: move |_| write_status(true),
                    }
                }
            }
        }
    }
}

/// A reload unmounts its opener. Recover focus on the rebuilt action or Back
/// after an error, without overriding deliberate navigation during the reload.
fn restore_detail_focus(target: &'static str) {
    let target = serde_json::json!(target);
    document::eval(&format!(
        "requestAnimationFrame(() => {{
            const active = document.activeElement;
            if (!active || active === document.body || active.id === 'project-detail-back') {{
                document.getElementById({target})?.focus();
            }}
        }});",
    ));
}

#[cfg(test)]
mod list_tests {
    use std::{any::Any, cell::RefCell};

    use super::*;
    use crate::models::permission_editor::PermissionRequester;
    use crate::models::project::{ProjectOverviewClient, ProjectOverviewRow};

    fn row(client: (u128, &str), name: &str, code: Option<&str>) -> ProjectOverviewRow {
        ProjectOverviewRow {
            project: Project {
                id: Uuid::now_v7(),
                org_id: Uuid::nil(),
                client_id: Uuid::from_u128(client.0),
                code: code.map(Into::into),
                name: name.into(),
                project_type: ProjectType::TimeAndMaterials,
                currency: "EUR".into(),
                rate_cents: None,
                starts_on: None,
                ends_on: None,
                budget_kind: BudgetKind::None,
                budget_amount_cents: None,
                budget_minutes: None,
                active: true,
                created_at: chrono::Utc::now(),
            },
            client: Some(ProjectOverviewClient {
                id: Uuid::from_u128(client.0),
                name: client.1.into(),
                active: true,
            }),
            can_edit: false,
        }
    }

    fn overview(projects: Vec<ProjectOverviewRow>) -> ProjectOverview {
        ProjectOverview {
            requester: PermissionRequester {
                org_id: Uuid::nil(),
                user_id: Uuid::nil(),
            },
            canonical_permissions: true,
            can_create: false,
            can_import: false,
            can_change_legacy_status: false,
            projects,
        }
    }

    #[test]
    fn catalog_groups_by_client_and_orders_names_case_insensitively() {
        let catalog = ProjectCatalog::new(&overview(vec![
            row((2, "beta"), "zeta", None),
            row((1, "Alpha"), "Mobile", Some("M1")),
            row((3, "Beta"), "Apps", None),
            row((1, "Alpha"), "api", None),
            row((2, "beta"), "Web", None),
        ]));
        let order: Vec<_> = catalog.rows.iter().map(|row| row.label.as_str()).collect();
        // Same-named clients stay apart, ordered by ID.
        assert_eq!(order, ["api", "[M1] Mobile", "Web", "zeta", "Apps"]);
    }

    #[test]
    fn search_matches_project_or_client_names_ignoring_case() {
        let catalog = ProjectCatalog::new(&overview(vec![
            row((1, "Acme Corp"), "Website", None),
            row((2, "Globex"), "Acme migration", None),
            row((3, "Initech"), "Payroll", None),
        ]));
        let found = |query: &str| -> Vec<&str> {
            catalog
                .rows
                .iter()
                .filter(|row| row.matches(query, "active", ""))
                .map(|row| row.project.name.as_str())
                .collect()
        };
        assert_eq!(found("acme"), ["Website", "Acme migration"]);
        assert_eq!(found("pay"), ["Payroll"]);
        assert_eq!(found(""), ["Website", "Acme migration", "Payroll"]);
    }

    /// Per render: the catalog's address and the visible project names.
    type RenderLog = Rc<RefCell<Vec<(usize, Vec<String>)>>>;

    #[test]
    fn typing_a_search_renders_the_list_once_per_keystroke_without_rebuilding_it() {
        use dioxus::dioxus_core::Mutation;
        use dioxus::html::{SerializedFormData, SerializedHtmlEventConverter};

        dioxus::html::set_event_converter(Box::new(SerializedHtmlEventConverter));
        // The list wiring of ProjectListContent around the production hook.
        fn list(log: RenderLog) -> Element {
            let mut query = use_signal(String::new);
            let scope = use_signal(|| "active".to_string());
            let client = use_signal(String::new);
            let tagged = use_memo(|| None::<BTreeSet<Uuid>>);
            let admitted = use_signal(|| {
                Some(overview(vec![
                    row((1, "Acme"), "Website", None),
                    row((1, "Acme"), "Web shop", None),
                    row((2, "Globex"), "Payroll", None),
                ]))
            });
            let (catalog, visible) = use_project_list(
                admitted,
                ProjectFilters {
                    query,
                    scope,
                    client,
                    tagged,
                },
            );
            let names = visible
                .read()
                .iter()
                .map(|&index| catalog.rows[index].project.name.clone())
                .collect();
            log.borrow_mut()
                .push((Rc::as_ptr(&catalog) as usize, names));
            rsx! {
                input { value: "{query}", oninput: move |event| query.set(event.value()) }
            }
        }
        let log = RenderLog::default();
        let mut dom = VirtualDom::new_with_props(list, log.clone());
        let input = dom
            .rebuild_to_vec()
            .edits
            .iter()
            .find_map(|edit| match edit {
                Mutation::NewEventListener { name, id } if name == "input" => Some(*id),
                _ => None,
            })
            .unwrap();
        for value in ["w", "we", "web", "webs"] {
            dom.runtime().handle_event(
                "input",
                Event::new(
                    Rc::new(PlatformEventData::new(Box::new(SerializedFormData::new(
                        value.into(),
                        vec![],
                    )))) as Rc<dyn Any>,
                    true,
                ),
                input,
            );
            dom.render_immediate_to_vec();
        }
        let log = log.borrow();
        assert_eq!(log.len(), 5, "one render to mount, then one per keystroke");
        assert!(
            log.iter().all(|(catalog, _)| *catalog == log[0].0),
            "typing must not rebuild the catalog"
        );
        let visible: Vec<_> = log.iter().map(|(_, names)| names.join(", ")).collect();
        // Each keystroke's own render already shows its filtered rows.
        assert_eq!(
            visible,
            [
                "Web shop, Website, Payroll",
                "Web shop, Website",
                "Web shop, Website",
                "Web shop, Website",
                "Website",
            ]
        );
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
fn SavedProjectDetails(project: ProjectDetails) -> Element {
    rsx! {
        details { class: "mt-4 text-sm",
            summary { class: "cursor-pointer text-muted", "Project information" }
            dl { class: "grid sm:grid-cols-2 gap-4 m-0 mt-4",
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
