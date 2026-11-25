//! A new invoice in two steps: the client, then what to bill.

use std::cell::Cell;
use std::rc::Rc;

use chrono::{Datelike, NaiveDate};
use dioxus::dioxus_core::Task;
use dioxus::prelude::*;
use uuid::Uuid;

use crate::components::back_link::BackLink;
use crate::components::button::Button;
use crate::components::confirm_dialog::{ConfirmDialog, ConfirmTone};
use crate::components::controls::Checkbox;
use crate::components::date_field::DateField;
use crate::components::form::FormGroup;
use crate::components::menu::{Menu, MenuItem};
use crate::components::page_header::PageHeader;
use crate::components::screen_state::{
    EmptyState, ErrorState, LOAD_FAILED, LoadError, NoAccess, Skeleton,
};
use crate::components::select_field::SelectField;
use crate::components::table::DataTable;
use crate::i18n::{Tr, split_tag, use_tr};
use crate::models::Project;
use crate::models::invoice::{
    InvoiceDefaults, InvoiceDuplicate, InvoiceFeeSelection, InvoiceGenerationRequest,
    InvoicePreparation, InvoiceSource, LastInvoice,
};
use crate::models::project_creation::InvoiceDefaultsInput;
use crate::route::Route;
use crate::server_fns;

use super::defaults_form::{DefaultsFields, fields_from, parse_fields};
use super::recovery::{PendingInvoice, RecoveryStorage, release_navigation};
use super::{INVOICES_HEADING, breakdown_label, date, money, prepares_invoices};

#[path = "preparation/fees.rs"]
mod fees;

/// How long edits must pause before the preview is asked for again.
const PREVIEW_DEBOUNCE: std::time::Duration = std::time::Duration::from_millis(300);

#[derive(Clone, PartialEq, Eq, Debug)]
struct InvoiceRequest {
    client: String,
    from: String,
    to: String,
    projects: Option<Vec<String>>,
    overrides: Option<InvoiceDefaults>,
    fees: Option<Vec<InvoiceFeeSelection>>,
}

impl InvoiceRequest {
    fn same_sources(&self, other: &Self) -> bool {
        self.client == other.client
            && self.from == other.from
            && self.to == other.to
            && self.projects == other.projects
    }
}

// A preview is usable only for the exact values that were reviewed. Invalid
// edits have no request and can never fall back to a previous valid preview.
fn is_reviewed(current: Option<&InvoiceRequest>, reviewed: Option<&InvoiceRequest>) -> bool {
    current
        .zip(reviewed)
        .is_some_and(|(current, reviewed)| current == reviewed)
}

/// The period presets. Since the last invoice is the default when the client
/// has one, and This month otherwise. Up to today has no start, so it finds
/// everything still unbilled, as the Uninvoiced view does; it is never the
/// default, so old or imported time is billed only when someone asks for it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Preset {
    SinceLastInvoice,
    ThisMonth,
    LastMonth,
    UpToToday,
    Custom,
}

impl Preset {
    const ALL: [Self; 5] = [
        Self::SinceLastInvoice,
        Self::ThisMonth,
        Self::LastMonth,
        Self::UpToToday,
        Self::Custom,
    ];

    fn label(self, tr: Tr) -> String {
        match self {
            Self::SinceLastInvoice => tr.invoices_period_since_last(),
            Self::UpToToday => tr.invoices_period_up_to_today(),
            Self::ThisMonth => tr.invoices_period_this_month(),
            Self::LastMonth => tr.invoices_period_last_month(),
            Self::Custom => tr.invoices_period_custom(),
        }
    }

    /// The first preset for a client: since its last invoice if it has one.
    fn default_for(last: Option<&LastInvoice>) -> Self {
        if last.is_some() {
            Self::SinceLastInvoice
        } else {
            Self::ThisMonth
        }
    }

    /// The dates the preset stands for, or `None` for Custom and for Since
    /// the last invoice when there is none. A last invoice that covers today
    /// starts the period today rather than after it.
    fn range(self, today: NaiveDate, last: Option<&LastInvoice>) -> Option<(NaiveDate, NaiveDate)> {
        let first = today.with_day(1)?;
        match self {
            Self::SinceLastInvoice => Some((last?.next_start()?.min(today), today)),
            Self::UpToToday => Some((NaiveDate::from_ymd_opt(1970, 1, 1)?, today)),
            Self::ThisMonth => Some((first, today)),
            Self::LastMonth => {
                let last = first.pred_opt()?;
                Some((last.with_day(1)?, last))
            }
            Self::Custom => None,
        }
    }
}

/// The screen's gate: no access, a failed permissions read, or the content.
#[component]
fn PrepareGate(children: Element) -> Element {
    let tr = use_tr();
    let session = crate::components::session::use_session();
    let retry_session = use_callback(move |()| session.refresh());
    if session.permissions.read().is_err() {
        // A failed read says nothing about access, so it offers a retry.
        return rsx! {
            ErrorState { title: tr.invoices_permissions_failed(), body: LOAD_FAILED, onretry: retry_session,
                retry_disabled: (session.refreshing)(), focus_on_recovery: INVOICES_HEADING }
        };
    }
    if !prepares_invoices(session) {
        return rsx! { NoAccess {} };
    }
    rsx! { {children} }
}

