use dioxus::prelude::*;

use super::client_settings::HarvestOAuthApp;
use super::connect_outcome::SecretFocus;
use super::{Conn, use_connection};
use crate::components::back_link::BackLink;
use crate::components::page_header::PageHeader;
use crate::route::Route;

/// The Harvest OAuth client this workspace authenticates against.
#[component]
pub fn HarvestOAuth(focus: SecretFocus) -> Element {
    let mut status = use_connection();
    let changed = use_callback(move |_: ()| status.restart());
    let connected = Conn::from_resource(&status)
        .ready()
        .is_some_and(|s| s.connected);
    rsx! {
        div { class: "himp-page min-w-0",
            BackLink { to: Route::HarvestOverview { outcome: Default::default() }, label: "Back to Harvest" }
            PageHeader { title: "Harvest OAuth app" }
            p { class: "text-secondary text-sm mb-6",
                "Credentials for the Harvest app this workspace authenticates against. Stored once, used for every connection."
            }
            HarvestOAuthApp { connected, focus_secret: focus.0, on_changed: changed }
        }
    }
}
