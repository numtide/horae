//! Invoice settings: the organization's issuer details, payment details and
//! invoice defaults. `get_org_branding`/`update_org_branding` decide who may
//! open and save it; this page only hides the way in.

use dioxus::prelude::*;
use horae_core::permissions::catalog::{PERMISSION_CATALOG_VERSION, Permission};

use crate::components::form::{FormGroup, Input, Textarea};
use crate::models::own_permissions::OwnPermissions;
use crate::models::{OrgBranding, branding_value_error};
use crate::route::Route;
use crate::server_fns;

const FORBIDDEN: u16 = 403;
/// Addresses and notes keep the line breaks they were typed with, as on the PDF.
const PRE_LINE: &str = "white-space: pre-line;";

/// Whether to offer the settings action: the grant `update_org_branding`
/// requires, from a catalog this build understands.
pub(crate) fn edits_invoice_settings(own: &OwnPermissions) -> bool {
    own.catalog_version == PERMISSION_CATALOG_VERSION
        && own.grants.contains(&Permission::InvoiceWriteAll)
}

/// The server's own message, without Dioxus's "error running server function" wrapper.
fn message_of(error: ServerFnError) -> String {
    match error {
        ServerFnError::ServerError { message, .. } => message,
        other => other.to_string(),
    }
}

struct Field {
    id: &'static str,
    label: &'static str,
    kind: &'static str,
    multiline: bool,
    value: fn(&mut OrgBranding) -> &mut Option<String>,
}

const fn field(
    id: &'static str,
    label: &'static str,
    kind: &'static str,
    value: fn(&mut OrgBranding) -> &mut Option<String>,
) -> Field {
    Field {
        id,
        label,
        kind,
        multiline: false,
        value,
    }
}

const fn multiline(
    id: &'static str,
    label: &'static str,
    value: fn(&mut OrgBranding) -> &mut Option<String>,
) -> Field {
    Field {
        multiline: true,
        ..field(id, label, "text", value)
    }
}

/// The settings cards; each one saves only its own fields.
#[derive(Clone, Copy, PartialEq)]
enum Card {
    Company,
    Payment,
    Defaults,
}

impl Card {
    const ALL: [Card; 3] = [Card::Company, Card::Payment, Card::Defaults];

    fn title(self) -> &'static str {
        match self {
            Card::Company => "Company information",
            Card::Payment => "Payment details",
            Card::Defaults => "Default values",
        }
    }

    fn description(self) -> &'static str {
        match self {
            Card::Company => {
                "Your details as the issuer. Every invoice PDF shows the current values, including invoices already sent."
            }
            Card::Payment => {
                "How clients pay you. Every invoice PDF shows the current values, including invoices already sent."
            }
            Card::Defaults => {
                "Terms print on every invoice PDF. Notes are copied into each new invoice; existing invoices keep theirs."
            }
        }
    }

    fn fields(self) -> &'static [Field] {
        const COMPANY: [Field; 5] = [
            field("inv-settings-name", "Company name", "text", |b| {
                &mut b.provider_name
            }),
            multiline("inv-settings-address", "Address", |b| {
                &mut b.provider_address
            }),
            field("inv-settings-tax-id", "Tax ID", "text", |b| {
                &mut b.provider_tax_id
            }),
            field("inv-settings-email", "Email", "email", |b| {
                &mut b.provider_email
            }),
            field("inv-settings-phone", "Phone", "tel", |b| {
                &mut b.provider_phone
            }),
        ];
        const PAYMENT: [Field; 5] = [
            field("inv-settings-bank", "Bank name", "text", |b| {
                &mut b.bank_name
            }),
            field("inv-settings-iban", "IBAN", "text", |b| &mut b.bank_iban),
            field("inv-settings-bic", "BIC / SWIFT", "text", |b| {
                &mut b.bank_bic
            }),
            field("inv-settings-routing", "Routing number", "text", |b| {
                &mut b.bank_routing
            }),
            field("inv-settings-account", "Account number", "text", |b| {
                &mut b.bank_account
            }),
        ];
        const DEFAULTS: [Field; 2] = [
            field("inv-settings-terms", "Payment terms", "text", |b| {
                &mut b.invoice_payment_terms
            }),
            multiline("inv-settings-notes", "Invoice notes", |b| {
                &mut b.invoice_notes
            }),
        ];
        match self {
            Card::Company => &COMPANY,
            Card::Payment => &PAYMENT,
            Card::Defaults => &DEFAULTS,
        }
    }

    /// `saved` with this card's fields taken from `draft`, trimmed and blank
    /// ones cleared, so saving one card never sends another card's unsaved edits.
    fn request(self, saved: &OrgBranding, draft: &OrgBranding) -> OrgBranding {
        let (mut request, mut draft) = (saved.clone(), draft.clone());
        for field in self.fields() {
            let value = (field.value)(&mut draft).take();
            *(field.value)(&mut request) = value
                .map(|value| value.trim().to_owned())
                .filter(|value| !value.is_empty());
        }
        request
    }

    /// The first field this card cannot save, with the reason.
    fn first_error(self, draft: &OrgBranding) -> Option<(&'static str, &'static str)> {
        let mut draft = draft.clone();
        self.fields().iter().find_map(|field| {
            let value = (field.value)(&mut draft).as_deref()?;
            branding_value_error(value).map(|message| (field.id, message))
        })
    }
}