/// Step 1: who the invoice is for.
#[component]
pub(super) fn ChooseClient() -> Element {
    let tr = use_tr();
    let storage = use_context::<RecoveryStorage>();
    let navigator = use_navigator();
    let mut clients = use_resource(|| async move { server_fns::list_clients(false).await });
    let retry = use_callback(move |()| clients.restart());
    let mut client = use_signal(String::new);
    let query = use_signal(String::new);
    let pick = use_callback(move |value: String| client.set(value));
    let next = use_callback(move |_: MouseEvent| {
        let chosen = client.peek().clone();
        if !chosen.is_empty() {
            navigator.push(Route::NewInvoiceForClient { client: chosen });
        }
    });
    rsx! {
        div {
            BackLink { to: Route::InvoiceList { filter: Default::default() }, label: tr.invoices_back() }
            PageHeader { title: tr.invoices_new_title(), title_id: INVOICES_HEADING }
            PrepareGate {
                match &*clients.read() {
                    None => rsx! { Skeleton { rows: 2, columns: 1, label: tr.invoices_clients_loading() } },
                    Some(Err(error)) => rsx! {
                        LoadError { error: error.clone(), what: tr.invoices_clients_what(), retry,
                            pending: clients.state()() != UseResourceState::Ready, focus_on_recovery: INVOICES_HEADING }
                    },
                    Some(Ok(list)) => rsx! {
                        section { class: "card p-6 max-w-form", aria_labelledby: "invoice-step-1",
                            p { class: "text-sm text-muted mb-2", {tr.invoices_step(1, 2)} }
                            h2 { id: "invoice-step-1", class: "card-title mb-4", {tr.invoices_step_client_title()} }
                            FormGroup { label: tr.invoices_column_client(), id: "inv-client",
                                SelectField { id: "inv-client", label: tr.invoices_column_client(),
                                    options: list.iter().map(|c| (c.id.to_string(), c.name.clone())).collect::<Vec<_>>(),
                                    selected: client(), query: Some(query), onselect: pick }
                            }
                            p { class: "text-sm text-muted mb-5", {tr.invoices_step_client_hint()} }
                            Button { id: "invoice-continue", label: tr.invoices_continue(),
                                disabled: client().is_empty() || !storage.ready(), onclick: next }
                        }
                    },
                }
            }
        }
    }
}

/// Step 2 for one client: what was loaded before the form can open.
#[derive(Clone, PartialEq)]
struct Sources {
    client: Uuid,
    client_name: String,
    projects: Vec<Project>,
    template: Option<InvoiceDuplicate>,
    /// Where the default period starts.
    last_invoice: Option<LastInvoice>,
}

#[component]
pub(super) fn PrepareForClient(client: String, duplicate: Option<Uuid>) -> Element {
    let tr = use_tr();
    // Everything step 2 needs, asked for at once.
    let mut sources = use_resource(move || {
        let client = client.clone();
        async move {
            let Ok(id) = client.parse::<Uuid>() else {
                return Ok(None);
            };
            let (clients, projects, last_invoice, template) = futures_util::join!(
                server_fns::list_clients(false),
                server_fns::list_projects(Some(id.to_string()), true),
                server_fns::get_last_client_invoice(id.to_string()),
                async {
                    match duplicate {
                        Some(source) => server_fns::get_invoice_duplicate(source.to_string())
                            .await
                            .map(Some),
                        None => Ok(None),
                    }
                },
            );
            let Some(found) = clients?.into_iter().find(|c| c.id == id) else {
                return Ok(None);
            };
            Ok::<_, ServerFnError>(Some(Sources {
                client: id,
                client_name: found.name,
                projects: projects?,
                template: template?.filter(|t: &InvoiceDuplicate| t.client_id == id),
                last_invoice: last_invoice?,
            }))
        }
    });
    let retry = use_callback(move |()| sources.restart());
    rsx! {
        div {
            BackLink { to: Route::InvoiceList { filter: Default::default() }, label: tr.invoices_back() }
            PageHeader { title: tr.invoices_new_title(), title_id: INVOICES_HEADING }
            PrepareGate {
                match &*sources.read() {
                    None => rsx! { Skeleton { rows: 6, columns: 4, label: tr.invoices_preparing() } },
                    Some(Err(error)) => rsx! {
                        LoadError { error: error.clone(), what: tr.invoices_clients_what(), retry,
                            pending: sources.state()() != UseResourceState::Ready, focus_on_recovery: INVOICES_HEADING }
                    },
                    Some(Ok(None)) => rsx! {
                        ErrorState { title: tr.invoices_client_unavailable(), body: tr.invoices_client_unavailable_body(),
                            actions: rsx! { Link { to: Route::NewInvoice {}, class: "btn btn-secondary", {tr.invoices_choose_client()} } } }
                    },
                    Some(Ok(Some(loaded))) => rsx! { Prepare { sources: loaded.clone() } },
                }
            }
        }
    }
}

