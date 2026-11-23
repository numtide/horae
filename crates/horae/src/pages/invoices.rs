use dioxus::prelude::*;
use horae_core::permissions::catalog::{PERMISSION_CATALOG_VERSION, Permission};
use horae_core::permissions::invoices;
use horae_core::types::InvoiceStatus;
use uuid::Uuid;

use crate::components::badge::Badge;
use crate::components::session::Session;
use crate::i18n::Tr;

/// The Invoices title, where focus lands once a failed read loads again.
const INVOICES_HEADING: &str = "invoices-heading";
/// An invoice's title, where focus lands once voiding removes its menu.
const INVOICE_HEADING: &str = "invoice-heading";

/// Focus `id` after the next frame, once the render that shows it has landed.
fn focus_soon(id: &str) {
    let id = serde_json::json!(id);
    document::eval(&format!(
        "requestAnimationFrame(() => document.getElementById({id})?.focus());"
    ));
}

#[path = "invoices/defaults_form.rs"]
mod defaults_form;
#[path = "invoices/detail.rs"]
mod detail;
#[path = "invoices/editing.rs"]
mod editing;
#[path = "invoices/filter.rs"]
mod filter;
#[path = "invoices/list.rs"]
mod list;
#[path = "invoices/preparation.rs"]
mod preparation;
#[path = "invoices/recovery.rs"]
mod recovery;
#[path = "invoices/settings.rs"]
mod settings;

pub use filter::InvoiceFilter;
pub use settings::InvoiceSettings;

/// Whether the session's own permissions, from a catalog this build
/// understands, hold every `required` grant: the same sets the server checks.
/// Reading ([`invoices::READ`]) is checked by `RecoveryGate`.
fn holds(session: Session, required: &[Permission]) -> bool {
    matches!(
        &*session.permissions.read(),
        Ok(own) if own.catalog_version == PERMISSION_CATALOG_VERSION
            && invoices::holds(|grant| own.grants.contains(&grant), required)
    )
}

/// Whether to offer changing an invoice's status.
fn manages_invoices(session: Session) -> bool {
    holds(session, invoices::WRITE)
}

/// Whether to offer preparing a new invoice or editing a draft's values.
fn prepares_invoices(session: Session) -> bool {
    holds(session, invoices::PREPARE)
}

/// What an invoice's badge says: its status, or Overdue for a sent invoice
/// past its due date. Overdue is computed, never stored.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Shown {
    Status(InvoiceStatus),
    Overdue,
}

impl Shown {
    fn of(status: InvoiceStatus, overdue: bool) -> Self {
        if overdue {
            Self::Overdue
        } else {
            Self::Status(status)
        }
    }

    fn label(self, tr: Tr) -> String {
        match self {
            Self::Status(InvoiceStatus::Draft) => tr.invoices_status_draft(),
            Self::Status(InvoiceStatus::Sent) => tr.invoices_status_sent(),
            Self::Status(InvoiceStatus::Paid) => tr.invoices_status_paid(),
            Self::Status(InvoiceStatus::Void) => tr.invoices_status_void(),
            Self::Status(InvoiceStatus::Closed) => tr.invoices_status_closed(),
            Self::Overdue => tr.invoices_status_overdue(),
        }
    }

    fn variant(self) -> &'static str {
        match self {
            Self::Status(InvoiceStatus::Draft) => "warning",
            Self::Status(InvoiceStatus::Sent) => "info",
            Self::Status(InvoiceStatus::Paid) => "success",
            Self::Status(InvoiceStatus::Void | InvoiceStatus::Closed) => "neutral",
            Self::Overdue => "danger",
        }
    }
}

/// The status badge, one convention for the list and the detail.
#[component]
pub(super) fn StatusBadge(shown: Shown) -> Element {
    let tr = crate::i18n::use_tr();
    rsx! { Badge { variant: shown.variant(), {shown.label(tr)} } }
}

/// A date the way the app writes it (`16 Nov 2026`).
fn date(day: chrono::NaiveDate) -> String {
    day.format("%-d %b %Y").to_string()
}

/// An amount through the one money formatter.
fn money(cents: i64, currency: &str) -> String {
    horae_core::money::format_cents(cents, currency)
}

/// Translates a stored breakdown label. The server builds them in English for
/// the PDF; the screen names them from the catalog.
fn breakdown_label(tr: Tr, label: &str) -> String {
    match label {
        "Subtotal" => tr.invoices_subtotal(),
        "Total" => tr.invoices_total(),
        "Paid" => tr.invoices_paid(),
        "Due" => tr.invoices_amount_due(),
        other => other.to_owned(),
    }
}

/// Step 1 of a new invoice: who it is for.
#[component]
pub fn NewInvoice() -> Element {
    rsx! { recovery::RecoveryGate { preparation::ChooseClient {} } }
}

/// Step 2 of a new invoice for one client.
#[component]
pub fn NewInvoiceForClient(client: String) -> Element {
    rsx! {
        for client in [client] {
            recovery::RecoveryGate { key: "{client}",
                preparation::PrepareForClient { client: client.clone(), duplicate: None }
            }
        }
    }
}

/// Step 2 started from invoice `source`: its values and projects, for the
/// same client.
#[component]
pub fn DuplicateInvoice(client: String, source: Uuid) -> Element {
    rsx! {
        for (client, source) in [(client, source)] {
            recovery::RecoveryGate { key: "{client}:{source}",
                preparation::PrepareForClient { client: client.clone(), duplicate: Some(source) }
            }
        }
    }
}

#[component]
pub fn InvoiceList(filter: InvoiceFilter) -> Element {
    rsx! { recovery::RecoveryGate { list::InvoiceListContent { filter } } }
}

#[component]
pub fn InvoiceDetail(id: Uuid) -> Element {
    // A keyed fragment resets invoice-local resources and actions when the
    // router reuses this page for another ID.
    rsx! { for id in [id] { recovery::RecoveryGate { key: "{id}", detail::InvoiceDetailContent { id } } } }
}

/// The draft editor, its own screen under the invoice.
#[component]
pub fn EditInvoice(id: Uuid) -> Element {
    rsx! { for id in [id] { recovery::RecoveryGate { key: "{id}", editing::DraftEditorPage { id } } } }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overdue_replaces_the_sent_badge_only() {
        assert_eq!(Shown::of(InvoiceStatus::Sent, true), Shown::Overdue);
        assert_eq!(
            Shown::of(InvoiceStatus::Sent, false),
            Shown::Status(InvoiceStatus::Sent)
        );
        assert_eq!(Shown::Overdue.variant(), "danger");
    }
}
