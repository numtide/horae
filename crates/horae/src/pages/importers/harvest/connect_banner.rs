use dioxus::prelude::*;
use horae_core::importers::harvest::client::ConnectFailure;

use super::connect_outcome::SecretFocus;
use super::registered_redirect::RegisteredRedirect;
use crate::route::Route;

/// Where an administrator registers Horae's redirect URL in Harvest.
const HARVEST_DEVELOPERS: &str = "https://id.getharvest.com/developers";

/// Why the last connection attempt failed, and the one action that fixes it.
/// Dismissing it drops the query, so a reload does not show it again.
#[component]
pub(super) fn ConnectBanner(
    failure: ConnectFailure,
    busy: bool,
    on_connect: EventHandler<()>,
) -> Element {
    let navigator = use_navigator();
    let denied = failure == ConnectFailure::Denied;
    let title = match failure {
        ConnectFailure::InvalidClient => {
            "Harvest didn't accept Horae's OAuth app credentials.".to_owned()
        }
        ConnectFailure::RedirectMismatch => {
            "The redirect URL doesn't match the one registered in your Harvest app.".to_owned()
        }
        ConnectFailure::CodeExpired => {
            "The authorization expired before Horae could use it.".to_owned()
        }
        ConnectFailure::Denied => "You cancelled the connection in Harvest.".to_owned(),
        ConnectFailure::State => {
            "The connection couldn't be verified. Start it again from this page.".to_owned()
        }
        ConnectFailure::Other {
            status: Some(status),
        } => format!("Harvest refused the connection (HTTP {status})."),
        ConnectFailure::Other { status: None } => {
            "Horae couldn't finish connecting to Harvest.".to_owned()
        }
    };
    let connect_label = match failure {
        ConnectFailure::CodeExpired => "Connect again",
        ConnectFailure::Other { .. } => "Try again",
        _ => "Connect Harvest",
    };

    rsx! {
        div {
            class: if denied { "banner banner-info mb-0" } else { "banner banner-danger mb-0" },
            role: if denied { "status" } else { "alert" },
            "data-testid": "connect-banner",
            span { class: "banner-icon", aria_hidden: "true", if denied { "ⓘ" } else { "✕" } }
            div { class: "banner-body flex flex-col gap-3",
                div {
                    p { class: "banner-title", "{title}" }
                    match failure {
                        ConnectFailure::InvalidClient => rsx! {
                            p { class: "banner-detail",
                                "Check the client ID and secret. A blank secret keeps the stored one, so if you regenerated it in Harvest, paste the new one."
                            }
                        },
                        ConnectFailure::RedirectMismatch => rsx! {
                            p { class: "banner-detail",
                                "Register this URL in your Harvest app exactly as shown, then connect again."
                            }
                        },
                        ConnectFailure::CodeExpired => rsx! {
                            p { class: "banner-detail",
                                "Connect again. If it keeps happening, check the redirect URL in your Harvest app: Harvest reports a mismatch the same way."
                            }
                        },
                        ConnectFailure::Other { .. } => rsx! {
                            p { class: "banner-detail", "Try again. The server log has the details." }
                        },
                        ConnectFailure::Denied | ConnectFailure::State => rsx! {},
                    }
                }
                if failure == ConnectFailure::RedirectMismatch {
                    RegisteredRedirect {}
                }
                div { class: "flex gap-3 items-center flex-wrap",
                    match failure {
                        ConnectFailure::InvalidClient => rsx! {
                            Link {
                                to: Route::HarvestOAuth { focus: SecretFocus(true) },
                                class: "btn btn-primary btn-sm",
                                "data-testid": "fix-oauth-secret".to_owned(),
                                "Open OAuth app credentials"
                            }
                        },
                        ConnectFailure::RedirectMismatch => rsx! {
                            a {
                                href: HARVEST_DEVELOPERS,
                                target: "_blank",
                                rel: "noopener noreferrer",
                                class: "btn btn-primary btn-sm",
                                "data-testid": "harvest-developers",
                                aria_label: "Open Harvest developer settings (opens in a new tab)",
                                "Open Harvest developer settings ↗"
                            }
                        },
                        _ => rsx! {
                            button {
                                r#type: "button",
                                class: "btn btn-primary btn-sm",
                                "data-testid": "banner-connect".to_owned(),
                                disabled: busy,
                                onclick: move |_| on_connect.call(()),
                                if busy { "Connecting…" } else { "{connect_label}" }
                            }
                        },
                    }
                    button {
                        r#type: "button",
                        class: "btn btn-ghost btn-sm",
                        "data-testid": "dismiss-connect-banner".to_owned(),
                        onclick: move |_| {
                            navigator.replace(Route::HarvestOverview { outcome: Default::default() });
                        },
                        "Dismiss"
                    }
                }
            }
        }
    }
}
