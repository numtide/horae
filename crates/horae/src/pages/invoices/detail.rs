use dioxus::prelude::*;
use horae_core::types::InvoiceStatus;
use uuid::Uuid;

use super::recovery::RecoveryStorage;
use super::{
    INVOICE_HEADING, Shown, StatusBadge, breakdown_label, date, focus_soon, manages_invoices,
    money, prepares_invoices,
};
use crate::components::back_link::BackLink;
use crate::components::button::Button;
use crate::components::confirm_dialog::ConfirmDialog;
use crate::components::menu::{Menu, MenuDivider, MenuItem};
use crate::components::page_header::PageHeader;
use crate::components::screen_state::{LoadError, Skeleton};
use crate::components::table::DataTable;
use crate::components::toast::{Toast, ToastContainer};
use crate::i18n::use_tr;
use crate::models::invoice::InvoiceDetailView;
use crate::route::Route;
use crate::server_fns;

#[component]
pub(super) fn InvoiceDetailContent(id: Uuid) -> Element {
    let tr = use_tr();
    let mut invoice =
        use_resource(move || async move { server_fns::get_invoice(id.to_string()).await });
    let retry = use_callback(move |()| invoice.restart());
    rsx! {
        div {
            BackLink { to: Route::InvoiceList {}, label: tr.invoices_back() }
            match &*invoice.read() {
                Some(Ok(data)) => rsx! { Detail { data: data.clone(), onchanged: retry } },
                Some(Err(error)) => rsx! {
                    PageHeader { title: tr.invoices_invoice() }
                    LoadError { error: error.clone(), what: tr.invoices_detail_what(), retry,
                        pending: invoice.state()() != UseResourceState::Ready, focus_on_recovery: "page-back" }
                },
                None => rsx! {
                    PageHeader { title: tr.invoices_invoice() }
                    Skeleton { rows: 4, columns: 4, label: tr.invoices_detail_loading() }
                },
            }
        }
    }
}