#[component]
fn Prepare(sources: Sources) -> Element {
    let tr = use_tr();
    let storage = use_context::<RecoveryStorage>();
    let navigator = use_navigator();
    let today = crate::components::today::today();
    let client = sources.client.to_string();
    let duplicate_of = sources.template.as_ref().map(|t| t.source_id);
    let all_projects: Vec<String> = sources.projects.iter().map(|p| p.id.to_string()).collect();

    let last_invoice = sources.last_invoice.clone();
    let first_preset = Preset::default_for(last_invoice.as_ref());
    let mut preset = use_signal(|| first_preset);
    let initial = first_preset
        .range(today, last_invoice.as_ref())
        .unwrap_or((today, today));
    let mut from = use_signal(|| initial.0.to_string());
    let mut to = use_signal(|| initial.1.to_string());
    let template = sources.template.clone();
    let mut selected = use_signal(|| {
        template.as_ref().map(|t| {
            t.project_ids
                .iter()
                .map(Uuid::to_string)
                .collect::<Vec<_>>()
        })
    });
    let mut fields = use_signal(|| {
        template
            .as_ref()
            .map(|t| fields_from(&t.defaults))
            .unwrap_or_default()
    });
    let mut started = use_signal(|| template.is_some());
    let mut fees = use_signal(Vec::<fees::FeeInput>::new);
    let mut loaded = use_signal(|| None::<InvoiceRequest>);
    let mut preview = use_signal(|| None::<(InvoiceRequest, InvoicePreparation)>);
    let mut preview_error = use_signal(|| None::<String>);
    let mut attempt = use_signal(|| 0_u32);
    let mut previewing = use_signal(|| false);
    let busy = use_signal(|| false);
    let mut uncertain = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let mut generation = use_signal(|| None::<(InvoiceRequest, InvoiceGenerationRequest)>);
    let mut confirm = use_signal(|| false);
    // Leaving asks only once the person changed something: the period, the
    // projects, or a value the preview filled in while nothing had changed.
    // Peeks: every render already reads these signals.
    let chosen = move || {
        let period = (from.peek().clone(), to.peek().clone());
        (*preset.peek(), period, selected.peek().clone())
    };
    let values = move || (fields.peek().clone(), fees.peek().clone());
    let opened = use_signal(chosen);
    let mut filled = use_signal(values);
    let untouched = move || *opened.peek() == chosen() && *filled.peek() == values();

    // All ticked is the same as no selection, which lets the server resolve
    // values from the projects that bill something.
    let normalized = {
        let all = all_projects.clone();
        move |ids: Option<Vec<String>>| {
            ids.filter(|ids| !(ids.len() == all.len() && all.iter().all(|id| ids.contains(id))))
        }
    };
    let source_request = InvoiceRequest {
        client: client.clone(),
        from: from(),
        to: to(),
        projects: normalized(selected()),
        overrides: None,
        fees: None,
    };
    let same_sources = loaded
        .read()
        .as_ref()
        .is_some_and(|last| last.same_sources(&source_request));
    let current = if started() {
        parse_fields(tr, &fields.read()).map(Some)
    } else {
        Ok(None)
    }
    .and_then(|overrides| {
        let selected_fees = if same_sources {
            Some(fees::parse(tr, &fees.read())?)
        } else {
            None
        };
        Ok(InvoiceRequest {
            overrides,
            fees: selected_fees,
            ..source_request.clone()
        })
    })
    .and_then(|request| {
        if request.from.is_empty() || request.to.is_empty() || request.from > request.to {
            Err(tr.invoices_period_invalid())
        } else if request.projects.as_ref().is_some_and(Vec::is_empty) {
            Err(tr.invoices_projects_none())
        } else {
            Ok(request)
        }
    });

    // The preview follows the form: each valid change asks for it again once
    // edits pause. A request already reviewed is not asked for twice.
    let waiting = use_hook(|| Rc::new(Cell::new(None::<Task>)));
    let wanted = current.clone();
    use_effect(use_reactive!(|wanted| {
        // A retry, or a request the server refused, asks again for the same values.
        let _ = attempt();
        if let Some(task) = waiting.take() {
            task.cancel();
            previewing.set(false);
        }
        let Ok(request) = wanted else {
            return;
        };
        if preview
            .peek()
            .as_ref()
            .is_some_and(|(reviewed, _)| *reviewed == request)
            || *busy.peek()
        {
            return;
        }
        waiting.set(Some(spawn(async move {
            crate::components::debounce::sleep(PREVIEW_DEBOUNCE).await;
            previewing.set(true);
            let result = server_fns::prepare_invoice(
                request.client.clone(),
                request.from.clone(),
                request.to.clone(),
                request.projects.clone(),
                request.overrides.clone(),
                request.fees.clone(),
            )
            .await;
            previewing.set(false);
            match result {
                Ok(estimate) => {
                    preview_error.set(None);
                    let was_untouched = untouched();
                    if request.fees.is_none() {
                        fees.set(fees::from_review(&estimate));
                    }
                    loaded.set(Some(request.clone()));
                    let reviewed = match &estimate.defaults {
                        Some(defaults) => {
                            fields.set(fields_from(defaults));
                            InvoiceRequest {
                                overrides: Some(defaults.clone()),
                                fees: Some(estimate.fee_selection()),
                                ..request
                            }
                        }
                        None => {
                            if request.overrides.is_none() {
                                fields.set(InvoiceDefaultsInput {
                                    terms_days: String::new(),
                                    ..Default::default()
                                });
                            }
                            request
                        }
                    };
                    if was_untouched {
                        filled.set(values());
                    }
                    started.set(true);
                    preview.set(Some((reviewed, estimate)));
                }
                Err(failure) => preview_error.set(Some(failure.to_string())),
            }
        })));
    }));

    let reviewed = preview.read().as_ref().map(|(request, _)| request.clone());
    let ready = is_reviewed(current.as_ref().ok(), reviewed.as_ref());
    let estimate = preview
        .read()
        .as_ref()
        .map(|(_, estimate)| estimate.clone());
    let has_selection = ready
        && estimate
            .as_ref()
            .is_some_and(|review| review.lines.iter().any(|line| line.selected));
    let has_excess = ready
        && estimate
            .as_ref()
            .is_some_and(|review| !review.excess.is_empty());
    let stale = estimate.is_some() && !ready;

    let generate_request = current.clone();
    let generate = use_callback(move |confirm_excess: bool| {
        let mut busy = busy;
        if (*busy.peek() && !*uncertain.peek()) || !storage.ready() {
            return;
        }
        let Ok(request) = generate_request.clone() else {
            return;
        };
        let mutation = if *uncertain.peek() {
            generation
                .peek()
                .as_ref()
                .filter(|(previous, _)| previous == &request)
                .map(|(_, mutation)| mutation.clone())
        } else {
            preview
                .peek()
                .as_ref()
                .filter(|(reviewed, _)| reviewed == &request)
                .map(|(_, review)| InvoiceGenerationRequest {
                    request_id: Uuid::now_v7(),
                    review: review.clone(),
                    confirmed_excess: if confirm_excess {
                        review.excess.clone()
                    } else {
                        Vec::new()
                    },
                    duplicate_of,
                })
        };
        let Some(mutation) = mutation else {
            return;
        };
        generation.set(Some((request.clone(), mutation.clone())));
        uncertain.set(false);
        confirm.set(false);
        busy.set(true);
        error.set(None);
        spawn(async move {
            let pending = PendingInvoice::Generate {
                client: request.client,
                from: request.from,
                to: request.to,
                projects: request.projects,
                overrides: request.overrides,
                request: Box::new(mutation),
            };
            if let Err(message) = storage.store(&pending).await {
                busy.set(false);
                error.set(Some(tr.invoices_request_not_sent(message)));
                return;
            }
            match pending.submit().await {
                Ok(id) => match storage.clear().await {
                    Ok(()) => {
                        release_navigation().await;
                        busy.set(false);
                        navigator.push(Route::EditInvoice { id });
                    }
                    Err(message) => {
                        error.set(Some(tr.invoices_recovery_not_cleared(message)));
                        uncertain.set(true);
                    }
                },
                Err(failure) => {
                    if matches!(&failure, ServerFnError::ServerError { code, .. } if (400..500).contains(code))
                    {
                        error.set(Some(failure.to_string()));
                        preview.set(None);
                        if let Err(message) = storage.clear().await {
                            error.set(Some(format!("{failure}. {message}")));
                        }
                        busy.set(false);
                        // What the server refused is out of date: preview again.
                        attempt += 1;
                    } else {
                        error.set(Some(tr.invoices_generation_uncertain()));
                        uncertain.set(true);
                    }
                }
            }
        });
    });
    let create = use_callback(move |_: MouseEvent| {
        if *uncertain.peek() {
            generate.call(false);
        } else if has_excess {
            confirm.set(true);
        } else {
            generate.call(false);
        }
    });
    let generate_excess = use_callback(move |()| generate.call(true));
    let keep_preparing = use_callback(move |()| confirm.set(false));
    let preset_last = last_invoice.clone();
    let choose_preset = use_callback(move |value: Preset| {
        let was = *preset.peek();
        preset.set(value);
        // Custom dates start from this month rather than the open start.
        let range = match value {
            Preset::Custom if was == Preset::UpToToday => {
                Preset::ThisMonth.range(today, preset_last.as_ref())
            }
            other => other.range(today, preset_last.as_ref()),
        };
        if let Some((start, end)) = range {
            from.set(start.to_string());
            to.set(end.to_string());
        }
    });
    let retry_preview = use_callback(move |_: MouseEvent| {
        preview_error.set(None);
        attempt += 1;
    });
    let set_from = use_callback(move |value: String| from.set(value));
    let set_to = use_callback(move |value: String| to.set(value));
    let toggle_all = use_callback(move |_: MouseEvent| {
        let none_selected = selected.peek().as_ref().is_some_and(Vec::is_empty);
        selected.set(if none_selected {
            None
        } else {
            Some(Vec::new())
        });
    });
    let toggle_project = {
        let all = all_projects.clone();
        use_callback(move |id: String| {
            let mut ids = selected.peek().clone().unwrap_or_else(|| all.clone());
            if ids.contains(&id) {
                ids.retain(|value| value != &id);
            } else {
                ids.push(id);
                ids.sort();
            }
            // Ticking every project again is the selection the screen opened with.
            selected.set((ids.len() < all.len()).then_some(ids));
        })
    };
    let refresh_fees = use_callback(move |_: MouseEvent| {
        // New or settled fee sources: keep the edits, drop what is gone.
        let request = InvoiceRequest {
            overrides: None,
            fees: None,
            ..source_request.clone()
        };
        spawn(async move {
            previewing.set(true);
            let result = server_fns::prepare_invoice(
                request.client,
                request.from,
                request.to,
                request.projects,
                None,
                None,
            )
            .await;
            previewing.set(false);
            match result {
                Ok(estimate) => {
                    let refreshed = fees::refresh(&fees.peek(), fees::from_review(&estimate));
                    fees.set(refreshed);
                    preview_error.set(None);
                }
                Err(failure) => preview_error.set(Some(failure.to_string())),
            }
        });
    });

    let ticked = |id: &str| {
        selected
            .read()
            .as_ref()
            .is_none_or(|ids| ids.iter().any(|value| value == id))
    };
    let none_ticked = selected.read().as_ref().is_some_and(Vec::is_empty);
    let some_ticked = selected
        .read()
        .as_ref()
        .is_some_and(|ids| !ids.is_empty() && ids.len() < all_projects.len());
    let period_label = match (
        NaiveDate::parse_from_str(&from(), "%Y-%m-%d"),
        NaiveDate::parse_from_str(&to(), "%Y-%m-%d"),
    ) {
        _ if preset() == Preset::UpToToday => tr.invoices_period_up_to(date(today)),
        (Ok(start), Ok(_)) if preset() == Preset::SinceLastInvoice => {
            let number = last_invoice
                .as_ref()
                .map(|last| last.number.clone())
                .unwrap_or_default();
            tr.invoices_period_since(date(start), number)
        }
        (Ok(start), Ok(end)) => tr.invoices_period_range(date(start), date(end)),
        _ => preset().label(tr),
    };
    let currency = currency(estimate.as_ref());
    let awaiting = estimate
        .as_ref()
        .map_or(0, |review| review.awaiting_approval_minutes);
    let warning = tr.invoices_awaiting_approval(horae_core::duration::format_hhmm(awaiting));
    let (warning_before, warning_link, warning_after) = split_tag(&warning, "link");

    rsx! {
        div { "data-editor-kind": "invoice", "data-editor-state": if busy() { "pending" } else if untouched() { "clean" } else { "dirty" },
        if let Some(template) = &sources.template {
            p { class: "text-secondary mb-5", role: "status", {tr.invoices_duplicating(template.source_number.clone())} }
        }
        section { class: "card p-5 mb-6 flex flex-wrap items-end gap-6",
            div {
                div { class: "text-sm text-muted", {tr.invoices_column_client()} }
                div { class: "font-semibold wrap-anywhere", "{sources.client_name}" }
                Link { to: Route::NewInvoice {}, class: "text-sm", {tr.invoices_change_client()} }
            }
            div {
                div { id: "inv-period-label", class: "text-sm text-muted", {tr.invoices_period()} }
                Menu { id: "inv-period", label: period_label, trigger_class: "text-sm px-4",
                    for value in Preset::ALL.into_iter().filter(|value| *value != Preset::SinceLastInvoice || last_invoice.is_some()) {
                        PresetOption { key: "{value:?}", value, current: preset(), onpick: choose_preset }
                    }
                }
            }
            if preset() == Preset::Custom {
                FormGroup { label: tr.invoices_period_from(), id: "inv-from",
                    DateField { id: "inv-from", label: tr.invoices_period_from(),
                        placeholder: tr.invoices_pick_date(), value: from(), onchange: set_from }
                }
                FormGroup { label: tr.invoices_period_to(), id: "inv-to",
                    DateField { id: "inv-to", label: tr.invoices_period_to(),
                        placeholder: tr.invoices_pick_date(), value: to(), onchange: set_to }
                }
            }
            p { class: "text-sm text-muted ml-auto", {tr.invoices_step(2, 2)} }
        }

        if let Err(message) = &current {
            if started() || preset() == Preset::Custom {
                div { class: "alert alert-danger mb-5", role: "alert", "{message}" }
            }
        }
        if let Some(message) = error() {
            div { class: "alert alert-danger mb-5", role: "alert", "{message}" }
        }
        if let Some(message) = preview_error() {
            div { class: "alert alert-danger mb-5", role: "alert",
                p { "{message}" }
                button { r#type: "button", class: "btn btn-secondary btn-sm mt-3", disabled: busy() || previewing(),
                    onclick: retry_preview, {tr.invoices_preview_retry()} }
            }
        }

        section { class: "mb-6", aria_labelledby: "inv-projects-title",
            h2 { id: "inv-projects-title", class: "card-title mb-3", {tr.invoices_projects()} }
            DataTable {
                table {
                    thead { tr {
                        th { scope: "col",
                            Checkbox { label: tr.invoices_all_projects(), compact: true, checked: !none_ticked,
                                mixed: some_ticked, disabled: busy(), onclick: toggle_all }
                        }
                        th { scope: "col", {tr.invoices_column_project()} }
                        th { scope: "col", class: "text-right", {tr.invoices_column_unbilled_time()} }
                        th { scope: "col", class: "text-right", {tr.invoices_column_fees_due()} }
                    } }
                    tbody {
                        for project in sources.projects.iter() {
                            ProjectRow { key: "{project.id}", project: project.clone(), ticked: ticked(&project.id.to_string()),
                                estimate: estimate.clone(), disabled: busy(), ontoggle: toggle_project }
                        }
                    }
                    tfoot { tr {
                        th { scope: "row", colspan: "2", class: "text-right", {tr.invoices_total()} }
                        {unbilled_cells(unbilled(estimate.as_ref(), |project| ticked(&project.to_string())), &currency)}
                    } }
                }
            }
        }

        if awaiting > 0 && ready {
            div { id: "inv-awaiting-approval", class: "alert alert-warning mb-6", role: "status",
                "{warning_before}"
                Link { to: Route::Approvals { week: String::new() }, "{warning_link}" }
                "{warning_after}"
            }
        }

        if let Some(review) = estimate.as_ref().filter(|review| review.defaults.is_none()) {
            div { class: "alert alert-warning", role: "status", {tr.invoices_defaults_conflict()} }
            div { class: "flex flex-wrap gap-3 mb-4",
                for project in &review.projects {
                    UseDefaults { key: "{project.project_id}", name: project.name.clone(), defaults: project.defaults.clone(), fields, disabled: busy() }
                }
            }
        }
        if started() {
            section { class: "card p-5 mb-6 max-w-form", aria_labelledby: "inv-values-title",
                h2 { id: "inv-values-title", class: "card-title mb-4", {tr.invoices_values()} }
                DefaultsFields { fields, disabled: busy() }
            }
        }
        if same_sources && !fees.read().is_empty() {
            section { class: "card p-5 mb-6 max-w-form",
                fees::FeeFields { fees, disabled: busy() }
                // Fees changed elsewhere (a new schedule, another draft) since
                // this list was loaded: reload them, keeping the edits.
                button { r#type: "button", class: "btn btn-secondary btn-sm", disabled: busy() || previewing(),
                    onclick: refresh_fees, {tr.invoices_refresh_fees()} }
            }
        }

        section { aria_labelledby: "inv-preview-title", class: "mb-6",
            h2 { id: "inv-preview-title", class: "card-title mb-3", {tr.invoices_preview()} }
            match &estimate {
                None if previewing() || current.is_ok() => rsx! { Skeleton { rows: 3, columns: 4, label: tr.invoices_preview_loading() } },
                None => rsx! {},
                // Nothing billable, for example because all of it waits for
                // approval: the warning above says what is waiting.
                Some(review) if review.lines.is_empty() => rsx! {
                    div { class: "inv-content", aria_busy: stale || previewing(),
                        EmptyState { id: "inv-preview-empty", icon: "invoices", title: tr.invoices_preview_empty(),
                            body: tr.invoices_preview_empty_body() }
                    }
                },
                Some(review) => rsx! {
                    div { class: "inv-content", aria_busy: stale || previewing(),
                        PreparedLines { estimate: review.clone() }
                    }
                },
            }
        }

        div { class: "flex flex-wrap items-center gap-3",
            Button { id: "invoice-create", label: if uncertain() { tr.invoices_retry_create() } else { tr.invoices_create_draft() },
                disabled: (busy() && !uncertain()) || !has_selection || !storage.ready(), onclick: create }
            if !ready && current.is_ok() && !busy() {
                p { class: "text-sm text-muted", role: "status", {tr.invoices_preview_updating()} }
            }
        }

        // A generated draft can still be corrected, so this is not a
        // destructive confirmation.
        ConfirmDialog { id: "generation-excess", labelledby: "generation-excess-title", open: confirm() && ready, busy: busy(),
            tone: ConfirmTone::Neutral, title: tr.invoices_excess_title(),
            confirm_label: tr.invoices_excess_create(), cancel_label: tr.invoices_excess_cancel(),
            on_confirm: generate_excess, on_dismiss: keep_preparing,
            p { {tr.invoices_excess_create_body()} }
            if let Some(review) = estimate.as_ref().filter(|_| ready) {
                for excess in &review.excess {
                    if let Some(line) = review.lines.iter().find(|line| line.source == excess.source) {
                        p { key: "{excess.source:?}", {tr.invoices_excess_line(money(excess.excess_cents, &currency), line.description.clone())} }
                    }
                }
            }
        }
        }
    }
}