#[component]
pub fn InvoiceSettings() -> Element {
    let branding = use_resource(server_fns::get_org_branding);
    rsx! {
        div {
            div { class: "page-header",
                h1 { class: "page-title", "Invoice settings" }
                div { class: "page-actions",
                    Link { to: Route::InvoiceList {}, class: "btn btn-secondary", "Back to Invoices" }
                }
            }
            match &*branding.read() {
                None => rsx! { p { class: "text-muted text-sm", role: "status", "Loading invoice settings…" } },
                Some(Ok(initial)) => rsx! { SettingsForm { initial: initial.clone() } },
                Some(Err(ServerFnError::ServerError { code: FORBIDDEN, .. })) => rsx! {
                    div { class: "alert alert-danger", role: "alert",
                        "Invoice settings are available to people who manage all invoices. Ask an Administrator if you need access."
                    }
                },
                Some(Err(error)) => rsx! { div { class: "alert alert-danger", role: "alert", {message_of(error.clone())} } },
            }
        }
    }
}

#[component]
fn SettingsForm(initial: OrgBranding) -> Element {
    let saved = use_signal(|| initial.clone());
    let draft = use_signal(|| initial.clone());
    rsx! {
        div { class: "flex flex-col lg:flex-row gap-6",
            div { class: "flex flex-col gap-6 flex-1 min-w-0",
                for card in Card::ALL {
                    SettingsCard { key: "{card.title()}", card, saved, draft }
                }
            }
            // The wrapper stretches with the row; the preview card keeps its own height.
            div { class: "w-full max-w-sm min-w-0",
                InvoicePreview { draft }
            }
        }
    }
}

