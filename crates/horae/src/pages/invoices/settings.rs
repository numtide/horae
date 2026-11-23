//! Invoice settings: the organization's issuer details, payment details and
//! invoice defaults. `get_org_branding`/`update_org_branding` decide who may
//! open and save it; this page only hides the way in.

use dioxus::prelude::*;
use horae_core::permissions::catalog::{PERMISSION_CATALOG_VERSION, Permission};

use crate::components::back_link::BackLink;
use crate::components::form::{FormGroup, Input, Textarea};
use crate::components::page_header::PageHeader;
use crate::components::screen_state::{LoadError, Loading};
use crate::i18n::{Tr, use_tr};
use crate::models::own_permissions::OwnPermissions;
use crate::models::{OrgBranding, branding_value_error};
use crate::route::Route;
use crate::server_fns;

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

/// Why a value cannot be saved, in the catalog's words. The rule itself is
/// `branding_value_error`, which the server applies too.
fn value_error(tr: Tr, value: &str) -> Option<String> {
    branding_value_error(value)?;
    Some(
        if value.len() > crate::models::organization::MAX_BRANDING_VALUE_BYTES {
            tr.invoices_settings_too_long()
        } else {
            tr.invoices_settings_nul()
        },
    )
}

struct Field {
    id: &'static str,
    label: fn(Tr) -> String,
    kind: &'static str,
    multiline: bool,
    value: fn(&mut OrgBranding) -> &mut Option<String>,
}