#[component]
fn PresetOption(value: Preset, current: Preset, onpick: Callback<Preset>) -> Element {
    let tr = use_tr();
    let choose = use_callback(move |_: MouseEvent| onpick.call(value));
    rsx! { MenuItem { selected: current == value, onclick: choose, {value.label(tr)} } }
}

/// "Use the values of Website": fills the invoice values from one project.
#[component]
fn UseDefaults(
    name: String,
    defaults: InvoiceDefaults,
    fields: Signal<InvoiceDefaultsInput>,
    disabled: bool,
) -> Element {
    let tr = use_tr();
    let mut fields = fields;
    let apply = use_callback(move |_: MouseEvent| fields.set(fields_from(&defaults)));
    rsx! {
        button { r#type: "button", class: "btn btn-secondary btn-sm", disabled, onclick: apply,
            {tr.invoices_use_defaults_from(name)}
        }
    }
}

/// One of the client's projects, ticked when it is part of the invoice, with
/// what the preview finds unbilled on it.
#[component]
fn ProjectRow(
    project: Project,
    ticked: bool,
    estimate: Option<InvoicePreparation>,
    disabled: bool,
    ontoggle: Callback<String>,
) -> Element {
    let tr = use_tr();
    let id = project.id.to_string();
    let name_id = format!("inv-project-{id}");
    let toggle = use_callback(move |_: MouseEvent| ontoggle.call(id.clone()));
    let name = if project.active {
        project.name.clone()
    } else {
        tr.invoices_project_archived(project.name.clone())
    };
    let found = if ticked {
        unbilled(estimate.as_ref(), |other| other == project.id)
    } else {
        (0, 0)
    };
    rsx! {
        tr {
            td {
                Checkbox { label: name.clone(), labelledby: name_id.clone(), compact: true, checked: ticked, disabled,
                    onclick: toggle }
            }
            td { id: name_id, class: "wrap-anywhere", "{name}" }
            {unbilled_cells(found, &currency(estimate.as_ref()))}
        }
    }
}