#[component]
fn SettingsCard(card: Card, saved: Signal<OrgBranding>, draft: Signal<OrgBranding>) -> Element {
    let mut busy = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let mut notice = use_signal(|| false);
    let mut invalid = use_signal(|| None::<(&'static str, &'static str)>);
    let heading = format!("{}-title", card.fields()[0].id);

    let save = move |event: FormEvent| {
        event.prevent_default();
        if busy() {
            return;
        }
        notice.set(false);
        error.set(None);
        invalid.set(card.first_error(&draft.peek()));
        if invalid.peek().is_some() {
            return;
        }
        let expected = saved.peek().clone();
        let request = card.request(&expected, &draft.peek());
        busy.set(true);
        spawn(async move {
            match server_fns::update_org_branding(request, expected).await {
                Ok(stored) => {
                    let (mut current, mut from_server) = (draft.peek().clone(), stored.clone());
                    for field in card.fields() {
                        *(field.value)(&mut current) = (field.value)(&mut from_server).take();
                    }
                    draft.set(current);
                    saved.set(stored);
                    notice.set(true);
                }
                // A conflict keeps the draft; the message says how to recover.
                Err(failure) => error.set(Some(message_of(failure))),
            }
            busy.set(false);
        });
    };

    rsx! {
        form { class: "card p-5", novalidate: true, aria_labelledby: "{heading}", onsubmit: save,
            h2 { id: "{heading}", class: "card-title", "{card.title()}" }
            p { class: "text-sm text-secondary mb-4", "{card.description()}" }
            if let Some(message) = error() {
                div { class: "alert alert-danger", role: "alert", "{message}" }
            }
            fieldset { class: "border-0 p-0 m-0 min-w-0", disabled: busy(),
                for field in card.fields() {
                    {
                        let value = (field.value)(&mut draft.read().clone()).clone().unwrap_or_default();
                        let message = invalid().filter(|(id, _)| *id == field.id).map(|(_, message)| message);
                        let error_id = message.map(|_| format!("{}-error", field.id));
                        let edit = move |event: FormEvent| {
                            notice.set(false);
                            if invalid.peek().is_some_and(|(id, _)| id == field.id) {
                                invalid.set(None);
                            }
                            *(field.value)(&mut draft.write()) = Some(event.value());
                        };
                        rsx! {
                            FormGroup { key: "{field.id}", label: field.label, id: field.id,
                                if field.multiline {
                                    Textarea { id: field.id, error_id: error_id.clone(), value, rows: 4, oninput: edit }
                                } else {
                                    Input { id: field.id, kind: field.kind, error_id: error_id.clone(), value, oninput: edit }
                                }
                                if let (Some(id), Some(message)) = (error_id, message) {
                                    p { id, class: "text-sm text-danger mt-1", "{message}" }
                                }
                            }
                        }
                    }
                }
            }
            div { class: "flex flex-wrap items-center gap-3",
                button { r#type: "submit", class: "btn btn-solid", disabled: busy(),
                    if busy() { "Saving…" } else { "Save {card.title().to_lowercase()}" }
                }
                p { class: "text-sm text-secondary m-0", role: "status",
                    if notice() { "Saved." }
                }
            }
        }
    }
}

/// How the issuer block, payment details, terms and notes read on an invoice
/// PDF; it follows `templates/invoice.typ` and shows unsaved edits.
#[component]
fn InvoicePreview(draft: Signal<OrgBranding>) -> Element {
    let b = draft.read();
    let filled = |value: &Option<String>| value.clone().filter(|value| !value.trim().is_empty());
    let issuer = [
        b.provider_address.clone(),
        b.provider_tax_id.as_ref().map(|id| format!("Tax ID: {id}")),
        b.provider_email.clone(),
        b.provider_phone.clone(),
    ];
    let payment = [
        ("Bank", &b.bank_name),
        ("IBAN", &b.bank_iban),
        ("BIC", &b.bank_bic),
        ("Routing", &b.bank_routing),
        ("Account", &b.bank_account),
    ]
    .into_iter()
    .filter_map(|(label, value)| filled(value).map(|value| format!("{label}: {value}")))
    .collect::<Vec<_>>();
    let terms = filled(&b.invoice_payment_terms);
    let notes = filled(&b.invoice_notes);
    rsx! {
        section { class: "card p-5 min-w-0", aria_labelledby: "inv-settings-preview-title",
            h2 { id: "inv-settings-preview-title", class: "card-title", "Preview" }
            p { class: "text-sm text-secondary mb-4", "How these details read on an invoice, including unsaved edits." }
            div { class: "flex flex-col gap-4 text-sm wrap-anywhere",
                div {
                    if let Some(name) = filled(&b.provider_name) {
                        p { class: "font-semibold m-0", "{name}" }
                    } else {
                        p { class: "text-muted m-0", "No company name" }
                    }
                    for line in issuer.into_iter().filter_map(|value| filled(&value)) {
                        p { class: "text-secondary m-0", style: PRE_LINE, "{line}" }
                    }
                }
                if !payment.is_empty() {
                    PreviewBlock { title: "Payment details", lines: payment }
                }
                if let Some(terms) = terms {
                    PreviewBlock { title: "Terms", lines: vec![terms] }
                }
                if let Some(notes) = notes {
                    PreviewBlock { title: "Notes", lines: vec![notes] }
                }
            }
        }
    }
}

