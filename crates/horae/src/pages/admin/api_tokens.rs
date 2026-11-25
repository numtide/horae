//! Every personal access token in the organization, for administrators: who
//! owns it, when it was used, and revoke. No secret is ever shown here.

use dioxus::prelude::*;
use uuid::Uuid;

use crate::components::page_header::PageHeader;
use crate::pages::settings::api_tokens::TokenTable;
use crate::server_fns;

#[component]
pub fn AdminApiTokens() -> Element {
    let tr = crate::i18n::use_tr();
    let mut tokens = use_resource(server_fns::list_org_api_tokens);
    let mut revoke_error = use_signal(|| None::<String>);
    let on_revoke = use_callback(move |id: Uuid| {
        revoke_error.set(None);
        spawn(async move {
            match server_fns::revoke_api_token(id.to_string()).await {
                Ok(_) => tokens.restart(),
                Err(_) => revoke_error.set(Some(tr.api_tokens_revoke_failed())),
            }
        });
    });
    rsx! {
        PageHeader { title: tr.api_tokens_title(), title_id: "admin-api-tokens-title" }
        p { class: "text-secondary text-sm mb-4", {tr.api_tokens_admin_intro()} }
        if let Some(message) = revoke_error() {
            p { class: "alert alert-danger", role: "alert", "{message}" }
        }
        TokenTable { tokens, show_owner: true, focus_fallback: "admin-api-tokens-title", on_revoke }
    }
}