/// The currency the preview prices the invoice in.
fn currency(estimate: Option<&InvoicePreparation>) -> String {
    estimate
        .map(|review| review.currency.clone())
        .unwrap_or_default()
}

/// The unbilled minutes and fees the preview finds on the projects `keeps`
/// lets through, counting only the lines that will be on the invoice.
fn unbilled(estimate: Option<&InvoicePreparation>, keeps: impl Fn(Uuid) -> bool) -> (i64, i64) {
    let lines = estimate
        .into_iter()
        .flat_map(|review| review.lines.iter())
        .filter(|line| line.selected && keeps(line.project_id));
    lines.fold((0, 0), |(minutes, fees), line| {
        let fee = matches!(line.source, InvoiceSource::Fee { .. });
        (
            minutes + line.minutes.map_or(0, i64::from),
            fees + if fee { line.amount_cents } else { 0 },
        )
    })
}

/// A project row's or the total's unbilled time and fees, a dash for none.
fn unbilled_cells((minutes, fees): (i64, i64), currency: &str) -> Element {
    rsx! {
        td { class: "text-mono text-right",
            {if minutes > 0 { horae_core::duration::format_hhmm(minutes) } else { "—".into() }}
        }
        td { class: "text-mono text-right",
            {if fees > 0 { money(fees, currency) } else { "—".into() }}
        }
    }
}

