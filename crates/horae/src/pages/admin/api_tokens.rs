//! Every personal access token in the organization, for administrators: who
//! owns it, when it was used, and revoke. No secret is ever shown here.

use dioxus::prelude::*;
use uuid::Uuid;

use crate::components::page_header::PageHeader;
use crate::pages::settings::api_tokens::TokenTable;
use crate::server_fns;

#[component]
pub fn AdminApiTokens() -> Element {
    let mut tokens = use_resource(server_fns::list_org_api_tokens);
    let mut revoke_error = use_signal(|| None::<String>);
    let on_revoke = use_callback(move |id: Uuid| {
        revoke_error.set(None);
        spawn(async move {
            match server_fns::revoke_api_token(id.to_string()).await {
                Ok(_) => tokens.restart(),
                Err(_) => revoke_error.set(Some("Could not revoke the token. Try again.".into())),
            }
        });
    });
    rsx! {
        PageHeader { title: "API tokens" }
        p { class: "text-secondary text-sm mb-4",
            "Every token people in this workspace have created. Revoking one stops it working at once; its owner can create another."
        }
        if let Some(message) = revoke_error() {
            p { class: "alert alert-danger", role: "alert", "{message}" }
        }
        match &*tokens.read() {
            None => rsx! { p { class: "text-muted text-sm", role: "status", "Loading tokens…" } },
            Some(Err(_)) => rsx! {
                p { class: "alert alert-danger", role: "alert", "Could not load the tokens. Reload the page to try again." }
            },
            Some(Ok(rows)) if rows.is_empty() => rsx! {
                p { class: "text-muted text-sm", "Nobody has created a token yet." }
            },
            Some(Ok(rows)) => rsx! { TokenTable { rows: rows.clone(), show_owner: true, on_revoke } },
        }
    }
}