#[component]
fn PreviewBlock(title: &'static str, lines: Vec<String>) -> Element {
    rsx! {
        div {
            p { class: "text-xs font-semibold uppercase tracking-wide text-muted m-0 mb-1", "{title}" }
            for line in lines {
                p { class: "m-0", style: PRE_LINE, "{line}" }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn branding() -> OrgBranding {
        OrgBranding {
            provider_name: Some("Saved name".into()),
            provider_address: None,
            provider_tax_id: None,
            provider_email: None,
            provider_phone: None,
            bank_name: Some("Saved bank".into()),
            bank_iban: None,
            bank_bic: None,
            bank_routing: None,
            bank_account: None,
            invoice_notes: None,
            invoice_payment_terms: Some("Net 30".into()),
        }
    }

    #[test]
    fn only_invoice_writers_for_everyone_are_offered_the_settings() {
        let own = |grants: &[Permission], is_administrator, catalog_version| OwnPermissions {
            catalog_version,
            grants: grants.to_vec(),
            is_administrator,
            access_revision: 1,
            person_revision: 1,
            managed_person_ids: Vec::new(),
            managed_project_ids: Vec::new(),
        };
        let current = PERMISSION_CATALOG_VERSION;
        assert_eq!(
            [
                own(&[Permission::InvoiceWriteAll], false, current),
                own(&[Permission::InvoiceReadAll], false, current),
                own(&[Permission::InvoiceWriteManaged], false, current),
                // Being an administrator is not the grant.
                own(&[Permission::CompanyWrite], true, current),
                own(&[Permission::InvoiceWriteAll], false, current + 1),
            ]
            .iter()
            .map(edits_invoice_settings)
            .collect::<Vec<_>>(),
            [true, false, false, false, false]
        );
    }

    #[test]
    fn every_branding_field_belongs_to_exactly_one_card() {
        let mut all = branding();
        for card in Card::ALL {
            for field in card.fields() {
                let previous = (field.value)(&mut all).replace("seen".into());
                assert_ne!(
                    previous.as_deref(),
                    Some("seen"),
                    "{} is repeated",
                    field.id
                );
            }
        }
        let seen = Some("seen".to_string());
        assert_eq!(
            all,
            OrgBranding {
                provider_name: seen.clone(),
                provider_address: seen.clone(),
                provider_tax_id: seen.clone(),
                provider_email: seen.clone(),
                provider_phone: seen.clone(),
                bank_name: seen.clone(),
                bank_iban: seen.clone(),
                bank_bic: seen.clone(),
                bank_routing: seen.clone(),
                bank_account: seen.clone(),
                invoice_notes: seen.clone(),
                invoice_payment_terms: seen,
            }
        );
    }

    #[test]
    fn saving_a_card_sends_only_its_own_trimmed_edits_and_clears_blank_values() {
        let saved = branding();
        let draft = OrgBranding {
            provider_name: Some("  ".into()),
            provider_email: Some("  billing@example.com\n".into()),
            bank_name: Some("Unsaved bank".into()),
            ..saved.clone()
        };
        assert_eq!(
            Card::Company.request(&saved, &draft),
            OrgBranding {
                provider_name: None,
                provider_email: Some("billing@example.com".into()),
                ..saved
            }
        );
    }

    #[test]
    fn a_refused_save_shows_the_server_message_alone() {
        let refused = ServerFnError::ServerError {
            message: "Manager access required".into(),
            code: FORBIDDEN,
            details: None,
        };
        assert_eq!(message_of(refused), "Manager access required");
    }

    #[test]
    fn a_card_reports_the_first_value_it_cannot_save() {
        let draft = OrgBranding {
            bank_iban: Some("x".repeat(crate::models::organization::MAX_BRANDING_VALUE_BYTES + 1)),
            invoice_notes: Some("bad\0".into()),
            ..branding()
        };
        assert_eq!(Card::Company.first_error(&draft), None);
        assert_eq!(
            Card::Payment.first_error(&draft).map(|(id, _)| id),
            Some("inv-settings-iban")
        );
        assert_eq!(
            Card::Defaults.first_error(&draft).map(|(id, _)| id),
            Some("inv-settings-notes")
        );
    }
}