/// The lines the draft will hold, with its totals inside the table.
#[component]
fn PreparedLines(estimate: InvoicePreparation) -> Element {
    let tr = use_tr();
    let currency = estimate.currency.clone();
    let lines: Vec<_> = estimate.lines.iter().filter(|line| line.selected).collect();
    rsx! {
        p { class: "text-sm text-muted mb-3",
            {tr.invoices_preview_dates(lines.len() as i64, estimate.due_on.map(date).unwrap_or_default(), date(estimate.issued_on))}
        }
        DataTable { label: tr.invoices_lines_label(), compact: true,
            table { aria_label: tr.invoices_lines_label(),
                thead { tr {
                    th { scope: "col", {tr.invoices_column_description()} }
                    th { scope: "col", class: "text-right", {tr.invoices_column_hours()} }
                    th { scope: "col", class: "text-right", {tr.invoices_column_rate()} }
                    th { scope: "col", class: "text-right", {tr.invoices_column_amount()} }
                } }
                tbody {
                    for (index, line) in lines.iter().enumerate() {
                        tr { key: "{index}",
                            td { class: "wrap-anywhere",
                                "{line.description}"
                                if let Some(balance) = line.fee_balance {
                                    div { class: "text-sm text-muted mt-2 wrap-anywhere",
                                        p { {tr.invoices_fee_agreed(money(balance.agreed_cents, &currency))} }
                                        p { {tr.invoices_fee_invoiced(money(balance.invoiced_cents, &currency))} }
                                        p {
                                            if balance.remaining_cents < 0 {
                                                {tr.invoices_fee_over(money(balance.remaining_cents, &currency))}
                                            } else {
                                                {tr.invoices_fee_available(money(balance.remaining_cents, &currency))}
                                            }
                                        }
                                        if let Some(net) = line.net_before_tax_cents {
                                            p { {tr.invoices_fee_net(money(net, &currency))} }
                                        }
                                    }
                                }
                            }
                            td { class: "text-mono text-right", {line.minutes.map(|minutes| horae_core::duration::format_hhmm(minutes.into())).unwrap_or_else(|| "—".into())} }
                            td { class: "text-mono text-right", {line.rate_cents.map(|rate| money(rate, &currency)).unwrap_or_else(|| "—".into())} }
                            td { class: "text-mono text-right", {money(line.amount_cents, &currency)} }
                        }
                    }
                }
                tfoot {
                    for (label, amount) in estimate.breakdown().unwrap_or_default() {
                        tr { key: "{label}",
                            th { scope: "row", colspan: "3", class: "text-right wrap-anywhere", {breakdown_label(tr, &label)} }
                            td { class: if label == "Total" { "text-mono text-right font-semibold" } else { "text-mono text-right" }, {money(amount, &currency)} }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_cover_whole_months_and_everything_up_to_today() {
        let today: NaiveDate = "2026-03-15".parse().unwrap();
        let day = |text: &str| text.parse::<NaiveDate>().unwrap();
        assert_eq!(
            Preset::ThisMonth.range(today, None),
            Some((day("2026-03-01"), today))
        );
        assert_eq!(
            Preset::LastMonth.range(today, None),
            Some((day("2026-02-01"), day("2026-02-28")))
        );
        assert_eq!(
            Preset::UpToToday.range(today, None).map(|r| r.1),
            Some(today)
        );
        assert_eq!(Preset::Custom.range(today, None), None);
    }

    fn last(covered_to: &str) -> LastInvoice {
        LastInvoice {
            id: Uuid::nil(),
            number: "INV-1".into(),
            issued_on: covered_to.parse().unwrap(),
            covered_to: covered_to.parse().unwrap(),
        }
    }

    #[test]
    fn the_period_starts_after_the_last_invoice_or_this_month() {
        let today: NaiveDate = "2026-03-15".parse().unwrap();
        assert_eq!(Preset::default_for(None), Preset::ThisMonth);
        let previous = last("2026-02-20");
        assert_eq!(
            Preset::default_for(Some(&previous)),
            Preset::SinceLastInvoice
        );
        assert_eq!(
            Preset::SinceLastInvoice.range(today, Some(&previous)),
            Some(("2026-02-21".parse().unwrap(), today))
        );
        // A last invoice that already covers today does not start tomorrow.
        assert_eq!(
            Preset::SinceLastInvoice.range(today, Some(&last("2026-03-15"))),
            Some((today, today))
        );
        assert_eq!(Preset::SinceLastInvoice.range(today, None), None);
    }

    #[test]
    fn invoice_review_requires_matching_sources_and_valid_values() {
        let request = InvoiceRequest {
            client: "client".into(),
            from: "2026-01-01".into(),
            to: "2026-01-31".into(),
            projects: None,
            overrides: Some(InvoiceDefaults::default()),
            fees: None,
        };
        assert!(is_reviewed(Some(&request), Some(&request)));
        assert!(!is_reviewed(None, Some(&request)));
        assert!(!is_reviewed(None, None));
        assert!(!is_reviewed(Some(&request), None));
        let changes = [
            InvoiceRequest {
                client: "different".into(),
                ..request.clone()
            },
            InvoiceRequest {
                from: "2026-01-02".into(),
                ..request.clone()
            },
            InvoiceRequest {
                to: "2026-02-01".into(),
                ..request.clone()
            },
            InvoiceRequest {
                projects: Some(vec!["project".into()]),
                ..request.clone()
            },
            InvoiceRequest {
                overrides: None,
                ..request.clone()
            },
            InvoiceRequest {
                fees: Some(Vec::new()),
                ..request.clone()
            },
            InvoiceRequest {
                overrides: Some(InvoiceDefaults {
                    tax1_bps: 2100,
                    ..Default::default()
                }),
                ..request.clone()
            },
        ];
        for changed in changes {
            assert!(!is_reviewed(Some(&changed), Some(&request)));
        }
    }
}