const fn field(
    id: &'static str,
    label: fn(Tr) -> String,
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
    label: fn(Tr) -> String,
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

    fn title(self, tr: Tr) -> String {
        match self {
            Card::Company => tr.invoices_settings_company(),
            Card::Payment => tr.invoices_settings_payment(),
            Card::Defaults => tr.invoices_settings_defaults(),
        }
    }

    fn save_label(self, tr: Tr) -> String {
        match self {
            Card::Company => tr.invoices_settings_save_company(),
            Card::Payment => tr.invoices_settings_save_payment(),
            Card::Defaults => tr.invoices_settings_save_defaults(),
        }
    }

    fn description(self, tr: Tr) -> String {
        match self {
            Card::Company => tr.invoices_settings_company_hint(),
            Card::Payment => tr.invoices_settings_payment_hint(),
            Card::Defaults => tr.invoices_settings_defaults_hint(),
        }
    }

    fn fields(self) -> &'static [Field] {
        const COMPANY: [Field; 5] = [
            field(
                "inv-settings-name",
                |tr| tr.invoices_settings_name(),
                "text",
                |b| &mut b.provider_name,
            ),
            multiline(
                "inv-settings-address",
                |tr| tr.invoices_settings_address(),
                |b| &mut b.provider_address,
            ),
            field(
                "inv-settings-tax-id",
                |tr| tr.invoices_settings_tax_id(),
                "text",
                |b| &mut b.provider_tax_id,
            ),
            field(
                "inv-settings-email",
                |tr| tr.invoices_settings_email(),
                "email",
                |b| &mut b.provider_email,
            ),
            field(
                "inv-settings-phone",
                |tr| tr.invoices_settings_phone(),
                "tel",
                |b| &mut b.provider_phone,
            ),
        ];
        const PAYMENT: [Field; 5] = [
            field(
                "inv-settings-bank",
                |tr| tr.invoices_settings_bank(),
                "text",
                |b| &mut b.bank_name,
            ),
            field(
                "inv-settings-iban",
                |tr| tr.invoices_settings_iban(),
                "text",
                |b| &mut b.bank_iban,
            ),
            field(
                "inv-settings-bic",
                |tr| tr.invoices_settings_bic(),
                "text",
                |b| &mut b.bank_bic,
            ),
            field(
                "inv-settings-routing",
                |tr| tr.invoices_settings_routing(),
                "text",
                |b| &mut b.bank_routing,
            ),
            field(
                "inv-settings-account",
                |tr| tr.invoices_settings_account(),
                "text",
                |b| &mut b.bank_account,
            ),
        ];
        const DEFAULTS: [Field; 2] = [
            field(
                "inv-settings-terms",
                |tr| tr.invoices_terms(),
                "text",
                |b| &mut b.invoice_payment_terms,
            ),
            multiline(
                "inv-settings-notes",
                |tr| tr.invoices_settings_notes(),
                |b| &mut b.invoice_notes,
            ),
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
    fn first_error(self, tr: Tr, draft: &OrgBranding) -> Option<(&'static str, String)> {
        let mut draft = draft.clone();
        self.fields().iter().find_map(|field| {
            let value = (field.value)(&mut draft).as_deref()?;
            value_error(tr, value).map(|message| (field.id, message))
        })
    }
}

#[component]
pub fn InvoiceSettings() -> Element {
    let tr = use_tr();
    let mut branding = use_resource(server_fns::get_org_branding);
    let retry = use_callback(move |_| branding.restart());
    rsx! {
        div {
            BackLink { to: Route::InvoiceList { filter: Default::default() }, label: tr.invoices_back() }
            PageHeader { title: tr.invoices_settings() }
            match &*branding.read() {
                None => rsx! { Loading { label: tr.invoices_settings_loading() } },
                Some(Ok(initial)) => rsx! { SettingsForm { initial: initial.clone() } },
                // A refusal shows `NoAccess`, as on every screen.
                Some(Err(error)) => rsx! {
                    LoadError { error: error.clone(), what: tr.invoices_settings_what(), retry }
                },
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
                    SettingsCard { key: "{card.fields()[0].id}", card, saved, draft }
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
    let tr = use_tr();
    let mut busy = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let mut notice = use_signal(|| false);
    let mut invalid = use_signal(|| None::<(&'static str, String)>);
    let heading = format!("{}-title", card.fields()[0].id);

    let save = move |event: FormEvent| {
        event.prevent_default();
        if busy() {
            return;
        }
        notice.set(false);
        error.set(None);
        invalid.set(card.first_error(tr, &draft.peek()));
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
            h2 { id: "{heading}", class: "card-title", {card.title(tr)} }
            p { class: "text-sm text-secondary mb-4", {card.description(tr)} }
            if let Some(message) = error() {
                div { class: "alert alert-danger", role: "alert", "{message}" }
            }
            fieldset { class: "border-0 p-0 m-0 min-w-0", disabled: busy(),
                for field in card.fields() {
                    {
                        let value = (field.value)(&mut draft.read().clone()).clone().unwrap_or_default();
                        let message = invalid().filter(|(id, _)| *id == field.id).map(|(_, message)| message);
                        let error_id = message.as_ref().map(|_| format!("{}-error", field.id));
                        let edit = move |event: FormEvent| {
                            notice.set(false);
                            if invalid.peek().as_ref().is_some_and(|(id, _)| *id == field.id) {
                                invalid.set(None);
                            }
                            *(field.value)(&mut draft.write()) = Some(event.value());
                        };
                        rsx! {
                            FormGroup { key: "{field.id}", label: (field.label)(tr), id: field.id,
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
                button { r#type: "submit", class: "btn btn-primary", disabled: busy(),
                    if busy() { {tr.invoices_settings_saving()} } else { {card.save_label(tr)} }
                }
                p { class: "text-sm text-secondary m-0", role: "status",
                    if notice() { {tr.invoices_settings_saved()} }
                }
            }
        }
    }
}

/// How the issuer block, payment details, terms and notes read on an invoice
/// PDF; it follows `templates/invoice.typ` and shows unsaved edits.
#[component]
fn InvoicePreview(draft: Signal<OrgBranding>) -> Element {
    let tr = use_tr();
    let b = draft.read();
    let filled = |value: &Option<String>| value.clone().filter(|value| !value.trim().is_empty());
    // The PDF prints these labels in English (`templates/invoice.typ`); the
    // preview names them in the viewer's language.
    let issuer = [
        b.provider_address.clone(),
        b.provider_tax_id
            .as_ref()
            .map(|id| tr.invoices_settings_preview_tax_id(id.clone())),
        b.provider_email.clone(),
        b.provider_phone.clone(),
    ];
    let payment = [
        (tr.invoices_settings_preview_bank(), &b.bank_name),
        (tr.invoices_settings_iban(), &b.bank_iban),
        (tr.invoices_settings_preview_bic(), &b.bank_bic),
        (tr.invoices_settings_preview_routing(), &b.bank_routing),
        (tr.invoices_settings_preview_account(), &b.bank_account),
    ]
    .into_iter()
    .filter_map(|(label, value)| {
        filled(value).map(|value| tr.invoices_settings_preview_line(label, value))
    })
    .collect::<Vec<_>>();
    let terms = filled(&b.invoice_payment_terms);
    let notes = filled(&b.invoice_notes);
    rsx! {
        section { class: "card p-5 min-w-0", aria_labelledby: "inv-settings-preview-title",
            h2 { id: "inv-settings-preview-title", class: "card-title", {tr.invoices_preview()} }
            p { class: "text-sm text-secondary mb-4", {tr.invoices_settings_preview_hint()} }
            div { class: "flex flex-col gap-4 text-sm wrap-anywhere",
                div {
                    if let Some(name) = filled(&b.provider_name) {
                        p { class: "font-semibold m-0", "{name}" }
                    } else {
                        p { class: "text-muted m-0", {tr.invoices_settings_preview_no_name()} }
                    }
                    // Addresses and notes keep the line breaks they were typed with, as on the PDF.
                    for line in issuer.into_iter().filter_map(|value| filled(&value)) {
                        p { class: "text-secondary m-0 whitespace-pre-line", "{line}" }
                    }
                }
                if !payment.is_empty() {
                    PreviewBlock { title: tr.invoices_settings_payment(), lines: payment }
                }
                if let Some(terms) = terms {
                    PreviewBlock { title: tr.invoices_settings_preview_terms(), lines: vec![terms] }
                }
                if let Some(notes) = notes {
                    PreviewBlock { title: tr.invoices_notes(), lines: vec![notes] }
                }
            }
        }
    }
}

#[component]
fn PreviewBlock(title: String, lines: Vec<String>) -> Element {
    rsx! {
        div {
            p { class: "text-xs font-semibold uppercase tracking-wide text-muted m-0 mb-1", "{title}" }
            for line in lines {
                p { class: "m-0 whitespace-pre-line", "{line}" }
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
            profile: None,
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
        const FORBIDDEN: u16 = 403;
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
        let tr = Tr::default();
        assert_eq!(Card::Company.first_error(tr, &draft), None);
        assert_eq!(
            Card::Payment.first_error(tr, &draft),
            Some(("inv-settings-iban", tr.invoices_settings_too_long()))
        );
        assert_eq!(
            Card::Defaults.first_error(tr, &draft),
            Some(("inv-settings-notes", tr.invoices_settings_nul()))
        );
    }
}
