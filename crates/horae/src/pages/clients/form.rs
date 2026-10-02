use dioxus::prelude::*;
use horae_core::client::{parse_default_rate, validate_profile};
use horae_core::money::format_cents_plain;
use uuid::Uuid;

use crate::components::form::{FormGroup, Input, Select, Textarea};
use crate::components::modal::Modal;
use crate::models::client::{
    ClientBillingSnapshot, ClientDetails, ClientProfile, ClientProfileEdit, ClientRateChange,
};
use crate::server_fns;

#[derive(Clone, PartialEq)]
struct ClientFields {
    name: String,
    currency: String,
    address: String,
    tax_id: String,
    rate: String,
    rate_touched: bool,
    confirmed_currency: Option<String>,
}

impl ClientFields {
    fn new(initial: Option<&ClientDetails>) -> Self {
        Self {
            name: initial
                .map(|detail| detail.client.name.clone())
                .unwrap_or_default(),
            currency: initial
                .map(|detail| detail.client.currency.clone())
                .unwrap_or_else(|| "USD".into()),
            address: initial
                .and_then(|detail| detail.client.address.clone())
                .unwrap_or_default(),
            tax_id: initial
                .and_then(|detail| detail.client.tax_id.clone())
                .unwrap_or_default(),
            rate: initial
                .and_then(|detail| detail.billing.as_ref()?.default_rate_cents)
                .map(format_cents_plain)
                .unwrap_or_default(),
            rate_touched: false,
            confirmed_currency: None,
        }
    }

    fn profile(&self) -> Result<ClientProfile, String> {
        validate_profile(
            &self.name,
            &self.currency,
            Some(&self.address),
            Some(&self.tax_id),
        )
        .map_err(|error| error.to_string())?;
        parse_default_rate(&self.rate).map_err(|error| error.to_string())?;
        Ok(ClientProfile {
            name: self.name.clone(),
            currency: self.currency.clone(),
            address: Some(self.address.clone()),
            tax_id: Some(self.tax_id.clone()),
        })
    }

    fn needs_rate_confirmation(&self, initial: &ClientDetails) -> bool {
        initial
            .billing
            .as_ref()
            .is_some_and(|billing| billing.default_rate_cents.is_some())
            && initial.client.currency != self.currency
            && !self.rate.trim().is_empty()
            && self.confirmed_currency.as_deref() != Some(self.currency.as_str())
    }

    fn edit(&self, initial: &ClientDetails) -> Result<ClientProfileEdit, String> {
        let billing = initial
            .billing
            .as_ref()
            .ok_or("Manager access is required to edit this client.")?;
        let profile = self.profile()?;
        if self.needs_rate_confirmation(initial) {
            return Err("Confirm the default rate in the new currency, enter a replacement, or clear the rate.".into());
        }
        let rate_change = if !self.rate_touched {
            ClientRateChange::Keep
        } else if self.rate.trim().is_empty() {
            ClientRateChange::Clear
        } else {
            ClientRateChange::Replace(self.rate.clone())
        };
        Ok(ClientProfileEdit {
            profile,
            rate_change,
            original: ClientBillingSnapshot {
                currency: initial.client.currency.clone(),
                default_rate_cents: billing.default_rate_cents,
            },
        })
    }
}

fn uncertain_creation(error: &ServerFnError) -> bool {
    !matches!(
        error,
        ServerFnError::ServerError {
            code: 400..=499,
            ..
        }
    )
}

#[component]
pub(super) fn ClientEditor(
    open: bool,
    client_id: Option<Uuid>,
    focus_fallback: &'static str,
    on_dismiss: EventHandler<()>,
    on_saved: EventHandler<ClientDetails>,
) -> Element {
    let busy = use_signal(|| false);
    rsx! {
        Modal { id: "client-editor-dialog", labelledby: "client-editor-title", open, busy: busy(), focus_fallback: Some(focus_fallback),
            on_dismiss: move |_| { if !busy() { on_dismiss.call(()); } },
            h2 { id: "client-editor-title", class: "modal-title text-left px-6 py-4 m-0",
                if client_id.is_some() { "Edit client" } else { "New client" }
            }
            if open {
                ClientEditorLoader { key: "{client_id:?}", client_id, busy, on_dismiss, on_saved }
            }
        }
    }
}