#[component]
fn Detail(data: InvoiceDetailView, onchanged: EventHandler<()>) -> Element {
    let tr = use_tr();
    let storage = use_context::<RecoveryStorage>();
    let session = crate::components::session::use_session();
    let navigator = use_navigator();
    let manages = manages_invoices(session) && storage.ready();
    let prepares = prepares_invoices(session) && storage.ready();
    let mut busy = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let mut confirm_void = use_signal(|| false);
    let mut void_error = use_signal(|| None::<String>);
    let mut notice = use_signal(|| None::<String>);

    let inv = data.invoice.clone();
    let id = inv.id;
    let currency = inv.currency.trim().to_owned();
    let shown = Shown::of(inv.status, data.overdue);
    let open = matches!(inv.status, InvoiceStatus::Draft | InvoiceStatus::Sent);
    let edits = prepares && inv.status == InvoiceStatus::Draft;
    let voids = manages && open;

    // Moves the invoice on; a failed void stays inside its confirmation.
    let change = use_callback(move |target: InvoiceStatus| {
        if *busy.peek() {
            return;
        }
        busy.set(true);
        error.set(None);
        void_error.set(None);
        spawn(async move {
            let wire = match target {
                InvoiceStatus::Sent => "sent",
                InvoiceStatus::Paid => "paid",
                InvoiceStatus::Void => "void",
                InvoiceStatus::Draft => "draft",
            };
            match server_fns::update_invoice_status(id.to_string(), wire.to_owned()).await {
                Ok(_) => {
                    confirm_void.set(false);
                    notice.set(Some(match target {
                        InvoiceStatus::Sent => tr.invoices_marked_sent(),
                        InvoiceStatus::Paid => tr.invoices_marked_paid(),
                        _ => tr.invoices_voided(),
                    }));
                    if target == InvoiceStatus::Void {
                        // Voiding removes the menu that held it.
                        focus_soon(INVOICE_HEADING);
                    }
                    onchanged.call(());
                }
                Err(failure) if target == InvoiceStatus::Void => {
                    void_error.set(Some(failure.to_string()))
                }
                Err(failure) => error.set(Some(failure.to_string())),
            }
            busy.set(false);
        });
    });
    let mark_sent = use_callback(move |_: MouseEvent| change.call(InvoiceStatus::Sent));
    let mark_paid = use_callback(move |_: MouseEvent| change.call(InvoiceStatus::Paid));
    let void = use_callback(move |()| change.call(InvoiceStatus::Void));
    let ask_void = use_callback(move |_: MouseEvent| {
        void_error.set(None);
        confirm_void.set(true);
    });
    let keep = use_callback(move |()| {
        if !*busy.peek() {
            confirm_void.set(false);
        }
    });
    let client_id = inv.client_id;
    let edit = use_callback(move |_: MouseEvent| {
        navigator.push(Route::EditInvoice { id });
    });
    let duplicate = use_callback(move |_: MouseEvent| {
        navigator.push(Route::DuplicateInvoice {
            client: client_id.to_string(),
            source: id,
        });
    });
    let dismiss = use_callback(move |_: MouseEvent| notice.set(None));

    rsx! {
        PageHeader { title: tr.invoices_detail_title(inv.number.clone()), title_id: INVOICE_HEADING,
            meta: rsx! { StatusBadge { shown } },
            actions: rsx! {
                if prepares || voids {
                    Menu { id: "invoice-actions", label: tr.invoices_more(), align_right: true, trigger_class: "text-sm py-3 px-5",
                        if edits {
                            MenuItem { id: "invoice-edit", disabled: busy(), onclick: edit, {tr.invoices_edit_draft()} }
                        }
                        if prepares {
                            MenuItem { id: "invoice-duplicate", disabled: busy(), onclick: duplicate, {tr.invoices_duplicate()} }
                        }
                        if voids {
                            if prepares { MenuDivider {} }
                            MenuItem { id: "invoice-void", danger: true, disabled: busy(), onclick: ask_void, {tr.invoices_void_ellipsis()} }
                        }
                    }
                }
                a { class: "btn btn-secondary", href: "/api/invoices/{id}/export/pdf", target: "_blank", {tr.invoices_download_pdf()} }
                if manages && inv.status == InvoiceStatus::Draft {
                    Button { id: "invoice-mark-sent", label: tr.invoices_mark_sent(), disabled: busy(), onclick: mark_sent }
                }
                if manages && inv.status == InvoiceStatus::Sent {
                    Button { id: "invoice-mark-paid", label: tr.invoices_mark_paid(), disabled: busy(), onclick: mark_paid }
                }
            },
        }
        ConfirmDialog { id: "invoice-void-dialog", labelledby: "invoice-void-title",
            open: confirm_void(), busy: busy(), focus_fallback: Some(INVOICE_HEADING),
            title: tr.invoices_void_title(inv.number.clone()),
            confirm_label: tr.invoices_void_confirm(),
            on_confirm: void, on_dismiss: keep,
            p { {tr.invoices_void_body()} }
            if let Some(failure) = void_error() {
                p { class: "alert alert-danger", role: "alert", "{failure}" }
            }
        }
        if let Some(failure) = error() {
            div { class: "alert alert-danger", role: "alert", "{failure}" }
        }

        section { class: "card p-5 mb-6", aria_label: tr.invoices_summary_label(),
            div { class: "grid gap-4 inv-meta",
                Fact { label: tr.invoices_column_client(), value: data.client_name.clone() }
                Fact { label: tr.invoices_column_issued(), value: date(inv.issued_on), mono: true }
                Fact { label: tr.invoices_column_due(), value: date(inv.due_on), mono: true, danger: data.overdue }
                Fact { label: tr.invoices_terms(), value: tr.invoices_terms_days(i64::from(inv.terms_days)) }
                if !inv.po_number.is_empty() {
                    Fact { label: tr.invoices_po_number(), value: inv.po_number.clone() }
                }
                Fact { label: tr.invoices_total(), value: money(inv.total_cents, &currency), mono: true }
            }
            if !inv.subject.is_empty() {
                div { class: "mt-4",
                    div { class: "text-sm text-muted", {tr.invoices_subject()} }
                    div { class: "wrap-anywhere", "{inv.subject}" }
                }
            }
        }

        DataTable {
            table {
                thead { tr {
                    th { scope: "col", {tr.invoices_column_description()} }
                    th { scope: "col", class: "text-right", {tr.invoices_column_hours()} }
                    th { scope: "col", class: "text-right", {tr.invoices_column_rate()} }
                    th { scope: "col", class: "text-right", {tr.invoices_column_amount()} }
                } }
                tbody {
                    for line in data.lines.iter() {
                        tr { key: "{line.id}",
                            td { class: "wrap-anywhere", "{line.description}" }
                            td { class: "text-mono text-right",
                                {line.minutes.map(|minutes| horae_core::duration::format_hhmm(minutes.into())).unwrap_or_else(|| "—".into())}
                            }
                            td { class: "text-mono text-right",
                                {line.rate_cents.map(|rate| money(rate, &currency)).unwrap_or_else(|| "—".into())}
                            }
                            td { class: "text-mono text-right", {money(line.amount_cents, &currency)} }
                        }
                    }
                }
                tfoot {
                    for (label, cents) in inv.breakdown() {
                        tr { key: "{label}",
                            th { scope: "row", colspan: "3", class: "text-right wrap-anywhere", {breakdown_label(tr, &label)} }
                            td { class: if label == "Total" { "text-mono text-right font-semibold" } else { "text-mono text-right" },
                                {money(cents, &currency)}
                            }
                        }
                    }
                }
            }
        }
        if let Some(notes) = inv.notes.as_ref().filter(|notes| !notes.trim().is_empty()) {
            section { class: "mt-6",
                h2 { class: "card-title", {tr.invoices_notes()} }
                p { class: "text-secondary wrap-anywhere", "{notes}" }
            }
        }
        ToastContainer {
            if let Some(message) = notice() {
                Toast { message, variant: "success", icon: "✓", dismissible: true, ondismiss: dismiss }
            }
        }
    }
}

/// One labelled figure in the summary.
#[component]
fn Fact(
    label: String,
    value: String,
    #[props(default)] mono: bool,
    #[props(default)] danger: bool,
) -> Element {
    let class = match (mono, danger) {
        (true, true) => "text-mono text-danger",
        (true, false) => "text-mono",
        (false, _) => "wrap-anywhere",
    };
    rsx! {
        div {
            div { class: "text-sm text-muted", "{label}" }
            div { class, "{value}" }
        }
    }
}
