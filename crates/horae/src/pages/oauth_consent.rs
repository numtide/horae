//! The OAuth approval page (spec 024 FR-024 to FR-026): outside the app
//! shell, one card naming the app, what it will be able to do and where
//! Allow returns, with Allow and Deny. `GET /oauth/authorize` checked the
//! request and keeps it in the session under `request`; the page leaves for
//! the app's redirect URI with a full navigation once the person answers.
//!
//! The whole layout is this one component, so a change to the design stays
//! in one place.

use dioxus::prelude::*;

use crate::components::badge::Badge;
use crate::components::button::{Button, ButtonVariant};
use crate::components::icons::NavIcon;
use crate::components::logo::HoraeMark;
use crate::components::screen_state::ErrorState;
use crate::i18n::{Tr, split_tag};
use crate::server_fns;

/// What a server function answers for a request that is unknown, expired,
/// already answered, or whose app was disabled since.
const NOT_FOUND: u16 = 404;

#[derive(Clone, Copy, PartialEq)]
enum Answer {
    Allow,
    Deny,
}

#[component]
pub fn OAuthConsent(request: String) -> Element {
    let tr = crate::i18n::use_tr();
    let mut details = use_resource({
        let request = request.clone();
        move || server_fns::oauth_consent_details(request.clone())
    });
    let me = use_resource(server_fns::get_me);
    let retry = use_callback(move |_| details.restart());
    // The answer being sent; once set, both buttons stay disabled.
    let mut sending = use_signal(|| None::<Answer>);
    let mut expired = use_signal(|| false);
    let mut failed = use_signal(|| false);

    let answer = use_callback({
        let request = request.clone();
        move |answer: Answer| {
            if sending.peek().is_some() {
                return;
            }
            sending.set(Some(answer));
            failed.set(false);
            let request = request.clone();
            spawn(async move {
                let result = match answer {
                    Answer::Allow => server_fns::approve_oauth_request(request).await,
                    Answer::Deny => server_fns::deny_oauth_request(request).await,
                };
                match result {
                    Ok(url) => {
                        // A full navigation: the address is the app's, often
                        // another scheme. The page stays disabled as it leaves.
                        let js = format!(
                            "window.location.assign({});",
                            serde_json::to_string(&url).unwrap_or_default()
                        );
                        let _ = document::eval(&js).await;
                    }
                    Err(ServerFnError::ServerError {
                        code: NOT_FOUND, ..
                    }) => expired.set(true),
                    Err(_) => {
                        failed.set(true);
                        sending.set(None);
                    }
                }
            });
        }
    });
    let allow = use_callback(move |_: MouseEvent| answer.call(Answer::Allow));
    let deny = use_callback(move |_: MouseEvent| answer.call(Answer::Deny));

    let card = match &*details.read() {
        _ if expired() => expired_card(tr),
        None => loading_card(tr),
        Some(Err(ServerFnError::ServerError {
            code: NOT_FOUND, ..
        })) => expired_card(tr),
        Some(Err(_)) => rsx! {
            ErrorState {
                title: tr.oauth_consent_failed(),
                body: tr.oauth_consent_failed_body(),
                onretry: retry,
            }
        },
        Some(Ok(consent)) => {
            let return_line = tr.oauth_consent_return(consent.return_to_display.clone());
            let (before, target, after) = split_tag(&return_line, "target");
            let sent = sending();
            rsx! {
                section { class: "consent-card", aria_labelledby: "consent-title",
                    h1 { id: "consent-title", class: "consent-title",
                        {tr.oauth_consent_title(consent.app_name.clone())}
                    }
                    div { class: "consent-origin",
                        if consent.built_in {
                            Badge { variant: "info", {tr.oauth_consent_built_in()} }
                        } else {
                            Badge { variant: "neutral", {tr.oauth_consent_registered()} }
                        }
                    }
                    p { class: "consent-lead", {tr.oauth_consent_can()} }
                    ul { class: "consent-scopes",
                        for scope in consent.scopes.iter() {
                            if let Some((icon, text)) = scope_line(tr, scope) {
                                li { key: "{scope}", class: "consent-scope",
                                    span { class: "consent-scope-icon", aria_hidden: "true",
                                        NavIcon { name: icon }
                                    }
                                    span { {text} }
                                }
                            }
                        }
                    }
                    p { class: "consent-return",
                        {before}
                        code { class: "consent-target", {target} }
                        {after}
                    }
                    if failed() {
                        p { class: "consent-error", role: "alert", {tr.oauth_consent_send_failed()} }
                    }
                    div { class: "consent-actions",
                        Button {
                            id: "consent-allow",
                            label: tr.oauth_consent_allow(),
                            disabled: sent.is_some(),
                            aria_busy: (sent == Some(Answer::Allow)).then_some("true"),
                            onclick: allow,
                        }
                        Button {
                            id: "consent-deny",
                            variant: ButtonVariant::Secondary,
                            label: tr.oauth_consent_deny(),
                            disabled: sent.is_some(),
                            aria_busy: (sent == Some(Answer::Deny)).then_some("true"),
                            onclick: deny,
                        }
                    }
                }
            }
        }
    };

    let person = me
        .read()
        .as_ref()
        .and_then(|me| me.as_ref().ok())
        .map(|me| me.name.clone());
    rsx! {
        main { class: "consent-page",
            div { class: "consent-brand",
                HoraeMark {}
                span { class: "consent-brand-name", {tr.oauth_consent_brand()} }
            }
            {card}
            if let Some(name) = person {
                // Signing out keeps this request for whoever signs in next.
                form { class: "consent-account", method: "post", action: "/auth/logout",
                    input { r#type: "hidden", name: "return_to", value: "/oauth/consent?request={request}" }
                    span { class: "consent-person", {tr.oauth_consent_signed_in(name)} }
                    button { id: "consent-sign-out", r#type: "submit", class: "consent-sign-out",
                        {tr.oauth_consent_sign_out()}
                    }
                }
            }
        }
    }
}

/// The sentence and icon of one scope (FR-060); `None` for a name this
/// version does not know.
fn scope_line(tr: Tr, scope: &str) -> Option<(&'static str, String)> {
    match scope {
        "time:read" => Some(("time", tr.oauth_scope_time_read())),
        "time:write" => Some(("pencil", tr.oauth_scope_time_write())),
        "catalog:read" => Some(("projects", tr.oauth_scope_catalog_read())),
        _ => None,
    }
}

/// Nothing to answer: one sentence and no action, since there is nowhere
/// safe to send the person.
fn expired_card(tr: Tr) -> Element {
    rsx! {
        section { class: "consent-card", role: "alert",
            h1 { class: "consent-title consent-title-sm", {tr.oauth_consent_expired_title()} }
            p { class: "consent-lead", {tr.oauth_consent_expired_body()} }
        }
    }
}

/// The card at its loaded size, with bars for the title, the access list and
/// both buttons.
fn loading_card(tr: Tr) -> Element {
    rsx! {
        section { class: "consent-card",
            p { class: "sr-only", role: "status", {tr.oauth_consent_loading()} }
            div { class: "consent-skeleton", aria_busy: "true", aria_hidden: "true",
                span { class: "skeleton-bar consent-skeleton-title" }
                for width in ["90%", "60%", "75%", "70%", "80%"] {
                    span { key: "{width}", class: "skeleton-bar", style: "width: {width}" }
                }
                div { class: "consent-actions",
                    span { class: "skeleton-bar consent-skeleton-button" }
                    span { class: "skeleton-bar consent-skeleton-button" }
                }
            }
        }
    }
}