#[component]
fn ClientEditorLoader(
    client_id: Option<Uuid>,
    busy: Signal<bool>,
    on_dismiss: EventHandler<()>,
    on_saved: EventHandler<ClientDetails>,
) -> Element {
    let mut initial = use_resource(move || async move {
        match client_id {
            Some(id) => server_fns::get_client_details(id.to_string())
                .await
                .map(Some),
            None => Ok(None),
        }
    });
    let ready = initial.state()() == UseResourceState::Ready;
    let value = initial.read();
    rsx! {
        if !ready {
            p { class: "modal-body m-0", role: "status", aria_busy: "true", "Loading client editor…" }
        } else if let Some(Ok(detail)) = &*value {
            if detail.as_ref().is_none_or(|detail| detail.billing.is_some()) {
                ClientForm { initial: detail.clone(), busy, on_dismiss, on_saved }
            } else {
                div { class: "modal-body",
                    p { role: "alert", "Manager access is required to edit this client." }
                    button { r#type: "button", class: "btn btn-secondary", onclick: move |_| on_dismiss.call(()), "Close" }
                }
            }
        } else {
            div { class: "modal-body",
                p { role: "alert", "Could not load this client for editing." }
                div { class: "flex flex-wrap gap-3",
                    button { r#type: "button", class: "btn btn-primary", onclick: move |_| initial.restart(), "Retry editor" }
                    button { r#type: "button", class: "btn btn-secondary", onclick: move |_| on_dismiss.call(()), "Cancel" }
                }
            }
        }
    }
}

#[component]
fn ClientForm(
    initial: Option<ClientDetails>,
    mut busy: Signal<bool>,
    on_dismiss: EventHandler<()>,
    on_saved: EventHandler<ClientDetails>,
) -> Element {
    let mut fields = use_signal(|| ClientFields::new(initial.as_ref()));
    let initial = use_signal(|| initial);
    let baseline = use_signal(|| fields.peek().clone());
    let mut error = use_signal(|| None::<String>);
    let mut uncertain = use_signal(|| false);
    let editing = initial.read().is_some();
    let dirty = *fields.read() != *baseline.read();
    let needs_confirmation = initial
        .read()
        .as_ref()
        .is_some_and(|detail| fields.read().needs_rate_confirmation(detail));
    use_effect(move || {
        document::eval("document.getElementById('client-form-name')?.focus();");
    });
    use_effect(move || {
        if error().is_some() {
            document::eval("document.getElementById('client-form-error')?.focus();");
        }
    });
    rsx! {
        form { class: "modal-body", "data-editor-kind": "client",
            "data-editor-state": if busy() { "pending" } else if dirty || uncertain() { "dirty" } else { "clean" },
            onsubmit: move |event| {
                event.prevent_default();
                if busy() || uncertain() { return; }
                let submitted = fields.peek().clone();
                let original = initial.peek().clone();
                let profile = match submitted.profile() {
                    Ok(profile) => profile,
                    Err(message) => { error.set(Some(message)); return; }
                };
                let edit = match original.as_ref().map(|detail| submitted.edit(detail)).transpose() {
                    Ok(edit) => edit,
                    Err(message) => { error.set(Some(message)); return; }
                };
                busy.set(true);
                error.set(None);
                spawn(async move {
                    let result = match (original, edit) {
                        (Some(detail), Some(edit)) => server_fns::update_client_profile(detail.client.id.to_string(), edit).await,
                        (None, None) => server_fns::create_client_profile(profile, submitted.rate).await,
                        _ => Err(ServerFnError::new("Client editor state changed. Reopen the editor before saving.")),
                    };
                    busy.set(false);
                    match result {
                        Ok(detail) => on_saved.call(detail),
                        Err(problem) => {
                            if !editing && uncertain_creation(&problem) { uncertain.set(true); }
                            let message = match problem {
                                ServerFnError::ServerError { message, .. } => message,
                                _ => "The response could not be received. Your entered values have been kept.".into(),
                            };
                            error.set(Some(format!("Could not save client. {message}")));
                        }
                    }
                });
            },
            if let Some(message) = error() {
                div { id: "client-form-error", class: "alert alert-danger", role: "alert", tabindex: "-1", "{message}" }
            }
            if uncertain() {
                div { class: "alert alert-warning", role: "status",
                    p { class: "m-0 mb-3", "Creation may have completed. Check the refreshed client list before attempting another creation. Cancel does not undo a client that was already created." }
                    a { class: "btn btn-secondary btn-sm", href: "/clients", target: "_blank", rel: "noopener", "Check client list in a new tab" }
                    button { r#type: "button", class: "btn btn-secondary btn-sm mt-3", onclick: move |_| { uncertain.set(false); error.set(None); }, "I checked the list — allow another attempt" }
                }
            }
            FormGroup { label: "Client name", id: "client-form-name",
                Input { id: "client-form-name", value: fields.read().name.clone(), disabled: busy(), oninput: move |event: FormEvent| fields.write().name = event.value() }
            }
            div { class: "grid sm:grid-cols-2 gap-3",
                FormGroup { label: "Currency", id: "client-form-currency",
                    Select { id: "client-form-currency", selected: fields.read().currency.clone(), disabled: busy(),
                        options: horae_core::project::PROJECT_CURRENCIES.iter().map(|code| (code.to_string(), code.to_string())).collect(),
                        onchange: move |event: FormEvent| fields.write().currency = event.value(),
                    }
                }
                FormGroup { label: "Default rate", id: "client-form-rate",
                    Input { id: "client-form-rate", class: "font-mono", value: fields.read().rate.clone(), disabled: busy(),
                        oninput: move |event: FormEvent| {
                            let mut value = fields.write();
                            value.rate = event.value(); value.rate_touched = true; value.confirmed_currency = Some(value.currency.clone());
                        }
                    }
                    p { class: "text-xs text-secondary mt-1", "Per hour. Blank means not set; zero is a valid rate." }
                }
            }
            if needs_confirmation {
                div { class: "alert alert-warning",
                    p { class: "m-0 mb-3", "The saved default rate uses a different currency. Confirm this amount in the new currency, enter a replacement or clear it. Existing projects and invoices will not change." }
                    button { r#type: "button", class: "btn btn-secondary btn-sm", disabled: busy(),
                        onclick: move |_| {
                            let mut value = fields.write(); value.rate_touched = true; value.confirmed_currency = Some(value.currency.clone());
                        },
                        "Use this rate in {fields.read().currency}"
                    }
                }
            }
            FormGroup { label: "Billing address", id: "client-form-address",
                Textarea { id: "client-form-address", value: fields.read().address.clone(), disabled: busy(), oninput: move |event: FormEvent| fields.write().address = event.value() }
            }
            FormGroup { label: "Tax ID (optional)", id: "client-form-tax",
                Input { id: "client-form-tax", value: fields.read().tax_id.clone(), disabled: busy(), oninput: move |event: FormEvent| fields.write().tax_id = event.value() }
            }
            div { class: "flex flex-wrap gap-3 mt-4",
                button { r#type: "submit", class: "btn btn-primary", disabled: busy() || uncertain(),
                    if busy() { "Saving…" } else if editing { "Save changes" } else { "Create client" }
                }
                button { r#type: "button", class: "btn btn-secondary", disabled: busy(), onclick: move |_| { if !busy() { on_dismiss.call(()); } }, "Cancel" }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::client::{Client, ClientBilling};
    use uuid::Uuid;

    fn initial(rate: Option<i64>) -> ClientDetails {
        ClientDetails {
            client: Client {
                id: Uuid::now_v7(),
                org_id: Uuid::now_v7(),
                name: "Acme".into(),
                currency: "EUR".into(),
                address: Some("Address".into()),
                tax_id: None,
                active: true,
                created_at: chrono::DateTime::UNIX_EPOCH,
            },
            billing: Some(ClientBilling {
                default_rate_cents: rate,
            }),
        }
    }

    #[test]
    fn untouched_fields_keep_zero_rate_and_original_snapshot() {
        let initial = initial(Some(0));
        let fields = ClientFields::new(Some(&initial));
        assert_eq!(fields.rate, "0.00");
        let edit = fields.edit(&initial).unwrap();
        assert_eq!(edit.rate_change, ClientRateChange::Keep);
        assert_eq!(edit.original.default_rate_cents, Some(0));
        assert_eq!(edit.original.currency, "EUR");
    }

    #[test]
    fn rate_replacement_requires_confirmation_in_the_current_currency() {
        let initial = initial(Some(10000));
        let mut fields = ClientFields::new(Some(&initial));
        fields.currency = "USD".into();
        assert!(fields.edit(&initial).is_err());
        fields.rate_touched = true;
        fields.confirmed_currency = Some("USD".into());
        assert_eq!(
            fields.edit(&initial).unwrap().rate_change,
            ClientRateChange::Replace("100.00".into())
        );
        fields.currency = "GBP".into();
        assert!(fields.edit(&initial).is_err());
    }

    #[test]
    fn clearing_a_rate_is_explicit_and_needs_no_currency_confirmation() {
        let initial = initial(Some(0));
        let mut fields = ClientFields::new(Some(&initial));
        fields.currency = "USD".into();
        fields.rate.clear();
        fields.rate_touched = true;
        assert_eq!(
            fields.edit(&initial).unwrap().rate_change,
            ClientRateChange::Clear
        );
    }

    #[test]
    fn invalid_fields_are_not_rewritten_by_validation() {
        let initial = initial(None);
        let mut fields = ClientFields::new(Some(&initial));
        fields.name = "   ".into();
        fields.rate = "bad rate".into();
        let before = fields.clone();
        assert!(fields.profile().is_err());
        assert!(fields.edit(&initial).is_err());
        assert!(fields == before);
    }

    #[test]
    fn absent_billing_authority_never_produces_an_edit_payload() {
        let mut initial = initial(None);
        initial.billing = None;
        assert!(ClientFields::new(Some(&initial)).edit(&initial).is_err());
    }

    #[test]
    fn ambiguous_creation_errors_require_explicit_recovery() {
        assert!(uncertain_creation(&ServerFnError::new("Connection lost")));
        assert!(uncertain_creation(&ServerFnError::StreamError(
            "Connection lost".into()
        )));
        assert!(uncertain_creation(&ServerFnError::Deserialization(
            "Incomplete response".into()
        )));
        for code in [400, 403, 404, 409] {
            assert!(!uncertain_creation(&ServerFnError::ServerError {
                code,
                message: "Rejected".into(),
                details: None
            }));
        }
    }
}
